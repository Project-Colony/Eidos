//! `eidos export`: the mod list as CSV/markdown, for sharing a setup.

use std::process::exit;


use crate::*;

/// `eidos export <game-id> [-o <file>] [--active]`: export the active profile's mod
/// list to CSV in MO2's `exportModListCSV` format (CRLF, always-quoted strings, the
/// unquoted Nexus id, the same 13 columns). Fields Eidos doesn't track (author,
/// uploader) are emitted empty for column/parser parity.
pub(crate) fn cmd_export(args: &[String]) {
    let Some(id) = args.first() else {
        eidos_log::info!("usage: eidos export <game-id> [-o <file>] [--active]");
        exit(2);
    };
    let active_only = args.iter().any(|a| a == "--active");
    let out_path = args.iter().position(|a| a == "-o").and_then(|i| args.get(i + 1)).cloned();
    let target = resolve(id);
    let Some(game) = find_game(&target.game_id) else {
        eidos_log::info!("Game '{}' is not detected. Run `eidos games`.", target.game_id);
        exit(1);
    };
    let inst = target.inst;
    let _ = inst.ensure_profiles();
    let domain = game.def.nexus_game;

    let (csv, count) = eidos_instance::mod_list_csv(
        &inst,
        &inst.modlist(),
        if active_only { eidos_instance::ExportScope::Active } else { eidos_instance::ExportScope::All },
        eidos_instance::Column::ALL,
        domain,
    );

    match out_path {
        Some(p) => match std::fs::write(&p, &csv) {
            Ok(()) => println!("Exported {count} rows to {p}"),
            Err(e) => {
                eidos_log::warn!("write failed: {e}");
                exit(1);
            }
        },
        None => print!("{csv}"),
    }
}
