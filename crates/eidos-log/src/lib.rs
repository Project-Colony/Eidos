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
//!   `~/.local/state/Colony/Eidos/logs/` (XDG basedir 0.8 puts logs in the *state*
//!   dir, not data or cache: state is "persists between restarts, not portable,
//!   not precious", which is exactly a log). Appending to one growing file
//!   would force the reader to guess where the interesting run starts.
//! * **Rotation.** Only the last [`DEFAULT_KEEP`] sessions per instance survive,
//!   so an unattended machine cannot fill its home partition with logs. Per
//!   instance is not enough on its own - a bucket nobody writes to again is
//!   never pruned by it - so anything older than [`DEFAULT_MAX_AGE_DAYS`] is
//!   swept too, whatever its bucket, down to a floor of [`DEFAULT_KEEP`] files.
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

/// How old a session log may get before it is swept, regardless of which bucket
/// it belongs to.
///
/// Per-bucket retention alone does not bound the directory: it only ever deletes
/// files sharing the CURRENT run's prefix, so a bucket nobody writes to again -
/// an instance since renamed or deleted, or the per-link buckets `eidos nxm`
/// used to open before it was taught not to - keeps its files forever. One
/// collection's "fetch missing" left one such file per member.
///
/// Thirty days: a log nobody has attached to a bug report in a month is not
/// going to be. The sweep never takes the directory below [`DEFAULT_KEEP`]
/// files in total, so somebody who runs Eidos twice a year still has their last
/// ten sessions.
pub const DEFAULT_MAX_AGE_DAYS: u64 = 30;

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
    /// Sweep any session file in `dir` older than this many days, whatever its
    /// bucket - see [`DEFAULT_MAX_AGE_DAYS`]. `0` disables the sweep.
    pub max_age_days: u64,
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
            max_age_days: DEFAULT_MAX_AGE_DAYS,
            file_level: Level::Debug,
            stderr_level: Level::Info,
            stderr: true,
            version: None,
        };
        if let Some(l) = std::env::var("EIDOS_LOG")
            .ok()
            .and_then(|v| Level::parse(&v))
        {
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

/// `~/.local/state/Colony/Eidos`, or `~/.local/state/Colony/Eidos`.
pub fn state_dir() -> PathBuf {
    state_dir_from(std::env::var_os("XDG_STATE_HOME"), std::env::var_os("HOME"))
}

/// Where session logs live: `~/.local/state/Colony/Eidos/logs`.
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
    // `Colony/Eidos`, the ecosystem's layout - see `eidos_paths`, which owns the
    // rule. Spelled out here rather than delegated because this function takes
    // its environment as arguments precisely so tests need not mutate the
    // process's, and that shape is worth more than sharing four joins.
    let tail = Path::new(eidos_paths::VENDOR).join(eidos_paths::PROGRAM);
    if let Some(x) = xdg.filter(|x| !x.is_empty()) {
        return PathBuf::from(x).join(tail);
    }
    match home.filter(|h| !h.is_empty()) {
        Some(h) => PathBuf::from(h).join(".local/state").join(tail),
        None => std::env::temp_dir().join(tail),
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

/// Every session log on disk, newest first.
///
/// The file name is `<instance>.<timestamp>.<pid>.log`, so a lexicographic sort
/// within one instance is chronological; across instances the timestamp field is
/// still fixed-width, so sorting the whole name by its timestamp is enough
/// without stat()ing anything.
pub fn sessions() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(log_dir()) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "log"))
        .collect();
    // Sort on the TIMESTAMP field, not the whole name: sorting the whole name
    // would group by instance first and interleave the times, so the newest file
    // overall would not be first.
    out.sort_by_key(|p| {
        let stamp = p
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split('.').nth(1))
            .unwrap_or("")
            .to_string();
        std::cmp::Reverse(stamp)
    });
    out
}

