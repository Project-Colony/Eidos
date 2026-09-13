//! Tier-2 prerequisite installation: winetricks verbs (vcrun2022, dotnet8, ...)
//! that write registry / GAC / CLR-host and so cannot be file-copied like the
//! Tier-1 DLLs in [`native_dll`].
//!
//! We run the system `winetricks` pointed straight at Proton's own `wine` plus the
//! game prefix (the approach NaK/Jackify use), which bypasses Steam's
//! pressure-vessel container - and therefore the protontricks + Proton-GE
//! `bwrap`/`ntdll.so` mismatch. These verbs DOWNLOAD their payloads from Microsoft,
//! so this is only ever run on an explicit, user-consented action (`eidos prereqs
//! --install`), never silently at launch.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The Tier-2 installer verbs Eidos knows how to request (vs the Tier-1 bundled
/// DLLs in [`crate::is_tier1_dll`]). A verb in neither set is unknown.
const TIER2_VERBS: &[&str] = &[
    "vcrun2022",
    "dotnet6",
    "dotnet7",
    "dotnet8",
    "dotnetdesktop6",
    "dotnetdesktop8",
    "dotnet48",
    "xact",
    "xact_x64",
];

/// Every winetricks verb this prefix already carries, according to the prefix
/// itself.
///
/// winetricks appends each verb it installs to `winetricks.log` inside the
/// prefix, and protontricks is winetricks, so this is the record of what is
/// there no matter who put it there. Eidos's own `prereqs.done` only knows what
/// EIDOS installed - which reports a runtime the user set up years ago as
/// missing and offers to download it again.
///
/// A missing or unreadable log reads as "nothing recorded", never as "nothing
/// installed": the two are different, and the honest failure is to over-report
/// work rather than to claim a prerequisite is absent when it is not.
pub fn verbs_in_prefix(prefix: &Path) -> std::collections::BTreeSet<String> {
    std::fs::read_to_string(prefix.join("winetricks.log"))
        .map(|s| {
            s.lines()
                .map(str::trim)
                // Settings are logged the same way as packages (`fontsmooth=rgb`,
                // `winxp`); only the part before `=` is a verb name.
                .map(|l| l.split('=').next().unwrap_or(l).trim())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Read the selected prefix's own log and only the instance receipt explicitly
/// bound to that same canonical prefix. Older unscoped markers are not evidence
/// for a newly selected Steam/GOG/Epic prefix.
pub fn satisfied_prereqs_in(
    instance_root: &Path,
    prefix: Option<&Path>,
) -> std::collections::BTreeSet<String> {
    let Some(prefix) = prefix else {
        return Default::default();
    };
    let mut done = verbs_in_prefix(prefix);
    if let (Ok(key), Ok(receipt)) = (
        prefix_receipt_key(prefix),
        fs::read_to_string(instance_root.join("prereqs.done")),
    ) {
        let mut lines = receipt.lines();
        if lines.next() == Some(key.as_str()) {
            done.extend(
                lines
                    .map(str::trim)
                    .filter(|v| is_tier2_verb(v))
                    .map(str::to_string),
            );
        }
    }
    done
}

fn prefix_receipt_key(prefix: &Path) -> io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;
    let prefix = prefix.canonicalize()?;
    if !prefix.is_dir() {
        return Err(io::Error::other("Wine prefix is not a directory"));
    }
    use std::os::unix::fs::MetadataExt;
    let metadata = prefix.metadata()?;
    let mut digest = Sha256::new();
    digest.update(prefix.as_os_str().as_bytes());
    digest.update([0]);
    digest.update(metadata.dev().to_le_bytes());
    digest.update(metadata.ino().to_le_bytes());
    let hash: String = digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(format!("# eidos-prefix-sha256:{hash}"))
}

/// Atomically record successful verbs for one actual Wine prefix. The caller
/// holds the instance lock and has already completed the user-requested install.
pub fn record_prereqs(
    instance_root: &Path,
    prefix: &Path,
    verbs: &std::collections::BTreeSet<String>,
) -> io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut body = prefix_receipt_key(prefix)?;
    body.push('\n');
    for verb in verbs.iter().filter(|v| is_tier2_verb(v)) {
        body.push_str(verb);
        body.push('\n');
    }
    let temp = instance_root.join(format!(
        ".prereqs-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = file
        .write_all(body.as_bytes())
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temp, instance_root.join("prereqs.done")));
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

/// Whether `verb` is a Tier-2 installer verb (runs winetricks, downloads).
pub fn is_tier2_verb(verb: &str) -> bool {
    TIER2_VERBS.contains(&verb)
}

/// Locate a program on `PATH`. `None` if not installed.
fn on_path(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|d| d.join(program))
        .find(|p| p.is_file())
}

/// Locate `winetricks` on `PATH` (Eidos shells out to the system one rather than
/// vendoring it). `None` if it is not installed.
pub fn find_winetricks() -> Option<PathBuf> {
    on_path("winetricks")
}

/// Whether `cabextract` (which several winetricks verbs need) is on `PATH`.
pub fn cabextract_available() -> bool {
    on_path("cabextract").is_some()
}

/// Install ONE `verb` into `prefix` via winetricks, using Proton's own wine.
/// `proton` is the Proton entry script (its directory holds `files/bin/wine`);
/// `steam_env` is the `STEAM_COMPAT_*` set Proton uses on a real launch (passed for
/// parity). The verb's payload is fetched from Microsoft, so the caller must have
/// user consent. Installing one verb at a time lets the caller record each success,
/// so a later verb's failure does not lose the earlier ones.
///
/// We point winetricks straight at Proton's wine (no `LD_LIBRARY_PATH`: wine resolves
/// its builtin modules relative to its own binary, which is how NaK and Proton run
/// it), and we do NOT force `mscoree` (let winetricks manage mono/.NET itself).
pub fn install_tier2_verb(
    proton: &Path,
    prefix: &Path,
    steam_env: &[(String, String)],
    verb: &str,
) -> io::Result<()> {
    let winetricks = find_winetricks().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "winetricks not found on PATH (install it, e.g. `pacman -S winetricks`)",
        )
    })?;

    // The proton script lives at `<proton_dir>/proton`; its wine is under `files/bin`.
    let proton_dir = proton.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("malformed Proton path: {}", proton.display()),
        )
    })?;
    // Valve's official builds put wine under `files/bin`; several community and
    // distro-repackaged builds use `dist/bin` instead. Try both before giving up,
    // or a perfectly good Proton is rejected on layout alone.
    let (wine, wineserver) = ["files/bin", "dist/bin"]
        .iter()
        .map(|d| {
            (
                proton_dir.join(d).join("wine"),
                proton_dir.join(d).join("wineserver"),
            )
        })
        .find(|(w, _)| w.is_file())
        .ok_or_else(|| {
            // Fail loudly rather than letting winetricks silently fall back to a
            // system `wine` (a different version) against this Proton prefix.
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Proton wine not found under {}/files/bin or {}/dist/bin",
                    proton_dir.display(),
                    proton_dir.display()
                ),
            )
        })?;

    let mut cmd = Command::new(&winetricks);
    cmd.arg("-q")
        .arg(verb)
        .env("WINEPREFIX", prefix)
        .env("WINE", &wine)
        .env("WINESERVER", &wineserver)
        // Suppress only the Gecko/MSHTML download prompt for an unattended run; let
        // winetricks manage mono/mscoree itself (forcing mscoree=d breaks dotnet48).
        .env("WINEDLLOVERRIDES", "mshtml=d")
        .env("WINEDEBUG", "-all")
        // Xalia draws an accessibility overlay over Proton windows. It has nothing
        // to do here and gets in the way of an unattended install.
        .env("PROTON_USE_XALIA", "0");
    // Large Microsoft installers (the .NET runtimes above all) unpack into TMPDIR,
    // and on most systems /tmp and XDG_RUNTIME_DIR are small tmpfs mounts - which
    // makes them report ERROR_DISK_FULL while the prefix itself has plenty of
    // room. Point them at a directory on the same real filesystem as the prefix,
    // which Eidos owns and can clean up.
    if let Some(tmp) = installer_tmpdir(prefix) {
        cmd.env("TMPDIR", &tmp).env("TMP", &tmp).env("TEMP", &tmp);
    }
    // Parity with a real Proton launch: the STEAM_COMPAT_* vars Proton sets.
    for (k, v) in steam_env {
        cmd.env(k, v);
    }

    let status = cmd.status()?;
    match status.code() {
        Some(c) if installer_success(c) => Ok(()),
        Some(c) => Err(io::Error::other(format!(
            "winetricks {verb} exited with {c}{}",
            describe_installer_exit(c)
                .map(|d| format!(" ({d})"))
                .unwrap_or_default()
        ))),
        None => Err(io::Error::other(format!(
            "winetricks {verb} was killed by a signal"
        ))),
    }
}

