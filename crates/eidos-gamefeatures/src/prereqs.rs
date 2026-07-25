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

/// Whether `verb` is a Tier-2 installer verb (runs winetricks, downloads).
pub fn is_tier2_verb(verb: &str) -> bool {
    TIER2_VERBS.contains(&verb)
}

/// Locate a program on `PATH`. `None` if not installed.
fn on_path(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).map(|d| d.join(program)).find(|p| p.is_file())
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
        io::Error::new(io::ErrorKind::NotFound, format!("malformed Proton path: {}", proton.display()))
    })?;
    // Valve's official builds put wine under `files/bin`; several community and
    // distro-repackaged builds use `dist/bin` instead. Try both before giving up,
    // or a perfectly good Proton is rejected on layout alone.
    let (wine, wineserver) = ["files/bin", "dist/bin"]
        .iter()
        .map(|d| (proton_dir.join(d).join("wine"), proton_dir.join(d).join("wineserver")))
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
        .env("WINEDEBUG", "-all");
    // Parity with a real Proton launch: the STEAM_COMPAT_* vars Proton sets.
    for (k, v) in steam_env {
        cmd.env(k, v);
    }

    let status = cmd.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("winetricks {verb} exited with {status}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let dir = std::env::temp_dir()
            .join(format!("eidos-prq-{}-{}", std::process::id(), "1"));
        std::fs::create_dir_all(&dir).unwrap();
        let fake_proton = dir.join("proton");
        std::fs::write(&fake_proton, b"#!/bin/sh\n").unwrap();
        let r = install_tier2_verb(&fake_proton, Path::new("/tmp/pfx"), &[], "vcrun2022");
        assert!(r.is_err(), "must refuse when Proton's wine is absent");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
