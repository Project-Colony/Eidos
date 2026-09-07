//! The 7-Zip process seam: find the binary, drive it, read its progress.
//!
//! 7-Zip is not an optional extra for Eidos - without it no mod can be
//! installed at all - so every feature that touches an archive ends up needing
//! the same three things: which binary exists on this machine, how to spawn it
//! without deadlocking on its pipes, and how to turn its progress output into
//! numbers. This crate is those three things and nothing else.
//!
//! It lives on its own, with no Eidos dependency and no domain error type,
//! because the alternative was measured: the workspace already carried TWO
//! divergent copies of `find_7z` (one in `eidos-install`, one in
//! `eidos-gamefeatures`, differing in probe mechanism, candidate set and order)
//! and a third was about to be written for instance packing. A process wrapper
//! that belongs to no domain is exactly the shape of the other leaf crates
//! here - `eidos-paths` has an empty dependency list, `eidos-core` has only
//! libc - and making it one is what stops the fourth copy.
//!
//! What deliberately did NOT move here: the installer's temp-directory naming,
//! its backslash-path repair and its NTFS case-collision healing. Those live in
//! the same file today but they are archive-CONTENT policy, not 7-Zip
//! plumbing, and they answer to the installer's tests.

use std::io;
use std::path::Path;
use std::process::Command;

/// What can go wrong at the process seam, with no opinion about why the caller
/// wanted the archive.
///
/// Two variants because there are two genuinely different situations and the
/// remedies differ: a missing binary is fixed by installing a package, a failed
/// run is fixed by looking at what 7-Zip said. Callers map these onto their own
/// domain error - `eidos-install` turns them into `InstallError::No7z` and
/// `InstallError::Extract` - so no installer wording leaks into a crate that
/// has nothing to do with installing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SevenZipError {
    /// No usable 7-Zip binary on `PATH`.
    NotFound,
    /// 7-Zip ran and failed; carries its stderr, trimmed.
    Failed(String),
}

impl std::fmt::Display for SevenZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SevenZipError::NotFound => write!(f, "no 7-Zip binary found (install p7zip)"),
            SevenZipError::Failed(e) => write!(f, "7-Zip failed: {e}"),
        }
    }
}

impl std::error::Error for SevenZipError {}

/// The first usable 7-Zip binary on `PATH`.
///
/// Returns a NAME, not a path: it is resolved through `PATH` again at spawn
/// time, which is what lets a caller hold the answer across a `chdir`.
///
/// This probes by RUNNING each candidate, so it costs up to three fork+exec.
/// That matters more than it looks: a fork briefly shares every open file
/// descriptor, including an `flock` an instance holds, so callers that already
/// hold an instance lock should call this ONCE and keep the answer rather than
/// per-archive.
pub fn find_7z() -> Option<&'static str> {
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
pub fn drain_percents(buf: &mut String) -> Vec<u8> {
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
            let digits: &str =
                &t[..t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len()];
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
///
/// The percentages are NOT monotonic. Only consecutive duplicates are
/// suppressed, so a stream that restarts its count - a second pass, a
/// multi-volume archive - is forwarded as a decrease. A bar that must only ever
/// rise has to clamp with its own running maximum.
pub fn pump_progress(
    mut out: impl io::Read,
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

/// 7-Zip's last repaint is ` 99%`, never 100 - measured on a real 149 MB mod
/// archive. Close the gap so the caller can treat 100 as "the archive is read"
/// and say so, rather than leaving a bar stopped just short through the passes
/// that follow.
///
/// Only on success: a bar that fills up and is then followed by an error reads
/// as "it worked, then something else broke".
pub fn close_the_gap(success: bool, last: Option<u8>, on_progress: &mut impl FnMut(u8)) {
    if success && last != Some(100) {
        on_progress(100);
    }
}

/// Run 7-Zip with `args`, feeding its progress to `on_progress`.
///
/// The one place in the workspace that spawns 7-Zip. It exists so that no
/// caller has to remember the two things that make the difference between a
/// working invocation and a hang:
///
/// * stdout is piped and READ AS IT ARRIVES. The blocking version used
///   `Command::output()`, which holds everything until the child exits, so a
///   GUI driving it on its event thread froze for the whole archive.
/// * stderr is piped and drained on a SIDE THREAD. An error-chatty 7-Zip that
///   fills its stderr pipe blocks forever while we are busy reading stdout.
///
/// `args` must include the progress switches the caller wants (`-bsp1` puts
/// progress on stdout, `-bso0` silences the listing so progress is all that
/// arrives); this function adds none, because add and extract want different
/// ones and guessing on the caller's behalf is how a switch ends up in an
/// invocation that rejects it.
pub fn run<I, S>(
    bin: &str,
    args: I,
    on_progress: &mut impl FnMut(u8),
) -> Result<(), SevenZipError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    run_in(bin, args, None, on_progress)
}

/// [`run`], with a working directory.
///
/// Packing needs it and extraction does not: 7-Zip stores the paths it is
/// GIVEN, so the only way to get instance-relative entries out of absolute
/// paths on disk is to run it from the instance root. Passing `None` inherits
/// the caller's, which is what every extraction wants.
pub fn run_in<I, S>(
    bin: &str,
    args: I,
    cwd: Option<&Path>,
    on_progress: &mut impl FnMut(u8),
) -> Result<(), SevenZipError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    use std::io::Read;
    use std::process::Stdio;
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| SevenZipError::Failed(e.to_string()))?;
    let mut err = child.stderr.take().expect("stderr was piped");
    let drain = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let out = child.stdout.take().expect("stdout was piped");
    let last = pump_progress(out, on_progress).map_err(|e| SevenZipError::Failed(e.to_string()))?;
    let status = child
        .wait()
        .map_err(|e| SevenZipError::Failed(e.to_string()))?;
    let stderr = drain.join().unwrap_or_default();
    if !status.success() {
        // 7-Zip is not always chatty on failure; an empty stderr with a bad exit
        // code has to say SOMETHING or the caller reports a blank reason.
        let why = if stderr.trim().is_empty() {
            format!("exited with {status}")
        } else {
            stderr.trim().to_string()
        };
        return Err(SevenZipError::Failed(why));
    }
    close_the_gap(true, last, on_progress);
    Ok(())
}

