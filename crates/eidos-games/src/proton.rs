//! Resolve the Proton invocation for a Steam app, so Eidos can run an arbitrary
//! Windows tool (xEdit, FNIS, BodySlide) inside the game's prefix WITHOUT going
//! through Steam's `%command%` - the same trick protontricks uses.
//!
//! Resolution: `config.vdf` `CompatToolMapping` names the compat tool for the
//! app (falling back to the `"0"` global default, then to the prefix's own
//! `version` file). The tool's `proton` binary lives either in
//! `compatibilitytools.d/<name>/` (GE and other custom builds) or in a Steam
//! library's `steamapps/common/` (official "Proton X.Y" / "Proton - Experimental").
//! The invocation is then `proton waitforexitandrun <exe>` (Steam's main-app verb)
//! with `STEAM_COMPAT_DATA_PATH`, `STEAM_COMPAT_CLIENT_INSTALL_PATH`, the Steam app
//! id, and `STEAM_COMPAT_INSTALL_PATH` set - the last so Proton's drive setup
//! repairs the prefix's `s:` gamedrive instead of tearing it down.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{quoted_pair, steam_libraries};

/// A ready-to-spawn Proton invocation for one app.
#[derive(Debug, Clone)]
pub struct ProtonRun {
    /// The `proton` entry script (run as `proton waitforexitandrun <exe> [args...]`).
    pub proton: PathBuf,
    /// Environment Proton needs: `STEAM_COMPAT_DATA_PATH` (the app's compatdata),
    /// `STEAM_COMPAT_CLIENT_INSTALL_PATH` (the Steam root), the Steam app id
    /// (`SteamAppId`/`SteamGameId`/`STEAM_COMPAT_APP_ID`), and
    /// `STEAM_COMPAT_INSTALL_PATH`/`STEAM_COMPAT_LIBRARY_PATHS` (the game's dir/library).
    pub env: Vec<(String, String)>,
}

impl ProtonRun {
    /// The full argv to run `exe` (a Windows executable) through this Proton.
    ///
    /// Uses Steam's main-app verb `waitforexitandrun` (not the bare `run`): `run`
    /// executes Proton's `setup_game_dir_drive`, which - without
    /// `STEAM_COMPAT_INSTALL_PATH` - DELETES the prefix's `s:` gamedrive symlink,
    /// and it skips the `wineserver -w` wait that protects a launch right after a
    /// previous session. `waitforexitandrun` is what Steam itself invokes for the
    /// game, matching Eidos's exclusive single-process tool launch.
    pub fn command(&self, exe: &Path, args: &[String]) -> Vec<String> {
        let mut v = vec![
            self.proton.to_string_lossy().into_owned(),
            "waitforexitandrun".to_string(),
            exe.to_string_lossy().into_owned(),
        ];
        v.extend(args.iter().cloned());
        v
    }
}

/// The Steam root (the install holding `config/config.vdf` and
/// `compatibilitytools.d`), canonicalized.
pub fn steam_root(home: &Path) -> Option<PathBuf> {
    let roots = [
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".steam/root"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ];
    roots
        .iter()
        .find(|r| r.join("config/config.vdf").is_file())
        .map(|r| fs::canonicalize(r).unwrap_or_else(|_| r.to_path_buf()))
}

/// The compat-tool name configured for `app_id` in `config.vdf`
/// (`CompatToolMapping`), falling back to the `"0"` global-default entry.
fn compat_tool_name(steam_root: &Path, app_id: u32) -> Option<String> {
    let text = fs::read_to_string(steam_root.join("config/config.vdf")).ok()?;
    let mut in_mapping = false;
    let mut depth_at_mapping = 0usize;
    let mut depth = 0usize;
    let mut current_app: Option<String> = None;
    let (mut for_app, mut for_default) = (None, None);

    for line in text.lines() {
        let t = line.trim();
        if !in_mapping {
            if t.eq_ignore_ascii_case("\"CompatToolMapping\"") {
                in_mapping = true;
                depth_at_mapping = depth;
            }
        } else if depth == depth_at_mapping + 1 {
            // App-id level: a lone quoted token names the next app block.
            if t.starts_with('"') && t.ends_with('"') && !t.contains('\t') && t.len() > 2 {
                current_app = Some(t.trim_matches('"').to_string());
            }
        } else if depth == depth_at_mapping + 2 {
            if let Some((k, v)) = quoted_pair(t) {
                if k == "name" && !v.is_empty() {
                    match current_app.as_deref() {
                        Some(id) if id == app_id.to_string() => for_app = Some(v.to_string()),
                        Some("0") => for_default = Some(v.to_string()),
                        _ => {}
                    }
                }
            }
        }
        depth += t.matches('{').count();
        depth = depth.saturating_sub(t.matches('}').count());
        if in_mapping && depth <= depth_at_mapping && t.contains('}') {
            break; // left the CompatToolMapping block
        }
    }
    for_app.or(for_default)
}

