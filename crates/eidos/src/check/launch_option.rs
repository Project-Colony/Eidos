//! A18: Steam's launch option for the game starts this Eidos with `%command%`.
//! A28: Run hands the game to the store that owns the copy once that store's
//! setting starts Eidos.
//! Read-only: Eidos never writes a store's config, it only says what to type.

use std::path::{Path, PathBuf};

use eidos_games::{DetectedGame, GameDef, GameSource, Store};

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
    match wrapper_args(this_exe, game_id) {
        args if args.is_empty() => format!("{exe} %command%"),
        args => format!("{exe} {args} %command%"),
    }
}

/// Eidos's arguments as a store's wrapper, before the game command.
pub fn wrapper_args(this_exe: &Path, game_id: &str) -> String {
    if this_exe.file_name().is_some_and(|n| n == "eidos") {
        format!("play {game_id} --")
    } else {
        String::new()
    }
}

/// What Run does with the game in a window no store opened (A28).
#[derive(Debug, PartialEq, Eq)]
pub enum Handoff {
    /// Spawn `argv`: `store` starts the game through its Eidos setting.
    Start {
        store: &'static str,
        argv: Vec<String>,
    },
    /// The launch option does not start Eidos with `%command%`, or the copy was never started from Steam.
    SetSteamOption { line: String },
    /// Some Heroic settings file for the game has no wrapper that starts Eidos, or none exists.
    SetHeroicWrapper { exe: String, args: String },
    /// legendary's `config.ini` has no wrapper that starts Eidos for the game.
    SetLegendaryWrapper {
        file: PathBuf,
        app: String,
        wrapper: String,
    },
    /// The gamedef's script-extender loader is not next to this copy's executable.
    NoLoader { loader: &'static str },
    /// A source Eidos cannot hand the game to.
    ThroughStore,
}

/// Decide how Run starts `game` through its store. Spawns nothing: reads only
/// files under `home` and stats the binaries a setting names.
pub fn launch_handoff(
    game: &DetectedGame,
    loader_found: bool,
    this_exe: &Path,
    home: &Path,
    search_path: &[PathBuf],
) -> Handoff {
    enum Via<'a> {
        Steam,
        Heroic(&'static str, &'a str),
        Legendary(&'a str),
    }
    let def = game.def;
    let steam_hint = || Handoff::SetSteamOption {
        line: recommended_line(this_exe, def.id),
    };
    // No catch-all arm: a new source or store does not compile until it gets one.
    let via = match &game.source {
        GameSource::Steam if game.compatdata.as_ref().is_some_and(|c| c.is_dir()) => Via::Steam,
        // Never started from Steam (or its prefix was rebuilt): nothing to hand off yet.
        GameSource::Steam => return steam_hint(),
        GameSource::External { app_id, .. } if !eidos_games::plain_app_id(app_id) => {
            return Handoff::ThroughStore
        }
        GameSource::External {
            store: Store::Gog,
            heroic: true,
            app_id,
            ..
        } => Via::Heroic("gog", app_id),
        GameSource::External {
            store: Store::Epic,
            heroic: true,
            app_id,
            ..
        } => Via::Heroic("legendary", app_id),
        GameSource::External {
            store: Store::Epic,
            heroic: false,
            app_id,
            ..
        } => Via::Legendary(app_id),
        GameSource::External {
            store: Store::Gog,
            heroic: false,
            ..
        } => return Handoff::ThroughStore,
    };
    if let (false, Some(se)) = (loader_found, &def.script_extender) {
        return Handoff::NoLoader { loader: se.loader };
    }
    let starts_eidos = |words: &[String]| {
        // Neither Heroic nor legendary runs the wrapper through a shell, so `~` stays literal.
        words.iter().any(|w| {
            is_eidos_word(w)
                && !w.starts_with('~')
                && !w.starts_with("$HOME/")
                && resolve(w, home, search_path).is_some()
        })
    };
    match via {
        Via::Steam => {
            let starts =
                eidos_games::steam_launch_options(home, def.steam_app_id).is_some_and(|o| {
                    launch_option_findings(Some(&o), this_exe, home, search_path, def)
                        .iter()
                        .all(|f| f.code == "launch_option_other_copy")
                });
            if !starts {
                return steam_hint();
            }
            Handoff::Start {
                store: "Steam",
                argv: vec![
                    "steam".into(),
                    "-applaunch".into(),
                    def.steam_app_id.to_string(),
                ],
            }
        }
        Via::Heroic(runner, app) => {
            let lists = eidos_games::heroic_wrappers(home, app);
            // Heroic chains its wrappers, so Eidos may come after another one.
            let words = |list: &[(String, String)]| {
                list.iter()
                    .flat_map(|(exe, args)| std::iter::once(exe.clone()).chain(shell_words(args)))
                    .collect::<Vec<_>>()
            };
            // With both Heroics holding the game, either may receive the URI.
            if lists.is_empty() || !lists.iter().all(|l| starts_eidos(&words(l))) {
                return Handoff::SetHeroicWrapper {
                    exe: this_exe.display().to_string(),
                    args: wrapper_args(this_exe, def.id),
                };
            }
            Handoff::Start {
                store: "Heroic",
                argv: vec![
                    "xdg-open".into(),
                    format!("heroic://launch?appName={app}&runner={runner}"),
                ],
            }
        }
        Via::Legendary(app) => {
            let (file, wrapper) = eidos_games::legendary_wrapper(home, app);
            if !wrapper.is_some_and(|w| starts_eidos(&shell_words(&w))) {
                // legendary splits the wrapper with `shlex`, so the line's quoting holds.
                let line = recommended_line(this_exe, def.id);
                let wrapper = line.strip_suffix(" %command%").unwrap_or(&line).to_string();
                return Handoff::SetLegendaryWrapper {
                    file,
                    app: app.to_string(),
                    wrapper,
                };
            }
            Handoff::Start {
                store: "Legendary",
                argv: vec!["legendary".into(), "launch".into(), app.to_string()],
            }
        }
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

/// A word naming an Eidos binary: its file name is one of [`EIDOS_NAMES`] and
/// it is not a `NAME=value` assignment.
fn is_eidos_word(word: &str) -> bool {
    !is_env_assignment(word)
        && Path::new(word)
            .file_name()
            .is_some_and(|n| EIDOS_NAMES.iter().any(|e| n == *e))
}

/// `NAME=value`, as the shell reads it before a command: `EIDOS_INSTANCE=/x/eidos`
/// sets a variable and is not a path to an Eidos binary.
fn is_env_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
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
    let Some(at) = words.iter().position(|w| is_eidos_word(w)) else {
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
    let token = &words[at];
    let mut out = Vec::new();
    let found = resolve(token, home, search_path);
    if found.is_none() {
        out.push(finding(
            "launch_option_binary_missing",
            Level::Problem,
            vec![("path", token.clone()), ("line", line.clone())],
        ));
    }
    // Before Eidos, `%command%` runs the game and Eidos is only one of its arguments.
    if !words[at + 1..].iter().any(|w| w == "%command%") {
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

    use eidos_games::{DetectedGame, GameSource, Store};

    fn copy(e: &Env, source: GameSource) -> DetectedGame {
        let compat = e.t.join("compat");
        fs::create_dir_all(&compat).unwrap();
        DetectedGame {
            def: game(),
            source,
            install_path: e.t.join("game"),
            data_path: e.t.join("game/Data"),
            compatdata: Some(compat),
            steam_name: String::new(),
        }
    }
    fn external(store: Store, app: &str, heroic: bool) -> GameSource {
        GameSource::External {
            store,
            app_id: app.into(),
            prefix: None,
            heroic,
            runner: None,
        }
    }
    /// Task 22's layout: `config/config.vdf` marks the Steam root, the account's
    /// `localconfig.vdf` holds the option.
    fn steam_option(e: &Env, option: &str) {
        let steam = e.t.join(".local/share/Steam");
        touch(&steam.join("config/config.vdf"));
        let p = steam.join("userdata/1/config/localconfig.vdf");
        touch(&p);
        let option = option.replace('\\', "\\\\").replace('"', "\\\"");
        fs::write(
            p,
            format!("\"apps\" {{ \"489830\" {{ \"LaunchOptions\" \"{option}\" }} }}"),
        )
        .unwrap();
    }
    fn heroic_config(root: &Path, app: &str, wrappers: &str) {
        let p = root.join("GamesConfig").join(format!("{app}.json"));
        touch(&p);
        fs::write(
            p,
            format!(r#"{{"{app}": {{"wrapperOptions": {wrappers}}}, "version": "v0"}}"#),
        )
        .unwrap();
    }

    #[test]
    fn steam_handoff_needs_eidos_and_command_in_the_launch_option() {
        // One home per case: `steam_launch_options` memoizes on path, size and mtime.
        for (i, (option, hands_off)) in [
            (None, false),
            (Some(""), false),
            (Some("%command% -skipintro"), false),
            (Some("THIS"), false),
            (Some("/gone/eidos-gui %command%"), false),
            (Some("%command% THIS"), false),
            (Some("THIS %command%"), true),
        ]
        .into_iter()
        .enumerate()
        {
            let e = env(&format!("handoff-steam-{i}"));
            if let Some(o) = option {
                steam_option(&e, &o.replace("THIS", &e.this.display().to_string()));
            }
            let want = if hands_off {
                Handoff::Start {
                    store: "Steam",
                    argv: vec!["steam".into(), "-applaunch".into(), "489830".into()],
                }
            } else {
                Handoff::SetSteamOption {
                    line: recommended_line(&e.this, "skyrimse"),
                }
            };
            let got = launch_handoff(&copy(&e, GameSource::Steam), true, &e.this, &e.t, &[]);
            assert_eq!(got, want, "{option:?}");
            let _ = fs::remove_dir_all(&e.t);
        }
    }

    #[test]
    fn steam_handoff_needs_a_prefix() {
        let e = env("handoff-prefix");
        steam_option(&e, &format!("{} %command%", e.this.display()));
        let hint = || Handoff::SetSteamOption {
            line: recommended_line(&e.this, "skyrimse"),
        };
        let mut g = copy(&e, GameSource::Steam);
        g.compatdata = None;
        assert_eq!(launch_handoff(&g, true, &e.this, &e.t, &[]), hint());
        g.compatdata = Some(e.t.join("gone"));
        assert_eq!(launch_handoff(&g, true, &e.this, &e.t, &[]), hint());
        let _ = fs::remove_dir_all(&e.t);
    }

    #[test]
    fn heroic_handoff_reads_the_game_wrapper() {
        let e = env("handoff-heroic");
        let native = e.t.join(".config/heroic");
        let flatpak =
            e.t.join(".var/app/com.heroicgameslauncher.hgl/config/heroic");
        let eidos = format!(
            r#"[{{"exe": "/usr/bin/gamemoderun", "args": ""}}, {{"exe": "{}", "args": ""}}]"#,
            e.this.display()
        );
        let gog = copy(&e, external(Store::Gog, "gogapp", true));
        let epic = copy(&e, external(Store::Epic, "epicapp", true));
        let run = |g: &DetectedGame| launch_handoff(g, true, &e.this, &e.t, &[]);
        let uri = |app: &str, runner: &str| Handoff::Start {
            store: "Heroic",
            argv: vec![
                "xdg-open".into(),
                format!("heroic://launch?appName={app}&runner={runner}"),
            ],
        };
        let hint = || Handoff::SetHeroicWrapper {
            exe: e.this.display().to_string(),
            args: String::new(),
        };
        assert_eq!(run(&gog), hint(), "no settings file");
        heroic_config(
            &native,
            "gogapp",
            r#"[{"exe": "/usr/bin/gamemoderun", "args": ""}]"#,
        );
        assert_eq!(run(&gog), hint(), "a wrapper that is not Eidos");
        heroic_config(
            &native,
            "gogapp",
            r#"[{"exe": "/gone/eidos-gui", "args": ""}]"#,
        );
        assert_eq!(run(&gog), hint(), "an Eidos that does not exist");
        heroic_config(
            &native,
            "gogapp",
            r#"[{"exe": "~/bin/eidos-gui", "args": ""}]"#,
        );
        assert_eq!(run(&gog), hint(), "Heroic does not expand ~");
        heroic_config(&native, "gogapp", &eidos);
        assert_eq!(run(&gog), uri("gogapp", "gog"));
        heroic_config(&flatpak, "epicapp", &eidos);
        assert_eq!(run(&epic), uri("epicapp", "legendary"), "Flatpak Heroic");
        heroic_config(&flatpak, "gogapp", "[]");
        assert_eq!(
            run(&gog),
            hint(),
            "the other Heroic starts the game without Eidos"
        );
        let _ = fs::remove_dir_all(&e.t);
    }

    #[test]
    fn legendary_handoff_reads_the_game_wrapper() {
        let e = env("handoff-legendary");
        let ini = e.t.join(".config/legendary/config.ini");
        let g = copy(&e, external(Store::Epic, "epicapp", false));
        let run = || launch_handoff(&g, true, &e.this, &e.t, &[]);
        let start = || Handoff::Start {
            store: "Legendary",
            argv: vec!["legendary".into(), "launch".into(), "epicapp".into()],
        };
        let hint = || Handoff::SetLegendaryWrapper {
            file: ini.clone(),
            app: "epicapp".into(),
            wrapper: e.this.display().to_string(),
        };
        assert_eq!(run(), hint(), "no config.ini");
        let this = e.this.display();
        for (text, hands_off) in [
            (format!("[other]\nwrapper = {this}\n"), false),
            ("[epicapp]\nwrapper = gamemoderun\n".to_string(), false),
            ("[epicapp]\nwrapper = ~/bin/eidos-gui\n".to_string(), false),
            (
                format!("[epicapp]\n# note\nWrapper: gamemoderun \"{this}\"\n"),
                true,
            ),
            (format!("[default]\nwrapper = {this}\n"), true),
            (
                format!("[default]\nwrapper = {this}\n[epicapp]\nwrapper = gamemoderun\n"),
                false,
            ),
        ] {
            touch(&ini);
            fs::write(&ini, &text).unwrap();
            assert_eq!(run(), if hands_off { start() } else { hint() }, "{text}");
        }
        let _ = fs::remove_dir_all(&e.t);
    }

    #[test]
    fn no_handoff_without_the_loader_or_for_a_source_without_one() {
        let e = env("handoff-gates");
        steam_option(&e, &format!("{} %command%", e.this.display()));
        let run = |g: &DetectedGame, loader: bool| launch_handoff(g, loader, &e.this, &e.t, &[]);
        let no_loader = || Handoff::NoLoader {
            loader: "skse64_loader.exe",
        };
        assert_eq!(run(&copy(&e, GameSource::Steam), false), no_loader());
        assert_eq!(
            run(&copy(&e, external(Store::Epic, "epicapp", false)), false),
            no_loader()
        );
        assert_eq!(
            run(&copy(&e, external(Store::Gog, "gogapp", false)), true),
            Handoff::ThroughStore,
            "GOG outside Heroic"
        );
        assert_eq!(
            run(&copy(&e, external(Store::Gog, "../gogapp", true)), true),
            Handoff::ThroughStore,
            "an app id that is not plain"
        );
        let _ = fs::remove_dir_all(&e.t);
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
        assert_eq!(
            wrapper_args(Path::new("/opt/bin/eidos"), "skyrimse"),
            "play skyrimse --"
        );
        assert_eq!(
            wrapper_args(Path::new("/opt/bin/eidos-gui"), "skyrimse"),
            ""
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
        let after = format!("%command% {}", e.this.display());
        let f = launch_option_findings(Some(&after), &e.this, &e.t, &[], game());
        assert!(f.iter().any(|f| f.code == "launch_option_no_command"));
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
    fn an_instance_variable_ending_in_eidos_is_not_the_binary() {
        let e = env("envassign");
        let opt = format!(
            "EIDOS_INSTANCE={}/eidos {} %command%",
            e.t.display(),
            e.this.display()
        );
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
