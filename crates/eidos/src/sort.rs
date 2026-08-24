//! `eidos sort`: LOOT-order the plugin list.

use std::path::PathBuf;
use std::process::exit;

use eidos_instance::ModEntry;

use crate::*;

/// `eidos sort <game-id> [--dry-run] [--update-masterlist]` - run LOOT's real
/// graph sort (via the pure-Rust libloot) over this instance's plugins and write
/// the optimised order to plugins.txt / loadorder.txt. Mirrors `prepare_plugins`'
/// discovery so the sorted set matches exactly what a launch would deploy.
pub(crate) fn cmd_sort(args: &[String]) {
    let Some(id) = args.first() else {
        eidos_log::info!("usage: eidos sort <game-id> [--dry-run] [--update-masterlist]");
        exit(2);
    };
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let update = args.iter().any(|a| a == "--update-masterlist");

    let target = resolve(id);
    let id = &target.game_id;
    if !eidos_loot::is_supported(id) {
        eidos_log::info!("LOOT sorting is not supported for '{id}' (timestamp-ordered or unmanaged game).");
        exit(1);
    }
    let Some(game) = find_game(id) else {
        eidos_log::info!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };
    let Some(spec) = eidos_plugins::GameSpec::for_id(id) else {
        eidos_log::info!("No plugin support for '{id}'.");
        exit(1);
    };
    let Some(compatdata) = game.compatdata.as_ref() else {
        eidos_log::info!("No Proton prefix found for '{id}'. Launch it once through Steam first.");
        exit(1);
    };
    let prefix = compatdata.join("pfx");
    let local_dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);

    let inst = target.inst;
    let _ = inst.ensure_profiles();
    // The PROFILE owns the plugin state; the prefix copy is a shadow for
    // external tools. This command once wrote only the prefix, and the next
    // launch's profile-driven pass reverted the entire sort - actives included.
    let prof = inst.active();
    let _lock = match inst.try_lock("eidos sort") {
        Ok(l) => l,
        Err(e) => {
            eidos_log::warn!("Cannot sort now: {e}.");
            exit(1);
        }
    };
    if let Err(e) = prof.seed_plugin_state(&local_dir, &spec) {
        eidos_log::warn!("Cannot sort: adopting the plugin state into the profile failed ({e}).");
        exit(1);
    }
    let state_dir = prof.plugins_state_dir();

    // Discover exactly what a launch would use, preserving the current order.
    let enabled: Vec<ModEntry> =
        inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).collect();
    let sources = plugin_sources(&game.data_path, &enabled, &inst.overwrite_dir());
    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);
    list.apply_prefix_state(&state_dir, &spec);
    // Pins resist LOOT here exactly as they do in the window, or `eidos sort`
    // would quietly undo what the GUI promised to hold.
    list.locked = inst.active().read_locked_order();
    list.refresh(&spec);

    if list.plugins.is_empty() {
        eidos_log::info!("No plugins discovered for '{id}'; nothing to sort.");
        exit(1);
    }

    // Fetch/cache the per-game masterlist + shared prelude.
    let (_game_type, repo) = eidos_loot::loot_support(id).unwrap();
    let cache = inst.root.join("loot");
    let (masterlist, prelude) = match eidos_loot::ensure_masterlist(repo, &cache, update) {
        Ok(p) => p,
        Err(e) => {
            eidos_log::warn!("Could not obtain masterlist: {e}");
            exit(1);
        }
    };
    let userlist = cache.join("userlist.yaml");

    // Hand LOOT every discovered plugin by (name, real resolved path).
    let plugins: Vec<(String, PathBuf)> =
        list.plugins.iter().map(|p| (p.name.clone(), p.path.clone())).collect();
    // And where those plugins actually live: under Eidos the game's own Data dir
    // holds only vanilla, so without these every file-conditioned masterlist
    // rule is evaluated against a tree the mods are not in. Highest priority
    // first, Overwrite ahead of all, as the union resolves.
    // `load_order` already excludes separators and unmanaged content; the
    // is_dir filter guards against a mod folder deleted since the list was read,
    // which libloot answers with a bare "an I/O error occurred".
    let mut mod_dirs: Vec<PathBuf> = vec![inst.overwrite_dir()];
    mod_dirs.extend(inst.load_order());
    mod_dirs.retain(|p| p.is_dir());

    let view = eidos_loot::GameView {
        game_id: id,
        game_path: &game.install_path,
        // The PROFILE dir: it is the load-order authority now; the prefix copy is
        // a shadow that can be stale.
        local_path: &state_dir,
        plugins: &plugins,
        mod_dirs: &mod_dirs,
        masterlist: &masterlist,
        prelude: &prelude,
        userlist: Some(&userlist),
    };
    let sorted = match eidos_loot::sort(&view) {
        Ok(s) => s,
        Err(e) => {
            eidos_log::warn!("LOOT sort failed: {e}");
            exit(1);
        }
    };

    if dry_run {
        println!("LOOT-sorted order ({} plugins):", sorted.len());
        for (i, n) in sorted.iter().enumerate() {
            println!("  {i:>3}  {n}");
        }
        println!("\n(dry run - nothing written; drop --dry-run to apply)");
        return;
    }

    list.apply_sorted_order(&sorted);
    list.refresh(&spec);
    let active = list.plugins.iter().filter(|p| p.enabled).count();
    match list.write_load_order(&state_dir, &spec) {
        Ok(_) => {
            // Shadow for external tools reading the prefix; never fatal.
            let _ = list.write_load_order(&local_dir, &spec);
            println!("Sorted {} plugins ({active} active) and wrote the load order.", sorted.len())
        }
        Err(e) => {
            eidos_log::warn!("Could not write load order: {e}");
            exit(1);
        }
    }
}
