//! The profile's mod list and lifecycle: modlist.txt round-trip, the
//! persist-safety guard, create/copy/rename/delete.

//! A profile: one named set of enabled mods + their order (and, later, its own
//! `plugins.txt`, INIs and saves), all sharing the instance's single `mods/`
//! pool. This is what lets one mod collection serve several playthroughs.
//!
//! Mirrors Mod Organizer 2: a profile is just a directory under
//! `<instance>/profiles/<name>/`; its `modlist.txt` carries both the enabled set
//! and the priority order, while the mods themselves stay global.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ModEntry;

use super::*;

/// Recursively copy a directory tree, skipping symlinks (matching MO2's `copyDir`
/// NoSymLinks). Best-effort per entry.
pub(crate) fn copy_dir_recursive(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)?.flatten() {
        let Ok(meta) = e.metadata() else { continue };
        if meta.file_type().is_symlink() {
            continue;
        }
        let dst = to.join(e.file_name());
        if meta.is_dir() {
            let _ = copy_dir_recursive(&e.path(), &dst);
        } else {
            let _ = fs::copy(e.path(), &dst);
        }
    }
    Ok(())
}

/// Prefix of Eidos's own extraction temporaries under `mods/`. Only directories
/// starting with this are hidden from the mod list; every other name, leading dot
/// included, is a mod the user installed.
pub(crate) const INSTALL_TEMP_PREFIX: &str = ".eidos-install-";

/// Whether a freshly-scanned mod list may be written back over `modlist.txt`.
///
/// The reconciliation is destructive by nature - it exists to forget mods that are
/// gone - so it needs the same kind of sanity check the plugin-order capture has
/// (see [`active_loss`]). The order and enabled state are pure user labour,
/// derivable from nothing else on disk; the mod files themselves are replaceable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListTrust {
    /// The scan agrees with the file, or differs by a plausible amount. Safe to save.
    Good,
    /// The scan lost implausibly much. Fine to DISPLAY, never to persist. Carries a
    /// sentence explaining what was seen, for the status bar and the log.
    Suspect(String),
}

impl ListTrust {
    pub fn is_good(&self) -> bool {
        matches!(self, ListTrust::Good)
    }

    /// The reason a list is not to be trusted, if it is not.
    pub fn reason(&self) -> Option<&str> {
        match self {
            ListTrust::Good => None,
            ListTrust::Suspect(why) => Some(why),
        }
    }

    /// Judge a scan: `listed` entries parsed out of `modlist.txt`, `kept` of them
    /// still backed by a folder.
    ///
    /// Thresholds mirror [`active_loss`] deliberately - one rule of thumb for
    /// "this looks like an accident, not an edit" across the whole instance.
    fn judge(readable: bool, listed: usize, kept: usize) -> ListTrust {
        if !readable {
            return ListTrust::Suspect(
                "the mods folder could not be read (is the drive it lives on mounted?)".to_string(),
            );
        }
        let lost = listed.saturating_sub(kept);
        if lost == 0 {
            return ListTrust::Good;
        }
        // Everything vanishing at once is never a real edit. This is the unmounted
        // drive, the wrong instance root, the permissions accident.
        if kept == 0 && listed > 0 {
            return ListTrust::Suspect(format!(
                "all {listed} listed mod(s) are missing from the mods folder"
            ));
        }
        if listed > MIN_ACTIVES
            && lost > MAX_ABSOLUTE_DROP
            && (lost as f64 / listed as f64) > MAX_RELATIVE_DROP
        {
            return ListTrust::Suspect(format!(
                "{lost} of {listed} listed mod(s) are missing from the mods folder"
            ));
        }
        ListTrust::Good
    }
}

impl Profile {
    /// Where to read the mod list from: the profile's own file, or - for the
    /// `Default` profile of a not-yet-migrated instance - the legacy flat
    /// `<instance>/modlist.txt`.
    fn modlist_source(&self) -> PathBuf {
        let own = self.modlist_path();
        if own.exists() {
            return own;
        }
        let legacy = self.instance_root.join("modlist.txt");
        if self.name == "Default" && legacy.exists() {
            return legacy;
        }
        own
    }

