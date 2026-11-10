//! `logs/sessions/`: one JSON record per run, `<started>-<pid>.json`, the newest
//! [`KEEP_SESSIONS`] kept. `logs/` is machine-specific and `eidos pack` leaves it out.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{write_atomic, Instance};

pub const KEEP_SESSIONS: usize = 50;

/// What one run started with, and how it ended.
/// `serde(default)`: a field added later must not drop every older record.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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
    /// A test run (A36): `profile` names the real profile it copied.
    pub test: bool,
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
#[serde(default)]
pub struct SessionMod {
    pub name: String,
    pub version: Option<String>,
    /// `(modid, fileid)` from `meta.ini`'s `[installedFiles]`.
    pub files: Vec<(u64, u64)>,
}

/// Entries of `old` and `new` that sit at a different rank among the entries both hold.
fn moved<'a>(old: &[&'a str], new: &[&'a str]) -> usize {
    let rank = |list: &[&'a str], other: &[&'a str]| -> Vec<&'a str> {
        list.iter().copied().filter(|n| other.contains(n)).collect()
    };
    let (a, b) = (rank(old, new), rank(new, old));
    a.iter().zip(&b).filter(|(x, y)| x != y).count()
}

impl SessionRecord {
    /// What changed from this record to `now`, one line each; empty when nothing did.
    pub fn diff(&self, now: &SessionRecord) -> Vec<String> {
        let mut out = Vec::new();
        let ver = |v: &Option<String>| v.clone().unwrap_or_else(|| "?".into());
        if self.game_version != now.game_version {
            out.push(format!(
                "game {} -> {}",
                ver(&self.game_version),
                ver(&now.game_version)
            ));
        }
        if self.eidos != now.eidos {
            out.push(format!("Eidos {} -> {}", self.eidos, now.eidos));
        }
        if self.profile != now.profile {
            out.push(format!("profile {} -> {}", self.profile, now.profile));
        }
        let old_mod = |n: &str| self.mods.iter().find(|m| m.name == n);
        for m in now.mods.iter().filter(|m| old_mod(&m.name).is_none()) {
            out.push(match &m.version {
                Some(v) => format!("+ mod {} {v}", m.name),
                None => format!("+ mod {}", m.name),
            });
        }
        for m in self
            .mods
            .iter()
            .filter(|m| !now.mods.iter().any(|n| n.name == m.name))
        {
            out.push(format!("- mod {}", m.name));
        }
        for m in &now.mods {
            let Some(o) = old_mod(&m.name) else { continue };
            if o.version != m.version {
                out.push(format!(
                    "~ mod {} {} -> {}",
                    m.name,
                    ver(&o.version),
                    ver(&m.version)
                ));
            } else if o.files != m.files {
                let ids = |f: &[(u64, u64)]| {
                    f.iter()
                        .map(|(a, b)| format!("{a}:{b}"))
                        .collect::<Vec<_>>()
                        .join(",")
                };
                out.push(format!(
                    "~ mod {} files {} -> {}",
                    m.name,
                    ids(&o.files),
                    ids(&m.files)
                ));
            }
        }
        fn mod_names(mods: &[SessionMod]) -> Vec<&str> {
            mods.iter().map(|m| m.name.as_str()).collect()
        }
        let n = moved(&mod_names(&self.mods), &mod_names(&now.mods));
        if n > 0 {
            out.push(format!("mod order: {n} mod(s) moved"));
        }
        let old_plugin = |n: &str| self.plugins.iter().find(|p| p.0 == n);
        for (p, _) in now.plugins.iter().filter(|p| old_plugin(&p.0).is_none()) {
            out.push(format!("+ plugin {p}"));
        }
        for (p, _) in self
            .plugins
            .iter()
            .filter(|p| !now.plugins.iter().any(|n| n.0 == p.0))
        {
            out.push(format!("- plugin {p}"));
        }
        for (p, on) in &now.plugins {
            if old_plugin(p).is_some_and(|o| o.1 != *on) {
                out.push(format!(
                    "plugin {p} {}",
                    if *on { "enabled" } else { "disabled" }
                ));
            }
        }
        fn plugin_names(l: &[(String, bool)]) -> Vec<&str> {
            l.iter().map(|p| p.0.as_str()).collect()
        }
        let n = moved(&plugin_names(&self.plugins), &plugin_names(&now.plugins));
        if n > 0 {
            out.push(format!("load order: {n} plugin(s) moved"));
        }
        for (ini, crc) in &now.inis {
            if self.inis.iter().any(|o| o.0 == *ini && o.1 != *crc) {
                out.push(format!("~ {ini} changed"));
            }
        }
        out
    }
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

    #[test]
    fn diff_names_what_changed_between_two_runs() {
        let m = |n: &str, v: &str| SessionMod {
            name: n.into(),
            version: Some(v.into()),
            files: vec![],
        };
        let old = SessionRecord {
            game_version: Some("1.6.1170.0".into()),
            profile: "Default".into(),
            mods: vec![m("A", "1.0"), m("B", "2.0"), m("C", "1.0")],
            plugins: vec![("A.esp".into(), true), ("B.esp".into(), true)],
            inis: vec![("Skyrim.ini".into(), 1)],
            ..Default::default()
        };
        let now = SessionRecord {
            game_version: Some("1.6.1179.0".into()),
            profile: "Default".into(),
            mods: vec![m("B", "2.1"), m("A", "1.0"), m("D", "0.1")],
            plugins: vec![("A.esp".into(), false), ("D.esp".into(), true)],
            inis: vec![("Skyrim.ini".into(), 2)],
            ..Default::default()
        };
        assert_eq!(
            old.diff(&now),
            [
                "game 1.6.1170.0 -> 1.6.1179.0",
                "+ mod D 0.1",
                "- mod C",
                "~ mod B 2.0 -> 2.1",
                "mod order: 2 mod(s) moved",
                "+ plugin D.esp",
                "- plugin B.esp",
                "plugin A.esp disabled",
                "~ Skyrim.ini changed",
            ]
        );
        assert!(now.diff(&now).is_empty());
    }

    #[test]
    fn a_record_written_before_a_field_existed_still_reads() {
        let r: SessionRecord =
            serde_json::from_str(r#"{"started":5,"pid":1,"mods":[{"name":"A"}]}"#).unwrap();
        assert_eq!((r.started, r.mods[0].name.as_str()), (5, "A"));
    }

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
