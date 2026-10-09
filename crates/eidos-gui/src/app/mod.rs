//! The window's state: `App`, the screens and tabs it can show, and the types
//! its fields are made of.
//!
//! Only data lives here. `state` holds the helpers that read and change it,
//! `update` decides what each message does to it, and the drawing modules read
//! it.

use crate::*;

mod collections;
mod dialog_state;
mod downloads;
mod jobs;
mod lists;

pub(crate) use collections::*;
pub(crate) use dialog_state::*;
pub(crate) use downloads::*;
pub(crate) use jobs::*;
pub(crate) use lists::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    Welcome,
    Kind,
    Game,
    NameLoc,
    Summary,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Data,
    Plugins,
    Conflicts,
    Overwrite,
    /// The BSA/BA2 archives the enabled mods ship, and whether each one loads.
    Archives,
    Saves,
    Downloads,
    /// Live health checks for this setup (MO2's problems/diagnostics panel, plus
    /// the Linux-specific ones MO2 never needed).
    Diagnostics,
}

/// Tabs of the per-mod information dialog (MO2's modinfodialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InfoTab {
    General,
    Conflicts,
    Filetree,
    /// The mod's `INI Tweaks/` fragments, individually enabled.
    IniTweaks,
    Notes,
}

/// A keyboard navigation intent, independent of which list will answer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Nav {
    Up,
    Down,
    /// A page is ten rows: enough to be worth a key, small enough that the row
    /// you land on is still somewhere you were looking.
    PageUp,
    PageDown,
    First,
    Last,
    /// Space: flip the enabled state of the focused row, or of the whole
    /// selection when there is one.
    Toggle,
    /// Enter: open what the row is about (the mod information dialog).
    Activate,
    /// Delete: arm removal of the focused mod. Never destructive on its own -
    /// it opens the same two-step confirmation the context menu uses.
    Remove,
    /// Ctrl+Up / Ctrl+Down: MOVE the focused row (or the whole selection) one
    /// place, rather than moving the focus. This is what the per-row arrow
    /// buttons used to be, minus a column on every line.
    ShiftUp,
    ShiftDown,
}

/// Which list the keyboard is driving.
///
/// The mod list and the tab panel sit side by side, both visible, so "the
/// selected row" is ambiguous without this. It follows the last row the user
/// pressed, which is what a pointer user expects, and Tab moves it explicitly
/// for someone who never reaches for the mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pane {
    Mods,
    Plugins,
}

/// One Data-tab row: entry name, the layer providing it, and whether it is a
/// folder (the merged view as the FUSE union would serve it).
/// One row of the merged Data tree.
#[derive(Debug, Clone)]
pub(crate) struct DataRow {
    /// The file or folder name as the merged view serves it.
    pub(crate) name: String,
    /// What provides it: a mod name, `[Overwrite]`, or the game.
    pub(crate) source: String,
    pub(crate) is_dir: bool,
    /// The real path on disk behind the name - what Reveal and Open act on.
    pub(crate) real: PathBuf,
    /// Size and mtime of the winner. `None` for anything that cannot be stat'd.
    pub(crate) size: Option<u64>,
    pub(crate) mtime: Option<std::time::SystemTime>,
    /// Whether more than one mod provides this path (so a row can be filtered
    /// down to just the contested ones, which is what the tab is FOR).
    pub(crate) conflicted: bool,
}

/// One memoised recursive listing: the view generation it was built at, and the
/// entries behind an `Rc` so a cache HIT hands out a pointer bump instead of
/// cloning ~5k Strings per redraw (which is what it did, and what made the
/// "cache" allocate proportionally to its own payload).
pub(crate) type CachedListing = (u64, std::rc::Rc<Vec<String>>);

/// The same for one directory of the Data tab's merged view.
pub(crate) type CachedDataListing = (u64, std::rc::Rc<Vec<DataRow>>);

/// The Data tab's union and its layer labels, with the generation they were
/// built at (see [`view::data_stack`]).
pub(crate) type CachedDataStack = (
    u64,
    std::rc::Rc<eidos_core::LayerStack>,
    std::rc::Rc<view::DataSources>,
);

