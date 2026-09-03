//! The 7-Zip backend: listing, extraction into a unique temp dir, and the
//! [`ExtractedTree`] that owns it.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

pub(crate) static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The first usable 7-Zip binary on `PATH`.
pub(crate) fn find_7z() -> Option<&'static str> {
    ["7z", "7zz", "7za"]
        .into_iter()
        .find(|b| Command::new(b).output().is_ok())
}

/// Split off every complete chunk of 7-Zip `-bsp1` progress output in `buf`,
/// returning the percentages found in order.
///
/// 7-Zip repaints its progress line in place: chunks are separated by
/// backspaces or carriage returns, never reliably by newlines. A chunk is a
/// percentage only when it STARTS with one (`" 99% 27"` carries a file count,
/// and a printed filename may itself contain `%`). The text after the last
/// separator may be an unfinished repaint - a read can split ` 47%` into ` 4`
/// and `7%` - so it stays in `buf` for the next call.
pub(crate) fn drain_percents(buf: &mut String) -> Vec<u8> {
    const SEPS: [char; 3] = ['\u{8}', '\r', '\n'];
    let Some(cut) = buf.rfind(SEPS) else {
        return Vec::new();
    };
    let complete = buf[..cut].to_string();
    *buf = buf[cut + 1..].to_string();
    complete
        .split(SEPS)
        .filter_map(|chunk| {
            let t = chunk.trim_start();
            let digits: &str = &t[..t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len()];
            if digits.is_empty() || !t[digits.len()..].starts_with('%') {
                return None;
            }
            digits.parse::<u8>().ok().filter(|p| *p <= 100)
        })
        .collect()
}

/// Drain a 7-Zip progress stream, calling `on_progress` on each NEW percentage,
/// and report the last one seen.
///
/// Split out from the spawn so the reading can be tested against an in-memory
/// stream. Testing it through a throwaway shell script exec'd as a stand-in
/// 7-Zip is racy by construction: the harness runs tests in parallel threads,
/// and a `fork` in one thread inherits the write descriptor another thread still
/// holds on the script it has just written, so the `exec` fails with `ETXTBSY`.
/// That is a property of write-then-exec in a threaded process, not of anything
/// here, and it failed about one run in ten.
fn pump_progress(
    mut out: impl std::io::Read,
    on_progress: &mut impl FnMut(u8),
) -> io::Result<Option<u8>> {
    let mut buf = [0u8; 4096];
    let mut tail = String::new();
    let mut last = None;
    loop {
        let n = out.read(&mut buf)?;
        if n == 0 {
            break;
        }
        tail.push_str(&String::from_utf8_lossy(&buf[..n]));
        for p in drain_percents(&mut tail) {
            // The same value repaints constantly; the caller redraws per call.
            if last != Some(p) {
                last = Some(p);
                on_progress(p);
            }
        }
    }
    Ok(last)
}

/// Extract every entry of `archive` into `dest`, reporting 7-Zip's own progress.
///
/// The blocking path used `Command::output()`, which holds everything until the
/// child exits - the caller learned nothing until the end, and a GUI driving it
/// on its event thread froze for the whole archive. This one pipes stdout
/// (`-bsp1` puts the progress there; `-bso0` silences the listing so progress
/// is ALL that arrives) and feeds each new percentage to `on_progress` as it is
/// read. stderr is drained on a side thread: an error-chatty 7-Zip must never
/// fill its pipe and deadlock against us reading stdout.
pub(crate) fn extract_all_with(
    bin: &str,
    archive: &Path,
    dest: &Path,
    mut on_progress: impl FnMut(u8),
) -> Result<(), InstallError> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = Command::new(bin)
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", dest.display()))
        .arg("-bsp1")
        .arg("-bso0")
        .arg(archive)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| InstallError::Extract(e.to_string()))?;
    let mut err = child.stderr.take().expect("stderr was piped");
    let drain = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let out = child.stdout.take().expect("stdout was piped");
    let last =
        pump_progress(out, &mut on_progress).map_err(|e| InstallError::Extract(e.to_string()))?;
    let status = child
        .wait()
        .map_err(|e| InstallError::Extract(e.to_string()))?;
    let stderr = drain.join().unwrap_or_default();
    if !status.success() {
        return Err(InstallError::Extract(stderr.trim().to_string()));
    }
    close_the_gap(true, last, &mut on_progress);
    Ok(())
}

