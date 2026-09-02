//! Timestamped backups of the two lists a session can destroy: the mod order
//! and the plugin load order.
//!
//! Both files are already written atomically, and both already keep ONE
//! previous copy - `modlist.txt.bak` on every save, `plugins.txt.pre-session`
//! around a launch. Neither survives the failure that actually costs an
//! evening: a bad LOOT sort, a drag that landed forty rows off, or simply
//! "what did this look like yesterday". That needs several restore points the
//! user picks from, which is what MO2's Create Backup / Restore Backup give.
//!
//! Named `<file>.<unix seconds>` rather than MO2's `.yyyy_MM_dd_hh_mm_ss`: the
//! stamp sorts correctly as an integer, cannot be misread across locales, and
//! the human-readable form is produced for display instead of parsed back.
//!
//! # They live in `<profile>/backups/`, never beside the originals
//!
//! MO2 writes its copies next to the files. Eidos cannot: the plugin state
//! directory is BIND-MOUNTED over the game's own AppData folder at launch, so
//! anything left there is handed to the game and to every tool that reads that
//! directory. Two concrete failures came out of putting them there - the
//! freshness tiebreak that decides which of `plugins.txt` / `loadorder.txt` is
//! authoritative was being reset by the copies, and half-written pairs piled up
//! invisibly. A separate directory keeps the game's view exactly as it was.
//!
//! It also fixes a subtler one: the game rewrites `plugins.txt` as `Plugins.txt`
//! on Windows-cased filesystems, so the live file has to be found through
//! `newest_variant` while the backups keep a stable lowercase name of our own.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::Profile;

/// How many restore points to keep per list. MO2 keeps 10 and that is a good
/// number: enough to reach past a bad session, few enough that the restore
/// dialog stays a list rather than an archive.
pub const KEEP_BACKUPS: usize = 10;

/// Which list a backup covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    /// `modlist.txt` - the mod order and each mod's enabled state.
    ModList,
    /// `plugins.txt` + `loadorder.txt` - the plugin load order and active set.
    /// Backed up and restored TOGETHER: they describe one state between them,
    /// and restoring half of it produces an order the game never had.
    LoadOrder,
}

impl BackupKind {
    pub fn label(self) -> &'static str {
        match self {
            BackupKind::ModList => "mod list",
            BackupKind::LoadOrder => "load order",
        }
    }
}

/// One restore point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    /// Unix seconds, taken when the backup was made. Also its file suffix.
    pub stamp: u64,
    /// The files this restore point holds, already known to exist.
    pub files: Vec<PathBuf>,
}

impl Backup {
    /// `YYYY-MM-DD HH:MM` in UTC, for the restore list.
    pub fn when(&self) -> String {
        format_stamp(self.stamp)
    }
}

