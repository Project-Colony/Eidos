//! The mod and plugin lists' own state: their columns, grouping and sorting, a
//! drag in progress and the auto-scroll it drives, and the plugin sort result.

use crate::*;

/// A column of the mod list.
///
/// Priority and Name are not here: they are structural rather than optional.
/// Priority IS the load order, which is the thing the list exists to show, and a
/// list of nameless rows is not a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ModColumn {
    Category,
    Content,
    Version,
    /// MO2's Author column. Blank until a mod has been downloaded or
    /// update-checked, which is where the name comes from.
    Author,
    /// When the mod folder was last written.
    Installed,
    /// The Nexus mod id, for finding a mod's page by hand.
    ModId,
    /// The game the mod says it was downloaded for.
    Game,
    Flags,
}

impl ModColumn {
    /// In display order, left to right. This is also the order the settings
    /// file stores, so the toggles read the way the header does.
    pub(crate) const ALL: [ModColumn; 8] = [
        ModColumn::Category,
        ModColumn::Content,
        ModColumn::Version,
        ModColumn::Author,
        ModColumn::Installed,
        ModColumn::ModId,
        ModColumn::Game,
        ModColumn::Flags,
    ];
    /// What the header says.
    pub(crate) fn title(self) -> &'static str {
        match self {
            ModColumn::Category => "Category",
            ModColumn::Content => "Content",
            ModColumn::Version => "Version",
            ModColumn::Author => "Author",
            ModColumn::Installed => "Installed",
            ModColumn::ModId => "Nexus id",
            ModColumn::Game => "Game",
            ModColumn::Flags => "Flags",
        }
    }
    /// The key in `settings.ini`. Stable, and separate from the title so a
    /// heading can be reworded without silently resetting everybody's columns.
    pub(crate) fn key(self) -> &'static str {
        match self {
            ModColumn::Category => "category",
            ModColumn::Content => "content",
            ModColumn::Version => "version",
            ModColumn::Author => "author",
            ModColumn::Installed => "installed",
            ModColumn::ModId => "modid",
            ModColumn::Game => "game",
            ModColumn::Flags => "flags",
        }
    }
    /// How wide the cell is.
    pub(crate) fn width(self) -> f32 {
        match self {
            ModColumn::Category => 96.0,
            ModColumn::Content => 60.0,
            ModColumn::Version => 64.0,
            ModColumn::Author => 96.0,
            ModColumn::Installed => 86.0,
            ModColumn::ModId => 60.0,
            ModColumn::Game => 84.0,
            ModColumn::Flags => 46.0,
        }
    }
}

/// What the mod list is grouped under, when it is not grouped by separators.
///
/// `None` - the default - means the user's own separators, which are the
/// groups they wrote themselves and the only ones that survive a reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupBy {
    /// The mod's primary category, resolved to its name.
    Category,
    /// Whether it came from Nexus at all - the split that decides which mods
    /// an update check can even speak about.
    Source,
}

impl GroupBy {
    pub(crate) const ALL: [GroupBy; 2] = [GroupBy::Category, GroupBy::Source];
    pub(crate) fn label(self) -> &'static str {
        match self {
            GroupBy::Category => "Group by category",
            GroupBy::Source => "Group by source",
        }
    }
}

/// What the mod list is ordered by, when it is not in load order.
///
/// `None` - the default - is the real order: priority, which is what the list is
/// FOR. Any other ordering is a view of it, and dragging is disabled while one
/// is on, exactly as MO2 does: a drop in a sorted list has no meaning to give
/// the row it lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModSort {
    pub(crate) by: SortKey,
    pub(crate) ascending: bool,
}

/// What a sort orders by. `Name` and `Priority` are here even though they are
/// not optional columns - they are the two most useful things to sort on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortKey {
    Name,
    Column(ModColumn),
}

/// Which end of the list an auto-scroll is heading for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollEdge {
    Up,
    Down,
}

