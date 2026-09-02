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

/// The Flatpak application id of the official Steam package.
pub const STEAM_FLATPAK_ID: &str = "com.valvesoftware.Steam";

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
    /// This Proton belongs to the Flatpak Steam install. Eidos still runs it from
    /// the host (see [`is_flatpak_steam`]), but front ends warn, because its
    /// sandbox libraries may not resolve.
    pub flatpak: bool,
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
    crate::steam_roots(home)
        .into_iter()
        .find(|r| r.join("config/config.vdf").is_file())
        .map(|r| fs::canonicalize(&r).unwrap_or(r))
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
    let custom = steam_root
        .join("compatibilitytools.d")
        .join(name)
        .join("proton");
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
                if text
                    .lines()
                    .filter_map(|l| quoted_pair(l.trim()))
                    .any(|(_, v)| v == name)
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
        let Ok(rd) = fs::read_dir(lib.join("steamapps/common")) else {
            continue;
        };
        for e in rd.flatten() {
            let dir_name = e.file_name().to_string_lossy().into_owned();
            let hay: String = dir_name
                .to_ascii_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            let proton = e.path().join("proton");
            if proton.is_file()
                && (hay == needle || (needle.starts_with("proton") && hay.starts_with(&needle)))
            {
                return Some(proton);
            }
        }
    }
    None
}

