//! Which program the desktop hands `nxm://` links to, and registering Eidos.

use std::path::{Path, PathBuf};

const DESKTOP_ID: &str = "eidos-nxm.desktop";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NxmHandler {
    /// No program is registered.
    Unset,
    /// Eidos's own entry, pointing at a program that exists.
    Eidos { exec: String },
    /// Eidos's own entry, pointing at a program that is gone.
    EidosMissing { exec: String },
    /// Another program owns the scheme.
    Other { desktop: String },
    /// `xdg-mime` could not be asked.
    Unknown,
}

/// The first token of the first `Exec=` line, double quotes removed.
pub fn exec_program(desktop_text: &str) -> Option<String> {
    let line = desktop_text
        .lines()
        .find_map(|l| l.trim_start().strip_prefix("Exec="))?
        .trim();
    let tok = if let Some(rest) = line.strip_prefix('"') {
        rest.split('"').next()?
    } else {
        line.split_whitespace().next()?
    };
    (!tok.is_empty()).then(|| tok.to_string())
}

pub fn classify(
    desktop_id: &str,
    desktop_text: Option<&str>,
    exists: impl Fn(&str) -> bool,
) -> NxmHandler {
    if desktop_id.is_empty() {
        return NxmHandler::Unset;
    }
    if desktop_id != DESKTOP_ID {
        return NxmHandler::Other {
            desktop: desktop_id.to_string(),
        };
    }
    let exec = desktop_text.and_then(exec_program).unwrap_or_default();
    if !exec.is_empty() && exists(&exec) {
        NxmHandler::Eidos { exec }
    } else {
        NxmHandler::EidosMissing { exec }
    }
}

fn applications_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![eidos_paths::desktop_entries_dir()];
    let sys = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    dirs.extend(
        sys.split(':')
            .filter(|d| !d.is_empty())
            .map(|d| Path::new(d).join("applications")),
    );
    dirs
}

fn program_exists(prog: &str) -> bool {
    let p = Path::new(prog);
    if p.is_absolute() {
        return p.is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|d| d.join(prog).is_file()))
}

/// Asks `xdg-mime`, so it spawns a process: never call it from a code path that
/// runs on every edit.
pub fn read() -> NxmHandler {
    let out = std::process::Command::new("xdg-mime")
        .args(["query", "default", "x-scheme-handler/nxm"])
        .output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        _ => return NxmHandler::Unknown,
    };
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let text = applications_dirs()
        .into_iter()
        .find_map(|d| std::fs::read_to_string(d.join(&id)).ok());
    classify(&id, text.as_deref(), program_exists)
}

/// Writes the desktop entry and makes it the default `nxm` handler.
pub fn register() -> Result<PathBuf, String> {
    let (exe, needs_marker) = crate::this_exe();
    let marker = if needs_marker {
        format!(" {}", crate::CLI_MARKER)
    } else {
        String::new()
    };
    let apps = eidos_paths::desktop_entries_dir();
    std::fs::create_dir_all(&apps).map_err(|e| format!("{}: {e}", apps.display()))?;
    let desktop = apps.join(DESKTOP_ID);
    let body = format!(
        "[Desktop Entry]\nType=Application\nName=Eidos (Nexus nxm handler)\n\
         Exec={}{marker} nxm %u\nMimeType=x-scheme-handler/nxm;\nNoDisplay=true\nTerminal=false\n",
        exe.display()
    );
    std::fs::write(&desktop, body).map_err(|e| format!("{}: {e}", desktop.display()))?;
    // Best effort: some desktops only see a new entry after the cache refresh.
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps)
        .status();
    let st = std::process::Command::new("xdg-mime")
        .args(["default", DESKTOP_ID, "x-scheme-handler/nxm"])
        .status()
        .map_err(|e| format!("xdg-mime: {e}"))?;
    if !st.success() {
        return Err(format!("xdg-mime default exited with {st}"));
    }
    Ok(desktop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_first_token_handles_quotes_and_field_codes() {
        assert_eq!(
            exec_program("[Desktop Entry]\nExec=/opt/eidos/eidos --eidos-cli nxm %u\n").as_deref(),
            Some("/opt/eidos/eidos")
        );
        assert_eq!(
            exec_program("Exec=\"/home/a b/eidos-gui\" nxm %u\n").as_deref(),
            Some("/home/a b/eidos-gui")
        );
        assert_eq!(exec_program("Name=x\n"), None);
    }

    #[test]
    fn classify_tells_eidos_moved_binaries_and_other_apps_apart() {
        let text = Some("Exec=/gone/eidos nxm %u\n");
        assert_eq!(classify("", None, |_| true), NxmHandler::Unset);
        assert_eq!(
            classify("eidos-nxm.desktop", text, |_| true),
            NxmHandler::Eidos {
                exec: "/gone/eidos".into()
            }
        );
        assert_eq!(
            classify("eidos-nxm.desktop", text, |_| false),
            NxmHandler::EidosMissing {
                exec: "/gone/eidos".into()
            }
        );
        assert_eq!(
            classify("modorganizer2-nxm-handler.desktop", None, |_| true),
            NxmHandler::Other {
                desktop: "modorganizer2-nxm-handler.desktop".into()
            }
        );
    }
}
