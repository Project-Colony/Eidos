//! The game files an instance runs: the store install, or a copy of the game
//! kept in the instance folder.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

use eidos_games::DetectedGame;
use eidos_instance::{is_copy_folder_name, GameFiles, Instance, InstanceKind};

/// The folder a freeze copies the store install into.
pub const FROZEN_COPY: &str = "Stock Game";

/// The game files `inst` runs `game` from.
pub fn game_files(game: &DetectedGame, inst: &Instance) -> GameFiles {
    inst.game_files(&game.install_path, &game.data_path)
}

/// `game` with its install and Data paths moved to `files`, for library code
/// that reads the game's files and never mounts or launches it (the collection
/// driver, `recipe::runtime_check`). Its `plugin_state_dir()` stays right
/// because a root-mode game never runs from a copy.
pub fn files_view(game: &DetectedGame, files: &GameFiles) -> DetectedGame {
    DetectedGame {
        install_path: files.root.clone(),
        data_path: files.data.clone(),
        ..game.clone()
    }
}

/// `value` moved from the game copy at `copy` onto the store `install`, where
/// the merged view is during a run: a Unix path that starts with `copy`, or a
/// Wine path from its first `Z:` or `z:` (`-D:"Z:\i\Stock Game\Data"`), kept
/// in its own form and separator. None when it does not point into the copy.
pub fn onto_install(value: &str, copy: &Path, install: &Path) -> Option<String> {
    let (copy, install) = (copy.to_str()?, install.to_str()?);
    let moved = |path: &str, sep: char| {
        let rest = path.strip_prefix(&copy.replace('/', &sep.to_string()))?;
        (rest.is_empty() || rest.starts_with([sep, '"']))
            .then(|| format!("{}{rest}", install.replace('/', &sep.to_string())))
    };
    moved(value, '/').or_else(|| {
        let at = (0..value.len())
            .find(|&i| matches!(value.as_bytes()[i..], [b'Z' | b'z', b':', b'\\' | b'/', ..]))?;
        let sep = char::from(value.as_bytes()[at + 2]);
        Some(format!(
            "{}{}",
            &value[..at + 2],
            moved(&value[at + 2..], sep)?
        ))
    })
}

/// Whether the game folder `root` can be swapped for a copy: false for a game
/// that keeps its plugin list in the game folder (Morrowind, Oblivion with
/// `bUseMyGamesDirectory=0`), where the launch binds profile state over it.
pub fn supported(game: &DetectedGame, root: &Path) -> bool {
    game.plugin_spec().is_none_or(|spec| {
        eidos_plugins::plugin_state_dir(&game.prefix().unwrap_or_default(), root, &spec) != root
    })
}

/// Whether `root` holds the game's executable, in any case, also in a
/// subfolder (`bin64/CrimsonDesert.exe`).
pub fn has_game_binary(game: &DetectedGame, root: &Path) -> bool {
    game.def.game_binary.is_empty() || game_binary_in(game, root).is_some()
}

/// The game's executable in `root`, each path component matched in any case.
fn game_binary_in(game: &DetectedGame, root: &Path) -> Option<PathBuf> {
    game.def
        .game_binary
        .split('/')
        .try_fold(root.to_path_buf(), |dir, part| {
            std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
                let n = e.file_name();
                n.to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(part))
                    .then(|| e.path())
            })
        })
}

/// The file version of the game's executable in `root`.
fn binary_version(game: &DetectedGame, root: &Path) -> Option<String> {
    let exe = game_binary_in(game, root)?;
    let info = eidos_gamefeatures::preflight::inspect_pe(&exe).ok()?;
    info.file_version.map(|v| v.to_string())
}

/// The folder of a Wabbajack list's game copy in `root` that `game` can run
/// from: it holds the game's executable, and neither it nor the store install
/// keeps the plugin list in the game folder. Writes nothing.
pub fn list_copy(game: &DetectedGame, root: &Path) -> Option<String> {
    let folder = Instance::stock_game_folder(root)?;
    let copy = root.join(&folder);
    (has_game_binary(game, &copy) && supported(game, &game.install_path) && supported(game, &copy))
        .then_some(folder)
}

/// Point a list folder adopted for the first time at its game copy, the way
/// MO2 runs the list. The folder's name, or None with nothing written.
pub fn adopt_list_copy(game: &DetectedGame, inst: &Instance) -> Option<String> {
    let folder = list_copy(game, &inst.root)?;
    let adopted = inst
        .try_lock("adopting the list's game copy")
        .and_then(|_lock| inst.edit_manifest(|m| m.base_game = Some(folder.clone())));
    match adopted {
        Ok(()) => Some(folder),
        Err(error) => {
            eidos_log::warn!("Cannot run this list from '{folder}': {error}");
            None
        }
    }
}

/// What an instance's game files are and which copies it could switch to, for
/// `eidos stock-game` and the window's Game files card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameFilesFacts {
    pub files: GameFiles,
    /// A list's game copy folder, unless it is the copy a freeze made.
    pub list_folder: Option<String>,
    /// The copy a freeze made, while its folder exists.
    pub frozen: Option<String>,
    /// A freeze left an unfinished copy at [`unfinished_path`].
    pub unfinished: bool,
    pub store_version: Option<String>,
    /// The active copy's version, else the frozen copy's, else the list folder's.
    pub copy_version: Option<String>,
    /// Neither the install nor that copy keeps the plugin list in the game folder.
    pub supported: bool,
}

/// Where a freeze copies the install before the copy is complete.
pub fn unfinished_path(inst: &Instance) -> PathBuf {
    inst.root.join(format!("{FROZEN_COPY}.eidos-tmp"))
}

