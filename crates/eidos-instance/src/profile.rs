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

/// One profile of an instance.
#[derive(Debug, Clone)]
pub struct Profile {
    pub instance_root: PathBuf,
    pub name: String,
}

/// One save file in a profile's `saves/` directory (MO2's savegame list).
#[derive(Debug, Clone)]
pub struct SaveEntry {
    /// The file's name (e.g. `Save1_quicksave.ess`).
    pub filename: String,
    /// The absolute path on disk.
    pub path: PathBuf,
    /// Size in bytes.
    pub size: u64,
    /// Last-modified time (used as the in-game date proxy).
    pub mtime: std::time::SystemTime,
}

impl Profile {
    /// `<instance>/profiles/<name>/`.
    pub fn dir(&self) -> PathBuf {
        self.instance_root.join("profiles").join(&self.name)
    }

    /// The shared mod pool (instance-wide, not per-profile).
    fn mods_dir(&self) -> PathBuf {
        self.instance_root.join("mods")
    }

    fn modlist_path(&self) -> PathBuf {
        self.dir().join("modlist.txt")
    }

    /// Reserved per-profile plugin-order files (written at launch by
    /// eidos-plugins; kept here so each profile remembers its own order).
    pub fn plugins_txt_path(&self) -> PathBuf {
        self.dir().join("plugins.txt")
    }

    /// This profile's stored load order (`loadorder.txt`), the companion to
    /// [`Self::plugins_txt_path`] that records where INACTIVE plugins sit.
    pub fn loadorder_txt_path(&self) -> PathBuf {
        self.dir().join("loadorder.txt")
    }

