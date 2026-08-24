//! Per-profile saves: the directory, seeding, enumeration, co-saves.

//! A profile: one named set of enabled mods + their order (and, later, its own
//! `plugins.txt`, INIs and saves), all sharing the instance's single `mods/`
//! pool. This is what lets one mod collection serve several playthroughs.
//!
//! Mirrors Mod Organizer 2: a profile is just a directory under
//! `<instance>/profiles/<name>/`; its `modlist.txt` carries both the enabled set
//! and the priority order, while the mods themselves stay global.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};



use super::*;

/// The save-file extensions across the supported game families: Bethesda's
/// `.ess` (Skyrim LE/SE), `.fos` (Fallout 3/NV/4) and `.sfs` (Starfield).
pub(crate) const SAVE_EXTS: &[&str] = &["ess", "fos", "sfs"];

/// Script-extender co-saves that travel WITH a save: same stem, own extension.
pub(crate) const COSAVE_EXTS: &[&str] = &["skse", "f4se", "nvse", "fose", "sfse", "obse"];

pub(crate) fn ext_of(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// Save DATA: a save or its co-save - what the seed and the cloud sync move.
/// An earlier filter hard-coded `.ess`/`.skse` and killed the entire save
/// pipeline for the Fallout and Starfield families in one line.
pub fn is_save_data(name: &str) -> bool {
    let e = ext_of(name);
    SAVE_EXTS.contains(&e.as_str()) || COSAVE_EXTS.contains(&e.as_str())
}

/// A save as the user thinks of one - what the Saves tab lists. Co-saves travel
/// with their `.ess`/`.fos`/`.sfs` and are not shown separately.
pub fn is_save_listing(name: &str) -> bool {
    SAVE_EXTS.contains(&ext_of(name).as_str())
}

/// The co-save paths that belong to `save` (same stem, co-save extensions), for
/// operations that must treat the pair as one unit - deleting a save while
/// leaving its co-save made an invisible orphan the cloud sync pushed forever.
pub fn cosave_siblings(save: &Path) -> Vec<PathBuf> {
    let Some(stem) = save.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()) else {
        return Vec::new();
    };
    let Some(dir) = save.parent() else { return Vec::new() };
    // Case-insensitive on BOTH halves, like every other save predicate: the game
    // wrote these on a filesystem it thought folded case, so `Quicksave.SKSE`
    // next to `quicksave.ess` is normal - and an exact-case join recreated the
    // invisible-orphan class this helper exists to close.
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            let Some((s, ext)) = name.rsplit_once('.') else { return false };
            s == stem && COSAVE_EXTS.contains(&ext) && e.path().is_file()
        })
        .map(|e| e.path())
        .collect()
}

/// Whether a plugins.txt has the signature of a crash artifact: several plugins
/// LISTED, none active. A deliberate everything-off edit also matches - but at
/// seed time there is no user history to defer to, and deriving from discovery
/// beats founding a profile on a wreck.
pub(crate) fn looks_like_crash_artifact(path: &Path) -> bool {
    let Some(text) = eidos_plugins::read_decoded(path) else { return false };
    let mut listed = 0usize;
    let mut active = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        listed += 1;
        if line.starts_with('*') {
            active += 1;
        }
    }
    listed >= 3 && active == 0
}

impl Profile {
    /// This profile's own save-games directory (`profil/saves`), MO2's
    /// `savePath()`. At launch it is bind-mounted over the prefix's save dir so
    /// the game reads and writes this profile's saves.
    pub fn saves_dir(&self) -> PathBuf {
        self.dir().join("saves")
    }

