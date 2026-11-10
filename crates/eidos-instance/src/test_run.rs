//! A test run (A36): one game run whose writes land under `<instance>/test-run/`
//! instead of the profile, the Overwrite and the prefix, until the user keeps
//! them. The window writes the request, the next matching `eidos play` begins
//! it on a working copy of the profile, and the run's end is recorded here.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use eidos_core::{OPAQUE_MARKER, WHITEOUT_PREFIX};
use serde::{Deserialize, Serialize};

use crate::profile::{copy_atomic, copy_dir_recursive};
use crate::{
    merge_output_tree, move_one_into, write_atomic, Instance, Profile, ROOT_CONTROL_FILES,
};

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

/// The review's groups: each is kept or thrown away as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestGroup {
    Saves,
    Inis,
    Plugins,
    /// The Data and Root uppers, the profile's control files left out.
    Output,
    /// The My Games upper minus its INIs and its `Saves`.
    Logs,
}

impl TestGroup {
    pub const ALL: [TestGroup; 5] = [
        TestGroup::Saves,
        TestGroup::Inis,
        TestGroup::Plugins,
        TestGroup::Output,
        TestGroup::Logs,
    ];
}

/// What a test run wrote, by group. Every list is sorted and holds `/`-joined
/// paths relative to its group's folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestRunWrites {
    pub saves: Vec<String>,
    /// How many of `saves` have a save of the same name in the profile.
    pub saves_replacing: usize,
    /// Saves the game deleted; keeping the saves never applies a deletion.
    pub saves_deleted: usize,
    pub inis: Vec<String>,
    pub plugins: Vec<String>,
    /// False after a variant run, whose plugin list differs by construction.
    pub plugins_offered: bool,
    /// `Data/<path>` and `Root/<path>`.
    pub output: Vec<String>,
    /// Overwrite files the game deleted, a cleared folder's included; keeping
    /// the output deletes them.
    pub output_deleted: usize,
    pub logs: Vec<String>,
}

