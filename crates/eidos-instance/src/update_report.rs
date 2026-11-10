//! `logs/updates/<mod>.json`: what the last Replace changed in a mod folder.
//! It sits outside the mod so the mod's own files stay as the archive made them,
//! and under `logs/`, which `eidos pack` leaves out.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::write_atomic;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateReport {
    /// Unix seconds.
    pub when: u64,
    pub old_version: String,
    pub new_version: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub resized: Vec<String>,
    pub plugins_added: Vec<String>,
    pub plugins_removed: Vec<String>,
    pub masters_added: Vec<String>,
    pub masters_removed: Vec<String>,
    /// (plugin, kind before, kind after), kinds as `ESP`, `ESM`, `ESL`, `ESH`, `ESM+ESL`.
    pub kinds_changed: Vec<(String, String, String)>,
}

impl UpdateReport {
    /// Nothing changed in the files or plugins (the time and versions do not count).
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.resized.is_empty()
            && self.plugins_added.is_empty()
            && self.plugins_removed.is_empty()
            && self.masters_added.is_empty()
            && self.masters_removed.is_empty()
            && self.kinds_changed.is_empty()
    }

    /// One English line for the CLI.
    pub fn summary(&self) -> String {
        let mut out = format!(
            "files +{} -{} ~{}",
            self.added.len(),
            self.removed.len(),
            self.resized.len()
        );
        for (label, names) in [
            ("plugins added", &self.plugins_added),
            ("plugins removed", &self.plugins_removed),
            ("new masters", &self.masters_added),
            ("masters no longer needed", &self.masters_removed),
        ] {
            if !names.is_empty() {
                out.push_str(&format!("; {label}: {}", names.join(", ")));
            }
        }
        for (plugin, before, after) in &self.kinds_changed {
            out.push_str(&format!("; {plugin} {before} to {after}"));
        }
        out
    }
}

pub fn update_report_path(root: &Path, folder: &str) -> PathBuf {
    root.join("logs")
        .join("updates")
        .join(format!("{folder}.json"))
}

pub fn save_update_report(root: &Path, folder: &str, report: &UpdateReport) -> io::Result<()> {
    let path = update_report_path(root, folder);
    std::fs::create_dir_all(path.parent().expect("the report path has a parent"))?;
    let json = serde_json::to_string_pretty(report).map_err(io::Error::other)?;
    write_atomic(&path, json.as_bytes())
}

/// A missing or unreadable report is `None`.
pub fn load_update_report(root: &Path, folder: &str) -> Option<UpdateReport> {
    let text = std::fs::read_to_string(update_report_path(root, folder)).ok()?;
    serde_json::from_str(&text).ok()
}

/// A missing report is fine.
pub fn remove_update_report(root: &Path, folder: &str) {
    let _ = std::fs::remove_file(update_report_path(root, folder));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_round_trips_under_logs_and_reads_in_one_line() {
        let root = std::env::temp_dir().join(format!("eidos-update-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let r = UpdateReport {
            added: vec!["Extra.esm".into(), "sound/new.wav".into()],
            removed: vec!["Old.esp".into()],
            plugins_added: vec!["Extra.esm".into()],
            masters_added: vec!["Dawnguard.esm".into()],
            kinds_changed: vec![("Patch.esp".into(), "ESP".into(), "ESL".into())],
            ..Default::default()
        };
        save_update_report(&root, "Old", &r).unwrap();
        assert_eq!(
            update_report_path(&root, "Old"),
            root.join("logs/updates/Old.json")
        );
        assert_eq!(load_update_report(&root, "Old"), Some(r.clone()));
        assert_eq!(r.summary(), "files +2 -1 ~0; plugins added: Extra.esm; new masters: Dawnguard.esm; Patch.esp ESP to ESL");
        assert_eq!(UpdateReport::default().summary(), "files +0 -0 ~0");
        assert!(UpdateReport::default().is_empty() && !r.is_empty());
        std::fs::write(update_report_path(&root, "Bad"), "not json").unwrap();
        assert_eq!(load_update_report(&root, "Bad"), None);
        remove_update_report(&root, "Old");
        remove_update_report(&root, "Old");
        assert_eq!(load_update_report(&root, "Old"), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
