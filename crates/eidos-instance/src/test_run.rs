//! A test run (A36): one game run whose writes land under `<instance>/test-run/`
//! instead of the profile, the Overwrite and the prefix, until the user keeps
//! them. The window writes the request, the next matching `eidos play` begins
//! it on a working copy of the profile, and the run's end is recorded here.

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::profile::copy_dir_recursive;
use crate::{write_atomic, Instance, Profile};

pub const TEST_RUN_DIR: &str = "test-run";
/// The working copy's profile name. It holds a `/`, so `is_mod_folder_name`
/// refuses it as a name a user could give a profile.
pub const TEST_RUN_PROFILE: &str = "test-run/profile";

/// `test-run/test-run.json`. `serde(default)` on the fields a begun run adds,
/// so a request written before them still reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRun {
    /// The real profile the run copies.
    pub profile: String,
    pub game: String,
    /// `None`: the profile's own set. `Some(list)`: exactly those managed mods
    /// (vanilla is `Some(vec![])`).
    pub mods: Option<Vec<String>>,
    pub bisect: bool,
    /// Unix seconds, as are `started` and `ended`.
    pub requested: u64,
    #[serde(default)]
    pub started: Option<u64>,
    #[serde(default)]
    pub ended: Option<u64>,
    #[serde(default)]
    pub exit: Option<i32>,
    #[serde(default)]
    pub ini_files: Vec<String>,
    /// The game's `Documents/My Games/<game>` folder, when it has one.
    #[serde(default)]
    pub docs: Option<PathBuf>,
    /// Whether the saves are a Steam Cloud target (a test run never syncs).
    #[serde(default)]
    pub cloud: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestState {
    /// Written by the window, not yet begun by a run.
    Requested,
    /// Begun, and another process holds the instance lock.
    Running,
    Ended,
    /// Begun, not ended, and nobody holds the lock: the run died.
    Interrupted,
}

impl Instance {
    pub fn test_run_dir(&self) -> PathBuf {
        self.root.join(TEST_RUN_DIR)
    }

    /// The Data upper of a test run.
    pub fn test_data_upper(&self) -> PathBuf {
        self.test_run_dir().join("overwrite")
    }

    /// The upper over the game's `Documents/My Games/<game>` folder.
    pub fn test_docs_upper(&self) -> PathBuf {
        self.test_run_dir().join("docs")
    }

    /// The mountpoint of one of the test run's one-layer unions.
    pub fn test_view(&self, name: &str) -> PathBuf {
        self.test_run_dir().join("views").join(name)
    }

    pub fn test_run_profile(&self) -> Profile {
        self.profile(TEST_RUN_PROFILE)
    }

    fn test_run_manifest(&self) -> PathBuf {
        self.test_run_dir().join("test-run.json")
    }

    /// The request or run on disk; `None` when there is none or it cannot be read.
    pub fn test_run(&self) -> Option<TestRun> {
        serde_json::from_slice(&fs::read(self.test_run_manifest()).ok()?).ok()
    }

    pub fn test_run_state(&self) -> Option<(TestRun, TestState)> {
        let run = self.test_run()?;
        let state = if run.started.is_none() {
            TestState::Requested
        } else if run.ended.is_some() {
            TestState::Ended
        } else if self.try_lock("checking a test run").is_err() {
            TestState::Running
        } else {
            TestState::Interrupted
        };
        Some((run, state))
    }

    fn write_test_run(&self, run: &TestRun) -> io::Result<()> {
        let body = serde_json::to_vec_pretty(run).map_err(io::Error::other)?;
        write_atomic(&self.test_run_manifest(), &body)
    }