pub(crate) struct App {
    pub(crate) installer: Option<installers::Wizard>,
    pub(crate) mod_picker: u64,
    pub(crate) screen: Screen,
    pub(crate) games: Vec<DetectedGame>,
    pub(crate) kind: InstanceKind,
    pub(crate) portable_path: String,
    pub(crate) selected: Option<usize>,
    pub(crate) name: String,
    pub(crate) created: Option<Instance>,
    pub(crate) error: Option<String>,
    pub(crate) mods: Vec<ModEntry>,
    /// A content hash of `modlist.txt` as it was when `mods` was last read from
    /// or written to it; `None` until then. Another Eidos process (the CLI, a
    /// second window Steam or a collection link opened) can rewrite the file
    /// while this window shows its old copy, and saving that copy would erase
    /// what it wrote, so `save_mods` refuses when the file no longer matches.
    pub(crate) modlist_seen: std::cell::Cell<Option<u64>>,
    /// Cached ESP/ESM load order for the Plugins tab (recomputed on demand).
    pub(crate) plugins: Option<PluginList>,
    /// The same guard for `plugins`: a hash of the profile's plugin state files
    /// as `compute_plugins` read them or `write_plugin_state` last wrote them.
    pub(crate) plugins_seen: std::cell::Cell<Option<u64>>,
    /// Cached per-file conflict analysis for the Conflicts tab + mod-row flags.
    pub(crate) conflicts: Option<ConflictMap>,
    pub(crate) archive_epoch: std::cell::Cell<u64>,
    pub(crate) archive_completed_epoch: Option<u64>,
    pub(crate) archive_sources: HashMap<PathBuf, eidos_conflicts::ArchiveIdentity>,
    pub(crate) archive_job: Option<ArchiveWorker>,
    pub(crate) archive_plan: Option<eidos_gamefeatures::archives::ArchivePlan>,
    pub(crate) archive_warnings: Vec<String>,
    /// The last health-check run, cached.
    ///
    /// `diagnostics()` walks the mods directory, reads the script extender's log
    /// and parses an INI. It used to be called from `view()` - twice when the
    /// Diagnostics tab was open, since the tab LABEL carries the problem count -
    /// which means it ran on every single frame: roughly a hundred `read_dir`
    /// per keystroke in the filter box. That is the cost that made typing feel
    /// like wading, and it bought a number that only changes when the setup does.
    pub(crate) diag: Vec<Diagnostic>,
    /// Set when something might have changed the answer; consumed at the end of
    /// `update()`. A missed setter shows a stale COUNT until the next real
    /// change, never wrong data - the panel renders from this same cache, so the
    /// label and the panel can never disagree either.
    pub(crate) diag_dirty: bool,
    /// The `&App`-reachable half of the same flag, so `bump_views` can set it.
    pub(crate) diag_stale: std::cell::Cell<bool>,
    pub(crate) tab: Tab,
    pub(crate) status: Option<String>,
    /// Two-click guard for the destructive "Clear Overwrite" action.
    pub(crate) confirm_clear: bool,
    /// The in-progress "create mod from Overwrite" name, if that prompt is open.
    pub(crate) overwrite_to_mod: Option<String>,
    /// An open "send to priority" editor: `(row, typed text)`.
    pub(crate) send_priority: Option<(usize, String)>,
    /// An open "send to separator" chooser, for this row.
    pub(crate) send_separator: Option<usize>,
    /// The Proton command Steam passed via `%command%` (empty if launched
    /// standalone). The Run button launches the game through this.
    pub(crate) launch_command: Vec<String>,
    /// An open FOMOD installer wizard, if the user is mid-install.
    pub(crate) fomod: Option<FomodWizard>,
    /// An open install-collision prompt (target mod name already exists).
    pub(crate) collision: Option<CollisionPrompt>,
    /// An open manual / BAIN install picker.
    pub(crate) picker: Option<InstallPicker>,
    /// Tools runnable through the merged view (user tools.ini + per-game
    /// defaults), shown in the run-target picker next to Run.
    pub(crate) tools: Vec<eidos_instance::Tool>,
    /// The picked run target: `None` = the game, `Some(title)` = that tool.
    pub(crate) tool_choice: Option<String>,
    /// Mod-list filter query (case-insensitive substring on the mod name).
    pub(crate) search: String,
    /// The highlighted mod row, if any.
    pub(crate) selected_mod: Option<usize>,
    /// The mod whose right-click action menu is open (None = closed).
    pub(crate) menu_mod: Option<usize>,
    /// The plugin row whose right-click menu is open (MO2's plugin context
    /// menu). Separate from `menu_mod`: the two lists are shown at once, and a
    /// shared field would make right-clicking one dismiss nothing in the other.
    pub(crate) menu_plugin: Option<usize>,
    /// In-progress rename: `(mod index, edited name)`.
    pub(crate) rename: Option<(usize, String)>,
    /// Per-mod metadata for the extra columns + context menu, keyed by folder name.
    pub(crate) meta_cache: HashMap<String, RowMeta>,
    /// Two-click guard for the destructive per-mod "Remove" action.
    pub(crate) confirm_remove: Option<usize>,
    /// The mod whose info dialog is open (None = closed), its active tab, and the
    /// note text being edited.
    pub(crate) info_mod: Option<usize>,
    /// Generated receipts are read once when the mod information dialog opens.
    pub(crate) info_provenance: String,
    pub(crate) info_tab: InfoTab,
    pub(crate) notes_edit: String,
    /// Collapsed separators, keyed by display name (MO2 keys by display name too).
    /// Persisted per-profile so the grouping state survives a relaunch.
    pub(crate) collapsed: HashSet<String>,
    /// Active category filter (a top-level category id), or `None` for all.
    pub(crate) category_filter: Option<i32>,
    /// The state criteria the mod list is filtering on (MO2's filter pane).
    pub(crate) filters: ModFilters,
    /// Whether that pane is open.
    pub(crate) filters_open: bool,
    // ---- Settings / Nexus account (the status bar + endorse/update read these) ----
    /// The Preferences modal is open.
    pub(crate) settings_open: bool,
    /// The active Preferences tab.
    pub(crate) settings_tab: SettingsTab,
    /// Which collapsible sections of the Settings screen are open. Keyed by the
    /// same `&'static str` the section is built with, so a rename cannot drift.
    pub(crate) settings_expanded: HashSet<&'static str>,
    /// The validated Nexus account, if a stored session checked out.
    pub(crate) nexus_account: Option<eidos_nexus::Account>,
    /// A sign-in is in flight (guards the button + concurrent attempts).
    pub(crate) nexus_signing_in: bool,
    /// The last sign-in error, shown inline in the dialog.
    pub(crate) nexus_error: Option<String>,
    /// The persisted app-global preferences (theme, default game).
    pub(crate) prefs: Settings,
    // ---- Executables dialog ----
    /// The open Executables editor, if any (None = closed).
    pub(crate) executables: Option<ExecutablesDialogState>,
    /// The Backups dialog: the restore points of both lists, read when it opens
    /// so the list cannot go stale behind an open dialog.
    pub(crate) backups: Option<BackupsDialogState>,
    /// Files dropped from a file manager, waiting to be installed. A drop of
    /// several archives arrives as several messages, and each install can open a
    /// modal, so they are drained one at a time rather than handled inline.
    pub(crate) dropped: Vec<PathBuf>,
    /// Whether a file is currently hovering over the window (for the hint).
    pub(crate) files_hovering: bool,
    /// A download being dragged onto the mod list (MO2's drop-to-priority).
    pub(crate) download_drag: Option<DownloadDrag>,
    /// Where the install now in flight should land, if it was aimed at a gap,
    /// paired with the ARCHIVE it was aimed at.
    ///
    /// The archive is what makes it safe. This has to survive the FOMOD wizard,
    /// the BAIN picker and the collision prompt, so it cannot live on the drag -
    /// and an install that ends without reaching `after_install` (an extraction
    /// failure, an unrecognised layout, a dismissed dialog) would otherwise
    /// leave the aim behind for the NEXT mod installed to silently adopt.
    /// Matching on the archive means a stale aim simply never applies.
    pub(crate) install_at: Option<(usize, PathBuf)>,
    /// A landing position chosen from a context menu, waiting for the picker to
    /// name an archive. Separate from `install_at` because that one is PAIRED
    /// with its archive - the pairing is what stops a cancelled install moving an
    /// unrelated mod - and there is no archive yet at the moment of the click.
    pub(crate) install_gap: Option<usize>,
    /// The archive currently being extracted on a worker thread, if any.
    pub(crate) install_job: Option<InstallJob>,
    /// Two-click guard for the bulk enable/disable, holding the TARGET state so
    /// arming "Enable all" and then clicking "Disable all" does not fire.
    pub(crate) confirm_set_all: Option<bool>,
    /// Whether the File dropdown (the folder list) is showing.
    pub(crate) file_menu_open: bool,
    /// Where the File / View / Filters buttons actually are, measured by iced
    /// rather than guessed at: a dropdown hangs from the rectangle of the thing
    /// that opened it. `None` until the first measurement comes back, and after
    /// that the last known place - which is right, because none of the three
    /// buttons moves while its menu is open.
    pub(crate) file_menu_at: Option<iced::Rectangle>,
    pub(crate) view_menu_at: Option<iced::Rectangle>,
    pub(crate) filters_at: Option<iced::Rectangle>,
    /// The open Export dialog: which rows, and which columns are ticked.
    pub(crate) export: Option<ExportDialogState>,
    /// The open Pack dialog (the instance being written to one file).
    pub(crate) pack: Option<PackDialogState>,
    /// The open Unpack dialog. Deliberately NOT gated on an open instance.
    pub(crate) unpack: Option<UnpackDialogState>,
    /// The pack or unpack on a worker thread, and afterwards what it said.
    ///
    /// ONE field for both directions rather than two: they are both minutes of
    /// 7-Zip on the same disk, running them at once would only make each slower,
    /// and a single slot makes that impossible rather than merely discouraged.
    pub(crate) transfer_job: Option<TransferJob>,
    /// The open collection view, if any.
    pub(crate) collection: Option<CollectionState>,
    /// Whether the instance manager is showing.
    pub(crate) instances_open: bool,
    /// Where the instance registry lives. A field rather than a global so the
    /// window can be tested without writing the real user config - the handlers
    /// that forget and rename instances persist through it.
    pub(crate) registry_path: PathBuf,
    /// What is typed in the preferred-servers field, which is not the saved
    /// value until it is submitted - the same shape as the mod URL field.
    pub(crate) servers_edit: String,
    /// What is typed in the tools-folder box, saved on submit like the others.
    pub(crate) tools_dir_edit: String,
    /// Filetree: the entry being renamed and what has been typed, the entry
    /// armed for deletion, and the new-folder box.
    /// The mod NAME and the entry being renamed. By name for the same reason
    /// the delete is: an index outlives nothing, and a reload between opening
    /// the box and pressing Enter would aim the rename into another mod.
    pub(crate) tree_rename: Option<(String, String)>,
    pub(crate) tree_rename_text: String,
    /// The filetree entry armed for deletion: the MOD'S NAME and the relative
    /// path. By name for the same reason as `confirm_restore` - a reload
    /// between the clicks would otherwise aim the delete into another mod's
    /// folder, where the same relative path may well exist.
    pub(crate) tree_delete_armed: Option<(String, String)>,
    /// The mod the new-folder box belongs to, and what has been typed.
    pub(crate) tree_new_folder: Option<(String, String)>,
    /// The backup armed for restoring over its original, BY NAME.
    ///
    /// Not by index: an index is a position in a list that anything can reload
    /// between the two clicks, and the second click would then restore a
    /// different backup - one that is still a backup, so no guard catches it.
    pub(crate) confirm_restore: Option<String>,
    /// The file being previewed, and what could be made of it.
    pub(crate) preview: Option<Preview>,
    pub(crate) preview_pending: Option<file_preview::Pending>,
    pub(crate) extension_picker: u64,
    pub(crate) archive_filter: String,
    pub(crate) archive_page: usize,
    pub(crate) archive_export: Option<archive_conflicts::ExportRequest>,
    /// Downloads list: the name filter, the ordering, whether hidden rows are
    /// shown, and the two-click guard on the bulk purge.
    pub(crate) dl_filter: String,
    pub(crate) dl_sort: DownloadSort,
    pub(crate) dl_show_hidden: bool,
    pub(crate) confirm_purge_installed: bool,
    /// Mod-list columns currently drawn, in display order.
    pub(crate) mod_columns: Vec<ModColumn>,
    /// What the list is ordered by. `None` is load order - the real one.
    pub(crate) mod_sort: Option<ModSort>,
    /// What the list is grouped under. `None` is the user's own separators.
    pub(crate) group_by: Option<GroupBy>,
    /// Group headers the user has folded, by their synthetic label. Separate
    /// from `collapsed`, which keys on separator names: a category called
    /// "Armour" and a separator called "Armour" are not the same fold.
    pub(crate) groups_collapsed: std::collections::HashSet<String>,
    /// The instance row being renamed, and the pending name.
    pub(crate) instance_rename: Option<(usize, String)>,
    /// Two-click guard for forgetting an instance.
    pub(crate) confirm_forget: Option<usize>,
    /// Two-click guard for the Overwrite sync.
    pub(crate) confirm_sync: bool,
    /// The custom-URL editor in the mod info dialog.
    pub(crate) url_edit: String,
    /// Saves picked with Ctrl+click, for the batch actions.
    pub(crate) selected_saves: std::collections::BTreeSet<usize>,
    /// Two-click guard for deleting the selection.
    pub(crate) confirm_saves_delete: bool,
    /// The saves directory's shape at the last tick: how many entries and the
    /// newest mtime. Compared rather than re-listed into the model, so a quiet
    /// directory costs one `read_dir` and changes nothing the view depends on.
    pub(crate) saves_fingerprint: Option<(usize, std::time::SystemTime)>,
    /// The selected save's screenshot, decoded once on selection.
    ///
    /// Keyed by path so a stale image cannot be shown against another save, and
    /// held as a ready `image::Handle` rather than raw pixels - `view` runs every
    /// frame and must not decode anything.
    pub(crate) save_shot: Option<(PathBuf, Option<iced::widget::image::Handle>)>,
    /// The plugin row whose "Send to priority" field is open, and what is typed
    /// in it. Mirrors `send_priority` for mods; kept separate because the two
    /// lists are indexed independently and one menu must not aim the other.
    pub(crate) plugin_send_priority: Option<(usize, String)>,
    /// Something the drop wants said once the install finishes. It cannot say it
    /// itself: the installer sets its own status a moment later.
    pub(crate) pending_note: Option<String>,
    /// The Categories dialog: which mods it applies to and the pending choice.
    pub(crate) categories_dialog: Option<CategoriesDialogState>,
    /// The open INI editor, if any.
    pub(crate) ini_editor: Option<IniEditorState>,
    /// The open log pane, if any.
    pub(crate) log_pane: Option<LogPaneState>,
    /// User add-ons, read at startup and on demand.
    pub(crate) addons: Vec<eidos_addons::Addon>,
    /// Whether the Extensions list is showing.
    pub(crate) addons_open: bool,
    /// What the `diagnose` add-ons reported on the last refresh, by add-on name.
    pub(crate) addon_findings: Vec<(String, eidos_addons::Finding)>,
    /// Checks that failed, and why. Not retried until Reload: this runs on every
    /// diagnostics refresh, so retrying a hanging one would cost its timeout per
    /// click.
    pub(crate) addon_failed: HashMap<String, String>,
    /// Manifests that could not be parsed, and why - so a typo is visible in the
    /// Extensions list instead of only on a stderr the window never shows.
    pub(crate) addon_rejected: Vec<(PathBuf, String)>,
    // ---- Endorse / update in-flight + counts ----
    /// The mod index whose Nexus endorse is in flight (greys the toolbar button).
    pub(crate) endorsing: Option<usize>,
    /// Enabled mods that are endorsed (recomputed in `mods_changed`).
    pub(crate) endorsed_count: usize,
    /// Enabled mods with a Nexus update available (recomputed in `mods_changed`).
    pub(crate) updated_count: usize,
    /// What Nexus last said was left of the request budget.
    ///
    /// `None` until something has actually asked: a number invented before the
    /// first call would be a guess, and the whole point of showing it is that it
    /// is the server's own answer.
    ///
    /// Fed by the update check, which is the operation that can actually empty
    /// the bucket - it issues one request per mod, and is the only thing here
    /// that ever hits the hourly ceiling. The one-off calls (endorse, track,
    /// identify) spend a single request each and would need their result types
    /// widened to report it, which is a lot of plumbing for a number that moves
    /// by one.
    pub(crate) nexus_hourly_left: Option<i64>,
    pub(crate) nexus_daily_left: Option<i64>,
    /// A Nexus mod-update check is in flight (guards the Update button).
    pub(crate) update_in_progress: bool,
    /// A LOOT sort is in flight. iced runs on smol's single-threaded executor
    /// here, so a second sort does not race the first - it QUEUES behind it, and
    /// every queued completion re-opens the report modal and overwrites the
    /// status with its own (idempotent, so "nothing moved") result. A masterlist
    /// download is several seconds with no other visible sign of work, which is
    /// long enough to invite exactly that.
    pub(crate) sorting: bool,
    // ---- menu-bar UI toggles + About ----
    /// The toolbar / status bar are visible (View menu toggles).
    pub(crate) ui_toolbar_visible: bool,
    /// Fraction of the window width given to the mod list, 0.15 to 0.85.
    /// Mirrors `prefs.split`; kept here because it changes on every pointer
    /// move during a drag and the preferences are only written when it stops.
    pub(crate) split: f32,
    /// Whether the divider is being dragged right now.
    pub(crate) split_drag: bool,
    // ---- motion ----
    /// Whether this window animates at all (Preferences -> Appearance -> Motion,
    /// mirroring `prefs.motion`). Off means every animated value is drawn at its
    /// destination and the frame timer is never subscribed.
    pub(crate) motion: bool,
    /// The main tab strip's crossfade, and the tab it is fading AWAY from.
    ///
    /// The previous tab is kept because a crossfade needs both ends: without it
    /// the arriving tab fades in while the leaving one snaps, which reads as a
    /// glitch rather than as a transition.
    pub(crate) tab_anim: anim::Phase,
    pub(crate) tab_prev: Option<Tab>,
    /// The same for the mod-information strip on the right.
    pub(crate) info_anim: anim::Phase,
    pub(crate) info_prev: Option<InfoTab>,
    /// The status line's fade-in, and the text it is currently showing.
    ///
    /// The copy is what makes the fade fire at all: `status` is assigned from a
    /// dozen places across `state.rs`, and instrumenting each of them is a rule
    /// somebody would forget. Comparing after every message cannot be forgotten.
    pub(crate) status_anim: anim::Phase,
    pub(crate) status_shown: Option<String>,
    pub(crate) ui_statusbar_visible: bool,
    /// The View dropdown is open (iced has no native menu, so it's a floating card).
    pub(crate) view_menu_open: bool,
    /// The About box is open.
    pub(crate) about_open: bool,
    // ---- Saves tab (the details pane is the reason for the parse) ----
    /// The active profile's save files (newest first), lazily loaded.
    pub(crate) saves: Vec<SaveEntry>,
    /// Two-click guard for a save deletion (the save's index in `saves`).
    pub(crate) confirm_delete_save: Option<usize>,
    /// The save whose details pane is open, an index into `saves`.
    pub(crate) selected_save: Option<usize>,
    /// The parsed header of `selected_save`, keyed by its path so a stale parse is
    /// never shown against a different file. `Err` = unreadable, which degrades the
    /// pane to a message rather than hiding the save.
    pub(crate) save_info: Option<(PathBuf, Result<eidos_gamefeatures::SaveInfo, String>)>,
    /// The selected save's plugins that are no longer active, with the mods that
    /// could supply them. Recomputed with `save_info`.
    pub(crate) save_missing: Vec<eidos_gamefeatures::MissingPlugin>,
    // ---- Downloads manager ----
    /// The completed downloads (cached so the panel does not re-scan on redraw).
    pub(crate) downloads: Vec<DownloadRow>,
    /// A resume in flight: which download, the child, and the file its output
    /// went to. Held so the child can be REAPED (an unwaited one becomes a
    /// zombie, whose /proc entry makes it look alive to `stop_download`) and so
    /// a failure can be reported instead of the row silently going back to
    /// Stalled.
    pub(crate) resuming: Option<(String, std::process::Child, PathBuf)>,
    /// Two-click guard for a download deletion (the row's index in `downloads`).
    pub(crate) confirm_delete_download: Option<String>,
    /// The download whose MD5 lookup is in flight, so the row can say so and a
    /// second click cannot start the same hash twice.
    pub(crate) identifying_download: Option<String>,
    /// Last (instant, bytes) seen for each in-flight download, keyed by file
    /// name. Speed is a derivative, so it needs the previous sample; keeping it
    /// out of `DownloadRow` means a rebuilt row list does not lose the history.
    pub(crate) download_samples: HashMap<String, (std::time::Instant, u64)>,
    // ---- multi-select + batch actions ----
    /// Where a Shift extension counts FROM.
    ///
    /// Distinct from `selected_mod`, which is the focus and moves with every
    /// arrow key: if the extension counted from the focus, each Shift+Down would
    /// re-anchor on the row it just reached and the selection would only ever be
    /// two rows long. Set by a plain click and by Ctrl+click, left alone by
    /// Shift - the behaviour of every list widget that has one.
    pub(crate) sel_anchor: Option<usize>,
    /// The multi-selection set (indices into `app.mods`). `selected_mod` stays the
    /// focus anchor for single-row UI; this set drives batch actions and the row
    /// highlight when more than one row is selected.
    pub(crate) selected_mods: HashSet<usize>,
    /// Two-click guard for the destructive batch "Remove selected" action.
    pub(crate) confirm_batch_remove: bool,
    /// The keyboard modifiers currently held, so a plain left-click can branch to
    /// Ctrl-toggle / Shift-extend (iced fires a fixed `on_press` message otherwise).
    pub(crate) modifiers: iced::keyboard::Modifiers,
    /// An in-flight drag-to-reorder (None = not dragging).
    pub(crate) drag_state: Option<DragState>,
    /// Which edge of the mod list the pointer rests on mid-drag, if any. Drives
    /// the auto-scroll tick; `None` stops it.
    pub(crate) drag_scroll: Option<ScrollEdge>,
    /// A collapsed group the drag is resting on, and how many ticks it has
    /// rested. Dropping INTO a collapsed group is otherwise impossible without
    /// abandoning the drag to expand it first, which is the whole reason MO2
    /// expands on hover.
    pub(crate) drag_hover_group: Option<(String, u8)>,
    /// How deep into that band the pointer is, 0.0..1.0. Speed follows it, so
    /// nudging the edge creeps and pushing right against it flies.
    pub(crate) drag_scroll_depth: f32,
    /// The plugin list's Shift anchor; see [`App::sel_anchor`].
    pub(crate) plugin_anchor: Option<usize>,
    /// The focused plugin row, and the multi-selection around it - the same
    /// model the mod list uses, because every batch action needs the same answer
    /// to "which rows am I acting on".
    pub(crate) selected_plugin: Option<usize>,
    pub(crate) selected_plugins: HashSet<usize>,
    /// Which list the arrow keys move in.
    pub(crate) focus: Pane,
    /// The category catalog, read once instead of per frame.
    ///
    /// `Instance::category_factory` opens and parses `categories.dat` on every
    /// call, and the mod list asked it for one on every view - so tracking the
    /// pointer for context-menu placement turned that into a file read and a
    /// parse per mouse MOVE. Rebuilt where the mod list is rebuilt.
    pub(crate) categories: Option<eidos_instance::CategoryFactory>,
    /// The live pointer position, and the window it moves in.
    ///
    /// iced's `on_right_press` carries no coordinates, so a context menu has no
    /// way to know where it was summoned from unless the position is tracked
    /// separately. Both are fed by the event subscription and read only when a
    /// menu opens.
    pub(crate) cursor: iced::Point,
    pub(crate) window: iced::Size,
    /// Where the open context menu was summoned from, frozen at that moment - the
    /// pointer keeps moving afterwards, and a menu that slid along with it would
    /// be unusable.
    pub(crate) menu_at: Option<iced::Point>,
    /// The user is typing into a field on the main screen (the mod filter, a
    /// notes box, an inline rename).
    ///
    /// `on_key_press` is a global subscription: it does not know which widget
    /// has the caret, so without this a space typed into the filter box would
    /// toggle a mod and Home would jump the list instead of the text. Set by any
    /// keystroke that reached a field, cleared the moment a row is pressed or
    /// Escape is hit - approximate at the edges (clicking into a field without
    /// typing leaves it false), but wrong only in the harmless direction.
    pub(crate) typing: bool,
    /// The same, for the plugin list. Kept separate so a drag in one panel can
    /// never be committed against the other's indices, and carrying the legal
    /// range so the illegal strips can simply refuse to be targets.
    pub(crate) plugin_drag: Option<PluginDrag>,
    // ---- profile management (MO2 profiles dialog) ----
    /// The profile whose right-click action menu is open (None = closed).
    pub(crate) profile_menu: Option<String>,
    /// In-progress profile rename: `(original name, edited name)`.
    pub(crate) profile_rename: Option<(String, String)>,
    /// In-progress named copy: `(source name, edited new name)`.
    pub(crate) profile_copy: Option<(String, String)>,
    /// Two-click guard for a profile deletion (the armed profile name).
    pub(crate) profile_delete_confirm: Option<String>,
    // ---- run lock (MO2's "lock GUI while the application runs") ----
    /// A launched game/tool we are waiting on. While `Some` and `prefs.lock_gui`
    /// is set, the window is blocked behind the lock overlay until it exits (or the
    /// user clicks Unlock). `None` = nothing running / not locked.
    pub(crate) running: Option<RunningState>,
    /// The `eidos` binary lacks CAP_SYS_ADMIN (setcap wiped by a rebuild), so FUSE
    /// passthrough cannot engage even if asked for. Harmless by itself - the
    /// capability is optional and passthrough is off by default - so this only
    /// drives a banner when `passthrough_requested()`. Rechecked on Refresh and
    /// after every run.
    pub(crate) cap_missing: bool,
    /// Cached per-layer file walks for the conflict analysis, keyed by layer name
    /// (mod folder / "Overwrite" / "[game]"). RefCell so the read-path
    /// `compute_conflicts(&App)` can fill missing entries; a toggle or reorder
    /// then rebuilds the map without touching the filesystem at all. Entries are
    /// dropped when a layer's contents change (install/remove/rename/run).
    pub(crate) files_cache: std::cell::RefCell<HashMap<String, (Vec<String>, bool)>>,
    // ---- view memoisation (these listings ran on EVERY redraw) ----
    /// Bumped whenever the mod list or anything on disk changes; the memoised
    /// listings below rebuild only when it moves.
    pub(crate) view_generation: std::cell::Cell<u64>,
    /// Memoised Data-tab merged listings, one per directory (keyed by its path
    /// relative to `Data`, `""` for the root), each with the generation it was
    /// built at. The tree merges a level at a time, so only the directories the
    /// user actually opened are ever read.
    pub(crate) data_listing: std::cell::RefCell<HashMap<String, CachedDataListing>>,
    /// The profile chip row's data: every profile name and which one is active.
    ///
    /// Both were read from disk on EVERY frame of the main screen - a `read_dir`
    /// plus an `is_dir` per entry, and a file read for the active name. The
    /// chips are always drawn, so that was a few filesystem calls per frame for
    /// something that changes when the user creates or switches a profile.
    pub(crate) profiles_cache: std::cell::RefCell<Option<(u64, Vec<String>, String)>>,
    /// The Archives tab's rows, built once per view generation.
    ///
    /// `archive_rows` walks every enabled mod's folder and reads an INI. It is
    /// called from `view()`, which runs on every frame - every pointer move,
    /// every keystroke - so without this a four-hundred-mod list did four
    /// hundred `read_dir` calls per frame while the tab was open. The same memo
    /// the merged listing uses, keyed the same way.
    pub(crate) archives_cache: std::cell::RefCell<Option<(u64, Option<Vec<ArchiveRow>>)>>,
    /// The union the Data tab reads, built once per view generation.
    ///
    /// The SAME `LayerStack` the mount serves from, rather than a hand-rolled
    /// merge beside it: whiteouts, opaque directories, hidden names, case-folded
    /// dedup and NTFS collation all live in one place, and the tab can no longer
    /// disagree with the filesystem the game sees.
    pub(crate) data_stack: std::cell::RefCell<Option<CachedDataStack>>,
    /// Free-text filter over the Data tree.
    pub(crate) data_query: String,
    /// Show only paths more than one mod provides.
    pub(crate) data_conflicts_only: bool,
    /// Directories the user expanded in the Data tree, same keys as above. The
    /// root is implicitly expanded and never in here.
    pub(crate) data_expanded: HashSet<String>,
    /// Folders opened in the Overwrite tree, keyed the same way.
    pub(crate) overwrite_expanded: HashSet<String>,
    /// Memoised recursive file listings per directory (the Overwrite tab and the
    /// mod-info file tree), each with the generation it was built at.
    pub(crate) listing_cache: std::cell::RefCell<HashMap<PathBuf, CachedListing>>,
    // ---- LOOT report (MO2's post-sort report dialog) ----
    /// The report from the last LOOT sort, shown as a modal so the user sees
    /// missing masters / messages / dirty-plugin advice. `None` = no report open.
    pub(crate) loot_report: Option<eidos_loot::LootReport>,
    /// Per-plugin LOOT metadata from the last sort (messages, dirty info, Bash
    /// Tags), keyed by ASCII-lowercased plugin name - what the Plugins tab
    /// draws its badges and tooltip lines from. SEPARATE from `loot_report`
    /// deliberately: the report is dialog state, cleared the moment the modal
    /// closes, while these badges must outlive it (MO2 keeps its flags until
    /// the next sort). Cleared on profile/instance switch.
    pub(crate) loot_meta: Option<HashMap<String, eidos_loot::PluginMetadataBundle>>,
    /// Existing instances the welcome screen offers to open: registry
    /// portables + detected globals, last-used first. Refreshed on Restart,
    /// so the welcome screen doubles as the instance switcher.
    pub(crate) known: Vec<KnownInstance>,
}

