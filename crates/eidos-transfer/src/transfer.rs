//! Driving 7-Zip: the two passes that write a `.eidos` file, and the checks that
//! run before one is opened.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use eidos_instance::Instance;

use crate::manifest::BackupManifest;
use crate::plan::{scrub_meta, Left, Outside, Plan, Why};
use crate::relocate::{relocate, Relocated};
use crate::{
    existing_ancestor, free_bytes, human_bytes, par2, Options, Phase, TransferError, MANIFEST_NAME,
    SCHEMA_VERSION,
};

/// A scratch directory that removes itself, however the function returns.
///
/// Everything staged for a pack is tiny (the manifest, a few cleaned `.meta`
/// files, two list files); what matters is that an early return - a refused
/// preflight, a 7-Zip failure - does not leave it behind in `/tmp` under a name
/// nobody will recognise a month later.
struct Scratch(PathBuf);

impl Scratch {
    fn new(what: &str) -> Result<Scratch, TransferError> {
        use std::os::unix::fs::DirBuilderExt;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let seq = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("eidos-{what}-{}-{seq}", std::process::id()));
            // Clock ticks are not unique. Claim a fresh private directory and
            // never remove an existing candidate, even after PID reuse.
            match fs::DirBuilder::new().mode(0o700).create(&dir) {
                Ok(()) => return Ok(Scratch(dir)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    return Err(TransferError::Io(format!(
                        "could not create a scratch folder: {e}"
                    )))
                }
            }
        }
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// What a pack produced.
#[derive(Debug, Clone)]
pub struct PackReport {
    pub path: PathBuf,
    /// The size of the archive.
    pub bytes: u64,
    /// The size of what went into it.
    pub source_bytes: u64,
    pub entries: usize,
    pub manifest: BackupManifest,
    /// What did not make it into the archive although the user would expect it
    /// to. The file is real and worth keeping, but a non-empty list means the
    /// backup is NOT complete, and both front ends say so (`eidos pack` exits 1).
    pub warnings: Vec<String>,
    /// The PAR2 recovery files written beside the archive. Empty when recovery
    /// was off, par2 is not installed, or writing them failed.
    pub recovery: Vec<PathBuf>,
    /// Why recovery data was asked for and not written. Not in `warnings`: the
    /// archive is complete and checked without it, so this must not turn a
    /// good backup into exit 1.
    pub recovery_warning: Option<String>,
}

/// The one warning naming what the walk skipped for a reason the rules did not
/// anticipate: a symbolic link, a name 7-Zip's list file cannot carry, a folder
/// that could not be read. The deliberate exclusions (the prefix, logs, caches)
/// are not losses and stay out of it.
///
/// Without this, those items lived only in the manifest and the CLI's pre-pack
/// listing, and the report came back clean: a `mods/` symlinked to a second
/// drive was dropped whole while the window said "Packed" and `eidos pack ...
/// && rm -rf <old>` went ahead. An unreadable folder is worse - the pack stages
/// it from an empty scratch folder, so 7-Zip never sees it and never warns.
fn left_out_warning(left: &[Left]) -> Option<String> {
    const SHOWN: usize = 20;
    let lost: Vec<&Left> = left
        .iter()
        .filter(|l| {
            matches!(
                l.why,
                Why::Symlink | Why::NotUtf8 | Why::LineBreak | Why::Unreadable(_)
            )
        })
        .collect();
    if lost.is_empty() {
        return None;
    }
    let mut out = format!(
        "{} item(s) could not be packed and are NOT in the backup:",
        lost.len()
    );
    for l in lost.iter().take(SHOWN) {
        out.push_str(&format!("\n  {} - {}", l.path, l.why));
        // A `downloads/` linked to another drive is usually deliberate, and the
        // walk never follows a link (7-Zip would copy whatever it points at).
        // Leaving it out on request is not a loss, so say how: otherwise a
        // `pack && ...` script fails on every run with no way out given.
        if l.path == "downloads" && l.why == Why::Symlink {
            out.push_str(
                " (to back up without it, pass --no-downloads or untick \
                 \"Include downloads/\" in the window)",
            );
        }
    }
    if lost.len() > SHOWN {
        out.push_str(&format!("\n  ... and {} more", lost.len() - SHOWN));
    }
    Some(out)
}

impl PackReport {
    /// The archive as a percentage of the source.
    ///
    /// `None` below a megabyte, where the answer is about 7-Zip's own header
    /// rather than about compression: a 781-byte instance packs to 1.9 kB, and
    /// reporting "248% of 781 B" tells the reader nothing except that the ratio
    /// was printed without thinking.
    pub fn percent_of_source(&self) -> Option<f64> {
        (self.source_bytes >= 1_000_000)
            .then(|| self.bytes as f64 * 100.0 / self.source_bytes as f64)
    }
}

/// What reading an archive's table of contents established, as opposed to what
/// its manifest claims about itself.
struct Contents {
    entries: usize,
    warnings: Vec<String>,
}

/// What an unpack produced.
#[derive(Debug, Clone)]
pub struct UnpackReport {
    pub root: PathBuf,
    pub manifest: BackupManifest,
    pub entries: usize,
    pub relocated: Relocated,
    /// Tools the backup used that are not on this machine.
    pub missing_tools: Vec<Outside>,
    pub warnings: Vec<String>,
}

/// Whether 7-Zip refused a switch rather than the work.
///
/// `-spd` (literal file names, no wildcards) arrived in 7-Zip 21.01. On an older
/// build the whole command line is rejected, which looks nothing like a real
/// failure and must not be reported as one.
pub(crate) fn looks_like_an_unknown_switch(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("unsupported command")
        || e.contains("incorrect command line")
        || e.contains("unknown switch")
}

/// Whether an archive entry would write outside the folder it is extracted into.
///
/// 7-Zip strips a leading `/` on extraction, so a well-behaved 7-Zip is already
/// safe - but a `.eidos` file is made to be handed to somebody else, and "the
/// archiver probably checks" is not the standard to hold a format to that people
/// will download from a forum.
pub(crate) fn escapes_the_destination(entry: &str) -> bool {
    entry.starts_with('/')
        || entry.starts_with('\\')
        || entry.split(['/', '\\']).any(|c| c == "..")
        // `C:\...`: a Windows drive-absolute path.
        || entry
            .as_bytes()
            .get(1)
            .is_some_and(|&b| b == b':' && entry.as_bytes()[0].is_ascii_alphabetic())
}

/// `<path><suffix>`, without touching the extension the path already has.
fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

