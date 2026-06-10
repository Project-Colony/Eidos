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

    /// Create this profile by copying another profile's files (its modlist, and
    /// any plugin/INI files). Subdirectories (saves) are not copied.
    pub fn create_from(&self, other: &Profile) -> io::Result<()> {
        fs::create_dir_all(self.dir())?;
        if let Ok(rd) = fs::read_dir(other.dir()) {
            for e in rd.flatten() {
                if e.path().is_file() {
                    let _ = fs::copy(e.path(), self.dir().join(e.file_name()));
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

    /// The mod list for this profile: every folder in the shared `mods/`, in
    /// priority order with enabled state, reconciled with this profile's
    /// `modlist.txt`. Top of the list = highest priority.
    pub fn modlist(&self) -> Vec<ModEntry> {
        let mods_dir = self.mods_dir();
        let mut present: Vec<String> = fs::read_dir(&mods_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
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
                    (true, n)
                } else if let Some(n) = line.strip_prefix('-') {
                    (false, n)
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
        out
    }

    /// Persist this profile's mod list (`+Name` enabled, `-Name` disabled).
    pub fn save_modlist(&self, mods: &[ModEntry]) -> io::Result<()> {
        fs::create_dir_all(self.dir())?;
        let mut s = String::new();
        for m in mods {
            s.push(if m.enabled { '+' } else { '-' });
            s.push_str(&m.name);
            s.push('\n');
        }
        fs::write(self.modlist_path(), s)
    }

    /// Enabled mods, highest priority first: the layers to mount at launch.
    pub fn load_order(&self) -> Vec<PathBuf> {
        self.modlist().into_iter().filter(|m| m.enabled).map(|m| m.path).collect()
    }
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

    fn prof(root: &PathBuf, name: &str) -> Profile {
        Profile { instance_root: root.clone(), name: name.to_string() }
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
        // A pre-profiles instance: a flat <root>/modlist.txt.
        fs::write(root.join("modlist.txt"), "-A\n+B\n").unwrap();
        let p = prof(&root, "Default");
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("A".into(), false), ("B".into(), true)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn new_mods_are_appended_enabled() {
        let root = inst_with_mods(&["A", "New"]);
        let p = prof(&root, "Default");
        p.save_modlist(&[ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A") }]).unwrap();
        // "New" exists on disk but not in the saved list -> appended, enabled.
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("A".into(), true), ("New".into(), true)]);
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
}
