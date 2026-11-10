//! A profile's own launch variables and wrapper commands: `env=NAME=value` and
//! `wrapper=<command line>` lines in `profiles/<name>/launch-env.ini`.
//!
//! A file of its own, not MO2's `settings.ini`: MO2 reads that with QSettings,
//! which keeps one value per key, so a second `env=` line would be lost the
//! next time MO2 saved the profile.

use super::{read_text_lossy, Profile};
use crate::tools::is_env_name;

/// The file name, in the profile directory.
pub const LAUNCH_ENV_FILE: &str = "launch-env.ini";

/// What a profile adds to its runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchEnv {
    /// Variables, in file order.
    pub env: Vec<(String, String)>,
    /// Wrapper command lines, outermost first.
    pub wrappers: Vec<String>,
}

/// Read the text of a `launch-env.ini`. Blank lines and lines starting with `#`
/// or `;` are skipped. Any other line that is not `env=NAME=value` with a valid
/// name, or `wrapper=` with a command, is ignored and its 1-based number
/// returned, so the editor can name it.
pub fn parse_launch_env(text: &str) -> (LaunchEnv, Vec<usize>) {
    let mut out = LaunchEnv::default();
    let mut ignored = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        let var = line
            .strip_prefix("env=")
            .and_then(|kv| kv.split_once('='))
            .filter(|(name, _)| is_env_name(name));
        let wrapper = line
            .strip_prefix("wrapper=")
            .map(str::trim)
            .filter(|w| !w.is_empty());
        match (var, wrapper) {
            (Some((name, value)), _) => out.env.push((name.to_string(), value.to_string())),
            (None, Some(w)) => out.wrappers.push(w.to_string()),
            (None, None) => ignored.push(i + 1),
        }
    }
    (out, ignored)
}

impl Profile {
    /// This profile's `launch-env.ini`. A missing or unreadable file adds nothing.
    pub fn launch_env(&self) -> LaunchEnv {
        read_text_lossy(&self.dir().join(LAUNCH_ENV_FILE))
            .map(|(text, _)| parse_launch_env(&text).0)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_env_reads_env_and_wrapper_lines_and_names_the_ones_it_ignores() {
        let text = "# comment\nenv=PROTON_ENABLE_NVAPI=1\nenv=1BAD=x\n\nwrapper=gamemoderun\nenv=NOVALUE\nwrapper=\nfoo=bar\n; note\nenv=DXVK_HUD=fps\nwrapper=gamescope -W 1280 -H 800 -f -- %command%\n";
        let (l, ignored) = parse_launch_env(text);
        assert_eq!(
            l.env,
            [
                ("PROTON_ENABLE_NVAPI".to_string(), "1".to_string()),
                ("DXVK_HUD".into(), "fps".into())
            ]
        );
        assert_eq!(
            l.wrappers,
            ["gamemoderun", "gamescope -W 1280 -H 800 -f -- %command%"]
        );
        assert_eq!(ignored, [3, 6, 7, 8]);
    }

    #[test]
    fn a_profile_reads_its_own_launch_env_file() {
        let root = std::env::temp_dir().join(format!("eidos-launch-env-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let p = Profile {
            instance_root: root.clone(),
            name: "Default".into(),
        };
        assert_eq!(p.launch_env(), LaunchEnv::default());
        std::fs::create_dir_all(p.dir()).unwrap();
        std::fs::write(p.dir().join(LAUNCH_ENV_FILE), "env=DXVK_HUD=fps\n").unwrap();
        assert_eq!(
            p.launch_env().env,
            [("DXVK_HUD".to_string(), "fps".to_string())]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
