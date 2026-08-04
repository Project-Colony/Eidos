use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};

use eidos_instance::ModEntry;

use super::*;
use std::fs;

/// A throwaway temp dir, cleaned up on drop (the same idiom the other crates
/// use - no external dev-dependency).
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("eidos-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn touch(&self, rel: &str) {
        let p = self.0.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, b"").unwrap();
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A wrapper DLL that belongs at the GAME ROOT lives in the mod's `Root/`, not
/// at its top level, so scanning only the top level misses exactly the mods this
/// override exists for. The concrete case: SSE Engine Fixes' preloader ships as
/// `Root/d3dx9_42.dll`, Wine implements d3dx9_42, and without the override the
/// builtin wins and the preloader never runs - with no error anywhere.
#[test]
fn a_wrapper_dll_is_found_in_a_mods_root_folder() {
    let t = Tmp::new("shadow");
    t.touch("mods/EngineFixesPreloader/Root/d3dx9_42.dll");
    t.touch("mods/ENB/d3d11.dll");
    // Not a wrapper, and buried: must not be picked up.
    t.touch("mods/SomeMod/SKSE/Plugins/whatever.dll");

    let shadows = ["d3d11", "d3dx9_42"];
    let dirs = vec![
        t.0.join("mods/EngineFixesPreloader"),
        t.0.join("mods/EngineFixesPreloader/Root"),
        t.0.join("mods/ENB"),
        t.0.join("mods/SomeMod"),
    ];
    let stems = shipped_shadow_stems(&dirs, &shadows);

    assert!(stems.contains("d3dx9_42"), "the Root/ preloader must be found");
    assert!(stems.contains("d3d11"), "a top-level wrapper is still found");
    assert_eq!(stems.len(), 2, "nothing else, and nothing from a nested dir");
}

/// The Steam Cloud sync must be idempotent (fs::copy stamps the destination
/// with a NEWER mtime, so the second run finds nothing to do), must rescue a
/// prefix save the profile never saw before overwriting its fixed name, and
/// must ignore junk.
#[test]
fn cloud_sync_is_idempotent_and_rescues_diverged_saves() {
    let t = Tmp::new("cloudsync");
    let prof = t.0.join("prof");
    let prefix = t.0.join("prefix");
    fs::create_dir_all(&prof).unwrap();
    fs::create_dir_all(&prefix).unwrap();

    // A diverged prefix quicksave the profile has no copy of (a failed-bind
    // session wrote it), OLDER than the profile's own quicksave.
    fs::write(prefix.join("quicksave.ess"), b"orphan session").unwrap();
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let f = fs::File::options().write(true).open(prefix.join("quicksave.ess")).unwrap();
    f.set_modified(old).unwrap();
    drop(f);

    fs::write(prof.join("quicksave.ess"), b"current playthrough").unwrap();
    fs::write(prof.join("quicksave.skse"), b"cosave").unwrap();
    fs::write(prof.join("steam_autocloud.vdf"), b"junk").unwrap();

    let n = sync_saves_for_cloud(&prof, &prefix).unwrap();
    assert_eq!(n, 2, ".ess + .skse synced, junk ignored");
    assert_eq!(fs::read(prefix.join("quicksave.ess")).unwrap(), b"current playthrough");
    assert!(!prefix.join("steam_autocloud.vdf").exists());

    // The orphan was rescued into the profile before the overwrite.
    let rescued: Vec<String> = fs::read_dir(&prof)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("orphan-") && n.ends_with("quicksave.ess"))
        .collect();
    assert_eq!(rescued.len(), 1, "the only copy of the orphan session must survive");

    // Second run: nothing to do. (The rescued orphan syncs up once, at most.)
    let again = sync_saves_for_cloud(&prof, &prefix).unwrap();
    assert!(again <= 1, "the sync must converge, not recopy everything ({again})");
    assert_eq!(sync_saves_for_cloud(&prof, &prefix).unwrap(), 0, "and then be a no-op");

    // A save the profile ADOPTED at seeding shares name + size with the
    // prefix original but not its mtime (the seed copy did not preserve
    // mtimes). It must NOT be "rescued" into a duplicate.
    fs::write(prefix.join("Save 12 - Old.ess"), b"identical bytes").unwrap();
    let f = fs::File::options().write(true).open(prefix.join("Save 12 - Old.ess")).unwrap();
    f.set_modified(old).unwrap();
    drop(f);
    fs::write(prof.join("Save 12 - Old.ess"), b"identical bytes").unwrap();
    sync_saves_for_cloud(&prof, &prefix).unwrap();
    let orphans = fs::read_dir(&prof)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains("Save 12"))
        .count();
    assert_eq!(orphans, 1, "the adopted twin must not spawn an orphan duplicate");

    // Quicksave rotation: the game rewrites quicksave.ess in the profile,
    // and the sync overwrites the prefix copy IT WROTE last session. Without
    // the provenance manifest that copy looked like an unknown diverged save
    // and one orphan-* file was minted per session, forever.
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(prof.join("quicksave.ess"), b"rotated - a newer, longer playthrough").unwrap();
    sync_saves_for_cloud(&prof, &prefix).unwrap();
    let orphan_count = fs::read_dir(&prof)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("orphan-"))
        .count();
    assert_eq!(
        orphan_count, 1,
        "rotating a fixed-name save must not mint orphans of our own sync copies"
    );
    assert_eq!(
        fs::read(prefix.join("quicksave.ess")).unwrap(),
        b"rotated - a newer, longer playthrough"
    );
}