/// Throw away a half-built archive AND the scratch file 7-Zip builds it in.
///
/// 7-Zip writes `<archive>.tmp` beside the archive and renames at the end, so a
/// pack that dies leaves a full-size second file - tens of gigabytes - that
/// nothing here was removing and that the next attempt does not reuse.
fn discard_partial(part: &Path) {
    let _ = fs::remove_file(part);
    let _ = fs::remove_file(with_suffix(part, ".tmp"));
}

/// The line of a failed `7z t` worth showing: its first `ERROR:` line, and how
/// many more there were.
///
/// A damaged solid block fails every file in it, one `ERROR: Data Error : <file>`
/// line each, and a hundred of them in a dialog say less than the first one
/// and a count.
fn decisive_line(stderr: &str) -> String {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let errors: Vec<&&str> = lines.iter().filter(|l| l.starts_with("ERROR")).collect();
    let (first, more) = match errors.split_first() {
        Some((first, rest)) => (**first, rest.len()),
        None => (lines.first().copied().unwrap_or("no reason given"), 0),
    };
    if more > 0 {
        format!("{first} (and {more} more)")
    } else {
        first.to_string()
    }
}

/// The list file 7-Zip reads, one name per line - each in quotes.
///
/// Not decoration: 7-Zip TRIMS whitespace off every line it reads, so a mod
/// folder called `" Weapons"` - and Windows-sourced archives really do carry
/// leading spaces - is looked up under the trimmed name, not found, and dropped
/// from the backup with a warning nobody reads as data loss. Measured on 7-Zip
/// 26.02: with a bare line the leading-space file was skipped; with the line
/// quoted it packed.
///
/// A blanket quote is safe: 7-Zip strips exactly ONE surrounding pair and takes
/// the rest literally, so an embedded `"` needs no escaping - also measured,
/// with a file whose name contains one.
fn list_file_body(entries: &[String]) -> Result<String, TransferError> {
    let mut s = String::new();
    for e in entries {
        if e.contains(['\r', '\n']) {
            return Err(TransferError::Refused("A backup path contains a line break and cannot be represented safely in a 7-Zip list file".into()));
        }
        s.push('"');
        s.push_str(e);
        s.push_str("\"\n");
    }
    Ok(s)
}

/// The 7-Zip command line for one add pass at `level`.
///
/// Level 1, the default, is solid in blocks of at most 16 MB (`-ms=16m`). Fully
/// non-solid (`-ms=off`) compressed every file as its own stream, and on game
/// content that was the slow part: measured with 7-Zip 26.04 at level 1, solid
/// 16 MB blocks packed `.dds` files 2.4 times faster and unpacked them 2.2
/// times faster, `.nif` files 4.9 and 3.7 times, into a slightly smaller
/// archive. Pulling one file out still decodes at most its own block, never
/// everything before it. What it costs is damage: one corrupt byte loses every
/// file in that byte's block, up to 16 MB of neighbours, where non-solid lost
/// one file. The manifest goes in by its own pass, so it stays a block of its
/// own and [`Transfer::peek`] still decodes nothing else.
///
/// Every other level keeps `-ms=off`. From level 3 up the LZMA2 chunk (four
/// times the dictionary, 16 MB and more) is already as large as the block, so a
/// block cannot spread over threads and gives up the parallel decode of
/// separate files. At level 5 it packed `.nif` files 1.4 times slower and
/// unpacked `.dds` files 6.8 times slower; only the `.nif` unpack gained, 1.4
/// times. Level 0 stores, and a solid block of stored files saves nothing and
/// would only widen what one bad byte takes.
fn add_args(level: u8, spd: bool, archive: &Path, list: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["a".into(), "-t7z".into()];
    // Level 0 means stored. An explicit method overrides what -mx0 picks, so
    // `-m0=LZMA2 -mx0` was LZMA2 with a 64 KiB dictionary: about as slow as
    // level 1 and a little larger. Without it, -mx0 selects Copy.
    if level > 0 {
        args.push("-m0=LZMA2".into());
    }
    args.push(format!("-mx{level}").into());
    args.push(if level == 1 { "-ms=16m" } else { "-ms=off" }.into());
    args.extend(
        [
            "-mmt=on",
            "-y",
            // Progress on stdout, and nothing else on it, so the percentages
            // are all that has to be parsed.
            "-bso0",
            "-bsp1",
            // The list file is ours and it is UTF-8; saying so means a machine
            // with a Latin-1 locale reads the same names we wrote.
            "-scsUTF-8",
        ]
        .map(OsString::from),
    );
    if spd {
        // Names in a list file are patterns unless this says otherwise, and a
        // mod really can be called `Weapons * Armour`.
        args.push("-spd".into());
    }
    args.push(archive.as_os_str().to_os_string());
    let mut at = OsString::from("@");
    at.push(list.as_os_str());
    args.push(at);
    args
}

fn io(what: &str) -> impl Fn(std::io::Error) -> TransferError + '_ {
    move |e| TransferError::Io(format!("{what}: {e}"))
}

/// One 7-Zip, found once, driving both directions.
///
/// Finding the binary costs up to three fork+exec, and a fork briefly shares
/// every open descriptor - including the `flock` an instance lock holds - so a
/// caller that probes per archive can be refused by its own child. Holding the
/// answer in a value that is created ONCE, before the lock is taken, is what
/// makes that impossible rather than merely unlikely.
pub struct Transfer {
    bin: &'static str,
    /// par2cmdline, found by the same one-time probe, for the same reason.
    par2: Option<String>,
    opt: Options,
}

impl Transfer {
    /// Find 7-Zip, or say which package installs it. par2 is looked for too,
    /// and its absence is not an error.
    pub fn new(opt: Options) -> Result<Transfer, TransferError> {
        Transfer::with_par2(opt, par2::BIN)
    }

    /// [`Transfer::new`], with par2 looked up as `par2_bin` instead of `par2`.
    ///
    /// The tests point it at a path that does not exist, which is the only
    /// honest way to see what a machine without par2 does with recovery files.
    pub fn with_par2(opt: Options, par2_bin: &str) -> Result<Transfer, TransferError> {
        let bin = eidos_sevenzip::find_7z().ok_or(TransferError::NoSevenZip)?;
        let par2 = par2::probe(par2_bin).then(|| par2_bin.to_string());
        Ok(Transfer { bin, par2, opt })
    }

    /// Whether a pack will write recovery data: asked for, and par2 found.
    pub fn writes_recovery(&self) -> bool {
        self.opt.recovery && self.par2.is_some()
    }

