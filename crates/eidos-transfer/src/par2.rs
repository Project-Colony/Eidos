//! PAR2 recovery data beside a `.eidos` file, through par2cmdline.
//!
//! A `.eidos` file is written once and read once, often years apart, from a
//! disk nobody has looked at since. 7-Zip can say that a byte went bad but not
//! put it back, and with 16 MB solid blocks one bad byte costs up to 16 MB of
//! mods. Five percent of PAR2 recovery blocks beside the archive puts it back.
//!
//! par2cmdline is optional. Without it a pack writes no recovery data and an
//! unpack extracts as it always did, with a warning when recovery files are
//! there and cannot be used. The only switches passed are `-r` and `-q`,
//! checked against the par2cmdline 0.8.1 source, so the 0.8.x shipped by
//! distributions works.
//!
//! The files are named the way par2cmdline names them for `<archive>.par2`:
//! the index `<archive>.par2` and the volumes `<archive>.volNN+MM.par2`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The binary looked up on `PATH`.
pub(crate) const BIN: &str = "par2";

/// The share of the archive written as recovery blocks, in percent.
pub const REDUNDANCY_PERCENT: u64 = 5;

/// Whether `bin` runs at all.
///
/// One fork+exec, so the caller does it once and keeps the answer, before it
/// takes an instance lock (see `Transfer`).
pub(crate) fn probe(bin: &str) -> bool {
    Command::new(bin)
        .arg("-V")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// `<archive>.par2`, the file an unpack looks for.
pub(crate) fn index(archive: &Path) -> PathBuf {
    let mut s = archive.as_os_str().to_os_string();
    s.push(".par2");
    PathBuf::from(s)
}

/// Every recovery file of `archive`'s set that exists: the index and the
/// `.volNN+MM.par2` volumes. Nothing else that merely starts with the name.
pub(crate) fn files(archive: &Path) -> Vec<PathBuf> {
    let Some(name) = archive.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let dir = match archive.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let n = e.file_name();
            let Some(n) = n.to_str() else { return false };
            let Some(rest) = n.strip_prefix(name) else {
                return false;
            };
            rest == ".par2" || is_volume_suffix(rest)
        })
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// `.vol00+01.par2`, `.vol063+37.par2`: digits, a `+`, digits.
fn is_volume_suffix(rest: &str) -> bool {
    let Some(mid) = rest
        .strip_prefix(".vol")
        .and_then(|r| r.strip_suffix(".par2"))
    else {
        return false;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    mid.split_once('+')
        .is_some_and(|(a, b)| digits(a) && digits(b))
}

/// Remove `archive`'s recovery set.
///
/// Recovery files that describe a different archive are worse than none: an
/// unpack would read the new archive as damaged beyond repair and refuse it.
pub(crate) fn remove(archive: &Path) {
    for f in files(archive) {
        let _ = std::fs::remove_file(f);
    }
}

/// The line of par2's output that says why, for an error message.
///
/// par2 ends a verdict with its reason ("You need 12 more recovery blocks to
/// be able to repair."), so the last line of what it said is the one to keep.
fn last_line(out: &std::process::Output) -> String {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Without its full stop, so a caller can put the line inside a sentence.
    text.lines()
        .map(|l| l.trim().trim_end_matches('.'))
        .rfind(|l| !l.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("par2 exited with {}", out.status))
}

/// Run par2 with absolute paths.
///
/// Absolute because par2cmdline takes the folder of the `.par2` file as the
/// base every data file must live under, and because an absolute path never
/// starts with `-`, so no file name can be read as a switch.
fn run(
    bin: &str,
    verb: &str,
    extra: &[&str],
    archive: &Path,
) -> std::io::Result<std::process::Output> {
    let archive = std::path::absolute(archive)?;
    let mut cmd = Command::new(bin);
    cmd.arg(verb).args(extra).arg("-q").arg(index(&archive));
    if verb == "create" {
        cmd.arg(&archive);
    }
    // par2cmdline is built with OpenMP, whose idle threads spin by default.
    // Two par2 processes at once (the test suite runs them in parallel) then
    // burn every core waiting on each other: measured at 37 s and 563 s of CPU
    // for twelve tests that take 1 s one at a time. Passive waiting made a lone
    // 300 MB create no slower (2.72 s against 2.86 s).
    cmd.env("OMP_WAIT_POLICY", "passive")
        .stdin(Stdio::null())
        .output()
}

/// Write `archive`'s recovery set, or say why not.
///
/// A failure leaves no partial set behind: an index without its volumes would
/// make the next unpack report damage it cannot repair.
pub(crate) fn create(bin: &str, archive: &Path) -> Result<Vec<PathBuf>, String> {
    let redundancy = format!("-r{REDUNDANCY_PERCENT}");
    let failed = match run(bin, "create", &[&redundancy], archive) {
        Ok(out) if out.status.success() => None,
        Ok(out) => Some(last_line(&out)),
        Err(e) => Some(e.to_string()),
    };
    let written = files(archive);
    match failed {
        None if !written.is_empty() => Ok(written),
        None => Err("par2 reported success but wrote no recovery files".to_string()),
        Some(why) => {
            remove(archive);
            Err(why)
        }
    }
}

/// What `par2 verify` said about an archive.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Exit 0: every block matches.
    Intact,
    /// Exit 1: damaged, and there are enough recovery blocks to repair it.
    Repairable,
    /// Exit 2: damaged beyond what the recovery blocks cover.
    Unrepairable(String),
    /// Anything else: par2 could not answer (the recovery files themselves are
    /// unreadable, out of memory, ...). Says nothing about the archive.
    Unknown(String),
}

