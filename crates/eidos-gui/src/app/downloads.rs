//! The Downloads tab's rows, their install state, how the list is ordered, and
//! a download being dragged onto the mod list.

use crate::*;

/// The install status of a downloaded archive, derived from its `.meta` sidecar
/// (MO2's downloads-list state column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadState {
    /// No `.meta` sidecar (a manually dropped archive) - status unknown.
    Untracked,
    /// Arriving now: a `.unfinished` partial that is still growing.
    Downloading,
    /// A `.unfinished` partial that has stopped growing - the `eidos nxm` process
    /// died, the network went, or the user closed the terminal. Not lost: the
    /// partial resumes with a Range request on the next attempt.
    Stalled,
    /// Stopped ON PURPOSE (the Pause button), which is why it is not Stalled: the
    /// two look identical on disk - a partial with no live process - and only the
    /// sidecar's `paused` flag tells them apart. Saying "stalled" for a download
    /// the user paused a second ago reads as a failure.
    Paused,
    /// Downloaded but not yet installed into a mod.
    Ready,
    /// Already installed into a mod.
    Installed,
    /// Was installed then uninstalled (the mod was removed).
    Uninstalled,
}

/// One row of the Downloads manager: a completed archive plus its cached status,
/// so the panel does not re-read every `.meta` sidecar on each redraw.
#[derive(Debug, Clone)]
pub(crate) struct DownloadRow {
    /// The archive's file name.
    pub(crate) name: String,
    /// The absolute path to the archive.
    pub(crate) path: PathBuf,
    /// Size in bytes.
    pub(crate) size: u64,
    /// The installed `version` from the `.meta` sidecar (empty if none).
    pub(crate) version: String,
    /// The friendly mod name from the sidecar, if any (Nexus `modName`).
    pub(crate) mod_name: Option<String>,
    /// The Nexus mod id from the sidecar, so the row can reach the mod's page.
    /// The sidecar's `url=` is NOT usable for this: it is the CDN link, which
    /// carries an expiry and a signature and is dead within the hour.
    pub(crate) mod_id: Option<u64>,
    /// The derived install status.
    pub(crate) state: DownloadState,
    /// Bytes on disk so far. Equals `size` once finished.
    pub(crate) downloaded: u64,
    /// Total bytes from the sidecar's `totalSize`, `0` when unknown - an older
    /// download, or a manually dropped archive.
    pub(crate) total: u64,
    /// Bytes per second, measured between two ticks. `None` on the first tick of
    /// a download, when there is nothing to compare against yet.
    pub(crate) speed: Option<f64>,
    /// MO2's `removed=` in the sidecar: hidden from the list without the archive
    /// being deleted. The field was modelled and nothing read it.
    pub(crate) hidden: bool,
    /// When the archive was last written, for sorting by date.
    pub(crate) modified: std::time::SystemTime,
}

/// How the Downloads list is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DownloadSort {
    /// Newest first. What the list always did, and still the default: the
    /// archive somebody wants is nearly always the one that just arrived.
    #[default]
    Newest,
    Name,
    Size,
    /// Groups by install state, so "everything not installed yet" is one run.
    State,
}

impl DownloadSort {
    pub(crate) const ALL: [DownloadSort; 4] = [
        DownloadSort::Newest,
        DownloadSort::Name,
        DownloadSort::Size,
        DownloadSort::State,
    ];
    pub(crate) fn label(self) -> &'static str {
        match self {
            DownloadSort::Newest => "Newest",
            DownloadSort::Name => "Name",
            DownloadSort::Size => "Size",
            DownloadSort::State => "State",
        }
    }
}

impl std::fmt::Display for DownloadSort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A download row being dragged onto the mod list, to install it AT a priority.
///
/// Deliberately not folded into [`DragState`]: that one moves a row that is
/// already in the list, so it has a `from` and its own edges are no-ops. This one
/// has no row in the list yet, so every gap is a genuine target and the commit
/// runs the installer rather than a reorder.
#[derive(Debug, Clone)]
pub(crate) struct DownloadDrag {
    /// The archive to install.
    pub(crate) path: PathBuf,
    /// Where it should land, as an INSERTION index into `app.mods`.
    pub(crate) gap: usize,
    /// Whether the pointer ever reached an insertion strip. A press arms the
    /// drag, so a plain click on a download row arrives here as a drop.
    pub(crate) aimed: bool,
}

/// How long a `.unfinished` partial may go without growing before it is called
/// stalled rather than downloading. Generous: a slow mirror can go quiet for a
/// few seconds, and calling a live download dead is worse than the reverse.
pub(crate) const STALLED_AFTER: std::time::Duration = std::time::Duration::from_secs(20);