/// How far one auto-scroll tick moves, in pixels, at the inner lip of the band
/// and at the outer edge of the list.
///
/// Pixels, and applied with `scroll_by`, which is RELATIVE. The first version of
/// this mirrored the scroll offset in a field and wrote it back with `snap_to`,
/// which is absolute: any staleness in the mirror became a jump to wherever the
/// stale value pointed, usually the very top. There is no mirror now, so there
/// is nothing to go stale.
///
/// The range is what makes the band aimable: one row a tick lets you creep to
/// the row just off screen, and pushing to the edge crosses a long list without
/// waiting. A single speed can do one or the other, never both.
pub(crate) const DRAG_SCROLL_SLOW_PX: f32 = 8.0;

pub(crate) const DRAG_SCROLL_FAST_PX: f32 = 75.0;

/// How tall the auto-scroll bands are at each end of the list. They sit OVER the
/// list while a drag is under way, so every pixel of them is an insertion point
/// that cannot be aimed at: deep enough to hit without aiming, no deeper.
pub(crate) const DRAG_SCROLL_BAND: f32 = 28.0;

/// An in-flight mod-row drag (MO2's drag-to-reorder). `from` is the grabbed row's
/// index in `app.mods`; the move is only applied on release, and only when the
/// aimed gap is not one of the block's own edges.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DragState {
    pub(crate) from: usize,
    /// Whether the pointer ever reached an insertion point outside the block.
    /// A press arms a drag, so a plain CLICK arrives as a drop; with a
    /// multi-row selection there is no "own edge" to recognise, and committing
    /// it would COMPACT a non-contiguous selection and save that. See
    /// `PluginDrag::aimed`.
    pub(crate) aimed: bool,
    /// Where the block would land, as an INSERTION index, not a row index: `gap`
    /// means "before the row currently at `gap`", and `mods.len()` means the end.
    ///
    /// This distinction is the whole fix. Targeting a ROW is ambiguous - dropping
    /// "on" a mod could mean above or below it, and the answer used to depend on
    /// which way you came from, so the drop could not be aimed. An insertion
    /// index has exactly one meaning, which is also what `move_block` already
    /// expects, and it is what the indicator line draws between the two rows.
    /// MO2 makes the same distinction (`DropPosition::AboveItem/BelowItem`,
    /// modlistview.cpp:1394).
    pub(crate) gap: usize,
}

/// What comes back from a LOOT run: the fingerprint of the list it was asked
/// about, the sorted names, and the report - or why the run failed. The report
/// is a nested `Result` because it is advisory: losing it must not lose the
/// order that was successfully computed.
pub(crate) type SortOutcome = Result<
    (
        SortFingerprint,
        Vec<String>,
        Result<eidos_loot::LootReport, String>,
    ),
    String,
>;

/// The instance, profile and input generation a LOOT result belongs to.
/// Names alone do not detect activation changes or replacement of winning files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SortFingerprint {
    /// The game, not just the profile. Profiles are per-game and their names
    /// collide - two games both have a "Default" - so a switch mid-sort would
    /// compare equal and write one game's load order into the other's.
    pub(crate) game: String,
    pub(crate) profile: String,
    pub(crate) names: BTreeSet<String>,
    pub(crate) instance: Option<PathBuf>,
    pub(crate) epoch: u64,
}

/// An in-flight plugin drag. Unlike the mod list, where any order is legal, a
/// plugin's position is constrained by the engine, so the drag carries the range
/// it is allowed to land in - computed ONCE when the row is grabbed, not per
/// frame - and the strips outside it are not offered at all.
#[derive(Debug, Clone)]
pub(crate) struct PluginDrag {
    pub(crate) from: usize,
    /// Insertion index, same meaning as `DragState::gap`.
    pub(crate) gap: usize,
    /// Every row travelling, ascending - the whole selection when the grabbed
    /// row belonged to it, otherwise just that row.
    pub(crate) block: Vec<usize>,
    /// Whether the pointer ever reached an insertion point outside the block.
    ///
    /// A press arms a drag, so a plain CLICK on a row arrives here as a drop.
    /// With a single row the "landed on its own edge" test caught that, but a
    /// non-contiguous selection has no such edge: dropping it anywhere COMPACTS
    /// it, so a click on one of its rows silently rewrote the load order and
    /// saved it. Nothing commits until this is true.
    pub(crate) aimed: bool,
    /// Where this plugin may legally go, and which plugins bound it.
    pub(crate) range: MovableRange,
}
