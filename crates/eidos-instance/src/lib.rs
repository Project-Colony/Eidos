//! Eidos instance model, shared by the CLI and the GUI so both create and read
//! instances identically.
//!
//! An instance is one modding setup for a game. Like Mod Organizer 2 it can be:
//! - **Global**: stored centrally at `$XDG_DATA_HOME/eidos/<game-id>/`, managed
//!   by Eidos.
//! - **Portable**: a self-contained folder the user chooses (movable, isolated).
//!
//! Either way the layout is the same:
//! ```text
//! <root>/mods/<name>/...   one folder per mod
//! <root>/modlist.txt       order + enabled state (MO2 style; top = highest)
//! <root>/overwrite/        the writable layer (saves, regenerated configs)
//! <root>/overwrite/Root/   ... of it that lands beside the game's own exe
//! <root>/.base             bind-stash mountpoint for the pristine game files
//! ```
//!
//! There is ONE Overwrite, as in MO2. Writes aimed at the game install root go to
//! its `Root/` subdirectory - the same name a mod uses for the same content - so
//! turning the Overwrite into a mod yields a correctly shaped one.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod categories;
mod export;
mod manifest;
mod meta;
mod profile;
mod registry;
pub mod settings;
mod tools;
pub use export::{csv_quote, fmt_mtime, mod_list_csv, Column, ExportScope};
pub use categories::{
    format_categories, parse_all, parse_primary, Category, CategoryFactory, NexusCategory,
};
pub use manifest::Manifest;
pub use registry::{registry_path, InstanceRef, Registry};
pub use meta::ModMeta;
pub use profile::{
    cosave_siblings, format_stamp, Backup, BackupKind, is_save_data, is_save_listing, read_text_lossy, untweak_ini, write_text,
    ListTrust, Profile, SaveEntry, TweakedKey,
};
pub use settings::Settings;
pub use tools::{
    default_prereqs, default_tools, default_tools_in, merge_tools, read_tools, write_tools,
    tool_search_roots, GameExecutables, Tool,
};

/// Where an instance is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceKind {
    /// Centrally under `$XDG_DATA_HOME/eidos/<id>`.
    Global,
    /// In a self-contained folder chosen by the user.
    Portable,
}

/// One mod in the list: a folder under `mods/`, with its enabled state. Order in
/// the returned vec is priority order, highest first (wins file conflicts).
#[derive(Debug, Clone)]
pub struct ModEntry {
    pub name: String,
    pub enabled: bool,
    pub path: PathBuf,
    /// Content the GAME owns and Eidos did not install: the DLCs and Creation
    /// Club plugins that live in the game's own `Data`, which MO2 calls
    /// *unmanaged* (`ModInfoForeign`) and shows anyway.
    ///
    /// Showing them is not decoration. A mod list that lists four mods while the
    /// game will load eighty plugins invites exactly the question "is my DLC
    /// even there?", and answering it costs an evening. They are always enabled,
    /// never reordered (their priority is the engine's, not ours), never written
    /// to `modlist.txt`, and never mounted as layers - the files are already in
    /// the directory we mount over.
    pub unmanaged: bool,
}

/// Whether a mod folder name marks a SEPARATOR - MO2's `.*_separator` convention
/// for a visual group divider in the mod list. A separator is a real mod folder
/// (so it round-trips through `modlist.txt`) but contributes no files, plugins, or
/// mount layers; it only groups and labels the mods below it.
pub fn is_separator_name(name: &str) -> bool {
    name.ends_with("_separator")
}

/// Whether a mod folder name marks a BACKUP - MO2's `.*_backup` convention, and
/// derived from the name for the same reason separators are: a stored flag goes
/// stale on a rename, and this one decides whether files reach the game.
///
/// A trailing number counts. The second backup of a mod has to be called
/// something, and `X_backup2` is what it gets called; a check that only matched
/// the bare suffix would quietly let every backup after the first deploy itself
/// over the mod it copies. That is not a hypothetical - it is what the first
/// version of this function did, and what its test caught.
pub fn is_backup_name(name: &str) -> bool {
    name.trim_end_matches(|c: char| c.is_ascii_digit()).ends_with("_backup")
}

impl ModEntry {
    /// Whether this entry is a separator (derived from its folder name, like MO2 -
    /// never a stored flag, so it can't go stale on rename).
    pub fn is_separator(&self) -> bool {
        is_separator_name(&self.name)
    }

    /// Whether this entry is a saved copy of another mod rather than a mod.
    pub fn is_backup(&self) -> bool {
        is_backup_name(&self.name)
    }

    /// Whether this entry contributes files to the game.
    ///
    /// One predicate rather than the three copies of `enabled && !is_separator`
    /// this replaced, and the reason is exactly this commit: adding a second
    /// kind of inert row meant finding every one of them, and a filter that is
    /// spelled out three times is a filter that will be spelled out twice next
    /// time. A backup is deliberately NOT merely disabled - a user who ticks it
    /// would deploy two copies of the same mod over each other.
    pub fn is_active(&self) -> bool {
        self.enabled && !self.is_separator() && !self.is_backup()
    }

    /// The name shown to the user: the internal folder name with the `_separator`
    /// suffix stripped (MO2's `getDisplayName`). A normal mod is unchanged.
    pub fn display_name(&self) -> &str {
        self.name.strip_suffix("_separator").unwrap_or(&self.name)
    }

    /// Whether this entry is content the game owns rather than a mod Eidos
    /// installed. Unmanaged entries are shown, never reordered, never saved and
    /// never mounted.
    pub fn is_unmanaged(&self) -> bool {
        self.unmanaged
    }
}


/// Whether a file name is a Bethesda plugin. Kept here rather than reaching for
/// `eidos-plugins`, which depends on this crate's sibling and would make the
/// dependency circular.
fn is_plugin_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".esp") || n.ends_with(".esm") || n.ends_with(".esl")
}

/// `$XDG_DATA_HOME`, or `$HOME/.local/share`.
pub fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
            home.join(".local/share")
        })
}

/// A modding instance rooted at a directory.
#[derive(Debug, Clone)]
pub struct Instance {
    pub root: PathBuf,
}

/// A held instance lock (see [`Instance::try_lock`]). Dropping the value - or
/// the process dying, however abruptly - releases it; there is no stale-lock
/// cleanup because `flock` leaves nothing to clean.
pub struct InstanceLock {
    /// Nothing but the instance it belongs to: the descriptor is owned by the
    /// [`HELD`] table, so releasing it and forgetting it are one step.
    root: PathBuf,
}

/// Which instance roots this process currently holds, and how deep.
///
/// `flock` exists to keep OTHER processes out, and it is per-open-file-
/// description: a second descriptor is refused *even to the process that
/// already holds the first*. Every handler in the window takes the lock around
/// its write and then calls something that saves - and saving takes the lock.
/// The result was a handler deadlocking against itself, reported to the user as
/// "this instance is in use by the Eidos window", which is true and useless.
///
/// Worse than useless, in one case: renaming a mod took the lock, renamed the
/// folder, and then failed to write `modlist.txt` - so the list was reloaded
/// from a file still naming the old folder, the renamed one looked like a mod
/// nobody had seen before, and it landed at the top of the list, disabled.
///
/// So the lock is re-entrant WITHIN A THREAD. Keyed by thread, not by process:
/// two threads racing the same instance is a real conflict and still refused,
/// which is what the background tasks need.
/// One in-process holder: who has it, how deep, and THE OPEN DESCRIPTOR.
///
/// The file lives in the table rather than in the guard because that is what
/// ties the kernel's opinion to ours. Held separately, the two drift: a guard
/// whose `drop` body runs before its fields are dropped removes the entry while
/// the flock is still held, and a caller in that window is told the instance is
/// free, asks the kernel, and is refused by its own process.
struct Held {
    root: PathBuf,
    owner: std::thread::ThreadId,
    depth: usize,
    _file: std::fs::File,
}

static HELD: std::sync::Mutex<Vec<Held>> = std::sync::Mutex::new(Vec::new());

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let me = std::thread::current().id();
        let Ok(mut held) = HELD.lock() else { return };
        if let Some(pos) = held.iter().position(|e| e.root == self.root && e.owner == me) {
            held[pos].depth -= 1;
            if held[pos].depth == 0 {
                // Removing the entry closes the descriptor, releasing the flock -
                // both under the mutex, so no one can observe one without the
                // other.
                held.swap_remove(pos);
            }
        }
    }
}

impl Instance {
    /// A global instance for a game id: `$XDG_DATA_HOME/eidos/<id>`.
    pub fn global(game_id: &str) -> Self {
        Instance { root: data_home().join("eidos").join(game_id) }
    }

    /// A portable instance at an explicit folder.
    pub fn portable(root: PathBuf) -> Self {
        Instance { root }
    }