/// The game files `inst` runs `game` from and the copies it holds. Reads only.
pub fn facts(game: &DetectedGame, inst: &Instance) -> GameFilesFacts {
    let files = game_files(game, inst);
    let frozen_copy =
        (inst.read_manifest().and_then(|m| m.frozen_copy)).filter(|f| f == FROZEN_COPY);
    let list_folder =
        Instance::stock_game_folder(&inst.root).filter(|f| frozen_copy.as_ref() != Some(f));
    let frozen = frozen_copy.filter(|f| inst.root.join(f).is_dir());
    let candidate = (files.copy.as_ref())
        .or(frozen.as_ref())
        .or(list_folder.as_ref())
        .map(|f| inst.root.join(f));
    let install = &game.install_path;
    GameFilesFacts {
        unfinished: unfinished_path(inst).exists(),
        store_version: binary_version(game, install),
        copy_version: candidate.as_deref().and_then(|c| binary_version(game, c)),
        supported: supported(game, install)
            && candidate.as_deref().is_none_or(|c| supported(game, c)),
        files,
        list_folder,
        frozen,
    }
}

/// Run `game` from the copy `folder` in `inst`, or from the store install with
/// None. A folder that cannot run is refused with the reason, nothing written.
pub fn set_base(game: &DetectedGame, inst: &Instance, folder: Option<&str>) -> Result<(), String> {
    if let Some(folder) = folder {
        let copy = inst.root.join(folder);
        if !is_copy_folder_name(folder) || !copy.is_dir() {
            return Err(format!(
                "'{folder}' is not a folder in {}",
                inst.root.display()
            ));
        }
        // A freeze still copying, or one that stopped halfway: a partial Data folder.
        if folder.ends_with(".eidos-tmp") {
            return Err(format!(
                "'{folder}' is an unfinished copy and cannot run; delete it or freeze again"
            ));
        }
        if !has_game_binary(game, &copy) {
            return Err(format!(
                "{} has no {}, so the game cannot run from it",
                copy.display(),
                game.def.game_binary
            ));
        }
        if !supported(game, &game.install_path) || !supported(game, &copy) {
            return Err(format!(
                "{} keeps its plugin list in the game folder, so it cannot run from a copy",
                game.def.name
            ));
        }
    }
    let _lock = inst
        .try_lock("changing the game files")
        .map_err(|e| e.to_string())?;
    inst.edit_manifest(|m| m.base_game = folder.map(str::to_owned))
        .map_err(|e| e.to_string())
}

/// How a freeze copies the install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyKind {
    /// `FICLONE`: the copy shares the install's blocks until a file changes.
    Reflink,
    /// A byte copy on the same filesystem, which cannot share blocks.
    Full,
    /// A byte copy onto another filesystem.
    OtherFilesystem,
}

/// A reflink whenever the probe clone works, else a full copy. The probe
/// decides, not `st_dev`: two btrfs subvolumes (`@home`, `@games`) report
/// different `st_dev` on one filesystem, and a clone between them works.
pub fn copy_kind(same_filesystem: bool, clone_works: bool) -> CopyKind {
    match (clone_works, same_filesystem) {
        (true, _) => CopyKind::Reflink,
        (false, true) => CopyKind::Full,
        (false, false) => CopyKind::OtherFilesystem,
    }
}

/// What freezing the install would copy, how, and the space it has.
#[derive(Debug, Clone)]
pub struct FreezePlan {
    pub from: PathBuf,
    pub to: PathBuf,
    pub bytes: u64,
    pub files: u64,
    pub kind: CopyKind,
    /// The instance's filesystem, None when Eidos does not know its name.
    pub fs: Option<&'static str>,
    /// Free bytes on the instance's filesystem, None when statvfs fails.
    pub free: Option<u64>,
}

impl FreezePlan {
    /// Whether the copy fits in the free space. A reflink copy shares the
    /// install's blocks, and an unknown free space does not refuse.
    pub fn fits(&self) -> bool {
        self.kind == CopyKind::Reflink || self.free.is_none_or(|f| f >= self.bytes)
    }
}

/// Bytes and count of the regular files under `dir`; links are not followed.
pub fn tree_size(dir: &Path) -> io::Result<(u64, u64)> {
    let mut total = (0, 0);
    tally(dir, &mut total, &mut None)?;
    Ok(total)
}

/// Adds the regular files under `dir` to `total` and keeps the first one in `first`.
fn tally(dir: &Path, total: &mut (u64, u64), first: &mut Option<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() {
            tally(&path, total, first)?;
        } else if meta.is_file() {
            *total = (total.0 + meta.len(), total.1 + 1);
            first.get_or_insert(path);
        }
    }
    Ok(())
}

/// What freezing the game at `install` into `inst` would copy, how, and the
/// space left. Writes only the reflink probe's file, and removes it.
pub fn freeze_plan(install: &Path, inst: &Instance) -> io::Result<FreezePlan> {
    use std::os::unix::fs::MetadataExt;
    let (mut total, mut sample) = ((0, 0), None);
    tally(install, &mut total, &mut sample)?;
    let same_filesystem = fs::metadata(install)?.dev() == fs::metadata(&inst.root)?.dev();
    let clone_works = sample.is_some_and(|s| reflink_probe(&s, &inst.root));
    Ok(FreezePlan {
        from: install.to_path_buf(),
        to: inst.root.join(FROZEN_COPY),
        bytes: total.0,
        files: total.1,
        kind: copy_kind(same_filesystem, clone_works),
        fs: fs_magic(&inst.root).and_then(fs_name),
        free: eidos_transfer::free_bytes(&inst.root),
    })
}

/// The name of the filesystem whose statfs magic is `magic`.
pub fn fs_name(magic: i64) -> Option<&'static str> {
    Some(match magic {
        0xEF53 => "ext4",
        0x9123683E => "btrfs",
        0x58465342 => "XFS",
        0xCA451A4E => "bcachefs",
        0x2FC12FC1 => "ZFS",
        0xF2F52010 => "F2FS",
        0x01021994 => "tmpfs",
        0x7366746E => "NTFS",
        0x65735546 => "FUSE",
        _ => return None,
    })
}

/// The statfs magic of the filesystem holding `path`.
fn fs_magic(path: &Path) -> Option<i64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: libc::statfs is plain data, for which all zeros is a valid value.
    let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is NUL-terminated and outlives the call, which writes only into `buf`.
    let ok = unsafe { libc::statfs(c.as_ptr(), &mut buf) } == 0;
    // Through u32: every magic is 32 bits, and a 32-bit target's f_type is signed.
    ok.then_some(buf.f_type as u32 as i64)
}