// Guards FIX C1: the Overwrite layer must be the LAST (highest-priority) plugin
// source, so an ESP that lives only in Overwrite (xEdit / Bashed Patch output)
// is discovered, and an Overwrite copy wins same-name shadowing over a mod's
// copy - otherwise such plugins are silently dropped from plugins.txt.
#[test]
fn overwrite_is_the_highest_priority_plugin_source() {
    let t = Tmp::new("c1");
    let game_data = t.0.join("Data");
    fs::create_dir_all(&game_data).unwrap();

    // One enabled mod ships Patch.esp; Overwrite also has Patch.esp (a later
    // regeneration) plus a Bashed-Patch.esp that exists ONLY in Overwrite.
    let mod_dir = t.0.join("mods/AwesomeMod");
    fs::create_dir_all(&mod_dir).unwrap();
    fs::write(mod_dir.join("Patch.esp"), b"").unwrap();
    let overwrite = t.0.join("overwrite");
    t.touch("overwrite/Patch.esp");
    t.touch("overwrite/Bashed Patch.esp");

    let enabled = vec![ModEntry {
        name: "AwesomeMod".to_string(),
        enabled: true,
        path: mod_dir.clone(), unmanaged: false }];

    let sources = plugin_sources(&game_data, &enabled, &overwrite);
    // Overwrite must be the final, highest-priority source.
    assert_eq!(sources.last().unwrap().0, "overwrite");
    assert_eq!(sources.last().unwrap().1, overwrite);

    let spec = eidos_plugins::GameSpec::for_id("skyrimse").unwrap();
    let list = eidos_plugins::PluginList::discover(&sources, &spec);

    // The Overwrite-only plugin is discovered (would be dropped without C1).
    let bashed = list
        .plugins
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case("Bashed Patch.esp"))
        .expect("Overwrite-only plugin must be discovered");
    assert_eq!(bashed.origin_mod, "overwrite");

    // For the shadowed name, the Overwrite copy wins (highest priority).
    let patch = list
        .plugins
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case("Patch.esp"))
        .expect("Patch.esp must be present");
    assert_eq!(patch.origin_mod, "overwrite");
    assert!(
        patch.path.starts_with(&overwrite),
        "shadowed plugin should resolve to the Overwrite copy, got {}",
        patch.path.display()
    );
}

