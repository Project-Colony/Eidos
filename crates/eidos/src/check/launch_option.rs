//! A18: Steam's launch option for the game starts this Eidos with `%command%`.
//! Read-only: Eidos never writes `localconfig.vdf`, it only says what to type.

use std::path::{Path, PathBuf};

use eidos_games::GameDef;

use super::{Finding, Level};

const EIDOS_NAMES: [&str; 3] = ["eidos", "eidos-gui", "eidos-linux"];

/// The line to paste into the game's Launch Options in Steam.
pub fn recommended_line(this_exe: &Path, game_id: &str) -> String {
    let raw = this_exe.display().to_string();
    let exe = if raw.contains(char::is_whitespace) || raw.contains('"') {
        format!("\"{}\"", raw.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        raw
    };
    if this_exe.file_name().is_some_and(|n| n == "eidos") {
        format!("{exe} play {game_id} -- %command%")
    } else {
        format!("{exe} %command%")
    }
}

/// Shell-style words: single quotes literal, double quotes with `\` escapes,
/// backslash outside quotes.
fn shell_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut it = s.chars();
    while let Some(c) = it.next() {
        match c {
            '\'' => {
                in_word = true;
                for c in it.by_ref() {
                    if c == '\'' {
                        break;
                    }
                    cur.push(c);
                }
            }
            '"' => {
                in_word = true;
                while let Some(c) = it.next() {
                    match c {
                        '"' => break,
                        '\\' => match it.next() {
                            Some(e @ ('"' | '\\' | '$' | '`')) => cur.push(e),
                            Some(e) => {
                                cur.push('\\');
                                cur.push(e);
                            }
                            None => break,
                        },
                        c => cur.push(c),
                    }
                }
            }
            '\\' => {
                in_word = true;
                if let Some(e) = it.next() {
                    cur.push(e);
                }
            }
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            c => {
                in_word = true;
                cur.push(c);
            }
        }
    }
    if in_word {
        words.push(cur);
    }
    words
}

fn resolve(token: &str, home: &Path, search_path: &[PathBuf]) -> Option<PathBuf> {
    let expanded = if let Some(rest) = token.strip_prefix("~/") {
        home.join(rest)
    } else if let Some(rest) = token.strip_prefix("$HOME/") {
        home.join(rest)
    } else {
        PathBuf::from(token)
    };
    if token.contains('/') {
        return expanded.is_file().then_some(expanded);
    }
    search_path
        .iter()
        .map(|d| d.join(token))
        .find(|p| p.is_file())
}