/// A scratch directory for installers, beside the prefix so it is on the same
/// real filesystem (never a small tmpfs). Created on demand; `None` if it cannot
/// be, in which case the caller simply leaves TMPDIR alone.
fn installer_tmpdir(prefix: &Path) -> Option<PathBuf> {
    let dir = prefix.parent()?.join("eidos-installer-tmp");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Whether a Microsoft installer exit code means success.
///
/// These installers do not follow the "0 means OK" convention: they report
/// "success, restart required" and "a newer version is already installed" as
/// distinct non-zero codes, and treating those as failures aborts a prefix setup
/// that in fact completed.
fn installer_success(code: i32) -> bool {
    matches!(code, 0 | 105 | 194 | 236 | 1638 | 3010)
}

/// A human explanation for an installer exit code, when there is one worth
/// putting in front of the user.
fn describe_installer_exit(code: i32) -> Option<&'static str> {
    Some(match code {
        5 => "access denied - is the game or Steam still running against this prefix?",
        105 => "installed, restart required",
        112 => "not enough disk space",
        194 => "installed, restart scheduled",
        236 => "a newer version is already installed",
        1638 => "another version of this product is already installed",
        3010 => "installed, reboot required",
        _ => return None,
    })
}

/// Processes currently holding this Wine prefix, as `(pid, cmdline)`.
///
/// Running a prefix operation while the game, Steam or a stale `wineserver` is
/// still attached deadlocks: those processes hold registry and filesystem locks
/// that a new `wineboot` waits on forever. Eidos DETECTS and refuses rather than
/// killing anything - the prefix may well belong to a session the user is using.
///
/// Ownership is confirmed from `/proc/<pid>/environ`, not from the command line:
/// Wine processes carry Windows-style argv that never mentions the Linux prefix
/// path, so a cmdline match alone misses exactly the processes that matter.
pub fn prefix_busy(prefix: &Path, compatdata: &Path) -> Vec<(u32, String)> {
    prefix_busy_at(prefix, Some(compatdata))
}

