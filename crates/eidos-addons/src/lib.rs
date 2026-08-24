//! User extensions: declarative add-ons discovered from TOML, run out of process.
//!
//! # Why not MO2's plugin system
//!
//! MO2 loads plugins as Qt shared libraries (`QPluginLoader` + `qobject_cast`),
//! which works because Qt carries its own RTTI across a DLL boundary. Rust has no
//! stable ABI: a `cdylib` compiled against a different rustc, a different
//! optimisation flag or a different feature set of a shared dependency is
//! undefined behaviour, not a version mismatch. And an `iced::Element` is a
//! monomorphised generic carrying a lifetime, so a shared library could not
//! construct one to hand back even if the ABI were stable.
//!
//! MO2's other half is worse: its bundled plugins are Python against PyQt, so
//! hosting them would mean an embedded interpreter plus a Qt widget bridge into an
//! iced application.
//!
//! So there are exactly two safe shapes, and this crate is the second: an add-on
//! is a MANIFEST plus, at most, a program Eidos runs and reads the output of.
//! Nothing is loaded into the process. (The first shape - statically linked, in
//! tree - is what the game definitions, the installers and the conflict engine
//! already are.)
//!
//! # Why "add-on" and never "plugin"
//!
//! `plugin` already means an `.esp`/`.esm` everywhere else in this codebase: the
//! `eidos-plugins` crate, the Plugins tab, `plugins.txt`. A second meaning for the
//! same word inside the same window would make every sentence about either one
//! ambiguous.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// What an add-on contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonKind {
    /// An entry in the Tools menu that runs a program.
    Tool,
    /// A program run on refresh whose output becomes health-check rows.
    Diagnose,
}

/// One user add-on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addon {
    /// Stable identifier, also the settings key. Lowercase, no spaces.
    pub id: String,
    /// Display name.
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub kind: AddonKind,
    /// The program to run. Resolved against `PATH` when it has no separator.
    pub exec: PathBuf,
    /// Arguments, with `{placeholders}` expanded from the instance (see
    /// [`Context::expand`]).
    pub args: Vec<String>,
    /// Working directory, expanded the same way. Empty = the instance root.
    pub workdir: String,
    /// Game ids this applies to. Empty = every game.
    pub games: Vec<String>,
    /// Where the manifest was read from, for the Extensions list.
    pub source: PathBuf,
}

impl Addon {
    /// Whether this add-on applies to `game_id`.
    pub fn applies_to(&self, game_id: &str) -> bool {
        self.games.is_empty() || self.games.iter().any(|g| g.eq_ignore_ascii_case(game_id))
    }

    /// Why this add-on cannot run right now, or `None` if it can.
    ///
    /// Only the executable is checked, because that is the only requirement that
    /// can be checked without running anything. MO2 grew a whole requirement
    /// language (`pluginrequirements.h`) including plugin-to-plugin dependencies;
    /// that exists to sequence a load graph, which out-of-process add-ons do not
    /// have.
    pub fn unavailable(&self) -> Option<String> {
        if self.exec.as_os_str().is_empty() {
            return Some("no program to run".to_string());
        }
        if has_separator(&self.exec) {
            return (!self.exec.is_file())
                .then(|| format!("{} is not there", self.exec.display()));
        }
        which(&self.exec).is_none().then(|| format!("{} is not on PATH", self.exec.display()))
    }
}

/// Everything an add-on's placeholders can be filled from.
///
/// A read-only snapshot, deliberately not a handle to the running application.
/// MO2's `IOrganizer` is sixty virtuals over one object because MO2 has one
/// `OrganizerCore`; an add-on here gets values, cannot call back, and so cannot
/// leave the window in a state the window did not choose.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub values: BTreeMap<String, String>,
}

impl Context {
    /// Substitute `{key}` for every known key. An unknown placeholder is left
    /// exactly as written rather than blanked: a silently empty argument turns
    /// `--out {missing}` into `--out --next-flag`, and the program then does
    /// something nobody asked for.
    pub fn expand(&self, raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut rest = raw;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            // An unclosed brace is not a placeholder. The remainder is copied
            // here and the loop RETURNS: falling out of it would append `rest`
            // again, which still carries the prefix already pushed above.
            let Some(close) = rest[open..].find('}').map(|i| open + i) else {
                out.push_str(&rest[open..]);
                return out;
            };
            let key = &rest[open + 1..close];
            match self.values.get(key) {
                Some(v) => out.push_str(v),
                None => out.push_str(&rest[open..=close]),
            }
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        out
    }

