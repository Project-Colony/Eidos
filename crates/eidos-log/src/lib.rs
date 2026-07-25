//! Session logging for Eidos.
//!
//! Eidos exists to diagnose silent failures: a mod that did not deploy, a
//! script extender the game refused to load, a FUSE mount that quietly fell
//! back to rootless mode. Those diagnostics used to go to `eprintln!`, which is
//! written to a terminal that does not exist when the GUI is started from Steam
//! or from a desktop file. Every one of those messages was lost exactly in the
//! situation it was written for.
//!
//! This crate writes them to a file instead, and *also* to stderr so a run from
//! a terminal looks the same as it always did. The design constraints, in the
//! order they mattered:
//!
//! * **A file per run.** Sessions are the unit users talk about ("it broke the
//!   third time I launched"), so a run gets its own file under
//!   `$XDG_STATE_HOME/eidos/logs/` (XDG basedir 0.8 puts logs in the *state*
//!   dir, not data or cache: state is "persists between restarts, not portable,
//!   not precious", which is exactly a log). Appending to one growing file
//!   would force the reader to guess where the interesting run starts.
//! * **Rotation.** Only the last [`DEFAULT_KEEP`] sessions per instance survive,
//!   so an unattended machine cannot fill its home partition with logs.
//! * **Redaction.** Users paste these into GitHub issues. `/home/alice/...`
//!   renders as `~/...` and a bare `alice` as `<user>`, via a pure function
//!   ([`Redactor::apply`]) that is unit-tested rather than trusted.
//! * **Thread safety.** The FUSE daemon answers kernel requests on many threads
//!   and the GUI has its own; a record is one `write_all` under one mutex, so
//!   lines never interleave.
//!
//! Call sites read like the `eprintln!` they replace:
//!
//! ```no_run
//! eidos_log::init("skyrimse");
//! eidos_log::info!("deployed {} mods", 42);
//! eidos_log::warn!("no Proton prefix found, skipping plugins.txt");
//! ```
//!
//! `EIDOS_LOG=debug|info|warn|error` raises or lowers both thresholds at once.

use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// How many session logs to keep per instance before deleting the oldest.
///
/// Per instance, not global: launching Skyrim must not delete the Fallout log
/// the user was about to attach to a bug report. Ten is roughly a week of
/// evening play sessions, and a session log is a few kilobytes.
pub const DEFAULT_KEEP: usize = 10;

/// Severity of one record. Ordered, so a threshold is a simple `>=`.
///
/// `Debug` is new (nothing in the codebase used to print at that level); it goes
/// to the file but not to the terminal by default, which is how a log can be
/// verbose enough to be useful without changing what a terminal run looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Detail worth having in the file when something goes wrong later.
    Debug,
    /// Normal progress: what Eidos did.
    Info,
    /// Something is off but the operation continued.
    Warn,
    /// The operation failed.
    Error,
}

