use eidos_plugins::{canonical_path, GameSpec, PluginList};
use std::fs;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("eidos-refute-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn cc_disable_round_trip() {
    let root = tmp("root");
    let data = root.join("Data");
    fs::create_dir_all(&data).unwrap();
    // .ccc names two Creations, like the user's Skyrim.ccc
    fs::write(root.join("Skyrim.ccc"), "ccBGSSSE001-Fish.esm\nccBGSSSE002-ExoticArrows.esl\n").unwrap();
    for f in ["Skyrim.esm", "ccBGSSSE001-Fish.esm", "ccBGSSSE002-ExoticArrows.esl", "MyMod.esp"] {
        fs::write(data.join(f), b"").unwrap();
    }
    let spec = GameSpec::for_id("skyrimse").unwrap();
    let state = tmp("state");

    let sources = vec![(String::new(), data.clone())];
    let mut list = PluginList::discover(&sources, &spec);
    list.apply_prefix_state(&state, &spec);
    list.refresh(&spec);
    println!("implicit set = {:?}", list.implicit);
    let fish_before = list
        .plugins
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case("ccBGSSSE001-Fish.esm"))
        .unwrap()
        .clone();
    println!("fish before: enabled={} force_disabled={}", fish_before.enabled, fish_before.force_disabled);

    // Establish a baseline file pair, like a prior session.
    list.write_load_order(&state, &spec).unwrap();
    let plugins_txt_before = fs::read(canonical_path(&state, "plugins.txt")).unwrap();

    // === the GUI's TogglePlugin path ===
    list.set_enabled("ccBGSSSE001-Fish.esm", false);
    list.refresh(&spec);
    let listed = list.write_load_order(&state, &spec).unwrap();
    println!("write_load_order returned Ok({listed})");

    let plugins_txt_after = fs::read(canonical_path(&state, "plugins.txt")).unwrap();
    println!(
        "plugins.txt identical after the disable? {}",
        plugins_txt_before == plugins_txt_after
    );
    println!("plugins.txt =\n{}", String::from_utf8_lossy(&plugins_txt_after));

    // === the GUI's compute_plugins path (SelectTab / F5 / finish_run) ===
    let mut fresh = PluginList::discover(&sources, &spec);
    fresh.apply_prefix_state(&state, &spec);
    fresh.refresh(&spec);
    let fish_after = fresh
        .plugins
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case("ccBGSSSE001-Fish.esm"))
        .unwrap();
    println!("fish AFTER reload: enabled={}", fish_after.enabled);
    assert!(fish_after.enabled, "REFUTED: the disable survived the reload");

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&state);
}