    /// Take this instance's cross-process exclusive lock, without blocking.
    ///
    /// The GUI, the CLI and a running `eidos play` are separate PROCESSES writing
    /// the same profile files, with nothing between them: two concurrent runs
    /// interleaved their deploy/capture cycles, and a GUI edit mid-game wrote
    /// into the live bound plugins dir. `flock(2)` is advisory but every writer
    /// in this codebase goes through here, it dies with the process (a crashed
    /// holder cannot wedge the instance), and it is shared across mount
    /// namespaces so the launched game's wrapper cannot dodge it.
    ///
    /// Held for the whole run by `eidos play`; taken briefly around GUI and CLI
    /// mutations. `WouldBlock` means someone else has it - report WHO from the
    /// lockfile contents rather than a bare errno.
    pub fn try_lock(&self, holder: &str) -> std::io::Result<InstanceLock> {
        let me = std::thread::current().id();
        fs::create_dir_all(&self.root)?;
        let key = self.root.clone();
        // The table is held for the WHOLE acquisition, kernel call included.
        // Releasing it before calling `flock` leaves a gap in which two callers
        // both see "free", both open the file, and one is refused by the other -
        // inside the same process, where the answer should have been decided by
        // the table and never have reached the kernel at all.
        let mut held = HELD
            .lock()
            .map_err(|_| std::io::Error::other("the instance lock table is poisoned"))?;

        if let Some(e) = held.iter_mut().find(|e| e.root == key) {
            if e.owner == me {
                // Already ours on this thread: hand back a depth rather than
                // asking the kernel for a second descriptor it would refuse.
                e.depth += 1;
                return Ok(InstanceLock { root: key });
            }
            // Another thread of OUR process. A real conflict, answered here
            // rather than by an errno, because "in use by the Eidos window" is
            // useless when the Eidos window IS the caller.
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                format!(
                    "{} is already held by this process on another thread ({:?})",
                    self.root.display(),
                    e.owner
                ),
            ));
        }

        let path = key.join(".eidos.lock");
        let file = fs::OpenOptions::new().create(true).truncate(false).write(true).open(&path)?;
        // SAFETY: flock on an owned, open fd; no memory preconditions.
        let rc =
            unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&file), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            let err = std::io::Error::last_os_error();
            let who = fs::read_to_string(&path).unwrap_or_default();
            let who = who.trim();
            // Name the instance. "this instance is in use" is unfalsifiable from
            // the outside: with several instances open, or a background task on
            // another, the reader cannot tell WHICH one refused, or whether the
            // refusal is even about the thing they just clicked.
            let root = self.root.display();
            return Err(std::io::Error::new(
                err.kind(),
                if who.is_empty() {
                    format!("another Eidos process is using {root}")
                } else {
                    format!("{root} is in use by {who}")
                },
            ));
        }
        // Best-effort breadcrumb for the refusal message above. Truncate AFTER
        // locking, so a failed attempt cannot blank the holder's note.
        let _ = file.set_len(0);
        use std::io::Write;
        let mut f = &file;
        let _ = write!(f, "{holder} (pid {})", std::process::id());
        held.push(Held { root: key.clone(), owner: me, depth: 1, _file: file });
        Ok(InstanceLock { root: key })
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.root.join("mods")
    }

    /// The `meta.ini` path for a mod (`mods/<name>/meta.ini`).
    pub fn meta_path(&self, name: &str) -> PathBuf {
        self.mods_dir().join(name).join("meta.ini")
    }

    /// MO2-compatible metadata for a mod (`mods/<name>/meta.ini`); empty if none.
    pub fn mod_meta(&self, name: &str) -> ModMeta {
        ModMeta::read(&self.meta_path(name))
    }

    /// The enabled INI-tweak fragments across a mod list, in application order
    /// (lowest priority first, so a higher-priority mod's fragment wins).
    ///
    /// Only fragments that exist on disk are returned: a mod's `meta.ini` can name
    /// one that a later reinstall dropped, and a launch must not fail over that.
    pub fn enabled_ini_tweaks(&self, mods: &[ModEntry]) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for m in mods.iter().filter(|m| m.is_active()) {
            let dir = ini_tweaks_dir(&m.path);
            for name in self.mod_meta(&m.name).ini_tweaks() {
                let p = dir.join(name);
                if p.is_file() {
                    out.push(p);
                }
            }
        }
        out
    }

    /// The content the GAME owns that Eidos did not install: the DLCs and
    /// Creation Club plugins sitting in its own `Data`. MO2's `ModInfoForeign`,
    /// discovered the same way `GamebryoUnmangedMods::mods` does it.
    ///
    /// One entry per `.esp`/`.esl`/`.esm` in the game's data directory, named
    /// after the file WITHOUT its extension (MO2 chops it), minus the primary
    /// masters - those are the base game, not add-ons, and MO2 excludes them by
    /// the same rule. Anything a managed mod already provides is skipped too: a
    /// mod that replaces a DLC plugin owns that row.
    ///
    /// Returned lowest-priority-first, matching the display order the mod list
    /// uses, because that is where they belong: the engine loads them before
    /// anything a user installed.
    /// The content the GAME owns that Eidos did not install: the base masters,
    /// the DLCs and the Creation Club plugins sitting in its own `Data`. MO2's
    /// `ModInfoForeign`, discovered the way `GamebryoUnmangedMods::mods` does it -
    /// one entry per `.esp`/`.esl`/`.esm` in the data directory, named after the
    /// file WITHOUT its extension.
    ///
    /// Everything is listed, base game included. MO2 hides the primary masters
    /// from this list because they also appear in its plugin list, but a mod list
    /// showing four rows beside eighty loading plugins is what makes a user ask
    /// whether their DLC is even installed - and answering that took an evening.
    ///
    /// `engine_order` is the order the ENGINE imposes on this content: the primary
    /// masters followed by the `.ccc` entries. Names it lists come first, in its
    /// order; anything else follows alphabetically. Sorting these by name instead
    /// would put `_ResourcePack` above `Skyrim.esm`, which is exactly backwards.
    ///
    /// Anything a managed mod already provides is skipped: that mod shadows the
    /// game's copy through the mount, so listing both would be a lie.
    pub fn unmanaged_mods(
        &self,
        game_data: &Path,
        engine_order: &[String],
        managed: &[ModEntry],
    ) -> Vec<ModEntry> {
        // A managed mod providing the same plugin owns the row.
        let provided: HashSet<String> = managed
            .iter()
            .filter(|m| !m.is_separator() && !m.is_backup())
            .flat_map(|m| fs::read_dir(&m.path).into_iter().flatten().flatten())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| is_plugin_name(n))
            .map(|n| n.to_ascii_lowercase())
            .collect();

        // Ranked by STEM, because that is what the display name already is - the
        // alternative is rebuilding each file name to look the rank up, for no
        // gain. It also means a `.ccc` naming an extension the shipped file does
        // not use still matches.
        let stem = |n: &str| n.trim().rsplit_once('.').map_or(n.trim(), |(s, _)| s).to_ascii_lowercase();
        let rank: HashMap<String, usize> =
            engine_order.iter().enumerate().map(|(i, n)| (stem(n), i)).collect();

        let Ok(rd) = fs::read_dir(game_data) else { return Vec::new() };
        let mut out: Vec<ModEntry> = rd
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| is_plugin_name(n))
            .filter(|n| !provided.contains(&n.to_ascii_lowercase()))
            .map(|n| {
                let path = game_data.join(&n);
                let name = n.rsplit_once('.').map_or(n.clone(), |(stem, _)| stem.to_string());
                ModEntry { name, enabled: true, path, unmanaged: true }
            })
            .collect();
        // Engine order first, then the rest by name. `usize::MAX` parks unknown
        // names after everything the engine named, without a second pass.
        out.sort_by_key(|m| {
            let r = rank.get(&m.name.to_ascii_lowercase()).copied().unwrap_or(usize::MAX);
            (r, m.name.to_lowercase())
        });
        out
    }

    /// The category catalog: the instance's `categories.dat` if present (MO2
    /// format), else MO2's built-in defaults. Resolves a mod's `category=` ids to
    /// display names.
    pub fn category_factory(&self) -> CategoryFactory {
        CategoryFactory::load(&self.root)
    }

    /// Where the category catalog lives (the instance root, holding MO2's own
    /// `categories.dat` + `nexuscatmap.dat`, so a shared instance keeps one
    /// catalog and not two). Pass this to [`CategoryFactory::save`].
    pub fn categories_root(&self) -> &Path {
        &self.root
    }

    /// The instance manifest path (`<root>/eidos-instance.ini`).
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("eidos-instance.ini")
    }

    /// Read the instance manifest, if present.
    pub fn read_manifest(&self) -> Option<Manifest> {
        Manifest::read(&self.manifest_path())
    }

    /// Write the instance manifest if it is missing (so we don't churn one that
    /// already exists, e.g. on every launch).
    pub fn ensure_manifest(&self, game_id: &str, kind: InstanceKind) -> std::io::Result<()> {
        if self.manifest_path().exists() {
            return Ok(());
        }
        Manifest::new(game_id, kind).write(&self.manifest_path())
    }

    /// Whether a prospective instance root lives inside (or is) a game's
    /// install directory - the one place an instance must never live, however
    /// natural it feels to MO2 veterans. Two reasons, both fatal:
    ///
    /// - Steam owns that tree: updates, "verify integrity" and uninstalls
    ///   rewrite or delete it, and an uninstall would take the entire modding
    ///   setup with it.
    /// - Eidos mounts over the game root (Root/ files beside the exe, the
    ///   pristine tree bind-stashed into `<root>/.base`). An instance inside
    ///   the install sits inside its own mount target: the union would be
    ///   serving layers through itself.
    ///
    /// Symlinks are resolved before comparing, so `/mnt/link-to-game/Eidos`
    /// cannot sneak past a lexical check.
    pub fn root_inside_game(root: &Path, install: &Path) -> bool {
        // A root being CREATED does not exist yet: resolve its parent then
        // reattach the final component, so a symlinked parent still counts.
        let canon = |p: &Path| {
            fs::canonicalize(p).unwrap_or_else(|_| match (p.parent(), p.file_name()) {
                (Some(parent), Some(name)) => fs::canonicalize(parent)
                    .map(|c| c.join(name))
                    .unwrap_or_else(|_| p.to_path_buf()),
                _ => p.to_path_buf(),
            })
        };
        canon(root).starts_with(canon(install))
    }

    /// Open an existing folder as an instance, requiring it to be
    /// self-describing: the manifest names the game, so the caller needs
    /// nothing but the path. This is the resolution behind `eidos <cmd> <path>`
    /// and behind reopening a registered portable root - guessing the game
    /// from a folder the user named freely is exactly the mistake the manifest
    /// exists to prevent.
    ///
    /// The errors are user-facing sentences: every caller is one `eprintln!`
    /// or status line away from the person who typed the path.
    pub fn open_at(root: &Path) -> Result<(Instance, Manifest), String> {
        if !root.is_dir() {
            return Err(format!("'{}' is not a directory.", root.display()));
        }
        let inst = Instance::portable(root.to_path_buf());
        match inst.read_manifest() {
            Some(m) => Ok((inst, m)),
            None if inst.exists() => Err(format!(
                "'{}' looks like an instance (it has a mods/ folder) but has no readable \
                 eidos-instance.ini naming its game. Open it once via the GUI wizard \
                 (pick its game and this folder) to adopt it.",
                root.display()
            )),
            None => Err(format!("'{}' is not an Eidos instance folder.", root.display())),
        }
    }

    /// The instance's game id: from the manifest, else the last path component
    /// (correct for a global instance, whose folder is named after the game).
    pub fn game_id(&self) -> Option<String> {
        self.read_manifest()
            .map(|m| m.game_id)
            .or_else(|| self.root.file_name().map(|s| s.to_string_lossy().into_owned()))
    }

    pub fn overwrite_dir(&self) -> PathBuf {
        self.root.join("overwrite")
    }

    /// Whether the Overwrite currently holds anything.
    pub fn overwrite_is_empty(&self) -> bool {
        fs::read_dir(self.overwrite_dir()).into_iter().flatten().flatten().next().is_none()
    }

    /// MO2's "Create mod from Overwrite" / "Move content to mod": move everything
    /// the game wrote into `mods/<name>/`, leaving the Overwrite empty.
    ///
    /// `name` must be a plain folder name. An existing mod is MERGED into
    /// (matching MO2's move-into-existing-mod), a new one gets a minimal
    /// `meta.ini`. Both live under the instance root, so the moves are renames
    /// rather than copies. Returns the mod folder's path.
    pub fn overwrite_into_mod(&self, name: &str) -> std::io::Result<PathBuf> {
        use std::io::{Error, ErrorKind};
        let name = name.trim();
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid mod name"));
        }
        let src = self.overwrite_dir();
        if self.overwrite_is_empty() {
            return Err(Error::new(ErrorKind::NotFound, "the Overwrite is empty"));
        }
        let dest = self.mods_dir().join(name);
        let fresh = !dest.exists();
        fs::create_dir_all(&dest)?;
        move_tree(&src, &dest)?;
        if fresh {
            // The same minimal meta.ini `create_empty_mod` writes, so the new mod
            // reads back like any other.
            let _ =
                fs::write(dest.join("meta.ini"), "[General]\nmodid=0\nversion=\nendorsed=0\ntracked=0\n");
        }
        Ok(dest)
    }

    /// A metadata-only picture of the Overwrite, taken BEFORE a tool runs, so
    /// what the tool produced can be told apart from what was already there.
    ///
    /// Metadata only, no hashing: the Overwrite is normally small and this runs on
    /// the launch path. But not just (len, mtime), which is what an incremental
    /// build tool would use - a build tool is allowed to be wrong occasionally and
    /// this is not. See [`FileStamp`].
    pub fn overwrite_snapshot(&self) -> OverwriteSnapshot {
        let mut out = OverwriteSnapshot::default();
        snapshot_into(&self.overwrite_dir(), Path::new(""), &mut out);
        out
    }

    /// Move everything the run WROTE - and only that - out of the Overwrite and
    /// into `mods/<name>/`. Returns how many files moved.
    ///
    /// This is Eidos's answer to MO2's "Create files in mod instead of
    /// overwrite", and it is deliberately not the obvious port. MO2 flips a
    /// usvfs create-target flag; the equivalent here would be handing the mod's
    /// directory to the FUSE union as its write layer, which breaks three things
    /// at once:
    ///
    /// * the overwrite is consulted BEFORE the layers on every read, so the mod
    ///   would be silently promoted to top priority for the whole run and every
    ///   conflict it is in would flip - and flip back on the next run;
    /// * a directory that is both a layer and the write target needs no copy-up,
    ///   so writes would go straight THROUGH the mod's own files, destroying the
    ///   "mod sources stay pristine" invariant with no undo;
    /// * and the real Overwrite would leave the union entirely, so the tool
    ///   would stop seeing its own previously-generated output.
    ///
    /// Capturing afterwards reaches the same end state - the output is in the
    /// mod, the Overwrite is clean - with none of that. `Root/` output needs no
    /// special case: it lands at `mods/<name>/Root/`, which is already the
    /// convention a mod uses for game-root files.
    pub fn capture_overwrite_into_mod(
        &self,
        name: &str,
        before: &OverwriteSnapshot,
    ) -> std::io::Result<usize> {
        use std::io::{Error, ErrorKind};
        let name = name.trim();
        if !crate::tools::is_mod_folder_name(name) {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid mod name"));
        }
        let src = self.overwrite_dir();
        let dest = self.mods_dir().join(name);
        let fresh = !dest.exists();
        let mut after = OverwriteSnapshot::default();
        snapshot_into(&src, Path::new(""), &mut after);

        // Sorted, so a failure part-way through is reproducible rather than
        // dependent on hash order - and so the caller's error names a
        // deterministic point.
        let mut produced: Vec<&PathBuf> = after
            .files
            .iter()
            .filter(|(rel, stamp)| before.files.get(*rel) != Some(*stamp))
            .map(|(rel, _)| rel)
            .collect();
        produced.sort();

        let mut moved = 0usize;
        let mut failures: Vec<String> = Vec::new();
        for rel in produced {
            let to = dest.join(rel);
            // Every step is fallible and NONE of them aborts the loop. A type
            // conflict on one path (a file where this run wants a directory, or
            // the reverse) used to propagate with `?`, splitting the run's output
            // between the Overwrite and the mod at whatever point it hit - which
            // is the worst of both places, because neither is complete.
            if let Err(e) = move_one_into(&self.mods_dir(), &src.join(rel), &to) {
                failures.push(format!("{}: {e}", rel.display()));
                continue;
            }
            moved += 1;
        }
        if moved == 0 {
            // Nothing was produced. Do not leave an empty mod behind.
            if fresh {
                let _ = fs::remove_dir_all(&dest);
            }
            return match failures.first() {
                Some(first) => Err(std::io::Error::other(format!(
                    "{} file(s) could not be captured, e.g. {first}",
                    failures.len()
                ))),
                None => Ok(0),
            };
        }
        if fresh {
            let _ =
                fs::write(dest.join("meta.ini"), "[General]\nmodid=0\nversion=\nendorsed=0\ntracked=0\n");
        }
        // A capture that creates the mod has to REGISTER it, or the output is in
        // the instance and invisible: reconciliation lists an unknown folder as
        // disabled, `load_order` drops disabled mods, and the tool would
        // regenerate the same files on the next run having achieved nothing.
        // Enabled and at the end of the display order, which is highest priority
        // - generated output is meant to win, and that is where the GUI's own
        // "create mod from Overwrite" puts it too.
        if fresh {
            let mut ml = self.modlist();
            // The folder is already on disk by now, so reconciliation has
            // ALREADY put it in this list - appended DISABLED, which is what it
            // does with any folder it has not seen before. So the job is to
            // enable it, not to add it. Only when the capture created the mod:
            // a mod the user disabled on purpose must not be switched back on
            // behind them.
            match ml.iter_mut().find(|m| m.name.eq_ignore_ascii_case(name)) {
                Some(entry) => entry.enabled = true,
                None => ml.push(ModEntry {
                    name: name.to_string(),
                    enabled: true,
                    path: dest.clone(),
                    unmanaged: false,
                }),
            }
            let _ = self.save_modlist(&ml);
        }
        prune_empty_dirs(&src, before);
        if let Some(first) = failures.first() {
            return Err(std::io::Error::other(format!(
                "captured {moved} file(s); {} could not be moved, e.g. {first}",
                failures.len()
            )));
        }
        Ok(moved)
    }

    /// Move Overwrite files back into the mods that already provide that path
    /// (MO2's "Sync to Mods"). Returns how many moved.
    ///
    /// `owners` maps a relative path, lowercased, to the mod folder that should
    /// take it. The caller supplies it because the "who provides this path"
    /// question is already answered by the conflict map the window keeps for its
    /// own columns - recomputing it here would be a second, divergent answer to
    /// a question that already has one.
    ///
    /// This is how you clean up after a tool run without creating a junk mod:
    /// the files a generator regenerated go back to the mod they came from, and
    /// only what nothing else provides is left in the Overwrite for the user to
    /// decide about.
    pub fn sync_overwrite_to_mods(
        &self,
        owners: &HashMap<String, String>,
    ) -> std::io::Result<(usize, Vec<String>)> {
        let src = self.overwrite_dir();
        let mut snapshot = OverwriteSnapshot::default();
        snapshot_into(&src, Path::new(""), &mut snapshot);

        // Sorted, so a partial failure is reproducible rather than hash-ordered.
        let mut rels: Vec<&PathBuf> = snapshot.files.keys().collect();
        rels.sort();

        let mut moved = 0usize;
        let mut failures: Vec<String> = Vec::new();
        for rel in rels {
            let key = rel.to_string_lossy().to_ascii_lowercase();
            // Only paths some mod already provides. Anything else is genuinely
            // new output with no home to go back to, and inventing one would be
            // the junk mod this exists to avoid.
            let Some(owner) = owners.get(&key) else { continue };
            if !crate::tools::is_mod_folder_name(owner) {
                failures.push(format!("{}: '{owner}' is not a mod folder name", rel.display()));
                continue;
            }
            let dest = self.mods_dir().join(owner).join(rel);
            if let Err(e) = move_one_into(&self.mods_dir(), &src.join(rel), &dest) {
                failures.push(format!("{}: {e}", rel.display()));
                continue;
            }
            moved += 1;
        }
        if moved > 0 {
            prune_empty_dirs(&src, &snapshot);
        }
        Ok((moved, failures))
    }

    /// Bind-stash mountpoint for the pristine game files (used at launch).
    pub fn base_dir(&self) -> PathBuf {
        self.root.join(".base")
    }

    /// Bind-stash mountpoint for the pristine GAME ROOT, used when mods provide
    /// root-level files (MO2's Root Builder) and a second union covers the game
    /// install directory. Separate from [`Self::base_dir`], which stashes Data.
    pub fn base_root_dir(&self) -> PathBuf {
        self.root.join(".base-root")
    }

    /// Where writes to the GAME INSTALL ROOT land: a `Root/` subdirectory of the
    /// one Overwrite, not a second Overwrite of its own.
    ///
    /// MO2 has exactly one Overwrite and so does this. The subdirectory is not an
    /// arbitrary bucket either - it is the SAME convention a mod uses, which ships
    /// its game-root content in `<mod>/Root/`. So "turn the Overwrite into a mod"
    /// produces a mod already shaped correctly, and a file the user drags from one
    /// to the other keeps its meaning.
    ///
    /// This used to be `.base-root.root-overwrite`, a hidden sibling the GUI never
    /// listed. A game or tool writing beside its own exe put files there that the
    /// user had no way to see: BodySlide, misconfigured to output one directory too
    /// high, put 1442 built meshes in it silently.
    pub fn root_overwrite_dir(&self) -> PathBuf {
        self.overwrite_dir().join("Root")
    }

    /// Move anything left in the pre-`Root/` overwrite into its new home, once.
    /// Returns how many top-level entries moved. Idempotent: the legacy directory
    /// is removed when it empties, so later calls find nothing to do.
    pub fn migrate_root_overwrite(&self) -> std::io::Result<usize> {
        let legacy = self.root.join(".base-root.root-overwrite");
        let n = fs::read_dir(&legacy).into_iter().flatten().flatten().count();
        if n == 0 {
            // Also clears away an empty legacy dir left by an earlier run.
            let _ = fs::remove_dir(&legacy);
            return Ok(0);
        }
        let dest = self.root_overwrite_dir();
        fs::create_dir_all(&dest)?;
        move_tree(&legacy, &dest)?;
        let _ = fs::remove_dir(&legacy);
        Ok(n)
    }

    /// The `Root/` directories of the enabled mods, highest priority FIRST.
    ///
    /// A mod ships its game-root content (a script extender, ENB, ReShade, an
    /// `.asi` loader, Engine Fixes' `.toml`) in a `Root/` subdirectory, matched
    /// case-insensitively because archives spell it every way. Mods without one
    /// contribute nothing, so an ordinary load order returns an empty vec and no
    /// second mount happens at all.
    pub fn root_layers(&self) -> Vec<PathBuf> {
        // `modlist()` is display order (lowest priority first); the union wants
        // highest first, so walk it in reverse.
        self.modlist()
            .into_iter()
            .rev()
            .filter(|m| m.is_active())
            .filter_map(|m| find_root_dir(&m.path))
            .collect()
    }

    /// Downloaded mod archives land here (`<root>/downloads/`), each with its
    /// MO2-format `.meta` sidecar; shared by all profiles like `mods/`.
    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    /// The instance's tool list (`<root>/tools.ini`), user entries only - merge
    /// with per-game defaults via [`merge_tools`].
    pub fn tools(&self) -> Vec<Tool> {
        read_tools(&self.root.join("tools.ini"))
    }

    /// Persist the user's tool list.
    pub fn save_tools(&self, tools: &[Tool]) -> std::io::Result<()> {
        write_tools(&self.root.join("tools.ini"), tools)
    }

    pub fn exists(&self) -> bool {
        self.mods_dir().is_dir()
    }

    /// Create the `mods/` and `overwrite/` directories.
    pub fn create(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.mods_dir())?;
        fs::create_dir_all(self.overwrite_dir())?;
        // Never fatal: an instance that cannot be migrated is still perfectly
        // usable, it just keeps showing the old hidden directory.
        let _ = self.migrate_root_overwrite();
        self.ensure_profiles()?;
        Ok(())
    }

    /// The active profile's mod list plus whether it is fit to persist - see
    /// [`Profile::modlist_checked`]. A front end that saves the list back should
    /// use this and surface the reason, rather than discovering the refusal at
    /// write time.
    pub fn modlist_checked(&self) -> (Vec<ModEntry>, ListTrust) {
        self.active().modlist_checked()
    }

    /// The active profile's mod list (folders in the shared `mods/`, in priority
    /// order with enabled state).
    ///
    /// DISPLAY order, which is MO2's: the top of the list is the LOWEST priority
    /// and the bottom row wins every file conflict. `modlist.txt` stores the
    /// opposite orientation and [`Profile::modlist_checked`] flips it;
    /// [`Profile::load_order`] flips it back, because the union wants its layers
    /// highest-priority first. This comment used to say the top was highest,
    /// which is exactly backwards - and a wrong contract on a public method is
    /// read once and believed everywhere.
    pub fn modlist(&self) -> Vec<ModEntry> {
        self.active().modlist()
    }

    /// Persist the active profile's mod list.
    pub fn save_modlist(&self, mods: &[ModEntry]) -> std::io::Result<()> {
        self.active().save_modlist(mods)
    }

    /// Enabled mods of the active profile, highest priority first.
    pub fn load_order(&self) -> Vec<PathBuf> {
        self.active().load_order()
    }

    /// The active profile's save files (newest first), MO2's savegame list.
    pub fn savegames(&self) -> Vec<crate::SaveEntry> {
        self.active().savegames()
    }

    // ---- profiles ----

    /// `<root>/profiles/`.
    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    /// All profile names (at least `Default`).
    pub fn profiles(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(self.profiles_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        v.sort();
        if v.is_empty() {
            v.push("Default".to_string());
        }
        v
    }

    /// The profile of the given name (not necessarily existing on disk yet).
    pub fn profile(&self, name: &str) -> Profile {
        Profile { instance_root: self.root.clone(), name: name.to_string() }
    }

    /// The active profile name (from the manifest; `Default` if unset). If the
    /// manifest names a profile whose directory no longer exists (renamed or
    /// deleted out from under the manifest), fall back to the first existing
    /// profile rather than launching a ghost profile - which, lacking a
    /// `modlist.txt`, would silently enable every mod. Mirrors MO2's
    /// `OrganizerCore` profile-existence fallback.
    pub fn active_profile(&self) -> String {
        let selected = self
            .read_manifest()
            .and_then(|m| m.selected_profile)
            .unwrap_or_else(|| "Default".to_string());
        if self.profiles_dir().join(&selected).is_dir() {
            return selected;
        }
        self.profiles()
            .into_iter()
            .next()
            .unwrap_or_else(|| "Default".to_string())
    }

    /// Set the active profile, persisted in the manifest (if one exists).
    pub fn set_active_profile(&self, name: &str) -> std::io::Result<()> {
        if let Some(mut m) = self.read_manifest() {
            m.selected_profile = Some(name.to_string());
            m.write(&self.manifest_path())?;
        }
        Ok(())
    }

    /// The active [`Profile`].
    pub fn active(&self) -> Profile {
        self.profile(&self.active_profile())
    }

    /// Ensure a `Default` profile exists, migrating a legacy flat `modlist.txt`
    /// (a pre-profiles instance) into it. Idempotent.
    pub fn ensure_profiles(&self) -> std::io::Result<()> {
        let default_dir = self.profiles_dir().join("Default");
        fs::create_dir_all(&default_dir)?;
        let legacy = self.root.join("modlist.txt");
        let migrated = default_dir.join("modlist.txt");
        if legacy.exists() && !migrated.exists() {
            fs::rename(&legacy, &migrated)?;
        }
        Ok(())
    }

    /// Rename a profile, keeping the manifest's active-profile pointer consistent:
    /// if the renamed profile was the active one, the pointer follows it (so it
    /// never dangles). Refuses a no-op, a missing source, or an existing target.
    /// Use this rather than [`Profile::rename`] directly so the manifest stays sound.
    pub fn rename_profile(&self, old: &str, new: &str) -> std::io::Result<()> {
        use std::io::{Error, ErrorKind};
        if new.trim().is_empty() || old == new {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid new profile name"));
        }
        // A separator (or a dot-component) would escape profiles/ - the GUI already
        // filters these, but the library must hold on its own.
        if new.contains(['/', '\\']) || new == "." || new == ".." {
            return Err(Error::new(ErrorKind::InvalidInput, "profile names cannot contain path separators"));
        }
        if !self.profile(old).dir().is_dir() {
            return Err(Error::new(ErrorKind::NotFound, format!("no profile '{old}'")));
        }
        if self.profile(new).dir().exists() {
            return Err(Error::new(ErrorKind::AlreadyExists, format!("profile '{new}' exists")));
        }
        // Capture whether the manifest pointed at `old` BEFORE the rename: afterwards
        // `old`'s directory is gone and active_profile() would already have fallen back.
        let was_active =
            self.read_manifest().and_then(|m| m.selected_profile).as_deref() == Some(old);
        self.profile(old).rename(new)?;
        if was_active {
            self.set_active_profile(new)?;
        }
        Ok(())
    }

    /// Delete a profile. Refuses to delete the ACTIVE profile or the LAST remaining
    /// one (MO2 disables both - you must switch away / keep at least one), so the
    /// manifest can never point at a deleted profile.
    pub fn delete_profile(&self, name: &str) -> std::io::Result<()> {
        use std::io::{Error, ErrorKind};
        if self.active_profile() == name {
            return Err(Error::new(ErrorKind::InvalidInput, "cannot delete the active profile"));
        }
        if self.profiles().len() <= 1 {
            return Err(Error::new(ErrorKind::InvalidInput, "cannot delete the last profile"));
        }
        self.profile(name).delete()
    }

    // ---- mod creation ----

    /// Create an empty mod folder (`mods/<name>/`) with a minimal `meta.ini`,
    /// MO2's "Create empty mod". Returns the [`ModEntry`] so the caller can splice
    /// it into the active profile's list. Refuses an empty, path-separated, or
    /// already-existing name; the new folder is enabled by default.
    pub fn create_empty_mod(&self, name: &str) -> std::io::Result<ModEntry> {
        use std::io::{Error, ErrorKind};
        let name = name.trim();
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid mod name"));
        }
        let dest = self.mods_dir().join(name);
        if dest.exists() {
            return Err(Error::new(ErrorKind::AlreadyExists, format!("mod '{name}' exists")));
        }
        fs::create_dir_all(&dest)?;
        // A minimal meta.ini, mirroring MO2's createMod.
        fs::write(dest.join("meta.ini"), "[General]\nmodid=0\nversion=\nendorsed=0\ntracked=0\n")?;
        Ok(ModEntry { name: name.to_string(), enabled: true, path: dest, unmanaged: false })
    }

    /// Import an existing Mod Organizer 2 profile into this instance's ACTIVE
    /// profile: the mod order and enabled states from its `modlist.txt`, plus its
    /// plugin state (`plugins.txt` / `loadorder.txt`) verbatim.
    ///
    /// Eidos already speaks MO2's formats, so this is a filter-and-copy: only mods
    /// whose folder actually exists under `mods/` are taken (matched
    /// case-insensitively, since MO2 ran on a case-insensitive filesystem), any
    /// local mod MO2 never knew about is appended at the bottom, and everything
    /// MO2 listed but we do not have is reported rather than silently dropped.
    pub fn import_mo2_profile(&self, mo2_profile_dir: &Path) -> std::io::Result<Mo2Import> {
        use std::io::{Error, ErrorKind};
        let src_modlist = mo2_profile_dir.join("modlist.txt");
        if !src_modlist.is_file() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("no modlist.txt in {}", mo2_profile_dir.display()),
            ));
        }
        // Our mods, keyed by lowercased folder name.
        let present: HashMap<String, ModEntry> =
            self.modlist().into_iter().map(|m| (m.name.to_ascii_lowercase(), m)).collect();

        let text = fs::read_to_string(&src_modlist)?;
        let mut ordered: Vec<ModEntry> = Vec::new();
        let mut taken: HashSet<String> = HashSet::new();
        let mut missing: Vec<String> = Vec::new();
        // MO2 writes highest priority first; our in-memory list is display order
        // (lowest first), so collect then reverse.
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (enabled, name) = match line.split_at(1) {
                ("+", rest) => (true, rest.trim()),
                ("-", rest) => (false, rest.trim()),
                // MO2 marks unmanaged/foreign mods with '*'; we do not model those.
                ("*", _) => continue,
                _ => (true, line),
            };
            if name.is_empty() {
                continue;
            }
            let key = name.to_ascii_lowercase();
            match present.get(&key) {
                Some(m) if taken.insert(key) => {
                    ordered.push(ModEntry { enabled, ..m.clone() });
                }
                Some(_) => {} // duplicate line
                None => missing.push(name.to_string()),
            }
        }
        ordered.reverse();
        let matched = ordered.len();

        // Anything of ours MO2 did not list keeps its state, at the bottom
        // (lowest priority), so importing never loses a locally-installed mod.
        let mut kept_local = 0usize;
        let mut final_list: Vec<ModEntry> = Vec::new();
        for m in self.modlist() {
            if !taken.contains(&m.name.to_ascii_lowercase()) {
                final_list.push(m);
                kept_local += 1;
            }
        }
        final_list.extend(ordered);
        self.save_modlist(&final_list)?;

        // The plugin state transfers verbatim - the formats are identical. Into
        // the plugins STATE dir: the legacy top-level location is dead, and a
        // file written there would be silently ignored by everything.
        let prof = self.active();
        let state_dir = prof.plugins_state_dir();
        let mut plugins = 0usize;
        for f in ["plugins.txt", "loadorder.txt"] {
            let src = mo2_profile_dir.join(f);
            if src.is_file() {
                // Atomic: this dir may be all that stands between the user and a
                // lost load order, and fs::copy truncates in place.
                profile::copy_atomic(&src, &eidos_plugins::canonical_path(&state_dir, f))?;
                plugins += 1;
            }
        }

        Ok(Mo2Import { matched, kept_local, missing, plugin_files: plugins })
    }
}

