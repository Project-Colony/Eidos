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

/// Left in a moved instance once [`repoint`] has run clean on it.
///
/// The move and the repoint are two steps, and the move's own marker
/// ([`MIGRATION_MARKER`]) goes down with the first. Without a second one, a
/// repoint that failed, or a process killed between the two, would leave
/// `tools.ini` and every `meta.ini` naming the old folder for good. Until this
/// is down, every launch repoints the instance again: the pass only rewrites
/// values that still name the old folder, so running it twice changes nothing.
const REPOINTED_MARKER: &str = ".migrated-repointed";

/// Taken in the Colony instances folder by every Eidos process that finds an
/// instance still to move: exclusively for the move, then shared for the rest
/// of the run.
///
/// Every process resolves `Instance::global` and `Instance::portable` against
/// what has moved so far, and keeps that answer for its whole run. A window that
/// opened an old folder (its move was put off because a game still held it)
/// must not have the move happen under it, from a child it spawns or from a link
/// clicked in the browser: from then on that process would write into the new
/// tree and the window into the old one, and the window's edits would seem lost
/// on the next launch. So a process that cannot take this exclusively moves
/// nothing and waits only for a move already under way.
const MOVE_LOCK: &str = ".move.lock";

/// Move every global instance still at its old path.
///
/// Returns notes for the log, and a lock to hold until the process exits (see
/// [`MOVE_LOCK`]): dropping it early lets another process move an instance this
/// one may already have resolved to its old folder.
pub fn migrate_global_instances() -> (Vec<String>, Option<fs::File>) {
    match eidos_paths::legacy_data_dir() {
        Some(old) => migrate_global_instances_in(&old, &eidos_paths::data_dir().join("instances")),
        None => (Vec::new(), None),
    }
}