/// Whether `sample` clones into `dir`, tried on a probe file that is removed
/// again. Any error answers false.
fn reflink_probe(sample: &Path, dir: &Path) -> bool {
    let probe = dir.join(".eidos-reflink-probe.eidos-tmp");
    let Ok(src) = File::open(sample) else {
        return false;
    };
    let Ok(dst) = File::options().write(true).create_new(true).open(&probe) else {
        return false;
    };
    let cloned = clone_file(&src, &dst).is_ok();
    let removed = fs::remove_file(&probe).is_ok();
    cloned && removed
}

/// `dst` made to share the blocks of `src` (`FICLONE`).
fn clone_file(src: &File, dst: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: both descriptors stay open for the call, and FICLONE reads only the source fd it is passed.
    match unsafe { libc::ioctl(dst.as_raw_fd(), libc::FICLONE, src.as_raw_fd()) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

/// Copy the tree `from` into `to`, which must not exist: folders with their
/// mode, regular files with their mode and mtime (Oblivion and the Fallout 3
/// era games order plugins by time), symlinks as symlinks with the same target.
/// With `reflink`, each file is cloned (`FICLONE`), or copied when the clone
/// fails. Other file types are skipped with a warning. `stop` is asked before
/// each file; true ends the copy with `Interrupted`. The regular files copied.
pub fn copy_tree(
    from: &Path,
    to: &Path,
    reflink: bool,
    stop: &dyn Fn() -> bool,
) -> io::Result<u64> {
    let mut counts = (0, 0);
    copy_dir(from, to, reflink, stop, &mut counts)?;
    if counts.1 > 0 {
        eidos_log::warn!(
            "Skipped {} items under {} that are not files, folders or links",
            counts.1,
            from.display()
        );
    }
    Ok(counts.0)
}

/// [`copy_tree`] for one folder; `counts` is (files copied, items skipped).
fn copy_dir(
    from: &Path,
    to: &Path,
    reflink: bool,
    stop: &dyn Fn() -> bool,
    counts: &mut (u64, u64),
) -> io::Result<()> {
    let mode = fs::metadata(from)?.permissions();
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        let meta = fs::symlink_metadata(&src)?;
        if meta.is_dir() {
            copy_dir(&src, &dst, reflink, stop, counts)?;
            continue;
        }
        if !meta.is_file() && !meta.is_symlink() {
            counts.1 += 1;
            continue;
        }
        if stop() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "stopped"));
        }
        if meta.is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(&src)?, &dst)?;
        } else {
            copy_file(&src, &dst, &meta, reflink)?;
            counts.0 += 1;
        }
    }
    // Set last, so a read-only source folder still takes its files.
    fs::set_permissions(to, mode)
}

/// One regular file, cloned when `reflink` and the clone works, else copied,
/// then given the source's mode and mtime.
fn copy_file(src: &Path, dst: &Path, meta: &fs::Metadata, reflink: bool) -> io::Result<()> {
    let cloned = reflink
        && File::open(src)
            .and_then(|s| clone_file(&s, &File::options().write(true).create_new(true).open(dst)?))
            .is_ok();
    if !cloned {
        fs::copy(src, dst)?;
    }
    let file = File::open(dst)?;
    file.set_modified(meta.modified()?)?;
    file.set_permissions(meta.permissions())
}

/// The refusals a freeze checks before it locks anything: an instance inside
/// the game folder, a copy in the way, a game that keeps its plugin list in the
/// game folder, Steam writing the game.
fn refuse_freeze(game: &DetectedGame, inst: &Instance) -> Result<(), String> {
    // The copy would go under the store install and walk into itself as it grows.
    if Instance::root_inside_game(&inst.root, &game.install_path) {
        return Err(format!(
            "{} is inside the game's own folder ({}), so the copy would be written into \
             the store install. Move the instance folder out (a sibling of the game folder \
             works), then freeze again.",
            inst.root.display(),
            game.install_path.display()
        ));
    }
    let f = facts(game, inst);
    let i = shell_word(&inst.root.to_string_lossy());
    if let Some(folder) = &f.list_folder {
        let hint = if f.files.copy.as_ref() == Some(folder) {
            String::new()
        } else {
            format!(
                " `eidos stock-game {i} use {}` runs it.",
                shell_word(folder)
            )
        };
        return Err(format!(
            "{} holds the list's own game copy '{folder}', so Eidos does not make another one.{hint}",
            inst.root.display()
        ));
    }
    if let Some(copy) = &f.files.copy {
        return Err(format!(
            "The instance already runs the copy '{copy}'. To freeze the store version, \
             return to it first with `eidos stock-game {i} store`."
        ));
    }
    if let Some(folder) = &f.frozen {
        return Err(format!(
            "{} already holds a frozen copy. `eidos stock-game {i} use {}` runs it, and \
             `eidos stock-game {i} delete --yes` removes it.",
            inst.root.join(folder).display(),
            shell_word(folder)
        ));
    }
    if f.unfinished {
        return Err(format!(
            "{} holds an unfinished copy from an earlier freeze. \
             `eidos stock-game {i} delete --yes` removes it.",
            unfinished_path(inst).display()
        ));
    }
    if !supported(game, &game.install_path) {
        return Err(format!(
            "{} keeps its plugin list in the game folder, so it cannot run from a copy",
            game.def.name
        ));
    }
    // shortcut: no update state is read for Heroic or Legendary copies, read Heroic's gamesConfig/legendary installed.json when a user hits a mixed copy
    if matches!(game.source, eidos_games::GameSource::Steam) {
        // 4 = fully installed, 2 = an update queued; any other bit means Steam is writing.
        let busy = eidos_games::steam_state_flags(&game.install_path, game.def.steam_app_id)
            .filter(|f| f & 4 == 0 || f & !(4 | 2) != 0);
        if let Some(flags) = busy {
            return Err(format!(
                "Steam reports the game as busy or not fully installed (StateFlags {flags}); \
                 wait for Steam to finish"
            ));
        }
    }
    Ok(())
}

