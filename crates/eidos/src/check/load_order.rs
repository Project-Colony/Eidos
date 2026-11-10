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
}