    /// Every placeholder in `raw` that this context cannot fill.
    pub fn missing(&self, raw: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = raw;
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}').map(|i| open + i) else { break };
            let key = &rest[open + 1..close];
            if !self.values.contains_key(key) && !key.is_empty() {
                out.push(key.to_string());
            }
            rest = &rest[close + 1..];
        }
        out
    }
}

/// The directory user add-on manifests are read from, mirroring where user game
/// definitions live so there is one place to look for "things I added".
pub fn user_addons_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".config")
        });
    base.join("eidos").join("addons")
}

/// Parse every `*.toml` in `dir`, newest name order, skipping invalid ones.
///
/// Re-read on demand rather than cached in a `OnceLock` like the game registry:
/// a game definition is consulted before the window exists and never changes
/// under it, while an add-on is something the user is actively writing, and
/// making them restart to see a typo fixed is the wrong trade.
pub fn load_addons_from(dir: &Path) -> Vec<Addon> {
    scan_addons_from(dir).0
}

/// [`load_addons_from`] plus the manifests it REFUSED, each with the reason.
///
/// The refusals matter as much as the acceptances: an add-on that fails to parse
/// simply does not appear, and a list that then says "no extensions yet" is
/// telling the user their file is not there when it is - sitting one typo away
/// from working. A message on stderr does not reach a window started from a
/// desktop launcher.
pub fn scan_addons_from(dir: &Path) -> (Vec<Addon>, Vec<(PathBuf, String)>) {
    let Ok(rd) = fs::read_dir(dir) else { return (Vec::new(), Vec::new()) };
    let mut paths: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("toml")))
        .collect();
    paths.sort();
    let mut out: Vec<Addon> = Vec::new();
    let mut bad: Vec<(PathBuf, String)> = Vec::new();
    for p in paths {
        let text = match fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) => {
                bad.push((p, format!("could not be read: {e}")));
                continue;
            }
        };
        match parse_addon(&text, &p) {
            // Later files do not silently shadow earlier ones: two manifests
            // claiming one id is a mistake worth seeing, and the alternative is
            // an add-on that vanishes for no visible reason.
            Some(a) if out.iter().any(|b| b.id.eq_ignore_ascii_case(&a.id)) => {
                bad.push((p, format!("the id '{}' is already taken by another manifest", a.id)));
            }
            Some(a) => out.push(a),
            None => bad.push((p, why_rejected(&text))),
        }
    }
    (out, bad)
}

/// A specific reason a manifest was refused, for the Extensions list.
fn why_rejected(text: &str) -> String {
    let Ok(raw) = toml::from_str::<RawAddon>(text) else {
        return "not valid TOML, or a required field is missing (id, kind, exec)".to_string();
    };
    let id = raw.id.trim();
    if id.is_empty() {
        return "`id` is empty".to_string();
    }
    if id.contains(char::is_whitespace) {
        return "`id` contains a space - it is used as a key, so it cannot".to_string();
    }
    if raw.exec.trim().is_empty() {
        return "`exec` is empty - there is no program to run".to_string();
    }
    format!("`kind` is '{}'; it must be 'tool' or 'diagnose'", raw.kind.trim())
}

/// Every user add-on.
pub fn load_addons() -> Vec<Addon> {
    load_addons_from(&user_addons_dir())
}

/// The manifests in the user's add-on directory that were refused, and why.
pub fn rejected_manifests() -> Vec<(PathBuf, String)> {
    scan_addons_from(&user_addons_dir()).1
}

/// Parse one manifest.
pub fn parse_addon(text: &str, source: &Path) -> Option<Addon> {
    let raw: RawAddon = toml::from_str(text).ok()?;
    let kind = match raw.kind.to_ascii_lowercase().as_str() {
        "tool" => AddonKind::Tool,
        "diagnose" => AddonKind::Diagnose,
        _ => return None,
    };
    let id = raw.id.trim().to_ascii_lowercase();
    // An id is a settings key and a menu identity; a blank or whitespace one
    // cannot be either.
    if id.is_empty() || id.contains(char::is_whitespace) {
        return None;
    }
    let exec = PathBuf::from(raw.exec.trim());
    if exec.as_os_str().is_empty() {
        return None;
    }
    Some(Addon {
        name: if raw.name.trim().is_empty() { id.clone() } else { raw.name.trim().to_string() },
        id,
        author: raw.author.trim().to_string(),
        description: raw.description.trim().to_string(),
        version: raw.version.trim().to_string(),
        kind,
        exec,
        args: raw.args,
        workdir: raw.workdir.trim().to_string(),
        games: raw.games,
        source: source.to_path_buf(),
    })
}