    /// Read, change and write back the manifest of a begun run.
    fn update_test_run(&self, change: impl FnOnce(&mut TestRun)) -> io::Result<()> {
        let mut run = self.test_run().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no test run on this instance")
        })?;
        change(&mut run);
        self.write_test_run(&run)
    }

    /// One test run at a time per instance.
    pub fn request_test_run(&self, run: &TestRun) -> io::Result<()> {
        let _lock = self.try_lock("requesting a test run")?;
        if self.test_run_manifest().exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a test run is already waiting or not yet reviewed on this instance",
            ));
        }
        fs::create_dir_all(self.test_run_dir())?;
        self.write_test_run(run)
    }

    /// Drop a request no run has begun. A begun run holds the user's writes and
    /// is closed by its review instead.
    pub fn cancel_test_request(&self) -> io::Result<()> {
        let _lock = self.try_lock("cancelling a test run")?;
        if self.test_run().is_none_or(|r| r.started.is_some()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no test run request waits on this instance",
            ));
        }
        fs::remove_dir_all(self.test_run_dir())
    }

    /// Begin the waiting request if it is for this game and this profile: make
    /// the working copy and mark the run started. `Ok(None)` means this is a
    /// normal run. The caller holds the instance lock.
    pub fn begin_test_run(
        &self,
        real: &Profile,
        game_id: &str,
        ini_files: &[&str],
        docs: Option<PathBuf>,
        cloud: bool,
        now: u64,
    ) -> io::Result<Option<TestRun>> {
        let Some(mut run) = self.test_run() else {
            return Ok(None);
        };
        if run.started.is_some() || run.game != game_id || run.profile != real.name {
            return Ok(None);
        }
        let test = self.test_run_profile();
        fs::create_dir_all(test.dir())?;
        // Top-level files and the plugin state only: saves, runtime-root and
        // backups start empty, so every write of the run is the run's own.
        let entries = match fs::read_dir(real.dir()) {
            Ok(entries) => entries.collect::<io::Result<Vec<_>>>()?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        for e in entries {
            if e.file_type()?.is_file() {
                fs::copy(e.path(), test.dir().join(e.file_name()))?;
            }
        }
        if real.dir().join("plugins").is_dir() {
            copy_dir_recursive(&real.dir().join("plugins"), &test.dir().join("plugins"))?;
        }
        if let Some(on) = &run.mods {
            let mut list = test.modlist();
            for m in list
                .iter_mut()
                .filter(|m| !m.unmanaged && !m.is_separator())
            {
                m.enabled = on.contains(&m.name);
            }
            test.save_modlist(&list)?;
        }
        fs::create_dir_all(self.test_data_upper())?;
        fs::create_dir_all(self.test_docs_upper())?;
        run.started = Some(now);
        run.ini_files = ini_files.iter().map(|f| f.to_string()).collect();
        run.docs = docs;
        run.cloud = cloud;
        self.write_test_run(&run)?;
        Ok(Some(run))
    }

    /// Record how the run ended. The caller holds the instance lock.
    pub fn end_test_run(&self, exit: Option<i32>, now: u64) -> io::Result<()> {
        self.update_test_run(|r| {
            r.ended = Some(now);
            r.exit = exit;
        })
    }

    /// Close a run that died without recording its end, so it can be reviewed.
    pub fn close_interrupted_test_run(&self, now: u64) -> io::Result<()> {
        let _lock = self.try_lock("closing an unfinished test run")?;
        // A root-mode run leaves an INI receipt in the working copy; elsewhere a no-op.
        self.test_run_profile().recover_ini_session()?;
        self.end_test_run(None, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(name: &str) -> (Instance, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("eidos-test-run-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        (Instance::portable(root.clone()), root)
    }

    fn request(profile: &str, mods: Option<Vec<String>>) -> TestRun {
        TestRun {
            profile: profile.into(),
            game: "skyrimse".into(),
            mods,
            requested: 100,
            ..Default::default()
        }
    }

    #[test]
    fn a_request_waits_until_a_matching_game_run_begins_it() {
        let (inst, root) = inst("request");
        let real = inst.profile("Default");
        fs::create_dir_all(real.dir().join("plugins")).unwrap();
        fs::write(real.dir().join("modlist.txt"), "+B\n+A\n").unwrap();
        fs::write(real.dir().join("Skyrim.ini"), "[General]\n").unwrap();
        fs::write(real.dir().join("plugins/plugins.txt"), "*A.esp\n").unwrap();
        for d in ["saves", "runtime-root", "backups"] {
            fs::create_dir_all(real.dir().join(d)).unwrap();
            fs::write(real.dir().join(d).join("x"), "x").unwrap();
        }
        for m in ["A", "B"] {
            fs::create_dir_all(inst.mods_dir().join(m)).unwrap();
        }
        inst.request_test_run(&request("Default", None)).unwrap();
        assert_eq!(
            inst.request_test_run(&request("Default", None))
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(inst
            .begin_test_run(&inst.profile("Other"), "skyrimse", &[], None, false, 200)
            .unwrap()
            .is_none());
        assert!(inst
            .begin_test_run(&real, "fallout4", &[], None, false, 200)
            .unwrap()
            .is_none());
        let run = inst
            .begin_test_run(&real, "skyrimse", &["Skyrim.ini"], None, true, 200)
            .unwrap()
            .unwrap();
        assert_eq!((run.started, run.ended, run.cloud), (Some(200), None, true));
        assert_eq!(run.ini_files, ["Skyrim.ini"]);
        let test = inst.test_run_profile();
        assert_eq!(test.dir(), root.join("test-run/profile"));
        assert!(
            test.dir().join("Skyrim.ini").is_file()
                && test.dir().join("plugins/plugins.txt").is_file()
        );
        for d in ["saves", "runtime-root", "backups"] {
            assert!(!test.dir().join(d).join("x").exists(), "{d}");
        }
        assert_eq!(inst.profiles(), ["Default"]);
        assert!(
            inst.begin_test_run(&real, "skyrimse", &[], None, false, 300)
                .unwrap()
                .is_none(),
            "begun once"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_variant_enables_only_its_mods_in_the_working_copy() {
        let (inst, root) = inst("variant");
        let real = inst.profile("Default");
        fs::create_dir_all(real.dir()).unwrap();
        fs::write(real.dir().join("modlist.txt"), "+B\n+A\n-C\n").unwrap();
        for m in ["A", "B", "C"] {
            fs::create_dir_all(inst.mods_dir().join(m)).unwrap();
        }
        inst.request_test_run(&request("Default", Some(vec!["A".into()])))
            .unwrap();
        inst.begin_test_run(&real, "skyrimse", &[], None, false, 200)
            .unwrap()
            .unwrap();
        let on: Vec<_> = inst
            .test_run_profile()
            .modlist()
            .into_iter()
            .filter(|m| m.is_active())
            .map(|m| m.name)
            .collect();
        assert_eq!(on, ["A"]);
        assert_eq!(
            fs::read_to_string(real.dir().join("modlist.txt")).unwrap(),
            "+B\n+A\n-C\n",
            "the real list is untouched"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn state_follows_the_request_the_lock_and_the_end() {
        let (inst, root) = inst("state");
        fs::create_dir_all(inst.profile("Default").dir()).unwrap();
        assert_eq!(inst.test_run_state(), None);
        inst.request_test_run(&request("Default", None)).unwrap();
        assert_eq!(inst.test_run_state().unwrap().1, TestState::Requested);
        inst.begin_test_run(&inst.profile("Default"), "skyrimse", &[], None, false, 200)
            .unwrap();
        // another process holds the instance lock, as `eidos play` does for a whole run;
        // `--no-fork` so the process killed below is the one holding it
        let mut holder = std::process::Command::new("flock")
            .arg("--no-fork")
            .arg(root.join(".eidos.lock"))
            .args(["sleep", "30"])
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while inst.test_run_state().unwrap().1 != TestState::Running {
            assert!(
                std::time::Instant::now() < deadline,
                "the lock was never taken"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        holder.kill().unwrap();
        holder.wait().unwrap();
        assert_eq!(inst.test_run_state().unwrap().1, TestState::Interrupted);
        inst.close_interrupted_test_run(300).unwrap();
        let (run, state) = inst.test_run_state().unwrap();
        assert_eq!(
            (state, run.ended, run.exit),
            (TestState::Ended, Some(300), None)
        );
        assert_eq!(
            inst.cancel_test_request().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        let _ = fs::remove_dir_all(root);
    }
}
