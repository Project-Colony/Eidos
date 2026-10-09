//! Moving global instances onto the Colony layout.
//!
//! An older Eidos kept each global instance at `~/.local/share/eidos/<game>`;
//! the Colony layout puts it at `~/.local/share/Colony/Eidos/instances/<game>`.
//! This is the instance half of `eidos_paths::migrate_legacy_layout`, kept here
//! because the move needs two things that crate cannot reach: the instance lock,
//! so a game still running from the old folder is not linked across mid-write,
//! and [`relocate`], which repoints the absolute paths written into `tools.ini`
//! and each mod's `meta.ini`.
//!
//! The files are hard-linked, not copied (`eidos_paths::Carry::Link`): an
//! instance is tens of gigabytes, and a startup that duplicated it would take
//! minutes and could fill the disk. The old folder keeps every entry it had, and
//! until the new one is complete every resolver keeps using the old one.

use std::fs;
use std::path::Path;

use eidos_instance::Instance;
use eidos_paths::{Carry, MIGRATION_MARKER};

use crate::relocate::relocate;

/// Move every global instance still at its old path. Returns notes for the log.
pub fn migrate_global_instances() -> Vec<String> {
    match eidos_paths::legacy_data_dir() {
        Some(old) => migrate_global_instances_in(&old, &eidos_paths::data_dir().join("instances")),
        None => Vec::new(),
    }
}

/// [`migrate_global_instances`] between explicit roots, so it can be tested
/// without pointing the process's `HOME` anywhere.
pub fn migrate_global_instances_in(old: &Path, new: &Path) -> Vec<String> {
    let mut notes = Vec::new();
    let Ok(entries) = fs::read_dir(old) else {
        return notes;
    };
    for entry in entries.flatten() {
        let (from, to) = (entry.path(), new.join(entry.file_name()));
        // Built directly rather than through `Instance::portable`, which would
        // redirect a moved instance - exactly what is not wanted here.
        let inst = Instance { root: from.clone() };
        // An instance, not `runtimes/` or anything else sharing the folder.
        let is_instance = inst.exists() || inst.manifest_path().is_file();
        if !from.is_dir() || !is_instance || to.join(MIGRATION_MARKER).exists() {
            continue;
        }
        if to.symlink_metadata().is_ok() {
            notes.push(format!(
                "{} and {} both exist, so Eidos keeps using the first - move or remove one \
                 of them by hand to settle which instance is yours",
                from.display(),
                to.display()
            ));
            continue;
        }
        // Held for the whole move: a game an older Eidos started from the old
        // folder must not write into it while it is linked across, or the save
        // it writes last would stay behind.
        let _lock = match inst.try_lock("the move to the Colony layout") {
            Ok(lock) => lock,
            Err(e) => {
                notes.push(format!(
                    "could not lock the instance at {} ({e}) - it moves on a later launch",
                    from.display()
                ));
                continue;
            }
        };
        match eidos_paths::migrate_tree(&from, &to, Carry::Link) {
            Ok(n) => {
                notes.push(format!(
                    "moved the instance at {} to {} ({n} files, linked rather than copied, so \
                     it takes no extra space) - the old folder is left as it was",
                    from.display(),
                    to.display()
                ));
                notes.extend(repoint(&to, &from));
            }
            Err(e) => notes.push(format!(
                "could not move the instance at {} to {}: {e} - Eidos keeps using it where it is",
                from.display(),
                to.display()
            )),
        }
    }
    notes
}