/// What an [`Instance::import_mo2_profile`] run took over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mo2Import {
    /// Mods MO2 listed that we have, whose order and enabled state were applied.
    pub matched: usize,
    /// Local mods MO2 never listed, kept at the bottom of the order.
    pub kept_local: usize,
    /// Mods MO2 listed that are not installed here (install them, then re-import).
    pub missing: Vec<String>,
    /// How many of `plugins.txt` / `loadorder.txt` were imported.
    pub plugin_files: usize,
}

/// A mod's `Root/` directory, matched case-insensitively (archives ship `Root`,
/// `root` and `ROOT` alike). `None` when the mod has none, which is the common
/// case.
/// A mod's `INI Tweaks/` directory, matched case-insensitively - archives ship it
/// as `INI Tweaks`, `ini tweaks` and `INI tweaks` about equally often, and the
/// name only ever has to survive a Linux filesystem, which MO2 never had to.
/// Returns the conventional casing when the mod has no such directory, so callers
/// can join a name onto it unconditionally.
pub fn ini_tweaks_dir(mod_path: &Path) -> PathBuf {
    let found = fs::read_dir(mod_path).ok().and_then(|rd| {
        rd.flatten()
            .find(|e| {
                e.file_name().to_string_lossy().eq_ignore_ascii_case("INI Tweaks")
                    && e.path().is_dir()
            })
            .map(|e| e.path())
    });
    found.unwrap_or_else(|| mod_path.join("INI Tweaks"))
}