/// The sentence for a plan that does not [`FreezePlan::fits`].
fn no_space(plan: &FreezePlan) -> String {
    format!(
        "Not enough space: the copy needs {} and {} is free in {}",
        eidos_transfer::human_bytes(plan.bytes),
        eidos_transfer::human_bytes(plan.free.unwrap_or(0)),
        plan.to.parent().unwrap_or(&plan.to).display()
    )
}

/// Copy the store install into the instance's [`FROZEN_COPY`] and run the game
/// from it. The copy goes to [`unfinished_path`] first and is renamed once it
/// is complete and Steam's build id did not change meanwhile; a stop, a failure
/// or a build change leaves only that unfinished folder and the manifest as it
/// was. Each refusal is a sentence, with nothing written.
pub fn freeze(
    game: &DetectedGame,
    inst: &Instance,
    stop: &dyn Fn() -> bool,
) -> Result<FreezePlan, String> {
    refuse_freeze(game, inst)?;
    let _lock = inst
        .try_lock("copying the game files")
        .map_err(|e| e.to_string())?;
    let install = &game.install_path;
    let plan = freeze_plan(install, inst)
        .map_err(|e| format!("Could not measure {}: {e}", install.display()))?;
    if !plan.fits() {
        return Err(no_space(&plan));
    }
    // A central instance gets its manifest on first launch. Without one the
    // finished copy would read as a list's own folder, which delete leaves.
    inst.ensure_manifest(game.def.id, InstanceKind::Global)
        .map_err(|e| format!("Could not write the instance manifest: {e}"))?;
    if inst.read_manifest().is_none() {
        return Err(format!(
            "{} cannot be read, so the copy could not be recorded; fix it, then freeze again",
            inst.manifest_path().display()
        ));
    }
    let tmp = unfinished_path(inst);
    let unfinished = |why: String| {
        format!(
            "{why}; the unfinished copy is at {}, and `eidos stock-game {} delete --yes` removes it.",
            tmp.display(),
            shell_word(&inst.root.to_string_lossy())
        )
    };
    let app_id = game.def.steam_app_id;
    let before = eidos_games::steam_build_id(install, app_id);
    eidos_log::info!(
        "Copying {} in {} files from {} to {}",
        eidos_transfer::human_bytes(plan.bytes),
        plan.files,
        install.display(),
        tmp.display()
    );
    if let Err(e) = copy_tree(install, &tmp, plan.kind == CopyKind::Reflink, stop) {
        let why = match e.kind() {
            io::ErrorKind::Interrupted => "The copy was stopped".to_owned(),
            _ => format!("The copy failed: {e}"),
        };
        return Err(if tmp.exists() { unfinished(why) } else { why });
    }
    let after = eidos_games::steam_build_id(install, app_id);
    if let Some(b) = before.as_deref().filter(|b| after.as_deref() != Some(*b)) {
        return Err(unfinished(format!(
            "Steam changed the game from build {b} to {} while copying",
            after.as_deref().unwrap_or("unknown")
        )));
    }
    fs::rename(&tmp, &plan.to)
        .map_err(|e| unfinished(format!("Could not move the copy into place: {e}")))?;
    inst.edit_manifest(|m| {
        m.base_game = Some(FROZEN_COPY.into());
        m.frozen_copy = Some(FROZEN_COPY.into());
    })
    .map_err(|e| {
        format!(
            "The copy is at {}, but the instance manifest could not record it: {e}",
            plan.to.display()
        )
    })?;
    Ok(plan)
}

/// Delete the copy a freeze made and any unfinished one, never anything else
/// (a list's own game copy stays). Refused while the frozen copy runs. The
/// paths removed.
pub fn delete_copy(inst: &Instance) -> Result<Vec<PathBuf>, String> {
    let _lock = inst
        .try_lock("deleting the game copy")
        .map_err(|e| e.to_string())?;
    let (base, frozen) = inst
        .read_manifest()
        // Only freeze writes the key, always as FROZEN_COPY; a hand-edited
        // `frozen_copy=mods` must never reach remove_dir_all.
        .map(|m| (m.base_game, m.frozen_copy.filter(|f| f == FROZEN_COPY)))
        .unwrap_or_default();
    if let Some(f) = frozen.as_ref().filter(|f| base.as_ref() == Some(*f)) {
        return Err(format!(
            "The instance runs the frozen copy '{f}'; return to the store version first \
             with `eidos stock-game {} store`.",
            shell_word(&inst.root.to_string_lossy())
        ));
    }
    let mut removed = Vec::new();
    for path in frozen
        .iter()
        .map(|f| inst.root.join(f))
        .chain([unfinished_path(inst)])
    {
        if fs::symlink_metadata(&path).is_ok() {
            fs::remove_dir_all(&path)
                .map_err(|e| format!("Could not delete {}: {e}", path.display()))?;
            removed.push(path);
        }
    }
    if frozen.is_some() {
        inst.edit_manifest(|m| m.frozen_copy = None)
            .map_err(|e| e.to_string())?;
    }
    Ok(removed)
}

const USAGE: &str =
    "usage: eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]";

