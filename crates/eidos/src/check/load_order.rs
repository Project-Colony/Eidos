//! A set `sTestFile` line makes the game load those plugins and ignore
//! plugins.txt, so the load order Eidos writes has no effect.

use std::io;
use std::path::Path;

use eidos_ini::{delete_key, get_key};
use eidos_instance::{read_text_lossy, write_text, Profile};

use super::{Finding, Level};

pub const TEST_FILE_INIS: [&str; 2] = ["Fallout4Custom.ini", "StarfieldCustom.ini"];

/// The first of `def.ini_files` named in TEST_FILE_INIS (case-insensitive):
/// Fallout 4, Fallout 4 VR, Starfield. None for every other game.
pub fn test_file_ini(def: &eidos_games::GameDef) -> Option<&'static str> {
    TEST_FILE_INIS
        .into_iter()
        .find(|c| def.ini_files.iter().any(|f| f.eq_ignore_ascii_case(c)))
}

/// `text` without any `[General] sTestFile1..sTestFile10` line (every copy, every
/// `[General]` section), and the removed lines whose value is not empty, as
/// `sTestFile<N>=<trimmed value>`, in key order then file order.
pub fn strip_test_files(text: &str) -> (String, Vec<String>) {
    let mut t = text.to_string();
    let mut removed = Vec::new();
    for n in 1..=10 {
        let key = format!("sTestFile{n}");
        while let Some(v) = get_key(&t, "General", &key) {
            let value = v.trim().to_string();
            let next = delete_key(&t, "General", &key);
            // delete_key may not see a line get_key does; stop rather than loop.
            if next == t {
                break;
            }
            if !value.is_empty() {
                removed.push(format!("{key}={value}"));
            }
            t = next;
        }
    }
    (t, removed)
}

/// `stestfile_present` when the profile copy has a non-empty `sTestFile` line.
pub fn test_file_findings(def: &eidos_games::GameDef, profile: &Profile) -> Vec<Finding> {
    let Some(name) = test_file_ini(def) else {
        return Vec::new();
    };
    let ini = profile.ini_path(name);
    let Some((text, _)) = read_text_lossy(&ini) else {
        return Vec::new();
    };
    let (_, removed) = strip_test_files(&text);
    if removed.is_empty() {
        return Vec::new();
    }
    vec![Finding {
        code: "stestfile_present",
        level: Level::Problem,
        args: vec![
            ("ini", ini.display().to_string()),
            ("lines", removed.join(", ")),
        ],
    }]
}

/// Copies `ini` to `<ini>.pre-fix`, then writes the stripped text back in the
/// encoding it was found in.
pub fn fix_test_files(ini: &Path) -> io::Result<()> {
    let (text, cp1252) = read_text_lossy(ini)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, ini.display().to_string()))?;
    let mut backup = ini.as_os_str().to_owned();
    backup.push(".pre-fix");
    std::fs::copy(ini, backup)?;
    write_text(ini, &strip_test_files(&text).0, cp1252)
}

pub const BLUEPRINT_PREFIX: &str = "blueprintships-";

fn ends_with_ci(name: &str, ext: &str) -> bool {
    name.len() >= ext.len()
        && name.is_char_boundary(name.len() - ext.len())
        && name[name.len() - ext.len()..].eq_ignore_ascii_case(ext)
}

fn has_blueprint_prefix(name: &str) -> bool {
    name.get(..BLUEPRINT_PREFIX.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(BLUEPRINT_PREFIX))
}