/// One row a `diagnose` add-on produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// `problem`, `advice` or `ok`.
    pub level: FindingLevel,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingLevel {
    Problem,
    Advice,
    Ok,
}

/// Parse a `diagnose` add-on's stdout.
///
/// One finding per line: `level<TAB>title<TAB>detail`, with the detail optional.
/// Tab-separated rather than JSON so a manifest can be satisfied by a shell
/// script and an `echo`, which is the whole point of the tier - anything richer
/// would mean a serialisation library on the far side of every add-on.
///
/// A line that does not start with a known level is DROPPED rather than shown as
/// an unclassified row: a program's stray stderr-on-stdout, a progress line, or a
/// shell's own noise must not be able to raise something that looks like a
/// finding Eidos vouches for.
pub fn parse_findings(stdout: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let mut cells = line.split('\t');
        let level = match cells.next().map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("problem") => FindingLevel::Problem,
            Some("advice") => FindingLevel::Advice,
            Some("ok") => FindingLevel::Ok,
            _ => continue,
        };
        let title = cells.next().unwrap_or("").trim().to_string();
        if title.is_empty() {
            continue;
        }
        // Any further tabs belong to the detail: a path can contain one, and
        // splitting on all of them would truncate at the first.
        let detail = cells.collect::<Vec<_>>().join("\t").trim().to_string();
        out.push(Finding { level, title, detail });
    }
    out
}

/// Whether a path names a location rather than a bare command.
fn has_separator(p: &Path) -> bool {
    p.as_os_str().to_string_lossy().contains('/')
}

/// Resolve a bare command against `PATH`.
pub fn which(cmd: &Path) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(cmd))
        .find(|c| c.is_file())
}