/// 7-Zip's last repaint is ` 99%`, never 100 - measured on a real 149 MB mod
/// archive. Close the gap so the caller can treat 100 as "the archive is read"
/// and say so, rather than leaving a bar stopped just short through the passes
/// that follow (data-root search, NTFS case-collision healing).
///
/// Only on success: a bar that fills up and is then followed by an error reads
/// as "it worked, then something else broke".
fn close_the_gap(success: bool, last: Option<u8>, on_progress: &mut impl FnMut(u8)) {
    if success && last != Some(100) {
        on_progress(100);
    }
}

/// An archive already extracted into a temp directory beside `mods/`, with NTFS
/// case collisions healed. Holding one means the expensive 7-Zip pass is already
/// paid for: [`install_extracted`] installs straight from it, so a simple archive
/// is never extracted twice. The temp is removed when this is dropped.
pub struct ExtractedTree {
    pub(crate) tmp: PathBuf,
}

impl ExtractedTree {
    /// The extracted tree's root on disk.
    pub fn path(&self) -> &Path {
        &self.tmp
    }
}

impl Drop for ExtractedTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.tmp);
    }
}

/// Rebuild a real directory tree from an archive that used BACKSLASHES as its
/// path separator.
///
/// The ZIP spec is unambiguous (APPNOTE 4.4.17.1: the separator is `/`), but
/// Windows tools write `\` anyway and nobody notices, because on Windows 7-Zip
/// treats both as separators. On Linux a backslash is an ordinary filename
/// character, so p7zip extracts `Mod\Data\F4SE\Plugins\CBP.dll` as ONE file
/// whose name contains backslashes, at the top level. The result is a flat pile
/// with no `Data/` and no `fomod/`, so wrapper detection finds nothing, the
/// FOMOD is invisible, and the install fails in a way that looks like the mod is
/// broken. Observed on OpenCBP for Fallout 4, 15 entries out of 15.
///
/// Only fires when backslash-bearing names strictly OUTNUMBER the rest, so one
/// oddly-named file in an otherwise normal archive is left exactly as it is: a
/// backslash is legal here, and silently restructuring somebody's mod because of
/// one filename would be worse than the problem.
///
/// A name that is not valid UTF-8 never counts and is never touched, which is
/// not a detail. In every legacy CJK encoding the TRAIL byte of a two-byte
/// character can be 0x5C: CP932 `ソ` is 83 5C, `表` is 95 5C, and Big5 and GBK
/// have the same hazard - the dame-moji problem that has been biting Japanese
/// software since the 1980s. `to_string_lossy` turns the undecodable lead byte
/// into U+FFFD and leaves that 0x5C standing as a literal backslash, so a
/// perfectly ordinary Japanese mod looks like a flattened archive, and rebuilding
/// a destination from that same lossy string RENAMES the file, replacing its
/// bytes with EF BF BD. A `ソード.esp` sitting correctly at a mod's root ends up
/// buried in a garbage directory, and the install still reports success.
///
/// `to_str` is therefore the right tool and `OsStrExt` is the wrong one: the
/// bytes really do contain 0x5C, so splitting on them would stop the mangling
/// and still relocate the plugin into a phantom folder. An undecodable name is
/// simply an ordinary name in an encoding we do not know, and the crate already
/// treats it that way in `resolve_dir_collisions`.
///
/// Returns how many entries were rebuilt.
fn unflatten_backslash_paths(root: &Path) -> io::Result<usize> {
    let entries: Vec<_> = fs::read_dir(root)?.filter_map(Result::ok).collect();
    let (mut flat, mut plain) = (Vec::new(), 0usize);
    for e in entries {
        match e.file_name().to_str().is_some_and(|n| n.contains('\\')) {
            true => flat.push(e),
            false => plain += 1,
        }
    }
    if flat.len() <= plain {
        return Ok(0);
    }

    // Directories last: the flat extraction leaves the archive's directory
    // entries behind as empty placeholders (`Mod\Data\`), and moving the files
    // first means those placeholders are then either redundant or already
    // created by `create_dir_all` below.
    flat.sort_by_key(|e| e.path().is_dir());

    let mut moved = 0;
    for e in flat {
        // `to_str` again, for the same reason as the guard: a destination built
        // from a lossy string would rename the file it is meant to move.
        let Some(raw) = e.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(rel) = safe_relative(&raw) else {
            // Refused rather than repaired. Splitting on backslashes is exactly
            // what turns `..\..\etc\passwd` from one harmlessly-named file
            // into a path that climbs out of the temp directory, so this guard
            // is not theoretical - it is a hole this function would otherwise
            // have opened.
            continue;
        };
        let dest = root.join(&rel);
        let src = e.path();
        if src.is_dir() {
            // Remove FIRST, then mirror. A flattened extraction only ever leaves
            // EMPTY placeholders (`Mod\Data\`), so `remove_dir` succeeding is the
            // proof that this is one. Creating the destination first and then
            // swallowing a failed removal is what turns an archive that mixes
            // `\` and `/` into an empty mirror beside stranded real content -
            // and the install reports success, because `ArchiveTree::from_dir`
            // already normalises separators and so believes the files arrived.
            if fs::remove_dir(&src).is_err() {
                continue;
            }
            fs::create_dir_all(&dest)?;
            moved += 1;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&src, &dest)?;
        moved += 1;
    }
    Ok(moved)
}