/// The INI-tweak fragments a mod ships, sorted by name. MO2 flags a mod as having
/// tweaks exactly when this is non-empty (`hasIniTweaks`).
pub fn available_ini_tweaks(mod_path: &Path) -> Vec<String> {
    let Ok(rd) = fs::read_dir(ini_tweaks_dir(mod_path)) else { return Vec::new() };
    let mut out: Vec<String> = rd
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    out.sort_by_key(|n| n.to_lowercase());
    out
}

fn find_root_dir(mod_dir: &Path) -> Option<PathBuf> {
    fs::read_dir(mod_dir)
        .ok()?
        .flatten()
        .find(|e| {
            e.file_name().to_str().is_some_and(|n| n.eq_ignore_ascii_case("root"))
                && e.path().is_dir()
        })
        .map(|e| e.path())
}

/// Move every entry of `from` into `to`, merging into existing directories and
/// leaving `from` empty. Both sides live under the instance root (one
/// filesystem), so entries move by rename; a rename that fails because the
/// destination directory already exists recurses into it.
/// Write `bytes` to `path` through a temp file and a rename.
///
/// The temp name is UNIQUE per process and per call, and that is the whole point
/// of this function existing. Every writer here used a fixed `<name>.tmp`, which
/// is not atomic against a second WRITER: two Eidos processes - the window and an
/// `eidos` child, or two threads - both write the same temp path, their bytes
/// interleave, and whichever renames last publishes the mixture. It was
/// reproduced on the instance registry, which came out with a half-line in it.
///
/// The rename itself is what makes the result all-or-nothing for READERS; the
/// unique name is what makes it all-or-nothing between writers. Both are needed.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let tmp = path.with_extension(format!(
        "eidos-tmp.{}.{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    // The parent is NOT created here. `meta.write` on a mod with no folder used
    // to fail with a clean ENOENT that the GUI reported; creating it silently
    // planted an empty `mods/<name>/` holding only a meta.ini - which the next
    // reconcile then lists as a real mod. A writer's job is to write.
    fs::write(&tmp, bytes)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // A failed rename leaves the temp behind; it has a unique name, so
            // nothing else is going to trip over it, but it is still litter.
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Record every FILE under `dir` as `relpath -> (len, mtime)`.
///
/// The union filesystem's own bookkeeping is skipped: a whiteout
/// (`.eidoswh.<name>`) and an opaque marker are how the OVERWRITE layer records a
/// deletion, and they mean nothing in a mod - moved there they would be junk
/// files with no effect, while the deletion they encoded would be silently lost.
/// What identifies a file as "the same one, unchanged" for the output capture.
///
/// (len, mtime) alone is not enough, and the failure is silent: a tool that
/// rewrites a file to the same length inside one filesystem timestamp tick - or
/// one that restores the original mtime after writing, which xEdit and several
/// packers do - would be judged untouched and its output abandoned in the
/// Overwrite while the run reported having written nothing.
///
/// So the inode and the ctime come too. A write through a temp file and a rename
/// changes the inode; an in-place write changes the ctime, and unlike mtime the
/// ctime cannot be set back by `utimes` - setting it is itself a ctime change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStamp {
    len: u64,
    mtime: SystemTime,
    ino: u64,
    ctime: i64,
    ctime_ns: i64,
}

