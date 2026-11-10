//! `logs/launch.ini`: the game build at the last launch and at the last launch
//! that exited without an error. `logs/` is machine-specific and `eidos pack`
//! leaves it out, so an unpacked instance never inherits another machine's build.

use std::io;

use crate::{write_atomic, Instance};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameBuild {
    pub runtime: Option<String>,
    pub build_id: Option<String>,
}

impl GameBuild {
    fn is_empty(&self) -> bool {
        self.runtime.is_none() && self.build_id.is_none()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchRecord {
    pub started: Option<u64>,
    pub last: GameBuild,
    pub last_good: Option<GameBuild>,
}

fn read_build(text: &str, section: &str) -> GameBuild {
    let get = |key| {
        eidos_ini::get_key(text, section, key)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(String::from)
    };
    GameBuild {
        runtime: get("runtime"),
        build_id: get("build_id"),
    }
}

fn push_build(out: &mut String, build: &GameBuild) {
    if let Some(v) = &build.runtime {
        out.push_str(&format!("runtime={v}\n"));
    }
    if let Some(v) = &build.build_id {
        out.push_str(&format!("build_id={v}\n"));
    }
}

impl Instance {
    fn launch_ini(&self) -> std::path::PathBuf {
        self.root.join("logs").join("launch.ini")
    }

    /// Missing or unreadable file: the default record.
    pub fn launch_record(&self) -> LaunchRecord {
        let Ok(text) = std::fs::read_to_string(self.launch_ini()) else {
            return LaunchRecord::default();
        };
        let last_good = read_build(&text, "last_good");
        LaunchRecord {
            started: eidos_ini::get_key(&text, "last", "started")
                .and_then(|v| v.trim().parse().ok()),
            last: read_build(&text, "last"),
            last_good: (!last_good.is_empty()).then_some(last_good),
        }
    }

    fn write_launch_record(&self, rec: &LaunchRecord) -> io::Result<()> {
        let mut out = String::from("[last]\n");
        if let Some(t) = rec.started {
            out.push_str(&format!("started={t}\n"));
        }
        push_build(&mut out, &rec.last);
        if let Some(good) = &rec.last_good {
            out.push_str("[last_good]\n");
            push_build(&mut out, good);
        }
        let path = self.launch_ini();
        std::fs::create_dir_all(path.parent().expect("launch.ini has a parent"))?;
        write_atomic(&path, out.as_bytes())
    }

    /// Records a launch about to start; `last_good` stays as it was.
    pub fn record_launch_start(&self, build: &GameBuild, started: u64) -> io::Result<()> {
        let old = self.launch_record();
        self.write_launch_record(&LaunchRecord {
            started: Some(started),
            last: build.clone(),
            last_good: old.last_good,
        })
    }

    /// The launch recorded last exited without an error: it becomes `last_good`.
    pub fn record_good_launch(&self) -> io::Result<()> {
        let mut rec = self.launch_record();
        rec.last_good = Some(rec.last.clone());
        self.write_launch_record(&rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst() -> (Instance, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "eidos-launch-record-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (Instance { root: dir.clone() }, dir)
    }

    fn b(runtime: &str, id: &str) -> GameBuild {
        GameBuild {
            runtime: Some(runtime.into()),
            build_id: Some(id.into()),
        }
    }

    #[test]
    fn launch_record_round_trips_and_keeps_last_good() {
        let (inst, dir) = inst();
        let (b1, b2) = (b("1.6.1170.0", "1"), b("1.6.1179.0", "2"));
        inst.record_launch_start(&b1, 100).unwrap();
        inst.record_good_launch().unwrap();
        inst.record_launch_start(&b2, 200).unwrap();
        let r = inst.launch_record();
        assert_eq!((r.started, r.last, r.last_good), (Some(200), b2, Some(b1)));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_launch_record_is_empty() {
        let (inst, dir) = inst();
        assert_eq!(inst.launch_record(), LaunchRecord::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
