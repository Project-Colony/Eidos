//! Reading the script extender's own log, so "did my SKSE plugins load?" has an
//! answer instead of an inference.
//!
//! Eidos can only tell the user that FUSE passthrough is on or off, from which a
//! DLL load failure is *likely* or *unlikely*. The extender itself writes down
//! what actually happened, one line per plugin, and that is the evidence: a
//! plugin refused for an incompatible runtime version looks exactly like one
//! refused because the manager failed it, and only this file distinguishes them.
//!
//! Two line shapes matter (SKSE/F4SE `PluginManager::Init`):
//!
//! ```text
//! plugin <path> (<infoVersion:8hex> <name> <version:8hex>) loaded correctly
//! plugin <path> (00000001 po3_Tweaks 00000456) disabled, incompatible with current runtime version
//! couldn't load plugin <path> (Error 126)
//! ```
//!
//! Only `loaded correctly` and `no version data` count as success; every other
//! status is surfaced verbatim, because the extender's own wording ("disabled,
//! bad version data", "disabled, unsupported version independence method") is
//! more precise than anything this crate could restate.

use std::path::{Path, PathBuf};

/// One plugin's line from the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SePluginLoad {
    /// The plugin's declared name, or the DLL's file name when the line never got
    /// far enough to read one (the `couldn't load` shape).
    pub name: String,
    /// The DLL's file name, which is what the user sees in their mod folder.
    pub dll: String,
    /// The declared version, as the 8-hex field decoded into `a.b.c.d`.
    pub version: Option<String>,
    /// The extender's own words for what happened.
    pub status: String,
    pub loaded: bool,
}

/// Where the script extender writes its log for a game.
///
/// The location is split by engine generation, not by preference: the Gamebryo
/// titles (Oblivion, FO3, New Vegas) write beside the executable, and everything
/// from Skyrim on writes under the prefix's `Documents/My Games/<game>`. `docs`
/// is that per-game Documents directory; `install` is the game folder.
///
/// A `match` on the id rather than a `GameDef` field, so the game-definition JSON
/// and its deserializer stay untouched for what is one path per engine.
pub fn se_log_path(game_id: &str, docs: &Path, install: &Path) -> Option<PathBuf> {
    let under_docs = |rel: &str| Some(docs.join(rel));
    match game_id {
        "skyrimse" | "enderalse" | "skyrimsevr" => under_docs("SKSE/skse64.log"),
        "skyrimvr" => under_docs("SKSE/sksevr.log"),
        "skyrim" | "enderal" => under_docs("SKSE/skse.log"),
        "fallout4" => under_docs("F4SE/f4se.log"),
        "fallout4vr" => under_docs("F4SE/f4sevr.log"),
        "starfield" => under_docs("SFSE/sfse.log"),
        "oblivion" => Some(install.join("obse.log")),
        "falloutnv" | "tp" => Some(install.join("nvse.log")),
        "fallout3" => Some(install.join("fose.log")),
        _ => None,
    }
}

/// Parse a script-extender log into one entry per plugin it tried to load.
///
/// Lines that are neither shape are ignored: the log is mostly the extender's own
/// startup chatter, and a parser that guessed at it would invent failures.
pub fn parse_se_log(text: &str) -> Vec<SePluginLoad> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = strip_prefix_ci(line, "couldn't load plugin ") {
            out.push(parse_failed_load(rest));
        } else if let Some(rest) = strip_prefix_ci(line, "plugin ") {
            if let Some(entry) = parse_plugin_line(rest) {
                out.push(entry);
            }
        }
    }
    out
}

/// `<path> (<8hex> <name> <8hex>) <status>`.
fn parse_plugin_line(rest: &str) -> Option<SePluginLoad> {
    let (open, close) = info_group(rest)?;
    let path = rest[..open].trim();
    let info = &rest[open + 1..close];
    let status = clean_status(rest[close + 1..].trim());

    // The name may contain spaces ("SSE Engine Fixes"), so take the first and last
    // fields and treat everything between as the name.
    let fields: Vec<&str> = info.split_whitespace().collect();
    let (name, version) = match fields.as_slice() {
        [_, mid @ .., ver] if !mid.is_empty() => (mid.join(" "), decode_version(ver)),
        _ => (dll_name(path).to_string(), None),
    };
    Some(SePluginLoad {
        name,
        dll: dll_name(path).to_string(),
        version,
        loaded: is_success(&status),
        status,
    })
}