/// [`migrate_global_instances`] between explicit roots, so it can be tested
/// without pointing the process's `HOME` anywhere.
pub fn migrate_global_instances_in(old: &Path, new: &Path) -> (Vec<String>, Option<fs::File>) {
    let mut notes = Vec::new();
    let Ok(entries) = fs::read_dir(old) else {
        return (notes, None);
    };
    let pending: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            // Built directly rather than through `Instance::portable`, which
            // would redirect a moved instance - exactly what is not wanted here.
            let inst = Instance { root: entry.path() };
            let to = new.join(entry.file_name());
            // An instance, not `runtimes/` or anything else sharing the folder.
            let is_instance = inst.exists() || inst.manifest_path().is_file();
            (inst.root.is_dir() && is_instance && !to.join(REPOINTED_MARKER).exists())
                .then_some((inst, to))
        })
        .collect();
    // Nothing left to move, so nothing another process could move under us.
    if pending.is_empty() {
        return (notes, None);
    }
    let held = fs::create_dir_all(new).and_then(|()| {
        fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(new.join(MOVE_LOCK))
    });
    let held = match held.map(|f| (f.try_lock(), f)) {
        Ok((Ok(()), f)) => f,
        Ok((Err(fs::TryLockError::WouldBlock), f)) => {
            notes.push(format!(
                "another Eidos is running, so the instances still in {} move on a later launch",
                old.display()
            ));
            // Blocks only while a move is under way, so this process resolves
            // the instances after it, the same way the mover does.
            return match f.lock_shared() {
                Ok(()) => (notes, Some(f)),
                Err(_) => (notes, None),
            };
        }
        Ok((Err(fs::TryLockError::Error(e)), _)) | Err(e) => {
            notes.push(format!(
                "could not lock {} ({e}) - the instances in {} move on a later launch",
                new.join(MOVE_LOCK).display(),
                old.display()
            ));
            return (notes, None);
        }
    };
    for (inst, to) in pending {
        let from = inst.root.clone();
        // Moved by an earlier launch that did not get to finish repointing it.
        let moved = to.join(MIGRATION_MARKER).exists();
        if !moved && to.symlink_metadata().is_ok() {
            notes.push(format!(
                "{} and {} both exist, so Eidos keeps using the first - move or remove one \
                 of them by hand to settle which instance is yours",
                from.display(),
                to.display()
            ));
            continue;
        }
        // Held for the whole move and the repoint: a game an older Eidos started
        // from the old folder must not write into it while it is linked across,
        // or the save it writes last would stay behind. The moved instance's
        // `.eidos.lock` is a link to this one, so an Eidos that opened the new
        // folder holds this same lock and keeps the repoint out too.
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
        if !moved {
            match eidos_paths::migrate_tree(&from, &to, Carry::Link) {
                Ok(n) => notes.push(format!(
                    "moved the instance at {} to {} ({n} files, linked rather than copied) - \
                     the old folder is left as it was, and once you are happy with the move \
                     you can delete it: until then, every mod or download you remove or \
                     reinstall keeps its old copy on disk through it",
                    from.display(),
                    to.display()
                )),
                Err(e) => {
                    notes.push(format!(
                        "could not move the instance at {} to {}: {e} - Eidos keeps using it \
                         where it is",
                        from.display(),
                        to.display()
                    ));
                    continue;
                }
            }
        }
        let problems = repoint(&to, &from);
        if problems.is_empty() {
            // Best effort: without it the next launch only repoints again.
            let _ = fs::write(to.join(REPOINTED_MARKER), "");
        }
        notes.extend(problems);
    }
    // Shared from here on: another Eidos may now start, and must find the
    // instances where this one left them. Released first, so a process that
    // takes it exclusively in between only finishes a move this one put off,
    // and this one waits for that before resolving anything.
    let _ = held.unlock();
    match held.lock_shared() {
        Ok(()) => (notes, Some(held)),
        Err(_) => (notes, None),
    }
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

    /// One Eidos launch that exits right after: its move lock goes with it.
    fn launch(old: &Path, new: &Path) -> Vec<String> {
        migrate_global_instances_in(old, new).0
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

        let notes = launch(&old, &new);

        assert_eq!(notes.len(), 1, "{notes:?}");
        let to = new.join("skyrimse");
        assert!(to.join(MIGRATION_MARKER).is_file());
        assert!(to.join(REPOINTED_MARKER).is_file());
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
        assert!(launch(&old, &new).is_empty());
    }

    #[test]
    fn a_move_whose_repoint_never_ran_is_repointed_on_the_next_launch() {
        // A launch killed between the move and the repoint: the instance is in
        // place with its marker down, and its files still name the old folder.
        let t = Tmp::new("repoint");
        let (old, new) = (t.0.join("eidos"), t.0.join("instances"));
        let from = legacy_instance(&old, "skyrimse");
        let to = new.join("skyrimse");
        eidos_paths::migrate_tree(&from, &to, Carry::Link).unwrap();
        assert!(!to.join(REPOINTED_MARKER).exists());

        let notes = launch(&old, &new);

        assert!(notes.is_empty(), "{notes:?}");
        let tools = fs::read_to_string(to.join("tools.ini")).unwrap();
        assert!(
            tools.contains(&format!("exe={}/mods/A/tool.exe", to.display())),
            "{tools}"
        );
        assert!(to.join(REPOINTED_MARKER).is_file());
        assert!(launch(&old, &new).is_empty());
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

        // The window starts while the game holds the old folder, so it keeps
        // using that folder, and keeps its move lock for as long as it runs.
        let (notes, window) = migrate_global_instances_in(&old, &new);
        done_tx.send(()).unwrap();
        holder.join().unwrap();
        assert!(notes[0].starts_with("could not lock"), "{notes:?}");
        assert!(!new.join("fallout4").exists());

        // The game is over, but the window is still open on the old folder: a
        // child it spawns, or a link clicked in the browser, must not move the
        // instance out from under it.
        let notes = launch(&old, &new);
        assert!(
            notes[0].starts_with("another Eidos is running"),
            "{notes:?}"
        );
        assert!(!new.join("fallout4").exists());

        // Once the window has closed, the next launch moves it.
        drop(window);
        assert_eq!(launch(&old, &new).len(), 1);
        assert!(new.join("fallout4").join(REPOINTED_MARKER).is_file());
    }

    #[test]
    fn an_instance_already_at_the_new_path_is_never_merged_into() {
        let t = Tmp::new("both");
        let (old, new) = (t.0.join("eidos"), t.0.join("instances"));
        legacy_instance(&old, "skyrimse");
        write(new.join("skyrimse/modlist.txt"), "+Mine\n");

        let notes = launch(&old, &new);

        assert!(notes[0].contains("both exist"), "{notes:?}");
        assert!(!new.join("skyrimse/mods").exists());
        assert!(!new.join("skyrimse").join(MIGRATION_MARKER).exists());
    }
}
