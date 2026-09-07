//! Driving 7-Zip: the two passes that write a `.eidos` file, and the checks that
//! run before one is opened.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use eidos_instance::Instance;

use crate::manifest::BackupManifest;
use crate::plan::{scrub_meta, Outside, Plan};
use crate::relocate::{relocate, Relocated};
use crate::{
    existing_ancestor, free_bytes, human_bytes, Options, TransferError, MANIFEST_NAME,
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
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "eidos-{what}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).map_err(|e| {
            TransferError::Io(format!("could not create a scratch folder: {e}"))
        })?;
        Ok(Scratch(dir))
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
    /// Things worth saying that are not failures.
    pub warnings: Vec<String>,
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

fn list_file_body(entries: &[String]) -> String {
    let mut s = String::new();
    for e in entries {
        s.push_str(e);
        s.push('\n');
    }
    s
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
    opt: Options,
}

impl Transfer {
    /// Find 7-Zip, or say which package installs it.
    pub fn new(opt: Options) -> Result<Transfer, TransferError> {
        match eidos_sevenzip::find_7z() {
            Some(bin) => Ok(Transfer { bin, opt }),
            None => Err(TransferError::NoSevenZip),
        }
    }

    /// The binary being driven, for a log line.
    pub fn binary(&self) -> &'static str {
        self.bin
    }

    /// Add a list of entries to `archive`, from `cwd`.
    ///
    /// 7-Zip stores the paths it is GIVEN, so running it from the instance root
    /// with relative names is what produces an archive whose entries are
    /// instance-relative. `-ms=off` makes it non-solid: one file can be pulled
    /// out of a 70 GB backup without decompressing everything before it.
    fn add(
        &self,
        archive: &Path,
        cwd: &Path,
        list: &Path,
        spd: bool,
        on_progress: &mut impl FnMut(u8),
    ) -> Result<(), TransferError> {
        let mut args: Vec<OsString> = vec![
            "a".into(),
            "-t7z".into(),
            "-m0=LZMA2".into(),
            format!("-mx{}", self.opt.level).into(),
            "-ms=off".into(),
            "-mmt=on".into(),
            "-y".into(),
            // Progress on stdout, and nothing else on it, so the percentages are
            // all that has to be parsed.
            "-bso0".into(),
            "-bsp1".into(),
            // The list file is ours and it is UTF-8; saying so means a machine
            // with a Latin-1 locale reads the same names we wrote.
            "-scsUTF-8".into(),
        ];
        if spd {
            // Names in a list file are patterns unless this says otherwise, and a
            // mod really can be called `Weapons * Armour`.
            args.push("-spd".into());
        }
        args.push(archive.as_os_str().to_os_string());
        let mut at = OsString::from("@");
        at.push(list.as_os_str());
        args.push(at);
        eidos_sevenzip::run_in(self.bin, args, Some(cwd), on_progress)?;
        Ok(())
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
        // so less than this free cannot possibly work.
        if let Some(free) = free_bytes(&anchor) {
            let floor = plan.bytes / 4;
            if free < floor {
                return Err(TransferError::Refused(format!(
                    "Not enough room at '{}': {} free, and {} of mods cannot compress \
                     below about {}.",
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
    /// than a `.eidos` file that looks complete and is not.
    pub fn pack(
        &self,
        inst: &Instance,
        plan: &Plan,
        dest: &Path,
        on_progress: &mut impl FnMut(u8),
    ) -> Result<PackReport, TransferError> {
        self.preflight_pack(inst, plan, dest)?;
        if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(io("could not create the destination folder"))?;
        }

        let scratch = Scratch::new("pack")?;
        let stage = scratch.path().join("stage");
        fs::create_dir_all(&stage).map_err(io("could not stage the manifest"))?;

        let manifest = BackupManifest::describe(inst, plan, &self.opt);
        let mut warnings = Vec::new();
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

        let staged_list = scratch.path().join("staged.lst");
        fs::write(&staged_list, list_file_body(&staged))
            .map_err(io("could not write the list of staged files"))?;
        let bulk_list = scratch.path().join("bulk.lst");
        fs::write(&bulk_list, list_file_body(&plan.entries))
            .map_err(io("could not write the list of files to pack"))?;

        let part = with_suffix(dest, ".part");
        let _ = fs::remove_file(&part);

        // The first pass decides whether this 7-Zip understands `-spd`, while
        // failing costs nothing.
        let mut literal_names = true;
        if let Err(first) = self.add(&part, &stage, &staged_list, literal_names, &mut |_| {}) {
            let TransferError::Archive(ref why) = first else {
                let _ = fs::remove_file(&part);
                return Err(first);
            };
            if !looks_like_an_unknown_switch(why) {
                let _ = fs::remove_file(&part);
                return Err(first);
            }
            if let Some(name) = plan.wildcards.first() {
                let _ = fs::remove_file(&part);
                return Err(TransferError::Refused(format!(
                    "This 7-Zip is too old to pack '{name}' safely: its name contains a \
                     wildcard character, and without the -spd switch (7-Zip 21.01 and \
                     newer) 7-Zip would treat it as a pattern. Upgrade 7-Zip, or rename \
                     the file."
                )));
            }
            let _ = fs::remove_file(&part);
            literal_names = false;
            self.add(&part, &stage, &staged_list, literal_names, &mut |_| {})?;
        }

        if !plan.entries.is_empty() {
            if let Err(e) = self.add(&part, &inst.root, &bulk_list, literal_names, on_progress) {
                let _ = fs::remove_file(&part);
                return Err(e);
            }
        }

        fs::rename(&part, dest).map_err(io("could not put the finished backup in place"))?;
        let bytes = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
        Ok(PackReport {
            path: dest.to_path_buf(),
            bytes,
            source_bytes: plan.bytes,
            entries: plan.total_entries(),
            manifest,
            warnings,
        })
    }

    /// Read a backup's manifest without extracting it.
    ///
    /// This is what tells an Eidos backup from any other archive, so its failure
    /// message is the one a user sees when they point `unpack` at the wrong file.
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

    /// The checks that must pass before anything is written to `dest`.
    fn preflight_unpack(
        &self,
        archive: &Path,
        manifest: &BackupManifest,
        dest: &Path,
    ) -> Result<Vec<String>, TransferError> {
        if manifest.schema_version > SCHEMA_VERSION {
            return Err(TransferError::Refused(format!(
                "'{}' was made by a newer Eidos (backup format {}, this build understands \
                 {SCHEMA_VERSION}). Update Eidos and try again.",
                archive.display(),
                manifest.schema_version
            )));
        }
        let paths = eidos_sevenzip::list_paths(self.bin, archive)?;
        if let Some(bad) = paths.iter().find(|p| escapes_the_destination(p)) {
            return Err(TransferError::Refused(format!(
                "'{}' contains an entry that would be written OUTSIDE the folder you \
                 chose ('{bad}'). Eidos will not unpack it.",
                archive.display()
            )));
        }
        let mut warnings = Vec::new();
        if !paths.iter().any(|p| p == "eidos-instance.ini") {
            warnings.push(
                "The backup has no eidos-instance.ini, so the unpacked folder will not \
                 describe its own game. Open it once through the GUI wizard to adopt it."
                    .to_string(),
            );
        }
        if dest.symlink_metadata().is_ok() {
            if !dest.is_dir() {
                return Err(TransferError::Refused(format!(
                    "'{}' exists and is not a folder.",
                    dest.display()
                )));
            }
            let occupied = fs::read_dir(dest)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);
            if occupied && !self.opt.force {
                return Err(TransferError::Refused(format!(
                    "'{}' is not empty. Unpacking into it would merge the backup with \
                     whatever is already there. Choose an empty folder, or pass --force \
                     if you mean to overwrite it.",
                    dest.display()
                )));
            }
        }
        if let Some(free) = free_bytes(&existing_ancestor(dest)) {
            if free < manifest.bytes {
                return Err(TransferError::Refused(format!(
                    "Not enough room at '{}': {} free, and the backup unpacks to {}.",
                    dest.display(),
                    human_bytes(free),
                    human_bytes(manifest.bytes)
                )));
            }
        }
        Ok(warnings)
    }

    /// Put a backup back, and repair what a plain copy would have broken.
    pub fn unpack(
        &self,
        archive: &Path,
        dest: &Path,
        on_progress: &mut impl FnMut(u8),
    ) -> Result<UnpackReport, TransferError> {
        let manifest = self.peek(archive)?;
        let mut warnings = self.preflight_unpack(archive, &manifest, dest)?;
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
        eidos_sevenzip::run(self.bin, args, on_progress)?;

        // The instance now lives here, and `kind` has to agree: a backup of a
        // central instance unpacked into a folder of the user's choosing IS a
        // portable one, and an instance that says otherwise sends every command
        // looking for it under `$XDG_DATA_HOME`.
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
                    warnings.push(format!("Could not record where this instance now lives: {e}"));
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

        let entries = manifest.files as usize + manifest.directories as usize;
        Ok(UnpackReport {
            root: dest.to_path_buf(),
            manifest,
            entries,
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
        assert!(looks_like_an_unknown_switch(
            "\nIncorrect command line\n"
        ));
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
    fn a_list_file_is_one_entry_per_line_with_no_leading_dot_slash() {
        // The defect this exists for: a list built from `find .` gives every line
        // a `./` prefix, and 7-Zip then sees `./x` and `x` as two names for one
        // file and refuses the whole archive with "Duplicate filename on disk".
        let body = list_file_body(&["mods/A/x.dds".into(), "empty dir".into()]);
        assert_eq!(body, "mods/A/x.dds\nempty dir\n");
        assert!(!body.contains("./"));
        assert_eq!(list_file_body(&[]), "");
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
