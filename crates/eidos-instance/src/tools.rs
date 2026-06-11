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
            "args" if !v.is_empty() => {
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
        s.push_str(&format!("[Tool/{}]\n", t.title));
        s.push_str(&format!("exe={}\n", t.exe.display()));
        s.push_str(&format!("args={}\n", t.args.join(" ")));
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
                args: vec!["-IKnowWhatImDoing".into()],
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
