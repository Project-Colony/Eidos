//! Crash logs written by the crash loggers people install with a script extender:
//! Crash Logger SSE/AE, Buffout 4, NetScriptFramework and Trainwreck.
//!
//! The parser is a tolerant line scan over the section headings these loggers
//! share. It keeps names only, so a format change loses detail rather than
//! producing a wrong verdict, and what it returns are leads for the user.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What a crash log says about a crash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashLog {
    pub path: PathBuf,
    /// "Crash Logger", "Buffout 4", "NetScriptFramework" or "Trainwreck"; None when
    /// the log names none of them.
    pub logger: Option<&'static str>,
    /// The first line starting "Unhandled exception" or "Unhandled native exception", trimmed.
    pub exception: Option<String>,
    /// DLL names from the probable call stack, in order, deduplicated case-insensitively.
    pub modules: Vec<String>,
    /// Plugin names from the sections before the module and plugin lists, in order,
    /// deduplicated case-insensitively.
    pub plugins: Vec<String>,
}

/// The `(directory, file-name prefix, extension)` triples where a game's crash
/// loggers write. `docs` is the game's `Documents/My Games/<game>`. Crash Logger
/// and Buffout 4 write `crash-<timestamp>.log` beside the extender log, Trainwreck
/// writes into `Crashlogs/`, and NetScriptFramework writes into
/// `Data/NetScriptFramework/Crash`, which the union sends to the Overwrite.
pub fn crash_log_dirs(
    game_id: &str,
    docs: &Path,
    overwrite: &Path,
) -> Vec<(PathBuf, &'static str, &'static str)> {
    let extender = match game_id {
        "skyrimse" | "enderalse" | "skyrimvr" => "SKSE",
        "fallout4" | "fallout4vr" => "F4SE",
        "starfield" => "SFSE",
        _ => return Vec::new(),
    };
    let mut dirs = vec![
        (docs.join(extender), "crash-", "log"),
        (docs.join(extender).join("Crashlogs"), "", "log"),
    ];
    if extender == "SKSE" {
        dirs.push((overwrite.join("NetScriptFramework/Crash"), "Crash_", "txt"));
    }
    dirs
}

/// The newest crash log modified at or after `since` (the launch start), across
/// every directory of [`crash_log_dirs`]. Older logs the loggers keep belong to
/// earlier sessions and are ignored.
pub fn newest_crash_log(
    game_id: &str,
    docs: &Path,
    overwrite: &Path,
    since: SystemTime,
) -> Option<PathBuf> {
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    for (dir, prefix, ext) in crash_log_dirs(game_id, docs, overwrite) {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            let ext_ok = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(ext));
            if !ext_ok || !name.starts_with(&prefix.to_ascii_lowercase()) {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(mtime) = meta.modified() else {
                continue;
            };
            if meta.is_file() && mtime >= since && newest.as_ref().is_none_or(|(t, _)| mtime > *t) {
                newest = Some((mtime, path));
            }
        }
    }
    newest.map(|(_, path)| path)
}

/// Read a crash log's logger, exception line, call-stack DLLs and the plugins
/// named in its crash sections. `text` is the file read lossily.
pub fn parse_crash_log(path: &Path, text: &str) -> CrashLog {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let logger = if text.contains("CrashLogger") || text.contains("Crash Logger") {
        Some("Crash Logger")
    } else if text.contains("Buffout 4") {
        Some("Buffout 4")
    } else if text.contains("NetScriptFramework") || file_name.starts_with("crash_") {
        Some("NetScriptFramework")
    } else if text.contains("Trainwreck") {
        Some("Trainwreck")
    } else {
        None
    };
    let exception = text
        .lines()
        .map(str::trim)
        .find(|l| {
            l.starts_with("Unhandled exception") || l.starts_with("Unhandled native exception")
        })
        .map(str::to_owned);

    let mut modules = Vec::new();
    let mut in_stack = false;
    for line in text.lines() {
        let lower = line.trim().to_ascii_lowercase();
        if !in_stack {
            in_stack =
                lower.starts_with("probable call stack") || lower.starts_with("probable callstack");
            continue;
        }
        if lower.starts_with("registers") {
            break;
        }
        // shortcut: a DLL name with a space keeps only its last word, read the
        // frame's columns if real logs show such names.
        for token in line.split_whitespace() {
            let token = token.trim_matches(|c| matches!(c, '(' | ')' | '[' | ']'));
            // ASCII lowercasing keeps byte offsets, so `at` indexes `token` too.
            if let Some(at) = token.to_ascii_lowercase().find(".dll+") {
                push_unique(&mut modules, &token[..at + 4]);
            }
        }
    }

    let mut plugins = Vec::new();
    for line in text.lines() {
        let lower = line.trim().to_ascii_lowercase();
        if ["modules", "loaded modules", "plugins"]
            .iter()
            .any(|h| lower.starts_with(h))
            || lower.ends_with("plugins:")
        {
            break;
        }
        let mut candidates: Vec<&str> = quoted(line, '"').chain(quoted(line, '`')).collect();
        for label in ["File:", "Modified by:"] {
            if let Some(at) = line.find(label) {
                candidates.push(&line[at + label.len()..]);
            }
        }
        for candidate in candidates {
            for name in candidate.split("->").flat_map(|s| s.split("<-")) {
                let name = name.trim_matches(|c: char| {
                    matches!(c, '"' | '`' | '(' | ')') || c.is_whitespace()
                });
                let lower = name.to_ascii_lowercase();
                if [".esp", ".esm", ".esl"].iter().any(|e| lower.ends_with(e)) {
                    push_unique(&mut plugins, name);
                }
            }
        }
    }

    CrashLog {
        path: path.to_path_buf(),
        logger,
        exception,
        modules,
        plugins,
    }
}

