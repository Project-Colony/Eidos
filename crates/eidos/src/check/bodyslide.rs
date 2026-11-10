//! A49: BodySlide's data paths, and Data folders a tool wrote into Overwrite/Root.

use std::fs;
use std::path::{Path, PathBuf};

use eidos_gamefeatures::preflight::merged_data;
use eidos_games::DetectedGame;
use eidos_instance::Instance;

use super::{Finding, Level};

pub const CONFIG: &str = "CalienteTools/BodySlide/Config.xml";

/// BodySlide's own file is a few KiB; anything larger is a mod's file, not its settings.
const MAX_CONFIG: u64 = 1 << 20;

/// The exe's file name, lowercased, contains "bodyslide", "outfitstudio" or "outfit studio".
pub fn is_bodyslide(exe: &Path) -> bool {
    let name = exe
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    ["bodyslide", "outfitstudio", "outfit studio"]
        .iter()
        .any(|k| name.contains(k))
}

/// Direct children of the root element named GameDataPath or OutputDataPath with
/// non-empty trimmed text, in that order. Not XML, or no such child: empty.
pub fn config_paths(xml: &str) -> Vec<(&'static str, String)> {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return Vec::new();
    };
    ["GameDataPath", "OutputDataPath"]
        .into_iter()
        .filter_map(|name| {
            let node = doc
                .root_element()
                .children()
                .find(|c| c.has_tag_name(name))?;
            let text = node.text()?.trim();
            (!text.is_empty()).then(|| (name, text.to_string()))
        })
        .collect()
}

/// `X:\a\b\` during a tool run: `S:` is `s_drive` when given, other letters
/// `wine_drive_root`; backslashes become '/', trailing separators dropped.
/// None for a value without `<letter>:\`, no prefix, or an unmapped letter.
pub fn tool_path(value: &str, prefix: Option<&Path>, s_drive: Option<&Path>) -> Option<PathBuf> {
    let b = value.as_bytes();
    if b.len() < 3 || !b[0].is_ascii_alphabetic() || b[1] != b':' || b[2] != b'\\' {
        return None;
    }
    let prefix = prefix?;
    let drive = b[0].to_ascii_lowercase();
    let root = match s_drive {
        Some(s) if drive == b's' => s.to_path_buf(),
        _ => eidos_games::wine_drive_root(prefix, drive).ok()??,
    };
    // Leading separators too: a doubled `\\` must not make the rest absolute.
    let rest = value[3..].replace('\\', "/");
    let rest = rest.trim_matches('/');
    Some(if rest.is_empty() {
        root
    } else {
        root.join(rest)
    })
}

pub struct BodySlideFacts {
    /// The winning Config.xml.
    pub config: Option<PathBuf>,
    /// Setting name, raw value, and where it leads during a tool run.
    pub settings: Vec<(&'static str, String, Option<PathBuf>)>,
    pub data: PathBuf,
    pub mods_dir: PathBuf,
    pub root_overwrite: PathBuf,
    /// Overwrite/Root children named meshes or textures (any case), as found.
    pub root_data_dirs: Vec<String>,
}

pub fn bodyslide_facts(
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
) -> BodySlideFacts {
    let root_overwrite = inst.root_overwrite_dir();
    let mut facts = BodySlideFacts {
        config: None,
        settings: Vec::new(),
        data: game.data_path.clone(),
        mods_dir: inst.mods_dir(),
        root_data_dirs: Vec::new(),
        root_overwrite,
    };
    // BodySlide runs under Proton: no prefix, no BodySlide run to check.
    if let Some(prefix) = game.prefix() {
        let files = crate::stock_game::game_files(game, inst);
        facts.config = merged_data(&files.data, mods, &inst.overwrite_dir()).resolve(CONFIG);
        let xml = facts
            .config
            .as_ref()
            .filter(|p| fs::metadata(p).is_ok_and(|m| m.len() <= MAX_CONFIG))
            .and_then(|p| fs::read(p).ok());
        let s_drive = eidos_games::library_path(&game.install_path);
        if let Some(xml) = xml {
            facts.settings = config_paths(&String::from_utf8_lossy(&xml))
                .into_iter()
                .map(|(name, value)| {
                    let path = tool_path(&value, Some(&prefix), s_drive.as_deref());
                    (name, value, path)
                })
                .collect();
        }
    }
    // A game whose Data is its install folder has no Root to misplace into.
    if game.data_path != game.install_path {
        if let Ok(entries) = fs::read_dir(&facts.root_overwrite) {
            facts.root_data_dirs = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.eq_ignore_ascii_case("meshes") || n.eq_ignore_ascii_case("textures"))
                .collect();
            facts.root_data_dirs.sort();
        }
    }
    facts
}

