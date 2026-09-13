//! `eidos games` and `eidos init`: detection, and creating an instance.

use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind};

pub(crate) fn find_game(id: &str) -> Option<DetectedGame> {
    eidos_games::select_installation(&detect(&home()), id, None).cloned()
}

/// Recheck the manifest after resolving a target, without accepting another game's instance.
pub(crate) fn checked_instance_manifest(
    instance: &Instance,
    game_id: &str,
) -> Result<Option<eidos_instance::Manifest>, String> {
    let manifest = eidos_instance::Manifest::read_checked(&instance.manifest_path())
        .map_err(|error| format!("Cannot read the instance identity: {error}"))?;
    if manifest
        .as_ref()
        .is_some_and(|manifest| manifest.game_id != game_id)
    {
        return Err(format!(
            "The current instance manifest does not name '{game_id}'; reopen the intended instance"
        ));
    }
    Ok(manifest)
}

/// Validate against every current candidate while the instance mutation lock is held.
/// A missing or legacy key must remain unambiguous before matching the expected copy.
pub(crate) fn validate_selected_game(
    instance: &Instance,
    game_id: &str,
    game: &DetectedGame,
    candidates: &[DetectedGame],
) -> std::io::Result<()> {
    let manifest = checked_instance_manifest(instance, game_id).map_err(std::io::Error::other)?;
    let selected = eidos_games::select_installation(
        candidates,
        game_id,
        manifest.as_ref().and_then(|m| m.installation.as_deref()),
    );
    if game.def.id != game_id
        || selected.is_none_or(|selected| selected.selection_id() != game.selection_id())
    {
        return Err(std::io::Error::other(
            "The selected game installation changed; reopen the intended instance",
        ));
    }
    Ok(())
}

pub(crate) fn find_instance_game(target: &crate::resolve::Target) -> Option<DetectedGame> {
    select_instance_game(target, &detect(&home()))
}

fn select_instance_game(
    target: &crate::resolve::Target,
    games: &[DetectedGame],
) -> Option<DetectedGame> {
    let manifest = match checked_instance_manifest(&target.inst, &target.game_id) {
        Ok(value) => value,
        Err(error) => {
            eidos_log::warn!("{error}");
            return None;
        }
    };
    let selected = eidos_games::select_installation(
        games,
        &target.game_id,
        manifest.as_ref().and_then(|m| m.installation.as_deref()),
    )
    .cloned();
    if selected.is_none() && games.iter().any(|g| g.def.id == target.game_id) {
        eidos_log::warn!("The saved installation is unavailable or multiple copies exist. Select the exact game path when creating/opening the instance; Eidos will not choose another copy.");
    }
    selected
}

pub(crate) fn cmd_games() {
    let games = detect(&home());
    if games.is_empty() {
        println!(
            "No supported games detected. Check the Steam, Heroic or Legendary installed manifests."
        );
        return;
    }
    println!("Supported games installed:");
    for g in &games {
        println!("  {:<10} {}  ({})", g.def.id, g.def.name, g.source_name());
        println!("             game: {}", g.install_path.display());
        println!("             data: {}", g.data_path.display());
    }
    println!("\nNext: `eidos init <id>` to create a modding instance.");
}