/// `s` as one POSIX shell word, quoted only when it needs it.
fn shell_word(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c))
    {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

/// What `eidos stock-game` was asked to do, besides `delete --yes`.
enum Verb<'a> {
    Show,
    /// `use <folder>` (Some) or `store` (None).
    Set(Option<&'a str>),
    /// `freeze`, which only prints the plan without `--yes`.
    Freeze {
        yes: bool,
    },
}

/// `eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`.
pub(crate) fn cmd_stock_game(args: &[String]) {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    // Deleting needs no game: the copy can outlive the store install.
    if let [arg, "delete", "--yes"] = words[..] {
        return delete_command(&crate::resolve::resolve(arg).inst);
    }
    let (arg, verb) = match words[..] {
        [arg] => (arg, Verb::Show),
        [arg, "use", folder] => (arg, Verb::Set(Some(folder))),
        [arg, "store"] => (arg, Verb::Set(None)),
        [arg, "freeze"] => (arg, Verb::Freeze { yes: false }),
        [arg, "freeze", "--yes"] => (arg, Verb::Freeze { yes: true }),
        _ => {
            eidos_log::info!("{USAGE}");
            exit(2);
        }
    };
    let target = crate::resolve::resolve(arg);
    let Some(game) = crate::games::find_instance_game(&target) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(1);
    };
    let inst = &target.inst;
    let folder = match verb {
        Verb::Show => return print_facts(arg, &game, inst),
        Verb::Freeze { yes } => return freeze_command(&game, inst, yes),
        Verb::Set(folder) => folder,
    };
    let before = game_files(&game, inst).copy;
    if let Err(reason) = set_base(&game, inst, folder) {
        eidos_log::info!("eidos: {reason}");
        exit(1);
    }
    match (folder, before) {
        (Some(f), _) => println!(
            "The next launch runs the game files in '{f}'. The game still starts through its store."
        ),
        (None, Some(old)) => {
            println!("The next launch runs the store install. '{old}' is still on disk.")
        }
        (None, None) => println!("The next launch runs the store install."),
    }
}

/// `delete --yes`: each path removed, or the refusal with exit 1.
fn delete_command(inst: &Instance) {
    match delete_copy(inst) {
        Ok(removed) if removed.is_empty() => {
            println!("There is no game copy made by Eidos to delete.")
        }
        Ok(removed) => removed
            .iter()
            .for_each(|p| println!("Deleted {}", p.display())),
        Err(reason) => {
            eidos_log::info!("eidos: {reason}");
            exit(1);
        }
    }
}

/// `freeze`: the plan, or with `yes` the copy. A refusal exits 1.
fn freeze_command(game: &DetectedGame, inst: &Instance, yes: bool) {
    let refuse = |reason: String| -> ! {
        eidos_log::info!("eidos: {reason}");
        exit(1);
    };
    if yes {
        eidos_launch::install_stop_handler();
        match freeze(game, inst, &eidos_launch::stop_requested) {
            Ok(plan) => println!(
                "Frozen: {} now runs from {}.",
                game.def.name,
                plan.to.display()
            ),
            Err(reason) => refuse(reason),
        }
        return;
    }
    // The same refusals as with --yes, so the plan is never one that cannot run.
    if let Err(reason) = refuse_freeze(game, inst) {
        refuse(reason);
    }
    let plan = freeze_plan(&game.install_path, inst).unwrap_or_else(|e| {
        refuse(format!(
            "Could not measure {}: {e}",
            game.install_path.display()
        ))
    });
    let size = eidos_transfer::human_bytes(plan.bytes);
    let room = match plan.free {
        Some(free) => format!("{size} of {} free", eidos_transfer::human_bytes(free)),
        None => format!("{size}, free space unknown"),
    };
    let fs = plan.fs.unwrap_or("this filesystem");
    println!("from: {}", plan.from.display());
    println!("to: {}", plan.to.display());
    println!("size: {size} in {} files", plan.files);
    match plan.kind {
        CopyKind::Reflink => println!(
            "copy: shared blocks on {fs} (reflink); almost no space is used until the store changes a file"
        ),
        CopyKind::Full => println!("copy: full copy, {fs} cannot share blocks ({room})"),
        CopyKind::OtherFilesystem => println!(
            "copy: full copy, the game and the instance are on different filesystems ({room})"
        ),
    }
    if !plan.fits() {
        refuse(no_space(&plan));
    }
    println!("Run again with --yes to copy.");
}