/// The value for `STEAM_COMPAT_LIBRARY_PATHS`: the library ROOT, the directory
/// that HOLDS `steamapps`, e.g. `<lib>/steamapps/common/<game>` -> `<lib>`.
///
/// This is not a free choice. Proton hands the value straight to
/// `setup_dir_drive("gamedrive", "s:", ...)`, so the prefix's `S:` drive is
/// recreated to point at whatever we pass, on every single run - and Windows
/// programs find a Steam game by trying `<drive>\steamapps\common\<game>`,
/// which is the shape Steam's own libraries have. The root is what makes that
/// heuristic land on the game.
///
/// It returned `<lib>/steamapps` for one release and that was WRONG, proven by
/// behaviour rather than by inference: with `S:` one directory too low,
/// `S:\steamapps\common\Fallout 4` no longer existed, so BodySlide CREATED it
/// and wrote 267 MB of meshes into `<lib>/steamapps/steamapps/common/...` -
/// outside the union mount, invisible to Eidos, and never captured into
/// Overwrite. The doubled `steamapps` in that path is the whole story.
///
/// So: match Steam exactly. Eidos must not show the prefix a different world
/// than the one Steam shows it.
pub fn library_path(inside: &Path) -> Option<PathBuf> {
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
    let name = compat_tool_name(&root, app_id)
        .or_else(|| {
            // Fallback: the last tool that touched the prefix.
            fs::read_to_string(compatdata.join("version"))
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| config_info_name(compatdata))?;
    // config_info is the prefix's own record of the build that last ran against
    // it, so it is the right answer when the name lookup fails. There is
    // deliberately NO "any installed Proton" fallback beyond it: running a prefix
    // under a build the user never chose lets wineboot upgrade - or silently
    // DOWNGRADE - it, and a loud "could not resolve the Proton" beats quietly
    // rewriting the prefix with the wrong wineserver.
    let proton =
        find_proton_binary(&root, home, &name).or_else(|| proton_from_config_info(compatdata))?;

    let app = app_id.to_string();
    let mut env = vec![
        (
            "STEAM_COMPAT_DATA_PATH".to_string(),
            compatdata.to_string_lossy().into_owned(),
        ),
        (
            "STEAM_COMPAT_CLIENT_INSTALL_PATH".to_string(),
            root.to_string_lossy().into_owned(),
        ),
        // Steam sets all three for compat launches; tools using steam_api (e.g. a
        // script extender launching the game) and GE-Proton's protonfixes need the
        // app id in the environment, which MO2 sets on every spawn.
        ("SteamAppId".to_string(), app.clone()),
        ("SteamGameId".to_string(), app.clone()),
        ("STEAM_COMPAT_APP_ID".to_string(), app),
        // The game's install dir: Proton's drive setup (re)creates the prefix's
        // `s:` gamedrive from this instead of deleting it.
        (
            "STEAM_COMPAT_INSTALL_PATH".to_string(),
            install_path.to_string_lossy().into_owned(),
        ),
    ];
    if let Some(lib) = library_path(install_path).or_else(|| library_path(compatdata)) {
        env.push((
            "STEAM_COMPAT_LIBRARY_PATHS".to_string(),
            lib.to_string_lossy().into_owned(),
        ));
    }
    let flatpak = is_flatpak_steam(&proton) || is_flatpak_steam(&root);
    Some(ProtonRun {
        proton,
        env,
        flatpak,
    })
}

/// Resolve a Proton from the prefix's `config_info`, which Proton itself writes.
///
/// Line 1 is the build's own declared name and lines 2+ are paths into its
/// `files/` or `dist/` tree. The name can disagree with the directory it is
/// installed in (distro-repackaged builds do this), so the paths are the
/// reliable half: walk one up from the `files/`/`dist/` component to reach the
/// tool directory and take its `proton`. Returns the NAME for the normal lookup
/// path; [`proton_from_config_info`] returns the binary directly.
fn config_info_name(compatdata: &Path) -> Option<String> {
    let text = fs::read_to_string(compatdata.join("config_info")).ok()?;
    text.lines()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The `proton` binary named by a prefix's `config_info` path lines, if it exists.
fn proton_from_config_info(compatdata: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(compatdata.join("config_info")).ok()?;
    for line in text.lines().skip(1).map(str::trim) {
        let p = Path::new(line);
        // .../GE-Proton10-34/files/lib/... -> .../GE-Proton10-34/proton
        let tool_dir = p
            .ancestors()
            .find(|a| {
                matches!(
                    a.file_name().and_then(|n| n.to_str()),
                    Some("files") | Some("dist")
                )
            })
            .and_then(Path::parent);
        if let Some(dir) = tool_dir {
            let proton = dir.join("proton");
            if proton.is_file() {
                return Some(proton);
            }
        }
    }
    None
}

/// Whether this path belongs to the Flatpak Steam installation.
///
/// Deliberately a DIAGNOSTIC, not a switch. Flatpak Steam ships Proton with its
/// runtime and steamclient libraries inside the sandbox, so running that Proton
/// bare from the host can fail to resolve them. The obvious fix - re-launching
/// through `flatpak run` - is wrong for Eidos: the game would start in Flatpak's
/// own sandbox, which cannot see the FUSE union mounted in our private mount
/// namespace, and it would silently play VANILLA. So Eidos warns clearly and
/// keeps the mount, rather than trading a loud failure for a silent one.
pub fn is_flatpak_steam(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == STEAM_FLATPAK_ID)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_library_path_is_the_root_that_holds_steamapps() {
        use super::library_path;
        use std::path::{Path, PathBuf};
        // Proton turns this into the prefix's `S:` drive, and Windows programs
        // find a Steam game by trying `S:\steamapps\common\<game>`. Return the
        // `steamapps` dir itself and that lands one level too deep: BodySlide
        // then CREATES `<lib>/steamapps/steamapps/common/<game>/Data` and writes
        // its output outside the mount, where Eidos never sees it.
        let want = PathBuf::from("/mnt/Jeux/SteamLibrary");
        assert_eq!(
            library_path(Path::new(
                "/mnt/Jeux/SteamLibrary/steamapps/common/Fallout 4"
            )),
            Some(want.clone())
        );
        assert_eq!(
            library_path(Path::new(
                "/mnt/Jeux/SteamLibrary/steamapps/compatdata/377160"
            )),
            Some(want.clone())
        );
        // The property that actually matters, stated as the heuristic itself.
        assert_eq!(
            library_path(Path::new(
                "/mnt/Jeux/SteamLibrary/steamapps/common/Fallout 4"
            ))
            .map(|l| l.join("steamapps/common/Fallout 4")),
            Some(PathBuf::from(
                "/mnt/Jeux/SteamLibrary/steamapps/common/Fallout 4"
            )),
            "S:\\steamapps\\common\\<game> must resolve to the real install"
        );
        // Nothing to say about a path that is not inside a library at all.
        assert_eq!(
            library_path(Path::new("/mnt/Jeux/Eidos-Fallout4/mods")),
            None
        );
    }

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
        assert_eq!(
            compat_tool_name(&root, 489830).as_deref(),
            Some("GE-Proton10-34")
        );
        assert_eq!(
            compat_tool_name(&root, 22380).as_deref(),
            Some("proton_experimental")
        );
        // Unmapped app -> the "0" global default.
        assert_eq!(
            compat_tool_name(&root, 999999).as_deref(),
            Some("GE-Proton10-34")
        );
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
        assert_eq!(
            env.get("STEAM_COMPAT_DATA_PATH").map(String::as_str),
            Some(compat.to_str().unwrap())
        );
        assert!(env.contains_key("STEAM_COMPAT_CLIENT_INSTALL_PATH"));
        // MO2 sets the Steam app id on every spawn; Proton keys per-app fixes on it.
        assert_eq!(env.get("SteamAppId").map(String::as_str), Some("489830"));
        // The install dir lets Proton's drive setup repair s: instead of deleting it.
        assert_eq!(
            env.get("STEAM_COMPAT_INSTALL_PATH").map(String::as_str),
            Some(install.to_str().unwrap())
        );
        // The library ROOT, so that `S:\steamapps\common\<game>` - the way a
        // Windows program looks for a Steam game - resolves to the real install.
        assert_eq!(
            env.get("STEAM_COMPAT_LIBRARY_PATHS").map(String::as_str),
            Some(steam.to_str().unwrap())
        );

        let argv = run.command(
            Path::new("C:/Tools/xEdit/SSEEdit.exe"),
            &["-quickautoclean".to_string()],
        );
        // Steam's main-app verb, not the bare `run` (which deletes the s: drive).
        assert_eq!(argv[1], "waitforexitandrun");
        assert!(argv[2].ends_with("SSEEdit.exe"));
        assert_eq!(argv[3], "-quickautoclean");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn flatpak_steam_paths_are_recognised() {
        // The predicate drives a WARNING, never a re-launch: running the game
        // through `flatpak run` would hide the FUSE mount from it.
        assert!(is_flatpak_steam(Path::new(
            "/home/u/.var/app/com.valvesoftware.Steam/.local/share/Steam/compatibilitytools.d/GE/proton"
        )));
        assert!(is_flatpak_steam(Path::new(
            "/home/u/.var/app/com.valvesoftware.Steam/data/Steam"
        )));
        // A native install must not be flagged.
        assert!(!is_flatpak_steam(Path::new(
            "/home/u/.local/share/Steam/compatibilitytools.d/GE-Proton10-34/proton"
        )));
        assert!(!is_flatpak_steam(Path::new(
            "/mnt/Jeux/SteamLibrary/steamapps/common/Proton 9.0"
        )));
    }

    #[test]
    fn config_info_resolves_a_build_whose_name_lies() {
        // Distro-repackaged builds declare a name that does not match the folder
        // they live in, so the PATH lines are the reliable half.
        let root = tmp_root();
        let tool = root.join("compatibilitytools.d/GE-Proton10-34");
        fs::create_dir_all(tool.join("files/lib")).unwrap();
        fs::write(tool.join("proton"), "#!/bin/sh\n").unwrap();
        let compat = root.join("compatdata/489830");
        fs::create_dir_all(&compat).unwrap();
        fs::write(
            compat.join("config_info"),
            format!("CachyOS-11.0-100\n{}/files/lib/wine/\n", tool.display()),
        )
        .unwrap();

        // Line 1 is the (mismatching) declared name...
        assert_eq!(
            config_info_name(&compat).as_deref(),
            Some("CachyOS-11.0-100")
        );
        // ...but the path lines still find the real binary.
        assert_eq!(
            proton_from_config_info(&compat).as_deref(),
            Some(tool.join("proton").as_path())
        );
        let _ = fs::remove_dir_all(&root);
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