/// Locate the `proton` binary for a compat-tool `name`: custom builds in
/// `compatibilitytools.d` first (by folder name, then by each tool's declared
/// internal name), then official Protons in the libraries' `steamapps/common`
/// (matched loosely: `proton_experimental` -> "Proton - Experimental").
fn find_proton_binary(steam_root: &Path, home: &Path, name: &str) -> Option<PathBuf> {
    // 1. compatibilitytools.d/<name>/proton (GE-Proton10-34 etc.)
    let custom = steam_root.join("compatibilitytools.d").join(name).join("proton");
    if custom.is_file() {
        return Some(custom);
    }
    // 2. Any compatibilitytools.d entry whose compatibilitytool.vdf declares `name`.
    if let Ok(rd) = fs::read_dir(steam_root.join("compatibilitytools.d")) {
        for e in rd.flatten() {
            let vdf = e.path().join("compatibilitytool.vdf");
            let proton = e.path().join("proton");
            if !proton.is_file() {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&vdf) {
                if text.lines().filter_map(|l| quoted_pair(l.trim())).any(|(_, v)| v == name)
                    || text.contains(&format!("\"{name}\""))
                {
                    return Some(proton);
                }
            }
        }
    }
    // 3. Official Protons live as Steam apps: match a common/<dir> loosely.
    //    "proton_experimental" -> "Proton - Experimental", "proton_9" -> "Proton 9.0".
    let needle: String = name
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    for lib in steam_libraries(home) {
        let Ok(rd) = fs::read_dir(lib.join("steamapps/common")) else { continue };
        for e in rd.flatten() {
            let dir_name = e.file_name().to_string_lossy().into_owned();
            let hay: String = dir_name
                .to_ascii_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            let proton = e.path().join("proton");
            if proton.is_file() && (hay == needle || (needle.starts_with("proton") && hay.starts_with(&needle))) {
                return Some(proton);
            }
        }
    }
    None
}