    pub fn create(&self) -> io::Result<()> {
        fs::create_dir_all(self.dir())
    }

    /// Create this profile by copying another profile's files (its modlist, plugin
    /// and INI files) and its subdirectories (`saves/`). MO2's profile copy is
    /// fully recursive; copying `saves/` means the new profile starts from this
    /// profile's saves rather than later adopting the stale prefix saves.
    pub fn create_from(&self, other: &Profile) -> io::Result<()> {
        fs::create_dir_all(self.dir())?;
        // Propagate per-file failures: silently skipping files would report a
        // successful copy while producing a profile missing its modlist or INIs.
        if let Ok(rd) = fs::read_dir(other.dir()) {
            for e in rd.flatten() {
                let from = e.path();
                let to = self.dir().join(e.file_name());
                if from.is_file() {
                    fs::copy(&from, &to)?;
                } else if from.is_dir() {
                    copy_dir_recursive(&from, &to)?;
                }
            }
        }
        Ok(())
    }

    pub fn rename(&self, new_name: &str) -> io::Result<Profile> {
        let dest = self.instance_root.join("profiles").join(new_name);
        fs::rename(self.dir(), &dest)?;
        Ok(Profile {
            instance_root: self.instance_root.clone(),
            name: new_name.to_string(),
        })
    }

    pub fn delete(&self) -> io::Result<()> {
        fs::remove_dir_all(self.dir())
    }

    /// The mod list for this profile: every folder in the shared `mods/`, with
    /// enabled state, reconciled with this profile's `modlist.txt`.
    ///
    /// Returned in MO2's DISPLAY order: index 0 = TOP of the list = LOWEST priority
    /// (loaded first, loses file conflicts); the last entry = highest priority (wins,
    /// just above the always-on Overwrite). The on-disk `modlist.txt` keeps MO2's
    /// storage convention (highest priority FIRST), so it round-trips with MO2; this
    /// reverses it for display. Consumers that need launch/priority order (the FUSE
    /// `load_order`, plugin discovery, conflict origins) re-reverse to highest-first.
    pub fn modlist(&self) -> Vec<ModEntry> {
        self.modlist_checked().0
    }

