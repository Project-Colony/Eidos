//! Provision the Wine prefix registry with what Bethesda modding tools expect.
//!
//! Windows tools locate a game by reading `HKLM\Software\Bethesda Softworks\<game>`
//! `installed path`, written by the game's own installer. Steam under Proton
//! never runs that installer, so the key does not exist and xEdit, Wrye Bash,
//! DynDOLOD and friends open on an empty game path and ask the user to browse -
//! or simply refuse to start. Writing the key ourselves is what MO2's users get
//! for free on Windows.
//!
//! Everything here is deliberately conservative about a prefix Eidos does not
//! own (it is Steam's): additive keys only, never a delete, guarded by a marker
//! so a launch does not re-run wine, and skipped entirely if the prefix has not
//! been initialised yet or is currently in use.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The xEdit family. These are 32-bit-era Delphi applications whose file dialogs
/// misbehave under Wine's default Windows version; `winxp` is the compatibility
/// mode the community settled on, and it is applied per-executable so nothing
/// else in the prefix is affected.
const XEDIT_EXES: &[&str] = &[
    "SSEEdit.exe",
    "SSEEdit64.exe",
    "TES5Edit.exe",
    "TES5Edit64.exe",
    "TES4Edit.exe",
    "FO4Edit.exe",
    "FO4Edit64.exe",
    "FO3Edit.exe",
    "FNVEdit.exe",
    "SF1Edit64.exe",
    "xEdit.exe",
    "xEdit64.exe",
];

/// Render the `.reg` blob. Pure, so the exact text is unit-testable without a
/// prefix or a wine process anywhere in sight.
///
/// `registry_name` is the game's own key under `Bethesda Softworks` and
/// `install_path` its real Linux directory, which Wine sees through the `Z:`
/// drive. The trailing backslash on the value is REQUIRED: several tools
/// concatenate a relative path onto it without inserting a separator.
///
/// Both the plain and the `Wow6432Node` views are written because Wine does not
/// mirror them: a 32-bit tool reads the latter and a 64-bit tool the former, and
/// the xEdit family ships in both flavours.
pub fn registry_blob(registry_name: &str, install_path: &Path) -> String {
    let win_path = to_windows_path(install_path);
    let mut out = String::from("Windows Registry Editor Version 5.00\r\n");
    if !registry_name.is_empty() {
        for view in ["Software", "Software\\Wow6432Node"] {
            out.push_str(&format!(
                "\r\n[HKEY_LOCAL_MACHINE\\{view}\\Bethesda Softworks\\{registry_name}]\r\n"
            ));
            out.push_str(&format!("\"installed path\"=\"{}\"\r\n", escape_reg(&win_path)));
        }
    }
    for exe in XEDIT_EXES {
        out.push_str(&format!("\r\n[HKEY_CURRENT_USER\\Software\\Wine\\AppDefaults\\{exe}]\r\n"));
        out.push_str("\"Version\"=\"winxp\"\r\n");
    }
    out
}

/// A Linux path as Wine sees it on the `Z:` drive, with a trailing backslash.
fn to_windows_path(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    let s = s.strip_prefix('\\').map(|r| format!("\\{r}")).unwrap_or(s);
    if s.ends_with('\\') {
        format!("Z:{s}")
    } else {
        format!("Z:{s}\\")
    }
}