/// A backslash-separated archive name as a relative path, or `None` if it tries
/// to leave the tree. Every component must be an ordinary name: `..` climbs out,
/// an absolute component would escape entirely, and empty components come from
/// doubled separators and mean nothing.
fn safe_relative(raw: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    let mut any = false;
    for part in raw.split('\\') {
        let part = part.trim_end_matches('/');
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('/') || part.starts_with('/') {
            return None;
        }
        out.push(part);
        any = true;
    }
    any.then_some(out)
}

/// Extract `archive` into a fresh temp directory beside `mods_dir`, rebuild any
/// backslash-flattened tree and heal NTFS-style case collisions, so everything
/// downstream (wrapper detection, FOMOD lookup, the move) reads a consistent
/// tree. The returned handle owns the
/// temp and removes it on drop, including on the `?` paths here.
pub fn extract_to_temp(archive: &Path, mods_dir: &Path) -> Result<ExtractedTree, InstallError> {
    extract_to_temp_with(archive, mods_dir, |_| {})
}

/// [`extract_to_temp`], with 7-Zip's live percentage fed to `on_progress`.
pub fn extract_to_temp_with(
    archive: &Path,
    mods_dir: &Path,
    on_progress: impl FnMut(u8),
) -> Result<ExtractedTree, InstallError> {
    let bin = find_7z().ok_or(InstallError::No7z)?;
    let tmp = mods_dir.join(format!(
        ".eidos-install-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&tmp)?;
    let tree = ExtractedTree { tmp };
    extract_all_with(bin, archive, &tree.tmp, on_progress)?;
    // Before the case pass, not after: this one CREATES the directories that the
    // case pass then reconciles.
    match unflatten_backslash_paths(&tree.tmp) {
        Ok(0) => {}
        Ok(n) => eidos_log::info!(
            "eidos install: the archive used Windows path separators; rebuilt {n} entries into a real folder tree"
        ),
        Err(e) => eidos_log::warn!("eidos install: could not rebuild the archive's folder tree ({e})"),
    }
    normalize_case_collisions(&tree.tmp)?;
    Ok(tree)
}

#[cfg(test)]
mod backslash_tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "eidos-bs-{}-{tag}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_windows_separated_archive_becomes_a_real_tree() {
        // Exactly what p7zip leaves behind for OpenCBP_FO4-3.5.240.zip: every
        // entry at the top level, every name carrying its own path.
        let d = tmp("real");
        for f in [
            r"OpenCBP_FO4-3.5.240\Data\F4SE\Plugins\CBP.dll",
            r"OpenCBP_FO4-3.5.240\Data\Scripts\OCBP_API.pex",
            r"OpenCBP_FO4-3.5.240\FOMOD\info.xml",
            r"OpenCBP_FO4-3.5.240\OCBP_LICENSE",
        ] {
            fs::write(d.join(f), b"x").unwrap();
        }
        // The empty directory placeholders the archive also carried.
        for dir in [
            r"OpenCBP_FO4-3.5.240\Data\",
            r"OpenCBP_FO4-3.5.240\Data\F4SE\",
        ] {
            fs::create_dir(d.join(dir)).unwrap();
        }

        let n = unflatten_backslash_paths(&d).unwrap();
        assert_eq!(n, 6);

        // The two things every later step looks for and could not see before.
        assert!(d
            .join("OpenCBP_FO4-3.5.240/Data/F4SE/Plugins/CBP.dll")
            .is_file());
        assert!(d.join("OpenCBP_FO4-3.5.240/FOMOD/info.xml").is_file());
        assert!(d
            .join("OpenCBP_FO4-3.5.240/Data/Scripts/OCBP_API.pex")
            .is_file());
        assert!(d.join("OpenCBP_FO4-3.5.240/OCBP_LICENSE").is_file());
        // And nothing backslash-named survives at the top.
        assert!(
            !fs::read_dir(&d).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains('\\')),
            "a flattened name was left behind"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_legacy_cjk_name_is_never_mistaken_for_a_separator() {
        use std::os::unix::ffi::OsStrExt;
        // The dame-moji problem. CP932 `ソ` is 83 5C and `表` is 95 5C; Big5 and
        // GBK have the same hazard. `to_string_lossy` turns the undecodable lead
        // byte into U+FFFD and leaves the 5C standing as a backslash, so a
        // perfectly ordinary Japanese mod looks flattened - and rebuilding a
        // destination from that lossy string RENAMES the file, replacing its
        // bytes with EF BF BD. A plugin sitting correctly at the mod root ends up
        // buried in a garbage directory, and the install still reports success.
        let d = tmp("cjk");
        let esp = std::ffi::OsStr::from_bytes(b"\x83\x5c\x81\x5b\x83\x68.esp"); // ソード.esp
        let txt = std::ffi::OsStr::from_bytes(b"\x95\x5c.txt"); // 表.txt
        fs::write(d.join(esp), b"plugin").unwrap();
        fs::write(d.join(txt), b"readme").unwrap();
        fs::create_dir(d.join("meshes")).unwrap();

        // Two undecodable names against one ordinary directory: under a lossy
        // reading this is 2 "flat" vs 1 plain and the pass would fire.
        assert_eq!(
            unflatten_backslash_paths(&d).unwrap(),
            0,
            "an unreadable name is not a path"
        );
        assert!(
            d.join(esp).is_file(),
            "the plugin must still be at the mod root, byte for byte"
        );
        assert_eq!(fs::read(d.join(esp)).unwrap(), b"plugin");
        assert!(d.join(txt).is_file());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_backslash_named_directory_that_is_not_empty_is_left_alone() {
        // Only an EMPTY placeholder may be consumed. Creating the destination
        // first and then swallowing a failed removal is what strands real
        // content beside an empty mirror - and the install reports success,
        // because the classifier normalises separators and believes the files
        // arrived.
        let d = tmp("nonempty");
        fs::create_dir(d.join(r"Mod\Data")).unwrap();
        fs::write(d.join(r"Mod\Data").join("real.esp"), b"x").unwrap();
        fs::write(d.join(r"Mod\readme.txt"), b"x").unwrap();

        let n = unflatten_backslash_paths(&d).unwrap();
        assert!(
            d.join(r"Mod\Data").join("real.esp").is_file(),
            "real content must not be stranded"
        );
        assert!(
            !d.join("Mod/Data").is_dir(),
            "no empty mirror may be created beside it"
        );
        assert_eq!(n, 1, "only the plain file is rebuilt");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_tie_does_not_fire() {
        // One odd name against one ordinary entry is not evidence of a
        // flattened archive, and the promise made to users is that a single
        // strange filename is left alone.
        let d = tmp("tie");
        fs::create_dir(d.join("Data")).unwrap();
        fs::write(d.join(r"weird\name.txt"), b"x").unwrap();
        assert_eq!(unflatten_backslash_paths(&d).unwrap(), 0);
        assert!(d.join(r"weird\name.txt").is_file());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn one_odd_filename_in_a_normal_archive_is_left_alone() {
        // A backslash is a legal character here. Restructuring somebody's mod
        // because ONE file is named strangely would be worse than the bug.
        let d = tmp("odd");
        fs::create_dir(d.join("Data")).unwrap();
        fs::write(d.join("readme.txt"), b"x").unwrap();
        fs::write(d.join("Data/a.esp"), b"x").unwrap();
        fs::write(d.join(r"weird\name.txt"), b"x").unwrap();

        assert_eq!(
            unflatten_backslash_paths(&d).unwrap(),
            0,
            "must not fire on a minority"
        );
        assert!(
            d.join(r"weird\name.txt").is_file(),
            "the odd name must survive untouched"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_traversal_attempt_is_refused_not_repaired() {
        // Splitting on backslashes is what would turn this from one harmlessly
        // named file into a write outside the temp directory.
        let d = tmp("evil");
        fs::write(d.join(r"..\..\escaped.txt"), b"x").unwrap();
        fs::write(d.join(r"Mod\Data\ok.esp"), b"x").unwrap();

        let n = unflatten_backslash_paths(&d).unwrap();
        assert_eq!(n, 1, "only the safe entry may be rebuilt");
        assert!(d.join("Mod/Data/ok.esp").is_file());
        assert!(
            d.join(r"..\..\escaped.txt").is_file(),
            "left in place, not moved"
        );
        assert!(!d
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("escaped.txt")
            .exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn safe_relative_accepts_ordinary_names_and_refuses_the_rest() {
        assert_eq!(
            safe_relative(r"Mod\Data\a.esp"),
            Some(PathBuf::from("Mod/Data/a.esp"))
        );
        // Doubled separators and a trailing one are noise, not structure.
        assert_eq!(
            safe_relative(r"Mod\\Data\"),
            Some(PathBuf::from("Mod/Data"))
        );
        assert_eq!(safe_relative(r"..\x"), None);
        assert_eq!(safe_relative(r"Mod\..\..\x"), None);
        // A forward slash inside a component means the name was never purely
        // backslash-separated; refuse rather than guess which one is structure.
        assert_eq!(safe_relative(r"Mod\a/b.esp"), None);
        assert_eq!(safe_relative(r"\"), None);
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;

    /// Captured from a real `7z x -bsp1 -bso0 | cat -v` on this machine: the
    /// stream repaints in place with backspaces, never newlines, and mixes in
    /// a `0M Scan` phase and runs of padding spaces.
    #[test]
    fn percents_are_pulled_from_backspace_separated_repaints() {
        let mut b = String::from(
            "  0M Scan\u{8}\u{8}\u{8}         \u{8}\u{8}  0%\u{8}\u{8}\u{8}\u{8} 12%\u{8}\u{8}\u{8}\u{8} 99% 27\u{8}\u{8}\u{8}",
        );
        assert_eq!(drain_percents(&mut b), vec![0, 12, 99]);
    }

    /// A pipe read can end in the middle of a number; misreading " 4" as 4%
    /// would make the bar jump backwards.
    #[test]
    fn a_number_split_across_two_reads_is_not_misread() {
        let mut b = String::from(" 4");
        assert_eq!(drain_percents(&mut b), Vec::<u8>::new());
        b.push_str("7%\u{8}\u{8}\u{8}");
        assert_eq!(drain_percents(&mut b), vec![47]);
    }

    /// 7-Zip prints " NN% count - filename", and filenames may contain '%'.
    /// Only a percentage at the START of a repaint chunk counts.
    #[test]
    fn a_filename_containing_a_percent_sign_is_not_a_percentage() {
        let mut b = String::from(" 12% 3 - tex%40weird.dds\r");
        assert_eq!(drain_percents(&mut b), vec![12]);
        let mut c = String::from("3 - plain.dds\n");
        assert_eq!(drain_percents(&mut c), Vec::<u8>::new());
    }

    /// Feed a recorded stream through the reader. No process, no exec: see
    /// `pump_progress` for why a stand-in binary cannot be used here.
    fn pump(stream: &str) -> Vec<u8> {
        let mut seen = Vec::new();
        let last = pump_progress(std::io::Cursor::new(stream.as_bytes()), &mut |p| seen.push(p))
            .expect("an in-memory stream cannot fail");
        assert_eq!(
            last,
            seen.last().copied(),
            "the reported last IS the last sent"
        );
        seen
    }

    #[test]
    fn extraction_progress_reaches_the_caller_as_it_happens() {
        assert_eq!(pump(" 12%\u{8}\u{8}\u{8}\u{8} 47%\r100%\n"), vec![12, 47, 100]);
    }

    /// The same percentage repainted twice must not fire the callback twice -
    /// the GUI repaints on every call it gets.
    #[test]
    fn a_repeated_percentage_is_reported_once() {
        assert_eq!(
            pump(" 30%\u{8}\u{8}\u{8}\u{8} 30%\u{8}\u{8}\u{8}\u{8} 31%\n"),
            vec![30, 31]
        );
    }

    /// A pipe splits wherever it likes, including mid-number, so a stream
    /// delivered a byte at a time must read exactly like one delivered whole.
    #[test]
    fn a_stream_split_across_reads_is_read_the_same_as_a_whole_one() {
        struct Dribble(std::vec::IntoIter<u8>);
        impl std::io::Read for Dribble {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                match self.0.next() {
                    Some(b) => {
                        buf[0] = b;
                        Ok(1)
                    }
                    None => Ok(0),
                }
            }
        }
        let whole = " 12%\u{8}\u{8}\u{8}\u{8} 47%\r 99%\n";
        let mut seen = Vec::new();
        pump_progress(
            Dribble(whole.as_bytes().to_vec().into_iter()),
            &mut |p| seen.push(p),
        )
        .unwrap();
        assert_eq!(seen, pump(whole));
    }

    #[test]
    fn a_successful_extraction_ends_at_one_hundred() {
        let mut seen = Vec::new();
        close_the_gap(true, Some(99), &mut |p| seen.push(p));
        assert_eq!(seen, vec![100], "the caller is told the read is over");
    }

    #[test]
    fn a_stream_that_already_reached_a_hundred_is_not_told_twice() {
        let mut seen = Vec::new();
        close_the_gap(true, Some(100), &mut |p| seen.push(p));
        assert!(seen.is_empty());
    }

    /// A failure must NOT end at 100: the bar filling up and then an error
    /// appearing reads as "it worked, then something else broke".
    #[test]
    fn a_failed_extraction_does_not_end_at_one_hundred() {
        let mut seen = Vec::new();
        close_the_gap(false, Some(40), &mut |p| seen.push(p));
        assert!(seen.is_empty());
    }

    /// A 7-Zip that is not there at all is an install failure carrying the OS's
    /// words, not a panic. The non-zero-exit path is the same two lines and was
    /// exercised for real: pointed at a missing archive, it returned 7-Zip's own
    /// "ERROR: errno=2".
    #[test]
    fn a_missing_seven_zip_is_reported_as_an_install_failure() {
        let err = extract_all_with(
            "/nonexistent/definitely-not-7z",
            Path::new("/dev/null"),
            Path::new("/tmp"),
            |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, InstallError::Extract(_)), "{err:?}");
    }
}