impl TestRunWrites {
    /// How many files keeping `g` would add, replace or delete. Output counts
    /// its deletions, so a run that only deleted is not closed unasked.
    pub fn files(&self, g: TestGroup) -> usize {
        match g {
            TestGroup::Saves => self.saves.len(),
            TestGroup::Inis => self.inis.len(),
            TestGroup::Plugins => self.plugins.len(),
            TestGroup::Output => self.output.len() + self.output_deleted,
            TestGroup::Logs => self.logs.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        TestGroup::ALL.iter().all(|g| self.files(*g) == 0)
    }
}

/// Every regular file under `dir` (none when it is missing), as sorted
/// `/`-joined paths relative to it. Symlinks are left out.
// shortcut: a name that is not UTF-8 is listed lossily and its Keep fails, upgrade if a game writes one.
fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            match e.file_type() {
                Ok(t) if t.is_dir() => walk(&e.path(), &path, out),
                Ok(t) if t.is_file() => out.push(path),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

fn leaf(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

fn is_whiteout(rel: &str) -> bool {
    leaf(rel).starts_with(WHITEOUT_PREFIX)
}

/// A union's own bookkeeping: a whiteout or an opacity marker.
fn is_marker(rel: &str) -> bool {
    is_whiteout(rel) || leaf(rel).starts_with(OPAQUE_MARKER)
}

/// How many files merging `upper` (its `files_under`) into `shared` deletes,
/// matched without case as the merge does: a whiteout's target, or every file
/// of the folder it names, and the old files of a folder made opaque, except
/// `kept` ones and the paths the upper writes back. A marker that deletes none
/// of them still counts one, since it hides a mod's files from then on.
fn deleted_by(upper: &[String], shared: &Path, kept: impl Fn(&str) -> bool) -> usize {
    let markers: Vec<&String> = upper.iter().filter(|f| is_marker(f)).collect();
    if markers.is_empty() {
        return 0;
    }
    let written: HashSet<String> = upper
        .iter()
        .filter(|f| !is_marker(f))
        .map(|f| f.to_ascii_lowercase())
        .collect();
    let old: Vec<String> = files_under(shared)
        .into_iter()
        .filter(|f| !is_marker(f) && !kept(f))
        .map(|f| f.to_ascii_lowercase())
        .filter(|f| !written.contains(f))
        .collect();
    let mut gone = HashSet::new();
    for m in markers {
        let m = m.to_ascii_lowercase();
        let (dir, name) = m.rsplit_once('/').unwrap_or(("", &m));
        // A whiteout deletes its target; an opacity marker empties its folder.
        let target = match name.strip_prefix(WHITEOUT_PREFIX) {
            Some(t) if dir.is_empty() => t.to_string(),
            Some(t) => format!("{dir}/{t}"),
            None => dir.to_string(),
        };
        let mut hit = old
            .iter()
            .filter(|f| {
                target.is_empty()
                    || **f == target
                    || f.strip_prefix(&target).is_some_and(|r| r.starts_with('/'))
            })
            .peekable();
        if hit.peek().is_none() {
            gone.insert(m);
        }
        gone.extend(hit.cloned());
    }
    gone.len()
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

    fn require_test_run(&self) -> io::Result<TestRun> {
        self.test_run()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no test run on this instance"))
    }

    /// Read, change and write back the manifest of a begun run.
    fn update_test_run(&self, change: impl FnOnce(&mut TestRun)) -> io::Result<()> {
        let mut run = self.require_test_run()?;
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

    /// What the test run on disk wrote; empty when there is none.
    pub fn test_run_writes(&self) -> TestRunWrites {
        self.test_run()
            .map(|run| self.writes_of(&run))
            .unwrap_or_default()
    }

    fn writes_of(&self, run: &TestRun) -> TestRunWrites {
        let test = self.test_run_profile();
        let real = self.profile(&run.profile);
        let mut w = TestRunWrites {
            plugins_offered: run.mods.is_none(),
            ..Default::default()
        };
        for f in files_under(&test.saves_dir()) {
            if is_whiteout(&f) {
                w.saves_deleted += 1;
            } else if !is_marker(&f) {
                w.saves_replacing += usize::from(real.saves_dir().join(&f).exists());
                w.saves.push(f);
            }
        }
        // Morrowind.ini and plugins.txt live in the plugin state: that group has them.
        let test_plugins = test.plugins_state_dir();
        let differs = |copy: &Path, real: &Path| {
            fs::read(copy).is_ok_and(|bytes| fs::read(real).ok() != Some(bytes))
        };
        w.inis = run
            .ini_files
            .iter()
            .filter(|f| {
                let copy = test.ini_path(f);
                !copy.starts_with(&test_plugins) && differs(&copy, &real.ini_path(f))
            })
            .cloned()
            .collect();
        w.inis.sort();
        if w.plugins_offered {
            let real_plugins = real.plugins_state_dir();
            w.plugins = files_under(&test_plugins)
                .into_iter()
                .filter(|f| {
                    !f.contains('/') && differs(&test_plugins.join(f), &real_plugins.join(f))
                })
                .collect();
        }
        let mut tally = |prefix: &str, files: &[String]| {
            for f in files.iter().filter(|f| !is_marker(f)) {
                w.output.push(format!("{prefix}/{f}"));
            }
        };
        let data = files_under(&self.test_data_upper());
        tally("Data", &data);
        let control = |f: &str| {
            let name = f.strip_prefix(WHITEOUT_PREFIX).unwrap_or(f);
            !f.contains('/')
                && ROOT_CONTROL_FILES
                    .iter()
                    .any(|c| name.eq_ignore_ascii_case(c))
        };
        let root: Vec<_> = files_under(&test.dir().join("runtime-root"))
            .into_iter()
            .filter(|f| !control(f))
            .collect();
        tally("Root", &root);
        w.output_deleted = deleted_by(&data, &self.overwrite_dir(), |_| false)
            + deleted_by(&root, &self.root_overwrite_dir(), control);
        w.logs = files_under(&self.test_docs_upper())
            .into_iter()
            .filter(|f| {
                let top = f.split('/').next().unwrap_or(f);
                let ini =
                    !f.contains('/') && run.ini_files.iter().any(|i| i.eq_ignore_ascii_case(f));
                !is_marker(f) && !ini && !top.eq_ignore_ascii_case("Saves")
            })
            .collect();
        w
    }

    /// Move the kept groups into the profile, the Overwrite and the My Games
    /// folder, then remove `test-run/`, which throws the other groups away. On
    /// an error `test-run/` stays with what was not moved, and a retry moves the
    /// rest.
    pub fn finish_test_run(&self, keep: &[TestGroup]) -> io::Result<()> {
        let _lock = self.try_lock("closing a test run")?;
        let run = self.require_test_run()?;
        let w = self.writes_of(&run);
        let test = self.test_run_profile();
        let real = self.profile(&run.profile);
        for g in TestGroup::ALL.into_iter().filter(|g| keep.contains(g)) {
            match g {
                // A whiteout is never applied: a test run does not delete a save.
                TestGroup::Saves => {
                    let (from, to) = (test.saves_dir(), real.saves_dir());
                    fs::create_dir_all(&to)?;
                    for f in &w.saves {
                        move_one_into(&to, &from.join(f), &to.join(f))?;
                    }
                }
                TestGroup::Inis => {
                    for f in &w.inis {
                        copy_atomic(&test.ini_path(f), &real.ini_path(f))?;
                    }
                }
                TestGroup::Plugins => {
                    let (from, to) = (test.plugins_state_dir(), real.plugins_state_dir());
                    for f in &w.plugins {
                        copy_atomic(&from.join(f), &to.join(f))?;
                    }
                }
                TestGroup::Output => {
                    merge_output_tree(&self.test_data_upper(), &self.overwrite_dir(), &[])?;
                    test.merge_runtime_root_outputs(
                        &self.root_overwrite_dir(),
                        ROOT_CONTROL_FILES,
                    )?;
                }
                TestGroup::Logs => {
                    if let Some(docs) = &run.docs {
                        fs::create_dir_all(docs)?;
                        for f in &w.logs {
                            move_one_into(docs, &self.test_docs_upper().join(f), &docs.join(f))?;
                        }
                    }
                }
            }
        }
        fs::remove_dir_all(self.test_run_dir())
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

    fn put(path: PathBuf, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    /// A begun and ended run of `Default` that wrote into every group.
    fn ended(name: &str, mods: Option<Vec<String>>) -> (Instance, Profile, PathBuf, PathBuf) {
        let (inst, root) = inst(name);
        let real = inst.profile("Default");
        put(real.dir().join("Skyrim.ini"), "real");
        put(real.dir().join("plugins/plugins.txt"), "*A.esp\n");
        put(real.saves_dir().join("quick.ess"), "real");
        put(real.saves_dir().join("old.ess"), "old");
        put(real.dir().join("modlist.txt"), "+A\n");
        fs::create_dir_all(inst.mods_dir().join("A")).unwrap();
        let docs = root.join("docs-real");
        inst.request_test_run(&request("Default", mods)).unwrap();
        inst.begin_test_run(
            &real,
            "skyrimse",
            &["Skyrim.ini"],
            Some(docs.clone()),
            false,
            200,
        )
        .unwrap()
        .unwrap();
        let test = inst.test_run_profile();
        put(test.saves_dir().join("new.ess"), "new");
        put(test.saves_dir().join("quick.ess"), "test");
        put(test.saves_dir().join(".eidoswh.old.ess"), "");
        put(test.dir().join("Skyrim.ini"), "test");
        put(test.dir().join("plugins/plugins.txt"), "*A.esp\n*B.esp\n");
        put(inst.test_data_upper().join("SKSE/x.log"), "log");
        put(inst.test_data_upper().join(".eidoswh.gone.txt"), "");
        put(inst.overwrite_dir().join("gone.txt"), "gone");
        // The game cleared a folder and recreated it: opacity, not whiteouts.
        put(inst.overwrite_dir().join("cache/old.txt"), "old");
        put(inst.test_data_upper().join("cache/.eidoswh_opaque"), "");
        put(inst.test_data_upper().join("cache/new.txt"), "new");
        // A deleted folder is one whiteout for every file under it.
        put(inst.overwrite_dir().join("Foo/a.txt"), "a");
        put(inst.overwrite_dir().join("Foo/b.txt"), "b");
        put(inst.test_data_upper().join(".eidoswh.foo"), "");
        put(inst.root_overwrite_dir().join("Logs/r.txt"), "r");
        put(test.dir().join("runtime-root/Logs/.eidoswh_opaque"), "");
        put(test.dir().join("runtime-root/Root.txt"), "root");
        put(test.dir().join("runtime-root/plugins.txt"), "*A.esp\n");
        put(inst.test_docs_upper().join("SKSE/skse64.log"), "log");
        put(inst.test_docs_upper().join("Skyrim.ini"), "test");
        fs::create_dir_all(inst.test_docs_upper().join("Saves")).unwrap();
        inst.end_test_run(Some(0), 300).unwrap();
        (inst, real, docs, root)
    }

    fn ended_run(name: &str) -> (Instance, Profile, PathBuf, PathBuf) {
        ended(name, None)
    }

    fn ended_variant_run(name: &str) -> (Instance, Profile, PathBuf, PathBuf) {
        ended(name, Some(vec!["A".into()]))
    }

    #[test]
    fn writes_list_each_group_and_keep_moves_only_the_kept_ones() {
        let (inst, real, docs_real, root) = ended_run("keep");
        let w = inst.test_run_writes();
        assert_eq!(w.saves, ["new.ess", "quick.ess"]);
        assert_eq!((w.saves_replacing, w.saves_deleted), (1, 1));
        assert_eq!(w.inis, ["Skyrim.ini"]);
        assert_eq!(
            (w.plugins.as_slice(), w.plugins_offered),
            (["plugins.txt".to_string()].as_slice(), true)
        );
        assert_eq!(
            w.output,
            ["Data/SKSE/x.log", "Data/cache/new.txt", "Root/Root.txt"]
        );
        // gone.txt, cache/old.txt, Foo/a.txt, Foo/b.txt, Logs/r.txt.
        assert_eq!(w.output_deleted, 5);
        assert_eq!(w.logs, ["SKSE/skse64.log"]);
        inst.finish_test_run(&[TestGroup::Saves, TestGroup::Output])
            .unwrap();
        assert!(real.saves_dir().join("new.ess").is_file());
        assert_eq!(
            fs::read(real.saves_dir().join("quick.ess")).unwrap(),
            b"test"
        );
        assert!(
            real.saves_dir().join("old.ess").is_file(),
            "a save is never deleted"
        );
        assert_eq!(fs::read(real.ini_path("Skyrim.ini")).unwrap(), b"real");
        assert_eq!(
            fs::read(real.plugins_state_dir().join("plugins.txt")).unwrap(),
            b"*A.esp\n"
        );
        assert!(inst.overwrite_dir().join("SKSE/x.log").is_file());
        assert!(!inst.overwrite_dir().join("gone.txt").exists());
        assert!(!inst.overwrite_dir().join("cache/old.txt").exists());
        assert!(inst.overwrite_dir().join("cache/new.txt").is_file());
        assert!(!inst.overwrite_dir().join("Foo").exists());
        assert!(!inst.root_overwrite_dir().join("Logs/r.txt").exists());
        assert!(inst.root_overwrite_dir().join("Root.txt").is_file());
        assert!(!inst.root_overwrite_dir().join("plugins.txt").exists());
        assert!(!docs_real.join("SKSE/skse64.log").exists());
        assert!(!inst.test_run_dir().exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_state_is_not_offered_after_a_variant_run() {
        let (inst, _real, _docs, root) = ended_variant_run("variant-writes");
        let w = inst.test_run_writes();
        assert!(!w.plugins_offered && w.plugins.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_blocked_keep_leaves_the_test_run_in_place() {
        let (inst, real, _docs, root) = ended_run("blocked");
        fs::remove_dir_all(real.saves_dir()).unwrap();
        fs::write(real.saves_dir(), b"a file where the saves folder should be").unwrap();
        assert!(inst.finish_test_run(&[TestGroup::Saves]).is_err());
        assert!(inst
            .test_run_profile()
            .saves_dir()
            .join("new.ess")
            .is_file());
        assert!(inst.test_run().is_some());
        fs::remove_file(real.saves_dir()).unwrap();
        fs::create_dir_all(real.saves_dir()).unwrap();
        inst.finish_test_run(&[TestGroup::Saves]).unwrap();
        assert!(real.saves_dir().join("new.ess").is_file() && !inst.test_run_dir().exists());
        let _ = fs::remove_dir_all(root);
    }
}