/// A stamp as `YYYY-MM-DD HH:MM`, UTC. Howard Hinnant's civil_from_days, the
/// same arithmetic the CSV export uses - no date dependency for two fields.
pub fn format_stamp(secs: u64) -> String {
    let secs = secs as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let (h, mi) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32);
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Profile {
    /// Where the stamped copies are kept: outside the bind-mounted plugin
    /// directory, and outside anything the game or a tool reads.
    pub fn backups_dir(&self) -> PathBuf {
        self.dir().join("backups")
    }

    /// The canonical, lowercase base names a backup of `kind` covers. These
    /// name the STAMPED COPIES, so the listing survives the game rewriting the
    /// live file under a different case.
    fn backup_slots(kind: BackupKind) -> &'static [&'static str] {
        match kind {
            BackupKind::ModList => &["modlist.txt"],
            BackupKind::LoadOrder => &["plugins.txt", "loadorder.txt"],
        }
    }

    /// The directory the LIVE files of `kind` live in.
    fn live_dir(&self, kind: BackupKind) -> PathBuf {
        match kind {
            BackupKind::ModList => self.dir(),
            BackupKind::LoadOrder => self.plugins_state_dir(),
        }
    }

    /// The live files to copy FROM, resolved through the case-variant rule the
    /// rest of the crate uses: after a session the real file may be
    /// `Plugins.txt`, and reading the exact-case path would silently back up
    /// nothing.
    fn backup_live(&self, kind: BackupKind) -> Vec<PathBuf> {
        let dir = self.live_dir(kind);
        Self::backup_slots(kind)
            .iter()
            .map(|n| eidos_plugins::newest_variant(&dir, n).unwrap_or_else(|| dir.join(n)))
            .collect()
    }

    /// Where each slot's stamped copies are keyed from.
    fn backup_sources(&self, kind: BackupKind) -> Vec<PathBuf> {
        let dir = self.backups_dir();
        Self::backup_slots(kind)
            .iter()
            .map(|n| dir.join(n))
            .collect()
    }

    /// Take a restore point, and prune the oldest beyond [`KEEP_BACKUPS`].
    ///
    /// Refuses rather than writes an empty one when the list does not exist
    /// yet: a "backup" that restores nothing is worse than no backup, because
    /// it occupies a slot the user believes is a safety net.
    pub fn create_backup(&self, kind: BackupKind) -> io::Result<Backup> {
        // EVERY file of the kind, or none: `backups` only lists complete
        // restore points, so writing a partial one produces a success message
        // for something that will never appear in the list - which is exactly
        // how a safety net becomes a lie.
        let live = self.backup_live(kind);
        if let Some(missing) = live.iter().find(|p| !p.is_file()) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "there is no complete {} to back up yet ({} is missing)",
                    kind.label(),
                    missing.file_name().unwrap_or_default().to_string_lossy()
                ),
            ));
        }
        // Read every file BEFORE writing any: a backup that captured half of a
        // pair the game rewrote mid-copy would restore an order that never
        // existed.
        let mut staged = Vec::with_capacity(live.len());
        for src in &live {
            staged.push(fs::read(src)?);
        }
        fs::create_dir_all(self.backups_dir())?;
        let stamp = self.free_stamp(kind);
        let mut files = Vec::new();
        for (bytes, slot) in staged.iter().zip(self.backup_sources(kind)) {
            let dst = stamped_path(&slot, stamp);
            write_atomic(&dst, bytes)?;
            files.push(dst);
        }
        self.prune_backups(kind);
        Ok(Backup { stamp, files })
    }

    /// A stamp no restore point of `kind` is already using.
    ///
    /// Stamps are per-second and two backups in one second are ordinary - the
    /// safety copy `restore_backup` takes lands in the same second as the click
    /// that triggered it. Without this, the second one would overwrite the
    /// first, and the file it overwrote might be the very point being restored.
    fn free_stamp(&self, kind: BackupKind) -> u64 {
        let taken: Vec<u64> = self.backups(kind).into_iter().map(|b| b.stamp).collect();
        let mut stamp = now_unix();
        while taken.contains(&stamp) {
            stamp += 1;
        }
        stamp
    }

    /// Every restore point for `kind`, newest first.
    ///
    /// A stamp only counts when EVERY file of the kind is present: half a
    /// load-order backup cannot be restored, so offering it would be a button
    /// that fails on click.
    pub fn backups(&self, kind: BackupKind) -> Vec<Backup> {
        let sources = self.backup_sources(kind);
        let Some(first) = sources.first() else {
            return Vec::new();
        };
        let mut out: Vec<Backup> = stamps_for(first)
            .into_iter()
            .filter_map(|stamp| {
                let files: Vec<PathBuf> = sources.iter().map(|s| stamped_path(s, stamp)).collect();
                files
                    .iter()
                    .all(|f| f.is_file())
                    .then_some(Backup { stamp, files })
            })
            .collect();
        out.sort_by_key(|b| std::cmp::Reverse(b.stamp));
        out
    }

    /// Put a restore point back, after taking one of the CURRENT state first -
    /// restoring is itself a destructive edit, and the user who picks the wrong
    /// timestamp must be able to walk back out of it.
    pub fn restore_backup(&self, kind: BackupKind, stamp: u64) -> io::Result<()> {
        let slots = Self::backup_slots(kind);
        let sources = self.backup_sources(kind);
        let stamped: Vec<PathBuf> = sources.iter().map(|s| stamped_path(s, stamp)).collect();
        if let Some(missing) = stamped.iter().find(|p| !p.is_file()) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "that backup is incomplete: {} is missing",
                    missing.display()
                ),
            ));
        }
        // Read the restore point BEFORE the safety copy runs, so the order of
        // the two writes cannot matter.
        let mut staged: Vec<Vec<u8>> = Vec::with_capacity(stamped.len());
        for from in &stamped {
            staged.push(fs::read(from)?);
        }
        // The safety copy is part of the promise the GUI makes ("the state it
        // replaced was backed up first"), so a failure to take it stops the
        // restore rather than being swallowed - except when there is simply
        // nothing to save yet, which is not a failure.
        match self.create_backup(kind) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(io::Error::other(format!(
                    "refusing to restore: the current {} could not be backed up first ({e})",
                    kind.label()
                )))
            }
        }
        let dir = self.live_dir(kind);
        fs::create_dir_all(&dir)?;
        // Keep what is being replaced, in memory, so a failure part-way through
        // a pair can be undone. plugins.txt and loadorder.txt describe ONE
        // state: leaving one restored and one not is the split this whole
        // module exists to prevent.
        let mut undo: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
        for (bytes, slot) in staged.iter().zip(slots) {
            // Through canonical_path, like every other writer: it collapses the
            // case variants the game leaves behind instead of adding one more.
            let target = eidos_plugins::canonical_path(&dir, slot);
            undo.push((target.clone(), fs::read(&target).ok()));
            if let Err(e) = write_atomic(&target, bytes) {
                for (path, before) in undo.iter().rev().skip(1) {
                    match before {
                        Some(b) => {
                            let _ = write_atomic(path, b);
                        }
                        None => {
                            let _ = fs::remove_file(path);
                        }
                    }
                }
                return Err(e);
            }
        }
        Ok(())
    }

    /// Drop the oldest restore points past [`KEEP_BACKUPS`]. Best-effort: a
    /// backup that cannot be pruned is clutter, not a failure worth reporting.
    fn prune_backups(&self, kind: BackupKind) {
        let keep: Vec<u64> = self
            .backups(kind)
            .into_iter()
            .take(KEEP_BACKUPS)
            .map(|b| b.stamp)
            .collect();
        // Every stamped file of every slot, not just the complete points:
        // otherwise an interrupted backup leaves halves that `backups` cannot
        // see and that nothing would ever remove.
        for slot in self.backup_sources(kind) {
            for stamp in stamps_for(&slot) {
                if !keep.contains(&stamp) {
                    let _ = fs::remove_file(stamped_path(&slot, stamp));
                }
            }
        }
    }
}