/// Extract every entry of `archive` into `dest`, reporting 7-Zip's own progress.
pub fn extract_all_with(
    bin: &str,
    archive: &Path,
    dest: &Path,
    mut on_progress: impl FnMut(u8),
) -> Result<(), SevenZipError> {
    run(
        bin,
        [
            std::ffi::OsStr::new("x"),
            std::ffi::OsStr::new("-y"),
            std::ffi::OsStr::new(&format!("-o{}", dest.display())),
            std::ffi::OsStr::new("-bsp1"),
            std::ffi::OsStr::new("-bso0"),
            archive.as_os_str(),
        ],
        &mut on_progress,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pump(chunks: &[&str]) -> (Vec<u8>, Option<u8>) {
        struct Chunks<'a> {
            rest: &'a [&'a str],
        }
        impl io::Read for Chunks<'_> {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                let Some((first, rest)) = self.rest.split_first() else {
                    return Ok(0);
                };
                self.rest = rest;
                let b = first.as_bytes();
                buf[..b.len()].copy_from_slice(b);
                Ok(b.len())
            }
        }
        let mut seen = Vec::new();
        let last = pump_progress(Chunks { rest: chunks }, &mut |p| seen.push(p)).unwrap();
        (seen, last)
    }

    #[test]
    fn a_percentage_is_only_read_when_the_chunk_starts_with_one() {
        // " 99% 27" carries a file count after the percentage, and a printed
        // filename can itself contain a '%'. Reading either as progress was the
        // original defect.
        let mut buf = String::from(" 12%\u{8} 99% 27\u{8}50% of Foo%Bar.esp\u{8}");
        assert_eq!(drain_percents(&mut buf), vec![12, 99, 50]);
    }

    #[test]
    fn an_unfinished_repaint_is_kept_for_the_next_read() {
        // A read can split " 47%" into " 4" and "7%". Losing the tail would
        // drop the percentage entirely.
        let mut buf = String::from(" 10%\u{8} 4");
        assert_eq!(drain_percents(&mut buf), vec![10]);
        buf.push_str("7%\u{8}");
        assert_eq!(drain_percents(&mut buf), vec![47]);
    }

    #[test]
    fn a_value_above_a_hundred_is_not_a_percentage() {
        let mut buf = String::from(" 250%\u{8}");
        assert!(drain_percents(&mut buf).is_empty());
    }

    #[test]
    fn a_stream_split_across_reads_is_read_the_same_as_a_whole_one() {
        let whole = pump(&[" 1%\u{8} 50%\u{8} 99%\u{8}"]).0;
        let split = pump(&[" 1%\u{8} 5", "0%\u{8} 9", "9%\u{8}"]).0;
        assert_eq!(whole, split);
    }

    #[test]
    fn a_repeated_percentage_is_reported_once() {
        // 7-Zip repaints the same value constantly; forwarding each repaint
        // would make a caller redraw hundreds of times per second.
        assert_eq!(pump(&[" 7%\u{8} 7%\u{8} 7%\u{8} 8%\u{8}"]).0, vec![7, 8]);
    }

    #[test]
    fn the_gap_to_a_hundred_is_closed_only_on_success() {
        // 7-Zip's last repaint is 99%. A bar that fills and is THEN followed by
        // an error reads as "it worked, then something else broke".
        let mut seen = Vec::new();
        close_the_gap(true, Some(99), &mut |p| seen.push(p));
        assert_eq!(seen, vec![100]);

        let mut seen = Vec::new();
        close_the_gap(false, Some(99), &mut |p| seen.push(p));
        assert!(seen.is_empty(), "no synthetic 100 after a failure");

        let mut seen = Vec::new();
        close_the_gap(true, Some(100), &mut |p| seen.push(p));
        assert!(seen.is_empty(), "already at 100, do not repeat it");
    }

    #[test]
    fn a_failure_with_a_silent_stderr_still_carries_a_reason() {
        // `false` exits non-zero and says nothing. Reporting an empty string
        // gives the user a dialog with no cause in it.
        let mut none = |_| {};
        let err = run("false", ["ignored"], &mut none).unwrap_err();
        match err {
            SevenZipError::Failed(why) => assert!(!why.trim().is_empty(), "{why:?}"),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_binary_is_a_failure_to_spawn_not_a_panic() {
        let mut none = |_| {};
        assert!(run("eidos-no-such-binary-7z", ["x"], &mut none).is_err());
    }

    #[test]
    fn the_two_errors_read_differently_to_a_user() {
        assert!(SevenZipError::NotFound.to_string().contains("p7zip"));
        assert!(SevenZipError::Failed("bad archive".into())
            .to_string()
            .contains("bad archive"));
    }
}