/// Exit codes from par2cmdline's `Result` enum (libpar2.h): 0 success,
/// 1 repair possible, 2 repair not possible.
pub(crate) fn verify(bin: &str, archive: &Path) -> Verdict {
    match run(bin, "verify", &[], archive) {
        Ok(out) => match out.status.code() {
            Some(0) => Verdict::Intact,
            Some(1) => Verdict::Repairable,
            Some(2) => Verdict::Unrepairable(last_line(&out)),
            _ => Verdict::Unknown(last_line(&out)),
        },
        Err(e) => Verdict::Unknown(e.to_string()),
    }
}

/// Repair `archive` in place, returning where par2 put the damaged copy.
///
/// par2cmdline renames a damaged file to `<name>.1` (the first free number)
/// and writes the repaired one under the original name. That copy is the
/// user's file and is left for them to delete, so they are told where it is.
pub(crate) fn repair(bin: &str, archive: &Path) -> Result<Option<PathBuf>, String> {
    let numbered = |i: u32| {
        let mut s = archive.as_os_str().to_os_string();
        s.push(format!(".{i}"));
        PathBuf::from(s)
    };
    let set_aside = (1..).map(numbered).find(|p| p.symlink_metadata().is_err());
    match run(bin, "repair", &[], archive) {
        Ok(out) if out.status.success() => Ok(set_aside.filter(|p| p.exists())),
        Ok(out) => Err(last_line(&out)),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_files_of_this_archives_set_are_recovery_files() {
        let dir = std::env::temp_dir().join(format!("eidos-par2-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in [
            "b.eidos",
            "b.eidos.par2",
            "b.eidos.vol00+01.par2",
            "b.eidos.vol063+37.par2",
            // Not this set: another archive, a stray copy, a damaged copy.
            "b.eidos2.par2",
            "b.eidos.vol00+01.par2.bak",
            "b.eidos.volx+1.par2",
            "b.eidos.1",
            "a.eidos.par2",
        ] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let names: Vec<String> = files(&dir.join("b.eidos"))
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "b.eidos.par2",
                "b.eidos.vol00+01.par2",
                "b.eidos.vol063+37.par2"
            ]
        );
        remove(&dir.join("b.eidos"));
        assert!(files(&dir.join("b.eidos")).is_empty());
        assert!(dir.join("b.eidos2.par2").exists() && dir.join("b.eidos.1").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_binary_is_not_found() {
        assert!(!probe("/nonexistent/eidos-test/par2"));
    }
}