/// Write bytes through a temp file and a rename, so an interrupted write can
/// never leave a torn list where a whole one used to be.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    // The backups directory is made here rather than by the shared writer: a
    // restore point legitimately lands in a folder that has never existed.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::write_atomic(path, bytes)
}

/// `modlist.txt` + 1787580000 -> `modlist.txt.1787580000`.
fn stamped_path(src: &Path, stamp: u64) -> PathBuf {
    let mut name = src.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{stamp}"));
    src.with_file_name(name)
}

/// The stamps found beside `src`, from names of the form `<file>.<digits>`.
///
/// Digits only, deliberately: the directory also holds `modlist.txt.bak` and
/// hand-made copies like `modlist.txt.before-cosmos`, and neither is a restore
/// point this code wrote or knows how to interpret.
fn stamps_for(src: &Path) -> Vec<u64> {
    let (Some(dir), Some(base)) = (src.parent(), src.file_name().and_then(|n| n.to_str())) else {
        return Vec::new();
    };
    let prefix = format!("{base}.");
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            name.strip_prefix(&prefix)?.parse::<u64>().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    /// A profile with a real modlist and plugin order on disk.
    fn profile() -> Profile {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("eidos-bk-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&root);
        let p = Profile {
            instance_root: root,
            name: "Default".to_string(),
        };
        fs::create_dir_all(p.dir()).unwrap();
        fs::write(p.dir().join("modlist.txt"), "+A\n-B\n").unwrap();
        fs::create_dir_all(p.plugins_txt_path().parent().unwrap()).unwrap();
        fs::write(p.plugins_txt_path(), "*Skyrim.esm\n").unwrap();
        fs::write(p.loadorder_txt_path(), "Skyrim.esm\n").unwrap();
        p
    }

    fn cleanup(p: &Profile) {
        let _ = fs::remove_dir_all(&p.instance_root);
    }

    #[test]
    fn a_backup_round_trips_the_mod_list() {
        // The whole point: a list destroyed after the backup comes back.
        let p = profile();
        let b = p.create_backup(BackupKind::ModList).unwrap();
        fs::write(p.dir().join("modlist.txt"), "+RUINED\n").unwrap();
        p.restore_backup(BackupKind::ModList, b.stamp).unwrap();
        assert_eq!(
            fs::read_to_string(p.dir().join("modlist.txt")).unwrap(),
            "+A\n-B\n"
        );
        cleanup(&p);
    }

    #[test]
    fn the_load_order_backup_covers_both_files_at_once() {
        // plugins.txt and loadorder.txt describe ONE state between them;
        // restoring half of it invents an order the game never had.
        let p = profile();
        let b = p.create_backup(BackupKind::LoadOrder).unwrap();
        assert_eq!(b.files.len(), 2, "both files are in the restore point");
        fs::write(p.plugins_txt_path(), "*Wrong.esp\n").unwrap();
        fs::write(p.loadorder_txt_path(), "Wrong.esp\n").unwrap();
        p.restore_backup(BackupKind::LoadOrder, b.stamp).unwrap();
        assert_eq!(
            fs::read_to_string(p.plugins_txt_path()).unwrap(),
            "*Skyrim.esm\n"
        );
        assert_eq!(
            fs::read_to_string(p.loadorder_txt_path()).unwrap(),
            "Skyrim.esm\n"
        );
        cleanup(&p);
    }

    #[test]
    fn restoring_first_backs_up_what_it_is_about_to_replace() {
        // Picking the wrong timestamp must not be a one-way door.
        let p = profile();
        let first = p.create_backup(BackupKind::ModList).unwrap();
        fs::write(p.dir().join("modlist.txt"), "+CURRENT\n").unwrap();
        p.restore_backup(BackupKind::ModList, first.stamp).unwrap();
        let saved: Vec<String> = p
            .backups(BackupKind::ModList)
            .iter()
            .map(|b| fs::read_to_string(&b.files[0]).unwrap())
            .collect();
        assert!(
            saved.contains(&"+CURRENT\n".to_string()),
            "the replaced state was kept: {saved:?}"
        );
        cleanup(&p);
    }

    #[test]
    fn an_incomplete_restore_point_is_neither_listed_nor_restorable() {
        // Half a load-order backup would be a button that fails on click.
        let p = profile();
        let b = p.create_backup(BackupKind::LoadOrder).unwrap();
        fs::remove_file(&b.files[1]).unwrap();
        assert!(p.backups(BackupKind::LoadOrder).is_empty(), "not offered");
        assert!(
            p.restore_backup(BackupKind::LoadOrder, b.stamp).is_err(),
            "not restorable"
        );
        cleanup(&p);
    }

    #[test]
    fn hand_made_copies_beside_the_list_are_not_mistaken_for_backups() {
        // Real profiles hold modlist.txt.bak and things like
        // modlist.txt.before-cosmos. Neither is a restore point this code wrote.
        let p = profile();
        fs::write(p.dir().join("modlist.txt.bak"), "x").unwrap();
        fs::write(p.dir().join("modlist.txt.before-cosmos"), "x").unwrap();
        p.create_backup(BackupKind::ModList).unwrap();
        assert_eq!(
            p.backups(BackupKind::ModList).len(),
            1,
            "only the stamped one counts"
        );
        cleanup(&p);
    }

    #[test]
    fn backing_up_a_list_that_does_not_exist_refuses_instead_of_faking_one() {
        let p = profile();
        fs::remove_file(p.dir().join("modlist.txt")).unwrap();
        assert!(p.create_backup(BackupKind::ModList).is_err());
        cleanup(&p);
    }

    #[test]
    fn only_the_ten_newest_restore_points_are_kept() {
        // Stamps are per-second, so the loop writes them by hand to avoid a
        // ten-second test; create_backup's own pruning is what is under test.
        let p = profile();
        let src = p.dir().join("modlist.txt");
        fs::create_dir_all(p.backups_dir()).unwrap();
        let slot = p.backups_dir().join("modlist.txt");
        for stamp in 1_000..1_015u64 {
            fs::copy(&src, stamped_path(&slot, stamp)).unwrap();
        }
        assert_eq!(p.backups(BackupKind::ModList).len(), 15);
        p.create_backup(BackupKind::ModList).unwrap();
        let left = p.backups(BackupKind::ModList);
        assert_eq!(left.len(), KEEP_BACKUPS, "pruned to the cap");
        assert_eq!(
            left[0].stamp,
            left.iter().map(|b| b.stamp).max().unwrap(),
            "newest first"
        );
        assert!(
            left.iter().all(|b| b.stamp >= 1_005),
            "the oldest went, not the newest"
        );
        cleanup(&p);
    }

    #[test]
    fn a_load_order_the_game_recased_is_still_backed_up_and_restored() {
        // The defect this module shipped with: after one session the live file
        // is `Plugins.txt`, the exact-case path is gone, and the backup quietly
        // captured half a pair - reporting success while the restore list
        // stayed empty forever.
        let p = profile();
        fs::remove_file(p.plugins_txt_path()).unwrap();
        let recased = p.plugins_state_dir().join("Plugins.txt");
        fs::write(&recased, "*Recased.esp\n").unwrap();

        let b = p.create_backup(BackupKind::LoadOrder).unwrap();
        assert_eq!(b.files.len(), 2, "both halves captured despite the case");
        assert_eq!(
            p.backups(BackupKind::LoadOrder).len(),
            1,
            "and it is listed"
        );

        fs::write(&recased, "*Ruined.esp\n").unwrap();
        p.restore_backup(BackupKind::LoadOrder, b.stamp).unwrap();
        // Restored through canonical_path: one file, not a new case variant
        // beside the old one.
        let variants: Vec<String> = fs::read_dir(p.plugins_state_dir())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.eq_ignore_ascii_case("plugins.txt"))
            .collect();
        assert_eq!(variants.len(), 1, "no split pair left behind: {variants:?}");
        let live = eidos_plugins::newest_variant(&p.plugins_state_dir(), "plugins.txt").unwrap();
        assert_eq!(fs::read_to_string(live).unwrap(), "*Recased.esp\n");
        cleanup(&p);
    }

    #[test]
    fn backups_never_land_in_the_directory_the_game_is_shown() {
        // plugins_state_dir is bind-mounted over the game's own AppData folder
        // at launch. Anything left there is handed to the game and to every
        // tool that reads it - and the stamped copies were resetting the
        // freshness tiebreak that decides which file is authoritative.
        let p = profile();
        let before: Vec<String> = fs::read_dir(p.plugins_state_dir())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        p.create_backup(BackupKind::LoadOrder).unwrap();
        let after: Vec<String> = fs::read_dir(p.plugins_state_dir())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            before.len(),
            after.len(),
            "the game's view is untouched: {after:?}"
        );
        assert!(
            p.backups_dir().is_dir(),
            "they went to the profile's backups dir"
        );
        cleanup(&p);
    }

    #[test]
    fn half_a_pair_is_refused_rather_than_reported_as_a_backup() {
        // A partial restore point can never be listed, so writing one produces
        // a success message for something that will never appear.
        let p = profile();
        fs::remove_file(p.plugins_txt_path()).unwrap();
        let err = p.create_backup(BackupKind::LoadOrder).unwrap_err();
        assert!(
            format!("{err}").contains("plugins.txt"),
            "it says what is missing: {err}"
        );
        assert!(p.backups(BackupKind::LoadOrder).is_empty());
        cleanup(&p);
    }

    #[test]
    fn two_backups_in_the_same_second_do_not_overwrite_each_other() {
        // The safety copy a restore takes lands in the same second as the click
        // that triggered it - and the file it would overwrite could be the very
        // point being restored.
        let p = profile();
        let a = p.create_backup(BackupKind::ModList).unwrap();
        fs::write(p.dir().join("modlist.txt"), "+Second\n").unwrap();
        let b = p.create_backup(BackupKind::ModList).unwrap();
        assert_ne!(a.stamp, b.stamp, "each got its own slot");
        assert_eq!(p.backups(BackupKind::ModList).len(), 2);
        p.restore_backup(BackupKind::ModList, a.stamp).unwrap();
        assert_eq!(
            fs::read_to_string(p.dir().join("modlist.txt")).unwrap(),
            "+A\n-B\n"
        );
        cleanup(&p);
    }

    #[test]
    fn orphaned_halves_are_swept_rather_than_accumulating() {
        // An interrupted backup leaves a half `backups()` cannot see; nothing
        // would ever remove it, inside a directory that used to be the game's.
        let p = profile();
        fs::create_dir_all(p.backups_dir()).unwrap();
        let orphan = stamped_path(&p.backups_dir().join("loadorder.txt"), 42);
        fs::write(&orphan, "x").unwrap();
        assert!(
            p.backups(BackupKind::LoadOrder).is_empty(),
            "not a restore point"
        );
        p.create_backup(BackupKind::LoadOrder).unwrap();
        assert!(!orphan.exists(), "the orphan was swept");
        cleanup(&p);
    }

    #[test]
    fn a_restore_that_cannot_finish_leaves_the_pair_as_it_was() {
        // plugins.txt and loadorder.txt describe ONE state; a failure between
        // the two writes must not leave one restored and one not.
        let p = profile();
        let b = p.create_backup(BackupKind::LoadOrder).unwrap();
        fs::write(p.plugins_txt_path(), "*Now.esp\n").unwrap();
        fs::write(p.loadorder_txt_path(), "Now.esp\n").unwrap();
        // Make the SECOND write impossible: a directory cannot be renamed over.
        fs::remove_file(p.loadorder_txt_path()).unwrap();
        fs::create_dir(p.loadorder_txt_path()).unwrap();
        assert!(p.restore_backup(BackupKind::LoadOrder, b.stamp).is_err());
        assert_eq!(
            fs::read_to_string(p.plugins_txt_path()).unwrap(),
            "*Now.esp\n",
            "the first write was rolled back"
        );
        cleanup(&p);
    }

    #[test]
    fn the_stamp_reads_as_a_date() {
        // 2026-08-24 12:00:00 UTC.
        assert_eq!(format_stamp(1_787_572_800), "2026-08-24 12:00");
        assert_eq!(format_stamp(0), "1970-01-01 00:00");
    }
}
