//! A Nexus collection being browsed and installed: its members, and how each
//! one joins against what the instance already has.

use crate::*;

/// One member of a collection, joined against what the instance already has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberState {
    /// The exact Nexus source file is recorded in an installed mod.
    Installed,
    /// A matching mod lacks enough source metadata to verify the requested file.
    Unverified,
    /// The mod is installed, at a DIFFERENT version.
    ///
    /// Its own state because it is its own situation: the collection will not
    /// play as its author built it, and "installed" said otherwise. A user with
    /// an outdated copy of every member used to be told the whole collection was
    /// already installed.
    OtherVersion,
    /// The archive is in `downloads/`, whole, ready to install.
    Downloaded,
    /// Neither. This is what the collection is asking you to get.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CollectionTarget {
    pub(crate) instance: PathBuf,
    pub(crate) profile: String,
    pub(crate) installation: String,
}

/// Which half of the collection pane is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CollectionPane {
    /// Search the open game's published collections.
    Browse,
    /// Paste a link, and the collection it names.
    #[default]
    Link,
}

/// The Browse pane: one search page of the open game's collections.
#[derive(Default)]
pub(crate) struct CollectionBrowse {
    pub(crate) query: String,
    pub(crate) sort: eidos_nexus::collections::CollectionSort,
    /// True while the one GraphQL request is in flight.
    pub(crate) loading: bool,
    /// An error is the line drawn as it is: a local guard, or Nexus's failure
    /// already worded as `collection_browse_failed`.
    pub(crate) result: Option<Result<eidos_nexus::collections::CollectionSearch, String>>,
}

/// The link of a collection's latest revision, what View hands to the Link pane.
pub(crate) fn browse_link(domain: &str, slug: &str) -> String {
    format!("nxm://{domain}/collections/{slug}/revisions/latest")
}

/// The collection browser.
#[derive(Default)]
pub(crate) struct CollectionState {
    pub(crate) pane: CollectionPane,
    pub(crate) browse: CollectionBrowse,
    pub(crate) install_check: Option<u64>,
    pub(crate) runtime: Option<(CollectionTarget, eidos_collections::recipe::RuntimeCheck)>,
    /// What the user pasted, kept so the field survives a failed fetch.
    pub(crate) link: String,
    /// The fetched revision, once it arrives.
    pub(crate) revision: Option<eidos_nexus::collections::CollectionRevision>,
    /// Per member, in the revision's order. Computed locally - no requests.
    pub(crate) states: Vec<MemberState>,
    /// True while the one GraphQL request is in flight.
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    /// Two clicks before "fetch missing" spawns anything. Same idiom as every
    /// other bulk action here, and this one starts real transfers.
    pub(crate) confirm_fetch: bool,
    /// File ids this pane has already spawned a transfer for. Batches are taken
    /// from what is NOT in here, so clicking again advances instead of
    /// restarting the same few - a member stays `missing` for as long as its
    /// download runs, so the state alone cannot tell the two apart.
    pub(crate) asked: std::collections::HashSet<u64>,
    /// A free account's walk through the missing files, while one runs.
    pub(crate) walk: Option<CollectionWalk>,
}

/// A free account fetching a collection's missing files one page at a time.
/// Nexus wants one click per file there, so Eidos opens each page and watches
/// `downloads/` for the file to land.
pub(crate) struct CollectionWalk {
    /// Member indices still to fetch, in collection order.
    pub(crate) queue: Vec<usize>,
    /// Position in `queue`.
    pub(crate) at: usize,
    /// Download names already there, so only a new arrival can be a wrong file.
    pub(crate) seen: std::collections::HashSet<String>,
}

/// What the walk does after a look at `downloads/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkStep {
    Wait,
    /// Open this member's file page.
    Open(usize),
    /// Another file of the awaited member's mod arrived.
    Wrong {
        want: u64,
        got: u64,
    },
    Done,
}