    /// [`Profile::modlist`] plus whether the result is SAFE TO PERSIST.
    ///
    /// Like MO2, the disk decides which mods exist and `modlist.txt` decides only
    /// their order and enabled state, so an entry whose folder is gone drops out
    /// of the list. Unlike MO2, the drop is not automatically written back:
    /// `MO2::refreshModStatus` rewrites `modlist.txt` inside the same refresh, and
    /// its one apparent guard (`if (m_ModStatus.empty()) return;`, profile.cpp:254)
    /// is unreachable because the synthetic overwrite entry always makes the count
    /// non-zero. Point MO2 at an empty mods folder and the curated order is gone.
    ///
    /// That case is not exotic here. A mod pool is hundreds of gigabytes, so it
    /// routinely lives on another drive reached by a bind mount or symlink - and an
    /// unmounted bind-mount target is an existing, readable, EMPTY directory. The
    /// scan would report "you have no mods" for a fully intact pool, and the next
    /// click would make that permanent. So the caller is told when a result is only
    /// fit to display, and [`Profile::save_modlist`] refuses to write it.
    pub fn modlist_checked(&self) -> (Vec<ModEntry>, ListTrust) {
        let mods_dir = self.mods_dir();
        // `file_type()` reads the dirent's d_type instead of stat()ing the target.
        // Two things follow, both load-bearing: a directory that is readable but
        // not SEARCHABLE (mode 0600, an exFAT mount whose dmask drops +x) still
        // reports its children's kinds, where `path().is_dir()` returns false for
        // every one of them and empties the whole list; and a DANGLING symlink is
        // distinguishable from an absent folder.
        let scan = fs::read_dir(&mods_dir).map(|rd| {
            let mut names: Vec<String> = rd
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() || t.is_symlink()))
                .filter_map(|e| e.file_name().into_string().ok())
                // Eidos's own in-flight/crashed extraction temps are not mods. Only
                // ours: a leading dot is legal in a mod name and real mods use it
                // (".NET Script Framework" is a near-universal Skyrim SE
                // dependency), so filtering every dot-dir would silently erase them.
                .filter(|n| !n.starts_with(INSTALL_TEMP_PREFIX))
                .collect();
            names.sort();
            names
        });
        // An unreadable mods/ is NOT an empty one. Saying so is the difference
        // between "the user deleted their mods" and "the drive is not mounted".
        let (present, readable) = match scan {
            Ok(names) => (names, true),
            Err(_) => (Vec::new(), false),
        };

        let mut out: Vec<ModEntry> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut listed = 0usize;

        // A modlist.txt that EXISTS but cannot be read is not the same thing as one
        // that was never written, and the difference decides whether a launch is
        // safe. Absent is the fresh instance: no list yet, every folder gets
        // appended disabled, nothing is lost. Unreadable - a truncated file from an
        // interrupted write, a sync tool mid-copy, EACCES after a restore - looks
        // identical from here (`listed` stays 0, so `judge` computes no loss and
        // says Good) while meaning the opposite: the order EXISTS and we just
        // cannot see it. Launching on that verdict rebuilds the load order from the
        // game's own masters and writes it over the profile.
        let src = self.modlist_source();
        let content = fs::read_to_string(&src);
        let list_lost = matches!(&content, Err(e) if e.kind() != io::ErrorKind::NotFound)
            || matches!(&content, Ok(text) if text.trim().is_empty() && src.is_file() && !present.is_empty());

        if let Ok(content) = content {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let (enabled, unmanaged, name) = if let Some(n) = line.strip_prefix('+') {
                    (true, false, n.trim())
                } else if let Some(n) = line.strip_prefix('-') {
                    (false, false, n.trim())
                } else if let Some(n) = line.strip_prefix('*') {
                    // MO2's mark for the game's own content - DLCs, Creation Club.
                    // Kept as a POSITION, which is the whole point: without a line
                    // of its own the row can only be pinned to the top of the list,
                    // and nothing the user owns can ever sit above it.
                    (true, true, n.trim())
                } else {
                    (true, false, line)
                };
                listed += 1;
                if unmanaged {
                    // The path is not knowable here: it lives in the GAME's data
                    // directory, which this layer has never been told about. The
                    // caller that does know reconciles these against what the game
                    // actually ships - see `Instance::unmanaged_mods`.
                    if seen.insert(name.to_string()) {
                        out.push(ModEntry {
                            name: name.to_string(),
                            enabled,
                            path: PathBuf::new(),
                            unmanaged: true,
                        });
                    }
                } else if present.iter().any(|p| p == name) && seen.insert(name.to_string()) {
                    out.push(ModEntry {
                        name: name.to_string(),
                        enabled,
                        path: mods_dir.join(name),
                        unmanaged: false,
                    });
                }
            }
        }
        // A folder nobody listed: a mod dropped in by hand. MO2 appends it at the
        // highest priority and leaves it DISABLED - it has no idea where in the
        // conflict order it belongs, and enabling it silently could overwrite half
        // the load order's files on the next launch.
        for name in present {
            if seen.insert(name.clone()) {
                out.push(ModEntry {
                    path: mods_dir.join(&name),
                    name,
                    enabled: false,
                    unmanaged: false,
                });
            }
        }
        let trust = if list_lost {
            ListTrust::Suspect(
                "modlist.txt exists but could not be read (truncated, or permissions) - \
                 the load order it holds is unknown"
                    .to_string(),
            )
        } else {
            ListTrust::judge(readable, listed, out.len())
        };
        // `out` is built highest-priority-first (file order). MO2 DISPLAYS the list
        // the other way up - lowest priority at the top - so reverse for the return.
        // (This preserves every entry's priority; only the vec orientation flips.)
        out.reverse();
        (out, trust)
    }

    /// Persist this profile's mod list (`+Name` enabled, `-Name` disabled).
    ///
    /// Written atomically (MO2's `SafeWriteFile`/`QSaveFile`): the content goes to
    /// a `.tmp` sibling in the *same* profile directory and is then `rename()`d over
    /// `modlist.txt`, an atomic swap within one filesystem. A crash or ENOSPC
    /// mid-write thus leaves the previous `modlist.txt` intact instead of an
    /// empty/partial file - which [`Profile::modlist`] would otherwise rebuild as
    /// "everything enabled, alphabetical", destroying the curated order. The
    /// previous list is also copied one-deep to `modlist.txt.bak` first.
    pub fn save_modlist(&self, mods: &[ModEntry]) -> io::Result<()> {
        if let Some(why) = self.unsafe_to_persist() {
            // Loud and non-destructive: the caller surfaces this and the curated
            // order stays on disk untouched.
            return Err(io::Error::new(io::ErrorKind::InvalidData, why));
        }
        fs::create_dir_all(self.dir())?;
        let mut s = String::new();
        // `mods` is in MO2 DISPLAY order (lowest priority first); the file stores
        // highest priority first (MO2's on-disk convention), so write it reversed.
        //
        // Unmanaged entries - the game's own DLCs and Creation Club content - are
        // written with MO2's `*`, which says "this row has a position, but Eidos
        // does not own the files". Dropping them, as this did, cost the user the
        // one thing a row needs: somewhere to be. They were re-discovered from the
        // game's data directory on every refresh and pinned to the top, so no
        // separator could ever sit above them and the block could not be collapsed.
        //
        // Nothing else changes: `load_order` filters `!unmanaged`, so a `*` row is
        // never mounted, and reconciliation against the game directory drops a line
        // whose content is gone.
        for m in mods.iter().rev() {
            s.push(if m.unmanaged {
                '*'
            } else if m.enabled {
                '+'
            } else {
                '-'
            });
            s.push_str(&m.name);
            s.push('\n');
        }

        let target = self.modlist_path();
        // Keep a one-deep backup of the previous list before swapping it out.
        if target.exists() {
            let _ = fs::copy(&target, target.with_extension("txt.bak"));
        }
        // Through the shared writer, whose temp name is unique per process: the
        // window and `eidos install` both write this file and neither serialises
        // against the other (flock is advisory and the CLI does not take it), so
        // a fixed temp name let two of them splice the curated order.
        crate::write_atomic(&target, s.as_bytes())
    }

    /// Why writing `modlist.txt` right now would destroy the curated order rather
    /// than record an edit, or `None` when it is safe.
    ///
    /// The check is deliberately absolute rather than proportional, because the
    /// disaster is absolute: the mod pool is unreachable, so the in-memory list is
    /// missing EVERYTHING and any save flattens the order to nothing. A user who
    /// really did delete every mod hits this too and has to say so by removing
    /// `modlist.txt` themselves - an annoyance, weighed against permanently losing
    /// the one thing on disk that cannot be re-derived: which of forty overlapping
    /// mods wins each file conflict, and which are installed but deliberately off.
    ///
    /// MO2 has no equivalent. `Profile::refreshModStatus` rewrites the file inside
    /// the same refresh that dropped the entries, and the guard that looks like
    /// protection (`if (m_ModStatus.empty()) return;`) cannot fire because the
    /// synthetic overwrite entry always makes the count non-zero.
    fn unsafe_to_persist(&self) -> Option<String> {
        let listed = match fs::read_to_string(self.modlist_source()) {
            Ok(text) => text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .count(),
            // No list yet: nothing to lose, and this is how a fresh instance starts.
            Err(_) => return None,
        };
        if listed == 0 {
            return None;
        }
        match fs::read_dir(self.mods_dir()) {
            Err(e) => Some(format!(
                "the mods folder could not be read ({e}); refusing to overwrite a list of \
                 {listed} mod(s). Is the drive it lives on mounted?"
            )),
            Ok(rd) => {
                let any = rd
                    .flatten()
                    .any(|e| e.file_type().is_ok_and(|t| t.is_dir() || t.is_symlink()));
                (!any).then(|| {
                    format!(
                        "the mods folder is empty while the saved list has {listed} mod(s); \
                         refusing to overwrite it. If the mods really are gone, delete \
                         {} to start over.",
                        self.modlist_path().display()
                    )
                })
            }
        }
    }
}