/// The path as Wine sees it: canonicalized from its deepest existing ancestor,
/// lowercase (Wine resolves names case-insensitively), no trailing '/'.
fn folder_key(p: &Path) -> String {
    let p = p
        .ancestors()
        .find_map(|a| Some(fs::canonicalize(a).ok()?.join(p.strip_prefix(a).ok()?)))
        .unwrap_or_else(|| p.to_path_buf());
    p.to_string_lossy().trim_end_matches('/').to_lowercase()
}

pub fn bodyslide_findings(f: &BodySlideFacts) -> Vec<Finding> {
    let data = folder_key(&f.data);
    let mods = format!("{}/", folder_key(&f.mods_dir));
    let want = format!("Z:{}\\", f.data.display()).replace('/', "\\");
    let config = f
        .config
        .as_deref()
        .unwrap_or(Path::new(CONFIG))
        .display()
        .to_string();
    let mut out: Vec<Finding> = f
        .settings
        .iter()
        .filter(|(name, _, path)| {
            // An unresolved path (None) is reported.
            let Some(key) = path.as_deref().map(folder_key) else {
                return true;
            };
            // An OutputDataPath inside mods/ builds into an output mod.
            !(key == data || (*name == "OutputDataPath" && key.starts_with(&mods)))
        })
        .map(|(name, value, _)| Finding {
            code: "bodyslide_data_path",
            level: Level::Advice,
            args: vec![
                ("setting", name.to_string()),
                ("value", value.clone()),
                ("want", want.clone()),
                ("config", config.clone()),
            ],
        })
        .collect();
    if !f.root_data_dirs.is_empty() {
        out.push(Finding {
            code: "overwrite_root_data",
            level: Level::Advice,
            args: vec![
                ("dirs", f.root_data_dirs.join(", ")),
                ("path", f.root_overwrite.display().to_string()),
            ],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Tmp(PathBuf);
    impl Tmp {
        fn new() -> Self {
            static N: AtomicUsize = AtomicUsize::new(0);
            let p = std::env::temp_dir().join(format!(
                "eidos-bodyslide-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Facts with `data` = `<tmp>/game/Data` and `mods_dir` = `<tmp>/instance/mods`, both created.
    fn facts(tmp: &Tmp, settings: Vec<(&'static str, String, Option<PathBuf>)>) -> BodySlideFacts {
        let data = tmp.0.join("game/Data");
        let mods_dir = tmp.0.join("instance/mods");
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&mods_dir).unwrap();
        BodySlideFacts {
            config: None,
            settings,
            data,
            mods_dir,
            root_overwrite: tmp.0.join("instance/overwrite/Root"),
            root_data_dirs: vec![],
        }
    }

    #[test]
    fn config_paths_reads_the_two_settings_and_unescapes() {
        let xml = "<Config>\n<GameDataPath>Z:\\Games\\Mods &amp; More\\Data\\</GameDataPath>\n\
                   <GameDataPaths><SkyrimSpecialEdition>C:\\x\\</SkyrimSpecialEdition></GameDataPaths>\n<OutputDataPath/>\n</Config>";
        assert_eq!(
            config_paths(xml),
            [("GameDataPath", "Z:\\Games\\Mods & More\\Data\\".to_string())]
        );
        assert!(config_paths("not xml").is_empty());
    }

    #[test]
    fn s_drive_paths_resolve_to_the_library_root_eidos_gives_tools() {
        let tmp = Tmp::new();
        let pfx = tmp.0.join("pfx");
        let lib = tmp.0.join("lib");
        fs::create_dir_all(pfx.join("dosdevices")).unwrap();
        fs::create_dir_all(lib.join("steamapps")).unwrap();
        // What Steam leaves after a game launch.
        std::os::unix::fs::symlink(lib.join("steamapps"), pfx.join("dosdevices/s:")).unwrap();
        assert_eq!(
            tool_path(
                "S:\\steamapps\\common\\Skyrim Special Edition\\Data\\",
                Some(&pfx),
                Some(&lib)
            ),
            Some(lib.join("steamapps/common/Skyrim Special Edition/Data"))
        );
        assert_eq!(
            tool_path("Z:\\mnt\\x\\", Some(&pfx), None),
            Some(PathBuf::from("/mnt/x"))
        );
        assert_eq!(tool_path("relative\\x", Some(&pfx), None), None);
        assert_eq!(tool_path("Z:\\mnt\\x", None, None), None);
    }

    #[test]
    fn a_data_path_one_level_too_high_is_reported_with_the_value_to_set() {
        let tmp = Tmp::new();
        let data = tmp.0.join("game/Data");
        let parent = data.parent().unwrap().to_path_buf();
        let too_high = format!("Z:{}\\", parent.display()).replace('/', "\\");
        let f = bodyslide_findings(&facts(&tmp, vec![("GameDataPath", too_high, Some(parent))]));
        assert_eq!(
            (f.len(), f[0].code, f[0].arg("setting")),
            (1, "bodyslide_data_path", Some("GameDataPath"))
        );
        assert_eq!(
            f[0].arg("want"),
            Some(
                format!("Z:{}\\", data.display())
                    .replace('/', "\\")
                    .as_str()
            )
        );
        // Does not exist, compared lowercase.
        let other_case = PathBuf::from(data.display().to_string().to_uppercase());
        assert!(bodyslide_findings(&facts(
            &tmp,
            vec![("GameDataPath", "x".into(), Some(other_case))]
        ))
        .is_empty());
    }

    #[test]
    fn an_output_path_inside_the_instance_mods_is_accepted() {
        let tmp = Tmp::new();
        let mods_dir = tmp.0.join("instance/mods");
        assert!(bodyslide_findings(&facts(
            &tmp,
            vec![(
                "OutputDataPath",
                "x".into(),
                Some(mods_dir.join("BodySlide Output"))
            )]
        ))
        .is_empty());
        let f = bodyslide_findings(&facts(
            &tmp,
            vec![("OutputDataPath", "x".into(), Some("/tmp/elsewhere".into()))],
        ));
        assert_eq!(f[0].arg("setting"), Some("OutputDataPath"));
    }

    #[test]
    fn meshes_in_overwrite_root_are_reported() {
        let tmp = Tmp::new();
        let inst = Instance::portable(tmp.0.join("instance"));
        inst.create().unwrap();
        let meshes = inst.root_overwrite_dir().join("Meshes");
        fs::create_dir_all(&meshes).unwrap();
        fs::write(meshes.join("x.nif"), b"").unwrap();
        let install = tmp.0.join("install");
        let game = DetectedGame {
            source: Default::default(),
            def: eidos_games::GameDef::for_id("skyrimse").unwrap(),
            install_path: install.clone(),
            data_path: install.join("Data"),
            compatdata: None,
            steam_name: "synthetic".into(),
        };
        let f = bodyslide_findings(&bodyslide_facts(&game, &inst, &[]));
        let card = f.iter().find(|f| f.code == "overwrite_root_data").unwrap();
        assert_eq!(card.arg("dirs"), Some("Meshes"));
    }
}