/// Escape a value for a `.reg` string: backslashes and quotes are doubled.
fn escape_reg(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Whether this prefix already has the registry entries for `install_path`.
///
/// The marker records the exact path that was registered, so moving the game to
/// another drive re-runs the import instead of leaving a stale key behind.
fn marker_path(compatdata: &Path, registry_name: &str) -> PathBuf {
    let key = if registry_name.is_empty() { "xedit" } else { registry_name };
    compatdata.join(".eidos_registry").join(format!("{key}.v1"))
}

/// Write the registry entries into the prefix, unless they are already there.
///
/// Returns `Ok(true)` when an import ran, `Ok(false)` when it was unnecessary.
/// Callers treat a failure as non-fatal: a tool that cannot find its game path
/// is an inconvenience, not a reason to refuse the launch.
///
/// `proton_argv` builds the command for a `runinprefix` invocation - the verb
/// that runs a program in the existing prefix without Proton's game-drive setup.
pub fn ensure_registry(
    compatdata: &Path,
    install_path: &Path,
    registry_name: &str,
    proton_argv: impl FnOnce(&Path) -> Vec<String>,
    proton_env: &[(String, String)],
) -> io::Result<bool> {
    let prefix = compatdata.join("pfx");
    // Never touch an uninitialised prefix: Proton creates user.reg on first run,
    // and importing before that races its own bootstrap.
    if !prefix.join("user.reg").is_file() {
        return Ok(false);
    }
    // A live wineserver caches the registry in memory and rewrites it on
    // shutdown, so importing now would be silently undone. Refuse instead.
    if !crate::prefix_busy(&prefix, compatdata).is_empty() {
        return Ok(false);
    }

    let marker = marker_path(compatdata, registry_name);
    let want = to_windows_path(install_path);
    if fs::read_to_string(&marker).is_ok_and(|s| s.trim() == want) {
        return Ok(false);
    }

    let blob = registry_blob(registry_name, install_path);
    let reg_file = compatdata.join(".eidos_registry").join("eidos.reg");
    if let Some(parent) = reg_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&reg_file, &blob)?;

    let argv = proton_argv(&reg_file);
    let Some((program, args)) = argv.split_first() else {
        return Err(io::Error::other("empty Proton command"));
    };
    let mut cmd = Command::new(program);
    cmd.args(args)
        // Nothing may pop a dialog: this runs unattended, before the game.
        .env("WINEDLLOVERRIDES", "mshtml=d")
        .env("PROTON_USE_XALIA", "0")
        .env("WINEDEBUG", "-all");
    for (k, v) in proton_env {
        cmd.env(k, v);
    }
    let status = cmd.status()?;
    if !status.success() {
        return Err(io::Error::other(format!("regedit import failed: {status}")));
    }
    fs::write(&marker, &want)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_writes_both_registry_views_with_a_trailing_backslash() {
        let blob = registry_blob(
            "Skyrim Special Edition",
            Path::new("/mnt/Jeux/SteamLibrary/steamapps/common/Skyrim Special Edition"),
        );
        // Wine does not mirror the two views: 32-bit tools read Wow6432Node.
        assert!(blob.contains("[HKEY_LOCAL_MACHINE\\Software\\Bethesda Softworks\\Skyrim Special Edition]"));
        assert!(blob.contains(
            "[HKEY_LOCAL_MACHINE\\Software\\Wow6432Node\\Bethesda Softworks\\Skyrim Special Edition]"
        ));
        // Backslashes are doubled in a .reg value, and the path must END with one:
        // tools concatenate onto it without inserting a separator.
        assert!(blob.contains(
            r#""installed path"="Z:\\mnt\\Jeux\\SteamLibrary\\steamapps\\common\\Skyrim Special Edition\\""#
        ));
        // The xEdit compatibility block travels with it.
        assert!(blob.contains("[HKEY_CURRENT_USER\\Software\\Wine\\AppDefaults\\SSEEdit.exe]"));
        assert!(blob.contains("\"Version\"=\"winxp\""));
        assert!(blob.starts_with("Windows Registry Editor Version 5.00"));
    }

    #[test]
    fn a_game_without_a_bethesda_key_still_gets_the_xedit_block() {
        // Enderal has no key of its own; the writer must not emit an empty one.
        let blob = registry_blob("", Path::new("/games/Enderal"));
        assert!(!blob.contains("Bethesda Softworks"));
        assert!(blob.contains("AppDefaults\\SSEEdit.exe"));
    }

    #[test]
    fn windows_path_conversion_is_z_drive_with_one_trailing_separator() {
        assert_eq!(to_windows_path(Path::new("/a/b")), "Z:\\a\\b\\");
        // An already-terminated path must not gain a second separator.
        assert_eq!(to_windows_path(Path::new("/a/b/")), "Z:\\a\\b\\");
    }

    #[test]
    fn an_uninitialised_prefix_is_left_alone() {
        let dir = std::env::temp_dir().join(format!("eidos-reg-{}", std::process::id()));
        fs::create_dir_all(dir.join("pfx")).unwrap();
        // No user.reg: Proton has not bootstrapped this prefix yet.
        let ran = ensure_registry(
            &dir,
            Path::new("/games/Skyrim"),
            "Skyrim Special Edition",
            |_| vec!["/bin/false".to_string()],
            &[],
        )
        .unwrap();
        assert!(!ran, "must not import into a prefix Proton has not created yet");
        let _ = fs::remove_dir_all(&dir);
    }
}