fn print_facts(arg: &str, game: &DetectedGame, inst: &Instance) {
    let f = facts(game, inst);
    let version = |v: &Option<String>| format!("version {}", v.as_deref().unwrap_or("unknown"));
    println!("Game files for {}", inst.root.display());
    let active = f.files.copy.as_deref();
    match active {
        None => println!(
            "runs: the store install, {}, {}",
            version(&f.store_version),
            game.install_path.display()
        ),
        Some(copy) => println!(
            "runs: the copy '{copy}', {} (the store install has {})",
            version(&f.copy_version),
            version(&f.store_version)
        ),
    }
    let others = |folder: &Option<String>| folder.clone().filter(|f| Some(f.as_str()) != active);
    let (list, frozen) = (others(&f.list_folder), others(&f.frozen));
    if let Some(folder) = &list {
        println!("available: the list's game copy '{folder}'");
    }
    if let Some(folder) = &frozen {
        println!("available: the frozen copy '{folder}'");
    }
    if f.unfinished {
        println!(
            "available: an unfinished copy at {}, which cannot run",
            unfinished_path(inst).display()
        );
    }
    if !f.supported {
        println!(
            "{} keeps its plugin list in the game folder, so it cannot run from a copy yet.",
            game.def.name
        );
    }
    let arg = shell_word(arg);
    let mut commands: Vec<String> = Vec::new();
    if f.supported {
        for folder in list.iter().chain(&frozen) {
            commands.push(format!("eidos stock-game {arg} use {}", shell_word(folder)));
        }
    }
    if active.is_some() {
        commands.push(format!("eidos stock-game {arg} store"));
    }
    // What `refuse_freeze` checks first, short of reading Steam's state.
    let nothing_in_the_way = active.is_none() && f.list_folder.is_none() && f.frozen.is_none();
    if f.supported && nothing_in_the_way && !f.unfinished {
        commands.push(format!("eidos stock-game {arg} freeze"));
    }
    if frozen.is_some() || f.unfinished {
        commands.push(format!("eidos stock-game {arg} delete --yes"));
    }
    if !commands.is_empty() {
        println!("commands:");
    }
    for c in commands {
        println!("  {c}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eidos-stock-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("game")).unwrap();
        d
    }

    fn game(id: &str, install: PathBuf) -> DetectedGame {
        DetectedGame {
            def: eidos_games::GameDef::for_id(id).unwrap(),
            source: Default::default(),
            data_path: install.join("Data"),
            install_path: install,
            compatdata: None,
            steam_name: id.into(),
        }
    }

    fn list(t: &Path, id: &str) -> Instance {
        let inst = Instance::portable(t.join("list"));
        fs::create_dir_all(inst.root.join("Stock Game")).unwrap();
        inst.ensure_manifest(id, InstanceKind::Portable).unwrap();
        inst
    }

    #[test]
    fn adopting_a_list_folder_runs_its_copy_only_when_it_holds_the_game() {
        let t = temp("adopt");
        let game = game("skyrimse", t.join("game"));
        let inst = list(&t, "skyrimse");
        assert_eq!(adopt_list_copy(&game, &inst), None);
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        fs::write(inst.root.join("Stock Game/SKYRIMSE.EXE"), b"x").unwrap();
        assert_eq!(adopt_list_copy(&game, &inst).as_deref(), Some("Stock Game"));
        assert_eq!(
            inst.read_manifest().unwrap().base_game.as_deref(),
            Some("Stock Game")
        );
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn the_game_binary_is_found_in_a_subfolder_in_any_case() {
        let t = temp("subfolder");
        let game = game("crimsondesert", t.join("game"));
        let copy = t.join("Stock Game");
        fs::create_dir_all(copy.join("BIN64")).unwrap();
        assert!(!has_game_binary(&game, &copy));
        fs::write(copy.join("BIN64/crimsondesert.exe"), b"x").unwrap();
        assert!(has_game_binary(&game, &copy));
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn copy_paths_move_onto_the_install_in_both_spellings() {
        let (c, i) = (Path::new("/i/Stock Game"), Path::new("/g/Skyrim"));
        assert_eq!(
            onto_install("/i/Stock Game/SKSE/x.exe", c, i).as_deref(),
            Some("/g/Skyrim/SKSE/x.exe")
        );
        assert_eq!(
            onto_install("-D:\"Z:\\i\\Stock Game\\Data\"", c, i).as_deref(),
            Some("-D:\"Z:\\g\\Skyrim\\Data\"")
        );
        assert_eq!(
            onto_install("z:/i/Stock Game/Data", c, i).as_deref(),
            Some("z:/g/Skyrim/Data")
        );
        assert_eq!(onto_install("/i/Stock Games/x", c, i), None);
        assert_eq!(onto_install("-IKnowWhatImDoing", c, i), None);
    }

    fn copy_instance(t: &Path) -> Instance {
        let inst = Instance::portable(t.join("i"));
        fs::create_dir_all(&inst.root).unwrap();
        inst.ensure_manifest("skyrimse", InstanceKind::Portable)
            .unwrap();
        inst
    }

    #[test]
    fn use_then_store_switches_the_manifest_and_refuses_what_is_not_a_copy() {
        let t = temp("use-store");
        let game = game("skyrimse", t.join("game"));
        let inst = copy_instance(&t);
        fs::create_dir_all(inst.root.join("Game Root")).unwrap();
        fs::write(inst.root.join("Game Root/SkyrimSE.exe"), b"x").unwrap();
        fs::create_dir_all(inst.root.join("Empty")).unwrap();
        assert!(set_base(&game, &inst, Some("Empty"))
            .unwrap_err()
            .contains("SkyrimSE.exe"));
        assert!(set_base(&game, &inst, Some("../x")).is_err());
        assert!(set_base(&game, &inst, Some("Missing")).is_err());
        fs::create_dir_all(unfinished_path(&inst)).unwrap();
        fs::write(unfinished_path(&inst).join("SkyrimSE.exe"), b"x").unwrap();
        assert!(set_base(&game, &inst, Some("Stock Game.eidos-tmp"))
            .unwrap_err()
            .contains("unfinished"));
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        set_base(&game, &inst, Some("Game Root")).unwrap();
        assert_eq!(
            inst.read_manifest().unwrap().base_game.as_deref(),
            Some("Game Root")
        );
        set_base(&game, &inst, None).unwrap();
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        assert!(inst.root.join("Game Root/SkyrimSE.exe").is_file());
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn facts_offer_the_list_folder_but_not_the_frozen_copy_as_a_list_folder() {
        let t = temp("facts");
        let game = game("skyrimse", t.join("game"));
        let inst = copy_instance(&t);
        fs::create_dir_all(inst.root.join("Stock Game")).unwrap();
        fs::write(inst.root.join("Stock Game/SkyrimSE.exe"), b"x").unwrap();
        inst.edit_manifest(|m| m.frozen_copy = Some("Stock Game".into()))
            .unwrap();
        let f = facts(&game, &inst);
        assert_eq!(
            (f.list_folder, f.frozen.as_deref(), f.unfinished),
            (None, Some("Stock Game"), false)
        );
        assert!(f.supported && f.files.copy.is_none());
        fs::create_dir_all(unfinished_path(&inst)).unwrap();
        assert!(facts(&game, &inst).unfinished);
        inst.edit_manifest(|m| m.frozen_copy = None).unwrap();
        let f = facts(&game, &inst);
        assert_eq!(
            (f.list_folder.as_deref(), f.frozen),
            (Some("Stock Game"), None)
        );
        let _ = fs::remove_dir_all(t);
    }

    /// A small game folder in `dir`: an executable (0o755, dated 2016-10-27), a
    /// plugin in Data and a symlink. Its regular files' size in bytes.
    fn game_tree(dir: &Path) -> u64 {
        use std::os::unix::fs::PermissionsExt;
        fs::create_dir_all(dir.join("Data")).unwrap();
        fs::write(dir.join("SkyrimSE.exe"), b"MZ game").unwrap();
        fs::write(dir.join("Data/Skyrim.esm"), b"TES4 plugin bytes").unwrap();
        let exe = fs::File::options()
            .write(true)
            .open(dir.join("SkyrimSE.exe"))
            .unwrap();
        exe.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_477_526_400))
            .unwrap();
        exe.set_permissions(fs::Permissions::from_mode(0o755))
            .unwrap();
        std::os::unix::fs::symlink("SkyrimSE.exe", dir.join("link")).unwrap();
        (b"MZ game".len() + b"TES4 plugin bytes".len()) as u64
    }

    #[test]
    fn copy_tree_keeps_files_dirs_links_modes_and_times() {
        use std::os::unix::fs::PermissionsExt;
        let t = temp("copy-tree");
        let (from, to) = (t.join("from"), t.join("to"));
        game_tree(&from);
        let n = copy_tree(&from, &to, false, &|| false).unwrap();
        assert_eq!(n, 2);
        assert_eq!(
            fs::read(to.join("Data/Skyrim.esm")).unwrap(),
            fs::read(from.join("Data/Skyrim.esm")).unwrap()
        );
        let (a, b) = (
            fs::metadata(from.join("SkyrimSE.exe")).unwrap(),
            fs::metadata(to.join("SkyrimSE.exe")).unwrap(),
        );
        assert_eq!(
            (a.permissions().mode() & 0o777, a.modified().unwrap()),
            (b.permissions().mode() & 0o777, b.modified().unwrap())
        );
        assert_eq!(
            fs::read_link(to.join("link")).unwrap(),
            Path::new("SkyrimSE.exe")
        );
        assert!(
            copy_tree(&from, &to, false, &|| false).is_err(),
            "to exists"
        );
        // reflink = true gives the same tree whatever the filesystem (FICLONE, or a plain copy per file)
        let to2 = to.with_extension("reflink");
        assert_eq!(copy_tree(&from, &to2, true, &|| false).unwrap(), 2);
        assert_eq!(
            fs::read(to2.join("Data/Skyrim.esm")).unwrap(),
            fs::read(from.join("Data/Skyrim.esm")).unwrap()
        );
        assert_eq!(
            fs::metadata(to2.join("SkyrimSE.exe"))
                .unwrap()
                .modified()
                .unwrap(),
            a.modified().unwrap()
        );
        assert_eq!(
            fs::read_link(to2.join("link")).unwrap(),
            Path::new("SkyrimSE.exe")
        );
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn a_stopped_copy_returns_interrupted() {
        let t = temp("copy-stop");
        let (from, to) = (t.join("from"), t.join("to"));
        game_tree(&from);
        let calls = std::cell::Cell::new(0);
        let err = copy_tree(&from, &to, false, &|| {
            calls.set(calls.get() + 1);
            calls.get() > 1
        })
        .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Interrupted);
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn the_copy_kind_needs_one_filesystem_and_a_working_clone() {
        assert_eq!(copy_kind(true, true), CopyKind::Reflink);
        assert_eq!(copy_kind(true, false), CopyKind::Full);
        assert_eq!(copy_kind(false, false), CopyKind::OtherFilesystem);
        assert_eq!(
            copy_kind(false, true),
            CopyKind::Reflink,
            "btrfs subvolumes: one filesystem, two st_dev"
        );
        assert_eq!(fs_name(0xEF53), Some("ext4"));
        assert_eq!(fs_name(0x9123683E), Some("btrfs"));
        assert_eq!(fs_name(1), None);
    }

    #[test]
    fn freeze_plan_counts_the_install_and_never_fits_a_full_copy_into_less_space() {
        let t = temp("plan");
        let install = t.join("game");
        let expected_bytes = game_tree(&install);
        let inst = copy_instance(&t);
        let p = freeze_plan(&install, &inst).unwrap();
        assert_eq!(
            (p.files, p.bytes, p.to.clone()),
            (2, expected_bytes, inst.root.join("Stock Game"))
        );
        assert_eq!(p.from, install);
        assert_eq!(tree_size(&install).unwrap(), (expected_bytes, 2));
        assert!(!inst.root.join(".eidos-reflink-probe.eidos-tmp").exists());
        assert!(!FreezePlan {
            kind: CopyKind::Full,
            free: Some(p.bytes - 1),
            ..p.clone()
        }
        .fits());
        assert!(FreezePlan {
            kind: CopyKind::Full,
            free: None,
            ..p.clone()
        }
        .fits());
        assert!(FreezePlan {
            kind: CopyKind::Reflink,
            free: Some(0),
            ..p
        }
        .fits());
        let _ = fs::remove_dir_all(t);
    }

    /// `<lib>/steamapps/appmanifest_489830.acf` with `buildid` and `StateFlags`.
    fn write_acf(lib: &Path, build: &str, flags: &str) {
        fs::write(
            lib.join("steamapps/appmanifest_489830.acf"),
            format!("\"AppState\"\n{{\n\t\"buildid\"\t\t\"{build}\"\n\t\"StateFlags\"\t\t\"{flags}\"\n}}\n"),
        )
        .unwrap();
    }

    /// Skyrim SE in a Steam library at `t/lib` (build 1, fully installed), an
    /// instance at `t/i`, and the library folder.
    fn steam_freeze(t: &Path) -> (DetectedGame, Instance, PathBuf) {
        let lib = t.join("lib");
        let install = lib.join("steamapps/common/Game");
        game_tree(&install);
        write_acf(&lib, "1", "4");
        (game("skyrimse", install), copy_instance(t), lib)
    }

    #[test]
    fn a_finished_freeze_runs_the_copy_and_leaves_the_install_alone() {
        let t = temp("freeze");
        let (game, inst, _) = steam_freeze(&t);
        let plan = freeze(&game, &inst, &|| false).unwrap();
        assert_eq!(plan.to, inst.root.join("Stock Game"));
        let m = inst.read_manifest().unwrap();
        assert_eq!(
            (m.base_game.as_deref(), m.frozen_copy.as_deref()),
            (Some("Stock Game"), Some("Stock Game"))
        );
        assert!(inst.root.join("Stock Game/Data/Skyrim.esm").is_file());
        assert!(!unfinished_path(&inst).exists());
        assert_eq!(tree_size(&game.install_path).unwrap().1, 2);
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn a_freeze_records_the_copy_in_an_instance_never_launched() {
        let t = temp("freeze-new");
        let (game, _, _) = steam_freeze(&t);
        let inst = Instance::portable(t.join("central"));
        fs::create_dir_all(&inst.root).unwrap();
        freeze(&game, &inst, &|| false).unwrap();
        let m = inst.read_manifest().unwrap();
        assert_eq!(
            (m.game_id.as_str(), m.kind),
            ("skyrimse", InstanceKind::Global)
        );
        assert_eq!(m.frozen_copy.as_deref(), Some("Stock Game"));
        // A manifest Eidos cannot read is refused before anything is copied.
        let inst = Instance::portable(t.join("damaged"));
        fs::create_dir_all(&inst.root).unwrap();
        fs::write(inst.manifest_path(), "[eidos]\nkind=global\n").unwrap();
        assert!(freeze(&game, &inst, &|| false)
            .unwrap_err()
            .contains("cannot be read"));
        assert!(!inst.root.join("Stock Game").exists() && !unfinished_path(&inst).exists());
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn freeze_refuses_while_steam_updates_the_game() {
        let t = temp("freeze-busy");
        let (game, inst, lib) = steam_freeze(&t);
        write_acf(&lib, "1", "1026");
        assert!(freeze(&game, &inst, &|| false)
            .unwrap_err()
            .contains("StateFlags 1026"));
        assert!(!inst.root.join("Stock Game").exists() && !unfinished_path(&inst).exists());
        // 6 = installed and an update queued: Steam is not writing, the copy runs.
        write_acf(&lib, "1", "6");
        freeze(&game, &inst, &|| false).unwrap();
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn a_build_change_during_the_copy_keeps_it_unfinished() {
        let t = temp("freeze-build");
        let (game, inst, lib) = steam_freeze(&t);
        let err = freeze(&game, &inst, &|| {
            write_acf(&lib, "2", "4");
            false
        })
        .unwrap_err();
        assert!(err.contains("build 1 to 2"), "{err}");
        assert!(unfinished_path(&inst).is_dir() && !inst.root.join("Stock Game").exists());
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn a_stopped_freeze_leaves_only_the_unfinished_folder() {
        let t = temp("freeze-stop");
        let (game, inst, _) = steam_freeze(&t);
        let err = freeze(&game, &inst, &|| true).unwrap_err();
        assert!(
            err.contains("stopped") && err.contains("delete --yes"),
            "{err}"
        );
        assert!(unfinished_path(&inst).is_dir() && !inst.root.join("Stock Game").exists());
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn freeze_refuses_while_an_unfinished_copy_exists() {
        let t = temp("freeze-unfinished");
        let (game, inst, _) = steam_freeze(&t);
        fs::create_dir_all(unfinished_path(&inst)).unwrap();
        assert!(freeze(&game, &inst, &|| false)
            .unwrap_err()
            .contains("delete --yes"));
        fs::remove_dir(unfinished_path(&inst)).unwrap();
        fs::create_dir_all(inst.root.join("Game Root")).unwrap();
        assert!(freeze(&game, &inst, &|| false)
            .unwrap_err()
            .contains("'Game Root'"));
        assert!(!inst.root.join("Stock Game").exists() && !unfinished_path(&inst).exists());
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn freeze_refuses_an_instance_inside_the_game_folder() {
        let t = temp("freeze-inside");
        let install = t.join("game");
        game_tree(&install);
        let game = game("skyrimse", install.clone());
        let inst = Instance::portable(install.join("Eidos"));
        fs::create_dir_all(&inst.root).unwrap();
        inst.ensure_manifest("skyrimse", InstanceKind::Portable)
            .unwrap();
        // Checked first: without the refusal, freeze would copy the install into itself.
        assert!(refuse_freeze(&game, &inst)
            .unwrap_err()
            .contains("inside the game's own folder"));
        assert!(freeze(&game, &inst, &|| false)
            .unwrap_err()
            .contains("inside the game's own folder"));
        assert!(!unfinished_path(&inst).exists() && !inst.root.join("Stock Game").exists());
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn delete_removes_only_the_copy_eidos_made_and_never_the_active_one() {
        let t = temp("delete");
        let (game, inst, _) = steam_freeze(&t);
        freeze(&game, &inst, &|| false).unwrap();
        fs::create_dir_all(inst.root.join("Game Root")).unwrap(); // a list's folder
        assert!(delete_copy(&inst).unwrap_err().contains("store version"));
        set_base(&game, &inst, None).unwrap();
        fs::create_dir_all(unfinished_path(&inst)).unwrap();
        assert_eq!(
            delete_copy(&inst).unwrap(),
            [inst.root.join("Stock Game"), unfinished_path(&inst)]
        );
        assert!(inst.root.join("Game Root").is_dir());
        assert!(game.install_path.join("Data/Skyrim.esm").is_file());
        assert_eq!(inst.read_manifest().unwrap().frozen_copy, None);
        assert_eq!(delete_copy(&inst).unwrap(), Vec::<PathBuf>::new());
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn a_hand_edited_frozen_copy_key_names_nothing_to_delete() {
        let t = temp("delete-edited");
        let inst = copy_instance(&t);
        let game = game("skyrimse", t.join("game"));
        for folder in ["mods", "Game Root"] {
            fs::create_dir_all(inst.root.join(folder)).unwrap();
            fs::write(inst.root.join(folder).join("SkyrimSE.exe"), b"x").unwrap();
            inst.edit_manifest(|m| m.frozen_copy = Some(folder.into()))
                .unwrap();
            assert_eq!(facts(&game, &inst).frozen, None, "{folder}");
            assert_eq!(delete_copy(&inst).unwrap(), Vec::<PathBuf>::new());
            assert!(inst.root.join(folder).join("SkyrimSE.exe").is_file());
        }
        // A list folder named in the key is still the list's copy.
        assert_eq!(
            facts(&game, &inst).list_folder.as_deref(),
            Some("Game Root")
        );
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn morrowind_never_adopts_a_copy() {
        let t = temp("morrowind");
        let game = game("morrowind", t.join("game"));
        let inst = list(&t, "morrowind");
        fs::write(inst.root.join("Stock Game/Morrowind.exe"), b"x").unwrap();
        assert_eq!(adopt_list_copy(&game, &inst), None);
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        let _ = fs::remove_dir_all(t);
    }
}
