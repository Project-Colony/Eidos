//! The `eidos play` side of a test run (A36): begin the instance's waiting
//! request, point every writable mount at `<instance>/test-run/`, and say where
//! the run's files wait.

use std::path::{Path, PathBuf};

use eidos_games::{DetectedGame, GameDef};
use eidos_instance::{Instance, Profile, TestGroup, TestRun, TestRunWrites};

/// Prepare the instance's waiting test run when it is for this game and `real`;
/// the launch begins it only once nothing can refuse the game any more.
/// `Ok(None)` is a normal run; a request for another game or profile keeps
/// waiting, and the run says so.
pub(crate) fn take(
    inst: &Instance,
    game_id: &str,
    game: &DetectedGame,
    real: &Profile,
) -> std::io::Result<Option<TestRun>> {
    let docs = game
        .prefix()
        .zip(game.plugin_spec())
        .map(|(prefix, spec)| eidos_plugins::documents_my_games_dir(&prefix, &spec));
    let cloud = game
        .saves_path()
        .is_some_and(|p| crate::launch::steam_cloud_save_target(game, &p));
    let run = inst.prepare_test_run(
        real,
        game_id,
        eidos_gamefeatures::ini_files_for(game_id),
        docs,
        cloud,
    )?;
    if run.is_none() {
        if let Some(waiting) = inst.test_run().filter(|r| r.started.is_none()) {
            eidos_log::info!(
                "eidos: a test run is waiting for profile '{}' of {}; this run uses '{}', so it is a normal run",
                waiting.profile,
                waiting.game,
                real.name
            );
        }
    }
    Ok(run)
}

/// Where a test run's mounts read and write: every writable path is under
/// `<instance>/test-run/`, and the real Overwrite is read-only below it.
pub(crate) struct TestMounts {
    pub overwrite: PathBuf,
    pub readonly_overwrite: PathBuf,
    pub root_overwrite: PathBuf,
    pub root_readonly_overwrite: PathBuf,
    /// `(lower, upper, view)` one-layer unions.
    pub union_views: Vec<(PathBuf, PathBuf, PathBuf)>,
    pub binds: Vec<(PathBuf, PathBuf)>,
}

/// The mounts of a test run. `docs` is the game's My Games folder (None in
/// root mode); `saves` is the `(profile_saves, target)` bind a normal run uses.
pub(crate) fn test_mounts(
    inst: &Instance,
    docs: Option<&Path>,
    saves: Option<&(PathBuf, PathBuf)>,
) -> TestMounts {
    let mut m = TestMounts {
        overwrite: inst.test_data_upper(),
        readonly_overwrite: inst.overwrite_dir(),
        root_overwrite: inst.test_run_profile().dir().join("runtime-root"),
        root_readonly_overwrite: inst.root_overwrite_dir(),
        union_views: Vec::new(),
        binds: Vec::new(),
    };
    // My Games first: the saves folder sits inside it, so the saves bind has to
    // land on top of the My Games view.
    if let Some(docs) = docs {
        let view = inst.test_view("docs");
        m.union_views
            .push((docs.to_path_buf(), inst.test_docs_upper(), view.clone()));
        m.binds.push((view, docs.to_path_buf()));
    }
    if let Some((profile_saves, target)) = saves {
        let view = inst.test_view("saves");
        m.union_views.push((
            profile_saves.clone(),
            inst.test_run_profile().saves_dir(),
            view.clone(),
        ));
        m.binds.push((view, target.clone()));
    }
    m
}

/// Point the command at the loader the test run's Root layers provide, or at
/// the game binary when they provide none: a variant run can leave the script
/// extender's mod off, and its loader then does not exist.
pub(crate) fn retarget_loader(
    def: &GameDef,
    install: &Path,
    files_root: &Path,
    root_layers: &[PathBuf],
    root_overwrite: &Path,
    command: &mut [String],
) {
    let Some(se) = def.script_extender else {
        return;
    };
    let resolve = |exe| {
        eidos_instance::root_executable(install, files_root, root_layers, root_overwrite, exe)
    };
    let Some(target) = resolve(se.loader).or_else(|| resolve(def.game_binary)) else {
        return;
    };
    for argument in command.iter_mut() {
        if Path::new(argument).file_name().is_some_and(|n| {
            n.eq_ignore_ascii_case(se.launcher) || n.eq_ignore_ascii_case(se.loader)
        }) {
            *argument = target.to_string_lossy().into_owned();
        }
    }
}

