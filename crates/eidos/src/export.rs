//! `eidos export`: the mod list as CSV/markdown, for sharing a setup.

use std::process::exit;


use crate::*;

/// Quote a CSV string field MO2-style: always wrapped in double quotes, embedded
/// quotes doubled.
pub(crate) fn csv_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// A directory's modified time as `yyyy/MM/dd HH:mm:ss` (UTC; MO2 uses local time,
/// a documented divergence to stay dependency-free). Empty if unreadable.
pub(crate) fn fmt_mtime(path: &std::path::Path) -> String {
    let Ok(secs) = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).map_err(|_| std::io::Error::other("pre-epoch")))
        .map(|d| d.as_secs() as i64)
    else {
        return String::new();
    };
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let (h, mi, s) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}/{m:02}/{d:02} {h:02}:{mi:02}:{s:02}")
}

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
    let factory = inst.category_factory();
    let domain = game.def.nexus_game;

    let mut csv = String::from(
        "#Mod_Priority,#Mod_Status,#Mod_Name,#Note,#Primary_Category,#Mod_Author,\
         #Mod_Uploader,#Nexus_ID,#Mod_Nexus_URL,#Mod_Uploader_URL,#Mod_Version,\
         #Install_Date,#Download_File_Name\r\n",
    );
    let mut count = 0usize;
    for (i, m) in inst.modlist().iter().enumerate() {
        if active_only && !m.enabled {
            continue;
        }
        let meta = inst.mod_meta(&m.name);
        let note = meta.notes().unwrap_or_default().replace(',', "");
        let category = meta
            .category()
            .as_deref()
            .and_then(eidos_instance::parse_primary)
            .and_then(|cid| factory.name_for_id(cid))
            .unwrap_or_default()
            .to_string();
        let nexus_id = meta.mod_id().unwrap_or(0);
        let nexus_url = if nexus_id > 0 && !domain.is_empty() {
            format!("https://www.nexusmods.com/{domain}/mods/{nexus_id}")
        } else {
            String::new()
        };
        // (value, quoted) per MO2: every string quoted; Nexus_ID is the only bare int.
        let cells: [(String, bool); 13] = [
            (format!("{i:04}"), true),
            ((if m.enabled { "+" } else { "-" }).to_string(), true),
            (m.name.clone(), true),
            (note, true),
            (category, true),
            (String::new(), true), // author - not tracked
            (String::new(), true), // uploader - not tracked
            (nexus_id.to_string(), false),
            (nexus_url, true),
            (String::new(), true), // uploader url - not tracked
            (meta.version().unwrap_or_default(), true),
            (fmt_mtime(&m.path), true),
            (meta.installation_file().unwrap_or_default(), true),
        ];
        let row: Vec<String> =
            cells.into_iter().map(|(v, quoted)| if quoted { csv_quote(&v) } else { v }).collect();
        csv.push_str(&row.join(","));
        csv.push_str("\r\n");
        count += 1;
    }

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