/// The Steam library root (the dir holding `steamapps`) for a path inside a
/// library, e.g. `<lib>/steamapps/common/<game>` -> `<lib>`. Used for
/// `STEAM_COMPAT_LIBRARY_PATHS`.
fn library_root(inside: &Path) -> Option<PathBuf> {
    inside
        .ancestors()
        .find(|a| a.file_name().map(|n| n == "steamapps").unwrap_or(false))
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

/// The Proton invocation for an app: tool name from `config.vdf` (falling back
/// to the prefix's `version` file), binary + env resolved. `compatdata` is the
/// app's `steamapps/compatdata/<appid>` dir and `install_path` its
/// `steamapps/common/<game>` dir (Eidos detection already has both).
pub fn proton_command(
    home: &Path,
    app_id: u32,
    compatdata: &Path,
    install_path: &Path,
) -> Option<ProtonRun> {
    let root = steam_root(home)?;
    let name = compat_tool_name(&root, app_id).or_else(|| {
        // Fallback: the last tool that touched the prefix.
        fs::read_to_string(compatdata.join("version"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })?;
    let proton = find_proton_binary(&root, home, &name)?;

    let app = app_id.to_string();
    let mut env = vec![
        ("STEAM_COMPAT_DATA_PATH".to_string(), compatdata.to_string_lossy().into_owned()),
        ("STEAM_COMPAT_CLIENT_INSTALL_PATH".to_string(), root.to_string_lossy().into_owned()),
        // Steam sets all three for compat launches; tools using steam_api (e.g. a
        // script extender launching the game) and GE-Proton's protonfixes need the
        // app id in the environment, which MO2 sets on every spawn.
        ("SteamAppId".to_string(), app.clone()),
        ("SteamGameId".to_string(), app.clone()),
        ("STEAM_COMPAT_APP_ID".to_string(), app),
        // The game's install dir: Proton's drive setup (re)creates the prefix's
        // `s:` gamedrive from this instead of deleting it.
        ("STEAM_COMPAT_INSTALL_PATH".to_string(), install_path.to_string_lossy().into_owned()),
    ];
    if let Some(lib) = library_root(install_path).or_else(|| library_root(compatdata)) {
        env.push(("STEAM_COMPAT_LIBRARY_PATHS".to_string(), lib.to_string_lossy().into_owned()));
    }
    Some(ProtonRun { proton, env })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp_root() -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("eidos-proton-{}-{}", std::process::id(), n));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// A config.vdf shaped like the real one (app mapping + "0" default).
    const VDF: &str = r#""InstallConfigStore"
{
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"CompatToolMapping"
				{
					"0"
					{
						"name"		"GE-Proton10-34"
						"config"		""
						"priority"		"75"
					}
					"489830"
					{
						"name"		"GE-Proton10-34"
						"config"		""
						"priority"		"250"
					}
					"22380"
					{
						"name"		"proton_experimental"
						"config"		""
						"priority"		"250"
					}
				}
			}
		}
	}
}
"#;

    fn fake_steam(root: &Path) {
        fs::create_dir_all(root.join("config")).unwrap();
        fs::write(root.join("config/config.vdf"), VDF).unwrap();
        // A GE build in compatibilitytools.d.
        let ge = root.join("compatibilitytools.d/GE-Proton10-34");
        fs::create_dir_all(&ge).unwrap();
        fs::write(ge.join("proton"), "#!/bin/sh\n").unwrap();
        // An official Proton in this root's own library.
        let official = root.join("steamapps/common/Proton - Experimental");
        fs::create_dir_all(&official).unwrap();
        fs::write(official.join("proton"), "#!/bin/sh\n").unwrap();
        fs::create_dir_all(root.join("steamapps")).unwrap();
    }

    #[test]
    fn maps_app_to_tool_with_default_fallback() {
        let root = tmp_root();
        fake_steam(&root);
        assert_eq!(compat_tool_name(&root, 489830).as_deref(), Some("GE-Proton10-34"));
        assert_eq!(compat_tool_name(&root, 22380).as_deref(), Some("proton_experimental"));
        // Unmapped app -> the "0" global default.
        assert_eq!(compat_tool_name(&root, 999999).as_deref(), Some("GE-Proton10-34"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_custom_and_official_binaries() {
        let root = tmp_root();
        fake_steam(&root);
        // The fake home: steam root at .local/share/Steam so steam_libraries sees it.
        let home = tmp_root();
        let link = home.join(".local/share");
        fs::create_dir_all(&link).unwrap();
        // copy the fake root under home (symlink would also do)
        let steam = link.join("Steam");
        fs::create_dir_all(&steam).unwrap();
        fs_extra_copy(&root, &steam);

        let ge = find_proton_binary(&steam, &home, "GE-Proton10-34").unwrap();
        assert!(ge.ends_with("compatibilitytools.d/GE-Proton10-34/proton"));
        let official = find_proton_binary(&steam, &home, "proton_experimental").unwrap();
        assert!(official.to_string_lossy().contains("Proton - Experimental"));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn proton_command_builds_env_and_argv() {
        let home = tmp_root();
        let steam = home.join(".local/share/Steam");
        fs::create_dir_all(&steam).unwrap();
        fake_steam(&steam);
        let compat = steam.join("steamapps/compatdata/489830");
        fs::create_dir_all(&compat).unwrap();
        let install = steam.join("steamapps/common/Skyrim Special Edition");
        fs::create_dir_all(&install).unwrap();

        let run = proton_command(&home, 489830, &compat, &install).unwrap();
        assert!(run.proton.ends_with("GE-Proton10-34/proton"));
        let env: std::collections::HashMap<_, _> = run.env.iter().cloned().collect();
        assert_eq!(env.get("STEAM_COMPAT_DATA_PATH").map(String::as_str), Some(compat.to_str().unwrap()));
        assert!(env.contains_key("STEAM_COMPAT_CLIENT_INSTALL_PATH"));
        // MO2 sets the Steam app id on every spawn; Proton keys per-app fixes on it.
        assert_eq!(env.get("SteamAppId").map(String::as_str), Some("489830"));
        // The install dir lets Proton's drive setup repair s: instead of deleting it.
        assert_eq!(env.get("STEAM_COMPAT_INSTALL_PATH").map(String::as_str), Some(install.to_str().unwrap()));
        // Library root derived from the install dir (its `steamapps` parent).
        assert_eq!(env.get("STEAM_COMPAT_LIBRARY_PATHS").map(String::as_str), Some(steam.to_str().unwrap()));

        let argv = run.command(Path::new("C:/Tools/xEdit/SSEEdit.exe"), &["-quickautoclean".to_string()]);
        // Steam's main-app verb, not the bare `run` (which deletes the s: drive).
        assert_eq!(argv[1], "waitforexitandrun");
        assert!(argv[2].ends_with("SSEEdit.exe"));
        assert_eq!(argv[3], "-quickautoclean");
        let _ = fs::remove_dir_all(&home);
    }

    /// Minimal recursive copy for the test fixture.
    fn fs_extra_copy(from: &Path, to: &Path) {
        for e in fs::read_dir(from).unwrap().flatten() {
            let dst = to.join(e.file_name());
            if e.path().is_dir() {
                fs::create_dir_all(&dst).unwrap();
                fs_extra_copy(&e.path(), &dst);
            } else {
                fs::copy(e.path(), &dst).unwrap();
            }
        }
    }
}
