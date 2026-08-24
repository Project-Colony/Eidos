//! The MO2-compatible `meta.ini` written beside every install, and the
//! date-stamp version fallback.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use eidos_instance::ModMeta;

use crate::{
    fix_directory_name, guess_mod_name,
};



/// Re-apply the user-set fields (endorsement, tracked, category) from a previous
/// install's meta.ini onto a freshly written one, so a Replace doesn't lose them.
pub(crate) fn reapply_user_meta(old: &ModMeta, meta_path: &Path) {
    let mut m = ModMeta::read(meta_path);
    if old.endorsed() {
        m.set("endorsed", "1");
    }
    if old.tracked() {
        m.set("tracked", "1");
    }
    if let Some(c) = old.category() {
        m.set("category", &format!("\"{c}\""));
    }
    // Everything else the USER typed, which a reinstall rewrites from the
    // archive and would otherwise destroy. These are not recoverable from
    // anywhere - a note is a sentence somebody wrote, a colour is a decision
    // about a list, a page is a link nothing else records - so losing them to
    // "update this mod" is a silent, permanent cost of keeping a setup current.
    if let Some(n) = old.notes() {
        m.set_notes(&n);
    }
    if let Some(rgb) = old.color() {
        m.set_color(Some(rgb));
    }
    if let Some(u) = old.url() {
        m.set_url(&u);
    }
    // The local flags too: "ignore updates" survives an update by definition,
    // and re-arming it every reinstall is exactly the wrong default.
    if old.ignore_update() {
        m.set_ignore_update(true);
    }
    let _ = m.write(meta_path);
}

/// Write a MO2-compatible `meta.ini`, seeded from the download's `<archive>.meta`
/// sidecar if MO2/Nexus left one next to the file. `guessed_id` is the mod id
/// recovered from the filename, used when the sidecar carries none.
pub(crate) fn write_meta(archive: &Path, dest: &Path, game_id: &str, guessed_id: Option<u64>) -> io::Result<()> {
    // The sidecar is the full archive name + ".meta" (e.g. Mod-1234.7z.meta).
    let sidecar = PathBuf::from(format!("{}.meta", archive.to_string_lossy()));
    let from = ModMeta::read(&sidecar);

    let mut meta = ModMeta::default();
    // MO2 records the game's SHORT NAME here (`SkyrimSE`), not a lowercase id.
    // Eidos was writing its own id, which nothing reads back for behaviour but
    // which MO2 does not recognise when it opens a mod Eidos installed. An id
    // outside the catalog falls back to itself rather than inventing a spelling.
    let short = eidos_gamedef::GameDef::for_id(game_id)
        .map(|d| d.short_name)
        .filter(|s| !s.is_empty())
        .unwrap_or(game_id);
    meta.set("gameName", &from.game_name().unwrap_or_else(|| short.to_string()));
    // Mod id: the sidecar's, else the one guessed from the Nexus filename, so a
    // manually-downloaded archive with no sidecar can still be update-checked.
    if let Some(id) = from.mod_id().or(guessed_id) {
        meta.set("modid", &id.to_string());
    }
    // Version: the sidecar's, else a date stamp from the archive mtime (MO2's
    // dYYYY.M.D fallback) so update_available has a baseline to compare against.
    if let Some(v) = from.version().or_else(|| archive_date_version(archive)) {
        meta.set("version", &v);
    }
    if let Some(nv) = from.newest_version() {
        meta.set("newestVersion", &nv);
    }
    // The sidecar's category is a raw Nexus id we don't map yet; leave uncategorised.
    meta.set("category", "\"-1,\"");
    // nexusFileStatus mirrors the sidecar's fileCategory (1 = main file by default).
    meta.set("nexusFileStatus", &from.file_category().unwrap_or_else(|| "1".to_string()));
    // Record where the archive came from, absolute (MO2 stores the full path for a
    // file outside the downloads folder).
    let install_file = fs::canonicalize(archive)
        .unwrap_or_else(|_| archive.to_path_buf())
        .to_string_lossy()
        .into_owned();
    meta.set("installationFile", &install_file);
    meta.set("repository", &from.repository().unwrap_or_else(|| "Nexus".to_string()));
    meta.set("endorsed", "0");
    meta.set("tracked", "0");
    meta.write(&dest.join("meta.ini"))
}

/// MO2's date-stamp version fallback (`dYYYY.M.D`, no zero-padding) from the
/// archive's modification time, used when the download has no real version.
pub(crate) fn archive_date_version(archive: &Path) -> Option<String> {
    let secs = fs::metadata(archive)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let (y, m, d) = civil_from_unix(secs);
    Some(format!("d{y}.{m}.{d}"))
}

/// Year/month/day (UTC) from a Unix timestamp - Hinnant's civil-from-days, so no
/// calendar crate is needed.
pub(crate) fn civil_from_unix(secs: u64) -> (i64, u32, u32) {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The mod folder name for `archive`, with MO2's precedence: the download sidecar's
/// `modName`, else its `name`, else the filename guess - then sanitized with
/// [`fix_directory_name`] (real Nexus names contain `:`).
pub fn mod_name_for(archive: &Path) -> String {
    let sidecar = PathBuf::from(format!("{}.meta", archive.to_string_lossy()));
    let meta = ModMeta::read(&sidecar);
    let picked = meta
        .mod_name()
        .or_else(|| meta.name())
        .unwrap_or_else(|| guess_mod_name(&archive.to_string_lossy()));
    fix_directory_name(&picked)
        .or_else(|| fix_directory_name(&guess_mod_name(&archive.to_string_lossy())))
        .unwrap_or_else(|| "Mod".to_string())
}
