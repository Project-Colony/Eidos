//! xEdit QuickAutoClean: the command line that cleans one plugin unattended,
//! and a reader for the log it writes.

use std::path::Path;

use crate::runtime::to_windows_dir;

/// The log file xEdit is told to write, inside the run's scratch directory.
pub const QAC_LOG: &str = "xedit.log";

/// Arguments for `xEdit -qac` on one plugin. Backups, cache and log go under
/// `scratch` as Wine `Z:` paths; the plugin name stays one argument, spaces and all.
pub fn qac_args(plugin: &str, scratch: &Path) -> Vec<String> {
    let dir = to_windows_dir(scratch);
    vec![
        "-qac".into(),
        "-autoexit".into(),
        "-autoload".into(),
        format!(r"-B:{dir}\backups\"),
        format!(r"-C:{dir}\cache\"),
        format!(r"-R:{dir}\{QAC_LOG}"),
        plugin.into(),
    ]
}

/// What a QuickAutoClean run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QacLog {
    pub removed: u32,
    pub undeleted: u32,
    pub saved: bool,
}

/// The number after `key` in `line` (`Removed Records: 3,` -> 3).
fn count_after(line: &str, key: &str) -> u32 {
    line.split_once(key)
        .map(|(_, rest)| {
            rest.trim_start()
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Totals from an xEdit QAC log, or `None` when neither pass reported done
/// (xEdit crashed or never got that far).
pub fn parse_qac_log(text: &str) -> Option<QacLog> {
    let mut log = QacLog::default();
    let mut done = false;
    for line in text.lines() {
        if line.contains("Done: Removing") {
            done = true;
            log.removed += count_after(line, "Removed Records:");
        } else if line.contains("Done: Undeleting") {
            done = true;
            log.undeleted += count_after(line, "Undeleted Records:");
        } else if line.contains("Saving:") {
            log.saved = true;
        }
    }
    done.then_some(log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qac_args_keep_spaced_paths_and_plugin_names_whole() {
        let args = qac_args(
            "Unofficial Skyrim Special Edition Patch.esp",
            Path::new("/mnt/My Games/inst/logs/qac-1"),
        );
        assert_eq!(
            args,
            [
                "-qac",
                "-autoexit",
                "-autoload",
                r"-B:Z:\mnt\My Games\inst\logs\qac-1\backups\",
                r"-C:Z:\mnt\My Games\inst\logs\qac-1\cache\",
                r"-R:Z:\mnt\My Games\inst\logs\qac-1\xedit.log",
                "Unofficial Skyrim Special Edition Patch.esp",
            ]
        );
    }

    // Synthetic, written from xEdit's documented QAC messages; real-log check pending the owner.
    const LOG: &str = "\
[00:00] Quick Auto Clean mode
[00:03] Done: Removing \"Identical to Master\" records, Processed Records: 4210, Removed Records: 3, Elapsed Time: 00:01
[00:03] Done: Undeleting and Disabling References, Processed Records: 4207, Undeleted Records: 2, Elapsed Time: 00:00
[00:04] Done: Removing \"Identical to Master\" records, Processed Records: 4207, Removed Records: 1, Elapsed Time: 00:00
[00:04] Saving: Dirty.esp
";

    #[test]
    fn parse_qac_log_sums_both_passes_and_sees_the_save() {
        assert_eq!(
            parse_qac_log(LOG),
            Some(QacLog {
                removed: 4,
                undeleted: 2,
                saved: true
            })
        );
        let clean = "[00:01] Done: Removing \"Identical to Master\" records, Processed Records: 9, Removed Records: 0, Elapsed Time: 00:00\n";
        assert_eq!(
            parse_qac_log(clean),
            Some(QacLog {
                removed: 0,
                undeleted: 0,
                saved: false
            })
        );
        assert_eq!(parse_qac_log("xEdit crashed\n"), None);
    }
}
