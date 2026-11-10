//! `logs/sessions/`: one JSON record per run, `<started>-<pid>.json`, the newest
//! [`KEEP_SESSIONS`] kept. `logs/` is machine-specific and `eidos pack` leaves it out.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{write_atomic, Instance};

pub const KEEP_SESSIONS: usize = 50;

/// What one run started with, and how it ended.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Unix seconds.
    pub started: u64,
    /// The `eidos` process that ran it: two runs in one second differ here.
    pub pid: u32,
    pub ended: Option<u64>,
    pub eidos: String,
    pub profile: String,
    /// `None` for a game run.
    pub tool: Option<String>,
    pub game_version: Option<String>,
    /// Enabled managed mods, in mod-list order.
    pub mods: Vec<SessionMod>,
    pub plugins: Vec<(String, bool)>,
    /// Each profile INI that exists, with its CRC32.
    pub inis: Vec<(String, u32)>,
    /// `None` while running, or when the launch itself failed.
    pub exit: Option<i32>,
    pub new_saves: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionMod {
    pub name: String,
    pub version: Option<String>,
    /// `(modid, fileid)` from `meta.ini`'s `[installedFiles]`.
    pub files: Vec<(u64, u64)>,
}

/// `(started, pid)` of a `<started>-<pid>.json` record; `None` for any other file.
fn stamp(path: &Path) -> Option<(u64, u32)> {
    if path.extension()? != "json" {
        return None;
    }
    let (started, pid) = path.file_stem()?.to_str()?.split_once('-')?;
    Some((started.parse().ok()?, pid.parse().ok()?))
}

impl Instance {
    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("logs").join("sessions")
    }

    /// Record files, newest first. Sorted by number: `9-1` is older than `10-1`.
    fn session_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<_> = fs::read_dir(self.sessions_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| Some((stamp(&e.path())?, e.path())))
            .collect();
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        files.into_iter().map(|(_, p)| p).collect()
    }

    /// Writes `<started>-<pid>.json` (again, when the run ends), then drops the
    /// oldest records past [`KEEP_SESSIONS`].
    pub fn write_session(&self, r: &SessionRecord) -> io::Result<PathBuf> {
        let dir = self.sessions_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}-{}.json", r.started, r.pid));
        write_atomic(&path, &serde_json::to_vec_pretty(r)?)?;
        for old in self.session_files().into_iter().skip(KEEP_SESSIONS) {
            let _ = fs::remove_file(old);
        }
        Ok(path)
    }

    /// Every readable record, newest first.
    pub fn sessions(&self) -> Vec<(PathBuf, SessionRecord)> {
        self.session_files()
            .into_iter()
            .filter_map(|p| {
                let r = serde_json::from_slice(&fs::read(&p).ok()?).ok()?;
                Some((p, r))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(tag: &str) -> (Instance, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "eidos-sessions-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (Instance { root: dir.clone() }, dir)
    }

    #[test]
    fn sessions_keep_the_newest_fifty_and_list_newest_first() {
        let (inst, dir) = inst("keep");
        for n in 0..52u64 {
            inst.write_session(&SessionRecord {
                started: 1_700_000_000 + n,
                pid: 1,
                ..Default::default()
            })
            .unwrap();
        }
        let all = inst.sessions();
        assert_eq!(all.len(), KEEP_SESSIONS);
        assert_eq!(all[0].1.started, 1_700_000_051);
        assert_eq!(all.last().unwrap().1.started, 1_700_000_002);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn two_runs_in_one_second_write_two_records() {
        let (inst, dir) = inst("same-second");
        let a = inst
            .write_session(&SessionRecord {
                started: 5,
                pid: 10,
                ..Default::default()
            })
            .unwrap();
        let b = inst
            .write_session(&SessionRecord {
                started: 5,
                pid: 11,
                ..Default::default()
            })
            .unwrap();
        assert_ne!(a, b);
        assert_eq!(inst.sessions().len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }
}
