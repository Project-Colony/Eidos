//! A mod's file list and its update links, and the choice of which file an
//! installed one should be replaced by. Port of MO2's `findUpdateChainSuccessors`
//! and `pickNewestVersion`.

use crate::{s, ModGate, Nexus};
use serde_json::Value;

/// Status of an installed file the mod page no longer lists.
pub const ARCHIVED_HIDDEN: u64 = 1000;
const REMOVED: u64 = 6;

/// Main (1), update (2), optional (3) and miscellaneous (5) files are
/// downloadable; old (4), deleted (6) and archived (7) ones are not.
pub fn is_active_file_status(status: u64) -> bool {
    matches!(status, 1 | 2 | 3 | 5)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NexusFile {
    pub file_id: u64,
    pub name: String,
    pub file_name: String,
    pub version: String,
    pub category_id: u64,
    pub size_in_bytes: u64,
    pub uploaded_timestamp: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModFiles {
    pub files: Vec<NexusFile>,
    /// `(old file id, new file id)`.
    pub updates: Vec<(u64, u64)>,
}

impl ModFiles {
    pub fn from_payload(v: &Value) -> ModFiles {
        let u = |f: &Value, k: &str| f.get(k).and_then(Value::as_u64);
        let list = |k: &str| {
            v.get(k)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let files = list("files")
            .iter()
            .filter_map(|f| {
                Some(NexusFile {
                    file_id: u(f, "file_id")?,
                    name: s(f, "name"),
                    file_name: s(f, "file_name"),
                    version: s(f, "version"),
                    category_id: u(f, "category_id").unwrap_or(0),
                    // Same fallback as `file_info`: the kilobyte figure is rounded.
                    size_in_bytes: u(f, "size_in_bytes")
                        .or_else(|| {
                            u(f, "size_kb")
                                .or_else(|| u(f, "size_in_kb"))
                                .map(|kb| kb * 1024)
                        })
                        .unwrap_or(0),
                    uploaded_timestamp: u(f, "uploaded_timestamp").unwrap_or(0),
                })
            })
            .collect();
        let updates = list("file_updates")
            .iter()
            .filter_map(|l| Some((u(l, "old_file_id")?, u(l, "new_file_id")?)))
            .collect();
        ModFiles { files, updates }
    }

    fn get(&self, id: u64) -> Option<&NexusFile> {
        self.files.iter().find(|f| f.file_id == id)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileVerdict {
    pub newest: String,
    /// The installed file's status (`ARCHIVED_HIDDEN` when unlisted).
    pub status: u64,
    pub successor: Option<u64>,
}

fn verdict_for(page_version: &str, files: &ModFiles, installed: u64) -> FileVerdict {
    // Walk the links from the installed id; `seen` stops a cycle.
    let mut seen = vec![installed];
    let mut chain = Vec::new();
    let mut at = 0;
    let mut frontier = vec![installed];
    while at < frontier.len() {
        let cur = frontier[at];
        at += 1;
        for &(old, new) in &files.updates {
            if old == cur && !seen.contains(&new) {
                seen.push(new);
                chain.push(new);
                frontier.push(new);
            }
        }
    }
    let own = files.get(installed);
    let status = own.map_or(ARCHIVED_HIDDEN, |f| f.category_id);
    let active = |id: &u64| {
        files
            .get(*id)
            .is_some_and(|f| is_active_file_status(f.category_id))
    };
    let successor = chain.iter().rev().copied().find(active).or_else(|| {
        if is_active_file_status(status) {
            return None;
        }
        chain
            .iter()
            .rev()
            .copied()
            .find(|id| files.get(*id).is_some_and(|f| f.category_id != REMOVED))
    });
    let newest = match (successor.and_then(|id| files.get(id)), own) {
        (Some(f), _) => f.version.clone(),
        (None, Some(f)) if is_active_file_status(status) => f.version.clone(),
        _ => page_version.to_string(),
    };
    FileVerdict {
        newest,
        status,
        successor,
    }
}

/// Several installed ids (one mod, several files): the first one with a
/// successor decides, else the first id.
pub fn decide_update(page_version: &str, files: &ModFiles, installed: &[u64]) -> FileVerdict {
    let mut first = None;
    for &id in installed {
        let v = verdict_for(page_version, files, id);
        if v.successor.is_some() {
            return v;
        }
        first.get_or_insert(v);
    }
    first.unwrap_or_else(|| FileVerdict {
        newest: page_version.to_string(),
        status: ARCHIVED_HIDDEN,
        successor: None,
    })
}

impl Nexus {
    /// `games/{game}/mods/{id}/files`. Refuses a closed gate like `file_info`.
    pub fn mod_files(&self, gate: &ModGate, game: &str, mod_id: u64) -> Result<ModFiles, String> {
        if let Some(why) = gate.reason() {
            return Err(why.message().to_string());
        }
        let v = self.get(&format!("games/{game}/mods/{mod_id}/files"))?;
        Ok(ModFiles::from_payload(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ModFiles {
        ModFiles::from_payload(
            &serde_json::from_str(include_str!("../tests/data/mod_files.json")).unwrap(),
        )
    }

    #[test]
    fn mod_files_parse_the_list_and_the_update_links() {
        let f = fixture();
        assert_eq!(f.files.len(), 6);
        assert_eq!(f.updates[0], (101, 102));
        assert_eq!(f.files[2].category_id, 1);
        assert_eq!(f.files[1].size_in_bytes, 2048);
    }

    #[test]
    fn the_update_chain_names_the_newest_active_successor() {
        let f = fixture();
        let v = |ids: &[u64]| decide_update("1.2", &f, ids);
        assert_eq!(
            (
                v(&[101]).newest.as_str(),
                v(&[101]).status,
                v(&[101]).successor
            ),
            ("1.2", 4, Some(103))
        );
        assert_eq!(
            (v(&[103]).newest.as_str(), v(&[103]).successor),
            ("1.2", None)
        );
        assert_eq!((v(&[201]).newest.as_str(), v(&[201]).status), ("0.9", 3));
        assert_eq!(
            (
                v(&[301]).newest.as_str(),
                v(&[301]).status,
                v(&[301]).successor
            ),
            ("1.2", 7, None)
        );
        assert_eq!(
            (v(&[999]).newest.as_str(), v(&[999]).status),
            ("1.2", ARCHIVED_HIDDEN)
        );
        assert_eq!(v(&[201, 101]).successor, Some(103));
    }

    #[test]
    fn a_cycle_in_the_update_links_terminates() {
        assert_eq!(
            decide_update("3.0", &fixture(), &[401]).successor,
            Some(402)
        );
    }
}