/// Like `prefix_busy`, with no fabricated compatdata for an external Wine prefix.
pub fn prefix_busy_at(prefix: &Path, compatdata: Option<&Path>) -> Vec<(u32, String)> {
    const MARKERS: [&str; 5] = [
        "wineboot",
        "wineserver",
        "pv-adverb",
        "wine-preloader",
        "steam.exe",
    ];
    let want_prefix = fs::canonicalize(prefix).unwrap_or_else(|_| prefix.to_path_buf());
    let want_compat =
        compatdata.map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
    let me = std::process::id();

    let mut busy = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return busy;
    };
    for e in entries.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        if pid == me {
            continue;
        }
        // Cheap filter first: only a handful of processes are ever candidates.
        let Ok(raw_cmd) = fs::read(e.path().join("cmdline")) else {
            continue;
        };
        let cmdline = String::from_utf8_lossy(&raw_cmd)
            .replace('\0', " ")
            .trim()
            .to_string();
        if !MARKERS.iter().any(|m| cmdline.contains(m)) {
            continue;
        }
        // Then confirm the process actually belongs to THIS prefix. Unreadable
        // environ means another user's process, which is not ours to worry about.
        let Ok(raw_env) = fs::read(e.path().join("environ")) else {
            continue;
        };
        if environ_owns_prefix_at(&raw_env, &want_prefix, want_compat.as_deref()) {
            busy.push((pid, cmdline));
        }
    }
    busy
}

