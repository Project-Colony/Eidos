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

/// The xEdit family, by base name. These are 32-bit-era Delphi applications
/// whose file dialogs misbehave under Wine's default Windows version; `winxp` is
/// the compatibility mode the community settled on, and it is applied
/// per-executable so nothing else in the prefix is affected.
const XEDIT_BASES: &[&str] = &[
    "SSEEdit",
    "SSEEdit64",
    "TES5Edit",
    "TES5Edit64",
    "TES4Edit",
    "FO4Edit",
    "FO4Edit64",
    "FO3Edit",
    "FNVEdit",
    "SF1Edit64",
    "xEdit",
    "xEdit64",
];

/// Every executable that should run in `winxp` mode: each base name, and its
/// `QuickAutoClean` sibling.
///
/// The siblings are DERIVED rather than listed, because listing them is how the
/// original list came to cover `SSEEdit.exe` but not `SSEEditQuickAutoClean.exe`
/// - and QuickAutoClean is the one a user actually runs, since cleaning the
/// official masters is a prerequisite of DynDOLOD and of most load-order guides.
/// A key for an executable that does not exist costs nothing: wine only consults
/// it if something by that name runs.
fn xedit_exes() -> impl Iterator<Item = String> {
    XEDIT_BASES
        .iter()
        .flat_map(|base| [format!("{base}.exe"), format!("{base}QuickAutoClean.exe")])
}

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
    for exe in xedit_exes() {
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
/// Bump the suffix whenever the BLOB gains entries that are not covered by the
/// value checks: a prefix written by an older Eidos then re-imports once instead
/// of keeping a blob that no longer says everything it should. `v2` added the
/// `QuickAutoClean` executables.
fn marker_path(compatdata: &Path, registry_name: &str) -> PathBuf {
    let key = if registry_name.is_empty() { "xedit" } else { registry_name };
    compatdata.join(".eidos_registry").join(format!("{key}.v2"))
}

/// Whether `system.reg` currently holds `want` in BOTH views of the game's key.
///
/// The marker alone is not enough. It answers "did Eidos write this?", and the
/// prefix is not ours: the game's own 32-bit launcher writes `installed path`
/// too, through whatever drive letter Wine happened to offer it. Observed on a
/// real prefix (2026-07-30): Eidos wrote `Z:\...\steamapps\common\...` to both
/// views, `SkyrimSELauncher.exe` later rewrote the `Wow6432Node` view as
/// `S:\common\...`, and Steam subsequently repointed `S:` from
/// `<library>/steamapps` to `<library>` - leaving a key that had been correct
/// when written and now resolved to a directory that does not exist. TexGen
/// then died with `EDirectoryNotFoundException`.
///
/// So the question has to be "is the key right NOW", which is what this asks.
/// Anything unparsed reads as "does not match", because re-importing is cheap
/// and idempotent while a wrong path costs the user a support thread.
fn registry_matches(system_reg: &str, registry_name: &str, want: &str) -> bool {
    if registry_name.is_empty() {
        // xEdit-only mode registers no game path, so there is nothing to check.
        return true;
    }
    let want_escaped = escape_reg(want);
    ["Software", "Software\\Wow6432Node"].iter().all(|view| {
        let header = format!("[{}]", escape_reg(&format!("{view}\\Bethesda Softworks\\{registry_name}")));
        let Some(section) = system_reg.split(&header).nth(1) else {
            return false;
        };
        // Stop at the next section so a later key's value cannot be mistaken
        // for this one's.
        section
            .split("\n[")
            .next()
            .unwrap_or("")
            .lines()
            .filter_map(|l| l.trim().strip_prefix("\"installed path\"="))
            .any(|v| v.trim().trim_matches('"') == want_escaped)
    })
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
    // Both must agree before skipping: the marker says we did the work, the
    // registry says the work is still there. Another program overwriting the key
    // is not hypothetical - see `registry_matches`. The prefix is idle here (the
    // busy check above returned nothing), so `system.reg` is authoritative
    // rather than a stale copy of what wineserver holds in memory.
    let already_written = fs::read_to_string(&marker).is_ok_and(|s| s.trim() == want);
    let still_correct = fs::read_to_string(prefix.join("system.reg"))
        .is_ok_and(|reg| registry_matches(&reg, registry_name, &want));
    if already_written && still_correct {
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

    // ---- "is the key still right?" ------------------------------------------
    //
    // Reproduced from a real prefix on 2026-07-30: Eidos wrote the correct Z:
    // path to both views, the game's 32-bit launcher later rewrote the
    // Wow6432Node view through the S: drive, and Steam then repointed S: one
    // directory up. The value stayed syntactically fine and became wrong.

    const WANT: &str = r"Z:\mnt\Jeux\SteamLibrary\steamapps\common\Skyrim Special Edition\";

    fn system_reg(plain: &str, wow: &str) -> String {
        format!(
            "WINE REGISTRY Version 2\n\n\
             [Software\\\\Bethesda Softworks\\\\Skyrim Special Edition] 1784981197\n\
             #time=1dd1c2e0b98867a\n\
             \"installed path\"=\"{plain}\"\n\n\
             [Software\\\\Borland\\\\Database Engine] 1775303033\n\
             \"SHAREDMEMLOCATION\"=\"9000\"\n\n\
             [Software\\\\Wow6432Node\\\\Bethesda Softworks\\\\Skyrim Special Edition] 1784982116\n\
             #time=1dd1c302f5cd1d6\n\
             \"installed path\"=\"{wow}\"\n"
        )
    }

    #[test]
    fn both_views_correct_is_a_match() {
        let esc = escape_reg(WANT);
        assert!(registry_matches(&system_reg(&esc, &esc), "Skyrim Special Edition", WANT));
    }

    #[test]
    fn the_exact_failure_that_killed_texgen_is_detected() {
        // Plain view still ours, Wow6432Node view rewritten via a drive letter
        // that has since moved. The marker would have said "done"; this must not.
        let reg = system_reg(&escape_reg(WANT), &escape_reg(r"S:\common\Skyrim Special Edition\"));
        assert!(
            !registry_matches(&reg, "Skyrim Special Edition", WANT),
            "a clobbered 32-bit view was reported as correct"
        );
    }

    #[test]
    fn a_missing_key_is_not_a_match() {
        assert!(!registry_matches("WINE REGISTRY Version 2\n", "Skyrim Special Edition", WANT));
    }

    #[test]
    fn a_value_from_a_neighbouring_key_is_not_borrowed() {
        // Our key present but EMPTY, with the right-looking value sitting in the
        // next section. Reading past the section boundary would pass this.
        let reg = format!(
            "WINE REGISTRY Version 2\n\n\
             [Software\\\\Bethesda Softworks\\\\Skyrim Special Edition] 1\n\n\
             [Software\\\\Wow6432Node\\\\Bethesda Softworks\\\\Skyrim Special Edition] 2\n\n\
             [Software\\\\Something Else] 3\n\
             \"installed path\"=\"{}\"\n",
            escape_reg(WANT)
        );
        assert!(!registry_matches(&reg, "Skyrim Special Edition", WANT));
    }

    #[test]
    fn a_different_game_in_the_same_prefix_is_not_confused_for_ours() {
        let esc = escape_reg(WANT);
        let reg = system_reg(&esc, &esc);
        assert!(!registry_matches(&reg, "Fallout4", WANT));
    }

    #[test]
    fn quick_auto_clean_gets_the_same_compatibility_mode() {
        // The one a user actually runs: cleaning the official masters is a
        // prerequisite of DynDOLOD and of most load-order guides, and it is a
        // SEPARATE executable from the editor. Listing the editors by hand is
        // how it was missed.
        let blob = registry_blob("Skyrim Special Edition", Path::new("/games/skyrim"));
        for exe in ["SSEEdit.exe", "SSEEditQuickAutoClean.exe", "FO4EditQuickAutoClean.exe"] {
            assert!(
                blob.contains(&format!("AppDefaults\\{exe}]")),
                "{exe} missing from the blob"
            );
        }
    }

    #[test]
    fn every_editor_has_a_cleaner_beside_it() {
        // The derivation, not a sample of it: each base must yield exactly two
        // entries, so adding a game to the list cannot half-cover it.
        let names: Vec<String> = xedit_exes().collect();
        assert_eq!(names.len(), XEDIT_BASES.len() * 2);
        for base in XEDIT_BASES {
            assert!(names.contains(&format!("{base}.exe")), "{base}");
            assert!(names.contains(&format!("{base}QuickAutoClean.exe")), "{base} cleaner");
        }
    }

    #[test]
    fn xedit_only_mode_has_nothing_to_verify() {
        // No game key is written in that mode, so demanding one would re-import
        // on every single launch.
        assert!(registry_matches("", "", WANT));
    }

    #[test]
    fn what_the_blob_writes_is_what_the_check_accepts() {
        // The producer and the verifier must agree on escaping, or Eidos
        // re-imports for ever. Build a system.reg the way wine would from our
        // own blob and require the check to pass.
        let path = Path::new("/mnt/Jeux/SteamLibrary/steamapps/common/Skyrim Special Edition");
        let blob = registry_blob("Skyrim Special Edition", path);
        // wine stores HKLM section names WITHOUT the hive prefix and WITH their
        // backslashes escaped - `[Software\\Bethesda Softworks\\...]`, as seen in
        // a real system.reg. Values are already escaped in the blob.
        let as_system_reg: String = blob
            .replace("\r\n", "\n")
            .lines()
            .map(|l| match l.strip_prefix("[HKEY_LOCAL_MACHINE\\").and_then(|r| r.strip_suffix(']')) {
                Some(key) => format!("[{}] 1\n", escape_reg(key)),
                None => format!("{l}\n"),
            })
            .collect();
        assert!(
            registry_matches(&as_system_reg, "Skyrim Special Edition", &to_windows_path(path)),
            "blob and check disagree:\n{as_system_reg}"
        );
    }
}