/// The line `eidos play` prints when a test run ends: a run started from Steam
/// may have no window open to show the review.
pub(crate) fn summary_line(w: &TestRunWrites, dir: &Path) -> String {
    let [saves, inis, plugins, output, logs] = TestGroup::ALL.map(|g| w.files(g));
    format!(
        "eidos: test run ended: {saves} save(s), {inis} INI change(s), {plugins} plugin file(s), \
         {output} output file(s) and {logs} log file(s) wait in {}. Open Eidos to keep or throw \
         them away.",
        dir.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_mounts_put_every_write_under_test_run() {
        let inst = Instance::portable(
            std::env::temp_dir().join(format!("eidos-tm-{}", std::process::id())),
        );
        let docs = PathBuf::from("/prefix/My Games/Skyrim Special Edition");
        let saves = (inst.profile("Default").saves_dir(), docs.join("Saves"));
        let m = test_mounts(&inst, Some(&docs), Some(&saves));
        let under = inst.test_run_dir();
        for p in [&m.overwrite, &m.root_overwrite] {
            assert!(p.starts_with(&under), "{p:?}");
        }
        assert_eq!(
            (m.readonly_overwrite, m.root_readonly_overwrite),
            (inst.overwrite_dir(), inst.root_overwrite_dir())
        );
        assert_eq!(
            m.union_views,
            [
                (docs.clone(), inst.test_docs_upper(), inst.test_view("docs")),
                (
                    saves.0.clone(),
                    inst.test_run_profile().saves_dir(),
                    inst.test_view("saves")
                ),
            ]
        );
        assert_eq!(
            m.binds,
            [
                (inst.test_view("docs"), docs.clone()),
                (inst.test_view("saves"), saves.1.clone())
            ]
        );
        assert!(test_mounts(&inst, None, None).union_views.is_empty());
    }

    #[test]
    fn retarget_loader_falls_back_to_the_game_binary() {
        let t = std::env::temp_dir().join(format!("eidos-retarget-{}", std::process::id()));
        let (install, skse) = (t.join("game"), t.join("mods/SKSE/Root"));
        for (dir, file) in [(&install, "SkyrimSE.exe"), (&skse, "skse64_loader.exe")] {
            fs::create_dir_all(dir).unwrap();
            fs::write(dir.join(file), b"MZ").unwrap();
        }
        let def = eidos_games::GameDef::for_id("skyrimse").unwrap();
        let mut cmd = vec![
            "proton".to_string(),
            "/x/SkyrimSELauncher.exe".into(),
            "-skipintro".into(),
        ];
        retarget_loader(
            def,
            &install,
            &install,
            std::slice::from_ref(&skse),
            &t.join("ow-root"),
            &mut cmd,
        );
        assert_eq!(cmd[1], install.join("skse64_loader.exe").to_string_lossy());
        retarget_loader(def, &install, &install, &[], &t.join("ow-root"), &mut cmd);
        assert_eq!(cmd[1], install.join("SkyrimSE.exe").to_string_lossy());
        assert_eq!(cmd[2], "-skipintro");
        // A list's Stock Game holds the loader and no Root layer does: it resolves
        // in the copy and runs from the install path, where the root union shows it.
        let copy = t.join("Stock Game");
        fs::create_dir_all(&copy).unwrap();
        fs::write(copy.join("skse64_loader.exe"), b"MZ").unwrap();
        cmd[1] = "/x/SkyrimSELauncher.exe".into();
        retarget_loader(def, &install, &copy, &[], &t.join("ow-root"), &mut cmd);
        assert_eq!(cmd[1], install.join("skse64_loader.exe").to_string_lossy());
        let _ = fs::remove_dir_all(t);
    }
}
