//! Where Eidos keeps its files.
//!
//! Eidos is one program in the Colony ecosystem, and the ecosystem answers this
//! question once, in `colony_ui::paths`, for every program in it:
//!
//! ```text
//! <platform root>/Colony/<Program>/
//! ```
//!
//! On Linux, which is the only platform Eidos targets, that gives:
//!
//! | Kind | Path | What lives there |
//! |---|---|---|
//! | [`config_dir`] | `~/.config/Colony/Eidos/` | preferences, the Nexus session, the instance list, the game and add-on definitions the user wrote |
//! | [`data_dir`] | `~/.local/share/Colony/Eidos/` | global instances (`instances/<game>/`), downloaded runtimes (`runtimes/`), session logs (`logs/`) |
//! | [`cache_dir`] | `~/.cache/Colony/Eidos/` | LOOT masterlists (`loot/<repo>/`), re-fetched when missing |
//!
//! This crate does not rebuild those roots. It asks `colony_ui::paths::locate`
//! for them, names the sub-directories Eidos uses inside them, and carries the
//! user's files over from the layout Eidos used before it joined the ecosystem.
//!
//! # Reading a path does not create it
//!
//! Every function here only joins paths and looks at what exists. Showing a path
//! on a settings screen must not bring the directory into existence, and every
//! writer in this tree already calls `create_dir_all` before it writes.
//!
//! # The move off the old layout
//!
//! An older Eidos kept its files in `~/.config/eidos`, `~/.local/share/eidos`
//! (global instances and runtimes) and `~/.local/state/eidos` (logs), and 1.18
//! moved the logs to `~/.local/state/Colony/Eidos`. [`migrate_legacy_layout`]
//! carries all of that onto the Colony layout at startup, by the ecosystem's
//! rule for a live path:
//!
//! 1. Copy first, into a staging directory beside the destination.
//! 2. Put a marker ([`MIGRATION_MARKER`]) in the copy, then rename the copy into
//!    place, so the new directory appears complete or not at all.
//! 3. Never remove or rename the old directory in the release that adds the move.
//! 4. Until the marker is down, keep using the old directory: every resolver
//!    here ([`config_dir`], [`logs_dir`], [`runtimes_dir`],
//!    [`global_instance_dir`]) returns the legacy path while it exists and the
//!    new one has no marker. A migration that fails degrades to the old
//!    location for that session, never to an empty profile.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use colony_ui::paths::locate;

/// The organisation directory every Colony program nests under.
pub use colony_ui::paths::VENDOR;

/// This program, spelled the way it spells itself. Not a lowercased slug: the
/// ecosystem's directories are `Colony/Eidos`, not `colony/eidos`.
pub const PROGRAM: &str = "Eidos";

/// One Colony root, or a scratch location when the machine has no home at all.
///
/// `locate` fails only when neither `$HOME` nor the password database names a
/// home directory: a systemd unit with an empty environment, a bare container.
/// Every caller then goes on to `create_dir_all` and write, which works in the
/// temp dir where it would fail on `/`. The `kind` level keeps config, data and
/// cache apart even there, so clearing one can never take the others with it.
///
/// A relative answer counts as none. `dirs` checks the `XDG_*` variables for
/// that but takes `$HOME` as given, and a relative one would resolve against
/// the working directory, which for Eidos under Proton is the game's folder.
fn root(found: io::Result<PathBuf>, kind: &str) -> PathBuf {
    found
        .ok()
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| std::env::temp_dir().join(kind).join(VENDOR).join(PROGRAM))
}

/// An old-layout root, or `None` where there can be nothing to carry over.
fn legacy(base: Option<PathBuf>, rel: &str) -> Option<PathBuf> {
    base.filter(|p| p.is_absolute()).map(|p| p.join(rel))
}

/// `~/.config/Colony/Eidos` - preferences, credentials, and what the user wrote.
///
/// The legacy `~/.config/eidos` while that still exists and its copy here has
/// not been completed (see the crate docs): every reader of a setting goes
/// through this, so a failed copy can never show the user an empty profile.
pub fn config_dir() -> PathBuf {
    resolved(colony_config_dir(), legacy_config_dir())
}

