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

use crate::{fix_directory_name, guess_mod_name};

/// Write a MO2-compatible `meta.ini`, seeded from the download's `<archive>.meta`
/// sidecar if MO2/Nexus left one next to the file. `guessed_id` is the mod id
/// recovered from the filename, used when the sidecar carries none.
#[cfg(test)]
pub(crate) fn write_meta(
    archive: &Path,
    dest: &Path,
    game_id: &str,
    guessed_id: Option<u64>,
) -> io::Result<()> {
    write_meta_preserving(archive, dest, game_id, guessed_id, None, false)
}

pub(crate) fn write_meta_preserving(
    archive: &Path,
    dest: &Path,
    game_id: &str,
    guessed_id: Option<u64>,
    preserved: Option<ModMeta>,
    merging: bool,
) -> io::Result<()> {
    // The sidecar is the full archive name + ".meta" (e.g. Mod-1234.7z.meta).
    let sidecar = PathBuf::from(format!("{}.meta", archive.to_string_lossy()));
    let from = ModMeta::read_checked(&sidecar)?;

    // Preserve raw user keys and other sections, including collection ownership.
    // Only archive facts and the source array are updated; failure aborts publication.
    let had_metadata = preserved.is_some();
    let mut meta = preserved.unwrap_or_default();
    // Only an explicit collection-owned publication may retain replacement authority.
    meta.set("eidosCollectionOwner", "");
    let mut sources = if merging {
        meta.installed_files()
    } else {
        Vec::new()
    };
    if merging && sources.is_empty() && !meta.has_installed_files() {
        if let (Some(m), Some(f)) = (meta.mod_id(), meta.file_id()) {
            sources.push((m, f));
        }
    }
    if let (Some(m), Some(f)) = (from.mod_id().or(guessed_id), from.file_id()) {
        sources.push((m, f));
    }
    meta.set_installed_files(&sources);
    meta.set("fileID", &from.file_id().unwrap_or(0).to_string());
    // MO2 records the game's SHORT NAME here (`SkyrimSE`), not a lowercase id.
    // Eidos was writing its own id, which nothing reads back for behaviour but
    // which MO2 does not recognise when it opens a mod Eidos installed. An id
    // outside the catalog falls back to itself rather than inventing a spelling.
    let short = eidos_gamedef::GameDef::for_id(game_id)
        .map(|d| d.short_name)
        .filter(|s| !s.is_empty())
        .unwrap_or(game_id);
    meta.set(
        "gameName",
        &from.game_name().unwrap_or_else(|| short.to_string()),
    );
    // Mod id: the sidecar's, else the one guessed from the Nexus filename, so a
    // manually-downloaded archive with no sidecar can still be update-checked.
    meta.set(
        "modid",
        &from.mod_id().or(guessed_id).unwrap_or(0).to_string(),
    );
    // Version: the sidecar's, else a date stamp from the archive mtime (MO2's
    // dYYYY.M.D fallback) so update_available has a baseline to compare against.
    if let Some(v) = from.version().or_else(|| archive_date_version(archive)) {
        meta.set("version", &v);
    }
    if let Some(nv) = from.newest_version() {
        meta.set("newestVersion", &nv);
    }
    // The sidecar's category is a raw Nexus id we don't map yet; leave uncategorised.
    if !had_metadata {
        meta.set("category", "\"-1,\"");
    }
    // nexusFileStatus mirrors the sidecar's fileCategory (1 = main file by default).
    meta.set(
        "nexusFileStatus",
        &from.file_category().unwrap_or_else(|| "1".to_string()),
    );
    // Who made it, carried through from the sidecar the download wrote. Without
    // this the mod has no author until the user happens to run an update check,
    // so "Visit X's profile" is missing on exactly the mods just installed -
    // the ones somebody is most likely to want it on.
    if let Some(a) = from.author() {
        meta.set_author(&a);
    }
    if let Some(u) = from.uploader() {
        meta.set_uploader(&u, &from.uploader_url().unwrap_or_default());
    }
    // Record where the archive came from, absolute (MO2 stores the full path for a
    // file outside the downloads folder).
    let install_file = fs::canonicalize(archive)
        .unwrap_or_else(|_| archive.to_path_buf())
        .to_string_lossy()
        .into_owned();
    meta.set("installationFile", &install_file);
    meta.set(
        "repository",
        &from.repository().unwrap_or_else(|| "Nexus".to_string()),
    );
    if !had_metadata {
        meta.set("endorsed", "0");
        meta.set("tracked", "0");
    }
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