impl Level {
    /// The tag written in the log line. Fixed vocabulary so `grep -c ERROR`
    /// works on a pasted log.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    /// Parse a threshold from `EIDOS_LOG`. Case-insensitive, and `warning` is
    /// accepted because that is the word people type.
    pub fn parse(s: &str) -> Option<Level> {
        match s.trim().to_ascii_lowercase().as_str() {
            "debug" | "trace" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" | "warning" => Some(Level::Warn),
            "error" | "err" => Some(Level::Error),
            _ => None,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How a session should be logged. Built by [`Config::new`] from the
/// environment; the fields are public so a caller (or a test) can override any
/// of them before handing it to [`init_with`].
#[derive(Debug, Clone)]
pub struct Config {
    /// Names the run in the file name, normally the instance or game id. It is
    /// also the rotation bucket, so distinct processes that share an instance
    /// (the GUI and the FUSE daemon) should pass distinct names, e.g.
    /// `skyrimse` and `skyrimse-fuse`.
    pub instance: String,
    /// Directory the session file is created in.
    pub dir: PathBuf,
    /// Session files to keep in `dir` for this instance, oldest deleted first.
    pub keep: usize,
    /// Minimum level written to the file.
    pub file_level: Level,
    /// Minimum level echoed to stderr.
    pub stderr_level: Level,
    /// Whether to echo to stderr at all.
    pub stderr: bool,
    /// Version of the program opening the session, for the header. Set it from
    /// the *binary* crate (`env!("CARGO_PKG_VERSION")`), since that is the
    /// version a bug report is about; `None` omits the line.
    pub version: Option<String>,
}

impl Config {
    /// Defaults for `instance`: log to [`log_dir`], keep [`DEFAULT_KEEP`], write
    /// everything to the file, echo `Info` and above to stderr. `EIDOS_LOG`
    /// overrides both thresholds, which is the one knob a bug reporter can be
    /// asked to set over chat.
    pub fn new(instance: &str) -> Config {
        let mut cfg = Config {
            instance: instance.to_string(),
            dir: log_dir(),
            keep: DEFAULT_KEEP,
            file_level: Level::Debug,
            stderr_level: Level::Info,
            stderr: true,
            version: None,
        };
        if let Some(l) = std::env::var("EIDOS_LOG").ok().and_then(|v| Level::parse(&v)) {
            cfg.file_level = l;
            cfg.stderr_level = l;
        }
        cfg
    }

    /// Record which build this session came from.
    pub fn with_version(mut self, version: &str) -> Config {
        self.version = Some(version.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// XDG paths
// ---------------------------------------------------------------------------

/// `$XDG_STATE_HOME/eidos`, or `$HOME/.local/state/eidos`.
pub fn state_dir() -> PathBuf {
    state_dir_from(std::env::var_os("XDG_STATE_HOME"), std::env::var_os("HOME"))
}

/// Where session logs live: `$XDG_STATE_HOME/eidos/logs`.
pub fn log_dir() -> PathBuf {
    state_dir().join("logs")
}

/// The path resolution behind [`state_dir`], split out so it can be tested
/// without mutating process-wide environment variables (which would race with
/// every other test in the binary).
///
/// Falls back to the temp dir rather than `/` when `HOME` is unset: a log we
/// cannot write is worse than a log in an odd place, and `HOME` is genuinely
/// missing in some systemd and Steam-launcher contexts.
fn state_dir_from(xdg: Option<OsString>, home: Option<OsString>) -> PathBuf {
    if let Some(x) = xdg.filter(|x| !x.is_empty()) {
        return PathBuf::from(x).join("eidos");
    }
    match home.filter(|h| !h.is_empty()) {
        Some(h) => PathBuf::from(h).join(".local/state/eidos"),
        None => std::env::temp_dir().join("eidos"),
    }
}

// ---------------------------------------------------------------------------
// Global logger
// ---------------------------------------------------------------------------

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Start a session log for `instance`, returning the file that was opened.
///
/// Returns `None` if no file could be created (read-only home, full disk): in
/// that case logging still works and still reaches stderr, because a mod manager
/// that refuses to run because it cannot log is worse than one that runs quietly.
/// Calling this twice is harmless; the first session wins.
pub fn init(instance: &str) -> Option<PathBuf> {
    init_with(Config::new(instance))
}

/// [`init`] with an explicit [`Config`].
pub fn init_with(cfg: Config) -> Option<PathBuf> {
    let logger = LOGGER.get_or_init(|| Logger::open(&cfg));
    logger.path.clone()
}

/// The session log file, once [`init`] has run. The GUI shows this so a user can
/// find the file to attach without knowing what XDG is.
pub fn path() -> Option<PathBuf> {
    LOGGER.get().and_then(|l| l.path.clone())
}

/// Emit one record. The macros call this; call it directly only when the level
/// is a runtime value.
pub fn log(level: Level, args: fmt::Arguments<'_>) {
    match LOGGER.get() {
        Some(l) => l.emit(level, args),
        // Before init (early argument parsing, a panic hook) there is nothing to
        // write to but stderr. Losing these would be worse than the duplicate
        // formatting cost.
        None => {
            if level >= Level::Info {
                let msg = redact(&args.to_string()).into_owned();
                let _ = writeln!(std::io::stderr(), "{:<5} {}", level.as_str(), msg);
            }
        }
    }
}

/// Log at [`Level::Debug`]: detail for the file, invisible in the terminal.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { $crate::log($crate::Level::Debug, ::core::format_args!($($arg)*)) };
}

/// Log at [`Level::Info`]: the drop-in replacement for a plain `eprintln!`.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::log($crate::Level::Info, ::core::format_args!($($arg)*)) };
}

/// Log at [`Level::Warn`]: it kept going, but say so.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { $crate::log($crate::Level::Warn, ::core::format_args!($($arg)*)) };
}

/// Log at [`Level::Error`]: it did not work.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { $crate::log($crate::Level::Error, ::core::format_args!($($arg)*)) };
}

/// The process-wide sink. Held in a `OnceLock`, so it is created once and shared
/// by every thread without further synchronisation beyond the file mutex.
struct Logger {
    /// `None` when the file could not be opened; stderr still works.
    file: Option<Mutex<File>>,
    path: Option<PathBuf>,
    file_level: Level,
    stderr_level: Level,
    stderr: bool,
}

impl Logger {
    fn open(cfg: &Config) -> Logger {
        match open_session(&cfg.dir, &cfg.instance, cfg.keep) {
            Ok((file, path)) => {
                let logger = Logger {
                    file: Some(Mutex::new(file)),
                    path: Some(path),
                    file_level: cfg.file_level,
                    stderr_level: cfg.stderr_level,
                    stderr: cfg.stderr,
                };
                logger.write_header(cfg);
                logger
            }
            Err(e) => {
                // Say why on stderr rather than failing: on a machine where the
                // state dir is unwritable the user still gets their messages.
                let _ = writeln!(
                    std::io::stderr(),
                    "WARN  eidos-log: no session log in {} ({e}); logging to stderr only",
                    redact(&cfg.dir.display().to_string())
                );
                Logger {
                    file: None,
                    path: None,
                    file_level: cfg.file_level,
                    stderr_level: cfg.stderr_level,
                    stderr: cfg.stderr,
                }
            }
        }
    }

    /// The preamble that makes a pasted log self-describing: when the run
    /// started, in which time zone the bare timestamps below are, which build
    /// and which command line produced it.
    fn write_header(&self, cfg: &Config) {
        let (secs, _) = now_parts();
        let offset = local_utc_offset(secs);
        // argv can carry the user's paths (an instance dir, a mod archive), so
        // it goes through the same redaction as any other record.
        let argv: Vec<String> = std::env::args().map(|a| redact(&a).into_owned()).collect();
        let mut header = String::new();
        header.push_str("==== eidos session log ====\n");
        header.push_str(&format!(
            "started  : {} {} (timestamps below are local time at this offset)\n",
            format_civil(secs + i64::from(offset), None),
            format_offset(offset)
        ));
        header.push_str(&format!("instance : {}\n", cfg.instance));
        if let Some(v) = &cfg.version {
            header.push_str(&format!("version  : {v}\n"));
        }
        header.push_str(&format!("pid      : {}\n", std::process::id()));
        header.push_str(&format!("command  : {}\n", argv.join(" ")));
        header.push_str(&format!(
            "retention: last {} session(s) for this instance\n",
            cfg.keep.max(1)
        ));
        header.push_str("===========================\n");
        if let Some(f) = &self.file {
            let mut f = lock(f);
            let _ = f.write_all(header.as_bytes());
        }
    }

    fn emit(&self, level: Level, args: fmt::Arguments<'_>) {
        let to_file = self.file.is_some() && level >= self.file_level;
        let to_stderr = self.stderr && level >= self.stderr_level;
        if !to_file && !to_stderr {
            return;
        }

        // Format and redact once, then reuse for both sinks: the two must never
        // disagree, or a user's pasted terminal output would contradict the file.
        let msg = redact(&args.to_string()).into_owned();

        if let Some(f) = self.file.as_ref().filter(|_| to_file) {
            let (secs, millis) = now_parts();
            let offset = local_utc_offset(secs);
            let line = format!(
                "{} {:<5} {}\n",
                format_civil(secs + i64::from(offset), Some(millis)),
                level.as_str(),
                msg
            );
            // One write_all per record, so records from different threads land
            // whole rather than interleaved. The lock is also held across the
            // stderr write so the two sinks keep the same order.
            let mut f = lock(f);
            let _ = f.write_all(line.as_bytes());
            if to_stderr {
                let _ = writeln!(std::io::stderr(), "{:<5} {}", level.as_str(), msg);
            }
        } else if to_stderr {
            // No timestamp on stderr: a terminal run should look like the
            // `eprintln!` output this replaced, just with the level in front.
            let _ = writeln!(std::io::stderr(), "{:<5} {}", level.as_str(), msg);
        }
    }
}

/// Lock a mutex, ignoring poisoning. A thread that panicked while logging must
/// not silence every later message - the panic is precisely what we want logged.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Session files and rotation
// ---------------------------------------------------------------------------

/// Create this run's log file in `dir` and drop older ones for the same
/// instance, keeping `keep` in total. Returns the open file and its path.
fn open_session(dir: &Path, instance: &str, keep: usize) -> std::io::Result<(File, PathBuf)> {
    fs::create_dir_all(dir)?;
    let slug = slug(instance);
    let (secs, _) = now_parts();
    let offset = local_utc_offset(secs);
    let name = session_file_name(&slug, secs + i64::from(offset), std::process::id());
    let path = dir.join(name);

    // Second plus pid makes a name collision practically impossible, and append
    // rather than truncate means that if one ever did happen (a recycled pid
    // inside the same second) the older session is added to, never erased.
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    // A log can incidentally capture a path, a game key in a command line, or an
    // API key echoed in an error. Other users on the box have no business
    // reading it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }

    rotate(dir, &format!("{slug}."), keep, &path);
    Ok((file, path))
}

/// `<slug>.<YYYYMMDD-HHMMSS>.<pid>.log`.
///
/// The `.` after the slug is the bucket separator and cannot occur inside a slug
/// (see [`slug`]), so instances `skyrim` and `skyrim-2` never rotate each other
/// away. Timestamp before pid means a plain name sort is a chronological sort.
fn session_file_name(slug: &str, local_secs: i64, pid: u32) -> String {
    let (y, m, d, hh, mm, ss) = civil_from_unix(local_secs);
    format!("{slug}.{y:04}{m:02}{d:02}-{hh:02}{mm:02}{ss:02}.{pid}.log")
}

/// Reduce an instance name to something safe in a file name: lowercase ASCII
/// alphanumerics, `-` and `_`. Everything else (spaces, `/`, accents, `.`)
/// becomes `-`, which also guarantees the slug cannot contain the `.` used as
/// the rotation-bucket separator.
fn slug(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    // The mapping above emits only ASCII, so truncating at a byte index cannot
    // split a character. 40 keeps the name readable in `ls`.
    s.truncate(40);
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "eidos".to_string()
    } else {
        s
    }
}

/// Delete session files in `dir` whose name starts with `prefix`, keeping the
/// `keep` newest by name. Returns how many were removed.
///
/// Best effort: a file we cannot delete is not worth failing a launch over.
/// `current` is never deleted even if it sorts last, which it can if the clock
/// jumped backwards between runs (dual-boot with a Windows RTC does exactly
/// that) - losing the log of the run in progress would be the worst outcome.
fn rotate(dir: &Path, prefix: &str, keep: usize, current: &Path) -> usize {
    let keep = keep.max(1);
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut mine: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix) && n.ends_with(".log"))
        })
        .collect();
    // Newest first. Same prefix means the next field is the timestamp, so a
    // lexicographic sort is chronological without stat()ing anything.
    mine.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    let mut removed = 0;
    for p in mine.into_iter().skip(keep) {
        if p == current {
            continue;
        }
        if fs::remove_file(&p).is_ok() {
            removed += 1;
        }
    }
    removed
}