/// The Colony config directory itself, migrated or not.
fn colony_config_dir() -> PathBuf {
    root(locate::config_dir(PROGRAM), "config")
}

/// `~/.local/share/Colony/Eidos` - what the program produced and cannot rebuild.
///
/// The root only. Ask for what lives in it by name ([`logs_dir`],
/// [`runtimes_dir`], [`global_instance_dir`]): those fall back to their old
/// locations until they have moved, and this cannot.
pub fn data_dir() -> PathBuf {
    root(locate::data_dir(PROGRAM), "data")
}

/// `~/.cache/Colony/Eidos` - what the program can rebuild by asking again.
pub fn cache_dir() -> PathBuf {
    root(locate::cache_dir(PROGRAM), "cache")
}

/// `~/.local/share/Colony/Eidos/logs` - session logs.
///
/// Logs used to sit in `~/.local/state`, a fourth root only Linux has. The
/// Colony layout has three, and a log is something the program produced, so
/// they live in `data` like every other Colony program's.
pub fn logs_dir() -> PathBuf {
    resolved(data_dir().join("logs"), legacy_logs_dirs())
}

/// `~/.local/share/Colony/Eidos/runtimes` - downloaded tool runtimes.
pub fn runtimes_dir() -> PathBuf {
    resolved(
        data_dir().join("runtimes"),
        legacy_data_dir().map(|d| d.join("runtimes")),
    )
}

/// `~/.local/share/Colony/Eidos/instances/<game>` - the global instance of a
/// game. Portable instances live wherever the user put them and never pass
/// through here.
pub fn global_instance_dir(game_id: &str) -> PathBuf {
    resolved(
        data_dir().join("instances").join(game_id),
        legacy_data_dir().map(|d| d.join(game_id)),
    )
}

/// Where a global instance named by its OLD path lives now, once it has moved.
///
/// `Some` only for `~/.local/share/eidos/<game>` whose Colony copy is complete.
/// A Steam launch option, a `.desktop` file or a script written before the move
/// can still name the old folder, and opening it as a portable instance would
/// fork the setup in two: the old folder is still there, so nothing would fail,
/// and every change from then on would land in a tree Eidos no longer reads.
pub fn moved_global_instance(root: &Path) -> Option<PathBuf> {
    if root.parent()? != legacy_data_dir()? {
        return None;
    }
    let new = data_dir().join("instances").join(root.file_name()?);
    new.join(MIGRATION_MARKER).exists().then_some(new)
}

/// `~/.local/share/applications`, where `.desktop` launchers go.
///
/// NOT the Colony tree: a launcher belongs where the desktop looks for one,
/// which is not ours to choose.
pub fn desktop_entries_dir() -> PathBuf {
    dirs::data_dir()
        .filter(|p| p.is_absolute())
        .unwrap_or_else(std::env::temp_dir)
        .join("applications")
}

// ---------------------------------------------------------------------------
// Migration off the old layout
// ---------------------------------------------------------------------------

/// Where Eidos kept its config before the Colony layout: `~/.config/eidos`.
pub fn legacy_config_dir() -> Option<PathBuf> {
    legacy(dirs::config_dir(), "eidos")
}

/// Where Eidos kept global instances and runtimes before: `~/.local/share/eidos`.
pub fn legacy_data_dir() -> Option<PathBuf> {
    legacy(dirs::data_dir(), "eidos")
}