/// `<path> (Error 126)` - the shape that fires when the DLL could not even be
/// mapped, which is what a passthrough failure or a missing dependency looks like.
fn parse_failed_load(rest: &str) -> SePluginLoad {
    let (path, tail) = match rest.rfind('(') {
        Some(i) => (rest[..i].trim(), rest[i + 1..].trim_end_matches(')').trim()),
        None => (rest.trim(), ""),
    };
    let dll = dll_name(path).to_string();
    // Windows error codes worth translating: these two are the whole of what goes
    // wrong with a script-extender plugin in practice, and neither is obvious.
    let code = tail.split_whitespace().last().and_then(|n| n.parse::<u32>().ok());
    let status = match code {
        Some(126) => "could not be loaded (error 126: a DLL it depends on is missing)".to_string(),
        Some(193) => {
            "could not be loaded (error 193: wrong architecture - a 32-bit DLL in a 64-bit game, or the reverse)"
                .to_string()
        }
        _ if tail.is_empty() => "could not be loaded".to_string(),
        _ => format!("could not be loaded ({tail})"),
    };
    SePluginLoad { name: dll.clone(), dll, version: None, status, loaded: false }
}

/// The byte range of the `(<8hex> <name> <8hex>)` group, found by looking for a
/// `(` followed by exactly eight hex digits and a space. Searching for the group's
/// SHAPE rather than the first `(` means a Windows path containing a parenthesis
/// ("Program Files (x86)") does not derail the parse.
fn info_group(rest: &str) -> Option<(usize, usize)> {
    for (open, _) in rest.match_indices('(') {
        let after = &rest[open + 1..];
        let looks_right = after.len() > 8
            && after[..8].bytes().all(|b| b.is_ascii_hexdigit())
            && after.as_bytes()[8] == b' ';
        if looks_right {
            if let Some(rel) = after.find(')') {
                return Some((open, open + 1 + rel));
            }
        }
    }
    None
}

/// Only these two statuses mean the plugin is live. "no version data" is an old
/// plugin that declared nothing, which the extender loads anyway.
fn is_success(status: &str) -> bool {
    let s = status.trim().to_ascii_lowercase();
    s.starts_with("loaded correctly") || s.starts_with("no version data")
}

/// Drop the trailing `(handle N)` the extender appends on success - a load-order
/// handle, not information for the user.
fn clean_status(status: &str) -> String {
    match status.rfind("(handle ") {
        Some(i) if status.trim_end().ends_with(')') => status[..i].trim_end().to_string(),
        _ => status.to_string(),
    }
}

/// Decode the extender's packed 8-hex version field into `a.b.c.d`. Its layout is
/// 8/8/12/4 bits (major/minor/build/sub), the same packing `MAKE_EXE_VERSION`
/// produces.
fn decode_version(hex: &str) -> Option<String> {
    let v = u32::from_str_radix(hex.trim(), 16).ok()?;
    if v == 0 {
        return None;
    }
    Some(format!("{}.{}.{}.{}", v >> 24, (v >> 16) & 0xFF, (v >> 4) & 0xFFF, v & 0xF))
}

/// The file name of a Windows or Unix path, without splitting on the drive letter.
fn dll_name(path: &str) -> &str {
    path.trim().rsplit(['\\', '/']).next().unwrap_or(path).trim()
}