/// Starfield only (the caller checks the game). Enabled plugins with a readable header.
pub fn starfield_findings(list: &eidos_plugins::PluginList) -> Vec<Finding> {
    let on: Vec<&eidos_plugins::Plugin> = list
        .plugins
        .iter()
        .filter(|p| p.enabled && p.header_error.is_none())
        .collect();
    let mut out = Vec::new();
    let esps: Vec<&str> = on
        .iter()
        .filter(|p| ends_with_ci(&p.name, ".esp") && !p.is_master)
        .map(|p| p.name.as_str())
        .collect();
    if !esps.is_empty() {
        out.push(Finding {
            code: "starfield_esp_active",
            level: Level::Advice,
            args: vec![
                ("n", esps.len().to_string()),
                ("plugins", esps[..esps.len().min(8)].join(", ")),
            ],
        });
    }
    let invalid: Vec<&str> = on
        .iter()
        .filter(|p| {
            let prefix = has_blueprint_prefix(&p.name);
            (p.is_blueprint && !(prefix && ends_with_ci(&p.name, ".esm")))
                || (!p.is_blueprint && prefix)
        })
        .map(|p| p.name.as_str())
        .collect();
    if !invalid.is_empty() {
        out.push(Finding {
            code: "blueprint_invalid",
            level: Level::Problem,
            args: vec![("plugins", invalid.join(", "))],
        });
    }
    let unpaired: Vec<String> = on
        .iter()
        .filter(|p| {
            p.is_blueprint && has_blueprint_prefix(&p.name) && ends_with_ci(&p.name, ".esm")
        })
        .filter_map(|p| {
            let base = &p.name[BLUEPRINT_PREFIX.len()..p.name.len() - 4];
            let found = list.plugins.iter().filter(|q| q.enabled).any(|q| {
                [".esm", ".esp", ".esl"].iter().any(|e| {
                    q.name.len() == base.len() + 4
                        && q.name.is_char_boundary(base.len())
                        && q.name[..base.len()].eq_ignore_ascii_case(base)
                        && q.name[base.len()..].eq_ignore_ascii_case(e)
                })
            });
            (!found).then(|| format!("{} ({base})", p.name))
        })
        .collect();
    if !unpaired.is_empty() {
        out.push(Finding {
            code: "blueprint_unpaired",
            level: Level::Advice,
            args: vec![("plugins", unpaired.join(", "))],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eidos_games::GameDef;
    use eidos_instance::Instance;
    use std::fs;

    fn inst(tag: &str, ini: &str, body: &str) -> (Instance, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("eidos-tf-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let p = inst.active().ini_path(ini);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, body).unwrap();
        (inst, root)
    }

    #[test]
    fn every_test_file_line_is_removed_and_reported_once() {
        let text = "[General]\r\nsTestFile1=Foo.esp\r\nsStartingConsoleCommand=x\r\nsTestFile1=Dup.esp\r\n\
                    [Display]\r\nsTestFile2=NotGeneral.esp\r\n[General]\r\nsTestFile10=Bar.esm\r\nsTestFile2=\r\n";
        let (out, removed) = strip_test_files(text);
        assert_eq!(
            removed,
            [
                "sTestFile1=Foo.esp",
                "sTestFile1=Dup.esp",
                "sTestFile10=Bar.esm"
            ]
        );
        assert_eq!(
            out,
            "[General]\r\nsStartingConsoleCommand=x\r\n[Display]\r\nsTestFile2=NotGeneral.esp\r\n[General]\r\n"
        );
    }

    #[test]
    fn an_empty_test_file_line_alone_is_not_reported() {
        let (inst, root) = inst("empty", "Fallout4Custom.ini", "[General]\nsTestFile1=\n");
        assert!(
            test_file_findings(GameDef::for_id("fallout4").unwrap(), &inst.active()).is_empty()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_test_file_line_is_a_problem_for_the_custom_ini_games() {
        let (inst, root) = inst(
            "problem",
            "Fallout4Custom.ini",
            "[General]\nsTestFile1=MyTest.esp\n",
        );
        let f = test_file_findings(GameDef::for_id("fallout4").unwrap(), &inst.active());
        assert_eq!(
            (f.len(), f[0].code, f[0].level),
            (1, "stestfile_present", Level::Problem)
        );
        assert_eq!(f[0].arg("lines"), Some("sTestFile1=MyTest.esp"));
        assert!(f[0].arg("ini").unwrap().ends_with("Fallout4Custom.ini"));
        assert_eq!(
            test_file_ini(GameDef::for_id("starfield").unwrap()),
            Some("StarfieldCustom.ini")
        );
        assert_eq!(
            test_file_ini(GameDef::for_id("fallout4vr").unwrap()),
            Some("Fallout4Custom.ini")
        );
        assert_eq!(test_file_ini(GameDef::for_id("skyrimse").unwrap()), None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn the_fix_keeps_the_old_file_and_the_encoding() {
        let original = b"[General]\r\nsTestFile1=A.esp\r\nsNote=Caf\xe9\r\n".to_vec();
        let (inst, root) = inst("fix", "Fallout4Custom.ini", "");
        let ini = inst.active().ini_path("Fallout4Custom.ini");
        fs::write(&ini, &original).unwrap();
        fix_test_files(&ini).unwrap();
        assert_eq!(fs::read(&ini).unwrap(), b"[General]\r\nsNote=Caf\xe9\r\n");
        assert_eq!(
            fs::read(ini.with_file_name("Fallout4Custom.ini.pre-fix")).unwrap(),
            original
        );
        let _ = fs::remove_dir_all(root);
    }

    fn sf(name: &str, blueprint: bool, master: bool, enabled: bool) -> eidos_plugins::Plugin {
        eidos_plugins::Plugin {
            name: name.to_string(),
            origin_mod: String::new(),
            path: std::path::PathBuf::new(),
            enabled,
            force_disabled: false,
            is_master: master,
            is_light: false,
            is_medium: false,
            is_dummy: false,
            header_error: None,
            form_version: None,
            author: None,
            description: None,
            header_version: None,
            is_blueprint: blueprint,
            is_update: false,
            masters: Vec::new(),
            priority: 0,
            index: None,
            ini: None,
        }
    }
    use eidos_plugins::PluginList;

    #[test]
    fn starfield_reports_active_esps_but_not_master_flagged_ones() {
        let list = PluginList {
            plugins: vec![
                sf("Starfield.esm", false, true, true),
                sf("Mod.esp", false, false, true),
                sf("Patch.esp", false, true, true),
                sf("Off.esp", false, false, false),
            ],
            ..Default::default()
        };
        let f = starfield_findings(&list);
        assert_eq!(
            f.iter().map(|f| f.code).collect::<Vec<_>>(),
            ["starfield_esp_active"]
        );
        assert_eq!(
            (f[0].arg("n"), f[0].arg("plugins"), f[0].level),
            (Some("1"), Some("Mod.esp"), Level::Advice)
        );
    }
    #[test]
    fn a_blueprint_needs_the_flag_the_prefix_and_esm() {
        let list = PluginList {
            plugins: vec![
                sf("A.esm", false, true, true),
                sf("BlueprintShips-A.esm", true, true, true),
                sf("BlueprintShips-B.esp", true, false, true),
                sf("Ships-C.esm", true, true, true),
                sf("BlueprintShips-D.esm", false, true, true),
                sf("BlueprintShips-E.esp", false, false, true),
            ],
            ..Default::default()
        };
        let bad = starfield_findings(&list)
            .into_iter()
            .find(|f| f.code == "blueprint_invalid")
            .unwrap();
        assert_eq!(bad.level, Level::Problem);
        assert_eq!(
            bad.arg("plugins"),
            Some("BlueprintShips-B.esp, Ships-C.esm, BlueprintShips-D.esm, BlueprintShips-E.esp")
        );
    }
    #[test]
    fn blueprint_pairing_compares_names_not_patterns() {
        let list = PluginList {
            plugins: vec![
                sf("BlueprintShips-Foo (v1.2).esm", true, true, true),
                sf("foo (V1.2).esm", false, true, true),
                sf("BlueprintShips-Bar.esm", true, true, true),
                sf("BarX.esm", false, true, true),
                sf("BlueprintShips-Baz.esm", true, true, true),
                sf("Baz.esp", false, false, false),
            ],
            ..Default::default()
        };
        let f = starfield_findings(&list);
        let unpaired = f.iter().find(|f| f.code == "blueprint_unpaired").unwrap();
        assert_eq!(
            unpaired.arg("plugins"),
            Some("BlueprintShips-Bar.esm (Bar), BlueprintShips-Baz.esm (Baz)")
        );
    }
    #[test]
    fn an_unreadable_header_is_not_called_an_invalid_blueprint() {
        let mut p = sf("BlueprintShips-X.esm", false, false, true);
        p.header_error = Some("truncated".into());
        assert!(starfield_findings(&PluginList {
            plugins: vec![p],
            ..Default::default()
        })
        .is_empty());
    }
}