/// Where earlier versions wrote session logs, newest layout first:
/// `~/.local/state/Colony/Eidos/logs` (1.18), then `~/.local/state/eidos/logs`.
fn legacy_logs_dirs() -> Vec<PathBuf> {
    let state = dirs::state_dir();
    [
        legacy(state.clone(), &format!("{VENDOR}/{PROGRAM}/logs")),
        legacy(state, "eidos/logs"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The marker left in a migrated directory, naming where it came from.
///
/// Its presence is what "migrated" means. It makes the copy run once - without
/// it, a file the user deleted would be copied back from the old tree on the
/// next launch - and it is what tells every resolver here to stop falling back.
pub const MIGRATION_MARKER: &str = ".migrated-from";

/// The new directory once its marker is down; until then the first legacy
/// directory that exists; the new one when there is nothing to fall back to.
fn resolved(new: PathBuf, legacy: impl IntoIterator<Item = PathBuf>) -> PathBuf {
    if new.join(MIGRATION_MARKER).exists() {
        return new;
    }
    legacy.into_iter().find(|d| d.is_dir()).unwrap_or(new)
}

/// How [`migrate_tree`] carries a regular file across.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carry {
    /// A real copy. For the small trees an older Eidos may still read: the old
    /// config must stay exactly as it was.
    Copy,
    /// A hard link. For trees too large to copy at startup: a global instance is
    /// tens of gigabytes of mods, and duplicating it would take minutes and could
    /// fill the disk. A link costs nothing, and the old path keeps its own entry,
    /// so removing either side never removes the other. Across filesystems it
    /// fails, and the tree stays where it is.
    Link,
}

/// Copy the legacy trees onto the Colony layout, once.
///
/// Called at startup by both binaries, before anything reads a setting or opens
/// a log, so every resolver already sees the outcome. Cheap when there is
/// nothing to do: a few `stat` calls. Returns one line per thing worth telling
/// the user, for the log.
///
/// Global instances are not handled here: moving one needs the instance lock
/// and a pass over the absolute paths written into it, which belong to
/// `eidos-instance` and `eidos-transfer`.
pub fn migrate_legacy_layout() -> Vec<String> {
    let jobs = [
        (
            legacy_config_dir(),
            colony_config_dir(),
            Carry::Copy,
            "settings",
        ),
        (
            legacy_logs_dirs().into_iter().find(|d| d.is_dir()),
            data_dir().join("logs"),
            Carry::Copy,
            "log",
        ),
        (
            legacy_data_dir().map(|d| d.join("runtimes")),
            data_dir().join("runtimes"),
            Carry::Link,
            "runtime",
        ),
    ];
    let mut notes = Vec::new();
    for (from, to, how, what) in jobs {
        let Some(from) = from else { continue };
        match migrate_tree(&from, &to, how) {
            Ok(0) => {}
            Ok(n) => notes.push(format!(
                "{} {n} {what} file(s) from {} to {} - the old directory is left as it was",
                if how == Carry::Link {
                    "linked"
                } else {
                    "copied"
                },
                from.display(),
                to.display()
            )),
            Err(e) => notes.push(format!(
                "could not copy the {what} files from {} to {}: {e} - Eidos keeps using {} \
                 and tries again next launch",
                from.display(),
                to.display(),
                from.display()
            )),
        }
    }
    notes
}

/// Carry the tree at `from` over to `to`, once, and mark `to` as migrated.
///
/// Returns how many files were carried: 0 when there was nothing to do. On an
/// error `to` carries no marker, so the resolvers keep returning `from`.
///
/// - When `to` does not exist, the copy is built in a staging directory beside
///   it and renamed into place with the marker already inside: the new tree
///   appears complete or not at all. A half-done copy that the user then edited
///   in the old place could otherwise shadow those edits on the next try.
/// - When `to` exists without a marker (an earlier version whose copy failed
///   half way and then wrote there), the missing entries are filled in and
///   nothing is overwritten.
/// - `from` is never removed, renamed or written to.
/// - A symlink is recreated pointing at the same place, never followed: nothing
///   is read through a link, and a dotfile manager's links keep working.
pub fn migrate_tree(from: &Path, to: &Path, how: Carry) -> io::Result<usize> {
    if to.join(MIGRATION_MARKER).exists() || !from.is_dir() {
        return Ok(0);
    }
    let marker = format!("{}\n", from.display());
    // Resolved once, so a relative link's `..` is worked out on the real tree
    // (`~/.local/share/eidos` is often itself a link to a bigger disk). Nothing
    // under it is a link we follow, so every path below stays real.
    let from = &fs::canonicalize(from)?;

    if to.symlink_metadata().is_ok() {
        let n = carry(from, to, from, how)?;
        fs::write(to.join(MIGRATION_MARKER), marker)?;
        return Ok(n);
    }

    let parent = to
        .parent()
        .ok_or_else(|| io::Error::other(format!("{} has no parent", to.display())))?;
    fs::create_dir_all(parent)?;
    let name = to.file_name().unwrap_or_default().to_string_lossy();
    let staging = parent.join(format!(".{name}.migrating-{}", std::process::id()));
    // Only ever a leftover of a crashed run that had this same pid: the name is
    // ours, and it holds nothing but copies.
    let _ = fs::remove_dir_all(&staging);
    let done = carry(from, &staging, from, how)
        .and_then(|n| fs::write(staging.join(MIGRATION_MARKER), &marker).map(|()| n))
        .and_then(|n| fs::rename(&staging, to).map(|()| n));
    match done {
        Ok(n) => Ok(n),
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            // Another Eidos process (the window and a Steam launch, started
            // together) can finish the same move first. Its tree is complete.
            if to.join(MIGRATION_MARKER).exists() {
                Ok(0)
            } else {
                Err(e)
            }
        }
    }
}

/// Copy or link everything under `from` into `to`, never overwriting.
fn carry(from: &Path, to: &Path, root: &Path, how: Carry) -> io::Result<usize> {
    fs::create_dir_all(to)?;
    let mut n = 0;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        // `file_type` does not follow a symlink, so a link is never walked.
        let kind = entry.file_type()?;
        if kind.is_dir() {
            n += carry(&src, &dst, root, how)?;
        } else if dst.symlink_metadata().is_ok() {
            // Already there: the destination is the newer of the two.
        } else if kind.is_symlink() {
            std::os::unix::fs::symlink(relink(root, &src)?, &dst)?;
            n += 1;
        } else if kind.is_file() {
            // `fs::copy` creates the file with the source's mode, so nexus.ini,
            // which holds an OAuth token, is never readable by others, not
            // even for a moment.
            match how {
                Carry::Copy => {
                    fs::copy(&src, &dst)?;
                    keep_mtime(&src, fs::File::open(&dst));
                }
                Carry::Link => fs::hard_link(&src, &dst)?,
            }
            n += 1;
        }
        // Sockets, FIFOs and devices: nothing a profile is made of.
    }
    // Last, once nothing more is added to it: a mod folder's mtime is the
    // "installed" date the mod list shows, and every mod moved today would
    // otherwise read as installed today.
    keep_mtime(from, fs::File::open(to));
    Ok(n)
}