/// A picture of the Overwrite before a run: every file's identity, plus the
/// directories that were ALREADY empty (which the post-run sweep must not remove
/// - see [`prune_empty_dirs`]).
#[derive(Debug, Clone, Default)]
pub struct OverwriteSnapshot {
    files: HashMap<PathBuf, FileStamp>,
    empty_dirs: std::collections::HashSet<PathBuf>,
}

fn snapshot_into(dir: &Path, prefix: &Path, out: &mut OverwriteSnapshot) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(eidos_core::WHITEOUT_PREFIX) || name_str == eidos_core::OPAQUE_MARKER
        {
            continue;
        }
        let rel = prefix.join(&name);
        // `symlink_metadata`, so a symlink is recorded as itself rather than as
        // whatever it points at (which may be outside the instance entirely).
        let Ok(md) = fs::symlink_metadata(e.path()) else { continue };
        if md.is_dir() {
            let before = out.files.len();
            snapshot_into(&e.path(), &rel, out);
            // Nothing under it, at any depth: record it so the sweep leaves it.
            if out.files.len() == before && fs::read_dir(e.path()).into_iter().flatten().next().is_none()
            {
                out.empty_dirs.insert(rel);
            }
        } else {
            use std::os::unix::fs::MetadataExt;
            out.files.insert(
                rel,
                FileStamp {
                    len: md.len(),
                    mtime: md.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    ino: md.ino(),
                    ctime: md.ctime(),
                    ctime_ns: md.ctime_nsec(),
                },
            );
        }
    }
}

/// Remove directories left empty by a capture, deepest first. The Overwrite's own
/// root is never removed - the mount expects it to exist.
fn prune_empty_dirs(root: &Path, before: &OverwriteSnapshot) {
    // Everything now empty is swept up EXCEPT a directory that was already empty
    // before the run. That one is not debris this capture left: a tool that
    // creates its output folder once and expects to find it next time would be
    // told it is missing, and the whole promise of the feature is that what the
    // run did not touch stays put.
    let keep = &before.empty_dirs;

    fn walk(
        dir: &Path,
        rel: &Path,
        keep: &std::collections::HashSet<PathBuf>,
        depth: usize,
    ) -> bool {
        // A symlink loop inside the Overwrite would otherwise recurse forever.
        if depth > 64 {
            return false;
        }
        let mut empty = true;
        let Ok(rd) = fs::read_dir(dir) else { return false };
        for e in rd.flatten() {
            let p = e.path();
            let child_rel = rel.join(e.file_name());
            // `symlink_metadata`, so a symlink TO a directory is an entry, not a
            // directory to descend into and possibly unlink through.
            if fs::symlink_metadata(&p).map(|m| m.file_type().is_dir()).unwrap_or(false) {
                if walk(&p, &child_rel, keep, depth + 1) && !keep.contains(&child_rel) {
                    let _ = fs::remove_dir(&p);
                } else {
                    empty = false;
                }
            } else {
                empty = false;
            }
        }
        empty
    }
    // The Overwrite root itself is never removed - the mount expects it.
    walk(root, Path::new(""), keep, 0);
}

/// Move one produced file into a destination inside `base`, resolving a type
/// conflict the source's way exactly as [`move_tree`] does: what the tool just
/// wrote is the newer truth.
///
/// `base` is a directory that ALREADY EXISTS and is never touched - the mods
/// directory. Only components below it are created. That bound is the whole
/// safety property: an earlier version walked the destination from `/` and
/// replaced any symlinked component it met, which on the layout this crate
/// explicitly supports - a mod pool on another drive reached by a symlinked
/// `mods/` - deleted the link to the pool and put an empty directory in its
/// place on the first file synced.
fn move_one_into(base: &Path, from: &Path, to: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let rel = to
        .strip_prefix(base)
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "destination is outside the mods folder"))?;
    if let Some(parent) = rel.parent() {
        create_dirs_under(base, parent)?;
    }

    // A destination that is in the way is moved ASIDE, not deleted, until the
    // rename has actually succeeded. Removing it first meant a failed rename -
    // EXDEV, when mods/ is a mount from another drive - destroyed the mod's own
    // copy and moved nothing, while reporting only "could not be moved".
    let existing = fs::symlink_metadata(to).ok();
    let stash = existing.as_ref().map(|_| {
        to.with_extension(format!("eidos-replaced.{}", std::process::id()))
    });
    if let Some(stash) = &stash {
        fs::rename(to, stash)?;
    }

    let outcome = fs::rename(from, to).or_else(|e| {
        // Different filesystems: rename cannot cross them, so fall back to a
        // copy and only unlink the source once the copy is safely there.
        if e.raw_os_error() == Some(libc::EXDEV) {
            fs::copy(from, to)?;
            fs::remove_file(from)?;
            Ok(())
        } else {
            Err(e)
        }
    });

    match (outcome, stash) {
        (Ok(()), Some(stash)) => {
            // The move worked: the displaced original is genuinely superseded.
            let _ = if stash.is_dir() {
                fs::remove_dir_all(&stash)
            } else {
                fs::remove_file(&stash)
            };
            Ok(())
        }
        (Ok(()), None) => Ok(()),
        (Err(e), Some(stash)) => {
            // Put it back. The caller records a failure and the user's file is
            // exactly where it was.
            let _ = fs::rename(&stash, to);
            Err(e)
        }
        (Err(e), None) => Err(e),
    }
}

/// Create every component of `rel` under `base`, which must already exist.
///
/// Never leaves `base`, and never follows or removes a symlink. A component that
/// is a symlink is REFUSED rather than replaced: it could point anywhere, the
/// user put it there deliberately, and writing through it would put files
/// outside the instance. A component that is a plain FILE is replaced, because
/// that is the same type conflict `move_tree` resolves the source's way (a mod
/// shipping `SKSE` as a file while a tool wrote `SKSE/Plugins/x.dll`).
fn create_dirs_under(base: &Path, rel: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let mut cur = base.to_path_buf();
    for comp in rel.components() {
        // Only plain names. `..` would climb out of `base`, which is the one
        // thing this function exists to prevent.
        let std::path::Component::Normal(name) = comp else {
            return Err(Error::new(ErrorKind::InvalidInput, "path escapes the mods folder"));
        };
        cur.push(name);
        match fs::symlink_metadata(&cur) {
            Ok(md) if md.file_type().is_dir() => {}
            Ok(md) if md.file_type().is_symlink() => {
                return Err(Error::other(format!(
                    "{} is a symlink; Eidos will not write through it",
                    cur.display()
                )));
            }
            Ok(_) => {
                fs::remove_file(&cur)?;
                fs::create_dir(&cur)?;
            }
            Err(_) => fs::create_dir(&cur)?,
        }
    }
    Ok(())
}