/// The substrings of `line` between pairs of `quote`.
fn quoted(line: &str, quote: char) -> impl Iterator<Item = &str> {
    line.split(quote).skip(1).step_by(2)
}

fn push_unique(names: &mut Vec<String>, name: &str) {
    if !name.is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
        names.push(name.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const CRASH_LOGGER: &str = "Skyrim SSE v1.6.1170\nCrashLoggerSSE v1-15-0-0 Jan 30 2026 10:00:00\n\n\
Unhandled exception \"EXCEPTION_ACCESS_VIOLATION\" at 0x7FF6A1B2C3D4 SkyrimSE.exe+0123456\n\n\
PROBABLE CALL STACK:\n\t[0] 0x7FF6A1B2C3D4 SkyrimSE.exe+0123456\n\t[1] 0x7FFB11112222 po3_PapyrusExtender.dll+0012345\n\
\t[2] 0x7FFB33334444 hdtSMP64.dll+00ABCDE\n\t[3] 0x7FFB11112299 po3_PapyrusExtender.dll+0012399\n\n\
REGISTERS:\n\tRAX 0x1        (TESObjectREFR*)\n\t\tFile: \"Skyrim.esm\"\n\
\t\tModified by: Skyrim.esm -> Unofficial Skyrim Special Edition Patch.esp -> MyFollower.esp\n\n\
STACK:\n\t[RSP+8 ] 0x2   (TESNPC*)\n\t\tFile: \"MyFollower.esp\"\n\n\
MODULES:\n\tSkyrimSE.exe 0x7FF6A0000000\n\tunrelated.dll 0x7FFB00000000\n\n\
SKSE PLUGINS:\n\tunrelated.dll v1\n\nPLUGINS:\n\tLight: 0\tRegular: 2\tTotal: 2\n\t[00] Skyrim.esm\n\t[01] NotInvolved.esp\n";

    const BUFFOUT: &str = "Fallout 4 v1.10.984\nBuffout 4 v1.37.0 Mar 12 2025 22:11:48\n\n\
Unhandled exception \"EXCEPTION_ACCESS_VIOLATION\" at 0x7FF7C3F1E2B0 Fallout4.exe+0D1E2B0\n\n\
PROBABLE CALL STACK:\n\t[0] 0x7FF7C3F1E2B0 Fallout4.exe+0D1E2B0\n\t[1] 0x7FFB55556666 f4ee.dll+000123\n\n\
REGISTERS:\n\tRAX 0x1        (TESObjectREFR*)\n\t\tFile: \"Fallout4.esm\"\n\
\tRCX 0x2        (TESObjectARMO*)\n\t\tFile: \"Armorsmith Extended.esp\"\n\n\
F4SE PLUGINS:\n\tf4ee.dll v1.6.20\n\nPLUGINS:\n\t[00] Fallout4.esm\n\t[01] Bystander.esp\n";

    const NSF: &str = "Unhandled native exception occurred at 0x7FF6A1B2C3D4 (SkyrimSE.exe+0123456) on thread 1234!\n\n\
Possible relevant objects (1)\n{\n  [ 0] Character(FormId: 00000014, File: `Skyrim.esm <- Update.esm`)\n}\n\n\
Probable callstack\n{\n  [0] 0x7FF6A1B2C3D4 (SkyrimSE.exe+0123456)\n  [1] 0x7FFB11112222 (SomeMod.dll+12345)\n}\n\n\
Registers\n{\n}\n";

    fn tmp() -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "eidos-crashlog-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Whole seconds, as the launch record stores them.
    fn launch_time() -> SystemTime {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        UNIX_EPOCH + Duration::from_secs(now.as_secs())
    }

    fn put(path: &Path, mtime: SystemTime) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "crash").unwrap();
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
    }

    #[test]
    fn crash_logger_names_dlls_and_plugins_from_the_crash_sections() {
        let log = parse_crash_log(Path::new("crash-2026-11-09-21-30-00.log"), CRASH_LOGGER);
        assert_eq!(log.logger, Some("Crash Logger"));
        assert!(log
            .exception
            .as_deref()
            .unwrap()
            .starts_with("Unhandled exception \"EXCEPTION_ACCESS_VIOLATION\""));
        assert_eq!(log.modules, ["po3_PapyrusExtender.dll", "hdtSMP64.dll"]);
        assert_eq!(
            log.plugins,
            [
                "Skyrim.esm",
                "Unofficial Skyrim Special Edition Patch.esp",
                "MyFollower.esp"
            ]
        );
    }

    #[test]
    fn buffout4_log_parses_with_the_same_sections() {
        let log = parse_crash_log(Path::new("crash-x.log"), BUFFOUT);
        assert_eq!(
            (log.logger, log.modules.as_slice()),
            (Some("Buffout 4"), ["f4ee.dll".to_string()].as_slice())
        );
        assert_eq!(log.plugins, ["Fallout4.esm", "Armorsmith Extended.esp"]);
    }

    #[test]
    fn a_log_without_a_known_logger_or_exception_line_says_so() {
        let log = parse_crash_log(Path::new("crash-x.log"), "PROBABLE CALL STACK:\n");
        assert_eq!((log.logger, log.exception), (None, None));
    }

    #[test]
    fn netscriptframework_crash_txt_is_recognised() {
        let log = parse_crash_log(Path::new("Crash_2026_11_9_21-30-00.txt"), NSF);
        assert_eq!(log.logger, Some("NetScriptFramework"));
        assert_eq!(
            log.exception.as_deref(),
            Some("Unhandled native exception occurred at 0x7FF6A1B2C3D4 (SkyrimSE.exe+0123456) on thread 1234!")
        );
        assert_eq!(log.modules, ["SomeMod.dll"]);
        assert_eq!(log.plugins, ["Skyrim.esm", "Update.esm"]);
    }

    #[test]
    fn a_log_older_than_the_launch_is_ignored() {
        let (docs, ow) = (tmp(), tmp());
        let since = launch_time();
        put(
            &docs.join("SKSE/crash-old.log"),
            since - Duration::from_secs(60),
        );
        put(
            &docs.join("SKSE/skse64.log"),
            since + Duration::from_secs(5),
        );
        assert_eq!(newest_crash_log("skyrimse", &docs, &ow, since), None);
        put(
            &docs.join("SKSE/Crashlogs/trainwreck.log"),
            since + Duration::from_secs(5),
        );
        assert_eq!(
            newest_crash_log("skyrimse", &docs, &ow, since),
            Some(docs.join("SKSE/Crashlogs/trainwreck.log"))
        );
    }

    #[test]
    fn the_newest_log_wins_across_directories() {
        let (docs, ow) = (tmp(), tmp());
        let since = launch_time();
        put(
            &docs.join("SKSE/crash-a.log"),
            since + Duration::from_secs(5),
        );
        put(
            &ow.join("NetScriptFramework/Crash/Crash_b.txt"),
            since + Duration::from_secs(9),
        );
        assert_eq!(
            newest_crash_log("skyrimse", &docs, &ow, since),
            Some(ow.join("NetScriptFramework/Crash/Crash_b.txt"))
        );
    }

    #[test]
    fn games_without_a_known_crash_logger_have_no_crash_log_dirs() {
        let (docs, ow) = (Path::new("/d"), Path::new("/o"));
        assert!(crash_log_dirs("oblivion", docs, ow).is_empty());
        assert_eq!(
            crash_log_dirs("fallout4vr", docs, ow),
            [
                (docs.join("F4SE"), "crash-", "log"),
                (docs.join("F4SE/Crashlogs"), "", "log")
            ]
        );
        assert_eq!(
            crash_log_dirs("starfield", docs, ow)[1].0,
            docs.join("SFSE/Crashlogs")
        );
    }

    #[test]
    fn lossy_text_still_yields_names() {
        let text = String::from_utf8_lossy(
            b"PROBABLE CALL STACK:\n\t[1] 0x1 Caf\xe9.dll+01\nREGISTERS:\n\t\tFile: \"Caf\xe9.esp\"\n",
        )
        .into_owned();
        let log = parse_crash_log(Path::new("crash-x.log"), &text);
        assert_eq!((log.modules.len(), log.plugins.len()), (1, 1));
    }
}