/// Give `dst` the modification time of `src`. Best effort: a date is not worth
/// failing a migration over. A read-only handle is enough, because setting an
/// explicit time asks for ownership, not write permission.
fn keep_mtime(src: &Path, dst: io::Result<fs::File>) {
    if let (Ok(time), Ok(dst)) = (fs::metadata(src).and_then(|m| m.modified()), dst) {
        let _ = dst.set_modified(time);
    }
}

/// The target a carried symlink gets: the place the original pointed at.
///
/// A relative link that stays inside the tree is kept as written, so it points
/// into the new tree. One that climbs out is made absolute, because from the
/// new directory the same climb would land somewhere else.
fn relink(root: &Path, link: &Path) -> io::Result<PathBuf> {
    let target = fs::read_link(link)?;
    if target.is_absolute() {
        return Ok(target);
    }
    let mut abs = link.parent().unwrap_or(root).to_path_buf();
    for part in target.components() {
        match part {
            Component::ParentDir => {
                abs.pop();
            }
            Component::CurDir => {}
            other => abs.push(other),
        }
    }
    Ok(if abs.starts_with(root) { target } else { abs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::sync::Mutex;

    /// The environment is process-wide and tests run on threads: every test
    /// that points `HOME` somewhere holds this for its whole run.
    static ENV: Mutex<()> = Mutex::new(());

    const VARS: [&str; 5] = [
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_STATE_HOME",
    ];

    /// A fresh, empty `HOME` with no XDG overrides, restored and removed on drop.
    struct Home {
        dir: PathBuf,
        saved: Vec<(&'static str, Option<OsString>)>,
        _env: std::sync::MutexGuard<'static, ()>,
    }

    impl Home {
        fn new(tag: &str) -> Home {
            let env = ENV.lock().unwrap_or_else(|e| e.into_inner());
            let dir =
                std::env::temp_dir().join(format!("eidos-paths-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            let saved = VARS.iter().map(|v| (*v, std::env::var_os(v))).collect();
            // SAFETY: every test that touches the environment holds `ENV`.
            unsafe {
                for v in VARS {
                    std::env::remove_var(v);
                }
                std::env::set_var("HOME", &dir);
            }
            Home {
                dir,
                saved,
                _env: env,
            }
        }

        fn join(&self, rel: &str) -> PathBuf {
            self.dir.join(rel)
        }

        fn write(&self, rel: &str, body: &str) {
            let p = self.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, body).unwrap();
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            // SAFETY: still holding `ENV`.
            unsafe {
                for (v, old) in &self.saved {
                    match old {
                        Some(x) => std::env::set_var(v, x),
                        None => std::env::remove_var(v),
                    }
                }
            }
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn read(p: PathBuf) -> String {
        fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    /// A legacy profile shaped like a real one: settings, a credential, a user
    /// game definition, 1.18's logs and a downloaded runtime.
    fn legacy_layout(h: &Home) {
        h.write(".config/eidos/settings.ini", "theme=dark\n");
        h.write(".config/eidos/nexus.ini", "access_token=abc\n");
        fs::set_permissions(
            h.join(".config/eidos/nexus.ini"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        h.write(".config/eidos/games/stardew.toml", "id='stardew'\n");
        h.write(
            ".local/state/Colony/Eidos/logs/gui.20261001.1.log",
            "old log\n",
        );
        h.write(".local/share/eidos/runtimes/dotnet-8/dotnet", "binary\n");
    }

    #[test]
    fn the_layout_is_vendor_then_program_and_the_roots_stay_apart() {
        let _h = Home::new("layout");
        for dir in [config_dir(), data_dir(), cache_dir()] {
            let tail: Vec<_> = dir
                .components()
                .rev()
                .take(2)
                .map(|c| c.as_os_str().to_owned())
                .collect();
            assert_eq!(tail, vec![PROGRAM, VENDOR], "{}", dir.display());
        }
        let all = [config_dir(), data_dir(), cache_dir()];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert!(!a.starts_with(b) && !b.starts_with(a), "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn a_fresh_install_uses_the_colony_layout_and_creates_nothing() {
        let h = Home::new("fresh");

        assert!(migrate_legacy_layout().is_empty());

        let data = h.join(".local/share/Colony/Eidos");
        assert_eq!(config_dir(), h.join(".config/Colony/Eidos"));
        assert_eq!(data_dir(), data);
        assert_eq!(cache_dir(), h.join(".cache/Colony/Eidos"));
        assert_eq!(logs_dir(), data.join("logs"));
        assert_eq!(runtimes_dir(), data.join("runtimes"));
        assert_eq!(
            global_instance_dir("skyrimse"),
            data.join("instances/skyrimse")
        );
        // Nothing to move means nothing written: not even an empty tree.
        assert_eq!(fs::read_dir(&h.dir).unwrap().count(), 0);
    }

    #[test]
    fn an_existing_legacy_layout_is_copied_and_left_in_place() {
        let h = Home::new("legacy");
        legacy_layout(&h);
        // Before the move, every reader still lands on the old files.
        assert_eq!(config_dir(), h.join(".config/eidos"));
        assert_eq!(logs_dir(), h.join(".local/state/Colony/Eidos/logs"));
        assert_eq!(runtimes_dir(), h.join(".local/share/eidos/runtimes"));

        let notes = migrate_legacy_layout();

        assert_eq!(notes.len(), 3, "{notes:?}");
        let (config, data) = (
            h.join(".config/Colony/Eidos"),
            h.join(".local/share/Colony/Eidos"),
        );
        assert_eq!(config_dir(), config);
        assert_eq!(read(config.join("settings.ini")), "theme=dark\n");
        assert_eq!(read(config.join("games/stardew.toml")), "id='stardew'\n");
        let mode = fs::metadata(config.join("nexus.ini")).unwrap().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "an OAuth token must not become readable by moving"
        );
        assert_eq!(logs_dir(), data.join("logs"));
        assert_eq!(read(data.join("logs/gui.20261001.1.log")), "old log\n");
        assert_eq!(runtimes_dir(), data.join("runtimes"));
        // The runtime is linked, not duplicated...
        let ino = |p: PathBuf| fs::metadata(p).unwrap().ino();
        assert_eq!(
            ino(data.join("runtimes/dotnet-8/dotnet")),
            ino(h.join(".local/share/eidos/runtimes/dotnet-8/dotnet"))
        );
        // ...and every old file is still where it was.
        assert_eq!(read(h.join(".config/eidos/settings.ini")), "theme=dark\n");
        assert!(h
            .join(".local/state/Colony/Eidos/logs/gui.20261001.1.log")
            .is_file());
        assert!(h
            .join(".local/share/eidos/runtimes/dotnet-8/dotnet")
            .is_file());
        // And no staging directory is left behind.
        for parent in [h.join(".config/Colony"), data.clone()] {
            for e in fs::read_dir(parent).unwrap().flatten() {
                assert!(!e.file_name().to_string_lossy().contains(".migrating-"));
            }
        }
    }

    #[test]
    fn a_copy_that_fails_keeps_the_legacy_directory_in_use() {
        let h = Home::new("readonly");
        legacy_layout(&h);
        let colony = h.join(".config/Colony");
        fs::create_dir_all(&colony).unwrap();
        fs::set_permissions(&colony, fs::Permissions::from_mode(0o555)).unwrap();
        // Root ignores the mode, so there is no failure to observe.
        if fs::write(colony.join("probe"), b"").is_ok() {
            return;
        }

        let notes = migrate_legacy_layout();
        fs::set_permissions(&colony, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            notes
                .iter()
                .any(|n| n.starts_with("could not copy the settings")),
            "{notes:?}"
        );
        assert_eq!(config_dir(), h.join(".config/eidos"));
        assert!(!h.join(".config/Colony/Eidos").exists());
        assert_eq!(fs::read_dir(&colony).unwrap().count(), 0, "no staging left");
        // The trees that could move did.
        assert_eq!(logs_dir(), h.join(".local/share/Colony/Eidos/logs"));
    }

    #[test]
    fn a_second_run_does_nothing_and_does_not_restore_a_deleted_file() {
        let h = Home::new("twice");
        legacy_layout(&h);
        assert!(!migrate_legacy_layout().is_empty());
        let settings = h.join(".config/Colony/Eidos/settings.ini");
        fs::remove_file(&settings).unwrap();

        assert!(migrate_legacy_layout().is_empty());

        assert!(!settings.exists(), "a deletion must not undo itself");
        assert_eq!(config_dir(), h.join(".config/Colony/Eidos"));
    }

    #[test]
    fn a_half_done_copy_is_filled_in_without_overwriting() {
        let h = Home::new("fill");
        legacy_layout(&h);
        // What a 1.18 whose copy failed half way leaves: the new directory, no
        // marker, and a file the user has since changed there.
        h.write(".config/Colony/Eidos/settings.ini", "theme=light\n");

        migrate_legacy_layout();

        let config = h.join(".config/Colony/Eidos");
        assert_eq!(read(config.join("settings.ini")), "theme=light\n");
        assert_eq!(read(config.join("nexus.ini")), "access_token=abc\n");
        assert!(config.join(MIGRATION_MARKER).is_file());
    }

    #[test]
    fn a_relative_xdg_variable_is_ignored() {
        // Not pedantry: a relative value resolves against the working directory,
        // which for Eidos under Proton is the GAME's directory.
        let h = Home::new("relative");
        // SAFETY: `Home` holds the environment lock.
        unsafe {
            std::env::set_var("XDG_DATA_HOME", "relative/share");
            std::env::set_var("XDG_CONFIG_HOME", "relative/config");
            std::env::set_var("XDG_STATE_HOME", "relative/state");
        }
        assert_eq!(data_dir(), h.join(".local/share/Colony/Eidos"));
        assert_eq!(config_dir(), h.join(".config/Colony/Eidos"));
        assert_eq!(legacy_data_dir(), Some(h.join(".local/share/eidos")));
        assert_eq!(legacy_config_dir(), Some(h.join(".config/eidos")));
        assert_eq!(
            legacy_logs_dirs()[0],
            h.join(".local/state/Colony/Eidos/logs")
        );

        // Nor is a relative HOME, which `dirs` takes as given.
        // SAFETY: as above.
        unsafe {
            std::env::set_var("HOME", "relative/home");
            std::env::remove_var("XDG_DATA_HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("XDG_STATE_HOME");
        }
        for dir in [config_dir(), data_dir(), cache_dir(), logs_dir()] {
            assert!(dir.is_absolute(), "{}", dir.display());
        }
        assert!(desktop_entries_dir().is_absolute());
        assert_eq!(legacy_data_dir(), None);
        assert!(migrate_legacy_layout().is_empty());
    }

    #[test]
    fn a_global_instance_falls_back_until_its_copy_is_complete() {
        let h = Home::new("instance");
        let old = h.join(".local/share/eidos/skyrimse");
        let new = h.join(".local/share/Colony/Eidos/instances/skyrimse");
        h.write(".local/share/eidos/skyrimse/modlist.txt", "+A\n");
        h.write(".local/share/eidos/skyrimse/mods/A/a.esp", "plugin");
        let installed = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        fs::File::open(old.join("mods/A"))
            .unwrap()
            .set_modified(installed)
            .unwrap();
        assert_eq!(global_instance_dir("skyrimse"), old);
        assert_eq!(moved_global_instance(&old), None);

        assert!(migrate_tree(&old, &new, Carry::Link).unwrap() > 0);

        assert_eq!(global_instance_dir("skyrimse"), new);
        assert_eq!(moved_global_instance(&old), Some(new.clone()));
        // The mod list's "installed" date is the folder's mtime: it moves too.
        let mtime = |p: PathBuf| fs::metadata(p).unwrap().modified().unwrap();
        assert_eq!(mtime(new.join("mods/A")), installed);
        // A portable instance somewhere else is nobody's old path.
        assert_eq!(moved_global_instance(&h.join("Games/skyrimse")), None);
        assert!(old.join("modlist.txt").is_file());
    }

    #[test]
    fn a_symlink_is_recreated_pointing_where_it_pointed() {
        let h = Home::new("links");
        let (from, to) = (h.join("old"), h.join("new"));
        h.write("old/real.ini", "x");
        h.write("elsewhere/secret", "not ours");
        std::os::unix::fs::symlink("real.ini", from.join("inside.ini")).unwrap();
        std::os::unix::fs::symlink("../elsewhere", from.join("outside")).unwrap();

        migrate_tree(&from, &to, Carry::Copy).unwrap();

        // Inside the tree: still relative, so it follows the tree.
        assert_eq!(
            fs::read_link(to.join("inside.ini")).unwrap(),
            Path::new("real.ini")
        );
        // Out of it: made absolute, so it still reaches the same folder...
        let out = fs::read_link(to.join("outside")).unwrap();
        assert_eq!(out, fs::canonicalize(h.join("elsewhere")).unwrap());
        // ...which was linked, never walked or copied.
        assert!(to
            .join("outside")
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