/// Split one log line into its level and its message, or `None` for a line that
/// is not a record (a continuation of a multi-line message, or a file written by
/// something else).
///
/// The format is fixed by `emit`: `YYYY-MM-DD HH:MM:SS.mmm LEVEL message`, which
/// puts the level at a known offset. Parsing it here rather than in the reader
/// keeps the two halves of the format in one file.
pub fn parse_line(line: &str) -> Option<(Level, &str)> {
    // 23 characters of timestamp, a space, then the level.
    let rest = line.get(24..)?;
    let (level, msg) = rest.split_once(' ')?;
    Some((Level::parse(level.trim())?, msg.trim_start()))
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
        match open_session(&cfg.dir, &cfg.instance, cfg.keep, cfg.max_age_days) {
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
fn open_session(
    dir: &Path,
    instance: &str,
    keep: usize,
    max_age_days: u64,
) -> std::io::Result<(File, PathBuf)> {
    fs::create_dir_all(dir)?;
    let slug = slug(instance);
    let (secs, _) = now_parts();
    let offset = local_utc_offset(secs);
    let local = secs + i64::from(offset);
    let name = session_file_name(&slug, local, std::process::id());
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

    // Both stamps are local time, so comparing them compares two instants -
    // except across a DST change, where an hour of local time repeats and two
    // files inside it can order backwards. That costs at most an hour of
    // ordering once or twice a year, against a cutoff measured in days.
    //
    // Clamped before the cast, not after: `u64::MAX as i64` is -1, which would
    // put the cutoff TOMORROW and sweep every log on the machine down to the
    // floor. A century is more than any retention policy needs.
    let days = max_age_days.min(365 * 100) as i64;
    let cutoff = (days > 0).then(|| stamp(local - days * 86_400));
    let now = stamp(local);
    rotate(
        dir,
        &Sweep {
            prefix: &format!("{slug}."),
            keep,
            current: &path,
            older_than: cutoff.as_deref(),
            now: &now,
            live: &pid_is_live,
        },
    );
    Ok((file, path))
}

/// `<slug>.<YYYYMMDD-HHMMSS>.<pid>.log`.
///
/// The `.` after the slug is the bucket separator and cannot occur inside a slug
/// (see [`slug`]), so instances `skyrim` and `skyrim-2` never rotate each other
/// away. Timestamp before pid means a plain name sort is a chronological sort.
fn session_file_name(slug: &str, local_secs: i64, pid: u32) -> String {
    format!("{slug}.{}.{pid}.log", stamp(local_secs))
}

/// The `YYYYMMDD-HHMMSS` field of a session file name.
///
/// Fixed width on purpose: two stamps compare lexicographically exactly as the
/// instants they name compare, so both the rotation sort and the age sweep work
/// on names alone - no `stat`, no date arithmetic beyond this one call.
fn stamp(local_secs: i64) -> String {
    let (y, m, d, hh, mm, ss) = civil_from_unix(local_secs);
    format!("{y:04}{m:02}{d:02}-{hh:02}{mm:02}{ss:02}")
}

/// The stamp and pid of a session file name, if the name has that shape.
///
/// `None` for anything else in the directory - a foreign file, or a name from
/// some future format. Retention acts only on what it can positively identify:
/// an unrecognised name is neither deleted nor counted towards the floor. Both
/// halves of that matter - twenty game crash logs dropped into this folder used
/// to satisfy the floor on their own and let the sweep take every real session
/// log underneath them.
fn session_parts(name: &str) -> Option<(&str, u32)> {
    let mut parts = name.strip_suffix(".log")?.split('.');
    let (_slug, stamp, pid) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let pid: u32 = pid.parse().ok()?;
    let (date, time) = stamp.split_once('-')?;
    let shaped = date.len() == 8
        && time.len() == 6
        && date.bytes().chain(time.bytes()).all(|b| b.is_ascii_digit());
    shaped.then_some((stamp, pid))
}

/// Whether `pid` still names a running *Eidos* process.
///
/// Retention must never unlink a log another Eidos process is still writing to,
/// and the pid is right there in the name. Linux only, which is what Eidos
/// targets; anywhere else this says no and retention behaves as it did before
/// the check existed.
///
/// The name check is not decoration. Pids are recycled - Linux wraps at
/// `pid_max`, often 32768 - so "a process with this pid exists" is true for a
/// great many stale logs, and low pids are worse than random: 1 is init and 2
/// upwards are kernel threads, alive on every machine forever. Sparing those
/// files would leave the directory exactly as unbounded as before.
///
/// A pid recycled by ANOTHER Eidos process still answers yes, which spares a
/// file that could have gone. That direction is free: its own bucket's rotation
/// will reach it.
fn pid_is_live(pid: u32) -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    // `comm` is the binary name, truncated to 15 bytes - long enough for both
    // `eidos` and `eidos-gui`.
    fs::read_to_string(Path::new("/proc").join(pid.to_string()).join("comm"))
        .is_ok_and(|c| c.trim_end().starts_with("eidos"))
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

/// What one run's retention pass is allowed to do.
struct Sweep<'a> {
    /// `<slug>.` of the run doing the sweeping.
    prefix: &'a str,
    /// Session files to keep for that bucket, and the floor the age sweep stops at.
    keep: usize,
    /// The file this run is writing to. Never a candidate under either rule.
    current: &'a Path,
    /// Stamps strictly before this are old enough to sweep, whatever the bucket.
    /// `None` disables the age rule entirely.
    older_than: Option<&'a str>,
    /// This run's own stamp. Kept for the header and for tests; the age rule
    /// deliberately does NOT sweep stamps after it - see the filter in
    /// `rotate`, and the test that pins why.
    #[allow(dead_code)]
    now: &'a str,
    /// Whether a pid still names a running process. Injected rather than called
    /// directly so tests can pin it instead of depending on what happens to be
    /// running on the machine.
    live: &'a dyn Fn(u32) -> bool,
}

/// Prune the log directory. Returns how many files were removed.
///
/// Two rules, over ONE listing, and both act only on files this crate wrote -
/// a name [`session_parts`] cannot read is neither deleted nor counted:
///
/// * **This bucket, by count.** Keep the `keep` newest files whose name starts
///   with `prefix`. Per bucket, not global, because launching Skyrim must not
///   delete the Fallout log the user was about to attach to a bug report.
/// * **Everything, by age.** Delete anything stamped outside
///   `older_than..=now`, whatever its bucket, oldest first. This rule exists
///   because the first cannot bound the directory: it only ever touches files
///   sharing the CURRENT run's prefix, so a bucket nobody writes to again keeps
///   its files forever. The sweep stops before the count of session logs would
///   drop below `keep`, so a rare user still keeps their last sessions however
///   old those are.
///
/// Ordering is on the STAMP, not the whole name. The whole name begins with the
/// slug, so once candidates span buckets a name sort is an alphabetical sort:
/// with 250 stale `nxm---*` orphans beside ten real `gui.*` logs, "oldest
/// first" deleted every log of every instance the user actually runs and left
/// the floor filled with junk twenty days older.
///
/// Best effort: a file we cannot delete is not worth failing a launch over.
fn rotate(dir: &Path, s: &Sweep) -> usize {
    let keep = s.keep.max(1);
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    // Only files this crate wrote take part. A foreign `.log` is never deleted,
    // and - the half that bit - never counts towards the floor either: the GUI
    // itself drops a `resume.log` here, and its "Open logs folder" button
    // invites a user to park game crash logs beside it. Files that can never be
    // swept were filling the floor, so the sweep went on deleting real session
    // logs until the count of EVERYTHING reached `keep`.
    let mut sessions: Vec<(PathBuf, String, u32)> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some((st, pid)) = session_parts(name) else {
            continue;
        };
        let st = st.to_string();
        sessions.push((p, st, pid));
    }

    // Never a file another Eidos process is still writing to. Both rules need
    // it: five `eidos nxm` children now share ONE bucket, so past `keep` of them
    // the newest would otherwise unlink the oldest's live log - and a machine
    // whose clock jumps forward after NTP makes an already-open session look
    // ancient to the next process that starts.
    let spared = |p: &Path, pid: u32| p == s.current || (s.live)(pid);

    // Rule one: this bucket, by count. One prefix means the next field is the
    // stamp, so here a whole-name sort IS chronological. `current` takes part in
    // the sort and is then spared, so a backwards clock jump costs a slot rather
    // than the live session.
    let mut mine: Vec<&(PathBuf, String, u32)> = sessions
        .iter()
        .filter(|(p, _, _)| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(s.prefix))
        })
        .collect();
    mine.sort_by(|a, b| b.0.file_name().cmp(&a.0.file_name()));
    let mut doomed: std::collections::HashSet<PathBuf> = mine
        .into_iter()
        .skip(keep)
        .filter(|(p, _, pid)| !spared(p, *pid))
        .map(|(p, _, _)| p.clone())
        .collect();

    // Rule two: every bucket, by age, ordered on the stamp field.
    if let Some(cutoff) = s.older_than {
        let mut aged: Vec<&(PathBuf, String, u32)> = sessions
            .iter()
            .filter(|(p, _, pid)| !spared(p, *pid) && !doomed.contains(p))
            // Older than the cutoff. A stamp AFTER this run was once swept too -
            // the reasoning being a clock set to 2099 - but that is the wrong
            // way round: a clock that steps BACKWARD (an NTP correction, a
            // dual-boot RTC) makes every log written before it look like the
            // future, and the sweep would delete the sessions just written
            // while keeping nothing. A future stamp is bounded by the per-bucket
            // rule instead, which does not care what time it is.
            .filter(|(_, st, _)| st.as_str() < cutoff)
            .collect();
        aged.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| a.0.file_name().cmp(&b.0.file_name()))
        });
        let mut surviving = sessions.len() - doomed.len();
        for (p, _, _) in aged {
            if surviving <= keep {
                break;
            }
            doomed.insert(p.clone());
            surviving -= 1;
        }
    }

    doomed.iter().filter(|p| fs::remove_file(p).is_ok()).count()
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
    "steam",
    "proton",
    "wine",
    "game",
    "games",
    "mod",
    "mods",
    "data",
    "home",
    "user",
    "users",
    "root",
    "log",
    "logs",
    "bin",
    "lib",
    "usr",
    "var",
    "tmp",
    "share",
    "local",
    "state",
    "default",
    "eidos",
    "profile",
    "profiles",
    "save",
    "saves",
    "plugin",
    "plugins",
    "overwrite",
    "instance",
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
        Redactor {
            home,
            home_win,
            user,
        }
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
        let prev_ok = text[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_name_char(c));
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
///
/// Public because the window needs it too - a mod's install date is one of its
/// columns - and two copies of a calendar algorithm is exactly one too many.
pub fn civil_from_unix(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (
        y,
        m,
        d,
        (sod / 3600) as u32,
        ((sod % 3600) / 60) as u32,
        (sod % 60) as u32,
    )
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
    let m: u32 = if mp < 10 {
        (mp + 3) as u32
    } else {
        (mp - 9) as u32
    }; // [1, 12]
       // Undo the March-first shift: January and February belong to the next year.
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    #[test]
    fn a_log_line_splits_into_its_level_and_message() {
        // The exact shape `emit` writes.
        assert_eq!(
            parse_line("2026-08-24 17:04:11.238 INFO  mounted 412 layers"),
            Some((Level::Info, "mounted 412 layers"))
        );
        assert_eq!(
            parse_line("2026-08-24 17:04:11.238 ERROR could not open the prefix"),
            Some((Level::Error, "could not open the prefix"))
        );
        // A continuation line of a multi-line message is NOT a record: reading
        // it as one would invent a level for text that has none.
        assert_eq!(parse_line("    at some/path.rs:12"), None);
        assert_eq!(parse_line(""), None);
        // And a line from some other program in the same folder.
        assert_eq!(parse_line("this is not an eidos log line at all"), None);
        // Multi-byte characters before the offset must not panic (`get` is a
        // char-boundary-checked slice, which is why it is used here).
        assert_eq!(parse_line("héllo"), None);
    }
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
            r().apply("prefix owned by alice, not by the current user")
                .as_ref(),
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
        assert_eq!(
            r().apply(s).as_ref(),
            r#"could not load Z:~\.steam\root\x.dll"#
        );
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
        assert_eq!(
            short.apply("al opened the archive").as_ref(),
            "al opened the archive"
        );
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
    /// A sweep with nothing running: tests must not depend on whether some pid
    /// they invented happens to exist on the machine running them.
    fn sweep<'a>(
        prefix: &'a str,
        keep: usize,
        current: &'a Path,
        older_than: Option<&'a str>,
    ) -> Sweep<'a> {
        Sweep {
            prefix,
            keep,
            current,
            older_than,
            now: "99999999-999999",
            live: &|_| false,
        }
    }

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

        let removed = rotate(t.path(), &sweep("skyrimse.", 3, current, None));

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
    fn an_abandoned_bucket_is_swept_by_age() {
        // The defect this exists for. Per-bucket retention only ever touches
        // files sharing the CURRENT run's prefix, so a bucket nobody writes to
        // again keeps its files forever - and `eidos nxm` used to open a bucket
        // per download LINK, one per mod ever fetched. Nothing on the machine
        // would ever have deleted them.
        let t = TempDir::new();
        let orphans: Vec<PathBuf> = (0..8)
            .map(|i| seed(t.path(), &format!("nxm---skyrim-mods-{i}"), 1).remove(0))
            .collect();
        let live = seed(t.path(), "skyrimse", 4);
        let current = &live[3];

        // Everything above is stamped 2023-11-14; a run a year later sweeps it.
        let cutoff = stamp(1_700_000_000 + 365 * 86_400);
        let removed = rotate(t.path(), &sweep("skyrimse.", 10, current, Some(&cutoff)));

        assert!(current.exists(), "never the live session");
        // Twelve files, a floor of ten: two go, and the sweep stops there.
        assert_eq!(removed, 2, "down to the floor and no further");
        assert_eq!(names(t.path()).len(), 10);
        // Oldest first, so the orphans go before any of the live bucket.
        assert!(!orphans[0].exists() && !orphans[1].exists());
        assert!(orphans[2].exists(), "the floor stopped the sweep here");
        assert!(
            live.iter().all(|p| p.exists()),
            "and never at a live bucket's expense"
        );
    }

    #[test]
    fn the_age_sweep_orders_by_stamp_not_by_bucket_name() {
        // The defect this exists for, measured: the candidate list spans every
        // bucket, so sorting it by whole file name sorts by SLUG. With 250 stale
        // `nxm---*` orphans beside the user's own `gui.*` logs, "oldest first"
        // deleted every log of every instance they actually run and left the
        // floor filled with junk twenty days older.
        let t = TempDir::new();
        let day = 86_400i64;
        let base = 1_700_000_000;
        let mk = |slug: &str, at: i64, pid: u32| {
            let p = t.path().join(session_file_name(slug, at, pid));
            fs::write(&p, b"x").unwrap();
            p
        };
        // `gui` sorts BEFORE `nxm---`, and is the newer of the two by 20 days.
        let gui: Vec<PathBuf> = (0..6)
            .map(|i| mk("gui", base + 20 * day + i, 100 + i as u32))
            .collect();
        let orphans: Vec<PathBuf> = (0..8)
            .map(|i| mk(&format!("nxm---mods-{i}"), base + i, 200 + i as u32))
            .collect();
        let current = mk("gui", base + 400 * day, 999);

        let cutoff = stamp(base + 300 * day);
        let removed = rotate(t.path(), &sweep("gui.", 10, &current, Some(&cutoff)));

        assert_eq!(removed, 5, "fifteen files, a floor of ten");
        assert!(current.exists());
        // The five that went are the five OLDEST - orphans - not the five that
        // happen to sort first.
        assert!(orphans[..5].iter().all(|p| !p.exists()), "the oldest went");
        assert!(
            gui.iter().all(|p| p.exists()),
            "the user's own logs survived"
        );
    }

    #[test]
    fn foreign_logs_do_not_fill_the_retention_floor() {
        // Reachable with no user action: eidos-gui writes a `resume.log` here,
        // and its "Open logs folder" button invites a user to park game crash
        // logs beside it. Counting files the sweep can never delete let the
        // floor be satisfied by them alone, and every real session log went.
        let t = TempDir::new();
        for foreign in ["resume.log", "crash-0.log", "crash-1.log", "notes.log"] {
            fs::write(t.path().join(foreign), b"x").unwrap();
        }
        let old = seed(t.path(), "skyrimse", 11);
        let current = old.last().unwrap();

        let cutoff = stamp(1_700_000_000 + 365 * 86_400);
        let removed = rotate(t.path(), &sweep("skyrimse.", 10, current, Some(&cutoff)));

        assert_eq!(removed, 1, "eleven SESSION logs, a floor of ten");
        assert_eq!(
            names(t.path())
                .iter()
                .filter(|n| n.starts_with("skyrimse."))
                .count(),
            10,
            "the promised ten survived, not six"
        );
        for foreign in ["resume.log", "crash-0.log", "crash-1.log", "notes.log"] {
            assert!(
                t.path().join(foreign).exists(),
                "{foreign} is not ours to delete"
            );
        }
    }

    #[test]
    fn a_log_another_eidos_process_is_writing_is_never_deleted() {
        // Two ways this bites. Five `eidos nxm` children share ONE bucket since
        // the link stopped being its own, so past `keep` of them the newest
        // would unlink the oldest's live file. And a machine whose clock jumps
        // forward after NTP makes an already-open session look ancient to the
        // next process that starts.
        let t = TempDir::new();
        let running = seed(t.path(), "nxm", 6);
        let current = t
            .path()
            .join(session_file_name("nxm", 1_700_000_000 + 999, 4242));
        fs::write(&current, b"x").unwrap();

        // Pretend every seeded pid is a live Eidos process, which is what five
        // concurrent children look like.
        let live = |_: u32| true;
        let now = stamp(1_700_000_000 + 999);
        let cutoff = stamp(1_700_000_000 + 365 * 86_400);
        let removed = rotate(
            t.path(),
            &Sweep {
                prefix: "nxm.",
                keep: 2,
                current: &current,
                older_than: Some(&cutoff),
                now: &now,
                live: &live,
            },
        );

        assert_eq!(removed, 0, "not one line of another run's log");
        assert!(running.iter().all(|p| p.exists()));
    }

    #[test]
    fn a_clock_that_steps_backward_does_not_delete_the_logs_just_written() {
        // The age rule sweeps only what is OLDER than the cutoff, never what is
        // newer than this run. Sweeping the future looks tidy - it would catch a
        // session stamped 2099 by a dead CMOS battery - but it is the wrong way
        // round: a clock stepping BACKWARD (an NTP correction on a machine that
        // booted at epoch, a dual-boot RTC) makes every log written before the
        // step look like the future, and the sweep would then delete the whole
        // history the moment the time was corrected.
        let t = TempDir::new();
        // Written "later" than the run doing the sweeping, which is what a
        // backward step looks like from here.
        let recent = seed(t.path(), "skyrimse", 3);
        let current = t
            .path()
            .join(session_file_name("skyrimse", 1_700_000_000 - 86_400, 4242));
        fs::write(&current, b"x").unwrap();

        let now = stamp(1_700_000_000 - 86_400);
        let removed = rotate(
            t.path(),
            &Sweep {
                prefix: "skyrimse.",
                keep: 10,
                current: &current,
                older_than: Some(&stamp(1_700_000_000 - 400 * 86_400)),
                now: &now,
                live: &|_| false,
            },
        );

        assert_eq!(
            removed, 0,
            "not one line of a session that is merely newer than this one"
        );
        assert!(recent.iter().all(|p| p.exists()));
    }

    #[test]
    fn a_bucket_nobody_writes_to_again_is_still_bounded_by_the_count_rule() {
        // What the future-stamp sweep was for: a log that no age cutoff will
        // ever reach. The per-bucket count rule handles it without caring what
        // time it is - which is why the age rule can afford to leave it alone.
        let t = TempDir::new();
        let mut all = seed(t.path(), "nxm", 12);
        let current = all.pop().unwrap();
        let removed = rotate(
            t.path(),
            &Sweep {
                prefix: "nxm.",
                keep: 3,
                current: &current,
                older_than: None,
                now: "20991231-235959",
                live: &|_| false,
            },
        );
        assert!(removed > 0);
        // Three, and the live one is the newest so it is one of them - `keep`
        // counts it rather than being a floor above it.
        assert_eq!(names(t.path()).len(), 3);
        assert!(current.exists());
    }
    #[test]
    fn an_absurd_max_age_cannot_sweep_everything() {
        // `u64::MAX as i64` is -1, which would put the cutoff TOMORROW and take
        // every log on the machine down to the floor. Clamped before the cast.
        let t = TempDir::new();
        let fresh = seed(t.path(), "skyrimse", 4);
        let (_f, path) = open_session(t.path(), "skyrimse", 10, u64::MAX).unwrap();

        assert!(
            fresh.iter().all(|p| p.exists()),
            "four fresh logs, nothing to sweep"
        );
        assert!(path.exists());
    }

    #[test]
    fn the_age_sweep_never_empties_the_directory() {
        // Somebody who runs Eidos twice a year must still have their last
        // sessions, however old every one of them is.
        let t = TempDir::new();
        let old = seed(t.path(), "skyrimse", 4);
        let current = &old[3];
        let cutoff = stamp(1_700_000_000 + 365 * 86_400);

        assert_eq!(
            rotate(t.path(), &sweep("skyrimse.", 10, current, Some(&cutoff))),
            0
        );
        assert!(
            old.iter().all(|p| p.exists()),
            "all four are older than the cutoff"
        );
    }

    #[test]
    fn the_age_sweep_only_dates_names_it_recognises() {
        let t = TempDir::new();
        let mine = seed(t.path(), "skyrimse", 2);
        // Not session logs, or not shaped like one. The sweep deletes only what
        // it can positively date, so these are left alone rather than guessed at.
        for odd in [
            "notes.log",
            "skyrimse.log",
            "a.b.c.d.log",
            "sse.20231114-.1000.log",
        ] {
            fs::write(t.path().join(odd), b"x").unwrap();
        }
        let cutoff = stamp(1_700_000_000 + 365 * 86_400);
        rotate(t.path(), &sweep("skyrimse.", 1, &mine[1], Some(&cutoff)));

        assert!(t.path().join("notes.log").exists());
        assert!(t.path().join("skyrimse.log").exists());
        assert!(t.path().join("a.b.c.d.log").exists());
        assert!(t.path().join("sse.20231114-.1000.log").exists());

        assert_eq!(
            session_parts("sse.20231114-215544.1000.log"),
            Some(("20231114-215544", 1000))
        );
        assert_eq!(
            session_parts("sse.20231114-215544.pid.log"),
            None,
            "pid must be a number"
        );
        assert_eq!(
            session_parts("sse.2023111-215544.1000.log"),
            None,
            "date is eight digits"
        );
        assert_eq!(
            session_parts("sse.20231114-21554.1000.log"),
            None,
            "time is six"
        );
    }

    #[test]
    fn rotation_never_deletes_the_file_being_written() {
        let t = TempDir::new();
        let files = seed(t.path(), "sse", 5);
        // The OLDEST name is the current session: what a backwards clock jump
        // (dual boot, bad RTC) looks like from here.
        let current = &files[0];

        rotate(t.path(), &sweep("sse.", 2, current, None));

        assert!(
            current.exists(),
            "the live session log must survive rotation"
        );
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

        rotate(t.path(), &sweep("skyrim.", 1, mine.last().unwrap(), None));

        assert_eq!(mine.iter().filter(|p| p.exists()).count(), 1);
        assert!(
            other.iter().all(|p| p.exists()),
            "another instance was rotated away"
        );
        assert!(t.path().join("skyrim.notes.txt").exists());
    }

    #[test]
    fn rotation_with_keep_zero_still_keeps_the_current_session() {
        let t = TempDir::new();
        let files = seed(t.path(), "fo4", 3);
        rotate(t.path(), &sweep("fo4.", 0, files.last().unwrap(), None));
        let live = files[2].file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(names(t.path()), vec![live]);
    }

    #[test]
    fn open_session_creates_a_rotated_private_file() {
        let t = TempDir::new();
        // Pre-fill the bucket so the very first real session already rotates.
        let old = seed(t.path(), "sse", 6);
        let (mut f, path) = open_session(t.path(), "SSE", 3, 0).unwrap();
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
        // `Colony/Eidos` - the ecosystem's layout, so a user finds one tree per
        // program rather than one spelling per crate that needed a directory.
        let xdg = state_dir_from(Some("/x/state".into()), Some("/home/alice".into()));
        assert_eq!(xdg, PathBuf::from("/x/state/Colony/Eidos"));
        // Empty is treated as unset, as the XDG spec requires.
        let home = state_dir_from(Some("".into()), Some("/home/alice".into()));
        assert_eq!(home, PathBuf::from("/home/alice/.local/state/Colony/Eidos"));
        // And with no environment at all it still lands somewhere writable,
        // because a log we cannot write is worse than a log in an odd place.
        let none = state_dir_from(None, None);
        assert!(none.ends_with("Colony/Eidos"), "{}", none.display());
    }

    #[test]
    fn civil_conversion_matches_known_instants() {
        assert_eq!(format_civil(0, None), "1970-01-01 00:00:00");
        assert_eq!(
            format_civil(1_700_000_000, Some(7)),
            "2023-11-14 22:13:20.007"
        );
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
        assert!(
            Level::Error > Level::Warn && Level::Warn > Level::Info && Level::Info > Level::Debug
        );
        assert_eq!(Level::parse("WARNING"), Some(Level::Warn));
        assert_eq!(Level::parse(" debug "), Some(Level::Debug));
        assert_eq!(Level::parse("shout"), None);
        assert_eq!(Level::Warn.to_string(), "WARN");
    }
}