/// Whether a `/proc/<pid>/environ` buffer (NUL-separated `KEY=VALUE` entries)
/// names this prefix. Split out from the `/proc` walk so it can be tested against
/// a synthetic buffer without needing real Wine processes.
#[cfg(test)]
fn environ_owns_prefix(raw: &[u8], prefix: &Path, compatdata: &Path) -> bool {
    environ_owns_prefix_at(raw, prefix, Some(compatdata))
}

fn environ_owns_prefix_at(raw: &[u8], prefix: &Path, compatdata: Option<&Path>) -> bool {
    String::from_utf8_lossy(raw)
        .split('\0')
        .any(|kv| match kv.split_once('=') {
            // Compare canonically where possible so a symlinked or trailing-slash
            // spelling still matches, and fall back to a literal compare when the
            // path no longer resolves.
            Some(("WINEPREFIX", v)) => same_path(v, prefix),
            Some(("STEAM_COMPAT_DATA_PATH", v)) => {
                compatdata.is_some_and(|path| same_path(v, path))
            }
            _ => false,
        })
}

fn same_path(value: &str, want: &Path) -> bool {
    let v = Path::new(value);
    fs::canonicalize(v).map(|p| p == want).unwrap_or(false) || v == want
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_prefix_ownership_never_needs_a_steam_compat_path() {
        let prefix = Path::new("/tmp/eidos-external-wine");
        assert!(environ_owns_prefix_at(
            b"WINEPREFIX=/tmp/eidos-external-wine\0",
            prefix,
            None
        ));
        assert!(!environ_owns_prefix_at(
            b"STEAM_COMPAT_DATA_PATH=/tmp/eidos-external-wine\0",
            prefix,
            None
        ));
    }

    #[test]
    fn prereq_receipts_belong_to_the_exact_prefix_and_legacy_logs_still_work() {
        let root =
            std::env::temp_dir().join(format!("eidos-prereq-identity-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let a = root.join("a");
        let b = root.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(root.join("prereqs.done"), "dotnet8\n").unwrap();
        assert!(satisfied_prereqs_in(&root, Some(&a)).is_empty());
        let verbs = std::collections::BTreeSet::from(["vcrun2022".into()]);
        record_prereqs(&root, &a, &verbs).unwrap();
        assert_eq!(satisfied_prereqs_in(&root, Some(&a)), verbs);
        assert!(satisfied_prereqs_in(&root, Some(&b)).is_empty());
        assert!(satisfied_prereqs_in(&root, None).is_empty());
        fs::write(b.join("winetricks.log"), "dotnet8\n").unwrap();
        assert_eq!(
            satisfied_prereqs_in(&root, Some(&b)),
            std::collections::BTreeSet::from(["dotnet8".into()])
        );
        let alias = root.join("alias");
        std::os::unix::fs::symlink(&a, &alias).unwrap();
        assert_eq!(satisfied_prereqs_in(&root, Some(&alias)), verbs);
        fs::rename(&a, root.join("old-a")).unwrap();
        fs::create_dir(&a).unwrap();
        assert!(
            satisfied_prereqs_in(&root, Some(&a)).is_empty(),
            "recreated prefix must not inherit an old receipt"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn environ_identifies_the_owning_prefix() {
        // A real /proc/<pid>/environ: NUL-separated KEY=VALUE, no trailing sep
        // guarantees, and plenty of unrelated entries.
        let prefix = Path::new("/tmp/eidos-x/compatdata/489830/pfx");
        let compat = Path::new("/tmp/eidos-x/compatdata/489830");
        let env = |s: &str| s.replace('|', "\0").into_bytes();

        assert!(environ_owns_prefix(
            &env("LANG=C|WINEPREFIX=/tmp/eidos-x/compatdata/489830/pfx|PATH=/usr/bin"),
            prefix,
            compat
        ));
        // Wine processes carry Windows-style argv, so STEAM_COMPAT_DATA_PATH is
        // often the only Linux path in their environment.
        assert!(environ_owns_prefix(
            &env("STEAM_COMPAT_DATA_PATH=/tmp/eidos-x/compatdata/489830|X=1"),
            prefix,
            compat
        ));
        // A DIFFERENT game's prefix must not match.
        assert!(!environ_owns_prefix(
            &env("WINEPREFIX=/tmp/eidos-x/compatdata/22380/pfx"),
            prefix,
            compat
        ));
        // A mention of the path in an unrelated variable is not ownership.
        assert!(!environ_owns_prefix(
            &env("SOMETHING=/tmp/eidos-x/compatdata/489830/pfx"),
            prefix,
            compat
        ));
        assert!(!environ_owns_prefix(b"", prefix, compat));
    }

    #[test]
    fn microsoft_installer_exit_codes_are_not_all_failures() {
        // These installers report "done, restart required" and "already newer"
        // as distinct non-zero codes; treating them as failures aborts a prefix
        // setup that in fact completed.
        for ok in [0, 105, 194, 236, 1638, 3010] {
            assert!(installer_success(ok), "{ok} should be success");
        }
        for bad in [1, 5, 112, 1603] {
            assert!(!installer_success(bad), "{bad} should be failure");
        }
        // The codes a user is most likely to hit get an explanation.
        assert!(describe_installer_exit(5)
            .unwrap()
            .contains("still running"));
        assert!(describe_installer_exit(112).unwrap().contains("disk space"));
        assert!(describe_installer_exit(1603).is_none());
    }

    #[test]
    fn classifies_tier2_verbs() {
        assert!(is_tier2_verb("vcrun2022"));
        assert!(is_tier2_verb("dotnet8"));
        assert!(is_tier2_verb("dotnetdesktop8"));
        assert!(!is_tier2_verb("d3dx9_43")); // Tier 1 (bundled DLL)
        assert!(!is_tier2_verb("nonsense"));
    }

    #[test]
    fn refuses_a_proton_without_wine() {
        // A proton path whose dir has no files/bin/wine must error (never spawn
        // winetricks, which could fall back to a system wine against the prefix).
        let dir = std::env::temp_dir().join(format!("eidos-prq-{}-{}", std::process::id(), "1"));
        std::fs::create_dir_all(&dir).unwrap();
        let fake_proton = dir.join("proton");
        std::fs::write(&fake_proton, b"#!/bin/sh\n").unwrap();
        let r = install_tier2_verb(&fake_proton, Path::new("/tmp/pfx"), &[], "vcrun2022");
        assert!(r.is_err(), "must refuse when Proton's wine is absent");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod verbs_in_prefix_tests {
    use super::*;

    fn tmp() -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "eidos-vip-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_real_winetricks_log_is_read_verb_by_verb() {
        // Copied from a real prefix: package verbs and settings, mixed.
        let p = tmp();
        std::fs::write(
            p.join("winetricks.log"),
            "arial\nfontsmooth=rgb\nxact\nvcrun2022\nremove_mono internal\nwinxp\ndotnet48\ndotnetdesktop8\n",
        )
        .unwrap();
        let got = verbs_in_prefix(&p);
        assert!(got.contains("dotnetdesktop8"), "{got:?}");
        assert!(got.contains("vcrun2022"), "{got:?}");
        // A setting is logged like a package; only the name before `=` is a verb,
        // or `fontsmooth=rgb` would never match the verb `fontsmooth`.
        assert!(got.contains("fontsmooth"), "{got:?}");
        assert!(
            !got.iter().any(|v| v.contains('=')),
            "a setting leaked in: {got:?}"
        );
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn no_log_means_nothing_recorded_not_nothing_installed() {
        // The distinction matters: this feeds a "you already have it" check, and
        // over-reporting work is a nuisance while claiming a prerequisite is
        // absent when it is not sends the user to download what they have.
        let p = tmp();
        assert!(verbs_in_prefix(&p).is_empty());
        assert!(verbs_in_prefix(std::path::Path::new("/nonexistent/prefix")).is_empty());
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn blank_lines_and_comments_are_not_verbs() {
        let p = tmp();
        std::fs::write(p.join("winetricks.log"), "\n# a comment\n  \nvcrun2022\n\n").unwrap();
        let got = verbs_in_prefix(&p);
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got.contains("vcrun2022"));
        let _ = std::fs::remove_dir_all(&p);
    }
}
