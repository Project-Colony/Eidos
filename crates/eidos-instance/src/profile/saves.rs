//! Per-profile saves: the directory, seeding, enumeration, co-saves.

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

use super::*;

/// The save-file extensions across the supported game families: Bethesda's
/// `.ess` (Skyrim LE/SE), `.fos` (Fallout 3/NV/4) and `.sfs` (Starfield).
pub(crate) const SAVE_EXTS: &[&str] = &["ess", "fos", "sfs"];

/// Script-extender co-saves that travel WITH a save: same stem, own extension.
pub(crate) const COSAVE_EXTS: &[&str] = &["skse", "f4se", "nvse", "fose", "sfse", "obse", "mwse"];

pub(crate) fn ext_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
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
    let Some(stem) = save
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
    else {
        return Vec::new();
    };
    let Some(dir) = save.parent() else {
        return Vec::new();
    };
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
            let Some((s, ext)) = name.rsplit_once('.') else {
                return false;
            };
            s == stem && COSAVE_EXTS.contains(&ext) && e.path().is_file()
        })
        .map(|e| e.path())
        .collect()
}

/// Every content key any profile of this instance has pushed to the prefix:
/// `prof_saves` is `<instance>/profiles/<name>/saves`, and all those profiles
/// bind over the same prefix Saves dir.
pub fn instance_sync_keys(prof_saves: &Path) -> HashSet<String> {
    let Some(profiles) = prof_saves.parent().and_then(|p| p.parent()) else {
        return Default::default();
    };
    fs::read_dir(profiles)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| fs::read_to_string(e.path().join("saves/.cloud-sync-manifest")).ok())
        .flat_map(|t| t.lines().map(String::from).collect::<Vec<_>>())
        .collect()
}

/// Content provenance: matching sizes and second-resolution mtimes cannot prove
/// the prefix still holds our previous copy. A stdlib hash change merely causes
/// a conservative rescue on the first sync after an upgrade.
pub fn save_sync_key(path: &Path) -> io::Result<String> {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    fs::read(path)?.hash(&mut hash);
    Ok(format!(
        "{}\t{:016x}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        hash.finish()
    ))
}

/// What [`Profile::import_prefix_saves`] did: saves copied under their own
/// name, and saves written as `orphan-<secs>-<name>` because the profile
/// already had that name.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SavesImport {
    pub copied: usize,
    pub renamed: usize,
}

fn mtime_secs(md: &fs::Metadata) -> u64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .unwrap_or_default()
        .as_secs()
}

/// Whether two files hold different bytes, sizes first.
fn differs(a: &Path, b: &Path, a_len: u64, b_len: u64) -> bool {
    a_len != b_len || fs::read(a).ok() != fs::read(b).ok()
}