    /// The binary being driven, for a log line.
    pub fn binary(&self) -> &'static str {
        self.bin
    }

    /// Add a list of entries to `archive`, from `cwd`.
    ///
    /// 7-Zip stores the paths it is GIVEN, so running it from the instance root
    /// with relative names is what produces an archive whose entries are
    /// instance-relative. The switches themselves are in [`add_args`].
    /// Returns whatever 7-Zip warned about, which for an add means files it
    /// could not read - so the caller can say which ones are not in the backup
    /// instead of throwing away an otherwise complete archive.
    fn add(
        &self,
        archive: &Path,
        cwd: &Path,
        list: &Path,
        spd: bool,
        on_progress: &mut impl FnMut(u8),
    ) -> Result<Option<String>, TransferError> {
        Ok(eidos_sevenzip::run_in_tolerating_warnings(
            self.bin,
            add_args(self.opt.level, spd, archive, list),
            Some(cwd),
            on_progress,
        )?)
    }

    /// The checks a pack would make, without making one.
    ///
    /// `--dry-run` calls this so a preview is a real preview: a destination that
    /// already exists, or a disk that cannot hold the result, is exactly what
    /// somebody asking "what would this do" needs told.
    pub fn check_pack(
        &self,
        inst: &Instance,
        plan: &Plan,
        dest: &Path,
    ) -> Result<(), TransferError> {
        self.preflight_pack(inst, plan, dest)
    }

    /// The checks that must pass before a pack starts, because the alternative
    /// is finding out twenty minutes in.
    fn preflight_pack(
        &self,
        inst: &Instance,
        plan: &Plan,
        dest: &Path,
    ) -> Result<(), TransferError> {
        if !inst.exists() {
            return Err(TransferError::Refused(format!(
                "'{}' is not an instance folder - there is nothing to pack.",
                inst.root.display()
            )));
        }
        let anchor = existing_ancestor(dest);
        let here = fs::canonicalize(&anchor).unwrap_or_else(|_| anchor.clone());
        let root = fs::canonicalize(&inst.root).unwrap_or_else(|_| inst.root.clone());
        if here.starts_with(&root) {
            return Err(TransferError::Refused(format!(
                "The backup would be written inside the instance it is backing up \
                 ({}). Choose a destination outside '{}'.",
                dest.display(),
                inst.root.display()
            )));
        }
        if dest.is_dir() {
            return Err(TransferError::Refused(format!(
                "'{}' is a folder.",
                dest.display()
            )));
        }
        if dest.symlink_metadata().is_ok() && !self.opt.force {
            return Err(TransferError::Refused(format!(
                "'{}' already exists. Pass --force to replace it, or choose another name.",
                dest.display()
            )));
        }
        // A floor, not an estimate: no realistic mod corpus - textures and meshes
        // are already-compressed formats - comes out under a quarter of its size,
        // so less than this free cannot possibly work. Level 0 stores, so there
        // the floor is the whole size. Recovery data adds its share of the
        // archive on top, written after it.
        if let Some(free) = free_bytes(&anchor) {
            let floor = if self.opt.level == 0 {
                plan.bytes
            } else {
                plan.bytes / 4
            };
            let recovery = if self.writes_recovery() {
                floor * par2::REDUNDANCY_PERCENT / 100
            } else {
                0
            };
            if free < floor + recovery {
                // The recovery share is named on its own: it is not compression,
                // and it is the part the user can leave out.
                let extra = if recovery > 0 {
                    format!(
                        ", plus about {} of PAR2 recovery data ({}%; --no-recovery, or \
                         unticking it in the window, leaves it out)",
                        human_bytes(recovery),
                        par2::REDUNDANCY_PERCENT
                    )
                } else {
                    String::new()
                };
                return Err(TransferError::Refused(format!(
                    "Not enough room at '{}': {} free, and {} of mods cannot compress \
                     below about {}{extra}.",
                    anchor.display(),
                    human_bytes(free),
                    human_bytes(plan.bytes),
                    human_bytes(floor)
                )));
            }
        }
        Ok(())
    }

    /// Write the whole instance into one file.
    ///
    /// Two passes into the same archive. The first adds the manifest and the
    /// cleaned `.meta` copies from a scratch folder - it is instant, and having
    /// it FIRST means a mistake in the small, fiddly part is found before the
    /// twenty-minute part starts. The second adds the instance itself, and is
    /// the one whose progress the caller sees.
    ///
    /// The archive is built under a `.part` name and renamed at the end, so an
    /// interrupted pack leaves something that is obviously unfinished rather
    /// than a `.eidos` file that looks complete and is not. Before the rename
    /// it is read back whole (see [`Transfer::seal`]); after it, PAR2 recovery
    /// data is written beside it when par2 is installed and recovery is on.
    pub fn pack(
        &self,
        inst: &Instance,
        plan: &Plan,
        dest: &Path,
        on_progress: &mut impl FnMut(Phase, u8),
    ) -> Result<PackReport, TransferError> {
        self.preflight_pack(inst, plan, dest)?;
        if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(io("could not create the destination folder"))?;
        }

        let scratch = Scratch::new("pack")?;
        let stage = scratch.path().join("stage");
        fs::create_dir_all(&stage).map_err(io("could not stage the manifest"))?;

        let manifest = BackupManifest::describe(inst, plan, &self.opt);
        let mut warnings: Vec<String> = left_out_warning(&plan.left).into_iter().collect();
        fs::write(stage.join(MANIFEST_NAME), manifest.render())
            .map_err(io("could not write the manifest"))?;
        let mut staged = vec![MANIFEST_NAME.to_string()];
        for rel in &plan.scrub {
            let dst = stage.join(rel);
            if let Some(p) = dst.parent() {
                fs::create_dir_all(p).map_err(io("could not stage a download record"))?;
            }
            match fs::read_to_string(inst.root.join(rel)) {
                Ok(text) => {
                    fs::write(&dst, scrub_meta(&text))
                        .map_err(io("could not write a cleaned download record"))?;
                    staged.push(rel.clone());
                }
                // It was readable when the plan was made. Rather than pack a
                // signed download URL after promising to remove it, leave the
                // record out and say so.
                Err(e) => warnings.push(format!(
                    "{rel} could not be re-read to remove its download URL, so it is \
                     not in the backup ({e})."
                )),
            }
        }

        // 7-Zip recursively adds a named directory, even when the plan excluded
        // all its children. Preserve its shape from an empty scratch directory.
        let mut bulk = Vec::new();
        for rel in &plan.entries {
            if inst.root.join(rel).is_dir() {
                fs::create_dir_all(stage.join(rel))
                    .map_err(io("could not stage an empty directory"))?;
                staged.push(rel.clone());
            } else {
                bulk.push(rel.clone());
            }
        }

        let staged_list = scratch.path().join("staged.lst");
        fs::write(&staged_list, list_file_body(&staged)?)
            .map_err(io("could not write the list of staged files"))?;
        let bulk_list = scratch.path().join("bulk.lst");
        fs::write(&bulk_list, list_file_body(&bulk)?)
            .map_err(io("could not write the list of files to pack"))?;

        let part = with_suffix(dest, ".part");
        discard_partial(&part);

        // The first pass decides whether this 7-Zip understands `-spd`, while
        // failing costs nothing.
        let mut literal_names = true;
        if let Err(first) = self
            .add(&part, &stage, &staged_list, literal_names, &mut |_| {})
            .map(|_| ())
        {
            let TransferError::Archive(ref why) = first else {
                discard_partial(&part);
                return Err(first);
            };
            if !looks_like_an_unknown_switch(why) {
                discard_partial(&part);
                return Err(first);
            }
            if let Some(name) = plan.wildcards.first() {
                discard_partial(&part);
                return Err(TransferError::Refused(format!(
                    "This 7-Zip is too old to pack '{name}' safely: its name contains a \
                     wildcard character, and without the -spd switch (7-Zip 21.01 and \
                     newer) 7-Zip would treat it as a pattern. Upgrade 7-Zip, or rename \
                     the file."
                )));
            }
            discard_partial(&part);
            literal_names = false;
            self.add(&part, &stage, &staged_list, literal_names, &mut |_| {})?;
        }

        if !bulk.is_empty() {
            match self.add(&part, &inst.root, &bulk_list, literal_names, &mut |p| {
                on_progress(Phase::Archive, p)
            }) {
                Ok(None) => {}
                // 7-Zip could not read something and packed everything else. The
                // archive is real and worth keeping; what it is missing is worth
                // saying loudly.
                Ok(Some(said)) => warnings.push(format!(
                    "7-Zip could not read everything, so the backup is missing some \
                     files. It said:\n{said}"
                )),
                Err(e) => {
                    discard_partial(&part);
                    return Err(e);
                }
            }
        }

        self.seal(&part, dest, on_progress)?;
        let bytes = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);

        let (mut recovery, mut recovery_warning) = (Vec::new(), None);
        if let (true, Some(bin)) = (self.opt.recovery, &self.par2) {
            on_progress(Phase::Recovery, 0);
            match par2::create(bin, dest) {
                Ok(files) => recovery = files,
                Err(why) => {
                    recovery_warning = Some(format!(
                        "The backup is complete and checked, but par2 could not write \
                         recovery data for it ({why}). Nothing partial was left beside it."
                    ))
                }
            }
        }
        Ok(PackReport {
            path: dest.to_path_buf(),
            bytes,
            source_bytes: plan.bytes,
            entries: plan.total_entries(),
            manifest,
            warnings,
            recovery,
            recovery_warning,
        })
    }

    /// Make a finished `.part` the backup: flush it, read it back, rename it.
    ///
    /// 7-Zip exiting 0 says it wrote what it compressed, not that the disk now
    /// holds it. `7z t` decodes every block and checks every CRC, which is the
    /// same work an unpack does on the other machine, so a backup that would
    /// fail there fails here instead, while the instance it came from is still
    /// on this disk. A failure removes the `.part` and the destination is left
    /// as it was: no file ever gets the final name without passing the check.
    ///
    /// The fsync before the check is what makes a power cut after the rename
    /// leave either the old file or the whole new one. After it the written
    /// pages are clean but still cached, and `7z t` would decode memory rather
    /// than the disk, so they are dropped first and the check reads what the
    /// device returns. That is a hint the kernel may ignore (FUSE and network
    /// filesystems can), and a drive's own cache can still answer some reads,
    /// so the check proves what 7-Zip wrote and what the disk gave back once,
    /// not that the disk will keep it.
    fn seal(
        &self,
        part: &Path,
        dest: &Path,
        on_progress: &mut impl FnMut(Phase, u8),
    ) -> Result<(), TransferError> {
        let flushed = fs::File::open(part).and_then(|f| {
            f.sync_all()?;
            use std::os::fd::AsRawFd;
            // SAFETY: a valid open descriptor; the call only advises the
            // kernel about its cached pages, and its result is not needed.
            unsafe { libc::posix_fadvise(f.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
            Ok(())
        });
        if let Err(e) = flushed {
            discard_partial(part);
            return Err(TransferError::Io(format!(
                "could not flush the finished backup to disk: {e}. Nothing was kept."
            )));
        }
        if let Err(said) = self.test_archive(part, &mut |p| on_progress(Phase::Check, p)) {
            discard_partial(part);
            return Err(TransferError::Io(format!(
                "The backup was written but failed its check when 7-Zip read it back: \
                 {said}. The unfinished file was removed and nothing was written to '{}'. \
                 A freshly written archive that reads back wrong usually means the disk \
                 or the memory of this machine; pack again, to another disk if it \
                 happens twice.",
                dest.display()
            )));
        }
        // Recovery files left from the file this one replaces describe another
        // archive, and an unpack would have to read this one twice to find out
        // they are of no use. Removed BEFORE the rename, with any half-written
        // set, so no moment exists where they sit beside the new file.
        par2::remove(dest);
        fs::rename(part, dest).map_err(io("could not put the finished backup in place"))?;
        // The rename is only durable once the directory is. Best effort: some
        // filesystems refuse to fsync a directory, and the file itself is safe.
        let dir = match dest.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let _ = fs::File::open(dir).and_then(|d| d.sync_all());
        Ok(())
    }

    /// `7z t`: decode every block of `archive` and check every CRC. Reported
    /// as [`Phase::Check`], starting at 0. The error is 7-Zip's decisive line.
    fn test_archive(&self, archive: &Path, on_progress: &mut impl FnMut(u8)) -> Result<(), String> {
        on_progress(0);
        let args: Vec<OsString> = vec![
            "t".into(),
            "-bso0".into(),
            "-bsp1".into(),
            archive.as_os_str().to_os_string(),
        ];
        eidos_sevenzip::run(self.bin, args, on_progress).map_err(|e| match e {
            eidos_sevenzip::SevenZipError::Failed(s) => decisive_line(&s),
            other => other.to_string(),
        })
    }

    /// Whether `archive` has recovery data beside it that this machine can
    /// use, so a backup that cannot be read is worth repairing before giving up.
    pub fn can_repair(&self, archive: &Path) -> bool {
        self.par2.is_some() && par2::index(archive).is_file()
    }

    /// Check `archive` against its recovery data and repair it if it needs it,
    /// as [`Transfer::unpack`] does before extracting. Returns the sentence to
    /// show, if any.
    ///
    /// For a front end whose [`Transfer::peek`] failed where
    /// [`Transfer::can_repair`] says yes: 7-Zip keeps its table of contents at
    /// the end of the file, where an interrupted copy cuts it short, and then
    /// not even the manifest can be read until the file is repaired.
    pub fn repair_from_recovery(
        &self,
        archive: &Path,
        on_progress: &mut impl FnMut(Phase, u8),
    ) -> Result<Option<String>, TransferError> {
        self.check_recovery(archive, on_progress)
            .map(|(note, _)| note)
    }

    /// Read a backup's manifest without extracting it.
    ///
    /// This is what tells an Eidos backup from any other archive, so its failure
    /// message is the one a user sees when they point `unpack` at the wrong file.
    ///
    /// It never runs par2: a peek is the 39 ms read a dialog makes the moment a
    /// file is chosen, and a PAR2 verify reads the whole archive. When the read
    /// fails and recovery files sit beside the archive, it says how to use them.
    pub fn peek(&self, archive: &Path) -> Result<BackupManifest, TransferError> {
        if archive.symlink_metadata().is_err() {
            return Err(TransferError::Refused(format!(
                "'{}' does not exist.",
                archive.display()
            )));
        }
        let scratch = Scratch::new("peek")?;
        let mut out = OsString::from("-o");
        out.push(scratch.path().as_os_str());
        let args: Vec<OsString> = vec![
            "x".into(),
            "-y".into(),
            out,
            "-bso0".into(),
            "-bsp1".into(),
            archive.as_os_str().to_os_string(),
            MANIFEST_NAME.into(),
        ];
        // 7-Zip's exit code cannot tell "no such entry" from "this is not an
        // archive", so the answer is whether the file arrived, and 7-Zip's own
        // words are kept only to explain the case where it did not.
        let failed = eidos_sevenzip::run(self.bin, args, &mut |_| {}).err();
        match fs::read_to_string(scratch.path().join(MANIFEST_NAME)) {
            Ok(text) => BackupManifest::parse(&text).ok_or_else(|| {
                TransferError::Refused(format!(
                    "'{}' carries an {MANIFEST_NAME} that does not name a game, so Eidos \
                     cannot tell what it is a backup of.",
                    archive.display()
                ))
            }),
            Err(_) => Err(TransferError::Refused(match failed {
                Some(why) if self.can_repair(archive) => format!(
                    "'{}' could not be read as an Eidos backup: {why}\nRecovery data sits \
                     beside it, so unpacking it will first try to repair it from that.",
                    archive.display()
                ),
                Some(why) if par2::index(archive).is_file() => format!(
                    "'{}' could not be read as an Eidos backup: {why}\nRecovery data sits \
                     beside it, but par2 is not installed. Install par2cmdline (par2 on \
                     Debian and Ubuntu) and try again: unpacking will then try to repair it.",
                    archive.display()
                ),
                Some(why) => format!(
                    "'{}' could not be read as an Eidos backup: {why}",
                    archive.display()
                ),
                None => format!(
                    "'{}' is an archive, but not an Eidos backup: it has no {MANIFEST_NAME}. \
                     Extract it by hand if you know what is in it.",
                    archive.display()
                ),
            })),
        }
    }

    /// What `preflight_unpack` learned by reading the archive rather than by
    /// The checks that must pass before anything is written to `dest`.
    fn preflight_unpack(
        &self,
        archive: &Path,
        manifest: &BackupManifest,
        dest: &Path,
    ) -> Result<Contents, TransferError> {
        if manifest.schema_version > SCHEMA_VERSION {
            return Err(TransferError::Refused(format!(
                "'{}' was made by a newer Eidos (backup format {}, this build understands \
                 {SCHEMA_VERSION}). Update Eidos and try again.",
                archive.display(),
                manifest.schema_version
            )));
        }
        // The archive's own table of contents, not the manifest's summary of it.
        // The manifest is a claim BY the file about itself; these are the entries
        // that will actually be written, which is what both the safety check and
        // the free-space answer have to be about.
        let entries = eidos_sevenzip::list_entries(self.bin, archive)?;
        if let Some(bad) = entries.iter().find(|e| escapes_the_destination(&e.path)) {
            return Err(TransferError::Refused(format!(
                "'{}' contains an entry that would be written OUTSIDE the folder you \
                 chose ('{}'). Eidos will not unpack it.",
                archive.display(),
                bad.path
            )));
        }
        // Lexically safe members can still escape through an existing destination
        // link. Validate every ancestor and leaf before extracting any payload.
        // Each directory once: thousands of entries share their parents, dest
        // and dest's own ancestors, and re-checking them per entry was about
        // ten lstat calls per entry, 5M and ~4 s for a 500k-entry backup. A
        // parent already in the set had its own ancestors checked when it went
        // in, so the walk up stops there.
        let mut checked_dirs = std::collections::HashSet::new();
        for entry in &entries {
            let leaf = dest.join(&entry.path);
            for (depth, path) in leaf.ancestors().enumerate() {
                if depth > 0 && checked_dirs.contains(path) {
                    break;
                }
                match fs::symlink_metadata(path) {
                    Ok(metadata) if metadata.is_symlink() => {
                        return Err(TransferError::Refused(format!(
                            "The restore path '{}' is a symbolic link. Choose a destination without linked paths.",
                            path.display()
                        )));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(TransferError::Io(format!(
                            "could not inspect restore path '{}': {error}",
                            path.display()
                        )))
                    }
                }
                if depth > 0 {
                    checked_dirs.insert(path.to_path_buf());
                }
            }
        }
        let mut warnings = Vec::new();
        if !entries.iter().any(|e| e.path == "eidos-instance.ini") {
            warnings.push(
                "The backup has no eidos-instance.ini, so the unpacked folder will not \
                 describe its own game. Open it once through the GUI wizard to adopt it."
                    .to_string(),
            );
        }
        let bytes: u64 = entries.iter().map(|e| e.size).sum();
        let mut overwriting = false;
        if dest.symlink_metadata().is_ok() {
            if !dest.is_dir() {
                return Err(TransferError::Refused(format!(
                    "'{}' exists and is not a folder.",
                    dest.display()
                )));
            }
            // The lock file is not content. A caller that took the destination's
            // lock before asking - which is how `eidos unpack` stops a --force
            // from fighting an open GUI - would otherwise be told the folder it
            // just locked is not empty.
            let occupied = fs::read_dir(dest)
                .map(|d| {
                    d.flatten()
                        .any(|e| e.file_name().to_string_lossy() != ".eidos.lock")
                })
                .unwrap_or(false);
            if occupied && !self.opt.force {
                return Err(TransferError::Refused(format!(
                    "'{}' is not empty. Unpacking into it would merge the backup with \
                     whatever is already there. Choose an empty folder, or pass --force \
                     if you mean to overwrite it.",
                    dest.display()
                )));
            }
            overwriting = occupied;
        }
        if let Some(free) = free_bytes(&existing_ancestor(dest)) {
            // Not when replacing what is already there: a restore over the same
            // instance frees most of what it needs as it goes, so demanding the
            // whole size again would refuse the one case where the disk is
            // fullest - somebody putting a backup back where it came from.
            if free < bytes && !overwriting {
                return Err(TransferError::Refused(format!(
                    "Not enough room at '{}': {} free, and the backup unpacks to {}.",
                    dest.display(),
                    human_bytes(free),
                    human_bytes(bytes)
                )));
            }
            if free < bytes {
                warnings.push(format!(
                    "Only {} is free at '{}' and the backup unpacks to {}. It should fit \
                     because it is replacing what is there, but it is close.",
                    human_bytes(free),
                    dest.display(),
                    human_bytes(bytes)
                ));
            }
        }
        Ok(Contents {
            entries: entries.len(),
            warnings,
        })
    }

    /// Check `archive` against its PAR2 recovery data, repairing it if it can.
    ///
    /// Runs after the preflight when the archive opens, so a refusal that
    /// costs nothing (a folder that is not empty) is not made to wait for a
    /// verify that reads the whole archive.
    ///
    /// Returns the sentence the report should carry, if any, and whether the
    /// archive was repaired. Damage par2 cannot repair stops the unpack here,
    /// before a byte is extracted. Every other way par2 can fail to answer is
    /// a warning and the extraction goes ahead, because 7-Zip checks every
    /// file's CRC as it extracts and refuses a damaged one on its own.
    fn check_recovery(
        &self,
        archive: &Path,
        on_progress: &mut impl FnMut(Phase, u8),
    ) -> Result<(Option<String>, bool), TransferError> {
        let index = par2::index(archive);
        if !index.is_file() {
            return Ok((None, false));
        }
        let Some(bin) = &self.par2 else {
            return Ok((
                Some(format!(
                    "Recovery data sits beside the backup ({}), but par2 is not installed, so \
                     the backup was not checked against it. Install par2cmdline (par2 on \
                     Debian and Ubuntu) to use it next time.",
                    index.display()
                )),
                false,
            ));
        };
        on_progress(Phase::Verify, 0);
        let beyond = match par2::verify(bin, archive) {
            par2::Verdict::Intact => return Ok((None, false)),
            par2::Verdict::Unknown(why) => {
                return Ok((
                    Some(format!(
                        "par2 could not use the recovery data beside the backup ({why}), so \
                         the backup was not checked against it. 7-Zip still checked every \
                         file as it extracted."
                    )),
                    false,
                ))
            }
            par2::Verdict::Unrepairable(why) => Some(why),
            par2::Verdict::Repairable => None,
        };
        // par2 looks for its blocks anywhere in the file, so a set left beside
        // a newer backup of the same name finds most of them in it and calls
        // it repairable: a "repair" would then rebuild the OLD backup over the
        // new one. A renamed backup reads as wholly missing. So par2's verdict
        // is only believed once 7-Zip's own CRCs say the archive is damaged.
        if self
            .test_archive(archive, &mut |p| on_progress(Phase::Check, p))
            .is_ok()
        {
            return Ok((
                Some(format!(
                    "The recovery files beside the backup ({}) do not match it, and 7-Zip \
                     read the whole backup and found it intact, so they were not used. They \
                     belong to another file: an older backup under the same name, or this \
                     one under the name it had when it was packed. Delete them, or move \
                     them to the file they belong to.",
                    index.display()
                )),
                false,
            ));
        }
        if let Some(why) = beyond {
            return Err(TransferError::Refused(format!(
                "'{}' is damaged beyond what its recovery data can repair: {why}. Nothing \
                 was extracted. If the backup or its recovery files were renamed after \
                 packing, give them back their original names; if the recovery files \
                 belong to another version of this backup, move them away. Then unpack \
                 again.",
                archive.display()
            )));
        }
        // par2 renames the damaged file and writes a whole repaired copy beside
        // it, so a disk without room for a second copy fails halfway. Asked
        // first, while nothing has been touched.
        let size = fs::metadata(archive).map(|m| m.len()).unwrap_or(0);
        let dir = match archive.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        if let Some(free) = free_bytes(dir).filter(|&f| f < size) {
            return Err(TransferError::Refused(format!(
                "'{}' is damaged and its recovery data can repair it, but a repair writes \
                 a repaired copy beside it, which needs {} free there, and only {} is. \
                 Nothing was changed. Copy the .eidos file and its .par2 files to a disk \
                 with room and unpack from there.",
                archive.display(),
                human_bytes(size),
                human_bytes(free)
            )));
        }
        on_progress(Phase::Repair, 0);
        match par2::repair(bin, archive) {
            Ok(set_aside) => Ok((
                Some(format!(
                    "The backup was damaged and has been repaired from its recovery data \
                     before unpacking.{} The disk it was stored on may be failing.",
                    set_aside
                        .map(|p| format!(
                            " par2 kept the damaged copy as '{}'; delete it once you have \
                             checked the restore.",
                            p.display()
                        ))
                        .unwrap_or_default()
                )),
                true,
            )),
            Err(why) => {
                let left = if archive.is_file() {
                    "The backup file was left as it was."
                } else {
                    "par2 moved the damaged file to a numbered name beside it ('.1' or the \
                     next free number), and it could not be moved back."
                };
                Err(TransferError::Refused(format!(
                    "'{}' is damaged and par2 could not repair it: {why}. Nothing was \
                     extracted. {left} If the disk is full or read-only, copy the .eidos \
                     file and its .par2 files to a disk with room and unpack from there.",
                    archive.display()
                )))
            }
        }
    }

    /// Put a backup back, and repair what a plain copy would have broken.
    ///
    /// When recovery files sit beside the archive, the archive is checked
    /// against them before anything is extracted, and repaired if it needs it
    /// (see [`Transfer::check_recovery`]).
    pub fn unpack(
        &self,
        archive: &Path,
        dest: &Path,
        on_progress: &mut impl FnMut(Phase, u8),
    ) -> Result<UnpackReport, TransferError> {
        // 7-Zip keeps its table of contents at the END of the file, where an
        // interrupted copy cuts it short, and then not even the manifest opens.
        // Recovery data repairs that end as well as any other part.
        let mut early = None;
        let mut manifest = match self.peek(archive) {
            Ok(m) => m,
            Err(_) if self.can_repair(archive) => {
                early = Some(self.check_recovery(archive, on_progress)?.0);
                self.peek(archive)?
            }
            Err(e) => return Err(e),
        };
        let mut contents = self.preflight_unpack(archive, &manifest, dest)?;
        let note = match early {
            Some(note) => note,
            None => {
                let (note, repaired) = self.check_recovery(archive, on_progress)?;
                if repaired {
                    // The preflight read the damaged file. The entries, the
                    // escape and symlink checks and the free space have to be
                    // about the file that will be extracted, and the repaired
                    // copy now takes room on the disk too.
                    manifest = self.peek(archive)?;
                    contents = self.preflight_unpack(archive, &manifest, dest)?;
                }
                note
            }
        };
        let mut warnings = contents.warnings;
        warnings.extend(note);
        fs::create_dir_all(dest).map_err(io("could not create the destination folder"))?;

        let mut out = OsString::from("-o");
        out.push(dest.as_os_str());
        let args: Vec<OsString> = vec![
            "x".into(),
            "-y".into(),
            out,
            "-bso0".into(),
            "-bsp1".into(),
            archive.as_os_str().to_os_string(),
        ];
        eidos_sevenzip::run(self.bin, args, &mut |p| on_progress(Phase::Archive, p))?;

        // The instance now lives here, and `kind` has to agree: a backup of a
        // central instance unpacked into a folder of the user's choosing IS a
        // portable one, and an instance that says otherwise sends every command
        // looking for it in the central instance folder instead.
        let inst = Instance::portable(dest.to_path_buf());
        if let Some(mut m) = inst.read_manifest() {
            let central = Instance::global(&m.game_id).root;
            let kind = if fs::canonicalize(dest).unwrap_or_else(|_| dest.to_path_buf())
                == fs::canonicalize(&central).unwrap_or(central)
            {
                eidos_instance::InstanceKind::Global
            } else {
                eidos_instance::InstanceKind::Portable
            };
            if m.kind != kind {
                m.kind = kind;
                if let Err(e) = m.write(&inst.manifest_path()) {
                    warnings.push(format!(
                        "Could not record where this instance now lives: {e}"
                    ));
                }
            }
        }

        let relocated = relocate(dest, &manifest.source_root)
            .map_err(io("could not repair the instance's own paths"))?;
        for (path, why) in &relocated.problems {
            warnings.push(format!("{path} still names the old machine: it {why}."));
        }
        let missing_tools: Vec<Outside> = manifest
            .tools_outside
            .iter()
            .filter(|t| !Path::new(&t.exe).exists())
            .cloned()
            .collect();

        Ok(UnpackReport {
            root: dest.to_path_buf(),
            manifest,
            // Counted in the archive, not read off the manifest: the two differ
            // whenever a pack lost a file, and the number the user is shown
            // should be what arrived.
            entries: contents.entries,
            relocated,
            missing_tools,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_allocations_keep_live_payloads_separate() {
        let first = Scratch::new("isolation").unwrap();
        let sentinel = first.path().join("payload");
        fs::write(&sentinel, b"first transfer").unwrap();
        let second = Scratch::new("isolation").unwrap();
        assert_ne!(first.path(), second.path());
        drop(second);
        assert_eq!(fs::read(&sentinel).unwrap(), b"first transfer");
        let path = first.path().to_path_buf();
        drop(first);
        assert!(!path.exists());
    }

    #[test]
    fn an_entry_that_would_write_outside_the_destination_is_recognised() {
        for bad in [
            "/etc/passwd",
            "../../../etc/passwd",
            "mods/../../escape",
            "\\windows\\system32",
            "mods\\..\\..\\escape",
            "C:/Windows/system32",
            "C:\\Windows",
        ] {
            assert!(escapes_the_destination(bad), "{bad} should be refused");
        }
        for good in [
            "mods/A/Textures/x.dds",
            "eidos-backup.ini",
            "mods/..weird name/x",
            "mods/a..b/x",
            "profiles/Default/plugins.txt",
            "mods/[Rudolph] Dark Souls/x",
        ] {
            assert!(!escapes_the_destination(good), "{good} should be allowed");
        }
    }

    #[test]
    fn a_rejected_switch_is_told_apart_from_a_rejected_job() {
        assert!(looks_like_an_unknown_switch("Unsupported command: -spd"));
        assert!(looks_like_an_unknown_switch("\nIncorrect command line\n"));
        assert!(!looks_like_an_unknown_switch(
            "ERROR: Can not open output file : No space left on device"
        ));
        assert!(!looks_like_an_unknown_switch("Data error in 'mods/x.dds'"));
    }

    #[test]
    fn the_part_suffix_is_added_rather_than_replacing_the_extension() {
        // `with_extension` would turn `backup.eidos` into `backup.part`, which is
        // a different file - and after a rename, a `.eidos` that never existed.
        assert_eq!(
            with_suffix(Path::new("/x/backup.eidos"), ".part"),
            PathBuf::from("/x/backup.eidos.part")
        );
        assert_eq!(
            with_suffix(Path::new("/x/noext"), ".part"),
            PathBuf::from("/x/noext.part")
        );
    }

    #[test]
    fn a_list_file_is_one_quoted_entry_per_line_with_no_leading_dot_slash() {
        // Two defects in one line. A list built from `find .` gives every line a
        // `./` prefix, and 7-Zip then sees `./x` and `x` as two names for one
        // file and refuses the whole archive with "Duplicate filename on disk".
        // And 7-Zip TRIMS each line, so an unquoted ` Weapons` is looked up
        // trimmed, not found, and dropped from the backup.
        let body = list_file_body(&[
            "mods/A/x.dds".into(),
            "empty dir".into(),
            " leading space".into(),
            "trailing space ".into(),
        ])
        .unwrap();
        assert_eq!(
            body,
            "\"mods/A/x.dds\"\n\"empty dir\"\n\" leading space\"\n\"trailing space \"\n"
        );
        assert!(!body.contains("./"));
        // 7-Zip strips exactly one surrounding pair and takes the rest
        // literally, so a name containing a quote needs no escaping.
        assert_eq!(
            list_file_body(&["has\"quote".into()]).unwrap(),
            "\"has\"quote\"\n"
        );
        assert_eq!(list_file_body(&[]).unwrap(), "");
    }

    #[test]
    fn a_caller_supplied_list_cannot_inject_another_entry() {
        for name in ["a\nb", "a\rb", "x\"\n\"../private"] {
            assert!(list_file_body(&[name.into()]).is_err());
        }
    }

    #[test]
    fn only_unplanned_losses_make_the_backup_incomplete() {
        let left = |path: &str, why: Why| Left {
            path: path.into(),
            why,
        };
        // A stock instance: every exclusion is deliberate, nothing is lost.
        assert_eq!(
            left_out_warning(&[left("logs", Why::Local), left("loot", Why::Refetchable)]),
            None
        );
        let w = left_out_warning(&[
            left("logs", Why::Local),
            left("mods", Why::Symlink),
            left("mods/A/locked", Why::Unreadable("Permission denied".into())),
        ])
        .expect("a symlinked mods/ is a loss");
        assert!(w.starts_with("2 item(s)"), "{w}");
        assert!(w.contains("mods - a symbolic link"), "{w}");
        assert!(w.contains("mods/A/locked - unreadable"), "{w}");
        assert!(!w.contains("logs"), "{w}");
        assert!(!w.contains("--no-downloads"), "{w}");
        let w = left_out_warning(&[left("downloads", Why::Symlink)]).unwrap();
        assert!(
            w.contains("downloads - a symbolic link (to back up without it, pass --no-downloads"),
            "{w}"
        );
    }

    #[test]
    fn only_level_one_packs_in_16_mb_solid_blocks() {
        let args = |level, spd| -> Vec<String> {
            add_args(
                level,
                spd,
                Path::new("/x/b.eidos.part"),
                Path::new("/s/bulk.lst"),
            )
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
        };
        for (level, solid) in [
            (0, "-ms=off"),
            (1, "-ms=16m"),
            (3, "-ms=off"),
            (5, "-ms=off"),
            (7, "-ms=off"),
            (9, "-ms=off"),
        ] {
            let a = args(level, true);
            let switches: Vec<&String> = a.iter().filter(|s| s.starts_with("-ms=")).collect();
            assert_eq!(switches, [solid], "level {level}: {a:?}");
            assert!(a.contains(&format!("-mx{level}")), "{a:?}");
            if level > 0 {
                assert!(a.contains(&"-m0=LZMA2".to_string()), "{a:?}");
            }
        }
        let stored = args(0, true);
        assert!(!stored.iter().any(|a| a.starts_with("-m0=")), "{stored:?}");
        assert!(stored.contains(&"-mx0".to_string()), "{stored:?}");
        // The archive and the list file come last, in that order, and -spd
        // only when asked for.
        assert_eq!(
            &stored[stored.len() - 3..],
            ["-spd", "/x/b.eidos.part", "@/s/bulk.lst"]
        );
        assert!(!args(1, false).contains(&"-spd".to_string()));
    }

    #[test]
    fn the_first_error_line_of_a_failed_check_is_kept_with_a_count() {
        assert_eq!(
            decisive_line(
                "\nERROR: Data Error : m001.nif\nERROR: Data Error : m002.nif\nERROR: Data Error : m003.nif\n"
            ),
            "ERROR: Data Error : m001.nif (and 2 more)"
        );
        assert_eq!(
            decisive_line("ERROR: CRC Failed : a.dds"),
            "ERROR: CRC Failed : a.dds"
        );
        assert_eq!(decisive_line("  \nsomething else\n"), "something else");
        assert_eq!(decisive_line(""), "no reason given");
    }

    #[test]
    fn a_backup_that_fails_its_read_back_never_gets_the_final_name() {
        if eidos_sevenzip::find_7z().is_none() {
            eprintln!("SKIPPED: no 7-Zip on this machine, so there is nothing to read back.");
            return;
        }
        let base = std::env::temp_dir().join(format!("eidos-seal-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("source");
        fs::create_dir_all(root.join("mods/A")).unwrap();
        fs::write(
            root.join("eidos-instance.ini"),
            "[eidos]\nschema_version=1\ngame_id=skyrimse\nkind=portable\n",
        )
        .unwrap();
        // Incompressible, so the middle of the archive is this file's bytes and
        // flipping one of them is damage 7-Zip has to notice.
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let noise: Vec<u8> = (0..64 * 1024)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        fs::write(root.join("mods/A/x.dds"), &noise).unwrap();
        let inst = Instance::portable(root);
        let opt = Options {
            recovery: false,
            ..Options::default()
        };
        let t = Transfer::new(opt).unwrap();
        let good = base.join("good.eidos");
        t.pack(&inst, &crate::plan(&inst, &opt), &good, &mut |_, _| {})
            .unwrap();

        // The same archive, sealed again: it passes, and is reported as checked.
        let part = base.join("again.eidos.part");
        fs::copy(&good, &part).unwrap();
        let mut phases = Vec::new();
        t.seal(&part, &base.join("again.eidos"), &mut |ph, _| {
            phases.push(ph)
        })
        .unwrap();
        assert!(base.join("again.eidos").is_file() && !part.exists());
        assert!(phases.contains(&Phase::Check), "{phases:?}");

        // One flipped byte: refused, and no file under either name.
        let part = base.join("bad.eidos.part");
        let mut bytes = fs::read(&good).unwrap();
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xFF;
        fs::write(&part, &bytes).unwrap();
        let dest = base.join("bad.eidos");
        let err = t
            .seal(&part, &dest, &mut |_, _| {})
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed its check"), "{err}");
        assert!(err.contains("ERROR"), "7-Zip's own line is kept: {err}");
        assert!(!dest.exists(), "a damaged archive got the final name");
        assert!(!part.exists(), "the damaged .part was left behind");
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_scratch_folder_removes_itself() {
        let path = {
            let s = Scratch::new("test").unwrap();
            let p = s.path().to_path_buf();
            assert!(p.is_dir());
            p
        };
        assert!(!path.exists(), "{} outlived its guard", path.display());
    }
}