/// The owned, deserialisable shape of a manifest.
#[derive(serde::Deserialize)]
struct RawAddon {
    id: String,
    #[serde(default)]
    name: String,
    kind: String,
    exec: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    workdir: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    games: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(pairs: &[(&str, &str)]) -> Context {
        Context {
            values: pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    const TOOL: &str = r#"
id = "wrye"
name = "Wrye Bash"
kind = "tool"
exec = "/usr/bin/wrye"
args = ["--data", "{data}"]
games = ["skyrimse"]
"#;

    #[test]
    fn a_manifest_round_trips_into_an_addon() {
        let a = parse_addon(TOOL, Path::new("/x/wrye.toml")).expect("valid");
        assert_eq!(a.id, "wrye");
        assert_eq!(a.name, "Wrye Bash");
        assert_eq!(a.kind, AddonKind::Tool);
        assert_eq!(a.args, vec!["--data", "{data}"]);
        assert!(a.applies_to("skyrimse"));
        assert!(a.applies_to("SkyrimSE"), "game ids match case-blind");
        assert!(!a.applies_to("fallout4"));
        // No `games` at all means every game, not none.
        let any = parse_addon("id='x'\nkind='tool'\nexec='/bin/true'\n", Path::new("/x")).unwrap();
        assert!(any.applies_to("anything"));
        // A manifest with no name falls back to the id rather than rendering blank.
        assert_eq!(any.name, "x");
    }

    #[test]
    fn a_manifest_missing_what_it_needs_is_refused() {
        // No id, a blank id, an id with a space (it is a settings key), an
        // unknown kind, no exec.
        for bad in [
            "kind='tool'\nexec='/bin/true'\n",
            "id=''\nkind='tool'\nexec='/bin/true'\n",
            "id='a b'\nkind='tool'\nexec='/bin/true'\n",
            "id='x'\nkind='wat'\nexec='/bin/true'\n",
            "id='x'\nkind='tool'\nexec=''\n",
            "not even toml [[[",
        ] {
            assert!(parse_addon(bad, Path::new("/x")).is_none(), "must refuse: {bad:?}");
        }
    }

    #[test]
    fn placeholders_expand_and_an_unknown_one_is_left_visible() {
        let c = ctx(&[("data", "/games/Data"), ("profile", "Default")]);
        assert_eq!(c.expand("--data {data}"), "--data /games/Data");
        assert_eq!(c.expand("{profile}/{data}"), "Default//games/Data");
        assert_eq!(c.expand("no placeholders"), "no placeholders");
        // An unknown key is left AS WRITTEN. Blanking it would turn
        // `--out {missing}` into `--out --next-flag` and the program would then
        // do something nobody asked for.
        assert_eq!(c.expand("--out {missing} --keep"), "--out {missing} --keep");
        assert_eq!(c.missing("--out {missing} {data}"), vec!["missing"]);
        assert!(c.missing("{data}").is_empty());
        // An unclosed brace is not a placeholder and must not eat the rest.
        assert_eq!(c.expand("50% {of"), "50% {of");
        assert_eq!(c.expand("{}"), "{}");
    }

    #[test]
    fn findings_parse_and_junk_lines_are_dropped() {
        let out = parse_findings(
            "problem\tMissing master\tSkyrim.esm is not enabled\n\
             advice\tOld version\n\
             ok\tAll good\t\n\
             \n\
             Reading 412 mods...\n\
             warning\tnot a level we know\n",
        );
        assert_eq!(out.len(), 3, "{out:?}");
        assert_eq!(out[0].level, FindingLevel::Problem);
        assert_eq!(out[0].detail, "Skyrim.esm is not enabled");
        assert_eq!(out[1].detail, "", "the detail is optional");
        assert_eq!(out[2].level, FindingLevel::Ok);
        // A progress line or a stray `warning:` must NOT become a row: a finding
        // on screen reads as something Eidos vouches for.
        assert!(!out.iter().any(|f| f.title.contains("not a level")));
    }

    #[test]
    fn a_detail_containing_a_tab_is_not_truncated_at_it() {
        let out = parse_findings("problem\tBad path\t/a\tb/c is missing\n");
        assert_eq!(out[0].detail, "/a\tb/c is missing");
    }

    #[test]
    fn a_missing_program_is_reported_rather_than_run() {
        let a = parse_addon(
            "id='x'\nkind='tool'\nexec='/nope/definitely/not/here'\n",
            Path::new("/x"),
        )
        .unwrap();
        assert!(a.unavailable().is_some_and(|m| m.contains("not there")));
        // A bare command is looked up on PATH instead of stat'd relative to cwd.
        let sh = parse_addon("id='x'\nkind='tool'\nexec='sh'\n", Path::new("/x")).unwrap();
        assert_eq!(sh.unavailable(), None, "sh is on PATH everywhere this runs");
        let nope =
            parse_addon("id='x'\nkind='tool'\nexec='eidos-nope-xyz'\n", Path::new("/x")).unwrap();
        assert!(nope.unavailable().is_some_and(|m| m.contains("PATH")));
    }

    #[test]
    fn two_manifests_claiming_one_id_do_not_silently_shadow_each_other() {
        let dir = std::env::temp_dir().join(format!("eidos-addons-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("a.toml"), "id='dup'\nname='First'\nkind='tool'\nexec='sh'\n").unwrap();
        fs::write(dir.join("b.toml"), "id='dup'\nname='Second'\nkind='tool'\nexec='sh'\n").unwrap();
        fs::write(dir.join("c.toml"), "id='ok'\nkind='tool'\nexec='sh'\n").unwrap();
        fs::write(dir.join("notes.txt"), "id='ignored'\n").unwrap();

        let addons = load_addons_from(&dir);
        assert_eq!(addons.len(), 2, "{addons:?}");
        assert_eq!(addons[0].name, "First", "the first file wins, deterministically");
        assert!(addons.iter().any(|a| a.id == "ok"));
        assert!(!addons.iter().any(|a| a.id == "ignored"), "only .toml is read");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_refused_manifest_says_why_instead_of_vanishing() {
        let dir = std::env::temp_dir().join(format!("eidos-addons-bad-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("a.toml"), "id='a b'\nkind='tool'\nexec='sh'\n").unwrap();
        fs::write(dir.join("b.toml"), "id='b'\nkind='wat'\nexec='sh'\n").unwrap();
        fs::write(dir.join("c.toml"), "id='c'\nkind='tool'\nexec=''\n").unwrap();
        fs::write(dir.join("d.toml"), "not toml at all [[[").unwrap();
        fs::write(dir.join("e.toml"), "id='e'\nkind='tool'\nexec='sh'\n").unwrap();

        let (ok, bad) = scan_addons_from(&dir);
        assert_eq!(ok.len(), 1);
        assert_eq!(bad.len(), 4, "{bad:?}");
        // Each reason names the actual problem: a list that only said "invalid"
        // would leave the user comparing their file to the docs line by line.
        let why = |n: &str| {
            bad.iter().find(|(p, _)| p.ends_with(n)).map(|(_, w)| w.clone()).unwrap_or_default()
        };
        assert!(why("a.toml").contains("space"), "{}", why("a.toml"));
        assert!(why("b.toml").contains("'wat'"), "{}", why("b.toml"));
        assert!(why("c.toml").contains("exec"), "{}", why("c.toml"));
        assert!(why("d.toml").contains("TOML"), "{}", why("d.toml"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_addons_directory_is_simply_no_addons() {
        assert!(load_addons_from(Path::new("/no/such/dir/at/all")).is_empty());
    }
}
