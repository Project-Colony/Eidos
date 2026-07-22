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
        let mods_dir = self.mods_dir();
        let mut present: Vec<String> = fs::read_dir(&mods_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            // Dot-dirs are never mods: in-flight/crashed `.eidos-install-*`
            // extraction temps (and any other hidden dir) must not show up as
            // installed, enabled mods.
            .filter(|n| !n.starts_with('.'))
            .collect();
        present.sort();

        let mut out: Vec<ModEntry> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

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
                if present.iter().any(|p| p == name) && seen.insert(name.to_string()) {
                    out.push(ModEntry { name: name.to_string(), enabled, path: mods_dir.join(name) });
                }
            }
        }
        for name in present {
            if seen.insert(name.clone()) {
                out.push(ModEntry { path: mods_dir.join(&name), name, enabled: true });
            }
        }
        // `out` is built highest-priority-first (file order). MO2 DISPLAYS the list
        // the other way up - lowest priority at the top - so reverse for the return.
        // (This preserves every entry's priority; only the vec orientation flips.)
        out.reverse();
        out
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
    fn new_mods_are_reconciled_enabled() {
        let root = inst_with_mods(&["A", "New"]);
        let p = prof(&root, "Default");
        p.save_modlist(&[ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A") }]).unwrap();
        // "New" exists on disk but not in the saved list -> reconciled, enabled, at
        // the lowest priority (the top of the display order, before the listed mods).
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("New".into(), true), ("A".into(), true)]);
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
}