pub fn launch_option_findings(
    options: Option<&str>,
    this_exe: &Path,
    home: &Path,
    search_path: &[PathBuf],
    game: &GameDef,
) -> Vec<Finding> {
    let Some(options) = options else {
        return Vec::new();
    };
    let line = recommended_line(this_exe, game.id);
    let finding = |code, level, args: Vec<(&'static str, String)>| Finding { code, level, args };
    if options.trim().is_empty() {
        return vec![finding(
            "launch_option_missing",
            Level::Advice,
            vec![("game", game.name.to_string()), ("line", line)],
        )];
    }
    let words = shell_words(options);
    let Some(token) = words.iter().find(|w| {
        Path::new(w.as_str())
            .file_name()
            .is_some_and(|n| EIDOS_NAMES.iter().any(|e| n == *e))
    }) else {
        return vec![finding(
            "launch_option_no_eidos",
            Level::Advice,
            vec![
                ("game", game.name.to_string()),
                ("current", options.to_string()),
                ("line", line),
            ],
        )];
    };
    let mut out = Vec::new();
    let found = resolve(token, home, search_path);
    if found.is_none() {
        out.push(finding(
            "launch_option_binary_missing",
            Level::Problem,
            vec![("path", token.clone()), ("line", line.clone())],
        ));
    }
    if !words.iter().any(|w| w == "%command%") {
        out.push(finding(
            "launch_option_no_command",
            Level::Problem,
            vec![("current", options.to_string()), ("line", line.clone())],
        ));
    }
    if let Some(path) = found {
        let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let (a, b) = (canon(&path), canon(this_exe));
        // Same directory is the same install: the bundle's `eidos` beside `eidos-gui`.
        if a != b && a.parent() != b.parent() {
            out.push(finding(
                "launch_option_other_copy",
                Level::Advice,
                vec![
                    ("path", path.display().to_string()),
                    ("this", this_exe.display().to_string()),
                    ("line", line),
                ],
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Env {
        t: PathBuf,
        this: PathBuf,
    }
    fn env(tag: &str) -> Env {
        let t = std::env::temp_dir().join(format!("eidos-launchopt-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&t);
        fs::create_dir_all(t.join("bin")).unwrap();
        let this = t.join("bin/eidos-gui");
        fs::write(&this, "").unwrap();
        Env { t, this }
    }
    fn game() -> &'static GameDef {
        GameDef::for_id("skyrimse").unwrap()
    }
    fn touch(p: &Path) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "").unwrap();
    }

    #[test]
    fn an_unset_option_asks_for_one() {
        let e = env("unset");
        let f = launch_option_findings(Some(""), &e.this, &e.t, &[], game());
        assert_eq!(f[0].code, "launch_option_missing");
        assert_eq!(
            f[0].arg("line"),
            Some(format!("{} %command%", e.this.display()).as_str())
        );
    }

    #[test]
    fn the_cli_line_carries_the_verb() {
        assert_eq!(
            recommended_line(Path::new("/opt/bin/eidos"), "skyrimse"),
            "/opt/bin/eidos play skyrimse -- %command%"
        );
        assert_eq!(
            recommended_line(Path::new("/opt/My Games/eidos-gui"), "skyrimse"),
            "\"/opt/My Games/eidos-gui\" %command%"
        );
    }

    #[test]
    fn no_localconfig_says_nothing() {
        let e = env("none");
        assert!(launch_option_findings(None, &e.this, &e.t, &[], game()).is_empty());
    }

    #[test]
    fn an_option_without_eidos_is_reported() {
        let e = env("noeidos");
        let f = launch_option_findings(Some("%command% -skipintro"), &e.this, &e.t, &[], game());
        assert_eq!(f[0].code, "launch_option_no_eidos");
    }

    #[test]
    fn a_moved_binary_is_a_problem() {
        let e = env("moved");
        let f = launch_option_findings(
            Some("/gone/eidos-gui %command%"),
            &e.this,
            &e.t,
            &[],
            game(),
        );
        assert_eq!(
            (f[0].code, f[0].level),
            ("launch_option_binary_missing", Level::Problem)
        );
        assert_eq!(f[0].arg("path"), Some("/gone/eidos-gui"));
    }

    #[test]
    fn missing_command_is_a_problem() {
        let e = env("nocmd");
        let f = launch_option_findings(
            Some(&e.this.display().to_string()),
            &e.this,
            &e.t,
            &[],
            game(),
        );
        assert!(f
            .iter()
            .any(|f| f.code == "launch_option_no_command" && f.level == Level::Problem));
    }

    #[test]
    fn quoted_paths_with_spaces_and_env_prefixes_are_understood() {
        let mut e = env("quoted");
        e.this = e.t.join("My Games/eidos-gui");
        touch(&e.this);
        let opt = format!(r#"PROTON_ENABLE_NVAPI=1 "{}" %command%"#, e.this.display());
        assert!(launch_option_findings(Some(&opt), &e.this, &e.t, &[], game()).is_empty());
    }

    #[test]
    fn a_wrapper_before_eidos_is_fine() {
        let e = env("wrapper");
        let opt = format!("gamemoderun {} %command%", e.this.display());
        assert!(launch_option_findings(Some(&opt), &e.this, &e.t, &[], game()).is_empty());
    }

    #[test]
    fn tilde_expands_to_home() {
        let e = env("tilde");
        touch(&e.t.join("bin/eidos"));
        let opt = "~/bin/eidos play skyrimse -- %command%";
        assert!(launch_option_findings(Some(opt), &e.this, &e.t, &[], game()).is_empty());
    }

    #[test]
    fn a_bare_name_is_looked_up_on_the_search_path() {
        let e = env("bare");
        touch(&e.t.join("bin/eidos"));
        let opt = "eidos play skyrimse -- %command%";
        assert!(
            launch_option_findings(Some(opt), &e.this, &e.t, &[e.t.join("bin")], game()).is_empty()
        );
        assert_eq!(
            launch_option_findings(Some(opt), &e.this, &e.t, &[], game())[0].code,
            "launch_option_binary_missing"
        );
    }

    #[test]
    fn another_copy_is_advice() {
        let e = env("other");
        touch(&e.t.join("old/eidos-gui"));
        let opt = format!("{}/old/eidos-gui %command%", e.t.display());
        let f = launch_option_findings(Some(&opt), &e.this, &e.t, &[], game());
        assert_eq!(
            (f[0].code, f[0].level),
            ("launch_option_other_copy", Level::Advice)
        );
    }
}