fn move_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    for e in fs::read_dir(from)?.flatten() {
        let src = e.path();
        let dst = to.join(e.file_name());
        // Merging only makes sense dir-into-dir. On a TYPE conflict the source -
        // what the game just wrote into the Overwrite, the top layer of the
        // union - wins the name: recursing a file-occupied `dst` hit ENOTDIR
        // and `remove_file` on a dir-occupied one hit EISDIR, either way
        // aborting "move to mod" half-done with the Overwrite part-emptied.
        // `symlink_metadata` so a dangling link still counts as an occupant.
        let dst_is_dir =
            fs::symlink_metadata(&dst).map(|m| m.file_type().is_dir()).unwrap_or(false);
        if src.is_dir() && dst_is_dir {
            // Merge rather than clobber, then drop the now-empty source dir.
            move_tree(&src, &dst)?;
            let _ = fs::remove_dir(&src);
        } else {
            match fs::symlink_metadata(&dst).map(|m| m.file_type()) {
                Ok(t) if t.is_dir() => fs::remove_dir_all(&dst)?,
                Ok(_) => fs::remove_file(&dst)?,
                Err(_) => {}
            }
            fs::rename(&src, &dst)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn an_instance_root_inside_or_at_the_game_install_is_refused() {
        let game = Path::new("/games/Skyrim Special Edition");
        assert!(Instance::root_inside_game(Path::new("/games/Skyrim Special Edition/Eidos"), game));
        assert!(Instance::root_inside_game(game, game), "the install itself counts");
        assert!(!Instance::root_inside_game(Path::new("/games/Eidos-Skyrim"), game), "a sibling is the recommended layout");
        assert!(
            !Instance::root_inside_game(Path::new("/games/Skyrim Special Edition2"), game),
            "a shared name PREFIX is not containment - starts_with is per component"
        );
    }

    #[test]
    fn a_symlinked_parent_cannot_sneak_an_instance_into_the_install() {
        let base = std::env::temp_dir().join(format!("eidos-guard-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let game = base.join("game");
        fs::create_dir_all(&game).unwrap();
        let link = base.join("link");
        std::os::unix::fs::symlink(&game, &link).unwrap();
        // The typed root does not exist yet (creation-time check) and its
        // lexical path shares nothing with the install - only resolving the
        // symlinked parent reveals the containment.
        assert!(Instance::root_inside_game(&link.join("Eidos"), &game));
        let _ = fs::remove_dir_all(&base);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp_instance() -> Instance {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("eidos-inst-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Instance::portable(root)
    }

    #[test]
    fn active_profile_falls_back_when_selected_dir_is_gone() {
        let inst = tmp_instance();
        inst.ensure_manifest("skyrimse", InstanceKind::Portable).unwrap();
        // Two real profiles on disk...
        fs::create_dir_all(inst.profiles_dir().join("Default")).unwrap();
        fs::create_dir_all(inst.profiles_dir().join("Modded")).unwrap();
        // ...but the manifest still points at a profile deleted/renamed away.
        inst.set_active_profile("Ghost").unwrap();
        // active_profile must NOT return the ghost (which, lacking a modlist,
        // would launch with every mod on); it falls back to an existing profile.
        let active = inst.active_profile();
        assert_ne!(active, "Ghost");
        assert!(inst.profiles_dir().join(&active).is_dir());
        // With the selected profile present, it is honoured verbatim.
        inst.set_active_profile("Modded").unwrap();
        assert_eq!(inst.active_profile(), "Modded");
        let _ = fs::remove_dir_all(&inst.root);
    }

    #[test]
    fn rename_profile_follows_the_active_pointer() {
        let inst = tmp_instance();
        inst.ensure_manifest("skyrimse", InstanceKind::Portable).unwrap();
        fs::create_dir_all(inst.profiles_dir().join("Default")).unwrap();
        fs::create_dir_all(inst.profiles_dir().join("Modded")).unwrap();
        inst.set_active_profile("Modded").unwrap();
        // Renaming the ACTIVE profile updates the manifest pointer (no dangling).
        inst.rename_profile("Modded", "Heavy").unwrap();
        assert_eq!(inst.active_profile(), "Heavy");
        assert!(inst.profiles_dir().join("Heavy").is_dir());
        assert!(!inst.profiles_dir().join("Modded").exists());
        // Renaming onto an existing name is refused.
        assert!(inst.rename_profile("Default", "Heavy").is_err());
        let _ = fs::remove_dir_all(&inst.root);
    }

    #[test]
    fn delete_profile_guards_active_and_last() {
        let inst = tmp_instance();
        inst.ensure_manifest("skyrimse", InstanceKind::Portable).unwrap();
        fs::create_dir_all(inst.profiles_dir().join("Default")).unwrap();
        fs::create_dir_all(inst.profiles_dir().join("Modded")).unwrap();
        inst.set_active_profile("Modded").unwrap();
        // Cannot delete the active profile.
        assert!(inst.delete_profile("Modded").is_err());
        // A non-active one deletes fine.
        inst.delete_profile("Default").unwrap();
        assert!(!inst.profiles_dir().join("Default").exists());
        // Cannot delete the last remaining profile.
        assert!(inst.delete_profile("Modded").is_err());
        let _ = fs::remove_dir_all(&inst.root);
    }

    #[test]
    fn create_empty_mod_writes_minimal_meta() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let entry = inst.create_empty_mod("My New Mod").unwrap();
        assert_eq!(entry.name, "My New Mod");
        assert!(entry.enabled);
        assert!(entry.path.is_dir());
        assert!(entry.path.join("meta.ini").is_file());
        // A second create of the same name collides.
        assert!(inst.create_empty_mod("My New Mod").is_err());
        // Illegal names are refused (no folder is created).
        assert!(inst.create_empty_mod("").is_err());
        assert!(inst.create_empty_mod("a/b").is_err());
        let _ = fs::remove_dir_all(&inst.root);
    }

    #[test]
    fn separator_name_and_display_name() {
        let sep = ModEntry {
            name: "Gameplay_separator".into(),
            enabled: true,
            path: PathBuf::new(), unmanaged: false };
        assert!(sep.is_separator());
        assert_eq!(sep.display_name(), "Gameplay");

        let modd = ModEntry { name: "SkyUI".into(), enabled: true, path: PathBuf::new(), unmanaged: false };
        assert!(!modd.is_separator());
        assert_eq!(modd.display_name(), "SkyUI");

        // A backup is inert for the same reason and by the same kind of rule -
        // including every numbered one, which is where this first went wrong.
        assert!(is_backup_name("X_backup"));
        assert!(is_backup_name("X_backup2"));
        assert!(is_backup_name("X_backup17"));
        assert!(!is_backup_name("X_backups"));
        assert!(!is_backup_name("backup_X"));
        assert!(!is_backup_name("X"));
        assert!(is_separator_name("X_separator"));
        assert!(!is_separator_name("Xseparator"));
        assert!(!is_separator_name("separator_X"));
    }

    #[test]
    fn mo2_import_applies_order_and_states_keeping_local_mods() {
        let inst = tmp_instance();
        inst.create().unwrap();
        for m in ["SkyUI", "USSEP", "LocalOnly"] {
            fs::create_dir_all(inst.mods_dir().join(m)).unwrap();
        }
        // An MO2 profile: highest priority first, USSEP disabled, one mod we lack.
        let mo2 = inst.root.join("mo2profile");
        fs::create_dir_all(&mo2).unwrap();
        fs::write(mo2.join("modlist.txt"), "+SkyUI\n-ussep\n+NotInstalled\n*Foreign\n").unwrap();
        fs::write(mo2.join("plugins.txt"), b"*Skyrim.esm\n*SkyUI.esp\n").unwrap();

        let r = inst.import_mo2_profile(&mo2).unwrap();
        assert_eq!(r.matched, 2, "SkyUI + USSEP matched (case-insensitively)");
        assert_eq!(r.kept_local, 1, "LocalOnly is kept");
        assert_eq!(r.missing, vec!["NotInstalled".to_string()]);
        assert_eq!(r.plugin_files, 1);

        // Display order is lowest-priority-first: the untouched local mod sits at
        // the bottom, then MO2's order with SkyUI highest (last).
        let names: Vec<String> = inst.modlist().into_iter().map(|m| m.name).collect();
        assert_eq!(names, vec!["LocalOnly", "USSEP", "SkyUI"]);
        let ussep = inst.modlist().into_iter().find(|m| m.name == "USSEP").unwrap();
        assert!(!ussep.enabled, "MO2 had it disabled");
        assert!(inst.modlist().into_iter().find(|m| m.name == "SkyUI").unwrap().enabled);
        // The plugin state came across into the active profile.
        assert_eq!(fs::read(inst.active().plugins_txt_path()).unwrap(), b"*Skyrim.esm\n*SkyUI.esp\n");
    }

    #[test]
    fn mo2_import_rejects_a_directory_without_a_modlist() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let empty = inst.root.join("not-a-profile");
        fs::create_dir_all(&empty).unwrap();
        assert!(inst.import_mo2_profile(&empty).is_err());
    }

    #[test]
    fn root_layers_finds_root_dirs_highest_priority_first() {
        let inst = tmp_instance();
        inst.create().unwrap();
        // Archives spell it every way, so the match is case-insensitive.
        fs::create_dir_all(inst.mods_dir().join("SKSE/Root")).unwrap();
        fs::create_dir_all(inst.mods_dir().join("ENB/root")).unwrap();
        fs::create_dir_all(inst.mods_dir().join("PlainMod/textures")).unwrap();
        fs::create_dir_all(inst.mods_dir().join("Disabled/Root")).unwrap();
        // Display order is lowest priority first.
        inst.save_modlist(&[
            ModEntry { name: "SKSE".into(), enabled: true, path: inst.mods_dir().join("SKSE"), unmanaged: false },
            ModEntry { name: "PlainMod".into(), enabled: true, path: inst.mods_dir().join("PlainMod"), unmanaged: false },
            ModEntry { name: "Disabled".into(), enabled: false, path: inst.mods_dir().join("Disabled"), unmanaged: false },
            ModEntry { name: "ENB".into(), enabled: true, path: inst.mods_dir().join("ENB"), unmanaged: false },
        ])
        .unwrap();

        let layers = inst.root_layers();
        // Highest priority first (ENB is last in display order), disabled skipped,
        // and a mod without a Root/ contributes nothing.
        assert_eq!(layers.len(), 2, "got {layers:?}");
        assert!(layers[0].ends_with("ENB/root"));
        assert!(layers[1].ends_with("SKSE/Root"));
    }

    #[test]
    fn an_ordinary_load_order_asks_for_no_root_mount() {
        let inst = tmp_instance();
        inst.create().unwrap();
        fs::create_dir_all(inst.mods_dir().join("JustTextures/textures")).unwrap();
        inst.save_modlist(&[ModEntry {
            name: "JustTextures".into(),
            enabled: true,
            path: inst.mods_dir().join("JustTextures"), unmanaged: false }])
        .unwrap();
        // Empty means the launcher skips the second mount entirely, so existing
        // setups behave exactly as before.
        assert!(inst.root_layers().is_empty());
    }

    #[test]
    fn overwrite_into_new_mod_moves_everything_and_empties_it() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        fs::create_dir_all(ow.join("SKSE/Plugins")).unwrap();
        fs::write(ow.join("SKSE/Plugins/gen.json"), b"generated").unwrap();
        fs::write(ow.join("loose.txt"), b"x").unwrap();
        assert!(!inst.overwrite_is_empty());

        let dest = inst.overwrite_into_mod("Generated Output").unwrap();
        assert_eq!(fs::read(dest.join("SKSE/Plugins/gen.json")).unwrap(), b"generated");
        assert_eq!(fs::read(dest.join("loose.txt")).unwrap(), b"x");
        assert!(dest.join("meta.ini").is_file(), "a fresh mod gets a meta.ini");
        assert!(inst.overwrite_is_empty(), "the Overwrite must be left empty");
    }

    #[test]
    fn sync_sends_a_file_back_only_to_a_mod_that_already_provides_it() {
        let inst = tmp_instance();
        inst.create().unwrap();
        // Two mods, one of which owns meshes/a.nif.
        let owner = inst.mods_dir().join("Meshes Mod");
        fs::create_dir_all(owner.join("meshes")).unwrap();
        fs::write(owner.join("meshes/a.nif"), b"original").unwrap();

        let ow = inst.overwrite_dir();
        fs::create_dir_all(ow.join("meshes")).unwrap();
        fs::write(ow.join("meshes/a.nif"), b"regenerated").unwrap();
        // Genuinely new output: nothing else provides it, so it must STAY.
        fs::write(ow.join("brand-new.json"), b"{}").unwrap();

        let mut owners = HashMap::new();
        owners.insert("meshes/a.nif".to_string(), "Meshes Mod".to_string());

        let (moved, failures) = inst.sync_overwrite_to_mods(&owners).unwrap();
        assert_eq!((moved, failures.len()), (1, 0));
        assert_eq!(fs::read(owner.join("meshes/a.nif")).unwrap(), b"regenerated");
        assert!(!ow.join("meshes").exists(), "the emptied directory is swept up");
        // The whole point of sending files BACK rather than bundling them: what
        // no mod claims is left for the user to decide about.
        assert_eq!(fs::read(ow.join("brand-new.json")).unwrap(), b"{}");
    }

    #[test]
    fn sync_refuses_an_owner_that_is_not_a_mod_folder_name() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        fs::write(ow.join("x.txt"), b"x").unwrap();
        // The map comes from the window, but a bad name must never become a
        // path join that escapes mods/.
        let mut owners = HashMap::new();
        owners.insert("x.txt".to_string(), "../escape".to_string());
        let (moved, failures) = inst.sync_overwrite_to_mods(&owners).unwrap();
        assert_eq!(moved, 0);
        assert_eq!(failures.len(), 1);
        assert!(ow.join("x.txt").is_file(), "nothing moved");
    }

    #[test]
    fn sync_keeps_a_directory_that_was_already_empty() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        // A tool's own scratch folder, empty before the sync and expected to
        // still be there after it.
        fs::create_dir_all(ow.join("Nemesis_Engine/temp")).unwrap();
        fs::write(ow.join("moved.txt"), b"x").unwrap();
        let target = inst.mods_dir().join("Target");
        fs::create_dir_all(&target).unwrap();

        let mut owners = HashMap::new();
        owners.insert("moved.txt".to_string(), "Target".to_string());
        let (moved, _) = inst.sync_overwrite_to_mods(&owners).unwrap();
        assert_eq!(moved, 1);
        assert!(ow.join("Nemesis_Engine/temp").is_dir());
    }

    #[test]
    fn concurrent_atomic_writes_never_publish_a_mixture() {
        // The failure this exists for, reproduced: every writer used a fixed
        // `<name>.tmp`, so two writers of the same file wrote the SAME temp,
        // their bytes interleaved, and whichever renamed last published the
        // mixture. It was found on the instance registry, which came out with a
        // half-written `able=` line in it.
        let dir = std::env::temp_dir().join(format!("eidos-atomic-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let target = dir.join("contended.ini");

        // Two distinct bodies, different lengths, no common prefix - so any
        // interleaving is unmistakable in the result.
        let a = "a".repeat(64 * 1024);
        let b = "b".repeat(48 * 1024 + 7);
        let (ta, tb) = (target.clone(), target.clone());
        let (ba, bb) = (a.clone(), b.clone());
        let h1 = std::thread::spawn(move || {
            for _ in 0..40 {
                write_atomic(&ta, ba.as_bytes()).unwrap();
            }
        });
        let h2 = std::thread::spawn(move || {
            for _ in 0..40 {
                write_atomic(&tb, bb.as_bytes()).unwrap();
            }
        });
        h1.join().unwrap();
        h2.join().unwrap();

        // Whoever won, the file is one of the two whole bodies - never a splice.
        let got = fs::read_to_string(&target).unwrap();
        assert!(got == a || got == b, "torn: {} bytes, starts {:?}", got.len(), &got[..8.min(got.len())]);

        // And no litter: a unique temp is still cleaned up on the way through.
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("eidos-tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_does_not_invent_the_directory_it_is_pointed_at() {
        let dir = std::env::temp_dir().join(format!("eidos-atomic-mk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        // A writer's job is to write. Creating the parent turned "write meta.ini
        // for a mod that has no folder" from a clean ENOENT the GUI reported
        // into silently planting an empty `mods/<name>/` that the next reconcile
        // then lists as a real mod.
        let target = dir.join("deep/inside/file.ini");
        assert_eq!(write_atomic(&target, b"x").unwrap_err().kind(), std::io::ErrorKind::NotFound);
        assert!(!dir.exists());

        fs::create_dir_all(dir.join("deep/inside")).unwrap();
        write_atomic(&target, b"x").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"x");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_move_that_cannot_land_leaves_the_destination_exactly_as_it_was() {
        // The failure this ordering exists for: removing the destination BEFORE
        // the rename destroyed the mod's own copy whenever the rename then
        // failed - EXDEV, when mods/ is a mount from another drive - while
        // reporting only "could not be moved".
        let inst = tmp_instance();
        inst.create().unwrap();
        let dest_mod = inst.mods_dir().join("Target");
        fs::create_dir_all(dest_mod.join("sub")).unwrap();
        fs::write(dest_mod.join("sub/x.nif"), b"the mod's own copy").unwrap();

        // A source that does not exist makes the rename fail after the
        // destination has been stashed aside.
        let missing = inst.overwrite_dir().join("nope.nif");
        let err = move_one_into(&inst.mods_dir(), &missing, &dest_mod.join("sub/x.nif"));
        assert!(err.is_err());
        assert_eq!(
            fs::read(dest_mod.join("sub/x.nif")).unwrap(),
            b"the mod's own copy",
            "a failed move must not have consumed the original"
        );
    }

    #[test]
    fn a_move_never_climbs_out_of_the_mods_folder() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let src = inst.overwrite_dir().join("x.txt");
        fs::write(&src, b"x").unwrap();
        // A destination outside the base is refused outright rather than
        // resolved - the base is what makes every other guard meaningful.
        let outside = inst.root.join("elsewhere/x.txt");
        assert!(move_one_into(&inst.mods_dir(), &src, &outside).is_err());
        assert!(src.is_file(), "and the source is untouched");
    }

    #[test]
    fn capture_takes_only_what_the_run_produced() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        // Already in the Overwrite before the tool ran: someone else's output,
        // and it must stay exactly where it is.
        fs::create_dir_all(ow.join("SKSE/Plugins")).unwrap();
        fs::write(ow.join("SKSE/Plugins/old.json"), b"pre-existing").unwrap();
        let before = inst.overwrite_snapshot();
        assert_eq!(before.files.len(), 1);

        // What the run writes.
        fs::create_dir_all(ow.join("meshes/actors")).unwrap();
        fs::write(ow.join("meshes/actors/fnis.hkx"), b"generated").unwrap();
        fs::write(ow.join("SKSE/Plugins/new.json"), b"also generated").unwrap();

        let moved = inst.capture_overwrite_into_mod("FNIS Output", &before).unwrap();
        assert_eq!(moved, 2);
        let dest = inst.mods_dir().join("FNIS Output");
        assert_eq!(fs::read(dest.join("meshes/actors/fnis.hkx")).unwrap(), b"generated");
        assert_eq!(fs::read(dest.join("SKSE/Plugins/new.json")).unwrap(), b"also generated");
        assert!(dest.join("meta.ini").is_file(), "a fresh output mod gets a meta.ini");

        // The pre-existing file is untouched, and its now-childless parent
        // survives because it still holds it.
        assert_eq!(fs::read(ow.join("SKSE/Plugins/old.json")).unwrap(), b"pre-existing");
        assert!(!ow.join("meshes").exists(), "emptied directories are swept up");
    }

    #[test]
    fn capture_leaves_a_directory_that_was_already_empty_and_sweeps_the_ones_it_emptied() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        // A tool that creates its output folder once and expects to find it on
        // the next run. Removing it would tell that tool its setup is gone.
        fs::create_dir_all(ow.join("Nemesis_Engine/temp")).unwrap();
        let before = inst.overwrite_snapshot();

        fs::create_dir_all(ow.join("meshes/actors")).unwrap();
        fs::write(ow.join("meshes/actors/gen.hkx"), b"x").unwrap();

        assert_eq!(inst.capture_overwrite_into_mod("Out", &before).unwrap(), 1);
        assert!(ow.join("Nemesis_Engine/temp").is_dir(), "it was empty BEFORE the run");
        assert!(!ow.join("meshes").exists(), "this one the run created and emptied");
    }

    #[test]
    fn capture_survives_a_type_conflict_instead_of_abandoning_half_its_output() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        let before = inst.overwrite_snapshot();
        // Three files; the middle one's parent is occupied by a FILE in the
        // target mod, which `create_dir_all` refuses outright.
        for rel in ["a.txt", "SKSE/Plugins/gen.json", "z.txt"] {
            let p = ow.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"x").unwrap();
        }
        let dest = inst.mods_dir().join("Out");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("SKSE"), b"a file where a directory has to go").unwrap();

        // It does not abort: aborting mid-loop would split the run's output
        // between the Overwrite and the mod, which is worse than either.
        let n = inst.capture_overwrite_into_mod("Out", &before).unwrap();
        assert_eq!(n, 3, "the occupying file is replaced, source wins the name");
        assert!(dest.join("SKSE/Plugins/gen.json").is_file());
        assert!(dest.join("a.txt").is_file() && dest.join("z.txt").is_file());
    }

    #[test]
    fn capture_never_writes_through_a_symlink_out_of_the_instance() {
        let inst = tmp_instance();
        inst.create().unwrap();
        // Somewhere the capture must never reach.
        let outside = inst.root.join("..").join(format!("eidos-outside-{}", std::process::id()));
        let _ = fs::create_dir_all(&outside);
        fs::write(outside.join("precious.txt"), b"do not touch").unwrap();

        let dest = inst.mods_dir().join("Out");
        fs::create_dir_all(&dest).unwrap();
        std::os::unix::fs::symlink(&outside, dest.join("SKSE")).unwrap();

        let before = inst.overwrite_snapshot();
        let ow = inst.overwrite_dir();
        fs::create_dir_all(ow.join("SKSE")).unwrap();
        fs::write(ow.join("SKSE/precious.txt"), b"captured").unwrap();

        // REFUSED, not replaced. A symlink could point anywhere and the user put
        // it there; unlinking it to make room would be Eidos deciding that for
        // them, and writing THROUGH it would put files outside the instance.
        let err = inst.capture_overwrite_into_mod("Out", &before).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
        assert_eq!(fs::read(outside.join("precious.txt")).unwrap(), b"do not touch");
        assert!(fs::symlink_metadata(dest.join("SKSE")).unwrap().file_type().is_symlink());
        // And the file it could not place is still in the Overwrite, not lost.
        assert!(inst.overwrite_dir().join("SKSE/precious.txt").is_file());
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn a_created_output_mod_is_registered_and_enabled() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let before = inst.overwrite_snapshot();
        fs::write(inst.overwrite_dir().join("gen.esp"), b"x").unwrap();

        assert_eq!(inst.capture_overwrite_into_mod("FNIS Output", &before).unwrap(), 1);
        // Otherwise the output is in the instance and invisible: an unregistered
        // folder reconciles as DISABLED, `load_order` drops it, and the tool
        // regenerates the same files every run having achieved nothing.
        let entry = inst
            .modlist()
            .into_iter()
            .find(|m| m.name == "FNIS Output")
            .expect("registered in modlist.txt");
        assert!(entry.enabled);
        assert!(inst.load_order().iter().any(|p| p.ends_with("FNIS Output")));
    }

    #[test]
    fn capture_notices_a_file_that_was_rewritten_in_place() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        fs::write(ow.join("out.txt"), b"first").unwrap();
        let before = inst.overwrite_snapshot();
        // A rewrite that keeps the same LENGTH: only the mtime moves, which is
        // why the snapshot records both.
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(ow.join("out.txt"), b"AGAIN").unwrap();

        let moved = inst.capture_overwrite_into_mod("Out", &before).unwrap();
        assert_eq!(moved, 1, "a same-size rewrite is still output");
        assert_eq!(fs::read(inst.mods_dir().join("Out/out.txt")).unwrap(), b"AGAIN");
    }

    #[test]
    fn a_run_that_produced_nothing_leaves_no_empty_mod_behind() {
        let inst = tmp_instance();
        inst.create().unwrap();
        fs::write(inst.overwrite_dir().join("old.txt"), b"x").unwrap();
        let before = inst.overwrite_snapshot();

        assert_eq!(inst.capture_overwrite_into_mod("Nothing", &before).unwrap(), 0);
        assert!(
            !inst.mods_dir().join("Nothing").exists(),
            "an empty mod in the list would be noise the user has to clean up"
        );
        assert!(inst.overwrite_dir().join("old.txt").is_file(), "and nothing moved");
    }

    #[test]
    fn capture_merges_into_an_existing_output_mod_without_wiping_it() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let target = inst.mods_dir().join("Output");
        fs::create_dir_all(target.join("meshes")).unwrap();
        fs::write(target.join("meshes/keep.nif"), b"keep").unwrap();
        fs::write(target.join("meta.ini"), b"[General]\nendorsed=1\n").unwrap();

        let before = inst.overwrite_snapshot();
        fs::create_dir_all(inst.overwrite_dir().join("meshes")).unwrap();
        fs::write(inst.overwrite_dir().join("meshes/new.nif"), b"new").unwrap();

        assert_eq!(inst.capture_overwrite_into_mod("Output", &before).unwrap(), 1);
        assert_eq!(fs::read(target.join("meshes/keep.nif")).unwrap(), b"keep");
        assert_eq!(fs::read(target.join("meshes/new.nif")).unwrap(), b"new");
        // The existing meta.ini is NOT overwritten - the endorsement survives.
        assert!(fs::read_to_string(target.join("meta.ini")).unwrap().contains("endorsed=1"));
    }

    #[test]
    fn capture_leaves_the_union_filesystems_own_bookkeeping_alone() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let ow = inst.overwrite_dir();
        let before = inst.overwrite_snapshot();
        // A whiteout records "this lower-layer file is deleted". It only means
        // anything in the OVERWRITE; moved into a mod it would be a junk file,
        // and the deletion it encoded would be silently lost.
        fs::write(ow.join(format!("{}dead.esp", eidos_core::WHITEOUT_PREFIX)), b"").unwrap();
        fs::write(ow.join(eidos_core::OPAQUE_MARKER), b"").unwrap();
        fs::write(ow.join("real.txt"), b"x").unwrap();

        assert_eq!(inst.capture_overwrite_into_mod("Out", &before).unwrap(), 1);
        assert!(ow.join(format!("{}dead.esp", eidos_core::WHITEOUT_PREFIX)).is_file());
        assert!(ow.join(eidos_core::OPAQUE_MARKER).is_file());
        assert!(!inst.mods_dir().join("Out").join(eidos_core::OPAQUE_MARKER).exists());
    }

    #[test]
    fn capture_refuses_a_target_that_is_not_a_mod_folder_name() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let before = inst.overwrite_snapshot();
        for bad in ["", "  ", "..", ".", "a/b", "a\\b", "x\ny"] {
            assert!(
                inst.capture_overwrite_into_mod(bad, &before).is_err(),
                "must refuse {bad:?} - a tools.ini is hand-editable"
            );
        }
    }

    #[test]
    fn overwrite_into_existing_mod_merges_without_clobbering() {
        let inst = tmp_instance();
        inst.create().unwrap();
        let target = inst.mods_dir().join("MyMod");
        fs::create_dir_all(target.join("meshes")).unwrap();
        fs::write(target.join("meshes/keep.nif"), b"keep").unwrap();
        fs::write(target.join("meta.ini"), b"[General]\nendorsed=1\n").unwrap();

        let ow = inst.overwrite_dir();
        fs::create_dir_all(ow.join("meshes")).unwrap();
        fs::write(ow.join("meshes/new.nif"), b"new").unwrap();

        inst.overwrite_into_mod("MyMod").unwrap();
        assert_eq!(fs::read(target.join("meshes/keep.nif")).unwrap(), b"keep");
        assert_eq!(fs::read(target.join("meshes/new.nif")).unwrap(), b"new");
        // An existing mod keeps its own metadata.
        assert_eq!(fs::read(target.join("meta.ini")).unwrap(), b"[General]\nendorsed=1\n");
        assert!(inst.overwrite_is_empty());
    }

    #[test]
    fn overwrite_into_mod_survives_type_conflicts_in_both_directions() {
        // The game regenerated as a FILE what the mod ships as a DIRECTORY, and
        // vice versa. Both used to abort the move half-done (EISDIR / ENOTDIR)
        // with the Overwrite part-emptied; the Overwrite is the union's top
        // layer, so its shape wins the name.
        let inst = tmp_instance();
        inst.create().unwrap();
        let target = inst.mods_dir().join("MyMod");
        fs::create_dir_all(target.join("docs")).unwrap();
        fs::write(target.join("docs/readme.txt"), b"old").unwrap();
        fs::write(target.join("SKSE"), b"was a file").unwrap();

        let ow = inst.overwrite_dir();
        fs::write(ow.join("docs"), b"now a file").unwrap();
        fs::create_dir_all(ow.join("SKSE/Plugins")).unwrap();
        fs::write(ow.join("SKSE/Plugins/gen.json"), b"gen").unwrap();

        inst.overwrite_into_mod("MyMod").unwrap();
        assert_eq!(fs::read(target.join("docs")).unwrap(), b"now a file");
        assert_eq!(fs::read(target.join("SKSE/Plugins/gen.json")).unwrap(), b"gen");
        assert!(inst.overwrite_is_empty(), "nothing may be left behind");
    }

    #[test]
    fn overwrite_into_mod_rejects_bad_names_and_an_empty_overwrite() {
        let inst = tmp_instance();
        inst.create().unwrap();
        // Empty Overwrite.
        assert!(inst.overwrite_into_mod("Whatever").is_err());
        fs::write(inst.overwrite_dir().join("f.txt"), b"x").unwrap();
        for bad in ["", "  ", "a/b", "a\\b", "..", "."] {
            assert!(inst.overwrite_into_mod(bad).is_err(), "{bad:?} must be rejected");
        }
        assert!(!inst.overwrite_is_empty(), "a rejected move leaves the Overwrite alone");
    }

    #[test]
    fn root_overwrite_moves_into_the_one_overwrite() {
        let inst = tmp_instance();
        let legacy = inst.root.join(".base-root.root-overwrite");
        fs::create_dir_all(legacy.join("meshes/actors")).unwrap();
        fs::write(legacy.join("meshes/actors/body.nif"), b"nif").unwrap();
        fs::write(legacy.join("d3dx9_42.log"), b"log").unwrap();

        assert_eq!(inst.migrate_root_overwrite().unwrap(), 2, "two top-level entries");
        let root = inst.root_overwrite_dir();
        assert_eq!(fs::read(root.join("meshes/actors/body.nif")).unwrap(), b"nif");
        assert_eq!(fs::read(root.join("d3dx9_42.log")).unwrap(), b"log");
        assert!(!legacy.exists(), "the hidden directory is gone, not left half-empty");
        // The whole point: it is now under the Overwrite the front end lists.
        assert!(root.starts_with(inst.overwrite_dir()));
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        let inst = tmp_instance();
        let legacy = inst.root.join(".base-root.root-overwrite");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("a.txt"), b"1").unwrap();
        assert_eq!(inst.migrate_root_overwrite().unwrap(), 1);
        assert_eq!(inst.migrate_root_overwrite().unwrap(), 0);
        assert_eq!(fs::read(inst.root_overwrite_dir().join("a.txt")).unwrap(), b"1");
    }

    #[test]
    fn migration_merges_instead_of_clobbering() {
        // A user who already has Root/ content in the Overwrite must not lose it,
        // and must not lose the legacy file either.
        let inst = tmp_instance();
        let dest = inst.root_overwrite_dir();
        fs::create_dir_all(dest.join("meshes")).unwrap();
        fs::write(dest.join("meshes/kept.nif"), b"kept").unwrap();
        let legacy = inst.root.join(".base-root.root-overwrite");
        fs::create_dir_all(legacy.join("meshes")).unwrap();
        fs::write(legacy.join("meshes/moved.nif"), b"moved").unwrap();

        inst.migrate_root_overwrite().unwrap();
        assert_eq!(fs::read(dest.join("meshes/kept.nif")).unwrap(), b"kept");
        assert_eq!(fs::read(dest.join("meshes/moved.nif")).unwrap(), b"moved");
    }

    #[test]
    fn creating_an_instance_migrates() {
        // `create()` runs on every command, so the move happens without the user
        // ever being told to do anything.
        let inst = tmp_instance();
        let legacy = inst.root.join(".base-root.root-overwrite");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("stray.nif"), b"x").unwrap();
        inst.create().unwrap();
        assert!(inst.root_overwrite_dir().join("stray.nif").is_file());
    }

    #[test]
    fn the_instance_lock_is_re_entrant_on_one_thread_and_not_across_two() {
        let root = std::env::temp_dir().join(format!(
            "eidos-lock-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        let inst = Instance::portable(root.clone());

        // The defect this exists for: every handler takes the lock around its
        // write and then calls something that saves - and saving takes the lock.
        // `flock` refuses a second descriptor even to the holder, so the handler
        // deadlocked against itself. Renaming a mod then failed to write
        // modlist.txt, the list was reloaded from a file naming the old folder,
        // and the renamed mod arrived at the top of the list, disabled.
        let outer = inst.try_lock("the Eidos window").expect("first hold");
        let inner = inst.try_lock("the Eidos window").expect("nested hold on the same thread");
        drop(inner);
        // Still held after the inner one goes: a depth, not a second lock.
        assert!(inst.try_lock("again").is_ok());

        // Another THREAD is a real conflict and stays refused - which is what
        // the background tasks need.
        let r2 = root.clone();
        let across = std::thread::spawn(move || Instance::portable(r2).try_lock("other").is_err())
            .join()
            .unwrap();
        assert!(across, "a second thread must still be refused");

        drop(outer);
        // And once every hold is gone, another thread can take it.
        let r3 = root.clone();
        let after = std::thread::spawn(move || Instance::portable(r3).try_lock("other").is_ok())
            .join()
            .unwrap();
        assert!(after, "released for good once the last hold drops");
        let _ = fs::remove_dir_all(&root);
    }

}
