//! Per-instance tool list (MO2's `ExecutablesList`): the executables a user runs
//! through the merged view besides the game - xEdit, FNIS, BodySlide, Wrye Bash.
//!
//! Mirrors MO2's model: each `Executable` is `{title, binary, arguments,
//! workingDirectory}`; the list shown to the user merges per-game defaults
//! (MO2: the game plugin's `executables()`; Eidos: seeded from `GameDef`) with
//! the user's own entries, user entries winning on a title collision. Stored as
//! `<instance>/tools.ini`, one section per tool - Eidos's own file, so a plain
//! rewrite (no MO2 byte-preservation needed).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One tool the user can run through the merged view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// Display name and lookup key (case-insensitive), e.g. `SSEEdit`.
    pub title: String,
    /// The Windows executable: absolute, or relative to the game install dir.
    pub exe: PathBuf,
    /// Extra command-line arguments.
    pub args: Vec<String>,
    /// Working directory; `None` = the executable's own directory (MO2 default).
    pub workdir: Option<PathBuf>,
}

/// Read `tools.ini`. A missing file is an empty list.
pub fn read_tools(path: &Path) -> Vec<Tool> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out: Vec<Tool> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = eidos_ini::section_header(line) {
            if let Some(title) = section.strip_prefix("Tool/") {
                out.push(Tool {
                    title: title.to_string(),
                    exe: PathBuf::new(),
                    args: Vec::new(),
                    workdir: None,
                });
            }
            continue;
        }
        let (Some(tool), Some((k, v))) = (out.last_mut(), eidos_ini::key_value(line)) else {
            continue;
        };
        let v = v.trim();
        match k {
            "exe" => tool.exe = PathBuf::from(v),
            // One key per argument, written in order as arg0=, arg1=, ... (file
            // order == argument order). Lossless for arguments containing spaces.
            _ if k.len() > 3 && k.starts_with("arg") && k.as_bytes()[3..].iter().all(u8::is_ascii_digit) => {
                tool.args.push(v.to_string());
            }
            // Legacy single-line form (pre-per-key tools.ini): space-split.
            "args" if !v.is_empty() && tool.args.is_empty() => {
                tool.args = v.split(' ').map(String::from).collect();
            }
            "workdir" if !v.is_empty() => tool.workdir = Some(PathBuf::from(v)),
            _ => {}
        }
    }
    out.retain(|t| !t.exe.as_os_str().is_empty());
    out
}

/// Write the full `tools.ini`.
pub fn write_tools(path: &Path, tools: &[Tool]) -> io::Result<()> {
    let mut s = String::new();
    for t in tools {
        // A control char or empty title cannot round-trip the `[Tool/<title>]`
        // header (it would split the section across lines or vanish on re-read,
        // re-attaching this tool's keys to the previous one), so skip such an
        // entry rather than corrupt the whole file. The `add` command rejects
        // these up front; this guards a hand-edited or migrated bad entry.
        let title = t.title.trim();
        if title.is_empty() || title.chars().any(char::is_control) {
            continue;
        }
        s.push_str(&format!("[Tool/{title}]\n"));
        s.push_str(&format!("exe={}\n", t.exe.display()));
        // One key per argument: an argument may itself contain spaces (e.g. xEdit's
        // `-D:D:\My Mods\Data`), which a single space-joined `args=` line would
        // corrupt on the next read. File order == argument order.
        for (i, a) in t.args.iter().enumerate() {
            s.push_str(&format!("arg{i}={a}\n"));
        }
        if let Some(w) = &t.workdir {
            s.push_str(&format!("workdir={}\n", w.display()));
        }
        s.push('\n');
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, s)
}

/// Merge per-game defaults with the user's tools: user entries first and they
/// win a title collision (case-insensitive), like MO2's `ExecutablesList::load`.
pub fn merge_tools(user: Vec<Tool>, defaults: Vec<Tool>) -> Vec<Tool> {
    let mut out = user;
    for d in defaults {
        if !out.iter().any(|t| t.title.eq_ignore_ascii_case(&d.title)) {
            out.push(d);
        }
    }
    out
}

/// Per-game default tools (MO2's plugin-provided executables): the script
/// extender, when its loader is present in the game install dir. Decoupled
/// inputs so both the CLI and the GUI can call it without a GameDef dependency.
pub fn default_tools(script_extender_loader: Option<&str>, install: &Path) -> Vec<Tool> {
    let mut v = Vec::new();
    if let Some(loader) = script_extender_loader {
        if install.join(loader).is_file() {
            v.push(Tool {
                title: loader.trim_end_matches(".exe").to_string(),
                exe: PathBuf::from(loader),
                args: Vec::new(),
                workdir: None,
            });
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("eidos-tools-{}-{}.ini", std::process::id(), n))
    }

    #[test]
    fn round_trips_tools() {
        let p = tmp();
        let tools = vec![
            Tool {
                title: "SSEEdit".into(),
                exe: PathBuf::from("/mnt/Tools/SSEEdit/SSEEdit.exe"),
                // An argument containing spaces must survive the round trip - the
                // old space-joined `args=` line split it into four.
                args: vec!["-D:D:\\My Mods\\Data".into(), "-IKnowWhatImDoing".into()],
                workdir: None,
            },
            Tool {
                title: "BodySlide".into(),
                exe: PathBuf::from("Data/CalienteTools/BodySlide/BodySlide x64.exe"),
                args: Vec::new(),
                workdir: Some(PathBuf::from("/mnt/Tools")),
            },
        ];
        write_tools(&p, &tools).unwrap();
        assert_eq!(read_tools(&p), tools);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(read_tools(Path::new("/no/such/tools.ini")).is_empty());
    }

    #[test]
    fn legacy_space_joined_args_still_read() {
        // A pre-per-key tools.ini (single `args=` line) must still load.
        let p = tmp();
        fs::write(&p, "[Tool/Old]\nexe=/x/old.exe\nargs=-a -b -c\n").unwrap();
        let t = read_tools(&p);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].args, vec!["-a", "-b", "-c"]);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn control_char_title_is_skipped_on_write() {
        // A newline in a title would split the [Tool/..] header and corrupt the
        // file; write_tools must drop such an entry, not emit it.
        let p = tmp();
        let tools = vec![
            Tool { title: "Bad\nTitle".into(), exe: PathBuf::from("/x/a.exe"), args: vec![], workdir: None },
            Tool { title: "Good".into(), exe: PathBuf::from("/x/b.exe"), args: vec![], workdir: None },
        ];
        write_tools(&p, &tools).unwrap();
        let back = read_tools(&p);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].title, "Good");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn merge_user_wins_title_collisions() {
        let user = vec![Tool {
            title: "SKSE".into(),
            exe: PathBuf::from("/custom/skse64_loader.exe"),
            args: vec!["-forcesteamloader".into()],
            workdir: None,
        }];
        let defaults = vec![
            Tool {
                title: "skse".into(), // case-insensitive collision
                exe: PathBuf::from("skse64_loader.exe"),
                args: Vec::new(),
                workdir: None,
            },
            Tool {
                title: "Launcher".into(),
                exe: PathBuf::from("SkyrimSELauncher.exe"),
                args: Vec::new(),
                workdir: None,
            },
        ];
        let merged = merge_tools(user, defaults);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].exe, PathBuf::from("/custom/skse64_loader.exe")); // user won
        assert_eq!(merged[1].title, "Launcher");
    }
}
