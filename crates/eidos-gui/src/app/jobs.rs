//! Work that runs off the window's thread: an archive being opened, a pack or
//! unpack, and a game or tool the window is waiting on.

use crate::*;

/// An archive being opened on a worker thread.
///
/// Extraction used to run inside `update()`, on the window's own thread: a
/// 350 MB archive meant ~30 s during which iced processed no events at all, so
/// the compositor declared the window dead ("Eidos is not responding") and any
/// resize it had sent went unanswered until the extraction finished. The work
/// is the same; only the thread it runs on changed.
pub(crate) struct InstallJob {
    pub(crate) fresh: bool,
    pub(crate) cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(crate) target: Option<CollectionTarget>,
    /// The mod's name, for the dialog text.
    pub(crate) name: String,
    /// The archive being opened. Kept because the result handler needs it and
    /// the worker has moved its own copy away.
    pub(crate) archive: PathBuf,
    /// 7-Zip's own percentage, written by the worker as it reads them.
    pub(crate) percent: std::sync::Arc<std::sync::atomic::AtomicU8>,
    /// The worker's result, stored just before `done` flips.
    pub(crate) outcome: std::sync::Arc<
        std::sync::Mutex<Option<Result<eidos_install::Opened, eidos_install::InstallError>>>,
    >,
    /// Flipped to `true` by the worker once `outcome` is filled.
    pub(crate) done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for InstallJob {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// The worker hands `Opened` back across a thread; if that ever stops holding,
// fail here at compile time rather than at the `spawn` call site.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<eidos_install::Opened>();
    assert_send::<eidos_install::InstallError>();
    // A pack hands back sentences, not library types (see `TransferJob`), but
    // the worker owns an Instance and a Plan while it runs.
    assert_send::<eidos_instance::Instance>();
    assert_send::<eidos_transfer::Plan>();
    assert_send::<eidos_transfer::Transfer>();
};

/// Which way a transfer is going, for what the poll does when it finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferKind {
    Pack,
    Unpack,
    /// Installing a Nexus collection. In the same slot as the other two because
    /// they are all minutes of archive work on one instance, and running two at
    /// once would only make each slower.
    Collection,
}

/// A pack or an unpack running on a worker thread.
///
/// The same three-Arc shape as [`InstallJob`], for the same reason: twenty
/// minutes of 7-Zip inside `update()` would be twenty minutes of a window the
/// compositor declares dead. Two differences earned by this job being longer
/// and rarer than an extraction.
///
/// The outcome is a `Result<String, String>` rather than the library's own
/// types. `Message` derives Clone and the GUI has no business making
/// `PackReport` and `TransferError` satisfy the message plumbing's bounds, so
/// the worker turns its answer into the sentences the card will show. What the
/// GUI still needs afterwards - where the file or the instance landed - is
/// `target`, which it knew before the thread started.
///
/// And the job OUTLIVES its own completion: `finished` holds what the poll took
/// out of `outcome`, so the same card that showed a progress bar shows the
/// result until the user closes it. An extraction can report through the status
/// bar because it is thirty seconds and its result is the mod appearing; a pack
/// has a path, a size, a ratio and possibly a list of files it could not read,
/// and the welcome screen - where an unpack happens - draws no status bar at all.
pub(crate) struct TransferJob {
    pub(crate) kind: TransferKind,
    /// What the card says is happening.
    pub(crate) title: String,
    /// The archive being written, or the folder being restored into.
    pub(crate) target: PathBuf,
    /// 7-Zip's percentage, already clamped to its own maximum by the worker.
    ///
    /// The raw value is NOT monotonic and a pack makes two passes into one
    /// archive, so the bar would climb, reset and climb again. `fetch_max` in
    /// the worker is the whole fix, and it belongs there rather than in the
    /// view: a bar that walks backwards for minutes reads as a broken program,
    /// and the view is asked to draw sixty times a second.
    pub(crate) percent: std::sync::Arc<std::sync::atomic::AtomicU8>,
    pub(crate) outcome: std::sync::Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    pub(crate) done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// What the worker said, once the poll has taken it. `None` while running.
    pub(crate) finished: Option<Result<String, String>>,
}

impl TransferJob {
    /// Whether this job is still working - the condition for wanting frames.
    pub(crate) fn running(&self) -> bool {
        self.finished.is_none()
    }
}

/// A game/tool launched through Eidos that the GUI is waiting on. A detached
/// thread `wait()`s the `eidos` child (which itself outlives the game, holding the
/// FUSE mount) and flips `done` when it exits; a poll subscription notices and
/// unlocks. `pid` is shown in the lock overlay (MO2 shows the running process).
pub(crate) struct RunningState {
    /// What is running (the tool title or the game name), for the overlay text.
    pub(crate) title: String,
    /// The `eidos` child's pid, surfaced in the overlay like MO2's process list.
    pub(crate) pid: u32,
    /// Flipped to `true` by the wait thread once the child exits.
    pub(crate) done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The child's exit status, stored by the wait thread just before `done`.
    pub(crate) outcome: std::sync::Arc<std::sync::Mutex<Option<std::process::ExitStatus>>>,
    /// The per-run log file capturing the child's stdout+stderr (launch errors
    /// are invisible otherwise - the GUI has no terminal when started from Steam).
    pub(crate) log: Option<PathBuf>,
    /// Whether the lock overlay is up. `false` = "lock GUI" is off (or the user
    /// clicked Unlock): the run is still TRACKED (exit refresh, double-launch
    /// guard, error reporting) but the window stays interactive.
    pub(crate) lock: bool,
}