    /// One-time migration: copy the user's existing saves from `src_saves` (the
    /// prefix `Documents/My Games/<game>/Saves`) into this profile, but only if
    /// the profile has no saves yet - so an existing playthrough is adopted, not
    /// hidden when this profile's saves get bound over the prefix at launch.
    /// Returns how many save files were copied (0 if the profile already has any).
    pub fn seed_saves(&self, src_saves: &Path) -> io::Result<u32> {
        let dst = self.saves_dir();
        // "Already seeded" is a persistent MARKER, not the directory being
        // non-empty. The emptiness probe re-armed the seeding whenever the user
        // emptied the dir from the GUI - resurrecting years-old prefix saves
        // with fresh mtimes that then sorted above the real playthrough - and a
        // dir holding only a stray subdirectory blocked adoption forever.
        let marker = dst.join(".seeded");
        // Two marker states: "partial" means a previous adoption hit unreadable
        // files and the missing ones should be retried; anything else (including
        // the empty marker older binaries wrote) means done. Without the state,
        // the non-empty-dir shortcut below re-stamped a partial adoption as
        // complete on the very next run.
        let mut pending: std::collections::HashSet<String> = Default::default();
        let resuming = match fs::read_to_string(&marker) {
            Ok(state) => {
                let mut lines = state.lines();
                if lines.next().map(str::trim) != Some("partial") {
                    return Ok(0);
                }
                pending = lines.map(|l| l.trim().to_ascii_lowercase()).collect();
                true
            }
            Err(_) => false,
        };
        fs::create_dir_all(&dst)?;
        if !resuming {
            let already =
                fs::read_dir(&dst).map(|it| it.flatten().next().is_some()).unwrap_or(false);
            if already {
                // Pre-marker profiles: their saves are the adoption. Record and stop.
                let _ = fs::write(&marker, b"done");
                return Ok(0);
            }
        }
        let Ok(rd) = fs::read_dir(src_saves) else {
            // NO marker: an absent or unreadable source is a transient (wrong
            // drive not mounted, prefix not created yet), and stamping "done"
            // here disarmed adoption forever before anything existed to adopt.
            return Ok(0);
        };
        let mut n = 0;
        let mut failed: Vec<String> = Vec::new();
        let mut saw_any = false;
        for e in rd.flatten() {
            // Save data only, across every supported family (.ess/.fos/.sfs +
            // co-saves): steam_autocloud.vdf and .bak files are not saves and
            // used to show up in the Saves tab as one.
            if !is_save_data(&e.file_name().to_string_lossy()) || !e.path().is_file() {
                continue;
            }
            saw_any = true;
            let to = dst.join(e.file_name());
            if to.exists() {
                continue; // adopted on a previous (partial) pass
            }
            if resuming && !pending.contains(&e.file_name().to_string_lossy().to_ascii_lowercase())
            {
                // A resume only fetches what FAILED last time. Re-copying every
                // file missing from the profile resurrected saves the user had
                // deliberately deleted since the first pass.
                continue;
            }
            // One unreadable file must not abort the adoption of a whole
            // playthrough - it used to, silently, mid-loop, and the partial
            // profile then blocked completion forever via the emptiness probe.
            if copy_atomic(&e.path(), &to).is_err() {
                failed.push(e.file_name().to_string_lossy().into_owned());
                continue;
            }
            // Keep the save's real date: the Saves tab sorts by it, and a copy
            // stamped "now" made ancient saves sort above the live playthrough.
            if let Ok(mtime) = e.metadata().and_then(|m| m.modified()) {
                if let Ok(f) = fs::File::options().write(true).open(&to) {
                    let _ = f.set_modified(mtime);
                }
            }
            n += 1;
        }
        if !failed.is_empty() {
            // The marker records WHICH files failed: the resume fetches exactly
            // those, and stamping "done" over a partial adoption made the holes
            // permanent - unfixable even by deleting the marker, since a
            // non-empty dir also counted as adopted.
            eprintln!(
                "eidos: WARNING - {} save file(s) could not be adopted into profile '{}' \
                 (unreadable); adopted {n}, will retry the rest next launch",
                failed.len(),
                self.name
            );
            let body = format!("partial\n{}", failed.join("\n"));
            let _ = fs::write(&marker, body);
        } else if saw_any {
            let _ = fs::write(&marker, b"done");
        }
        // A readable dir holding NO save data writes no marker at all: Steam
        // creates the Saves dir (with its autocloud sidecar) before the user
        // ever saves, and stamping "done" then disarmed adoption forever for
        // saves that appeared five minutes later.
        Ok(n)
    }

    /// List this profile's save files (`profil/saves/<file>`), newest first and
    /// capped to a sane number. Directories and dotfiles are skipped; this never
    /// errors - a missing or unreadable saves dir yields an empty list.
    pub fn savegames(&self) -> Vec<SaveEntry> {
        let mut out: Vec<SaveEntry> = fs::read_dir(self.saves_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let md = e.metadata().ok()?;
                if !md.is_file() {
                    return None;
                }
                let filename = e.file_name().into_string().ok()?;
                // Real saves only, for every supported family: the Saves tab
                // listed steam_autocloud.vdf as a playthrough entry, and an
                // .ess-only filter would blank the tab for Fallout/Starfield.
                if filename.starts_with('.') || !is_save_listing(&filename) {
                    return None;
                }
                Some(SaveEntry {
                    filename,
                    path: e.path(),
                    size: md.len(),
                    mtime: md.modified().ok()?,
                })
            })
            .collect();
        out.sort_by_key(|s| std::cmp::Reverse(s.mtime));
        out
    }
}