/// Point the tool entries and install records of a moved instance at it.
///
/// [`relocate`] writes through `write_atomic`, which replaces the file: the old
/// folder's linked copy keeps naming the old folder, as an older Eidos expects.
fn repoint(to: &Path, from: &Path) -> Vec<String> {
    // Both spellings: Eidos wrote the root as it computed it, and
    // `~/.local/share/eidos` is often a link to a bigger disk.
    let mut froms = vec![from.to_path_buf()];
    if let Ok(real) = fs::canonicalize(from) {
        if real != from {
            froms.push(real);
        }
    }
    let mut notes = Vec::new();
    for old in froms {
        match relocate(to, &old.to_string_lossy()) {
            Ok(done) => notes.extend(done.problems.into_iter().map(|(file, why)| {
                format!(
                    "{file} in {} still names {}: it {why}",
                    to.display(),
                    old.display()
                )
            })),
            Err(e) => notes.push(format!(
                "could not repoint the paths in {}: {e}",
                to.display()
            )),
        }
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            let p = std::env::temp_dir().join(format!(
                "eidos-transfer-migrate-{}-{tag}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Tmp(fs::canonicalize(p).unwrap())
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(p: PathBuf, body: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    /// A global instance as an older Eidos left it, with a tool and an install
    /// record naming its own folder by absolute path.
    fn legacy_instance(old: &Path, game: &str) -> PathBuf {
        let root = old.join(game);
        fs::create_dir_all(&root).unwrap();
        eidos_instance::Manifest::new(game, eidos_instance::InstanceKind::Global)
            .write(&root.join("eidos-instance.ini"))
            .unwrap();
        write(root.join("mods/A/Data/a.esp"), "plugin");
        write(
            root.join("tools.ini"),
            &format!("[0]\ntitle=Tool\nexe={}/mods/A/tool.exe\n", root.display()),
        );
        write(
            root.join("mods/A/meta.ini"),
            &format!("installationFile={}/downloads/a.7z\n", root.display()),
        );
        root
    }

    #[test]
    fn an_instance_is_linked_across_repointed_and_left_in_place() {
        let t = Tmp::new("move");
        let (old, new) = (t.0.join("eidos"), t.0.join("Colony/Eidos/instances"));
        let from = legacy_instance(&old, "skyrimse");
        write(old.join("runtimes/dotnet-8/dotnet"), "binary");

        let notes = migrate_global_instances_in(&old, &new);

        assert_eq!(notes.len(), 1, "{notes:?}");
        let to = new.join("skyrimse");
        assert!(to.join(MIGRATION_MARKER).is_file());
        let ino = |p: PathBuf| fs::metadata(p).unwrap().ino();
        assert_eq!(
            ino(to.join("mods/A/Data/a.esp")),
            ino(from.join("mods/A/Data/a.esp")),
            "linked, not duplicated"
        );
        // The tool now runs from the new folder...
        let tools = fs::read_to_string(to.join("tools.ini")).unwrap();
        assert!(
            tools.contains(&format!("exe={}/mods/A/tool.exe", to.display())),
            "{tools}"
        );
        let meta = fs::read_to_string(to.join("mods/A/meta.ini")).unwrap();
        assert!(meta.contains(&to.display().to_string()), "{meta}");
        // ...while the old folder still says what it said.
        let old_tools = fs::read_to_string(from.join("tools.ini")).unwrap();
        assert!(old_tools.contains(&format!("exe={}/mods/A/tool.exe", from.display())));
        // Not every folder in there is an instance.
        assert!(!new.join("runtimes").exists());

        // And the second launch has nothing left to do.
        assert!(migrate_global_instances_in(&old, &new).is_empty());
    }

    #[test]
    fn an_instance_in_use_waits_for_a_later_launch() {
        let t = Tmp::new("busy");
        let (old, new) = (t.0.join("eidos"), t.0.join("instances"));
        let from = legacy_instance(&old, "fallout4");
        let (held_tx, held_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let holder = std::thread::spawn(move || {
            let _lock = Instance { root: from }.try_lock("a running game").unwrap();
            held_tx.send(()).unwrap();
            done_rx.recv().unwrap();
        });
        held_rx.recv().unwrap();

        let notes = migrate_global_instances_in(&old, &new);
        done_tx.send(()).unwrap();
        holder.join().unwrap();

        assert!(notes[0].starts_with("could not lock"), "{notes:?}");
        assert!(!new.join("fallout4").exists());
        assert_eq!(migrate_global_instances_in(&old, &new).len(), 1);
    }

    #[test]
    fn an_instance_already_at_the_new_path_is_never_merged_into() {
        let t = Tmp::new("both");
        let (old, new) = (t.0.join("eidos"), t.0.join("instances"));
        legacy_instance(&old, "skyrimse");
        write(new.join("skyrimse/modlist.txt"), "+Mine\n");

        let notes = migrate_global_instances_in(&old, &new);

        assert!(notes[0].contains("both exist"), "{notes:?}");
        assert!(!new.join("skyrimse/mods").exists());
        assert!(!new.join("skyrimse").join(MIGRATION_MARKER).exists());
    }
}