// ---------------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------------

/// Usernames that are also words Eidos itself logs constantly.
///
/// Replacing a bare `steam` or `mods` everywhere would shred the log while
/// protecting nothing that matters: the real leak vector is the home path, and
/// that is redacted regardless of what the account is called. Only the
/// bare-word pass is skipped for these.
const AMBIGUOUS_USERNAMES: &[&str] = &[
    "steam", "proton", "wine", "game", "games", "mod", "mods", "data", "home", "user", "users",
    "root", "log", "logs", "bin", "lib", "usr", "var", "tmp", "share", "local", "state", "default",
    "eidos", "profile", "profiles", "save", "saves", "plugin", "plugins", "overwrite", "instance",
];

/// The shortest username the bare-word pass will touch. Two-letter logins exist
/// but collide with too much ordinary text (`id`, `so`, `mo`) to be worth it.
const MIN_BARE_USERNAME: usize = 3;

/// Turns identifying strings into placeholders. Pure and cheap to construct, so
/// tests can pin the behaviour without touching the environment.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    /// Home directory with any trailing slash removed, e.g. `/home/alice`.
    home: Option<String>,
    /// The same with `\` separators, as Wine and Proton print it (`Z:\home\alice`).
    home_win: Option<String>,
    /// The login name, only when redacting it as a bare word is safe.
    user: Option<String>,
}