pub(crate) fn cmd_init(id: &str, folder: Option<&str>, game_path: Option<&str>) {
    let games = detect(&home());
    let game = if let Some(path) = game_path {
        let canonical = std::fs::canonicalize(crate::resolve::expand(path)).ok();
        let mut matched = games
            .iter()
            .filter(|g| g.def.id == id && canonical.as_ref() == Some(&g.install_path));
        let first = matched.next().cloned();
        if matched.next().is_some() {
            None
        } else {
            first
        }
    } else {
        find_game(id)
    };
    let Some(game) = game else {
        eidos_log::info!("Game '{id}' is not detected. Run `eidos games` and select a copy with --game-path <folder>.");
        exit(1);
    };
    // With a folder the instance is PORTABLE: self-contained there, movable,
    // and remembered in the registry so the GUI and later commands find it.
    let (inst, kind) = match folder {
        Some(f) => {
            let root = crate::resolve::expand(f);
            // Never inside a game's install - any detected game's. Steam owns
            // those trees (updates/uninstalls rewrite or delete them) and
            // Eidos mounts over the game root, so an instance there would sit
            // inside its own mount target.
            if let Some(g) = detect(&home())
                .into_iter()
                .find(|g| Instance::root_inside_game(&root, &g.install_path))
            {
                eidos_log::warn!(
                    "'{}' is inside {}'s own folder - an instance cannot live there.\n\
                     Steam owns that tree (an update or uninstall can wipe it), and Eidos\n\
                     mounts over the game root, so the instance would live inside its own\n\
                     mount target. Put it NEXT to the game instead, e.g. a sibling folder.",
                    root.display(),
                    g.def.name
                );
                exit(1);
            }
            if let Some(m) = Instance::portable(root.clone()).read_manifest() {
                if m.game_id != id {
                    eidos_log::info!(
                        "'{}' already holds a '{}' instance - not stamping it as '{id}'.",
                        root.display(),
                        m.game_id
                    );
                    exit(1);
                }
            }
            (Instance::portable(root), InstanceKind::Portable)
        }
        None => (Instance::global(id), InstanceKind::Global),
    };
    let _lock = inst
        .try_lock("creating an instance")
        .unwrap_or_else(|error| {
            eidos_log::warn!("Cannot create this instance: {error}");
            exit(1);
        });
    if let Err(error) = inst
        .create()
        .and_then(|()| inst.ensure_installation(id, kind, &game.selection_id()))
    {
        eidos_log::warn!("Cannot create this instance: {error}");
        exit(1);
    }
    if kind == InstanceKind::Portable {
        let mut reg = eidos_instance::Registry::load();
        reg.set_last(eidos_instance::InstanceRef::Portable(inst.root.clone()));
        if let Err(error) = reg.save() {
            eidos_log::warn!("Instance created, but it could not be registered: {error}");
            exit(1);
        }
    }
    // The old text promised a `../load_order.txt` that NOTHING has ever read
    // since profiles arrived: a user following it created a file with no effect
    // and believed their order applied. Describe the mechanism that exists.
    if let Err(error) = std::fs::write(
        inst.mods_dir().join("README.txt"),
        "Drop each mod here as its own folder.\n\
         A folder added by hand appears DISABLED at the bottom of the mod list;\n\
         enable and order it in the Eidos GUI (or edit the active profile's\n\
         profiles/<name>/modlist.txt: one +Name/-Name per line, top = highest\n\
         priority, wins file conflicts - MO2's format).\n",
    ) {
        eidos_log::warn!("Instance created, but its instructions could not be saved: {error}");
        exit(1);
    }
    println!(
        "Created {} instance for {} ({id}).",
        if kind == InstanceKind::Portable {
            "portable"
        } else {
            "global"
        },
        game.def.name
    );
    println!("  instance : {}", inst.root.display());
    println!("  game data: {}", game.data_path.display());
    println!("  add mods : {}", inst.mods_dir().display());
    // A portable instance is addressed by its folder from here on.
    let inst_arg = match kind {
        InstanceKind::Global => id.to_string(),
        InstanceKind::Portable => inst.root.display().to_string(),
    };
    if game.is_steam() {
        println!("\nThen: `eidos play {inst_arg} -- %command%` (as a Steam launch option).");
    } else {
        println!("\nLaunch manually with `eidos play {inst_arg} -- <runner> <game>`. Use this installation's runner and prefix; automatic Heroic/Legendary launching is not configured.");
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn validation_uses_all_candidates_before_accepting_a_legacy_identity() {
        struct Owned(std::path::PathBuf);
        impl Drop for Owned {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Owned(
            std::env::temp_dir().join(format!("eidos-cli-all-identities-{}", std::process::id())),
        );
        std::fs::create_dir(&root.0).unwrap();
        let instance = Instance::portable(root.0.clone());
        let first = DetectedGame {
            def: eidos_games::catalog()
                .iter()
                .find(|g| g.id == "skyrimse")
                .unwrap(),
            source: eidos_games::GameSource::External {
                store: eidos_games::Store::Gog,
                app_id: "synthetic".into(),
                prefix: Some(root.0.join("prefix-a")),
                heroic: true,
            },
            install_path: root.0.join("game"),
            data_path: root.0.join("game/Data"),
            compatdata: None,
            steam_name: "Synthetic".into(),
        };
        let mut second = first.clone();
        second.source = eidos_games::GameSource::External {
            store: eidos_games::Store::Gog,
            app_id: "synthetic".into(),
            prefix: Some(root.0.join("prefix-b")),
            heroic: true,
        };
        let candidates = [first.clone(), second];
        let mut manifest = eidos_instance::Manifest::new("skyrimse", InstanceKind::Portable);
        manifest.installation = Some(first.selection_id());
        manifest.write(&instance.manifest_path()).unwrap();
        validate_selected_game(&instance, "skyrimse", &first, &candidates).unwrap();
        assert!(
            validate_selected_game(&instance, "skyrimse", &first, &candidates[1..]).is_err(),
            "previously selected copy is no longer detected"
        );
        manifest.installation = Some(candidates[1].selection_id());
        manifest.write(&instance.manifest_path()).unwrap();
        assert!(
            validate_selected_game(&instance, "skyrimse", &first, &candidates).is_err(),
            "different full source identity"
        );
        manifest.installation = Some(
            serde_json::json!(["GOG:synthetic", first.install_path.to_string_lossy()]).to_string(),
        );
        manifest.write(&instance.manifest_path()).unwrap();
        assert!(
            validate_selected_game(&instance, "skyrimse", &first, &candidates).is_err(),
            "a legacy key must not hide a second prefix"
        );
        validate_selected_game(&instance, "skyrimse", &first, &candidates[..1]).unwrap();
        manifest.installation = None;
        manifest.write(&instance.manifest_path()).unwrap();
        assert!(
            validate_selected_game(&instance, "skyrimse", &first, &candidates).is_err(),
            "missing identity is ambiguous too"
        );
        validate_selected_game(&instance, "skyrimse", &first, &candidates[..1]).unwrap();
    }

    #[test]
    fn current_manifest_rejects_mismatches_but_preserves_legacy_selection() {
        let root =
            std::env::temp_dir().join(format!("eidos-cli-current-identity-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let instance = Instance::portable(root.clone());
        let game = DetectedGame {
            def: eidos_games::catalog()
                .iter()
                .find(|g| g.id == "skyrimse")
                .unwrap(),
            source: Default::default(),
            install_path: root.join("game"),
            data_path: root.join("game/Data"),
            compatdata: None,
            steam_name: "Synthetic".into(),
        };
        let games = [game];
        let target = crate::resolve::Target {
            inst: instance,
            game_id: "skyrimse".into(),
        };
        validate_selected_game(&target.inst, &target.game_id, &games[0], &games).unwrap();
        assert!(
            select_instance_game(&target, &games).is_some(),
            "absent legacy manifest"
        );
        eidos_instance::Manifest::new("skyrimse", InstanceKind::Global)
            .write(&target.inst.manifest_path())
            .unwrap();
        validate_selected_game(&target.inst, &target.game_id, &games[0], &games).unwrap();
        assert!(
            select_instance_game(&target, &games).is_some(),
            "valid legacy manifest without source key"
        );
        eidos_instance::Manifest::new("fallout4", InstanceKind::Global)
            .write(&target.inst.manifest_path())
            .unwrap();
        assert!(validate_selected_game(&target.inst, &target.game_id, &games[0], &games).is_err());
        assert!(
            select_instance_game(&target, &games).is_none(),
            "mismatched current global manifest"
        );
        let mut portable = eidos_instance::Manifest::new("skyrimse", InstanceKind::Portable);
        portable.installation = Some(games[0].selection_id());
        portable.write(&target.inst.manifest_path()).unwrap();
        validate_selected_game(&target.inst, &target.game_id, &games[0], &games).unwrap();
        assert!(select_instance_game(&target, &games).is_some());
        // Target still holds the original id after another actor changes its manifest.
        eidos_instance::Manifest::new("fallout4", InstanceKind::Portable)
            .write(&target.inst.manifest_path())
            .unwrap();
        assert!(validate_selected_game(&target.inst, &target.game_id, &games[0], &games).is_err());
        assert!(
            select_instance_game(&target, &games).is_none(),
            "changed portable manifest"
        );
        portable.installation = Some("another installation".into());
        portable.write(&target.inst.manifest_path()).unwrap();
        assert!(validate_selected_game(&target.inst, &target.game_id, &games[0], &games).is_err());
        assert!(
            select_instance_game(&target, &games).is_none(),
            "changed source key"
        );
        std::fs::write(target.inst.manifest_path(), b"broken manifest").unwrap();
        assert!(validate_selected_game(&target.inst, &target.game_id, &games[0], &games).is_err());
        assert!(
            select_instance_game(&target, &games).is_none(),
            "corrupt current manifest"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