/// `str::strip_prefix`, case-insensitively - the extender's own casing has changed
/// between versions and is not worth depending on.
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    (s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix))
        .then(|| &s[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    // A real-shaped skse64.log: the happy path, the three failure statuses that
    // actually turn up, an old plugin with no version data, and the mapping
    // failure that a broken manager produces.
    const LOG: &str = concat!(
        "SKSE64 runtime: initialize (version = 2.2.6 01050610 ...)\n",
        "plugin C:\\Program Files (x86)\\Data\\SKSE\\Plugins\\po3_Tweaks.dll (00000001 powerofthree's Tweaks 00000456) loaded correctly (handle 1)\n",
        "plugin Z:\\home\\me\\mods\\Engine Fixes\\SKSE\\Plugins\\EngineFixes.dll (00000001 SSE Engine Fixes 0004B000) loaded correctly (handle 2)\n",
        "plugin C:\\Data\\SKSE\\Plugins\\old.dll (00000001 OldPlugin 00000000) no version data\n",
        "plugin C:\\Data\\SKSE\\Plugins\\stale.dll (00000001 StalePlugin 00000100) disabled, incompatible with current runtime version\n",
        "plugin C:\\Data\\SKSE\\Plugins\\bad.dll (00000001 BadPlugin 00000100) disabled, unsupported version independence method\n",
        "couldn't load plugin C:\\Data\\SKSE\\Plugins\\missing.dll (Error 126)\n",
        "couldn't load plugin C:\\Data\\SKSE\\Plugins\\wrongbits.dll (Error 193)\n",
        "dispatch message to plugin listeners\n",
    );

    #[test]
    fn reads_every_plugin_line_and_nothing_else() {
        let got = parse_se_log(LOG);
        assert_eq!(got.len(), 7, "{got:#?}");
        let loaded: Vec<&str> = got.iter().filter(|p| p.loaded).map(|p| p.dll.as_str()).collect();
        assert_eq!(loaded, ["po3_Tweaks.dll", "EngineFixes.dll", "old.dll"]);
    }

    #[test]
    fn a_parenthesis_in_the_path_does_not_derail_the_parse() {
        // "Program Files (x86)" appears before the info group, so a parser that
        // took the first '(' would read "x86" as the version fields.
        let p = &parse_se_log(LOG)[0];
        assert_eq!(p.dll, "po3_Tweaks.dll");
        assert_eq!(p.name, "powerofthree's Tweaks");
        assert!(p.loaded);
        // The trailing `(handle 1)` is dropped: it is bookkeeping, not status.
        assert_eq!(p.status, "loaded correctly");
    }

    #[test]
    fn a_name_with_spaces_survives_and_the_version_decodes() {
        let p = &parse_se_log(LOG)[1];
        assert_eq!(p.name, "SSE Engine Fixes");
        // 0004B000 -> 0.4.b00.0
        assert_eq!(p.version.as_deref(), Some("0.4.2816.0"));
    }

    #[test]
    fn refused_plugins_keep_the_extenders_own_wording() {
        let got = parse_se_log(LOG);
        let stale = got.iter().find(|p| p.dll == "stale.dll").unwrap();
        assert!(!stale.loaded);
        assert_eq!(stale.status, "disabled, incompatible with current runtime version");
        let bad = got.iter().find(|p| p.dll == "bad.dll").unwrap();
        assert_eq!(bad.status, "disabled, unsupported version independence method");
        // No version data is a SUCCESS: the extender loads such a plugin anyway.
        let old = got.iter().find(|p| p.dll == "old.dll").unwrap();
        assert!(old.loaded);
        assert_eq!(old.version, None);
    }

    #[test]
    fn the_two_windows_error_codes_are_explained() {
        let got = parse_se_log(LOG);
        let missing = got.iter().find(|p| p.dll == "missing.dll").unwrap();
        assert!(!missing.loaded);
        assert!(missing.status.contains("depends on is missing"), "{}", missing.status);
        let bits = got.iter().find(|p| p.dll == "wrongbits.dll").unwrap();
        assert!(bits.status.contains("wrong architecture"), "{}", bits.status);
    }

    #[test]
    fn a_log_with_no_plugin_lines_yields_nothing() {
        assert!(parse_se_log("").is_empty());
        assert!(parse_se_log("SKSE64 runtime: initialize\nchecking plugin directory\n").is_empty());
        // A truncated line is skipped, not guessed at.
        assert!(parse_se_log("plugin C:\\a.dll (00000001 Broken").is_empty());
    }

    #[test]
    fn log_paths_split_between_documents_and_the_install_dir() {
        let docs = Path::new("/pfx/docs/Skyrim Special Edition");
        let install = Path::new("/games/Skyrim");
        assert_eq!(
            se_log_path("skyrimse", docs, install),
            Some(docs.join("SKSE/skse64.log"))
        );
        assert_eq!(se_log_path("fallout4", docs, install), Some(docs.join("F4SE/f4se.log")));
        // The Gamebryo titles write beside the executable instead.
        assert_eq!(se_log_path("falloutnv", docs, install), Some(install.join("nvse.log")));
        assert_eq!(se_log_path("oblivion", docs, install), Some(install.join("obse.log")));
        // A game with no script extender has no log to look for.
        assert_eq!(se_log_path("morrowind", docs, install), None);
    }
}