impl Redactor {
    /// Build from a home directory and a login name, applying the safety rules:
    /// a home of `/` (or shorter than `/x`) is ignored, because replacing every
    /// `/` with `~` would destroy the log; a username is only eligible for the
    /// bare-word pass when it is long enough and not [`AMBIGUOUS_USERNAMES`].
    pub fn new(home: Option<&str>, user: Option<&str>) -> Redactor {
        let home = home
            .map(|h| h.trim_end_matches('/').to_string())
            .filter(|h| h.len() >= 2 && h.starts_with('/'));
        let home_win = home.as_ref().map(|h| h.replace('/', "\\"));
        let user = user
            .map(str::trim)
            .filter(|u| u.len() >= MIN_BARE_USERNAME)
            .filter(|u| !AMBIGUOUS_USERNAMES.contains(&u.to_ascii_lowercase().as_str()))
            .map(str::to_string);
        Redactor { home, home_win, user }
    }

    /// Read `HOME` and `USER` from the environment, falling back to the last
    /// component of `HOME` when `USER` is unset (it often is under systemd and
    /// inside Steam's launcher).
    pub fn from_env() -> Redactor {
        let home = std::env::var("HOME").ok();
        let user = std::env::var("USER").ok().or_else(|| {
            home.as_deref()
                .and_then(|h| h.trim_end_matches('/').rsplit('/').next())
                .map(str::to_string)
        });
        Redactor::new(home.as_deref(), user.as_deref())
    }