/// Whether a plugins.txt has the signature of a crash artifact: several plugins
/// LISTED, none active. A deliberate everything-off edit also matches - but at
/// seed time there is no user history to defer to, and deriving from discovery
/// beats founding a profile on a wreck.
pub(crate) fn looks_like_crash_artifact(path: &Path) -> bool {
    let Some(text) = eidos_plugins::read_decoded(path) else {
        return false;
    };
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
            let already = fs::read_dir(&dst)
                .map(|it| it.flatten().next().is_some())
                .unwrap_or(false);
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

    /// Prefix saves the profile bind hides while the game runs, by name, sorted.
    /// A Steam Cloud download from another device or a launch without Eidos
    /// lands in the prefix, and nothing pulls it in: seeding is one-time so
    /// saves the user deleted stay deleted. Nothing before the profile is
    /// seeded (the `.seeded` marker).
    ///
    /// A name the profile lacks counts when it was written after seeding and no
    /// profile of this instance pushed that name; older names are the set the
    /// user deleted. A name the profile has counts when the prefix copy is
    /// newer with other bytes, no profile of this instance pushed that content
    /// (they all bind over the same prefix Saves), and it was not imported yet
    /// as `orphan-<secs>-<name>`.
    pub fn hidden_prefix_saves(&self, prefix_saves: &Path) -> Vec<String> {
        let saves = self.saves_dir();
        let Ok(seeded) = fs::metadata(saves.join(".seeded")).and_then(|m| m.modified()) else {
            return Vec::new(); // not seeded yet: the next seed adopts everything
        };
        let keys = instance_sync_keys(&saves);
        let pushed: HashSet<&str> = keys
            .iter()
            .filter_map(|k| k.split_once('\t').map(|(name, _)| name))
            .collect();
        let mut hidden: Vec<String> = fs::read_dir(prefix_saves)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                if !is_save_listing(&name) {
                    return None;
                }
                let md = e.metadata().ok()?;
                let modified = md.modified().ok()?;
                let ours = saves.join(&name);
                let shown = match fs::metadata(&ours) {
                    Err(_) => modified > seeded && !pushed.contains(name.as_str()),
                    Ok(own) => {
                        own.modified().is_ok_and(|t| modified > t)
                            && differs(&e.path(), &ours, md.len(), own.len())
                            && save_sync_key(&e.path()).is_ok_and(|k| !keys.contains(&k))
                            && !saves
                                .join(format!("orphan-{}-{name}", mtime_secs(&md)))
                                .exists()
                    }
                };
                shown.then_some(name)
            })
            .collect();
        hidden.sort();
        hidden
    }

    /// Copy the prefix saves `names` (from [`Self::hidden_prefix_saves`]) into
    /// this profile, each with its co-saves. A save whose file names clash with
    /// the profile's is written as `orphan-<secs>-<name>` (`secs` from the
    /// save's mtime), co-saves included. Files are created new, so a profile
    /// file is never replaced; an existing file with the same bytes counts as
    /// done. A group that fails part-way removes what it wrote and returns the
    /// error; groups already imported stay.
    pub fn import_prefix_saves(
        &self,
        prefix_saves: &Path,
        names: &[String],
    ) -> io::Result<SavesImport> {
        use std::io::Write;
        let saves = self.saves_dir();
        fs::create_dir_all(&saves)?;
        let mut done = SavesImport::default();
        for name in names {
            let save = prefix_saves.join(name);
            let secs = mtime_secs(&fs::metadata(&save)?);
            let mut group = vec![save.clone()];
            group.extend(cosave_siblings(&save));
            let file_name = |p: &Path| p.file_name().unwrap_or_default().to_os_string();
            let clash = group.iter().any(|p| saves.join(file_name(p)).exists());
            let mut written: Vec<PathBuf> = Vec::new();
            let result = group.iter().try_for_each(|src| {
                let mut dst = file_name(src);
                if clash {
                    dst = format!("orphan-{secs}-{}", dst.to_string_lossy()).into();
                }
                let dst = saves.join(dst);
                let bytes = fs::read(src)?;
                match fs::File::options().write(true).create_new(true).open(&dst) {
                    Ok(mut file) => {
                        written.push(dst.clone());
                        file.write_all(&bytes)?;
                        // Keep the save's date: the Saves tab sorts by it.
                        if let Ok(mtime) = fs::metadata(src).and_then(|m| m.modified()) {
                            let _ = file.set_modified(mtime);
                        }
                        Ok(())
                    }
                    Err(e)
                        if e.kind() == io::ErrorKind::AlreadyExists
                            && fs::read(&dst).is_ok_and(|known| known == bytes) =>
                    {
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            });
            if let Err(e) = result {
                for path in written {
                    let _ = fs::remove_file(path);
                }
                return Err(e);
            }
            if clash {
                done.renamed += 1;
            } else {
                done.copied += 1;
            }
        }
        Ok(done)
    }
}

#[cfg(test)]
mod morrowind_cosave_tests {
    use super::*;

    #[test]
    fn mwse_cosaves_seed_and_travel_with_their_save() {
        let root = std::env::temp_dir().join(format!("eidos-mwse-saves-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let source = root.join("game/Saves");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("Slot.ess"), b"save").unwrap();
        fs::write(source.join("slot.MWSE"), b"cosave").unwrap();
        let profile = Profile {
            instance_root: root.join("instance"),
            name: "Default".into(),
        };
        assert_eq!(profile.seed_saves(&source).unwrap(), 2);
        assert!(is_save_data("slot.MWSE"));
        assert!(!is_save_listing("slot.MWSE"));
        assert_eq!(
            cosave_siblings(&profile.saves_dir().join("Slot.ess")),
            vec![profile.saves_dir().join("slot.MWSE")]
        );
        assert_eq!(fs::read(source.join("slot.MWSE")).unwrap(), b"cosave");
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod hidden_prefix_tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    const T: u64 = 1_700_000_000;

    fn stamped(path: &Path, bytes: &[u8], secs: u64) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(secs))
            .unwrap();
    }

    fn mtime_secs(path: &Path) -> u64 {
        fs::metadata(path)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Profile `Main` seeded at T with its own quicksave at T + 10; the prefix
    /// holds a Steam Deck quicksave pair at T + 20 and a byte-identical Kept.ess.
    fn fixture(tag: &str) -> (PathBuf, Profile, PathBuf) {
        let root = std::env::temp_dir().join(format!("eidos-hidden-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let prof = Profile {
            instance_root: root.clone(),
            name: "Main".into(),
        };
        let saves = prof.saves_dir();
        let prefix = root.join("prefix");
        stamped(&saves.join(".seeded"), b"done", T);
        stamped(&saves.join("quicksave.ess"), b"pc", T + 10);
        stamped(&saves.join("Kept.ess"), b"kept", T + 10);
        stamped(&prefix.join("quicksave.ess"), b"deck", T + 20);
        stamped(&prefix.join("quicksave.skse"), b"deck cosave", T + 20);
        stamped(&prefix.join("Kept.ess"), b"kept", T + 10);
        (root, prof, prefix)
    }

    #[test]
    fn prefix_saves_made_after_seeding_outside_the_instance_are_reported() {
        let root = std::env::temp_dir().join(format!("eidos-hidden-saves-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let prof = Profile {
            instance_root: root.clone(),
            name: "Main".into(),
        };
        let saves = prof.saves_dir();
        let prefix = root.join("prefix");
        stamped(&saves.join(".seeded"), b"done", T);
        stamped(&saves.join("Kept.ess"), b"kept", T + 10);
        // Seeded then deleted by the user: older than the marker, stays deleted.
        stamped(&prefix.join("Deleted.ess"), b"deleted", T - 10);
        stamped(&prefix.join("Kept.ess"), b"kept", T + 10);
        // A sibling profile's own push, not a stranger's.
        let pushed = root.join("profiles/Other/saves");
        stamped(&prefix.join("Other.ess"), b"other", T + 10);
        fs::create_dir_all(&pushed).unwrap();
        fs::write(
            pushed.join(".cloud-sync-manifest"),
            "Other.ess\t0000000000000000\n",
        )
        .unwrap();
        // Made on a Steam Deck and downloaded by Steam Cloud.
        stamped(&prefix.join("Deck.ess"), b"deck", T + 10);
        stamped(&prefix.join("Deck.skse"), b"deck cosave", T + 10);
        let hidden = prof.hidden_prefix_saves(&prefix);
        assert_eq!(hidden, vec!["Deck.ess".to_string()]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_newer_same_name_prefix_save_is_listed_until_it_is_imported() {
        let (root, prof, prefix) = fixture("listed");
        let hidden = prof.hidden_prefix_saves(&prefix);
        assert_eq!(hidden, vec!["quicksave.ess".to_string()]);
        let done = prof.import_prefix_saves(&prefix, &hidden).unwrap();
        assert_eq!((done.copied, done.renamed), (0, 1));
        assert!(
            prof.hidden_prefix_saves(&prefix).is_empty(),
            "not offered twice"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_renames_a_clashing_save_with_its_cosave_and_never_overwrites() {
        let (root, prof, prefix) = fixture("import");
        stamped(&prefix.join("Deck.ess"), b"deck", T + 20);
        stamped(&prefix.join("Deck.skse"), b"deck cosave", T + 20);
        let done = prof
            .import_prefix_saves(&prefix, &["quicksave.ess".into(), "Deck.ess".into()])
            .unwrap();
        assert_eq!((done.copied, done.renamed), (1, 1));
        let saves = prof.saves_dir();
        assert_eq!(
            fs::read(saves.join("quicksave.ess")).unwrap(),
            b"pc",
            "the profile's own save is untouched"
        );
        let secs = T + 20;
        assert_eq!(
            fs::read(saves.join(format!("orphan-{secs}-quicksave.ess"))).unwrap(),
            b"deck"
        );
        assert!(
            saves
                .join(format!("orphan-{secs}-quicksave.skse"))
                .is_file(),
            "the co-save follows its save"
        );
        assert!(saves.join("Deck.ess").is_file() && saves.join("Deck.skse").is_file());
        assert_eq!(
            mtime_secs(&saves.join("Deck.ess")),
            T + 20,
            "imported saves keep their date"
        );
        // Importing again changes nothing and fails nothing.
        let before = fs::read_dir(&saves).unwrap().count();
        assert_eq!(
            prof.import_prefix_saves(&prefix, &["quicksave.ess".into()])
                .unwrap()
                .renamed,
            1
        );
        assert_eq!(fs::read_dir(&saves).unwrap().count(), before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_group_that_fails_part_way_leaves_nothing_behind() {
        let (root, prof, prefix) = fixture("partial");
        let saves = prof.saves_dir();
        let secs = T + 20;
        // Someone else's file already holds the co-save's rescue name.
        stamped(
            &saves.join(format!("orphan-{secs}-quicksave.skse")),
            b"other",
            T,
        );
        assert!(prof
            .import_prefix_saves(&prefix, &["quicksave.ess".into()])
            .is_err());
        assert!(!saves.join(format!("orphan-{secs}-quicksave.ess")).exists());
        assert_eq!(
            fs::read(saves.join(format!("orphan-{secs}-quicksave.skse"))).unwrap(),
            b"other"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_save_another_profile_pushed_is_not_a_hidden_save() {
        let (root, prof, prefix) = fixture("pushed");
        let survival = root.join("profiles/Survival/saves");
        fs::create_dir_all(&survival).unwrap();
        fs::write(
            survival.join(".cloud-sync-manifest"),
            format!(
                "{}\n",
                save_sync_key(&prefix.join("quicksave.ess")).unwrap()
            ),
        )
        .unwrap();
        assert!(prof.hidden_prefix_saves(&prefix).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