    /// The two files that make up this profile's plugin state.
    fn plugin_state_files(&self) -> [(PathBuf, &'static str); 2] {
        [
            (self.plugins_txt_path(), "plugins.txt"),
            (self.loadorder_txt_path(), "loadorder.txt"),
        ]
    }

    /// Whether this profile already owns a plugin state (so it should drive the
    /// load order rather than the prefix's copy).
    pub fn has_plugin_state(&self) -> bool {
        self.plugins_txt_path().is_file()
    }

    /// One-time migration, mirroring [`Self::seed_inis`]: adopt the prefix's
    /// existing `plugins.txt`/`loadorder.txt` (`src_dir` = where the game reads
    /// them) into this profile, without overwriting a state the profile already
    /// owns. Returns how many files were seeded.
    pub fn seed_plugin_state(&self, src_dir: &Path) -> io::Result<u32> {
        fs::create_dir_all(self.dir())?;
        let mut n = 0;
        for (dst, name) in self.plugin_state_files() {
            let Some(src) = eidos_plugins::newest_variant(src_dir, name) else { continue };
            if !dst.exists() {
                fs::copy(&src, &dst)?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Deploy this profile's plugin state into `dst_dir` before launch, so the
    /// game (and Eidos's own pre-launch pass) sees THIS profile's load order.
    /// Returns how many files were deployed.
    pub fn deploy_plugin_state(&self, dst_dir: &Path) -> io::Result<u32> {
        fs::create_dir_all(dst_dir)?;
        let mut n = 0;
        for (src, name) in self.plugin_state_files() {
            if src.is_file() {
                // Write to the casing the prefix already uses, collapsing any
                // variants: the game is on a case-sensitive filesystem but came
                // from a case-insensitive one, and would happily read a
                // `Plugins.txt` we did not write.
                fs::copy(&src, eidos_plugins::canonical_path(dst_dir, name))?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Capture the plugin state back from `src_dir` after the game exits: Skyrim
    /// rewrites `plugins.txt` itself, and those changes belong to the profile that
    /// was played. Returns how many files were captured.
    pub fn capture_plugin_state(&self, src_dir: &Path) -> io::Result<u32> {
        fs::create_dir_all(self.dir())?;
        let mut n = 0;
        for (dst, name) in self.plugin_state_files() {
            // Read whichever spelling the game actually wrote last.
            let Some(src) = eidos_plugins::newest_variant(src_dir, name) else { continue };
            // A game that crashed during shutdown rewrites plugins.txt with the
            // active set partially cleared. Copying that back permanently
            // destroys a load order the user may have spent hours on, so a
            // capture that loses most of the actives is refused rather than
            // trusted. Same spirit as the "never write an empty list" guard on
            // the write side.
            if name == "plugins.txt" {
                if let Some(reason) = active_loss(&dst, &src) {
                    eprintln!(
                        "eidos: NOT capturing plugins.txt back into profile '{}': {reason}. \
                         The profile keeps its own copy; the prefix file is left alone.",
                        self.name
                    );
                    continue;
                }
            }
            fs::copy(&src, &dst)?;
            n += 1;
        }
        Ok(n)
    }

    /// This profile's stored copy of a game INI (e.g. `Skyrim.ini`). The profile
    /// owns its INIs; they are deployed into the Proton prefix at launch.
    pub fn ini_path(&self, ini_file: &str) -> PathBuf {
        self.dir().join(ini_file)
    }

    /// One-time migration: copy the user's existing prefix INIs (`src_dir` = the
    /// prefix `Documents/My Games/<game>`) into this profile, but only those the
    /// profile doesn't already own - so an existing setup is adopted, not lost.
    /// Returns how many were seeded.
    ///
    /// Divergence from MO2: MO2 seeds a new profile's INIs from the *vanilla game
    /// folder* (a clean baseline - it owns the INIs from the start). Eidos adopts a
    /// pre-existing Proton setup, so we seed from the user's *current* prefix INIs
    /// to keep their working settings (resolution, language, tweaks). Like MO2 we
    /// never overwrite an INI the profile already has.
    pub fn seed_inis(&self, src_dir: &Path, ini_files: &[&str]) -> io::Result<u32> {
        fs::create_dir_all(self.dir())?;
        let mut n = 0;
        for f in ini_files {
            let dst = self.ini_path(f);
            let src = src_dir.join(f);
            if !dst.exists() && src.is_file() {
                fs::copy(&src, &dst)?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Deploy this profile's INIs into `dst_dir` (the prefix Documents) before
    /// launch, so the game reads this profile's settings. Only INIs the profile
    /// actually has are written. Returns how many were deployed.
    pub fn deploy_inis(&self, dst_dir: &Path, ini_files: &[&str]) -> io::Result<u32> {
        fs::create_dir_all(dst_dir)?;
        let mut n = 0;
        for f in ini_files {
            let src = self.ini_path(f);
            if src.is_file() {
                fs::copy(&src, dst_dir.join(f))?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Capture the (game-modified) INIs from `src_dir` back into the profile after
    /// the game exits, so in-game settings changes persist to the profile. Returns
    /// how many were captured.
    pub fn capture_inis(&self, src_dir: &Path, ini_files: &[&str]) -> io::Result<u32> {
        fs::create_dir_all(self.dir())?;
        let mut n = 0;
        for f in ini_files {
            let src = src_dir.join(f);
            if src.is_file() {
                fs::copy(&src, self.ini_path(f))?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// The profile's own INI tweak file, applied after every mod's fragments so
    /// the user always has the last word (MO2's `getProfileTweaks`). Optional: a
    /// profile without one simply contributes nothing.
    pub fn tweaks_path(&self) -> PathBuf {
        self.dir().join("initweaks.ini")
    }

    /// Merge INI-tweak fragments into a deployed game INI, in the order given
    /// (later wins), and return what each write displaced so [`untweak_ini`] can
    /// put it back after the run.
    ///
    /// `fragments` are the enabled `INI Tweaks/*.ini` files in mod priority order,
    /// lowest first; the profile's own tweak file, if any, is applied last.
    /// A fragment that cannot be read is skipped rather than failing the launch -
    /// a missing tweak must not stop the game from starting.
    pub fn apply_ini_tweaks(
        &self,
        deployed_ini: &Path,
        fragments: &[PathBuf],
    ) -> io::Result<Vec<TweakedKey>> {
        let mut record: Vec<TweakedKey> = Vec::new();
        let mut text = fs::read_to_string(deployed_ini).unwrap_or_default();
        let mut any = false;
        for frag in fragments.iter().chain(std::iter::once(&self.tweaks_path())) {
            let Ok(body) = fs::read_to_string(frag) else { continue };
            any |= merge_tweak(&mut text, &body, &mut record);
        }
        if any {
            if let Some(parent) = deployed_ini.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(deployed_ini, &text)?;
        }
        Ok(record)
    }

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
        let has_saves = fs::read_dir(&dst).map(|mut it| it.next().is_some()).unwrap_or(false);
        if has_saves {
            return Ok(0);
        }
        let Ok(rd) = fs::read_dir(src_saves) else {
            return Ok(0);
        };
        fs::create_dir_all(&dst)?;
        let mut n = 0;
        for e in rd.flatten() {
            if e.path().is_file() {
                fs::copy(e.path(), dst.join(e.file_name()))?;
                n += 1;
            }
        }
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
                if filename.starts_with('.') {
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
        out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        out
    }

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
        Ok(Profile { instance_root: self.instance_root.clone(), name: new_name.to_string() })
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

        if let Ok(content) = fs::read_to_string(self.modlist_source()) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let (enabled, name) = if let Some(n) = line.strip_prefix('+') {
                    (true, n.trim())
                } else if let Some(n) = line.strip_prefix('-') {
                    (false, n.trim())
                } else if let Some(n) = line.strip_prefix('*') {
                    // MO2 marks unmanaged/foreign mods with '*'; Eidos does not
                    // model foreign mods, so treat the line as an enabled entry.
                    (true, n.trim())
                } else {
                    (true, line)
                };
                listed += 1;
                if present.iter().any(|p| p == name) && seen.insert(name.to_string()) {
                    out.push(ModEntry { name: name.to_string(), enabled, path: mods_dir.join(name) });
                }
            }
        }
        // A folder nobody listed: a mod dropped in by hand. MO2 appends it at the
        // highest priority and leaves it DISABLED - it has no idea where in the
        // conflict order it belongs, and enabling it silently could overwrite half
        // the load order's files on the next launch.
        for name in present {
            if seen.insert(name.clone()) {
                out.push(ModEntry { path: mods_dir.join(&name), name, enabled: false });
            }
        }
        let trust = ListTrust::judge(readable, listed, out.len());
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
        for m in mods.iter().rev() {
            s.push(if m.enabled { '+' } else { '-' });
            s.push_str(&m.name);
            s.push('\n');
        }

        let target = self.modlist_path();
        // Keep a one-deep backup of the previous list before swapping it out.
        if target.exists() {
            let _ = fs::copy(&target, target.with_extension("txt.bak"));
        }
        // Write to a temp file in the same directory, then atomically rename it over
        // the target so a partial write can never clobber the curated order.
        let tmp = target.with_extension("txt.tmp");
        fs::write(&tmp, s)?;
        match fs::rename(&tmp, &target) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    }

    /// Enabled mods, highest priority first: the layers to mount at launch.
    /// Separators are group dividers, not content, so they are never mounted.
    ///
    /// `modlist()` returns MO2 display order (lowest priority first), so reverse to
    /// the highest-priority-first order the union mount expects (layer 0 wins).
    pub fn load_order(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self
            .modlist()
            .into_iter()
            .filter(|m| m.enabled && !m.is_separator())
            .map(|m| m.path)
            .collect();
        v.reverse();
        v
    }
}

/// A list smaller than this is never second-guessed - losing two of five entries
/// is an ordinary edit.
const MIN_ACTIVES: usize = 10;
/// Both thresholds must be crossed for a loss to look accidental rather than
/// deliberate. Shared by the plugin-order capture and the mod-list reconciliation
/// so "this looks like an accident" means one thing across the instance.
const MAX_ABSOLUTE_DROP: usize = 10;
const MAX_RELATIVE_DROP: f64 = 0.30;

/// Why a capture would lose too much of the active set to be trusted, or `None`
/// when it looks like a legitimate edit.
///
/// Only fires on a LARGE loss from an already-large list: dropping a couple of
/// plugins is exactly what a user does on purpose, while dropping most of a
/// 200-plugin order is what a half-written crash artefact looks like. Both an
/// absolute and a relative threshold must be crossed, so small lists are never
/// second-guessed.
fn active_loss(profile: &Path, candidate: &Path) -> Option<String> {
    let before = count_actives(profile)?;
    let after = count_actives(candidate)?;
    if before <= MIN_ACTIVES || after >= before {
        return None;
    }
    let dropped = before - after;
    let relative = dropped as f64 / before as f64;
    (dropped > MAX_ABSOLUTE_DROP && relative > MAX_RELATIVE_DROP).then(|| {
        format!("it drops {dropped} of {before} active plugins ({:.0}%)", relative * 100.0)
    })
}

/// Active (`*`-prefixed) entries in a plugins.txt, or `None` if unreadable.
fn count_actives(path: &Path) -> Option<usize> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.lines().filter(|l| l.trim_start().starts_with('*')).count())
}

/// Recursively copy a directory tree, skipping symlinks (matching MO2's `copyDir`
/// NoSymLinks). Best-effort per entry.
fn copy_dir_recursive(from: &Path, to: &Path) -> io::Result<()> {
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

impl Profile {
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

/// Prefix of Eidos's own extraction temporaries under `mods/`. Only directories
/// starting with this are hidden from the mod list; every other name, leading dot
/// included, is a mod the user installed.
const INSTALL_TEMP_PREFIX: &str = ".eidos-install-";

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

/// One key an INI tweak overwrote, kept so the post-run capture can restore the
/// profile's own value instead of adopting the tweak permanently.
///
/// Without this, tweaks are a one-way door: the launch writes them into the
/// deployed INI, `capture_inis` copies that file back into the profile, and by
/// the second launch the tweak is indistinguishable from a setting the user
/// chose. Disabling the fragment would then change nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TweakedKey {
    pub section: String,
    pub key: String,
    /// The value before any tweak touched it; `None` if the key was absent.
    pub before: Option<String>,
    /// What the last fragment wrote, so the capture can tell a value that is
    /// still ours from one the game or the user changed while running.
    pub after: String,
}

/// Apply one INI fragment to `text`, recording what it displaced. Returns whether
/// anything was written.
///
/// A deliberately dumb line parser, matching MO2's `mergeTweak` (profile.cpp:778):
/// blanks and `;` / `#` comments are skipped, `[Section]` sets the current
/// section, and a line is split on its FIRST `=` with both sides trimmed. Values
/// therefore may contain `=` and nothing a fragment says can corrupt the target -
/// the worst a malformed fragment can do is set nothing.
///
/// Keys outside any section are dropped: an INI's leading keys belong to no
/// section, and guessing one would write the tweak somewhere the engine never
/// reads.
fn merge_tweak(text: &mut String, fragment: &str, record: &mut Vec<TweakedKey>) -> bool {
    let mut section = String::new();
    let mut wrote = false;
    for line in fragment.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(s) = eidos_ini::section_header(line) {
            section = s.to_string();
            continue;
        }
        if section.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() {
            continue;
        }
        let current = eidos_ini::get_key(text, &section, key).map(|v| v.trim().to_string());
        // Only the FIRST fragment to touch a key records the original value; a
        // later one overwriting it must not record the earlier tweak as "before".
        match record
            .iter_mut()
            .find(|r| r.section.eq_ignore_ascii_case(&section) && r.key.eq_ignore_ascii_case(key))
        {
            Some(existing) => existing.after = value.to_string(),
            None => record.push(TweakedKey {
                section: section.clone(),
                key: key.to_string(),
                before: current,
                after: value.to_string(),
            }),
        }
        *text = eidos_ini::set_key(text, &section, key, value);
        wrote = true;
    }
    wrote
}

/// Undo what [`Profile::apply_ini_tweaks`] wrote, for the INI text captured back
/// from the prefix after a run.
///
/// A key is restored only if it still holds exactly what the tweak wrote. If the
/// game or the user changed it in-flight that is a real preference change and it
/// is kept - the tweak lost, which is the same rule MO2's "the user always wins"
/// ordering encodes at merge time.
pub fn untweak_ini(text: &str, record: &[TweakedKey]) -> String {
    let mut out = text.to_string();
    for r in record {
        let current = eidos_ini::get_key(&out, &r.section, &r.key).map(|v| v.trim().to_string());
        if current.as_deref() != Some(r.after.as_str()) {
            continue;
        }
        out = match &r.before {
            Some(v) => eidos_ini::set_key(&out, &r.section, &r.key, v),
            None => eidos_ini::delete_key(&out, &r.section, &r.key),
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn inst_with_mods(mods: &[&str]) -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("eidos-prof-{}-{}", std::process::id(), n));
        for m in mods {
            fs::create_dir_all(root.join("mods").join(m)).unwrap();
        }
        root
    }

    fn prof(root: &Path, name: &str) -> Profile {
        Profile { instance_root: root.to_path_buf(), name: name.to_string() }
    }

    #[test]
    fn later_fragments_win_and_the_original_value_is_what_gets_restored() {
        let mut ini = "[Display]\nfDefaultFOV=75.0\niSize W=1920\n".to_string();
        let mut rec = Vec::new();

        assert!(merge_tweak(&mut ini, "[Display]\nfDefaultFOV=90.0\n", &mut rec));
        // A second fragment overwrites the first; `before` must still be vanilla,
        // or disabling both would leave the user on the first tweak's value.
        assert!(merge_tweak(&mut ini, "[Display]\nfDefaultFOV = 110.0\n", &mut rec));
        assert_eq!(eidos_ini::get_key(&ini, "Display", "fDefaultFOV"), Some("110.0"));
        assert_eq!(rec.len(), 1);
        assert_eq!(rec[0].before.as_deref(), Some("75.0"));
        assert_eq!(rec[0].after, "110.0");

        let restored = untweak_ini(&ini, &rec);
        assert_eq!(eidos_ini::get_key(&restored, "Display", "fDefaultFOV"), Some("75.0"));
        assert_eq!(eidos_ini::get_key(&restored, "Display", "iSize W"), Some("1920"));
    }

    #[test]
    fn a_key_the_game_changed_in_flight_keeps_its_new_value() {
        let mut ini = "[Display]\nfDefaultFOV=75.0\n".to_string();
        let mut rec = Vec::new();
        merge_tweak(&mut ini, "[Display]\nfDefaultFOV=90.0\n", &mut rec);
        // The user moved the FOV slider in-game, so the captured INI no longer
        // holds what the tweak wrote. Their choice wins over the restore.
        let captured = eidos_ini::set_key(&ini, "Display", "fDefaultFOV", "100.0");
        let restored = untweak_ini(&captured, &rec);
        assert_eq!(eidos_ini::get_key(&restored, "Display", "fDefaultFOV"), Some("100.0"));
    }

    #[test]
    fn a_key_the_tweak_invented_is_deleted_again_not_blanked() {
        let mut ini = "[Display]\niSize W=1920\n".to_string();
        let mut rec = Vec::new();
        merge_tweak(&mut ini, "[Papyrus]\nbEnableLogging=1\n", &mut rec);
        assert_eq!(rec[0].before, None);
        let restored = untweak_ini(&ini, &rec);
        // Absent, not `bEnableLogging=`: the engines read those differently.
        assert_eq!(eidos_ini::get_key(&restored, "Papyrus", "bEnableLogging"), None);
        assert!(restored.contains("[Papyrus]"));
    }

    #[test]
    fn a_fragment_cannot_corrupt_the_target() {
        let mut ini = "[Display]\niSize W=1920\n".to_string();
        let mut rec = Vec::new();
        let junk = concat!(
            "; a comment\n",
            "# another\n",
            "\n",
            "strayKey=1\n",             // outside any section: dropped
            "[[not a header\n",         // not a section either
            "[General]\n",
            "sTestFile1 = a=b=c\n",     // value keeps its own '='
            "=novalue\n",               // empty key: dropped
            "no equals sign at all\n",
        );
        merge_tweak(&mut ini, junk, &mut rec);
        assert_eq!(eidos_ini::get_key(&ini, "General", "sTestFile1"), Some("a=b=c"));
        assert_eq!(rec.len(), 1);
        // The pre-existing key survived untouched.
        assert_eq!(eidos_ini::get_key(&ini, "Display", "iSize W"), Some("1920"));
    }

    #[test]
    fn the_profile_tweak_file_is_applied_after_every_mod() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        fs::write(p.tweaks_path(), "[Display]\nfDefaultFOV=100.0\n").unwrap();

        let frag = root.join("frag.ini");
        fs::write(&frag, "[Display]\nfDefaultFOV=90.0\n").unwrap();
        let deployed = root.join("Skyrim.ini");
        fs::write(&deployed, "[Display]\nfDefaultFOV=75.0\n").unwrap();

        let rec = p.apply_ini_tweaks(&deployed, &[frag]).unwrap();
        let text = fs::read_to_string(&deployed).unwrap();
        // The profile's own file is last, so the user beats the mod.
        assert_eq!(eidos_ini::get_key(&text, "Display", "fDefaultFOV"), Some("100.0"));
        assert_eq!(rec[0].before.as_deref(), Some("75.0"));
        assert_eq!(rec[0].after, "100.0");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn modlist_round_trips_per_profile() {
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        let mods = vec![
            ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B") },
            ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A") },
            ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C") },
        ];
        p.save_modlist(&mods).unwrap();
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("B".into(), true), ("A".into(), false), ("C".into(), true)]);

        // The atomic write must leave no stray ".tmp" sibling behind.
        let leftover_tmp = fs::read_dir(p.dir())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().ends_with(".tmp"));
        assert!(!leftover_tmp, "save_modlist left a leftover .tmp file in the profile dir");

        let _ = fs::remove_dir_all(&root);
    }

    /// The curated order must survive a destroyed/partial `modlist.txt`: because
    /// the write is atomic (temp file then rename), a save never leaves an empty
    /// file that [`Profile::modlist`] would rebuild as "everything enabled,
    /// alphabetical". Guards FIX F1 (MO2 `SafeWriteFile`/`QSaveFile` parity).
    #[test]
    fn save_modlist_is_atomic_and_keeps_a_backup() {
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        let v1 = vec![
            ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C") },
            ModEntry { name: "B".into(), enabled: false, path: root.join("mods/B") },
            ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A") },
        ];
        p.save_modlist(&v1).unwrap();

        // A second save (a toggle/move) over an existing list: backs the old one
        // up and swaps atomically.
        let v2 = vec![
            ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A") },
            ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C") },
            ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B") },
        ];
        p.save_modlist(&v2).unwrap();

        // The live file reflects the latest curated order (not the alphabetical default).
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("A".into(), false), ("C".into(), true), ("B".into(), true)]);

        // The one-deep backup holds the previous list and sits in the same dir.
        // The file stores highest-priority first (reverse of the in-memory v1).
        let bak = p.dir().join("modlist.txt.bak");
        assert!(bak.is_file(), "expected a one-deep modlist.txt.bak backup");
        assert_eq!(fs::read_to_string(&bak).unwrap(), "+A\n-B\n+C\n");

        // No temp file lingers after a successful save.
        assert!(!p.dir().join("modlist.txt.tmp").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_from_copies_saves_subdir() {
        let root = inst_with_mods(&["A"]);
        let src = prof(&root, "Src");
        src.create().unwrap();
        // A save in the source profile's saves/ subdir + a curated modlist.
        let saves = src.dir().join("saves");
        fs::create_dir_all(&saves).unwrap();
        fs::write(saves.join("Save1.ess"), b"x").unwrap();
        src.save_modlist(&[ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A") }])
            .unwrap();

        let dst = prof(&root, "Copy");
        dst.create_from(&src).unwrap();
        // The saves/ subdir is copied recursively (MO2 parity), not skipped...
        assert!(dst.dir().join("saves/Save1.ess").is_file());
        // ...and the modlist file came across too.
        assert!(!dst.modlist().iter().find(|m| m.name == "A").unwrap().enabled);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn modlist_parses_star_and_trims_names() {
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        // MO2 '*' foreign line (enabled), and +/- with padding that must be trimmed.
        // The file is highest-priority first; modlist() returns it reversed (display order).
        fs::write(p.modlist_path(), "*A\n-  B\n+ C \n").unwrap();
        let got: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(got, vec![("C".into(), true), ("B".into(), false), ("A".into(), true)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn two_profiles_share_mods_but_keep_own_order() {
        let root = inst_with_mods(&["A", "B"]);
        prof(&root, "Default")
            .save_modlist(&[
                ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A") },
                ModEntry { name: "B".into(), enabled: false, path: root.join("mods/B") },
            ])
            .unwrap();
        prof(&root, "Test")
            .save_modlist(&[
                ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B") },
                ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A") },
            ])
            .unwrap();
        let d: Vec<_> = prof(&root, "Default").modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        let t: Vec<_> = prof(&root, "Test").modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(d, vec![("A".into(), true), ("B".into(), false)]);
        assert_eq!(t, vec![("B".into(), true), ("A".into(), true)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn default_profile_falls_back_to_legacy_flat_modlist() {
        let root = inst_with_mods(&["A", "B"]);
        // A pre-profiles instance: a flat <root>/modlist.txt (highest-priority first).
        fs::write(root.join("modlist.txt"), "-A\n+B\n").unwrap();
        let p = prof(&root, "Default");
        // modlist() returns display order (reverse of the file): B (top) then A.
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("B".into(), true), ("A".into(), false)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_nobody_listed_appears_disabled() {
        let root = inst_with_mods(&["A", "New"]);
        let p = prof(&root, "Default");
        p.save_modlist(&[ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A") }]).unwrap();
        // "New" exists on disk but not in the saved list. It appears, but DISABLED
        // (MO2 parity): nothing knows where in the conflict order it belongs, and
        // silently enabling it could overwrite half the load order's files on the
        // next launch. A mod installed THROUGH Eidos never takes this path - the
        // installer writes its own modlist entry.
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("New".into(), false), ("A".into(), true)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_whose_folder_is_gone_leaves_the_list_but_not_the_file() {
        let root = inst_with_mods(&["A", "B"]);
        let p = prof(&root, "Default");
        let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n) };
        p.save_modlist(&[e("A"), e("B")]).unwrap();

        fs::remove_dir_all(root.join("mods/B")).unwrap();
        let (list, trust) = p.modlist_checked();
        assert_eq!(list.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["A"]);
        // One of two gone is an ordinary edit, not an accident.
        assert!(trust.is_good(), "{trust:?}");
        // The file still says both until something saves - the drop is a view.
        assert!(fs::read_to_string(p.modlist_path()).unwrap().contains("B"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unmounted_mods_folder_cannot_flatten_the_order() {
        // The disaster case: mods/ lives on another drive via a bind mount, and the
        // mount is not up. The directory EXISTS and is READABLE and is EMPTY, so
        // every guard that only checks for existence sails straight through.
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n) };
        p.save_modlist(&[e("A"), e("B"), e("C")]).unwrap();
        let before = fs::read_to_string(p.modlist_path()).unwrap();

        for m in ["A", "B", "C"] {
            fs::remove_dir_all(root.join("mods").join(m)).unwrap();
        }
        let (list, trust) = p.modlist_checked();
        assert!(list.is_empty());
        assert!(!trust.is_good(), "an empty scan against a non-empty list must not be trusted");

        // And the save is refused rather than silently flattening the order.
        let err = p.save_modlist(&[]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read_to_string(p.modlist_path()).unwrap(), before);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unreadable_mods_folder_is_not_an_empty_one() {
        use std::os::unix::fs::PermissionsExt;
        let root = inst_with_mods(&["A", "B"]);
        let p = prof(&root, "Default");
        let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n) };
        p.save_modlist(&[e("A"), e("B")]).unwrap();

        let mods = root.join("mods");
        fs::set_permissions(&mods, fs::Permissions::from_mode(0o000)).unwrap();
        let (_, trust) = p.modlist_checked();
        let refused = p.save_modlist(&[]).is_err();
        // Restore before asserting, so a failure does not leave an unremovable dir.
        fs::set_permissions(&mods, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(!trust.is_good(), "a read error must not read as 'you have no mods'");
        assert!(refused);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_whose_name_starts_with_a_dot_is_kept() {
        // ".NET Script Framework" is a real, near-universal Skyrim SE dependency.
        // Only Eidos's own extraction temps are hidden from the list.
        let root = inst_with_mods(&[".NET Script Framework", ".eidos-install-abc123", "A"]);
        let p = prof(&root, "Default");
        let names: Vec<String> = p.modlist().iter().map(|m| m.name.clone()).collect();
        assert!(names.iter().any(|n| n == ".NET Script Framework"), "{names:?}");
        assert!(!names.iter().any(|n| n.starts_with(".eidos-install-")), "{names:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_dangling_symlink_is_still_a_row() {
        // A mod symlinked to a drive that is not mounted is BROKEN, not absent: its
        // position, enabled state and intended target are the irreplaceable part,
        // and dropping the row throws them away. `path().is_dir()` follows the link
        // and cannot tell this from a deleted mod; `file_type()` can.
        let root = inst_with_mods(&["A"]);
        std::os::unix::fs::symlink(root.join("nowhere"), root.join("mods/Linked")).unwrap();
        let p = prof(&root, "Default");
        let names: Vec<String> = p.modlist().iter().map(|m| m.name.clone()).collect();
        assert!(names.iter().any(|n| n == "Linked"), "{names:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn inis_seed_deploy_and_capture_round_trip() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        // A fake prefix Documents dir holding the user's existing INIs.
        let prefix = root.join("prefix-docs");
        fs::create_dir_all(&prefix).unwrap();
        fs::write(prefix.join("Skyrim.ini"), "[General]\nsLanguage=ENGLISH\n").unwrap();
        fs::write(prefix.join("SkyrimPrefs.ini"), "[Display]\niSize W=1920\n").unwrap();
        let inis = ["Skyrim.ini", "SkyrimPrefs.ini"];

        // Seed adopts both into the profile; seeding again copies nothing.
        assert_eq!(p.seed_inis(&prefix, &inis).unwrap(), 2);
        assert!(p.ini_path("Skyrim.ini").is_file());
        assert_eq!(p.seed_inis(&prefix, &inis).unwrap(), 0);

        // The profile is now the source of truth: edit its copy, deploy elsewhere.
        fs::write(p.ini_path("Skyrim.ini"), "[General]\nsLanguage=FRENCH\n").unwrap();
        let prefix2 = root.join("prefix2");
        assert_eq!(p.deploy_inis(&prefix2, &inis).unwrap(), 2);
        assert!(fs::read_to_string(prefix2.join("Skyrim.ini")).unwrap().contains("FRENCH"));

        // The game writes to the prefix; capture pulls the change back.
        fs::write(prefix2.join("SkyrimPrefs.ini"), "[Display]\niSize W=2560\n").unwrap();
        assert_eq!(p.capture_inis(&prefix2, &inis).unwrap(), 2);
        assert!(fs::read_to_string(p.ini_path("SkyrimPrefs.ini")).unwrap().contains("2560"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn saves_seed_adopts_existing_then_skips() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        let prefix_saves = root.join("prefix-saves");
        fs::create_dir_all(&prefix_saves).unwrap();
        fs::write(prefix_saves.join("Save1.ess"), b"x").unwrap();
        fs::write(prefix_saves.join("Save2.ess"), b"y").unwrap();

        // First run adopts the existing playthrough.
        assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 2);
        assert!(p.saves_dir().join("Save1.ess").is_file());
        // Profile already has saves -> never re-seed (would clobber progress).
        fs::write(prefix_saves.join("Save3.ess"), b"z").unwrap();
        assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn savegames_lists_files_newest_first_and_skips_dirs() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        let saves = p.saves_dir();
        fs::create_dir_all(&saves).unwrap();
        // Write `Old` first, then sleep so `New` gets a strictly later mtime.
        fs::write(saves.join("Old.ess"), b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(saves.join("New.ess"), b"newer-and-bigger").unwrap();
        // A subdirectory and a dotfile are both ignored.
        fs::create_dir_all(saves.join("backup")).unwrap();
        fs::write(saves.join(".DS_Store"), b"junk").unwrap();

        let list = p.savegames();
        let names: Vec<_> = list.iter().map(|s| s.filename.clone()).collect();
        assert_eq!(names, vec!["New.ess".to_string(), "Old.ess".to_string()]);
        assert_eq!(list[0].size, "newer-and-bigger".len() as u64);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn savegames_is_empty_when_no_saves_dir() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        assert!(p.savegames().is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn display_order_file_order_and_load_order_stay_consistent() {
        // The orientation contract: modlist() is MO2 display order (lowest priority
        // at the top); the file stores highest-priority first; load_order() re-reverses
        // to highest-first for the mount. A regression here silently inverts conflicts.
        let root = inst_with_mods(&["Low", "High"]);
        let p = prof(&root, "Default");
        // Display order: Low at the top (lowest priority), High at the bottom (highest).
        p.save_modlist(&[
            ModEntry { name: "Low".into(), enabled: true, path: root.join("mods/Low") },
            ModEntry { name: "High".into(), enabled: true, path: root.join("mods/High") },
        ])
        .unwrap();
        // The file is highest-priority first (MO2 on-disk convention).
        assert_eq!(fs::read_to_string(p.dir().join("modlist.txt")).unwrap(), "+High\n+Low\n");
        // modlist() round-trips the display order.
        let names: Vec<_> = p.modlist().iter().map(|m| m.name.clone()).collect();
        assert_eq!(names, vec!["Low".to_string(), "High".to_string()]);
        // load_order() mounts highest priority first, so High wins same-name conflicts.
        assert_eq!(p.load_order(), vec![root.join("mods/High"), root.join("mods/Low")]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn separator_round_trips_keeps_position_and_is_excluded_from_load_order() {
        // A separator is a real `*_separator` folder; it must round-trip in place,
        // be recognised as a separator, and never become a mount layer.
        let root = inst_with_mods(&["A", "Sec_separator", "B"]);
        let p = prof(&root, "Default");
        let mods = vec![
            ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A") },
            ModEntry { name: "Sec_separator".into(), enabled: false, path: root.join("mods/Sec_separator") },
            ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B") },
        ];
        p.save_modlist(&mods).unwrap();

        // modlist.txt is byte-faithful, including the `-` prefix + `_separator` suffix,
        // and stored highest-priority first (reverse of the in-memory display order).
        assert_eq!(fs::read_to_string(p.dir().join("modlist.txt")).unwrap(), "+B\n-Sec_separator\n+A\n");

        // Read back: order + the separator flag preserved, separator at index 1.
        let read = p.modlist();
        let names: Vec<_> = read.iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(
            names,
            vec![("A".into(), true), ("Sec_separator".into(), false), ("B".into(), true)]
        );
        assert!(read[1].is_separator());
        assert_eq!(read[1].display_name(), "Sec");
        assert!(!read[0].is_separator());

        // load_order mounts only A and B - the separator is content-less - in
        // highest-priority-first order (B is below A in the display, so it wins).
        let order = p.load_order();
        assert_eq!(order, vec![root.join("mods/B"), root.join("mods/A")]);
        let _ = fs::remove_dir_all(&root);

        // An ENABLED separator (alone) still contributes no mount layer.
        let root2 = inst_with_mods(&["Solo_separator"]);
        let p2 = prof(&root2, "Default");
        p2.save_modlist(&[ModEntry {
            name: "Solo_separator".into(),
            enabled: true,
            path: root2.join("mods/Solo_separator"),
        }])
        .unwrap();
        assert!(p2.load_order().is_empty());
        let _ = fs::remove_dir_all(&root2);
    }

    #[test]
    fn a_crash_mangled_plugins_txt_is_not_captured_back() {
        let root = inst_with_mods(&["A"]);
        let prefix = root.join("prefix");
        fs::create_dir_all(&prefix).unwrap();
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();

        // A real 200-plugin order in the profile.
        let full: String = (0..200).map(|i| format!("*Mod{i}.esp\n")).collect();
        fs::write(p.plugins_txt_path(), &full).unwrap();
        // What a game that died during shutdown leaves behind: the active set
        // mostly cleared, the names still listed.
        let mangled: String = (0..200)
            .map(|i| if i < 5 { format!("*Mod{i}.esp\n") } else { format!("Mod{i}.esp\n") })
            .collect();
        fs::write(prefix.join("plugins.txt"), &mangled).unwrap();

        p.capture_plugin_state(&prefix).unwrap();
        assert_eq!(
            fs::read_to_string(p.plugins_txt_path()).unwrap(),
            full,
            "a crash artefact must not be allowed to destroy the load order"
        );

        // A legitimate edit - the user turning a handful of mods off - goes through.
        let edited: String = (0..200)
            .map(|i| if i < 195 { format!("*Mod{i}.esp\n") } else { format!("Mod{i}.esp\n") })
            .collect();
        fs::write(prefix.join("plugins.txt"), &edited).unwrap();
        p.capture_plugin_state(&prefix).unwrap();
        assert_eq!(fs::read_to_string(p.plugins_txt_path()).unwrap(), edited);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn small_load_orders_are_never_second_guessed() {
        // Turning off 4 of 6 plugins is a big RELATIVE drop but an obviously
        // deliberate one; the guard must not fire on lists this size.
        let root = inst_with_mods(&["A"]);
        let prefix = root.join("prefix");
        fs::create_dir_all(&prefix).unwrap();
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        fs::write(p.plugins_txt_path(), "*a\n*b\n*c\n*d\n*e\n*f\n").unwrap();
        fs::write(prefix.join("plugins.txt"), "*a\n*b\nc\nd\ne\nf\n").unwrap();

        p.capture_plugin_state(&prefix).unwrap();
        assert_eq!(fs::read_to_string(p.plugins_txt_path()).unwrap(), "*a\n*b\nc\nd\ne\nf\n");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn plugin_state_seeds_deploys_and_captures_per_profile() {
        let root = inst_with_mods(&["A"]);
        let prefix = root.join("prefix");
        fs::create_dir_all(&prefix).unwrap();
        fs::write(prefix.join("plugins.txt"), b"*Alpha.esp\nBeta.esp\n").unwrap();
        fs::write(prefix.join("loadorder.txt"), b"Alpha.esp\nBeta.esp\n").unwrap();

        // Seed: the profile adopts the prefix's existing order once.
        let a = prof(&root, "Default");
        assert!(!a.has_plugin_state());
        assert_eq!(a.seed_plugin_state(&prefix).unwrap(), 2);
        assert!(a.has_plugin_state());
        // Seeding again must not clobber the profile's own copy.
        fs::write(a.plugins_txt_path(), b"*Alpha.esp\n").unwrap();
        assert_eq!(a.seed_plugin_state(&prefix).unwrap(), 0);
        assert_eq!(fs::read(a.plugins_txt_path()).unwrap(), b"*Alpha.esp\n");

        // A second profile has its own, independent state.
        let b = prof(&root, "Testing");
        assert!(!b.has_plugin_state());
        fs::create_dir_all(b.dir()).unwrap();
        fs::write(b.plugins_txt_path(), b"*Beta.esp\n").unwrap();

        // Deploy: whichever profile is active drives what the game reads.
        assert_eq!(b.deploy_plugin_state(&prefix).unwrap(), 1);
        assert_eq!(fs::read(prefix.join("plugins.txt")).unwrap(), b"*Beta.esp\n");
        a.deploy_plugin_state(&prefix).unwrap();
        assert_eq!(fs::read(prefix.join("plugins.txt")).unwrap(), b"*Alpha.esp\n");

        // Capture: the game's own rewrite comes back into the played profile only.
        fs::write(prefix.join("plugins.txt"), b"*Alpha.esp\n*Beta.esp\n").unwrap();
        assert_eq!(a.capture_plugin_state(&prefix).unwrap(), 2);
        assert_eq!(fs::read(a.plugins_txt_path()).unwrap(), b"*Alpha.esp\n*Beta.esp\n");
        assert_eq!(fs::read(b.plugins_txt_path()).unwrap(), b"*Beta.esp\n");

        let _ = fs::remove_dir_all(&root);
    }
}