    /// Replace the home directory with `~` and the bare username with `<user>`.
    ///
    /// Borrowed back unchanged when there is nothing to redact, which is the
    /// common case for a log line. Home paths go first so that by the time the
    /// bare-word pass runs, `/home/alice/mods` is already `~/mods` and holds no
    /// username to find.
    pub fn apply<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let mut out = Cow::Borrowed(text);
        for (needle, with) in [
            (self.home.as_deref(), "~"),
            (self.home_win.as_deref(), "~"),
            (self.user.as_deref(), "<user>"),
        ] {
            let Some(needle) = needle else { continue };
            // Bound the borrow of `out` before reassigning it.
            let replaced = replace_bounded(out.as_ref(), needle, with);
            if let Some(s) = replaced {
                out = Cow::Owned(s);
            }
        }
        out
    }
}

/// Process-wide redactor, built once from the environment. Shared by the logger
/// and by [`redact`] so the GUI can scrub a string it is about to show or copy.
fn redactor() -> &'static Redactor {
    static R: OnceLock<Redactor> = OnceLock::new();
    R.get_or_init(Redactor::from_env)
}

/// Scrub `text` with the process-wide [`Redactor`].
pub fn redact(text: &str) -> Cow<'_, str> {
    redactor().apply(text)
}

/// Characters that can be part of a name, for the boundary test below.
///
/// `-` and `_` count, because `alice-backup` is a plausible *different* login
/// and rewriting half of it would be wrong. `.` deliberately does NOT count: a
/// path at the end of a sentence (`... could not open /home/alice.`) and a
/// per-user file (`alice.ini`) both have to redact, and no realistic identifier
/// is split wrongly by treating a dot as a boundary.
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Replace every whole-token occurrence of `needle`, returning `None` when there
/// was none.
///
/// "Whole token" means the characters immediately around the match are not name
/// characters, which is what keeps redaction from doing damage:
///
/// * `/home/alice` must not match inside `/home/alicia` (next char `i`), nor
///   turn `/mnt/backup/home/alice` into `/mnt/backup~` (previous char `p`) - that
///   leftover is caught by the bare-username pass instead, which yields
///   `/mnt/backup/home/<user>`.
/// * a user `lin` must not rewrite `linux` or `cylinder`.
///
/// `/` and `\` are not name characters, so `/home/alice/mods` and
/// `Z:\home\alice\mods` still match at their boundaries.
fn replace_bounded(text: &str, needle: &str, with: &str) -> Option<String> {
    if needle.is_empty() || !text.contains(needle) {
        return None;
    }
    let mut out: Option<String> = None;
    let mut last = 0;
    let mut from = 0;
    while let Some(rel) = text[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let prev_ok = text[..start].chars().next_back().is_none_or(|c| !is_name_char(c));
        let next_ok = text[end..].chars().next().is_none_or(|c| !is_name_char(c));
        if prev_ok && next_ok {
            let buf = out.get_or_insert_with(String::new);
            buf.push_str(&text[last..start]);
            buf.push_str(with);
            last = end;
        }
        // Advance past this match either way; occurrences cannot overlap.
        from = end;
    }
    out.map(|mut buf| {
        buf.push_str(&text[last..]);
        buf
    })
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

/// Seconds since the epoch plus the millisecond within that second.
fn now_parts() -> (i64, u32) {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_millis()),
        // Clock before 1970 (a dead CMOS battery does this). Do not panic in a
        // logger; a wrong timestamp beats a crash.
        Err(e) => (-(e.duration().as_secs() as i64), 0),
    }
}

// `void tzset(void)`, POSIX. libc 0.2 binds `localtime_r` but not `tzset`, and
// POSIX only guarantees `localtime_r` is thread-safe, not that it initialises
// the zone database, so we do it ourselves exactly once.
extern "C" {
    fn tzset();
}