// Guards FIX C2: on Unix a signal-killed child reports `code() == None`, so the
// exit status must fall back to 128 + signal (not 0) - otherwise a crashed game
// would make eidos exit 0 and hide the crash. Asserts the mapping the
// `run_through_view` exit path uses.
#[test]
fn signal_death_maps_to_128_plus_signal_not_zero() {
    use std::process::ExitStatus;

    // A child killed by SIGSEGV (11): code() is None, signal() is 11.
    let killed = ExitStatus::from_raw(11);
    assert_eq!(killed.code(), None, "signal death has no exit code on Unix");
    let mapped = killed.code().unwrap_or_else(|| 128 + killed.signal().unwrap_or(1));
    assert_eq!(mapped, 139, "SIGSEGV must map to 139, never 0");
    assert_ne!(mapped, 0);

    // A normal exit(3) is unaffected: code() is Some(3).
    let normal = ExitStatus::from_raw(3 << 8);
    assert_eq!(
        normal.code().unwrap_or_else(|| 128 + normal.signal().unwrap_or(1)),
        3
    );
}

#[test]
fn a_tool_inside_a_mod_runs_from_the_merged_view() {
    // MO2's `mods\FNIS\path\exe => game\data\path\exe`. BodySlide reads
    // its slider sets relative to its own executable, and ships none of them:
    // run it from its own folder and the list is empty.
    let data = PathBuf::from("/games/skyrim/Data");
    let layers = vec![PathBuf::from("/inst/mods/BodySlide 5.8.2")];
    assert_eq!(
        virtualize_under_data(
            Path::new("/inst/mods/BodySlide 5.8.2/CalienteTools/BodySlide/BodySlide.exe"),
            &layers,
            &data,
        ),
        Some(PathBuf::from("/games/skyrim/Data/CalienteTools/BodySlide/BodySlide.exe"))
    );
}

#[test]
fn a_tool_outside_every_layer_is_left_alone() {
    let data = PathBuf::from("/games/skyrim/Data");
    let layers = vec![PathBuf::from("/inst/mods/BodySlide")];
    // xEdit in the game root, and a mod the user disabled (so it is not a
    // layer): both must keep their real path rather than be pointed at a
    // merged path that will not contain them.
    assert_eq!(
        virtualize_under_data(Path::new("/games/skyrim/SSEEdit.exe"), &layers, &data),
        None
    );
    assert_eq!(
        virtualize_under_data(Path::new("/inst/mods/Disabled/tool.exe"), &layers, &data),
        None
    );
}

#[test]
fn the_layer_root_is_what_gets_stripped_not_the_mod_name() {
    // A mod whose real content sits one level down is MOUNTED from that level,
    // so that is what has to be stripped. Stripping a fixed number of
    // components (MO2's approach) would leave the subdirectory in the path.
    let data = PathBuf::from("/games/skyrim/Data");
    let layers = vec![PathBuf::from("/inst/mods/Weird Archive/Data")];
    assert_eq!(
        virtualize_under_data(
            Path::new("/inst/mods/Weird Archive/Data/CalienteTools/BodySlide/BodySlide.exe"),
            &layers,
            &data,
        ),
        Some(PathBuf::from("/games/skyrim/Data/CalienteTools/BodySlide/BodySlide.exe"))
    );
}

#[test]
fn the_layer_itself_does_not_become_the_data_dir() {
    // Guard against the degenerate case: a path equal to the layer root has an
    // empty tail, and `data.join("")` would hand back the Data dir itself.
    let layers = vec![PathBuf::from("/inst/mods/BodySlide")];
    assert_eq!(
        virtualize_under_data(
            Path::new("/inst/mods/BodySlide"),
            &layers,
            Path::new("/games/skyrim/Data"),
        ),
        None
    );
}