/// Which criterion a [`Message::CycleFilter`] refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterField {
    Active,
    Conflicted,
    Update,
    Plugins,
    Uncategorised,
}

/// The slice of a mod's `meta.ini` the main window shows (extra columns + the
/// Nexus action). Cached so a search keystroke doesn't re-read every file.
#[derive(Debug, Clone, Default)]
pub(crate) struct RowMeta {
    pub(crate) version: Option<String>,
    pub(crate) mod_id: Option<u64>,
    pub(crate) installed_files: Vec<(u64, u64)>,
    pub(crate) install_warning: Option<String>,
    /// The mod's PRIMARY category id (MO2 `category=` first id), for filtering.
    pub(crate) category_id: Option<i32>,
    /// That category resolved to a display name (MO2 Category column).
    pub(crate) category_name: Option<String>,
    /// MO2 Content column: a compact letters string of the kinds of content the mod
    /// ships (P/A/T/M/S/K/I/U/F), empty if none.
    pub(crate) content_tags: String,
    pub(crate) update: bool,
    /// The row's display colour (MO2's `color=@Variant(...)`), if set. Stored per
    /// mod, not only per separator.
    pub(crate) color: Option<[u8; 3]>,
    /// `Some(false)` when the last update check found the Nexus page gone.
    /// `None` means nobody has checked, which must NOT draw a warning.
    pub(crate) nexus_gone: bool,
    /// The user's note, shown as a glyph with the text on hover. MO2 gives it a
    /// column; here it rides the Flags cell, because every column costs width off
    /// the name and a note is read on demand rather than scanned.
    pub(crate) notes: Option<String>,
    /// The mod's top level looks like nothing this game loads - MO2's "no valid
    /// game data". Never set for a mod the user has marked valid.
    pub(crate) invalid_data: bool,
    /// Who made it (`author=`), for the Author column.
    pub(crate) author: Option<String>,
    /// The game the `meta.ini` names, whatever it is - the Source game column.
    /// Distinct from `other_game`, which is only set when it DIFFERS.
    pub(crate) game_name: Option<String>,
    /// When the mod folder was last written, for the Installed column and for
    /// sorting by it. MO2 shows the install date; a mod directory's mtime is
    /// the closest thing on disk that costs nothing extra to read.
    pub(crate) installed_at: Option<std::time::SystemTime>,
    /// The game this mod's `meta.ini` says it was downloaded for, when that is
    /// NOT the instance's game. `None` covers both "same game" and "does not
    /// say", which have to look identical: a mod installed from a folder never
    /// had a game recorded, and warning about that would flag half a list.
    pub(crate) other_game: Option<String>,
}