/// The local UTC offset in seconds, from the C library so `/etc/localtime` and
/// `TZ` are honoured, DST included. Returns 0 (i.e. UTC) if the lookup fails.
fn local_utc_offset(now_secs: i64) -> i32 {
    static TZ: Once = Once::new();
    // SAFETY: tzset() takes and returns nothing and is called exactly once,
    // before any localtime_r. localtime_r writes into a `tm` we own and returns
    // a pointer to it or null; we read the struct only on success. `tm` is plain
    // integers plus a pointer, so an all-zero value is a valid initial state.
    unsafe {
        TZ.call_once(|| tzset());
        let t = now_secs as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&t, &mut tm).is_null() {
            0
        } else {
            tm.tm_gmtoff as i32
        }
    }
}

/// `+0200` / `-0530` / `+0000`.
fn format_offset(offset_secs: i32) -> String {
    let sign = if offset_secs < 0 { '-' } else { '+' };
    let a = offset_secs.unsigned_abs();
    format!("{sign}{:02}{:02}", a / 3600, (a % 3600) / 60)
}

/// `YYYY-MM-DD HH:MM:SS` from a count of seconds, plus `.mmm` when `millis` is
/// given. The input is already offset into the target zone by the caller, so
/// this stays a pure function of its arguments and is directly testable.
fn format_civil(secs: i64, millis: Option<u32>) -> String {
    let (y, m, d, hh, mm, ss) = civil_from_unix(secs);
    match millis {
        Some(ms) => format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}.{ms:03}"),
        None => format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}"),
    }
}

/// Split seconds-since-epoch into `(year, month, day, hour, minute, second)`.
///
/// Euclidean division so times before 1970 still land on the right day. The
/// calendar conversion is Howard Hinnant's `civil_from_days` (public domain,
/// "chrono-Compatible Low-Level Date Algorithms"), valid for any year in range
/// and small enough that pulling in a date crate was not worth it.
fn civil_from_unix(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (y, m, d, (sod / 3600) as u32, ((sod % 3600) / 60) as u32, (sod % 60) as u32)
}

/// Days since 1970-01-01 to a civil `(year, month, day)`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01 so leap days fall at the end of the "year".
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // day of era, [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // month shifted so March is 0
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m: u32 = if mp < 10 { (mp + 3) as u32 } else { (mp - 9) as u32 }; // [1, 12]
    // Undo the March-first shift: January and February belong to the next year.
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A throwaway directory that cleans itself up, no external deps (same
    /// pattern the other crates use).
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let d = std::env::temp_dir().join(format!("eidos-log-{}-{}", std::process::id(), n));
            fs::create_dir_all(&d).unwrap();
            TempDir(d)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    // -- redaction ---------------------------------------------------------

    fn r() -> Redactor {
        Redactor::new(Some("/home/alice"), Some("alice"))
    }

    #[test]
    fn redacts_a_path_in_the_middle_of_a_sentence() {
        let s = "eidos play: could not open /home/alice/games/skyrim/Data for reading";
        assert_eq!(
            r().apply(s).as_ref(),
            "eidos play: could not open ~/games/skyrim/Data for reading"
        );
    }

    #[test]
    fn redacts_every_occurrence_on_a_line() {
        let s = "linking /home/alice/mods/a.esp -> /home/alice/instances/sse/overwrite/a.esp";
        assert_eq!(
            r().apply(s).as_ref(),
            "linking ~/mods/a.esp -> ~/instances/sse/overwrite/a.esp"
        );
    }

    #[test]
    fn redacts_a_path_that_is_exactly_the_home_dir() {
        assert_eq!(r().apply("/home/alice").as_ref(), "~");
        assert_eq!(r().apply("home is /home/alice.").as_ref(), "home is ~.");
        // A trailing slash is a boundary, so the shape of the path survives.
        assert_eq!(r().apply("/home/alice/").as_ref(), "~/");
    }

    #[test]
    fn redacts_the_username_as_a_bare_word() {
        assert_eq!(
            r().apply("prefix owned by alice, not by the current user").as_ref(),
            "prefix owned by <user>, not by the current user"
        );
        // Left over by the home pass because the path is not really the home dir.
        assert_eq!(
            r().apply("/mnt/backup/home/alice/mods").as_ref(),
            "/mnt/backup/home/<user>/mods"
        );
    }

    #[test]
    fn does_not_redact_inside_longer_words() {
        let r = Redactor::new(Some("/home/lin"), Some("lin"));
        // A different account whose name merely starts the same.
        assert_eq!(r.apply("/home/linus/mods").as_ref(), "/home/linus/mods");
        // Ordinary words that merely contain the login, and a hyphenated name
        // that is a DIFFERENT login.
        assert_eq!(r.apply("running on linux").as_ref(), "running on linux");
        assert_eq!(r.apply("cylinder head").as_ref(), "cylinder head");
        assert_eq!(r.apply("owner lin-dev").as_ref(), "owner lin-dev");
    }

    #[test]
    fn redacts_wine_style_paths() {
        // What Proton prints once a Linux path has been mapped into the prefix.
        let s = r#"could not load Z:\home\alice\.steam\root\x.dll"#;
        assert_eq!(r().apply(s).as_ref(), r#"could not load Z:~\.steam\root\x.dll"#);
    }

    #[test]
    fn handles_multibyte_neighbours_without_panicking() {
        // Slicing around a match must land on char boundaries, and a punctuation
        // character outside ASCII is still a boundary, not part of a name.
        assert_eq!(
            r().apply("mod «/home/alice/x» déployé").as_ref(),
            "mod «~/x» déployé"
        );
        // Debug-formatted paths arrive quoted; quotes are boundaries too.
        assert_eq!(
            r().apply(r#"open("/home/alice/mods") failed"#).as_ref(),
            r#"open("~/mods") failed"#
        );
    }

    #[test]
    fn leaves_clean_text_borrowed_and_unchanged() {
        let s = "mounted 12 layers in 8 ms";
        assert!(matches!(r().apply(s), Cow::Borrowed(_)));
        assert_eq!(r().apply(s).as_ref(), s);
    }

    #[test]
    fn refuses_dangerous_or_missing_needles() {
        // A home of "/" would turn every path separator into a tilde.
        let root = Redactor::new(Some("/"), None);
        assert_eq!(root.apply("/usr/lib/x").as_ref(), "/usr/lib/x");
        // Nothing configured at all is the identity.
        let empty = Redactor::new(None, None);
        assert_eq!(empty.apply("/home/alice/x").as_ref(), "/home/alice/x");
        // Too short to disambiguate from ordinary text.
        let short = Redactor::new(None, Some("al"));
        assert_eq!(short.apply("al opened the archive").as_ref(), "al opened the archive");
    }

    #[test]
    fn ambiguous_usernames_keep_path_redaction_but_skip_the_bare_pass() {
        let r = Redactor::new(Some("/home/steam"), Some("steam"));
        // The leak that matters is still closed.
        assert_eq!(r.apply("/home/steam/mods").as_ref(), "~/mods");
        // ... without shredding every sentence that mentions Steam.
        assert_eq!(
            r.apply("no steam install found").as_ref(),
            "no steam install found"
        );
    }

    #[test]
    fn from_env_falls_back_to_the_home_basename() {
        // No environment mutation: exercise the same construction rules.
        let r = Redactor::new(Some("/home/bob/"), Some("bob"));
        assert_eq!(r.apply("/home/bob/x").as_ref(), "~/x");
        assert_eq!(r.apply("bob").as_ref(), "<user>");
    }

    // -- rotation ----------------------------------------------------------

    /// Create `n` plausible session files for `slug`, one per minute, oldest
    /// first. Returns their paths in creation order.
    fn seed(dir: &Path, slug: &str, n: usize) -> Vec<PathBuf> {
        let base = 1_700_000_000; // 2023-11-14, an arbitrary fixed instant
        (0..n)
            .map(|i| {
                let name = session_file_name(slug, base + (i as i64) * 60, 1000 + i as u32);
                let p = dir.join(name);
                fs::write(&p, b"x").unwrap();
                p
            })
            .collect()
    }

    #[test]
    fn rotation_keeps_the_newest_and_deletes_the_rest() {
        let t = TempDir::new();
        let files = seed(t.path(), "skyrimse", 8);
        let current = files.last().unwrap();

        let removed = rotate(t.path(), "skyrimse.", 3, current);

        assert_eq!(removed, 5);
        let left = names(t.path());
        assert_eq!(left.len(), 3);
        // The three newest, by the timestamp embedded in the name.
        for keep in &files[5..] {
            assert!(keep.exists(), "{keep:?} should have survived");
        }
        for gone in &files[..5] {
            assert!(!gone.exists(), "{gone:?} should have been rotated out");
        }
    }

    #[test]
    fn rotation_never_deletes_the_file_being_written() {
        let t = TempDir::new();
        let files = seed(t.path(), "sse", 5);
        // The OLDEST name is the current session: what a backwards clock jump
        // (dual boot, bad RTC) looks like from here.
        let current = &files[0];

        rotate(t.path(), "sse.", 2, current);

        assert!(current.exists(), "the live session log must survive rotation");
        assert!(files[4].exists());
        assert!(files[3].exists());
        assert!(!files[2].exists());
    }

    #[test]
    fn rotation_only_touches_its_own_bucket() {
        let t = TempDir::new();
        let mine = seed(t.path(), "skyrim", 4);
        // A similarly named instance, plus files that are not session logs.
        let other = seed(t.path(), "skyrim-2", 4);
        fs::write(t.path().join("skyrim.notes.txt"), b"x").unwrap();

        rotate(t.path(), "skyrim.", 1, mine.last().unwrap());

        assert_eq!(mine.iter().filter(|p| p.exists()).count(), 1);
        assert!(other.iter().all(|p| p.exists()), "another instance was rotated away");
        assert!(t.path().join("skyrim.notes.txt").exists());
    }

    #[test]
    fn rotation_with_keep_zero_still_keeps_the_current_session() {
        let t = TempDir::new();
        let files = seed(t.path(), "fo4", 3);
        rotate(t.path(), "fo4.", 0, files.last().unwrap());
        let live = files[2].file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(names(t.path()), vec![live]);
    }

    #[test]
    fn open_session_creates_a_rotated_private_file() {
        let t = TempDir::new();
        // Pre-fill the bucket so the very first real session already rotates.
        let old = seed(t.path(), "sse", 6);
        let (mut f, path) = open_session(t.path(), "SSE", 3).unwrap();
        f.write_all(b"hello\n").unwrap();

        assert!(path.starts_with(t.path()));
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello\n");
        assert_eq!(names(t.path()).len(), 3);
        assert!(path.exists());
        assert!(old[..4].iter().all(|p| !p.exists()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "session logs must not be world-readable");
        }
    }

    // -- names, paths, time ------------------------------------------------

    #[test]
    fn slugs_are_file_name_safe_and_bucket_safe() {
        assert_eq!(slug("SkyrimSE"), "skyrimse");
        // No dot survives, so the rotation separator stays unambiguous.
        assert_eq!(slug("My Instance/v1.2"), "my-instance-v1-2");
        assert_eq!(slug("../../etc/passwd"), "etc-passwd");
        assert_eq!(slug(""), "eidos");
        assert!(slug(&"x".repeat(200)).len() <= 40);
    }

    #[test]
    fn session_names_sort_chronologically() {
        let a = session_file_name("sse", 1_700_000_000, 10);
        let b = session_file_name("sse", 1_700_000_060, 9);
        assert_eq!(a, "sse.20231114-221320.10.log");
        assert!(a < b, "name order must be time order: {a} vs {b}");
    }

    #[test]
    fn state_dir_prefers_xdg_then_home_then_temp() {
        let xdg = state_dir_from(Some("/x/state".into()), Some("/home/alice".into()));
        assert_eq!(xdg, PathBuf::from("/x/state/eidos"));
        // Empty is treated as unset, as the XDG spec requires.
        let home = state_dir_from(Some("".into()), Some("/home/alice".into()));
        assert_eq!(home, PathBuf::from("/home/alice/.local/state/eidos"));
        let none = state_dir_from(None, None);
        assert!(none.ends_with("eidos"));
    }

    #[test]
    fn civil_conversion_matches_known_instants() {
        assert_eq!(format_civil(0, None), "1970-01-01 00:00:00");
        assert_eq!(format_civil(1_700_000_000, Some(7)), "2023-11-14 22:13:20.007");
        // Leap day, and the last second before one.
        assert_eq!(format_civil(1_709_164_800, None), "2024-02-29 00:00:00");
        assert_eq!(format_civil(1_709_164_799, None), "2024-02-28 23:59:59");
        // Before the epoch: euclidean division must not round towards zero.
        assert_eq!(format_civil(-1, None), "1969-12-31 23:59:59");
    }

    #[test]
    fn offsets_render_with_a_sign_and_four_digits() {
        assert_eq!(format_offset(2 * 3600), "+0200");
        assert_eq!(format_offset(0), "+0000");
        assert_eq!(format_offset(-(5 * 3600 + 30 * 60)), "-0530");
    }

    #[test]
    fn level_thresholds_and_parsing() {
        assert!(Level::Error > Level::Warn && Level::Warn > Level::Info && Level::Info > Level::Debug);
        assert_eq!(Level::parse("WARNING"), Some(Level::Warn));
        assert_eq!(Level::parse(" debug "), Some(Level::Debug));
        assert_eq!(Level::parse("shout"), None);
        assert_eq!(Level::Warn.to_string(), "WARN");
    }
}
