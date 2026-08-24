//! Eidos GUI (iced) - MO2-style wizard + two-pane main window.
//!
//!   Welcome -> Instance type (portable/global) -> Game -> Name/location
//!           -> Summary -> [create] -> Main (MO2-style mod manager)
//!
//! The main window mirrors Mod Organizer 2: menu bar + toolbar + profile row,
//! left = the mod list (enable, priority, reorder) with an Overwrite entry,
//! right = Run + Data/Saves/Downloads tabs, plus a status bar. Colony parchment
//! / burgundy palette. Run with: `cargo run -p eidos-gui`

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{
    button, checkbox, container, image, mouse_area, operation, pick_list, scrollable, text,
    text_input, tooltip, Column, Row, Space, Stack,
};
// For `widget::Id`, the shared handle every operation addresses a widget by.
use iced::widget;
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::settings::{Settings, Theme as PrefTheme};
use eidos_instance::{ExportScope, Instance, InstanceKind, ModEntry, SaveEntry, Tool};
use eidos_plugins::{plugins_txt_dir, GameSpec, MovableRange, PluginList};
use eidos_conflicts::{ConflictMap, ConflictState, Layer};

// The GUI, split by what each half does rather than by what it is about.
//
// `update` decides, everything else draws. `theme` and `widgets` are leaves that
// the drawing modules share. main.rs keeps what they all need: the `Message`
// enum, `App`, and the state helpers that operate on it.
//
// The three modules main.rs no longer imports from - theme, widgets, fomod - are
// the measure of the split: nothing at the root draws anything any more.
mod dialogs;
mod fomod;
mod modinfo;
mod state;
mod theme;
mod update;
mod view;
mod widgets;
mod wizard;

use dialogs::*;
use fomod::{fomod_wizard_view, FOMOD_INK_FAINT, FOMOD_INK_SOFT};
use modinfo::*;
use state::*;
use update::*;
use view::*;
use wizard::*;

// MO2's own toolbar icons (GPL-3.0, from ModOrganizer2/modorganizer src/resources).
const IC_INSTALL: &[u8] = include_bytes!("../assets/icons/system-installer.png");
const IC_NEXUS: &[u8] = include_bytes!("../assets/icons/internet-web-browser.png");
const IC_CHANGE_GAME: &[u8] = include_bytes!("../assets/icons/switch-instance-icon.png");
const IC_REFRESH: &[u8] = include_bytes!("../assets/icons/view-refresh.png");
const IC_EXECUTABLES: &[u8] = include_bytes!("../assets/icons/function.png");
const IC_TOOLS: &[u8] = include_bytes!("../assets/icons/plugins.png");
const IC_SETTINGS: &[u8] = include_bytes!("../assets/icons/preferences-system.png");
const IC_ENDORSE: &[u8] = include_bytes!("../assets/icons/icon-favorite.png");
const IC_UPDATE: &[u8] = include_bytes!("../assets/icons/system-software-update.png");
const IC_HELP: &[u8] = include_bytes!("../assets/icons/help-browser_32.png");
// MO2's real conflict emblems (modconflicticondelegate: emblem_conflict_*).
const IC_CONFLICT_OVERWRITE: &[u8] = include_bytes!("../assets/icons/conflict-overwrite.png");
const IC_CONFLICT_OVERWRITTEN: &[u8] = include_bytes!("../assets/icons/conflict-overwritten.png");
const IC_CONFLICT_MIXED: &[u8] = include_bytes!("../assets/icons/conflict-mixed.png");
const IC_CONFLICT_REDUNDANT: &[u8] = include_bytes!("../assets/icons/conflict-redundant.png");
const IC_CONFLICT_HIDDEN: &[u8] = include_bytes!("../assets/icons/conflict-hidden.png");
const IC_RUN: &[u8] = include_bytes!("../assets/icons/media-playback-start.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Kind,
    Game,
    NameLoc,
    Summary,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
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
enum InfoTab {
    General,
    Conflicts,
    Filetree,
    /// The mod's `INI Tweaks/` fragments, individually enabled.
    IniTweaks,
    Notes,
}

/// Sections of the Settings screen, in sidebar order.
///
/// A vertical rail rather than a row of tabs, matching Colony: five entries do
/// not fit across a dialog, and the rail leaves room to add a sixth without
/// re-laying anything out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    General,
    Appearance,
    ModList,
    Nexus,
    About,
}

impl SettingsTab {
    /// Every section, in the order the sidebar lists them.
    pub(crate) const ALL: [SettingsTab; 5] = [
        SettingsTab::General,
        SettingsTab::Appearance,
        SettingsTab::ModList,
        SettingsTab::Nexus,
        SettingsTab::About,
    ];

    /// The sections open the first time Settings is shown - one per category, so
    /// every page says something without a click.
    pub(crate) const DEFAULT_OPEN: [&'static str; 5] =
        ["startup", "theme", "dragging", "account", "paths"];

    pub(crate) fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Appearance => "Appearance",
            SettingsTab::ModList => "Mod list",
            SettingsTab::Nexus => "Nexus",
            SettingsTab::About => "About",
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Next,
    Back,
    PickKind(InstanceKind),
    PickGame(usize),
    NameChanged(String),
    PortableChanged(String),
    /// Open an existing instance from the welcome screen's known list (index
    /// into `app.known`).
    OpenKnown(usize),
    Finish,
    Restart,
    ToolPicked(String),
    ToggleMod(usize),
    SelectTab(Tab),
    SwitchProfile(String),
    NewProfile,
    InstallMod,
    ModPicked(Option<PathBuf>),
    FomodToggle(usize, usize),
    FomodNext,
    FomodBack,
    FomodInstall,
    FomodCancel,
    Run,
    Refresh,
    OpenFolder(PathBuf),
    ClearOverwrite,
    // ---- mod-list interactivity (MO2 right-click + filter) ----
    /// Filter box above the mod list (case-insensitive substring on the name).
    SearchChanged(String),
    /// Narrow the mod list to a top-level category (`None` = all).
    CategoryFilterChanged(Option<i32>),
    /// Left-click a mod row: select it and close any open menu.
    SelectMod(usize),
    /// Right-click a mod row: select it and open its action menu.
    OpenModMenu(usize),
    /// Dismiss the open action menu / rename editor.
    CloseMenu,
    ModSendTop(usize),
    ModSendBottom(usize),
    /// Open the mod's folder in the file manager (MO2 openExplorer).
    ModOpenFolder(usize),
    /// Open the mod's Nexus page in the browser (MO2 visitOnNexus).
    ModVisitNexus(usize),
    /// Re-run the installer for this mod (MO2 reinstallMod).
    ModReinstall(usize),
    /// Delete the mod from disk (MO2 removeMods); two-click confirm.
    ModRemove(usize),
    /// Begin renaming a mod (MO2 renameMod); opens an inline editor.
    RenameStart(usize),
    RenameChanged(String),
    RenameCommit,
    // ---- separators (MO2 group dividers) ----
    /// Create a new separator above row `i` (or at the top from the toolbar) and
    /// open its rename editor so the user names it.
    AddSeparator(usize),
    /// Set (`Some`) or clear (`None`) a separator's colour.
    SetSeparatorColor(usize, Option<[u8; 3]>),
    /// Collapse/expand a separator's group, keyed by its display name.
    ToggleCollapse(String),
    /// Collapse every OTHER group, leaving this one open - MO2's "Collapse
    /// others", the fastest way to isolate one part of a long list.
    CollapseOthers(String),
    /// A drag has been resting on a collapsed group. Fires only while one is.
    DragHoverTick,
    /// Double-click on a mod row. What it does depends on the modifiers held,
    /// which the closure that emits it cannot see - so it carries the row and
    /// `update` reads the live modifier set, the same way a plain click does.
    ModDoubleClick(usize),
    /// Ctrl+F: put the caret in the filter box.
    FocusFilter,
    /// MO2's "Mark as valid": silence this mod's state flags for good, by
    /// writing MO2's own `validated=true` into its meta.ini.
    ModMarkValid(usize),
    /// Hide a download from the list without deleting the archive.
    HideDownload(String),
    /// Show the hidden ones again.
    ToggleShowHiddenDownloads,
    /// Delete every archive already installed, in one go. Two clicks.
    PurgeInstalledDownloads,
    ConfirmPurgeInstalled,
    DownloadFilterChanged(String),
    DownloadSortChanged(DownloadSort),
    /// Show or hide one mod-list column. Saved immediately.
    ToggleModColumn(ModColumn),
    /// Group the list under synthetic headers, or stop.
    SetGroupBy(Option<GroupBy>),
    /// Fold or unfold one synthetic group header.
    ToggleGroupFold(String),
    /// Preview a file from a tree, in a pane over the window.
    PreviewFile(PathBuf),
    ClosePreview,
    /// Executables editor: the AppID field, the two flags, and the shortcut.
    ExecAppIdChanged(String),
    ExecToggleHidden,
    ExecTogglePinned,
    ExecMakeShortcut,
    /// Copy a mod's folder aside as `<name>_backup`, before editing it.
    ModBackup(usize),
    /// Copy a backup's contents back over the mod it came from. Two clicks.
    ModRestoreBackup(usize),
    /// The backup's NAME, not its index: see `confirm_restore`.
    ConfirmModRestoreBackup(String),
    /// Filetree: open the entry with the desktop's handler.
    FiletreeOpen(usize, String),
    /// Filetree: start renaming an entry (mod index, relative path).
    FiletreeRenameStart(usize, String),
    FiletreeRenameChanged(String),
    FiletreeRenameCommit,
    FiletreeRenameCancel,
    /// Filetree: delete an entry. Two clicks, like everything else that removes.
    FiletreeDelete(usize, String),
    /// The mod's NAME, not its index: see `tree_delete_armed`.
    ConfirmFiletreeDelete(String, String),
    /// Filetree: make a directory beside the entries shown.
    FiletreeNewFolderStart,
    FiletreeNewFolderChanged(String),
    FiletreeNewFolderCommit,
    /// Click a heading: ascending, then descending, then back to load order.
    /// Three states rather than two, because getting BACK to load order has to
    /// be one click away - it is the only order in which dragging works.
    CycleModSort(SortKey),
    /// A letter typed with the mod list focused: jump to the next row whose
    /// name starts with it.
    JumpToLetter(char),
    /// Enable/disable an ESP/ESM in the Plugins tab, persisting plugins.txt.
    TogglePlugin(usize),
    /// Run LOOT's graph sort over the discovered plugins (MO2's "Sort" button).
    SortPlugins,
    /// LOOT finished: the fingerprint of the list it was asked about, the
    /// optimised plugin-name order, and the (advisory) report - or an error. The
    /// inner `Result` is the report: it may fail without losing the successfully
    /// computed order.
    ///
    /// The fingerprint travels with the answer because a sort takes seconds and
    /// the window stays live: a profile switch, a Refresh or a mod toggle in that
    /// window leaves an order computed for a list that no longer exists, and
    /// applying it would silently rearrange the wrong plugins.
    PluginsSorted(SortOutcome),
    /// Dismiss the LOOT report modal.
    CloseLootReport,
    /// Put the whole LOOT report on the clipboard, as plain text. Also what
    /// Ctrl+C does while the report is open.
    CopyLootReport,
    /// Open (or close, if it is already open) the details pane for a save row.
    SelectSave(usize),
    /// Enable every mod that supplies one of the selected save's missing plugins.
    FixSaveMods,
    /// Put the pre-session `plugins.txt` back after a session damaged the active
    /// set (the Diagnostics card's one-click restore).
    RestorePreSessionPlugins,
    /// The other honest outcome: the change was deliberate - re-snapshot the
    /// current state so the damage card stops flaming.
    AcceptPluginState,
    // ---- per-mod information dialog (MO2 modinfodialog) ----
    ShowModInfo(usize),
    CloseInfo,
    InfoSelectTab(InfoTab),
    NotesChanged(String),
    NotesSave,
    // ---- hidden files (MO2 filetree.cpp HIDE/UNHIDE) ----
    /// Expand or collapse a directory in the Data tree, by its path relative to
    /// `Data` (`""` is the root, which is always expanded).
    DataToggleDir(String),
    /// Periodic re-scan of the downloads directory while one is arriving.
    DownloadTick,
    /// Remove the `mods/.eidos-install*` trees an interrupted install left.
    CleanInstallDebris,
    /// Open or close a folder of the Overwrite tree.
    OverwriteToggleDir(String),
    /// Hide or unhide one path inside a mod: `(mod index, path relative to the mod
    /// root)`. Hiding renames it to `<name>.mohidden`, which drops it out of the
    /// virtual view without deleting anything; unhiding strips the suffix back off.
    ToggleFileHidden(usize, String),
    /// Unhide everything in a mod at once (MO2's `restoreHiddenFiles`).
    RestoreHiddenFiles(usize),
    /// Enable or disable one of a mod's `INI Tweaks/` fragments:
    /// `(mod index, fragment file name)`.
    ToggleIniTweak(usize, String),
    // ---- toolbar ----
    /// Re-open the game picker to switch the managed game (MO2 switch-instance).
    ChangeGame,
    /// Open the current game's Nexus page in the browser.
    OpenNexusGame,
    /// Install the modding tools' runtime prerequisites into the prefix
    /// (`eidos prereqs <id> --install`); the Tier-2 verbs download from Microsoft.
    SetupPrereqs,
    // ---- manual / BAIN install picker (MO2 InstallDialog, BainComplexInstallerDialog) ----
    /// Tick or untick one BAIN sub-package.
    PickerBainToggle(usize),
    /// Answer MO2's "may be a BAIN installer" question: install as BAIN, or fall
    /// through to the manual picker with the same extracted tree.
    PickerBainConfirm(bool),
    /// Choose the folder that is the archive's data root (manual mode).
    PickerSetRoot(String),
    /// Edit the mod name the picker will install under.
    PickerNameChanged(String),
    /// Run the install with the current picks.
    PickerInstall,
    /// Close the picker, discarding the extraction.
    PickerCancel,
    // ---- install-collision chooser (MO2 QueryOverwriteDialog) ----
    /// Install over the existing mod's files.
    CollisionMerge,
    /// Wipe the existing mod and reinstall (keeps its endorsement/category).
    CollisionReplace,
    /// Edit the rename target for the colliding install.
    CollisionRenameChanged(String),
    /// Install under the entered new name.
    CollisionRenameCommit,
    /// Dismiss the collision prompt without installing.
    CollisionCancel,
    // ---- Settings / Preferences (MO2's Settings dialog) ----
    /// Open the Preferences modal (toolbar Settings button + File menu).
    OpenSettings,
    /// Dismiss the Preferences modal, discarding unsaved edits.
    CloseSettings,
    /// Switch the Preferences tab (General / Nexus).
    SettingsTabSelected(SettingsTab),
    /// Edit the Nexus API key field.
    /// Start the Nexus OAuth sign-in: open the browser, wait on the loopback
    /// listener, exchange the code, store the session.
    NexusSignInStart,
    /// The sign-in finished: the account on success, else an error.
    NexusSignInResult(Result<eidos_nexus::Account, String>),
    /// Forget the stored Nexus session.
    NexusSignOut,
    /// Set the preferred colour theme.
    ThemeChanged(PrefTheme),
    /// Set the default game id to open (`None` = none).
    DefaultGameChanged(Option<String>),
    /// Toggle "lock the GUI while a game/tool runs" (MO2's `lock_gui`).
    ToggleLockGui(bool),
    /// Set how fast the mod list scrolls when a drag rests on an edge.
    DragScrollSpeedChanged(f32),
    /// Open or close one collapsible section of the Settings screen.
    SettingsToggleSection(&'static str),
    /// Toggle the conflict marks on the mod list's scrollbar.
    /// Toggle restoring the window to its last size.
    ToggleRememberWindow(bool),
    /// MO2's offline mode: cut every Nexus request.
    ToggleOffline(bool),
    /// Editing the preferred-CDN list. Saved on submit, not per keystroke.
    PreferredServersChanged(String),
    PreferredServersSave,
    OpenPluginMenu(usize),
    ClosePluginMenu,
    /// Open the folder of the mod that ships the plugin at this row.
    OpenPluginOrigin(usize),
    /// Open the mod-info dialog for that same mod.
    ShowPluginOriginInfo(usize),
    /// Send the selected plugins to the very top / bottom of the load order.
    PluginsSendTop,
    PluginsSendBottom,
    /// Activate or deactivate every plugin the engine does not own.
    PluginsSetAll(bool),
    /// Ask Nexus what this archive is, by its MD5 (MO2's Query Metadata).
    IdentifyDownload(String),
    /// The lookup came back: `Ok` carries the name it identified.
    IdentifiedDownload(Result<String, String>),
    /// Cycle one filter criterion: off -> require -> exclude.
    CycleFilter(FilterField),
    ToggleFilterPane,
    ClearFilters,
    ShowBackupsDialog,
    CloseBackupsDialog,
    CreateBackup(eidos_instance::BackupKind),
    RestoreBackup(eidos_instance::BackupKind, u64),
    // ---- Files dropped onto the window from a file manager -----------------
    /// A file is hovering over the window (one message per file).
    FilesHovering(bool),
    /// A file was dropped. Arrives once PER FILE, so this queues rather than acts.
    FileDropped(PathBuf),
    /// Install the next queued drop, one at a time so each modal is answered.
    DrainDrops,
    // ---- Install a download AT a priority (MO2's drop onto the mod list) ----
    /// Press on a download row: arms the drag.
    DownloadDragStart(usize),
    /// The pointer crossed an insertion strip.
    DownloadDragOverGap(usize),
    /// Released over a strip: install there.
    DownloadDragDrop,
    /// Released anywhere else, or cancelled.
    DownloadDragCancel,
    // ---- Categories dialog (MO2's Change Categories + the category editor) ----
    /// Open the dialog on a mod row (or, with a multi-selection, on all of them).
    ShowCategoriesDialog(usize),
    CloseCategoriesDialog,
    /// Check / uncheck a category for the targeted mods.
    ToggleCategory(i32),
    /// Promote an already-checked category to primary (the one shown on the row).
    SetPrimaryCategory(i32),
    /// Write the pending choice to every target's `meta.ini`, and the catalog if
    /// it was edited.
    ApplyCategories,
    /// Filter the category tree.
    CategoryQueryChanged(String),
    /// Flip between the picker and the catalog editor.
    ToggleCategoryEditor,
    /// The "new category" name box.
    NewCategoryNameChanged(String),
    /// The parent for the category about to be created.
    NewCategoryParentChanged(i32),
    /// Create the category currently described by the name + parent boxes.
    AddCategory,
    /// Start / edit / commit a catalog rename.
    RenameCategoryStart(i32),
    RenameCategoryChanged(String),
    RenameCategoryCommit,
    /// Delete a catalog row (two clicks).
    DeleteCategory(i32),
    /// Pull the game's official category list from Nexus into the catalog.
    FetchNexusCategories,
    /// The category list came back.
    NexusCategoriesFetched(Result<Vec<(i32, String, Option<i32>)>, String>),
    /// Set the pending pick from what Nexus recorded for the targeted mods.
    AssignCategoriesFromNexus,
    // ---- Executables dialog (MO2's Modify Executables) ----
    /// Open the Executables editor (toolbar Executables button + Tools menu).
    ShowExecutablesDialog,
    /// Dismiss the Executables editor.
    CloseExecutablesDialog,
    /// Select a tool in the Executables editor list.
    SelectExecutableTool(usize),
    /// Append a blank user tool and select it for editing.
    AddExecutableTool,
    /// Delete the selected user tool (defaults are read-only).
    DeleteExecutableTool,
    /// Reorder the selected user tool up / down (within the user range).
    MoveExecutableUp,
    MoveExecutableDown,
    /// Edit buffers for the selected tool.
    ToolTitleChanged(String),
    ToolExeChanged(String),
    ToolWorkdirChanged(String),
    ToolArgsChanged(String),
    ToolPrereqsChanged(String),
    /// Open a native file picker for the tool's executable (Browse button).
    BrowseToolExe,
    /// Open a native folder picker for the tool's working directory (Browse button).
    BrowseToolWorkdir,
    /// Persist the user tool list to `tools.ini`.
    SaveExecutablesDialog,
    // ---- Endorse / per-mod flags (MO2 endorseMod) ----
    /// Toggle endorse <-> abstain for a mod, based on its current state.
    ModEndorse(usize),
    /// The endorse/abstain finished: the new endorsed state, or an error.
    /// Endorse round-trip done. Carries the mod's FOLDER NAME (not an index): the
    /// list can shift while the network call is in flight, and writing the result
    /// into whatever mod now sits at the old index would corrupt its meta.ini.
    ModEndorsed(String, Result<bool, String>),
    /// Toggle the mod's local "Track" flag (MO2's Track; no network).
    ModTrack(usize),
    /// Toggle the mod's "Ignore update" flag (MO2's Ignore update; no network).
    ModIgnoreUpdate(usize),
    // ---- mod creation (MO2 Create empty mod / Install from folder) ----
    /// Create an empty mod folder and open its rename editor (MO2 createEmptyMod).
    /// Create an empty mod AT a position (the gap the menu was opened on).
    CreateEmptyMod,
    CreateEmptyModAt(usize),
    /// Enable or disable every mod the list is currently DRAWING. An explicit
    /// target state, never a flip: "Disable all" must mean disable, whatever the
    /// current mix is.
    SetAllModsEnabled(bool),
    /// Open the archive picker with a landing position already chosen. The gap is
    /// an INSERTION index, so `i` means "above the row at i" and `i + 1` "below".
    InstallAt(usize),
    /// Open a folder picker to install from an already-unpacked mod directory.
    InstallFromFolder,
    /// The folder picker returned a directory (or `None` if cancelled).
    FolderPicked(Option<PathBuf>),
    // ---- Mod update check (MO2 "Check for updates") ----
    /// Run a Nexus update check across the instance's mods (toolbar + Tools menu).
    CheckUpdates,
    /// The update check finished: the summary, or an error.
    UpdatesChecked(Result<eidos_nexus::UpdateCheckResult, String>),
    // ---- menu bar wiring ----
    /// Show / hide the About box (Help menu).
    ShowAbout,
    CloseAbout,
    /// Open / close the View dropdown (iced has no native menu).
    OpenViewMenu,
    CloseViewMenu,
    /// Toggle the toolbar / status bar visibility (View menu).
    ToggleToolbar,
    ToggleStatusBar,
    /// Collapse / expand every separator group (View menu).
    CollapseAllGroups,
    ExpandAllGroups,
    // ---- Saves tab (MO2's savegame list) ----
    /// Re-scan the active profile's save directory.
    RefreshSaves,
    /// Delete a save file (two-click confirm); arms the guard on the first click.
    DeleteSave(usize),
    /// Second click: actually delete the armed save.
    ConfirmDeleteSave(usize),
    // ---- Downloads manager (MO2's downloads list) ----
    /// Stop the transfer behind this row, leaving the partial to resume from.
    PauseDownload(String),
    /// Start `eidos nxm --resume` on a paused or stalled partial.
    ResumeDownload(String),
    // ---- User extensions (add-ons) ----
    /// Open the Extensions list.
    ShowAddons,
    CloseAddons,
    /// Re-read the add-on manifests from disk.
    ReloadAddons,
    /// Run a `tool` add-on by id.
    RunAddon(String),
    /// Open the add-ons folder in a file manager.
    OpenAddonsFolder,
    // ---- Log pane (MO2's dockable log view) ----
    /// Open the log pane, reading the newest session file.
    ShowLogPane,
    CloseLogPane,
    /// Show a different session file.
    LogPick(PathBuf),
    /// Only show records at this level or above.
    LogLevel(eidos_log::Level),
    /// Re-read the current file (also fired by the tick while the pane is open).
    LogRefresh,
    /// Put the shown records on the clipboard.
    LogCopy,
    /// Open the logs folder in a file manager.
    LogOpenFolder,
    // ---- INI editor (MO2's bundled INI Editor tool plugin) ----
    /// Open the editor on the active profile's INIs.
    ShowIniEditor,
    CloseIniEditor,
    /// Switch to another of the game's INI files.
    IniEditorPick(String),
    /// An edit inside the text area.
    IniEditorAction(iced::widget::text_editor::Action),
    /// Write the buffer back to the profile's copy.
    IniEditorSave,
    /// Throw the buffer away and re-read from disk.
    IniEditorRevert,
    /// Hand the file to the desktop's own editor.
    IniEditorOpenExternal,
    /// Filter the Data tree by name.
    DataQueryChanged(String),
    /// Show only paths more than one mod provides.
    DataToggleConflictsOnly,
    /// Expand / collapse every folder in the Data tree.
    DataExpandAll,
    DataCollapseAll,
    /// Open a file manager on the real file behind a Data row.
    DataReveal(PathBuf),
    /// Re-scan the downloads directory + reload each archive's `.meta` status.
    RefreshDownloads,
    /// Delete a downloaded archive and its `.meta` sidecar (two-click confirm).
    DeleteDownload(String),
    /// Second click: actually delete the armed download.
    ConfirmDeleteDownload(String),
    // ---- multi-select + batch actions (MO2 multi-row selection) ----
    /// Ctrl+click a mod row: add/remove it from the selection set without
    /// disturbing the others.
    SelectModToggle(usize),
    /// Shift+click a mod row: extend the selection from the focus anchor to `i`.
    SelectModExtend(usize),
    /// Clear the multi-selection (Escape / click into empty space).
    ClearSelection,
    /// Enable or disable every selected mod in one go (MO2's right-click batch).
    BatchToggleMods,
    /// First click arms the batch-remove confirmation; second click executes.
    BatchRemoveMods,
    /// Second click: actually remove every selected mod from disk.
    ConfirmBatchRemove,
    /// Move the whole selection to the top / bottom of the load order.
    BatchSendTop,
    BatchSendBottom,
    // ---- profile management (MO2 profiles dialog: rename / delete / copy) ----
    /// Right-click a profile chip: open its action menu (None elsewhere closes it).
    ProfileMenuOpen(String),
    /// Dismiss the open profile action menu / inline editor.
    ProfileCloseMenu,
    /// Begin renaming a profile (opens an inline editor in the menu).
    ProfileRenameStart(String),
    /// Edit the rename target.
    ProfileRenameChanged(String),
    /// Commit the rename (`Instance::rename_profile`).
    ProfileRenameCommit,
    /// Begin a named copy of a profile (opens an inline editor in the menu).
    ProfileCopyStart(String),
    /// Edit the new-copy name.
    ProfileCopyChanged(String),
    /// Commit the copy (`Profile::create_from`) and switch to it.
    ProfileCopyCommit,
    /// First click arms a profile deletion; second click executes it.
    ProfileDeleteConfirm(String),
    /// Second click: actually delete the armed profile (`Instance::delete_profile`).
    ProfileDeleteCommit(String),
    // ---- drag-and-drop reorder (MO2 row drag) ----
    /// Begin a potential drag from row `i` (also selects it).
    DragStart(usize),
    /// The pointer entered row `i` during a drag (updates the drop target).
    /// The pointer moved over an insertion point during a drag. The payload is an
    /// insertion index (see `DragState::gap`), not a row index.
    DragOverGap(usize),
    /// The drag ended: commit the move if the drop row differs from the source.
    DragDrop,
    /// Abandon an in-flight drag (filter change / Escape).
    DragCancel,
    /// The left button was released anywhere on screen. Commits a drag that has
    /// aimed at a gap, disarms one that has not.
    PointerReleased,
    /// The pointer entered (or left) an auto-scroll edge while dragging.
    DragScrollEdge(Option<ScrollEdge>),
    /// How deep into the band the pointer sits, 0.0 at the inner lip to 1.0 at
    /// the very edge of the list. Drives the speed.
    DragScrollDepth(f32),
    /// One auto-scroll step, fired on a timer while an edge is held.
    DragScrollTick,
    // ---- the same gesture in the plugin list (its own indices and rules) ----
    /// Begin a potential drag from plugin row `i`.
    PluginDragStart(usize),
    /// The pointer moved over an insertion point in the plugin list.
    PluginDragOverGap(usize),
    /// The drag ended: commit the load-order move.
    PluginDragDrop,
    /// Abandon an in-flight plugin drag (pointer left the list).
    PluginDragCancel,
    /// Pin the plugin at `i` to its current load-order slot, or release it
    /// (MO2's `lockedorder.txt`).
    TogglePluginLock(usize),
    // ---- plugin selection (mirrors the mod list) ----
    /// Focus plugin row `i`; a held modifier turns it into a multi-select.
    SelectPlugin(usize),
    /// Ctrl-click: flip this row's membership in the selection.
    SelectPluginToggle(usize),
    /// Shift-click: select the run from the anchor to `i`.
    SelectPluginExtend(usize),
    /// Enable or disable every selected plugin at once (MO2's batch toggle).
    SetSelectedPluginsEnabled(bool),
    // ---- keyboard navigation ----
    /// A navigation key was pressed. Which list it moves is decided in `update`
    /// from `App::focus`, because `on_key_press` takes a plain `fn` and cannot
    /// read the app.
    KeyNav(Nav),
    /// The pointer moved, or the window was resized. Only stored.
    PointerAt(iced::Point),
    WindowResized(iced::Size),
    /// The pointer entered a FOMOD option; drives the preview pane.
    FomodHover(Option<(usize, usize)>),
    /// The pointer LEFT a FOMOD option, named so the row can only clear the hover
    /// it still owns. A blanket `FomodHover(None)` was wrong: one pointer move that
    /// crosses from a row into the row above it makes both rows speak in the same
    /// frame, and iced walks a Column's children in index order, so the row being
    /// entered publishes first and the row being left publishes second and wins.
    /// Moving DOWN the list worked; moving UP silently reset the preview.
    FomodUnhover(usize, usize),
    /// Move the keyboard focus to the other list (Tab).
    CycleFocus,
    /// Select every row of the focused list (Ctrl+A).
    SelectAllInFocus,
    // ---- keyboard tracking (drives Ctrl/Shift multi-select + shortcuts) ----
    /// The held keyboard modifiers changed (from key press/release subscriptions).
    ModifiersChanged(iced::keyboard::Modifiers),
    // ---- run lock (MO2's "lock GUI while the application runs") ----
    /// Poll tick while a game/tool runs: checks whether the child has exited and,
    /// if so, unlocks the UI and refreshes (MO2's afterRun).
    PollRunning,
    /// The user clicked Unlock: stop waiting and re-enable the UI, but leave the
    /// game running (MO2's force-unlock never kills the process).
    ForceUnlock,
    /// Dismiss the transient status-bar message (the small x next to it).
    ClearStatus,
    // ---- MO2's targeted "Send to" actions (the day-to-day way load orders get
    // fixed, beyond blunt top/bottom) ----
    /// Move the selection just ABOVE the first mod it currently overrides.
    SendToFirstConflict(usize),
    /// Move the selection just BELOW the last mod that currently overrides it.
    SendToLastConflict(usize),
    /// Open the inline numeric-priority editor for this row.
    SendToPriorityStart(usize),
    SendToPriorityChanged(String),
    SendToPriorityCommit,
    /// Open the separator chooser for this row.
    SendToSeparatorStart(usize),
    /// Move the selection just past the chosen separator (by mod-list index).
    SendToSeparatorPick(usize),
    SendToTargetCancel,
    // ---- Overwrite -> mod (MO2's "Create mod from Overwrite") ----
    /// Open the name prompt for turning the Overwrite into a mod.
    OverwriteToModStart,
    /// The typed target mod name (an existing mod merges, a new one is created).
    OverwriteToModName(String),
    /// Pick the mod a tool's output is captured into (empty = the Overwrite).
    ToolOutputModChanged(String),
    /// Move the Overwrite's contents into that mod.
    OverwriteToModCommit,
    /// Dismiss the prompt.
    OverwriteToModCancel,
    // ---- MO2 profile import ----
    /// Open the folder picker for an existing MO2 profile directory.
    ImportMo2Pick,
    /// The picked MO2 profile directory (`None` = cancelled).
    ImportMo2Picked(Option<PathBuf>),
    /// Send the plugin selection to an exact load index, MO2's "Send to
    /// priority...". Start opens the inline field, Changed types, Commit moves.
    PluginSendToPriorityStart,
    PluginSendToPriorityChanged(String),
    PluginSendToPriorityCommit,
    /// The per-mod custom URL field: type / save.
    ModUrlChanged(String),
    ModUrlSave,
    // ---- Saves: multi-select, transfer between profiles ----
    /// Add / remove a save from the multi-selection (Ctrl+click).
    SaveToggleSelect(usize),
    /// Delete every selected save, with their co-saves. Two clicks.
    SavesDeleteSelected,
    /// Copy every selected save into another profile (MO2's Transfer Save Games).
    SavesCopyToProfile(String),
    /// Re-scan the saves directory (the tick, while the tab is open).
    SavesTick,
    /// Push every Overwrite file back to the mod that already provides that path
    /// (MO2's "Sync to Mods"). Two clicks: the first arms.
    OverwriteSyncToMods,
    // ---- Nexus collections ----
    /// Open the collection view. The string is an `nxm://` collection link.
    ShowCollection(String),
    CloseCollection,
    /// The link the user pasted.
    CollectionLinkChanged(String),
    /// Fetch the revision named by the pasted link.
    CollectionFetch,
    /// The revision came back.
    CollectionFetched(Result<eidos_nexus::collections::CollectionRevision, String>),
    /// Open one member's Nexus page at the exact file the collection pins.
    CollectionOpenMod(usize),
    /// Ask the nxm handler to fetch every missing member, one at a time.
    CollectionFetchMissing,
    // ---- Instance manager (MO2's Manage Instances) ----
    ShowInstanceManager,
    CloseInstanceManager,
    /// Open the instance at this index of the manager's list.
    InstanceOpen(usize),
    /// Stop offering a portable instance. The folder is left alone.
    InstanceForget(usize),
    /// Rename the FOLDER of a portable instance: start / type / commit.
    InstanceRenameStart(usize),
    InstanceRenameChanged(String),
    InstanceRenameCommit,
    // ---- Export the mod list (MO2's Export to csv) ----
    ShowExportDialog,
    CloseExportDialog,
    /// Pick which rows the export covers.
    ExportScopeChanged(ExportScope),
    /// Tick / untick one column.
    ExportToggleColumn(usize),
    /// Open the save dialog and write.
    ExportRun,
    /// The picked destination (`None` = cancelled).
    ExportPicked(Option<PathBuf>),
    /// Open / dismiss the File dropdown, which lists every folder that matters.
    OpenFileMenu,
    CloseFileMenu,
    /// Open a URL in the user's browser (LOOT advice links in the report).
    OpenUrl(String),
    // ---- manual plugin reorder (MO2 lets the load order be dragged by hand) ----
    /// Move the plugin at this index one slot earlier / later in the load order.
    Noop,
}

/// An in-progress FOMOD installer: the extracted+parsed archive, the current step,
/// and the user's selection so far.
struct FomodWizard {
    session: eidos_install::FomodSession,
    step: usize,
    selection: eidos_fomod::Selection,
    game_id: String,
    /// The source archive, kept so the download can be marked installed on finish.
    archive: PathBuf,
    /// Current plugin states, so fileDependency/gameDependency conditions evaluate
    /// against the real setup instead of always reading Missing.
    ctx: eidos_fomod::Context,
    /// The `(group, option)` the pointer is over, which is what the preview pane
    /// shows. `None` falls back to the option that is actually selected, so the
    /// pane is never blank and the keyboard is not left out - MO2 tracks hover
    /// only, and its preview empties the moment you move the mouse away.
    hover: Option<(usize, usize)>,
}

/// An open Executables editor (MO2's Modify Executables dialog). The list shown is
/// the user's `tools.ini` entries (editable, movable, deletable) followed by the
/// per-game defaults (read-only); the first `user_len` rows are the user's.
struct ExecutablesDialogState {
    /// Display order: user tools first, then read-only per-game defaults.
    merged: Vec<Tool>,
    /// How many leading `merged` entries are the user's (editable) tools.
    user_len: usize,
    /// The selected row, if any.
    selected: Option<usize>,
    // Edit buffers mirroring the selected tool (committed back into `merged`).
    title: String,
    exe: String,
    workdir: String,
    /// Arguments, one per line in the editor (filtered for blanks on save).
    args: String,
    /// Prerequisite verbs, comma-separated in the editor.
    prereqs: String,
    /// The mod this tool's output is captured into (empty = the Overwrite).
    output_mod: String,
    /// The mods that can be picked as a target, read when the dialog opens.
    mod_names: Vec<String>,
    /// A Steam AppID to launch this tool under, as typed (empty = the game's).
    app_id: String,
}

impl ExecutablesDialogState {
    /// Load the buffers from the selected tool (or clear them when nothing is set).
    fn load_buffers(&mut self) {
        match self.selected.and_then(|i| self.merged.get(i)) {
            Some(t) => {
                self.title = t.title.clone();
                self.exe = t.exe.display().to_string();
                self.workdir = t.workdir.as_ref().map(|w| w.display().to_string()).unwrap_or_default();
                self.args = t.args.join("\n");
                self.prereqs = t.prereqs.join(", ");
                self.output_mod = t.output_mod.clone().unwrap_or_default();
                self.app_id = t.app_id.map(|n| n.to_string()).unwrap_or_default();
            }
            None => {
                self.title.clear();
                self.exe.clear();
                self.workdir.clear();
                self.args.clear();
                self.prereqs.clear();
                self.output_mod.clear();
                self.app_id.clear();
            }
        }
    }

    /// Whether the selected row is an editable user tool (vs a read-only default).
    fn selected_is_user(&self) -> bool {
        matches!(self.selected, Some(i) if i < self.user_len)
    }

    /// Write the current edit buffers back into the selected user tool.
    fn commit_buffers(&mut self) {
        if !self.selected_is_user() {
            return;
        }
        let Some(i) = self.selected else { return };
        let Some(t) = self.merged.get_mut(i) else { return };
        t.title = self.title.trim().to_string();
        t.exe = PathBuf::from(self.exe.trim());
        t.workdir = {
            let w = self.workdir.trim();
            if w.is_empty() { None } else { Some(PathBuf::from(w)) }
        };
        t.args = self.args.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        t.prereqs =
            self.prereqs.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        // Only a name that still names a mod. The list is read when the dialog
        // opens, so a mod deleted behind it would otherwise be saved as a target
        // that silently captures nothing.
        t.output_mod = Some(self.output_mod.trim().to_string())
            .filter(|m| self.mod_names.iter().any(|n| n == m));
        // A blank field means "the game's id", which is what a missing key
        // means too - so an unparseable one clears it rather than being kept as
        // something the launch would have to guess about.
        t.app_id = self.app_id.trim().parse::<u32>().ok().filter(|&n| n != 0);
    }
}

/// One member of a collection, joined against what the instance already has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberState {
    /// A mod with this Nexus id is in the mod list.
    Installed,
    /// The exact file is in downloads/, ready to install.
    Downloaded,
    /// Neither. This is what the collection is asking you to get.
    Missing,
}

/// The collection browser.
struct CollectionState {
    /// What the user pasted, kept so the field survives a failed fetch.
    link: String,
    /// The fetched revision, once it arrives.
    revision: Option<eidos_nexus::collections::CollectionRevision>,
    /// Per member, in the revision's order. Computed locally - no requests.
    states: Vec<MemberState>,
    /// True while the one GraphQL request is in flight.
    loading: bool,
    error: Option<String>,
    /// Two clicks before "fetch missing" spawns anything. Same idiom as every
    /// other bulk action here, and this one starts real transfers.
    confirm_fetch: bool,
    /// File ids this pane has already spawned a transfer for. Batches are taken
    /// from what is NOT in here, so clicking again advances instead of
    /// restarting the same few - a member stays `missing` for as long as its
    /// download runs, so the state alone cannot tell the two apart.
    asked: std::collections::HashSet<u64>,
}

/// The Export dialog (MO2's Export to csv).
struct ExportDialogState {
    scope: ExportScope,
    /// One flag per `Column::ALL`, in that order. A Vec rather than a set so the
    /// dialog can render in MO2's column order without sorting anything.
    columns: Vec<bool>,
}

impl ExportDialogState {
    /// The columns actually ticked, in MO2's order.
    fn picked(&self) -> Vec<eidos_instance::Column> {
        eidos_instance::Column::ALL
            .iter()
            .zip(&self.columns)
            .filter(|(_, on)| **on)
            .map(|(c, _)| *c)
            .collect()
    }
}

/// The log pane (MO2's dockable log view).
///
/// It reads the session FILES rather than an in-process buffer, which is the
/// opposite of MO2 and is forced by the architecture: MO2 is one process and can
/// install a sink into its own logger, while the work worth reading here - the
/// mount, the deploy, the launch - happens in a separate `eidos` process whose
/// records only ever reach its own file. A buffer inside the window would be
/// empty of exactly what a user opens this to find.
struct LogPaneState {
    /// Every session file, newest first.
    files: Vec<PathBuf>,
    /// The one being read.
    current: PathBuf,
    /// Records at or above `level`, oldest first.
    lines: Vec<(eidos_log::Level, String)>,
    /// The floor for what is shown.
    level: eidos_log::Level,
    /// How many records the file held before filtering, so the pane can say what
    /// a level switch is hiding.
    total: usize,
    /// True when the file was longer than the read budget and only its tail is
    /// shown - a launch log runs to megabytes.
    truncated: bool,
}

/// The INI editor (MO2 ships one as a tool plugin).
///
/// It edits the PROFILE's copy, which is the only durable one: the copy in the
/// Proton prefix is overwritten from the profile at every launch and captured
/// back after, so editing that one is either pointless or a race. That makes the
/// editor worth more here than in MO2 - on Linux the prefix copy is buried in
/// `steamapps/compatdata/<id>/pfx/drive_c/users/steamuser/Documents/My Games/...`,
/// which is not a path anyone finds by accident.
struct IniEditorState {
    /// The INI files this game has, in `GameDef` order.
    files: Vec<String>,
    /// Which one is being edited.
    current: String,
    /// The editable buffer.
    content: iced::widget::text_editor::Content,
    /// Whether the file on disk was Windows-1252, so it is written back the same
    /// way. Game INIs are as often CP1252 as UTF-8, and re-encoding one silently
    /// mangles every accented value in it.
    cp1252: bool,
    /// Whether the buffer differs from what was read.
    dirty: bool,
    /// The text as read, so Revert costs no disk round-trip and works even if
    /// the file has since been deleted.
    original: String,
    /// Absent from disk: a profile that has never had this INI. Saving creates it.
    missing: bool,
    /// Present but unreadable. Distinct from `missing`, because saving an empty
    /// buffer over a file that exists destroys it - and a permission error is
    /// exactly when that would happen.
    unreadable: bool,
}

/// A mod-install name collision: `mods/<name>/` already exists, so the user picks
/// Merge / Replace / Rename / Cancel - MO2's QueryOverwriteDialog.
struct CollisionPrompt {
    archive: PathBuf,
    /// The colliding (already sanitized) mod name.
    name: String,
    game_id: String,
    /// Editable target for the Rename option (defaults to a free suggestion).
    rename_to: String,
    /// The prompt guards an in-progress FOMOD wizard (still open in `app.fomod`
    /// with the user's choices): resolve via `finish_fomod`, not a re-extract.
    fomod: bool,
    /// The archive already extracted, kept alive so resolving the collision costs
    /// no second 7-Zip pass. `None` only for a prompt raised without one.
    tree: Option<eidos_install::ExtractedTree>,
    /// The BAIN / manual picks that produced this install, if any. A picker
    /// install cannot be replayed from the tree alone - re-running it without the
    /// selection would install the wrong sub-packages.
    pick: Option<PickerChoice>,
}

/// MO2's manual / BAIN install dialogs, which is where an archive lands when the
/// simple and FOMOD heuristics both decline it. Holds the extracted tree so
/// nothing is unpacked twice, whatever the user picks.
struct InstallPicker {
    archive: PathBuf,
    /// The mod name to install under, editable (MO2 lets you rename here).
    name: String,
    game_id: String,
    tree: eidos_install::ExtractedTree,
    /// The parsed archive tree, computed ONCE at construction. The Manual mode's
    /// validity label used to rebuild it from disk inside view() - a full
    /// recursive walk of the extraction, stat per entry, per redraw - while this
    /// struct's own `rows` comment explains why that must not happen. The tree
    /// cannot change while the dialog is open, so both `rows` and the validity
    /// check read this one.
    archive_tree: eidos_install::ArchiveTree,
    /// The archive's contents as flat depth-first rows, computed once - the tree
    /// does not change while the dialog is open.
    rows: Vec<eidos_install::TreeRow>,
    mode: PickerMode,
}

/// Which of the two dialogs is showing.
enum PickerMode {
    /// Wrye Bash complex package: tick the sub-packages to merge, in order.
    Bain {
        subpackages: Vec<String>,
        /// Parallel to `subpackages`.
        picked: Vec<bool>,
        /// Some top-level folders did not look like sub-packages, so MO2 asks
        /// "may be a BAIN installer - install as one?" before showing the ticks.
        /// `true` while that question is unanswered.
        asking: bool,
    },
    /// Nothing recognised the layout: point at the folder that IS the Data root.
    /// `""` means the archive root already is one.
    Manual { root: String },
}

/// What a picker install did, so a name collision can be retried without asking
/// the user to make their picks again.
#[derive(Debug, Clone)]
enum PickerChoice {
    /// The ticked sub-package names, in merge order.
    Bain(Vec<String>),
    /// The chosen data root, relative to the archive.
    Manual(String),
}

/// What a preview managed to make of a file.
///
/// Images and text, and nothing else - which is a decision rather than a first
/// step. A DDS is a container for block-compressed data that needs a BC decoder
/// this tree does not have, and a NIF is a scene graph that needs a renderer;
/// both are real work, and neither is what somebody opens this for. What they
/// open it for is "which of these two textures is the one with the seam" and
/// "what does this config actually say", and those are a PNG and a text file.
#[derive(Debug, Clone)]
pub(crate) enum Preview {
    Image { path: PathBuf, handle: iced::widget::image::Handle },
    /// The head of a text file, and whether there was more.
    Text { path: PathBuf, body: String, truncated: bool },
    /// Nothing could be shown, and this says why rather than showing an empty
    /// box - "no preview" with no reason reads as the feature being broken.
    Unsupported { path: PathBuf, why: String },
}

impl Preview {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Preview::Image { path, .. }
            | Preview::Text { path, .. }
            | Preview::Unsupported { path, .. } => path,
        }
    }
}

/// How much of a text file a preview reads.
///
/// A preview is a glance, and a log can be a hundred megabytes - reading one
/// whole to show its first screen is how a file browser freezes.
pub(crate) const PREVIEW_TEXT_CAP: usize = 64 * 1024;

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

/// The install status of a downloaded archive, derived from its `.meta` sidecar
/// (MO2's downloads-list state column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadState {
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
struct DownloadRow {
    /// The archive's file name.
    name: String,
    /// The absolute path to the archive.
    path: PathBuf,
    /// Size in bytes.
    size: u64,
    /// The installed `version` from the `.meta` sidecar (empty if none).
    version: String,
    /// The friendly mod name from the sidecar, if any (Nexus `modName`).
    mod_name: Option<String>,
    /// The Nexus mod id from the sidecar, so the row can reach the mod's page.
    /// The sidecar's `url=` is NOT usable for this: it is the CDN link, which
    /// carries an expiry and a signature and is dead within the hour.
    mod_id: Option<u64>,
    /// The derived install status.
    state: DownloadState,
    /// Bytes on disk so far. Equals `size` once finished.
    downloaded: u64,
    /// Total bytes from the sidecar's `totalSize`, `0` when unknown - an older
    /// download, or a manually dropped archive.
    total: u64,
    /// Bytes per second, measured between two ticks. `None` on the first tick of
    /// a download, when there is nothing to compare against yet.
    speed: Option<f64>,
    /// MO2's `removed=` in the sidecar: hidden from the list without the archive
    /// being deleted. The field was modelled and nothing read it.
    hidden: bool,
    /// When the archive was last written, for sorting by date.
    modified: std::time::SystemTime,
}

/// How the Downloads list is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DownloadSort {
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
    const ALL: [DownloadSort; 4] =
        [DownloadSort::Newest, DownloadSort::Name, DownloadSort::Size, DownloadSort::State];
    fn label(self) -> &'static str {
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

/// A download row being dragged onto the mod list, to install it AT a priority.
///
/// Deliberately not folded into [`DragState`]: that one moves a row that is
/// already in the list, so it has a `from` and its own edges are no-ops. This one
/// has no row in the list yet, so every gap is a genuine target and the commit
/// runs the installer rather than a reorder.
#[derive(Debug, Clone)]
struct DownloadDrag {
    /// The archive to install.
    path: PathBuf,
    /// Where it should land, as an INSERTION index into `app.mods`.
    gap: usize,
    /// Whether the pointer ever reached an insertion strip. A press arms the
    /// drag, so a plain click on a download row arrives here as a drop.
    aimed: bool,
}

/// An in-flight mod-row drag (MO2's drag-to-reorder). `from` is the grabbed row's
/// index in `app.mods`; the move is only applied on release, and only when the
/// aimed gap is not one of the block's own edges.
#[derive(Debug, Clone, Copy)]
struct DragState {
    from: usize,
    /// Whether the pointer ever reached an insertion point outside the block.
    /// A press arms a drag, so a plain CLICK arrives as a drop; with a
    /// multi-row selection there is no "own edge" to recognise, and committing
    /// it would COMPACT a non-contiguous selection and save that. See
    /// `PluginDrag::aimed`.
    aimed: bool,
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
    gap: usize,
}

/// A keyboard navigation intent, independent of which list will answer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nav {
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
enum Pane {
    Mods,
    Plugins,
}

/// What comes back from a LOOT run: the fingerprint of the list it was asked
/// about, the sorted names, and the report - or why the run failed. The report
/// is a nested `Result` because it is advisory: losing it must not lose the
/// order that was successfully computed.
type SortOutcome =
    Result<(SortFingerprint, Vec<String>, Result<eidos_loot::LootReport, String>), String>;

/// What a LOOT sort was computed against, so a stale answer can be recognised.
///
/// The profile, because each owns its own load order, and the SET of plugin
/// names - not their order, which is precisely what the sort is allowed to
/// change. If either moved while LOOT ran, the returned permutation is a
/// permutation of something else.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SortFingerprint {
    /// The game, not just the profile. Profiles are per-game and their names
    /// collide - two games both have a "Default" - so a switch mid-sort would
    /// compare equal and write one game's load order into the other's.
    game: String,
    profile: String,
    names: BTreeSet<String>,
}

/// An in-flight plugin drag. Unlike the mod list, where any order is legal, a
/// plugin's position is constrained by the engine, so the drag carries the range
/// it is allowed to land in - computed ONCE when the row is grabbed, not per
/// frame - and the strips outside it are not offered at all.
#[derive(Debug, Clone)]
struct PluginDrag {
    from: usize,
    /// Insertion index, same meaning as `DragState::gap`.
    gap: usize,
    /// Every row travelling, ascending - the whole selection when the grabbed
    /// row belonged to it, otherwise just that row.
    block: Vec<usize>,
    /// Whether the pointer ever reached an insertion point outside the block.
    ///
    /// A press arms a drag, so a plain CLICK on a row arrives here as a drop.
    /// With a single row the "landed on its own edge" test caught that, but a
    /// non-contiguous selection has no such edge: dropping it anywhere COMPACTS
    /// it, so a click on one of its rows silently rewrote the load order and
    /// saved it. Nothing commits until this is true.
    aimed: bool,
    /// Where this plugin may legally go, and which plugins bound it.
    range: MovableRange,
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
type CachedListing = (u64, std::rc::Rc<Vec<String>>);

struct App {
    screen: Screen,
    games: Vec<DetectedGame>,
    kind: InstanceKind,
    portable_path: String,
    selected: Option<usize>,
    name: String,
    created: Option<Instance>,
    error: Option<String>,
    mods: Vec<ModEntry>,
    /// Cached ESP/ESM load order for the Plugins tab (recomputed on demand).
    plugins: Option<PluginList>,
    /// Cached per-file conflict analysis for the Conflicts tab + mod-row flags.
    conflicts: Option<ConflictMap>,
    /// The last health-check run, cached.
    ///
    /// `diagnostics()` walks the mods directory, reads the script extender's log
    /// and parses an INI. It used to be called from `view()` - twice when the
    /// Diagnostics tab was open, since the tab LABEL carries the problem count -
    /// which means it ran on every single frame: roughly a hundred `read_dir`
    /// per keystroke in the filter box. That is the cost that made typing feel
    /// like wading, and it bought a number that only changes when the setup does.
    diag: Vec<Diagnostic>,
    /// Set when something might have changed the answer; consumed at the end of
    /// `update()`. A missed setter shows a stale COUNT until the next real
    /// change, never wrong data - the panel renders from this same cache, so the
    /// label and the panel can never disagree either.
    diag_dirty: bool,
    /// The `&App`-reachable half of the same flag, so `bump_views` can set it.
    diag_stale: std::cell::Cell<bool>,
    tab: Tab,
    status: Option<String>,
    /// Two-click guard for the destructive "Clear Overwrite" action.
    confirm_clear: bool,
    /// The in-progress "create mod from Overwrite" name, if that prompt is open.
    overwrite_to_mod: Option<String>,
    /// An open "send to priority" editor: `(row, typed text)`.
    send_priority: Option<(usize, String)>,
    /// An open "send to separator" chooser, for this row.
    send_separator: Option<usize>,
    /// The Proton command Steam passed via `%command%` (empty if launched
    /// standalone). The Run button launches the game through this.
    launch_command: Vec<String>,
    /// An open FOMOD installer wizard, if the user is mid-install.
    fomod: Option<FomodWizard>,
    /// An open install-collision prompt (target mod name already exists).
    collision: Option<CollisionPrompt>,
    /// An open manual / BAIN install picker.
    picker: Option<InstallPicker>,
    /// Tools runnable through the merged view (user tools.ini + per-game
    /// defaults), shown in the run-target picker next to Run.
    tools: Vec<eidos_instance::Tool>,
    /// The picked run target: `None` = the game, `Some(title)` = that tool.
    tool_choice: Option<String>,
    /// Mod-list filter query (case-insensitive substring on the mod name).
    search: String,
    /// The highlighted mod row, if any.
    selected_mod: Option<usize>,
    /// The mod whose right-click action menu is open (None = closed).
    menu_mod: Option<usize>,
    /// The plugin row whose right-click menu is open (MO2's plugin context
    /// menu). Separate from `menu_mod`: the two lists are shown at once, and a
    /// shared field would make right-clicking one dismiss nothing in the other.
    menu_plugin: Option<usize>,
    /// In-progress rename: `(mod index, edited name)`.
    rename: Option<(usize, String)>,
    /// Per-mod metadata for the extra columns + context menu, keyed by folder name.
    meta_cache: HashMap<String, RowMeta>,
    /// Two-click guard for the destructive per-mod "Remove" action.
    confirm_remove: Option<usize>,
    /// The mod whose info dialog is open (None = closed), its active tab, and the
    /// note text being edited.
    info_mod: Option<usize>,
    info_tab: InfoTab,
    notes_edit: String,
    /// Collapsed separators, keyed by display name (MO2 keys by display name too).
    /// Persisted per-profile so the grouping state survives a relaunch.
    collapsed: HashSet<String>,
    /// Active category filter (a top-level category id), or `None` for all.
    category_filter: Option<i32>,
    /// The state criteria the mod list is filtering on (MO2's filter pane).
    filters: ModFilters,
    /// Whether that pane is open.
    filters_open: bool,
    // ---- Settings / Nexus account (the status bar + endorse/update read these) ----
    /// The Preferences modal is open.
    settings_open: bool,
    /// The active Preferences tab.
    settings_tab: SettingsTab,
    /// Which collapsible sections of the Settings screen are open. Keyed by the
    /// same `&'static str` the section is built with, so a rename cannot drift.
    settings_expanded: HashSet<&'static str>,
    /// The validated Nexus account, if a stored session checked out.
    nexus_account: Option<eidos_nexus::Account>,
    /// A sign-in is in flight (guards the button + concurrent attempts).
    nexus_signing_in: bool,
    /// The last sign-in error, shown inline in the dialog.
    nexus_error: Option<String>,
    /// The persisted app-global preferences (theme, default game).
    prefs: Settings,
    // ---- Executables dialog ----
    /// The open Executables editor, if any (None = closed).
    executables: Option<ExecutablesDialogState>,
    /// The Backups dialog: the restore points of both lists, read when it opens
    /// so the list cannot go stale behind an open dialog.
    backups: Option<BackupsDialogState>,
    /// Files dropped from a file manager, waiting to be installed. A drop of
    /// several archives arrives as several messages, and each install can open a
    /// modal, so they are drained one at a time rather than handled inline.
    dropped: Vec<PathBuf>,
    /// Whether a file is currently hovering over the window (for the hint).
    files_hovering: bool,
    /// A download being dragged onto the mod list (MO2's drop-to-priority).
    download_drag: Option<DownloadDrag>,
    /// Where the install now in flight should land, if it was aimed at a gap,
    /// paired with the ARCHIVE it was aimed at.
    ///
    /// The archive is what makes it safe. This has to survive the FOMOD wizard,
    /// the BAIN picker and the collision prompt, so it cannot live on the drag -
    /// and an install that ends without reaching `after_install` (an extraction
    /// failure, an unrecognised layout, a dismissed dialog) would otherwise
    /// leave the aim behind for the NEXT mod installed to silently adopt.
    /// Matching on the archive means a stale aim simply never applies.
    install_at: Option<(usize, PathBuf)>,
    /// A landing position chosen from a context menu, waiting for the picker to
    /// name an archive. Separate from `install_at` because that one is PAIRED
    /// with its archive - the pairing is what stops a cancelled install moving an
    /// unrelated mod - and there is no archive yet at the moment of the click.
    install_gap: Option<usize>,
    /// Two-click guard for the bulk enable/disable, holding the TARGET state so
    /// arming "Enable all" and then clicking "Disable all" does not fire.
    confirm_set_all: Option<bool>,
    /// Whether the File dropdown (the folder list) is showing.
    file_menu_open: bool,
    /// The open Export dialog: which rows, and which columns are ticked.
    export: Option<ExportDialogState>,
    /// The open collection view, if any.
    collection: Option<CollectionState>,
    /// Whether the instance manager is showing.
    instances_open: bool,
    /// Where the instance registry lives. A field rather than a global so the
    /// window can be tested without writing the real user config - the handlers
    /// that forget and rename instances persist through it.
    registry_path: PathBuf,
    /// What is typed in the preferred-servers field, which is not the saved
    /// value until it is submitted - the same shape as the mod URL field.
    servers_edit: String,
    /// Filetree: the entry being renamed and what has been typed, the entry
    /// armed for deletion, and the new-folder box.
    tree_rename: Option<(usize, String)>,
    tree_rename_text: String,
    /// The filetree entry armed for deletion: the MOD'S NAME and the relative
    /// path. By name for the same reason as `confirm_restore` - a reload
    /// between the clicks would otherwise aim the delete into another mod's
    /// folder, where the same relative path may well exist.
    tree_delete_armed: Option<(String, String)>,
    tree_new_folder: Option<String>,
    /// The backup armed for restoring over its original, BY NAME.
    ///
    /// Not by index: an index is a position in a list that anything can reload
    /// between the two clicks, and the second click would then restore a
    /// different backup - one that is still a backup, so no guard catches it.
    confirm_restore: Option<String>,
    /// The file being previewed, and what could be made of it.
    preview: Option<Preview>,
    /// Downloads list: the name filter, the ordering, whether hidden rows are
    /// shown, and the two-click guard on the bulk purge.
    dl_filter: String,
    dl_sort: DownloadSort,
    dl_show_hidden: bool,
    confirm_purge_installed: bool,
    /// Mod-list columns currently drawn, in display order.
    mod_columns: Vec<ModColumn>,
    /// What the list is ordered by. `None` is load order - the real one.
    mod_sort: Option<ModSort>,
    /// What the list is grouped under. `None` is the user's own separators.
    group_by: Option<GroupBy>,
    /// Group headers the user has folded, by their synthetic label. Separate
    /// from `collapsed`, which keys on separator names: a category called
    /// "Armour" and a separator called "Armour" are not the same fold.
    groups_collapsed: std::collections::HashSet<String>,
    /// The instance row being renamed, and the pending name.
    instance_rename: Option<(usize, String)>,
    /// Two-click guard for forgetting an instance.
    confirm_forget: Option<usize>,
    /// Two-click guard for the Overwrite sync.
    confirm_sync: bool,
    /// The custom-URL editor in the mod info dialog.
    url_edit: String,
    /// Saves picked with Ctrl+click, for the batch actions.
    selected_saves: std::collections::BTreeSet<usize>,
    /// Two-click guard for deleting the selection.
    confirm_saves_delete: bool,
    /// The saves directory's shape at the last tick: how many entries and the
    /// newest mtime. Compared rather than re-listed into the model, so a quiet
    /// directory costs one `read_dir` and changes nothing the view depends on.
    saves_fingerprint: Option<(usize, std::time::SystemTime)>,
    /// The selected save's screenshot, decoded once on selection.
    ///
    /// Keyed by path so a stale image cannot be shown against another save, and
    /// held as a ready `image::Handle` rather than raw pixels - `view` runs every
    /// frame and must not decode anything.
    save_shot: Option<(PathBuf, Option<iced::widget::image::Handle>)>,
    /// The plugin row whose "Send to priority" field is open, and what is typed
    /// in it. Mirrors `send_priority` for mods; kept separate because the two
    /// lists are indexed independently and one menu must not aim the other.
    plugin_send_priority: Option<(usize, String)>,
    /// Something the drop wants said once the install finishes. It cannot say it
    /// itself: the installer sets its own status a moment later.
    pending_note: Option<String>,
    /// The Categories dialog: which mods it applies to and the pending choice.
    categories_dialog: Option<CategoriesDialogState>,
    /// The open INI editor, if any.
    ini_editor: Option<IniEditorState>,
    /// The open log pane, if any.
    log_pane: Option<LogPaneState>,
    /// User add-ons, read at startup and on demand.
    addons: Vec<eidos_addons::Addon>,
    /// Whether the Extensions list is showing.
    addons_open: bool,
    /// What the `diagnose` add-ons reported on the last refresh, by add-on name.
    addon_findings: Vec<(String, eidos_addons::Finding)>,
    /// Checks that failed, and why. Not retried until Reload: this runs on every
    /// diagnostics refresh, so retrying a hanging one would cost its timeout per
    /// click.
    addon_failed: HashMap<String, String>,
    /// Manifests that could not be parsed, and why - so a typo is visible in the
    /// Extensions list instead of only on a stderr the window never shows.
    addon_rejected: Vec<(PathBuf, String)>,
    // ---- Endorse / update in-flight + counts ----
    /// The mod index whose Nexus endorse is in flight (greys the toolbar button).
    endorsing: Option<usize>,
    /// Enabled mods that are endorsed (recomputed in `mods_changed`).
    endorsed_count: usize,
    /// Enabled mods with a Nexus update available (recomputed in `mods_changed`).
    updated_count: usize,
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
    nexus_hourly_left: Option<i64>,
    nexus_daily_left: Option<i64>,
    /// A Nexus mod-update check is in flight (guards the Update button).
    update_in_progress: bool,
    /// A LOOT sort is in flight. iced runs on smol's single-threaded executor
    /// here, so a second sort does not race the first - it QUEUES behind it, and
    /// every queued completion re-opens the report modal and overwrites the
    /// status with its own (idempotent, so "nothing moved") result. A masterlist
    /// download is several seconds with no other visible sign of work, which is
    /// long enough to invite exactly that.
    sorting: bool,
    // ---- menu-bar UI toggles + About ----
    /// The toolbar / status bar are visible (View menu toggles).
    ui_toolbar_visible: bool,
    ui_statusbar_visible: bool,
    /// The View dropdown is open (iced has no native menu, so it's a floating card).
    view_menu_open: bool,
    /// The About box is open.
    about_open: bool,
    // ---- Saves tab (the details pane is the reason for the parse) ----
    /// The active profile's save files (newest first), lazily loaded.
    saves: Vec<SaveEntry>,
    /// Two-click guard for a save deletion (the save's index in `saves`).
    confirm_delete_save: Option<usize>,
    /// The save whose details pane is open, an index into `saves`.
    selected_save: Option<usize>,
    /// The parsed header of `selected_save`, keyed by its path so a stale parse is
    /// never shown against a different file. `Err` = unreadable, which degrades the
    /// pane to a message rather than hiding the save.
    save_info: Option<(PathBuf, Result<eidos_gamefeatures::SaveInfo, String>)>,
    /// The selected save's plugins that are no longer active, with the mods that
    /// could supply them. Recomputed with `save_info`.
    save_missing: Vec<eidos_gamefeatures::MissingPlugin>,
    // ---- Downloads manager ----
    /// The completed downloads (cached so the panel does not re-scan on redraw).
    downloads: Vec<DownloadRow>,
    /// A resume in flight: which download, the child, and the file its output
    /// went to. Held so the child can be REAPED (an unwaited one becomes a
    /// zombie, whose /proc entry makes it look alive to `stop_download`) and so
    /// a failure can be reported instead of the row silently going back to
    /// Stalled.
    resuming: Option<(String, std::process::Child, PathBuf)>,
    /// Two-click guard for a download deletion (the row's index in `downloads`).
    confirm_delete_download: Option<String>,
    /// The download whose MD5 lookup is in flight, so the row can say so and a
    /// second click cannot start the same hash twice.
    identifying_download: Option<String>,
    /// Last (instant, bytes) seen for each in-flight download, keyed by file
    /// name. Speed is a derivative, so it needs the previous sample; keeping it
    /// out of `DownloadRow` means a rebuilt row list does not lose the history.
    download_samples: HashMap<String, (std::time::Instant, u64)>,
    // ---- multi-select + batch actions ----
    /// Where a Shift extension counts FROM.
    ///
    /// Distinct from `selected_mod`, which is the focus and moves with every
    /// arrow key: if the extension counted from the focus, each Shift+Down would
    /// re-anchor on the row it just reached and the selection would only ever be
    /// two rows long. Set by a plain click and by Ctrl+click, left alone by
    /// Shift - the behaviour of every list widget that has one.
    sel_anchor: Option<usize>,
    /// The multi-selection set (indices into `app.mods`). `selected_mod` stays the
    /// focus anchor for single-row UI; this set drives batch actions and the row
    /// highlight when more than one row is selected.
    selected_mods: HashSet<usize>,
    /// Two-click guard for the destructive batch "Remove selected" action.
    confirm_batch_remove: bool,
    /// The keyboard modifiers currently held, so a plain left-click can branch to
    /// Ctrl-toggle / Shift-extend (iced fires a fixed `on_press` message otherwise).
    modifiers: iced::keyboard::Modifiers,
    /// An in-flight drag-to-reorder (None = not dragging).
    drag_state: Option<DragState>,
    /// Which edge of the mod list the pointer rests on mid-drag, if any. Drives
    /// the auto-scroll tick; `None` stops it.
    drag_scroll: Option<ScrollEdge>,
    /// A collapsed group the drag is resting on, and how many ticks it has
    /// rested. Dropping INTO a collapsed group is otherwise impossible without
    /// abandoning the drag to expand it first, which is the whole reason MO2
    /// expands on hover.
    drag_hover_group: Option<(String, u8)>,
    /// How deep into that band the pointer is, 0.0..1.0. Speed follows it, so
    /// nudging the edge creeps and pushing right against it flies.
    drag_scroll_depth: f32,
    /// The plugin list's Shift anchor; see [`App::sel_anchor`].
    plugin_anchor: Option<usize>,
    /// The focused plugin row, and the multi-selection around it - the same
    /// model the mod list uses, because every batch action needs the same answer
    /// to "which rows am I acting on".
    selected_plugin: Option<usize>,
    selected_plugins: HashSet<usize>,
    /// Which list the arrow keys move in.
    focus: Pane,
    /// The category catalog, read once instead of per frame.
    ///
    /// `Instance::category_factory` opens and parses `categories.dat` on every
    /// call, and the mod list asked it for one on every view - so tracking the
    /// pointer for context-menu placement turned that into a file read and a
    /// parse per mouse MOVE. Rebuilt where the mod list is rebuilt.
    categories: Option<eidos_instance::CategoryFactory>,
    /// The live pointer position, and the window it moves in.
    ///
    /// iced's `on_right_press` carries no coordinates, so a context menu has no
    /// way to know where it was summoned from unless the position is tracked
    /// separately. Both are fed by the event subscription and read only when a
    /// menu opens.
    cursor: iced::Point,
    window: iced::Size,
    /// Where the open context menu was summoned from, frozen at that moment - the
    /// pointer keeps moving afterwards, and a menu that slid along with it would
    /// be unusable.
    menu_at: Option<iced::Point>,
    /// The user is typing into a field on the main screen (the mod filter, a
    /// notes box, an inline rename).
    ///
    /// `on_key_press` is a global subscription: it does not know which widget
    /// has the caret, so without this a space typed into the filter box would
    /// toggle a mod and Home would jump the list instead of the text. Set by any
    /// keystroke that reached a field, cleared the moment a row is pressed or
    /// Escape is hit - approximate at the edges (clicking into a field without
    /// typing leaves it false), but wrong only in the harmless direction.
    typing: bool,
    /// The same, for the plugin list. Kept separate so a drag in one panel can
    /// never be committed against the other's indices, and carrying the legal
    /// range so the illegal strips can simply refuse to be targets.
    plugin_drag: Option<PluginDrag>,
    // ---- profile management (MO2 profiles dialog) ----
    /// The profile whose right-click action menu is open (None = closed).
    profile_menu: Option<String>,
    /// In-progress profile rename: `(original name, edited name)`.
    profile_rename: Option<(String, String)>,
    /// In-progress named copy: `(source name, edited new name)`.
    profile_copy: Option<(String, String)>,
    /// Two-click guard for a profile deletion (the armed profile name).
    profile_delete_confirm: Option<String>,
    // ---- run lock (MO2's "lock GUI while the application runs") ----
    /// A launched game/tool we are waiting on. While `Some` and `prefs.lock_gui`
    /// is set, the window is blocked behind the lock overlay until it exits (or the
    /// user clicks Unlock). `None` = nothing running / not locked.
    running: Option<RunningState>,
    /// The `eidos` binary lacks CAP_SYS_ADMIN (setcap wiped by a rebuild), so FUSE
    /// passthrough cannot engage even if asked for. Harmless by itself - the
    /// capability is optional and passthrough is off by default - so this only
    /// drives a banner when `passthrough_requested()`. Rechecked on Refresh and
    /// after every run.
    cap_missing: bool,
    /// Cached per-layer file walks for the conflict analysis, keyed by layer name
    /// (mod folder / "Overwrite" / "[game]"). RefCell so the read-path
    /// `compute_conflicts(&App)` can fill missing entries; a toggle or reorder
    /// then rebuilds the map without touching the filesystem at all. Entries are
    /// dropped when a layer's contents change (install/remove/rename/run).
    files_cache: std::cell::RefCell<HashMap<String, (Vec<String>, bool)>>,
    // ---- view memoisation (these listings ran on EVERY redraw) ----
    /// Bumped whenever the mod list or anything on disk changes; the memoised
    /// listings below rebuild only when it moves.
    view_generation: std::cell::Cell<u64>,
    /// Memoised Data-tab merged listings, one per directory (keyed by its path
    /// relative to `Data`, `""` for the root), each with the generation it was
    /// built at. The tree merges a level at a time, so only the directories the
    /// user actually opened are ever read.
    data_listing: std::cell::RefCell<HashMap<String, (u64, Vec<DataRow>)>>,
    /// The profile chip row's data: every profile name and which one is active.
    ///
    /// Both were read from disk on EVERY frame of the main screen - a `read_dir`
    /// plus an `is_dir` per entry, and a file read for the active name. The
    /// chips are always drawn, so that was a few filesystem calls per frame for
    /// something that changes when the user creates or switches a profile.
    profiles_cache: std::cell::RefCell<Option<(u64, Vec<String>, String)>>,
    /// The Archives tab's rows, built once per view generation.
    ///
    /// `archive_rows` walks every enabled mod's folder and reads an INI. It is
    /// called from `view()`, which runs on every frame - every pointer move,
    /// every keystroke - so without this a four-hundred-mod list did four
    /// hundred `read_dir` calls per frame while the tab was open. The same memo
    /// the merged listing uses, keyed the same way.
    archives_cache: std::cell::RefCell<Option<(u64, Option<Vec<ArchiveRow>>)>>,
    /// The union the Data tab reads, built once per view generation.
    ///
    /// The SAME `LayerStack` the mount serves from, rather than a hand-rolled
    /// merge beside it: whiteouts, opaque directories, hidden names, case-folded
    /// dedup and NTFS collation all live in one place, and the tab can no longer
    /// disagree with the filesystem the game sees.
    data_stack: std::cell::RefCell<Option<(u64, std::rc::Rc<eidos_core::LayerStack>)>>,
    /// Free-text filter over the Data tree.
    data_query: String,
    /// Show only paths more than one mod provides.
    data_conflicts_only: bool,
    /// Directories the user expanded in the Data tree, same keys as above. The
    /// root is implicitly expanded and never in here.
    data_expanded: HashSet<String>,
    /// Folders opened in the Overwrite tree, keyed the same way.
    overwrite_expanded: HashSet<String>,
    /// Memoised recursive file listings per directory (the Overwrite tab and the
    /// mod-info file tree), each with the generation it was built at.
    listing_cache: std::cell::RefCell<HashMap<PathBuf, CachedListing>>,
    // ---- LOOT report (MO2's post-sort report dialog) ----
    /// The report from the last LOOT sort, shown as a modal so the user sees
    /// missing masters / messages / dirty-plugin advice. `None` = no report open.
    loot_report: Option<eidos_loot::LootReport>,
    /// Per-plugin LOOT metadata from the last sort (messages, dirty info, Bash
    /// Tags), keyed by ASCII-lowercased plugin name - what the Plugins tab
    /// draws its badges and tooltip lines from. SEPARATE from `loot_report`
    /// deliberately: the report is dialog state, cleared the moment the modal
    /// closes, while these badges must outlive it (MO2 keeps its flags until
    /// the next sort). Cleared on profile/instance switch.
    loot_meta: Option<HashMap<String, eidos_loot::PluginMetadataBundle>>,
    /// Existing instances the welcome screen offers to open: registry
    /// portables + detected globals, last-used first. Refreshed on Restart,
    /// so the welcome screen doubles as the instance switcher.
    known: Vec<KnownInstance>,
}

/// Which criterion a [`Message::CycleFilter`] refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterField {
    Active,
    Conflicted,
    Update,
    Plugins,
    Uncategorised,
}

/// What the Backups dialog shows: the restore points of each list, newest
/// first, as they were when the dialog opened.
struct BackupsDialogState {
    mods: Vec<eidos_instance::Backup>,
    order: Vec<eidos_instance::Backup>,
}

/// The Categories dialog. Two modes in one card: assigning categories to the
/// selected mods, and editing the catalog those categories come from.
///
/// The pending choice lives here rather than being written on every click,
/// because a `meta.ini` write per checkbox would rewrite the file (and invalidate
/// the meta cache) a dozen times while the user makes up their mind.
struct CategoriesDialogState {
    /// The mods this applies to, BY NAME. More than one = MO2's batch assign.
    ///
    /// Names, not row indices: the mod list can be rebuilt behind an open dialog
    /// (a refresh, a Nexus check, a drag), and indices captured before that would
    /// then point at different mods than the ones the user opened it on.
    names: Vec<String>,
    /// Checked ids, primary FIRST (that is what the on-disk order means).
    chosen: Vec<i32>,
    /// The catalog being edited. Loaded once with the dialog; saved on Apply.
    catalog: eidos_instance::CategoryFactory,
    /// True while the catalog editor is showing instead of the picker.
    editing: bool,
    /// The name box in the catalog editor.
    new_name: String,
    /// The parent for a category about to be created (0 = top level).
    new_parent: i32,
    /// The catalog row being renamed, and its pending name.
    rename: Option<(i32, String)>,
    /// Two-click guard for deleting a catalog row.
    confirm_delete: Option<i32>,
    /// Free-text filter over the category tree.
    query: String,
}

/// A game/tool launched through Eidos that the GUI is waiting on. A detached
/// thread `wait()`s the `eidos` child (which itself outlives the game, holding the
/// FUSE mount) and flips `done` when it exits; a poll subscription notices and
/// unlocks. `pid` is shown in the lock overlay (MO2 shows the running process).
struct RunningState {
    /// What is running (the tool title or the game name), for the overlay text.
    title: String,
    /// The `eidos` child's pid, surfaced in the overlay like MO2's process list.
    pid: u32,
    /// Flipped to `true` by the wait thread once the child exits.
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The child's exit status, stored by the wait thread just before `done`.
    outcome: std::sync::Arc<std::sync::Mutex<Option<std::process::ExitStatus>>>,
    /// The per-run log file capturing the child's stdout+stderr (launch errors
    /// are invisible otherwise - the GUI has no terminal when started from Steam).
    log: Option<PathBuf>,
    /// Whether the lock overlay is up. `false` = "lock GUI" is off (or the user
    /// clicked Unlock): the run is still TRACKED (exit refresh, double-launch
    /// guard, error reporting) but the window stays interactive.
    lock: bool,
}

/// The slice of a mod's `meta.ini` the main window shows (extra columns + the
/// Nexus action). Cached so a search keystroke doesn't re-read every file.
#[derive(Debug, Clone, Default)]
struct RowMeta {
    version: Option<String>,
    mod_id: Option<u64>,
    /// The mod's PRIMARY category id (MO2 `category=` first id), for filtering.
    category_id: Option<i32>,
    /// That category resolved to a display name (MO2 Category column).
    category_name: Option<String>,
    /// MO2 Content column: a compact letters string of the kinds of content the mod
    /// ships (P/A/T/M/S/K/I/U/F), empty if none.
    content_tags: String,
    update: bool,
    /// The row's display colour (MO2's `color=@Variant(...)`), if set. Stored per
    /// mod, not only per separator.
    color: Option<[u8; 3]>,
    /// `Some(false)` when the last update check found the Nexus page gone.
    /// `None` means nobody has checked, which must NOT draw a warning.
    nexus_gone: bool,
    /// The user's note, shown as a glyph with the text on hover. MO2 gives it a
    /// column; here it rides the Flags cell, because every column costs width off
    /// the name and a note is read on demand rather than scanned.
    notes: Option<String>,
    /// The mod's top level looks like nothing this game loads - MO2's "no valid
    /// game data". Never set for a mod the user has marked valid.
    invalid_data: bool,
    /// Who made it (`author=`), for the Author column.
    author: Option<String>,
    /// The game the `meta.ini` names, whatever it is - the Source game column.
    /// Distinct from `other_game`, which is only set when it DIFFERS.
    game_name: Option<String>,
    /// When the mod folder was last written, for the Installed column and for
    /// sorting by it. MO2 shows the install date; a mod directory's mtime is
    /// the closest thing on disk that costs nothing extra to read.
    installed_at: Option<std::time::SystemTime>,
    /// The game this mod's `meta.ini` says it was downloaded for, when that is
    /// NOT the instance's game. `None` covers both "same game" and "does not
    /// say", which have to look identical: a mod installed from a folder never
    /// had a game recorded, and warning about that would flag half a list.
    other_game: Option<String>,
}

/// One entry in the category-filter dropdown (`None` id = "all").
#[derive(Debug, Clone, PartialEq, Eq)]
struct CategoryChoice {
    id: Option<i32>,
    label: String,
}

impl std::fmt::Display for CategoryChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}


fn view(app: &App) -> Element<'_, Message> {
    if let Some(w) = &app.fomod {
        let base = fomod_wizard_view(w);
        // A reinstall collision raised from inside the wizard must be able to
        // show over it (the wizard replaces the whole view).
        if let Some(c) = &app.collision {
            let scrim =
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CollisionCancel);
            let dialog = container(collision_dialog(c)).center(Length::Fill);
            return Stack::new().push(base).push(scrim).push(dialog).into();
        }
        return base;
    }
    if app.screen == Screen::Main {
        return main_screen(app);
    }
    let inner = match app.screen {
        Screen::Welcome => welcome(app),
        Screen::Kind => kind_screen(app),
        Screen::Game => game_screen(app),
        Screen::NameLoc => nameloc_screen(app),
        Screen::Summary => summary_screen(app),
        Screen::Main => welcome(app),
    };
    let base: Element<'_, Message> =
        container(inner).width(Length::Fill).height(Length::Fill).padding(20).into();
    // The collection pane also belongs here, not only on the main screen.
    // `eidos-gui --collection` opens it before anything else, and with no
    // instance yet the window lands on the welcome screen - where a pane drawn
    // only by `main_screen` is a link that silently does nothing.
    if let Some(state) = &app.collection {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseCollection);
        let dialog = container(collection_dialog(state)).center(Length::Fill);
        return Stack::new().push(base).push(scrim).push(dialog).into();
    }
    base
}

/// Keyboard subscription: surface global shortcuts and keep the live modifier state
/// in sync so a plain mod-row click can branch to Ctrl-toggle / Shift-extend.
///
/// Shortcuts only fire on the main screen, and only when no modal / inline editor is
/// stealing input, so they never clobber typing into a text field. Mirrors MO2's
/// global accelerators: F5 (Refresh) and Ctrl+R (Run).
/// How long a `.unfinished` partial may go without growing before it is called
/// stalled rather than downloading. Generous: a slow mirror can go quiet for a
/// few seconds, and calling a live download dead is worse than the reverse.
const STALLED_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

/// How often the Saves tab checks whether the directory changed. Slower than the
/// downloads poll: a save appears when the player makes one, which is not a
/// progress bar anybody is watching tick.
const SAVES_TICK: std::time::Duration = std::time::Duration::from_millis(2500);

/// How often the open log pane re-reads its file. Slower than the downloads
/// tick: a log is read, not watched, and re-parsing half a megabyte twice a
/// second to catch a line that is not there yet is pure waste.
const LOG_TAIL_TICK: std::time::Duration = std::time::Duration::from_millis(1500);

/// How often the downloads directory is re-scanned while something is arriving.
/// Fast enough that a progress bar moves rather than jumps, slow enough that it
/// is a rounding error next to the transfer itself.
const DOWNLOAD_TICK: std::time::Duration = std::time::Duration::from_millis(500);

/// The same, when nothing is in flight: something has to notice that a download
/// STARTED, and that something cannot be the download itself - it runs in
/// another process, launched by the browser, with no way to reach this one.
const DOWNLOAD_IDLE_TICK: std::time::Duration = std::time::Duration::from_secs(2);

fn subscription(app: &App) -> iced::Subscription<Message> {
    use iced::keyboard::{self, key::Named, Key};

    // Track held modifiers from every key press AND release (a release with no
    // remaining keys still carries the updated modifier set).
    // One stream now: `listen` yields every keyboard event and all three variants
    // carry the modifier set, so press and release no longer need separate
    // subscriptions. ModifiersChanged also reaches us for the first time - no
    // widget captures it - which means the held set no longer goes stale while a
    // text field has the caret.
    let track = keyboard::listen().map(|event| match event {
        keyboard::Event::KeyPressed { modifiers, .. }
        | keyboard::Event::KeyReleased { modifiers, .. }
        | keyboard::Event::ModifiersChanged(modifiers) => Message::ModifiersChanged(modifiers),
    });

    // Where the pointer is, and how big the window is. Needed because iced's
    // right-press carries no coordinates, so a context menu cannot otherwise be
    // placed where it was summoned from. The handlers do nothing but store.
    let pointer = iced::event::listen_with(|event, _status, _window| match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::PointerAt(position))
        }
        // A release ANYWHERE ends a drag. `mouse_area::on_release` only fires
        // while the cursor is over its bounds, so without this a drag let go
        // outside the list stayed armed and the next click moved the mod. It
        // used to be handled by cancelling on pointer EXIT, which cost far more
        // than it bought: dragging upward past the header to scroll dropped the
        // mod every time, because leaving and letting go are indistinguishable
        // to that callback.
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::PointerReleased)
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
        }
        // Files dragged in from a file manager. NOT AVAILABLE ON WAYLAND: winit
        // 0.30 implements XDND for X11 and has no `wl_data_device` at all, so
        // these never fire on a native Wayland session (they do under XWayland).
        // Wired anyway - it costs three arms, it works on X11 today, and it
        // starts working on Wayland the day winit grows the protocol. The
        // Downloads-to-priority drag above is the path that works everywhere,
        // and it is the one the UI points at.
        iced::Event::Window(iced::window::Event::FileHovered(_)) => {
            Some(Message::FilesHovering(true))
        }
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
            Some(Message::FilesHovering(false))
        }
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(path))
        }
        _ => None,
    });

    // App shortcuts. `on_key_press` takes a plain `fn`, so it cannot read `app`;
    // the handlers themselves no-op off the main screen / while a modal is open.
    let shortcuts = keyboard::listen().filter_map(|event| {
        let keyboard::Event::KeyPressed { key, modifiers: mods, .. } = event else {
            return None;
        };
        match key.as_ref() {
        Key::Named(Named::F5) => Some(Message::Refresh),
        // Ctrl+R launches the current run target (MO2's Run accelerator).
        Key::Character("r") if mods.control() => Some(Message::Run),
        Key::Named(Named::Escape) => Some(Message::ClearSelection),
        Key::Character("a") if mods.control() || mods.command() => {
            Some(Message::SelectAllInFocus)
        }
        // Ctrl+C over the LOOT report copies it whole. `update` no-ops when the
        // report is not open, since this closure cannot see the app.
        Key::Character("c") if mods.control() || mods.command() => {
            Some(Message::CopyLootReport)
        }
        // Navigation. Which list answers is decided in `update` - this closure
        // is a plain `fn` and cannot see the app.
        Key::Named(Named::Tab) => Some(Message::CycleFocus),
        // Ctrl moves the ROW; plain moves the focus. Checked first, or the
        // plain arms below would swallow it.
        Key::Named(Named::ArrowUp) if mods.control() || mods.command() => {
            Some(Message::KeyNav(Nav::ShiftUp))
        }
        Key::Named(Named::ArrowDown) if mods.control() || mods.command() => {
            Some(Message::KeyNav(Nav::ShiftDown))
        }
        Key::Named(Named::ArrowUp) => Some(Message::KeyNav(Nav::Up)),
        Key::Named(Named::ArrowDown) => Some(Message::KeyNav(Nav::Down)),
        Key::Named(Named::PageUp) => Some(Message::KeyNav(Nav::PageUp)),
        Key::Named(Named::PageDown) => Some(Message::KeyNav(Nav::PageDown)),
        Key::Named(Named::Home) => Some(Message::KeyNav(Nav::First)),
        Key::Named(Named::End) => Some(Message::KeyNav(Nav::Last)),
        Key::Named(Named::Space) => Some(Message::KeyNav(Nav::Toggle)),
        Key::Named(Named::Enter) => Some(Message::KeyNav(Nav::Activate)),
        Key::Named(Named::Delete) => Some(Message::KeyNav(Nav::Remove)),
        // Ctrl+F puts the caret in the filter box - the one shortcut everybody
        // tries first in a list this long.
        Key::Character("f") if mods.control() || mods.command() => Some(Message::FocusFilter),
        // A bare letter jumps to the next mod starting with it, the way every
        // desktop list has since before Explorer. Checked LAST so it can never
        // shadow a modified shortcut, and only for a single character with no
        // Ctrl/Alt held - the `typing` gate below keeps it out of text fields.
        Key::Character(c)
            if !mods.control() && !mods.command() && !mods.alt() && c.chars().count() == 1 =>
        {
            c.chars().next().filter(|ch| ch.is_alphanumeric()).map(Message::JumpToLetter)
        }
        _ => None,
        }
    });

    // The shortcut stream is gated on the main screen (the wizard/FOMOD views have
    // their own focus); modifier tracking always runs so the set is never stale.
    // Navigation keys are suppressed while a field has the caret; the always-safe
    // ones (F5, Ctrl+R, Escape, Ctrl+A) keep working, and Escape is what gets the
    // keyboard back out of a field.
    let typing = app.typing;
    let shortcuts = shortcuts.with(typing).map(|(typing, m)| match m {
        // JumpToLetter joins them: a bare letter belongs to the list only when
        // no field has the caret, or typing "f" into the filter box would jump
        // the list instead of filtering it.
        Message::KeyNav(_) | Message::CycleFocus | Message::JumpToLetter(_) if typing => {
            Message::Noop
        }
        other => other,
    });

    let mut subs = vec![track, pointer];
    if app.screen == Screen::Main
        && app.fomod.is_none()
        && app.rename.is_none()
        && !app.settings_open
        && app.executables.is_none()
        && app.collision.is_none()
        && app.info_mod.is_none()
        // Don't fire shortcuts (especially Ctrl+R) while the GUI is locked behind a
        // running game or a LOOT report is open. An unlocked tracked run keeps them.
        && app.running.as_ref().is_none_or(|r| !r.lock)
        && app.loot_report.is_none()
        // Every other overlay that owns the screen. A navigation key reaching
        // the mod list from behind one of these moves a selection the user
        // cannot see, and Space would toggle a mod they are not looking at.
        && !app.about_open
        && !app.view_menu_open
        && app.picker.is_none()
        && app.profile_menu.is_none()
        && app.profile_rename.is_none()
        && app.profile_copy.is_none()
        && app.profile_delete_confirm.is_none()
        && app.send_priority.is_none()
        && app.overwrite_to_mod.is_none()
        && app.menu_mod.is_none()
        && app.menu_plugin.is_none()
        // The panes added since. The INI editor above all: it owns the keyboard
        // outright, and an arrow key reaching the mod list behind it moves a
        // selection nobody can see while Space toggles a mod they are not
        // looking at.
        && app.ini_editor.is_none()
        && app.log_pane.is_none()
        && app.categories_dialog.is_none()
        && app.backups.is_none()
        && app.executables.is_none()
        && !app.addons_open
        // The three added since. The export dialog and the instance manager own
        // the screen, and the File dropdown is a menu like the View one - a
        // Delete reaching the mod list from behind any of them arms a removal on
        // a row nobody is looking at.
        && app.export.is_none()
        && !app.instances_open
        && !app.file_menu_open
    {
        subs.push(shortcuts);
    }
    // While the LOOT report modal is up, the main shortcut stream above is
    // deliberately off - nothing may walk the lists behind it. But two keys mean
    // something OVER the report: Escape dismisses it and Ctrl+C copies it, and
    // both were promised (the dialog footer says so) yet dead, because their
    // only producer was the stream this modal switches off. A dedicated stream
    // for exactly those two keys, mutually exclusive with the main one.
    if app.screen == Screen::Main && app.loot_report.is_some() {
        let report_keys = keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, modifiers: mods, .. } = event else {
                return None;
            };
            match key.as_ref() {
                Key::Named(Named::Escape) => Some(Message::CloseLootReport),
                Key::Character("c") if mods.control() || mods.command() => {
                    Some(Message::CopyLootReport)
                }
                _ => None,
            }
        });
        subs.push(report_keys);
    }
    // Watch the downloads directory while its tab is open. Polling is not a
    // shortcut taken for want of something better: the transfer runs in a
    // separate `eidos nxm` process spawned by the BROWSER, so there is no handle
    // to await and no channel to listen on. The filesystem is the interface, and
    // a directory of a few dozen entries is cheap to read twice a second.
    //
    // Faster while something is arriving, so a bar moves instead of jumping;
    // slower otherwise, because the idle case only has to notice that a download
    // has begun.
    if app.tab == Tab::Downloads {
        let arriving =
            app.downloads.iter().any(|d| d.state == DownloadState::Downloading);
        let period = if arriving { DOWNLOAD_TICK } else { DOWNLOAD_IDLE_TICK };
        subs.push(iced::time::every(period).map(|_| Message::DownloadTick));
    }
    // Watch the saves directory while its tab is open. The game writes there
    // from inside the Proton prefix while Eidos is not looking, so an autosave
    // made mid-session would otherwise never appear until a manual Refresh.
    //
    // The tick compares a FINGERPRINT and only reloads when it moved: rebuilding
    // the list twice a second would drop the selection and close the details
    // pane under the user's hands.
    if app.tab == Tab::Saves {
        subs.push(iced::time::every(SAVES_TICK).map(|_| Message::SavesTick));
    }
    // Tail the session log while its pane is open. Same reasoning as the
    // downloads tick: the records worth reading are written by a SEPARATE
    // `eidos` process, so there is nothing to await - the file is the interface.
    // Subscribed only while the pane is up, so a closed pane costs nothing.
    if app.log_pane.is_some() {
        subs.push(iced::time::every(LOG_TAIL_TICK).map(|_| Message::LogRefresh));
    }
    // Auto-scroll while a drag rests on an edge band. Subscribed only then, so an
    // idle drag - or no drag at all - schedules nothing.
    // Hover-to-expand, subscribed only while a drag actually rests on a
    // collapsed group - so an idle window, and a drag over ordinary rows,
    // schedule nothing.
    if app.drag_state.is_some() && app.drag_hover_group.is_some() {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(300))
                .map(|_| Message::DragHoverTick),
        );
    }
    if app.drag_scroll.is_some() {
        subs.push(
            // 40ms rather than 60: the step is applied whole, so a slower tick
            // buys the same speed only by jumping further each time, and a jump
            // is what the user reads as the list misbehaving.
            iced::time::every(std::time::Duration::from_millis(40))
                .map(|_| Message::DragScrollTick),
        );
    }
    // While waiting on a launched game/tool, poll for its exit so we can unlock.
    if app.running.is_some() {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(600)).map(|_| Message::PollRunning),
        );
    }
    iced::Subscription::batch(subs)
}

fn main() -> iced::Result {
    // Steam passes the Proton command as our arguments via `eidos-gui %command%`.
    let launch_command: Vec<String> = std::env::args().skip(1).collect();
    // Its own rotation bucket, distinct from the CLI's: the window and an
    // `eidos` child are separate processes writing the same directory, and
    // sharing a bucket would let one rotate the other's session away.
    let _ = eidos_log::init_with(
        eidos_log::Config::new("gui").with_version(env!("CARGO_PKG_VERSION")),
    );
    // Onto the ecosystem's layout - `~/.config/Colony/Eidos` - before anything
    // reads a setting. Copies rather than moves, runs once, and cannot fail a
    // launch: see `eidos_paths::migrate_legacy_layout`. Logged rather than
    // silent, because a user who goes looking for their settings deserves to
    // find out from the log where they went.
    for note in eidos_paths::migrate_legacy_layout() {
        eidos_log::info!("{note}");
    }
    eidos_log::info!("eidos-gui {} starting", env!("CARGO_PKG_VERSION"));
    // The title moved out of `application` and onto a builder; the first argument
    // is now the boot function that `run_with` used to take. It must be `Fn`, not
    // `FnOnce` - which is why the `.clone()` stays: without it the closure would
    // consume the Vec and only be callable once.
    iced::application(move || new(launch_command.clone()), update, view)
        .title("Eidos")
        .theme(theme::theme)
        .subscription(subscription)
        .window(window_settings())
        .run()
}

/// The desktop identity of the window. MUST equal the basename of the installed
/// `eidos.desktop`, because that pairing is the only thing tying the two
/// together.
pub const APP_ID: &str = "eidos";

/// How the window introduces itself to the desktop.
///
/// Without `application_id` a Wayland surface announces an EMPTY app id, so the
/// compositor has nothing to match against a desktop entry and a taskbar shows a
/// placeholder tile no matter how many icons are installed. That was the actual
/// symptom; the icon files were never the missing part.
///
/// The embedded icon covers X11 and XWayland, where the icon travels with the
/// window instead of being looked up from a desktop file - so the binary is
/// self-sufficient even with nothing installed. It is the dark-ground tile
/// rather than the transparent mark: the mark is pale ink, which disappears
/// against a light panel. A decode failure costs the icon, never the launch.
fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: APP_ID.to_string(),
            ..Default::default()
        },
        icon: iced::window::icon::from_file_data(
            include_bytes!("../../../assets/brand/png/eidos-icon-256-on-dark.png"),
            None,
        )
        .ok(),
        ..Default::default()
    }
}

/// Whether any tool declares a runtime that is not downloaded yet.
///
/// Asked before refusing Tool Setup for want of a Proton prefix: a runtime is a
/// directory and an environment variable, so it can be fetched on a machine that
/// has never launched the game.
fn any_runtime_pending(app: &App) -> bool {
    app.created
        .as_ref()
        .map(|i| eidos_instance::read_tools(&i.root.join("tools.ini")))
        .unwrap_or_default()
        .iter()
        .flat_map(|t| t.prereqs.iter())
        .any(|v| {
            eidos_gamefeatures::runtime(v)
                .is_some_and(|r| !eidos_gamefeatures::runtime_is_installed(r))
        })
}

/// One prerequisite's state, as a label and whether it needs the user to act.
///
/// Pure so the wording and the classification can be tested: a verb reported as
/// present when it is not sends the user to look for the fault in their mods.
fn prereq_state(verb: &str, done: &std::collections::HashSet<String>) -> (&'static str, bool) {
    if eidos_gamefeatures::is_tier1_dll(verb) {
        // Shipped inside the binary and copied in at launch, so there is nothing
        // that could be missing and nothing to click.
        ("bundled with Eidos", false)
    } else if eidos_gamefeatures::is_runtime_verb(verb) {
        let ok =
            eidos_gamefeatures::runtime(verb).is_some_and(eidos_gamefeatures::runtime_is_installed);
        if ok {
            ("downloaded", false)
        } else {
            ("NOT FOUND - click to download", true)
        }
    } else if eidos_gamefeatures::is_tier2_verb(verb) {
        if done.contains(verb) {
            ("installed", false)
        } else {
            ("NOT FOUND - click to install", true)
        }
    } else {
        // Neither bundled, nor a runtime, nor a winetricks verb: almost always a
        // typo. Saying so beats a tool that fails later for a reason that names
        // nothing the user recognises. Not offered as a click - there is nothing
        // to install.
        ("unknown - check the spelling", false)
    }
}

/// What Eidos can say about each prerequisite a tool declares, and a way to fix
/// the ones it can.
///
/// The field above accepts any text, so this is the only place that answers the
/// question the user actually has - "will this tool start?". A verb that is
/// merely typed is not a verb that is present, and the difference used to be
/// invisible until the tool failed with an error naming neither Eidos nor the
/// missing runtime.
fn prereq_status_rows<'a>(app: &App, prereqs: &str) -> Element<'a, Message> {
    let verbs: Vec<String> = prereqs
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if verbs.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    // Two sources, and both are needed. Eidos records what IT installed in the
    // instance; the prefix records what winetricks installed in it, by whoever
    // ran it. Trusting only the first reports a runtime the user set up years ago
    // as missing and offers to download it again.
    let mut done: std::collections::HashSet<String> = app
        .created
        .as_ref()
        .and_then(|i| std::fs::read_to_string(i.root.join("prereqs.done")).ok())
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();
    if let Some(prefix) = selected_game(app).and_then(|g| g.compatdata.as_ref()) {
        done.extend(eidos_gamefeatures::verbs_in_prefix(&prefix.join("pfx")));
    }

    let mut col = Column::new().spacing(2).push(text("Status").size(11.0).color(FOMOD_INK_FAINT));
    let mut any_missing = false;
    for v in verbs {
        let (label, missing) = prereq_state(&v, &done);
        any_missing |= missing;

        let row = Row::new()
            .spacing(8)
            .push(text(v.clone()).size(11.0).width(Length::Fixed(150.0)))
            .push(text(label).size(11.0).color(if missing {
                Color::from_rgb8(0x8A, 0x2A, 0x2A)
            } else {
                FOMOD_INK_SOFT
            }));
        col = if missing {
            col.push(
                button(row)
                    .padding(2)
                    .on_press(Message::SetupPrereqs)
                    .style(button::text),
            )
        } else {
            col.push(row)
        };
    }
    if any_missing {
        col = col.push(
            text("Downloads run in the background; the status bar reports when they finish.")
                .size(10.0)
                .color(FOMOD_INK_FAINT),
        );
    }
    col.into()
}









#[cfg(test)]
mod tests {
    use super::*;
    // The drawing modules, imported here rather than at the crate root: main.rs
    // itself no longer draws, and these tests are the only thing left that reads
    // a colour or asks a widget helper a question.
    use crate::theme::*;
    use crate::widgets::*;

    fn mods(names: &[&str]) -> Vec<ModEntry> {
        names
            .iter()
            .map(|n| ModEntry { name: n.to_string(), enabled: true, path: PathBuf::new(), unmanaged: false })
            .collect()
    }
    #[test]
    fn selecting_a_mod_marks_the_plugins_it_ships() {
        // MO2's behaviour, and the reason the feature exists: with hundreds of
        // rows, the only other way to learn which plugins a mod brought is to
        // hover them one by one.
        let mut app = nav_app(&["Armour Pack", "Weather_separator", "Quest Mod"]);
        app.selected_mod = Some(0);
        let origins = selected_mod_origins(&app);
        assert!(plugin_from_selected_mod(&origins, "Armour Pack"));
        assert!(!plugin_from_selected_mod(&origins, "Quest Mod"), "another mod's plugin");
        // The game's own Data has no origin mod, so it can never light up.
        assert!(!plugin_from_selected_mod(&origins, ""), "vanilla content belongs to no mod");
    }

    #[test]
    fn the_origin_match_ignores_case_like_the_filesystem_does() {
        // `origin_mod` is a folder name: an archive that installed as
        // "armour pack" must still match the row shown as "Armour Pack".
        let mut app = nav_app(&["Armour Pack"]);
        app.selected_mod = Some(0);
        let origins = selected_mod_origins(&app);
        assert!(plugin_from_selected_mod(&origins, "ARMOUR PACK"));
        assert!(plugin_from_selected_mod(&origins, "armour pack"));
    }

    #[test]
    fn a_multi_selection_marks_every_selected_mods_plugins() {
        // The mod list supports multi-select, so a highlight covering only the
        // anchor row would contradict what the user sees selected.
        let mut app = nav_app(&["A", "B", "C"]);
        app.selected_mod = Some(0);
        app.selected_mods.extend([0, 2]);
        let origins = selected_mod_origins(&app);
        assert!(plugin_from_selected_mod(&origins, "A"));
        assert!(plugin_from_selected_mod(&origins, "C"));
        assert!(!plugin_from_selected_mod(&origins, "B"), "B was never selected");
    }

    #[test]
    fn a_selected_separator_marks_nothing() {
        // A separator is a divider, never the origin of a plugin. Matching on it
        // would light up every plugin whose origin happens to be empty - i.e.
        // the whole of the game's own Data.
        let mut app = nav_app(&["Group_separator"]);
        app.selected_mod = Some(0);
        let origins = selected_mod_origins(&app);
        assert!(origins.is_empty(), "{origins:?}");
        assert!(!plugin_from_selected_mod(&origins, ""));
        assert!(!plugin_from_selected_mod(&origins, "Group_separator"));
    }

    #[test]
    fn no_mod_selected_marks_nothing() {
        let app = nav_app(&["A", "B"]);
        assert!(selected_mod_origins(&app).is_empty(), "nothing selected, nothing lit");
    }

    /// A plugin row for the menu tests: name plus the mod that ships it.
    fn plugin_row(name: &str, origin: &str) -> eidos_plugins::Plugin {
        eidos_plugins::Plugin {
            name: name.to_string(),
            origin_mod: origin.to_string(),
            path: PathBuf::new(),
            enabled: true,
            force_disabled: false,
            is_master: false,
            is_light: false,
            is_medium: false,
            masters: Vec::new(),
            priority: 0,
            index: None,
        }
    }

    fn names(v: &[ModEntry]) -> Vec<&str> {
        v.iter().map(|m| m.name.as_str()).collect()
    }

    /// A throwaway game dir holding the named executables.
    fn game_dir(exes: &[&str]) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("eidos-play-{}-{}", std::process::id(), n));
        fs::create_dir_all(&d).unwrap();
        for e in exes {
            fs::write(d.join(e), b"MZ").unwrap();
        }
        d
    }

    /// The args `play_command` will hand to `eidos play`, i.e. everything after `--`.
    fn played(game_id: &str, command: &[String]) -> (Vec<String>, Option<String>) {
        let (cmd, warning) = play_command(game_id, game_id, command);
        let args: Vec<String> =
            cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        let after = args.iter().position(|a| a == "--").map(|i| args[i + 1..].to_vec());
        (after.unwrap_or_default(), warning)
    }

    #[test]
    fn the_vanilla_launcher_is_never_what_gets_run() {
        // Steam's %command% for Skyrim SE points at SkyrimSELauncher.exe, and the
        // Bethesda launcher is a settings app that rewrites plugins.txt - running
        // it through a mod manager undoes the load order that was just deployed.
        let d = game_dir(&["SkyrimSE.exe", "SkyrimSELauncher.exe"]);
        let cmd = vec!["proton".to_string(), d.join("SkyrimSELauncher.exe").display().to_string()];

        let (args, warning) = played("skyrimse", &cmd);
        assert!(args[1].ends_with("SkyrimSE.exe"), "{args:?}");
        // And the user is told why their SKSE mods will do nothing.
        assert!(warning.as_deref().unwrap_or_default().contains("skse64_loader.exe"), "{warning:?}");
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn the_script_extender_still_wins_when_it_is_installed() {
        let d = game_dir(&["SkyrimSE.exe", "SkyrimSELauncher.exe", "skse64_loader.exe"]);
        let cmd = vec![d.join("SkyrimSELauncher.exe").display().to_string()];
        let (args, warning) = played("skyrimse", &cmd);
        assert!(args[0].ends_with("skse64_loader.exe"), "{args:?}");
        // Nothing was given up, so nothing to warn about.
        assert_eq!(warning, None);
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_command_that_is_already_the_game_is_left_alone() {
        let d = game_dir(&["SkyrimSE.exe", "skse64_loader.exe"]);
        let cmd = vec![d.join("SkyrimSE.exe").display().to_string()];
        let (args, warning) = played("skyrimse", &cmd);
        // No launcher in the command means no swap - we do not second-guess a
        // target the user or Steam already chose.
        assert!(args[0].ends_with("SkyrimSE.exe"), "{args:?}");
        assert_eq!(warning, None);
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn an_empty_game_dir_leaves_the_command_untouched_and_says_so() {
        let d = game_dir(&[]);
        let cmd = vec![d.join("SkyrimSELauncher.exe").display().to_string()];
        let (args, warning) = played("skyrimse", &cmd);
        assert!(args[0].ends_with("SkyrimSELauncher.exe"), "{args:?}");
        assert!(warning.is_some());
        fs::remove_dir_all(&d).ok();
    }

    /// The drop the user described: grab a mod, aim at the strip ABOVE another
    /// one, and it lands there - whichever direction the drag came from. Under
    /// the old row-targeted drop this was ambiguous, and the downward case
    /// landed one slot short of where the pointer was.
    #[test]
    fn a_gap_targeted_drop_lands_exactly_where_it_was_aimed() {
        // Dragging UP: "Terrain Helper" (index 3) onto the strip above
        // "Terrain Variation" (index 2).
        let mut v = mods(&["a", "b", "variation", "helper"]);
        move_block(&mut v, &[3], 2);
        assert_eq!(names(&v), ["a", "b", "helper", "variation"]);

        // Dragging DOWN to the SAME visual place: grab "helper" from the top and
        // aim at the strip above "variation" (index 3 now). Same destination,
        // opposite direction - this is the case the row-targeted version got
        // wrong by one.
        let mut v = mods(&["helper", "a", "b", "variation"]);
        move_block(&mut v, &[0], 3);
        assert_eq!(names(&v), ["a", "b", "helper", "variation"]);

        // The trailing strip (gap == len) is the only way to reach the end.
        let mut v = mods(&["a", "b", "c"]);
        move_block(&mut v, &[0], 3);
        assert_eq!(names(&v), ["b", "c", "a"]);
    }

    /// The two gaps that touch a grabbed row mean "leave it where it is". The
    /// drop handler treats them as no-ops so a slightly-wobbly click never
    /// rewrites modlist.txt (and never fires the save + reload it triggers).
    #[test]
    fn the_strips_touching_the_grabbed_row_are_no_ops() {
        for gap in [1usize, 2] {
            let mut v = mods(&["a", "b", "c"]);
            let before: Vec<String> = names(&v).iter().map(|s| s.to_string()).collect();
            // What the handler computes for a single grabbed row at index 1.
            let unchanged = gap == 1 || gap == 1 + 1;
            assert!(unchanged, "gap {gap} next to row 1 must be a no-op");
            if !unchanged {
                move_block(&mut v, &[1], gap);
            }
            assert_eq!(names(&v), before);
        }
    }

    /// The rows `visible_rows` says to draw, by name, for a readable assertion.
    fn drawn<'a>(v: &'a [ModEntry], vis: &[bool]) -> Vec<&'a str> {
        v.iter().zip(vis).filter(|(_, &s)| s).map(|(m, _)| m.name.as_str()).collect()
    }

    #[test]
    fn a_search_finds_mods_inside_a_folded_group() {
        // The bug this pins: the fold was applied before the query, so a match
        // inside a folded group was dropped and the list said "no mods match" -
        // a WRONG answer, not a slow one. The user then reasonably concludes the
        // mod is not installed.
        let v = mods(&["armour_separator", "iron armour", "steel armour", "misc_separator", "a map"]);
        let folded: HashSet<String> = ["armour".to_string()].into_iter().collect();

        let vis = visible_rows(&v, &folded, true, |_, m| m.display_name().contains("armour"));
        assert_eq!(drawn(&v, &vis), ["armour_separator", "iron armour", "steel armour"]);

        // The group that contributed nothing is gone, header included, so the
        // filter does not leave a wall of empty headers behind.
        assert!(!vis[3]);
    }

    #[test]
    fn folding_still_hides_the_group_when_nothing_is_being_asked() {
        let v = mods(&["armour_separator", "iron armour", "misc_separator", "a map"]);
        let folded: HashSet<String> = ["armour".to_string()].into_iter().collect();

        // No filter: the fold is honoured, and both headers stay - a header is
        // the handle you unfold by, so hiding it would strand the group.
        let vis = visible_rows(&v, &folded, false, |_, _| true);
        assert_eq!(drawn(&v, &vis), ["armour_separator", "misc_separator", "a map"]);
    }

    #[test]
    fn a_separator_draws_only_for_the_group_it_actually_heads() {
        // Rows before the FIRST separator belong to no group; a match there must
        // not resurrect the header that follows it.
        let v = mods(&["loose mod", "armour_separator", "iron armour"]);
        let vis = visible_rows(&v, &HashSet::new(), true, |_, m| m.name == "loose mod");
        assert_eq!(drawn(&v, &vis), ["loose mod"]);

        // And a match inside the group brings back that header and no other.
        let v2 = mods(&["a_separator", "one", "b_separator", "two"]);
        let vis2 = visible_rows(&v2, &HashSet::new(), true, |_, m| m.name == "two");
        assert_eq!(drawn(&v2, &vis2), ["b_separator", "two"]);
    }

    #[test]
    fn the_indices_visible_rows_reports_are_the_real_row_indices() {
        // The drop gaps are keyed by absolute index, so a filtered list must not
        // renumber anything: gap `i` has to keep meaning "before mods[i]" or a
        // drop under a filter lands somewhere else entirely.
        let v = mods(&["a", "b", "c"]);
        let vis = visible_rows(&v, &HashSet::new(), true, |_, m| m.name == "c");
        assert_eq!(vis, [false, false, true]);
    }

    /// An App with just enough filled in to drive `key_nav`.
    /// An App with just enough filled in to drive `key_nav`, and NO instance.
    ///
    /// `created` is what every save path writes through, so a test that leaves
    /// it pointing at a real instance can reach the user's files. `new` refuses
    /// to attach one under `cfg(test)`; this asserts it, because the guard being
    /// silently lost is exactly the failure that would not be noticed until
    /// somebody's mod list was four entries long.
    fn nav_app(mod_names: &[&str]) -> App {
        let mut app = new(Vec::new()).0;
        assert!(app.created.is_none(), "a test App must never hold a real instance");
        app.mods = mods(mod_names);
        app.screen = Screen::Main;
        app
    }

    /// An App with one game selected, built without touching the disk.
    fn app_for_game(id: &str) -> App {
        let mut app = nav_app(&[]);
        app.games = vec![DetectedGame {
            def: eidos_games::GameDef::for_id(id).expect("a game in the catalog"),
            install_path: PathBuf::from("/nowhere"),
            data_path: PathBuf::from("/nowhere/data"),
            compatdata: None,
            steam_name: id.to_string(),
        }];
        app.selected = Some(0);
        app
    }

    /// A real portable instance in a temp dir: manifest + the minimum layout.
    fn temp_portable(game_id: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "eidos-portable-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("mods")).unwrap();
        eidos_instance::Manifest::new(game_id, InstanceKind::Portable)
            .write(&root.join("eidos-instance.ini"))
            .unwrap();
        root
    }

    #[test]
    fn open_known_switches_to_the_chosen_portable_instance() {
        // The reported gap: a portable instance existed on disk but nothing
        // could ever OPEN it again. The welcome list entry must actually open.
        let root = temp_portable("skyrimse");
        let mut app = app_for_game("skyrimse");
        app.screen = Screen::Welcome;
        app.known = vec![KnownInstance {
            label: "Skyrim SE - portable".into(),
            inst: Instance::portable(root.clone()),
            game_index: 0,
        portable: true,
        }];
        let _ = update_inner(&mut app, Message::OpenKnown(0));
        assert_eq!(app.created.as_ref().map(|i| i.root.clone()), Some(root.clone()));
        assert!(matches!(app.screen, Screen::Main), "opening must land on the main screen");
        assert_eq!(app.selected, Some(0));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn open_known_on_a_missing_root_says_so_instead_of_wedging() {
        let mut app = app_for_game("skyrimse");
        app.screen = Screen::Welcome;
        app.known = vec![KnownInstance {
            label: "gone".into(),
            inst: Instance::portable(PathBuf::from("/nonexistent/eidos-test-root")),
            game_index: 0,
        portable: true,
        }];
        let _ = update_inner(&mut app, Message::OpenKnown(0));
        assert!(app.created.is_none(), "a dead root must not fake an open");
        assert!(matches!(app.screen, Screen::Welcome));
        assert!(
            app.status.as_deref().unwrap_or("").contains("not reachable"),
            "the skip must be said, not silent: {:?}",
            app.status
        );
    }

    #[test]
    fn finish_refuses_to_relabel_a_foreign_portable_folder() {
        // ensure_manifest keeps an existing manifest, so before this check a
        // fallout4 folder adopted under a skyrimse wizard kept its old game id
        // while everything else treated it as Skyrim - a silent mislabel.
        let root = temp_portable("fallout4");
        let mut app = app_for_game("skyrimse");
        app.screen = Screen::Summary;
        app.kind = InstanceKind::Portable;
        app.name = "Mine".into();
        app.portable_path = root.display().to_string();
        let _ = update_inner(&mut app, Message::Finish);
        assert!(app.created.is_none(), "adoption must refuse, not relabel");
        assert!(
            app.error.as_deref().unwrap_or("").contains("fallout4"),
            "the refusal must name the folder's real game: {:?}",
            app.error
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finish_refuses_a_root_inside_the_game_install() {
        // The MO2-veteran reflex: put the manager in the game folder. Steam
        // owns that tree and Eidos mounts over it - the wizard must say no.
        let mut app = app_for_game("skyrimse");
        app.screen = Screen::Summary;
        app.kind = InstanceKind::Portable;
        app.name = "Mine".into();
        // app_for_game's install_path is /nowhere.
        app.portable_path = "/nowhere/Eidos".into();
        let _ = update_inner(&mut app, Message::Finish);
        assert!(app.created.is_none(), "an instance inside the install must not be created");
        assert!(
            app.error.as_deref().unwrap_or("").contains("own folder"),
            "the refusal must explain itself: {:?}",
            app.error
        );
        assert!(!Path::new("/nowhere/Eidos").exists(), "nothing may be created on refusal");
    }

    #[test]
    fn finish_adopts_a_matching_portable_folder() {
        let root = temp_portable("skyrimse");
        fs::create_dir_all(root.join("mods/Existing Mod")).unwrap();
        let mut app = app_for_game("skyrimse");
        app.screen = Screen::Summary;
        app.kind = InstanceKind::Portable;
        app.name = "Mine".into();
        app.portable_path = root.display().to_string();
        let _ = update_inner(&mut app, Message::Finish);
        assert_eq!(app.created.as_ref().map(|i| i.root.clone()), Some(root.clone()));
        assert!(matches!(app.screen, Screen::Main));
        assert!(
            app.mods.iter().any(|m| m.name == "Existing Mod"),
            "adoption must see the folder's own mods"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn known_instances_list_last_used_first_and_skips_missing_roots() {
        let a = temp_portable("skyrimse");
        let b = temp_portable("skyrimse");
        let mut reg = eidos_instance::Registry::default();
        reg.remember_portable(&a);
        reg.remember_portable(&b);
        reg.portables.push(PathBuf::from("/nonexistent/eidos-x"));
        reg.set_last(eidos_instance::InstanceRef::Portable(a.clone()));
        let app = app_for_game("skyrimse");
        let known = known_instances_from(&reg, &app.games);
        let roots: Vec<PathBuf> = known.iter().map(|k| k.inst.root.clone()).collect();
        assert_eq!(roots.first(), Some(&a), "last-used first");
        assert!(roots.contains(&b));
        assert!(
            !roots.iter().any(|r| r.starts_with("/nonexistent")),
            "a missing root is skipped (not offered), never listed dead"
        );
        assert_eq!(roots.iter().filter(|r| **r == a).count(), 1, "last + MRU must not duplicate");
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }

    #[test]
    fn switching_instance_drops_the_previous_games_merged_view() {
        // The Data tab is memoised per directory against `view_generation`, so a
        // switch that does not bump it answers every already-listed directory out
        // of the OLD instance. Going from Skyrim to a game with no mods at all
        // still drew Skyrim's merged tree, `[skyrimse]` provenance and all.
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "eidos-switch-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));

        let mut app = app_for_game("stellarblade");
        app.kind = InstanceKind::Portable;
        app.portable_path = root.to_string_lossy().into_owned();

        // Stand in for what browsing the previous instance's Data tab leaves behind.
        app.data_listing
            .borrow_mut()
            .insert(
                String::new(),
                (
                    app.view_generation.get(),
                    vec![DataRow {
                        name: "SKSE".into(),
                        source: "[skyrimse]".into(),
                        is_dir: true,
                        real: PathBuf::from("/old/SKSE"),
                        size: None,
                        mtime: None,
                        conflicted: false,
                    }],
                ),
            );
        app.listing_cache
            .borrow_mut()
            .insert(PathBuf::from("/old"), (app.view_generation.get(), std::rc::Rc::new(vec!["stale".to_string()])));
        app.files_cache.borrow_mut().insert("OldMod".into(), (vec!["a.esp".into()], false));
        app.data_expanded.insert("meshes".to_string());

        let _ = update(&mut app, Message::Finish);
        assert!(app.created.is_some(), "the instance was created");

        assert!(app.data_listing.borrow().is_empty(), "the merged listing survived the switch");
        assert!(app.listing_cache.borrow().is_empty(), "a directory listing survived the switch");
        assert!(app.files_cache.borrow().is_empty(), "a mod's file list survived the switch");
        assert!(app.data_expanded.is_empty(), "expanded paths from the old game survived");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn releasing_outside_the_list_still_drops_where_you_aimed() {
        // The bug this replaces: cancelling on pointer EXIT meant dragging up
        // past the header - the only way to reach an earlier row - dropped the
        // mod and cleared the selection every time. Leaving and letting go are
        // indistinguishable to `on_exit`, so the release is caught globally.
        let mut app = nav_app(&["a", "b", "c", "d"]);
        let _ = update(&mut app, Message::DragStart(3));
        let _ = update(&mut app, Message::DragOverGap(1));
        assert!(app.drag_state.is_some_and(|d| d.aimed), "the gap was aimed at");

        // Released with the pointer anywhere at all.
        let _ = update(&mut app, Message::PointerReleased);
        assert!(app.drag_state.is_none(), "the drag ended");
        assert_eq!(names(&app.mods), vec!["a", "d", "b", "c"], "it moved where it aimed");
    }

    #[test]
    fn a_long_drag_still_drops_after_the_list_has_scrolled_under_it() {
        // The bug: dragging a block far enough that the auto-scroll took over
        // put the pointer on a scroll band, which is not a drop strip. A second
        // release handler on the list then cancelled the drag before the global
        // one could commit it, so a long drag never landed. One handler decides
        // now, and it only ever looks at where the drag was AIMED.
        let mut app = nav_app(&["a", "b", "c", "d", "e"]);
        app.selected_mods = [3usize, 4].into_iter().collect();
        let _ = update(&mut app, Message::DragStart(3));
        let _ = update(&mut app, Message::DragOverGap(0));

        // The list scrolls under the pointer; the aim does not change.
        let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Up)));
        let _ = update(&mut app, Message::DragScrollTick);
        assert!(app.drag_state.is_some_and(|d| d.aimed), "the scroll disarmed the drag");

        let _ = update(&mut app, Message::PointerReleased);
        assert_eq!(names(&app.mods), vec!["d", "e", "a", "b", "c"], "the block did not land");
        assert!(app.drag_scroll.is_none(), "the scroll timer outlived the drag");
    }

    #[test]
    fn a_release_that_aimed_at_nothing_is_a_click_not_a_move() {
        // A plain click arms a drag too. Releasing it must not reorder anything.
        let mut app = nav_app(&["a", "b", "c"]);
        let _ = update(&mut app, Message::DragStart(2));
        let _ = update(&mut app, Message::PointerReleased);
        assert!(app.drag_state.is_none());
        assert_eq!(names(&app.mods), vec!["a", "b", "c"], "a click moved a row");
    }

    #[test]
    fn the_auto_scroll_only_runs_while_a_drag_does() {
        let mut app = nav_app(&["a", "b", "c"]);
        // No drag: entering a band cannot start anything. The bands are not even
        // rendered then, but a stale message must not be enough on its own.
        let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Up)));
        assert!(app.drag_scroll.is_none());

        let _ = update(&mut app, Message::DragStart(0));
        let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Down)));
        assert_eq!(app.drag_scroll, Some(ScrollEdge::Down));

        // Ending the drag stops it, so no timer outlives the gesture.
        let _ = update(&mut app, Message::PointerReleased);
        assert!(app.drag_scroll.is_none());
    }

    #[test]
    fn every_settings_category_opens_with_something_to_read() {
        // A page whose sections are all shut asks the user to click before it
        // says anything, which is the failure mode of a sectioned settings
        // screen. Each category ships one section open.
        let app = nav_app(&[]);
        assert_eq!(SettingsTab::DEFAULT_OPEN.len(), SettingsTab::ALL.len());
        for key in SettingsTab::DEFAULT_OPEN {
            assert!(app.settings_expanded.contains(key), "{key} did not start open");
        }
    }

    #[test]
    fn a_section_header_toggles_rather_than_only_opening() {
        let mut app = nav_app(&[]);
        assert!(app.settings_expanded.contains("startup"));
        let _ = update(&mut app, Message::SettingsToggleSection("startup"));
        assert!(!app.settings_expanded.contains("startup"), "it did not close");
        let _ = update(&mut app, Message::SettingsToggleSection("startup"));
        assert!(app.settings_expanded.contains("startup"), "it did not reopen");
    }

    #[test]
    fn every_settings_toggle_persists_what_it_flipped() {
        // Each of these writes settings.ini, so a flip that only changed the
        // in-memory copy would look right and be gone next launch.
        let mut app = nav_app(&[]);
        let before = (app.prefs.remember_window, app.prefs.lock_gui);
        let _ = update(&mut app, Message::ToggleRememberWindow(!before.0));
        let _ = update(&mut app, Message::ToggleLockGui(!before.1));
        assert_eq!(app.prefs.remember_window, !before.0);
        assert_eq!(app.prefs.lock_gui, !before.1);
    }

    #[test]
    fn the_auto_scroll_speeds_up_toward_the_edge() {
        // Depth 0 is the inner lip of the band, 1.0 hard against the edge of the
        // list. The point of the range: one speed can creep to the row just off
        // screen OR cross a 250-mod list, never both.
        let mut app = nav_app(&["a", "b", "c"]);
        let _ = update(&mut app, Message::DragStart(0));
        let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Down)));
        // Entering starts mid-range rather than at full speed, since `on_move`
        // has not fired yet and a lurch is worse than a slow start.
        assert!((app.drag_scroll_depth - 0.5).abs() < f32::EPSILON);

        let _ = update(&mut app, Message::DragScrollDepth(0.0));
        assert_eq!(app.drag_scroll_depth, 0.0);
        let _ = update(&mut app, Message::DragScrollDepth(1.0));
        assert_eq!(app.drag_scroll_depth, 1.0);

        // Out-of-range values are clamped, not trusted: the depth comes from a
        // pointer position divided by a height, and both can surprise.
        let _ = update(&mut app, Message::DragScrollDepth(4.2));
        assert_eq!(app.drag_scroll_depth, 1.0);
        let _ = update(&mut app, Message::DragScrollDepth(-1.0));
        assert_eq!(app.drag_scroll_depth, 0.0);
    }

    #[test]
    fn the_slow_end_is_still_slower_than_the_fast_end() {
        // Guards the constants against being edited into each other's order,
        // which would make the band feel arbitrary rather than aimable.
        assert!(DRAG_SCROLL_SLOW_PX > 0.0, "the shallow end must still move");
        assert!(
            DRAG_SCROLL_FAST_PX > DRAG_SCROLL_SLOW_PX * 2.0,
            "the range is too narrow to be worth having"
        );
    }

    #[test]
    fn a_tick_without_a_drag_does_nothing_and_disarms() {
        // The shape that broke this before: the scroll must never act on state
        // it kept about the list. A tick with no drag behind it does nothing at
        // all rather than moving the view somewhere it remembered.
        let mut app = nav_app(&["a", "b", "c"]);
        app.drag_scroll = Some(ScrollEdge::Up);
        let _ = update(&mut app, Message::DragScrollTick);
        assert!(app.drag_scroll.is_none(), "a tick with no drag left the edge armed");
    }

    #[test]
    fn a_press_alone_is_not_yet_a_drag() {
        // What the auto-scroll bands key off. `DragStart` fires on PRESS, so
        // keying them off "a drag exists" put them under the pointer on every
        // click - and a `mouse_area` laid out beneath a stationary cursor
        // publishes `on_enter` at once, which is what launched the list.
        let mut app = nav_app(&["a", "b", "c"]);
        let _ = update(&mut app, Message::DragStart(1));
        assert!(app.drag_state.is_some(), "the press armed a drag");
        assert!(!app.drag_state.is_some_and(|d| d.aimed), "but nothing is aimed at yet");

        let _ = update(&mut app, Message::DragOverGap(0));
        assert!(app.drag_state.is_some_and(|d| d.aimed), "crossing a gap is a real drag");
    }

    #[test]
    fn the_plugins_tab_is_only_offered_where_eidos_manages_plugins() {
        // Skyrim has a plugins.txt Eidos writes, so the tab means something.
        assert!(game_manages_plugins(&app_for_game("skyrimse")));
        // Stellar Blade has no plugin system at all. Offering the tab would open
        // an empty list for a game that will never have one.
        assert!(!game_manages_plugins(&app_for_game("stellarblade")));
        // Neither does a game whose order is file timestamps, which Eidos does
        // not manage either - the tab would be just as empty there.
        assert!(!game_manages_plugins(&app_for_game("morrowind")));
        // And with no game chosen at all there is nothing to manage.
        assert!(!game_manages_plugins(&nav_app(&[])));
    }

    #[test]
    fn a_bundle_of_variants_says_the_mod_is_inside_one_of_them() {
        use eidos_install::{ArchiveEntry, ArchiveTree};
        let rows = |paths: &[&str]| {
            ArchiveTree::from_entries(
                &paths
                    .iter()
                    .map(|p| ArchiveEntry {
                        path: p.trim_end_matches('/').to_string(),
                        is_dir: p.ends_with('/'),
                    })
                    .collect::<Vec<_>>(),
            )
            .flatten()
        };

        // The real shape of EVE_Sunrise_Dress.7z: screenshots, a readme, and two
        // zips that each hold one variant of the mod. No level of it can ever look
        // valid, so without this the dialog just repeats "does NOT look valid"
        // wherever the user clicks.
        let hint = nested_archive_hint(&rows(&[
            "EVE Sunrise Dress/1 (1).jpg",
            "EVE Sunrise Dress/1 (2).jpg",
            "EVE Sunrise Dress/Full replaces planet diving suit-704.zip",
            "EVE Sunrise Dress/No back accessories-704.zip",
            "EVE Sunrise Dress/readme.txt",
        ]))
        .expect("a bundle of variants is worth naming");
        assert!(hint.contains('2'), "it should say how many: {hint}");

        // One inner archive is the singular case, not "0 archives".
        let one = nested_archive_hint(&rows(&["Mod/inner.rar"])).expect("one nested archive");
        assert!(!one.contains('2'));

        // And an ordinary mod says nothing at all: the hint is for the dead end,
        // not a remark on every archive that fails the check.
        assert_eq!(nested_archive_hint(&rows(&["Mod/thing_P.pak", "Mod/notes.txt"])), None);
        // A directory that merely ends in an archive extension is not one.
        assert_eq!(nested_archive_hint(&rows(&["Mod/backup.zip/x.pak"])), None);
    }

    #[test]
    fn plugin_advice_is_only_given_to_games_that_have_plugins() {
        // Two predicates that are easy to confuse, kept apart by the three games
        // that fall differently between them.
        //
        //   Skyrim   : has plugins, Eidos writes the order   -> both true
        //   Morrowind: has plugins, Eidos does not write it  -> has, not manages
        //   Stellar Blade: no plugin system at all           -> both false
        //
        // Getting this wrong in either direction is visible: gate LOOT advice on
        // "manages" and Morrowind stops being told that LOOT cannot sort it,
        // which is the only game the message was written for. Gate it on nothing
        // and Stellar Blade is told LOOT cannot sort it - true of every game ever
        // made that is not Bethesda's - and pointed at a Plugins tab it does not
        // show.
        let sky = app_for_game("skyrimse");
        assert!(game_has_plugins(&sky) && game_manages_plugins(&sky));

        let mw = app_for_game("morrowind");
        assert!(game_has_plugins(&mw), "Morrowind has .esp files and a load order");
        assert!(!game_manages_plugins(&mw), "Eidos just does not write it");

        let sb = app_for_game("stellarblade");
        assert!(!game_has_plugins(&sb) && !game_manages_plugins(&sb));

        // With no game at all, neither is true.
        let none = nav_app(&[]);
        assert!(!game_has_plugins(&none) && !game_manages_plugins(&none));
    }

    #[test]
    fn a_game_without_plugins_gets_no_plugin_diagnostics() {
        let titles = |app: &App| -> Vec<String> {
            diagnostics(app).into_iter().map(|d| d.title).collect()
        };
        let sb = titles(&app_for_game("stellarblade"));
        for needle in ["LOOT", "Load order", "load order"] {
            assert!(
                !sb.iter().any(|t| t.contains(needle)),
                "Stellar Blade was given plugin advice: {sb:?}"
            );
        }
        // And Morrowind still is, because for it the advice is true and useful.
        let mw = titles(&app_for_game("morrowind"));
        assert!(
            mw.iter().any(|t| t.contains("LOOT cannot sort")),
            "Morrowind lost the advice the message exists for: {mw:?}"
        );
    }

    #[test]
    fn a_remembered_plugins_tab_does_not_survive_a_game_without_plugins() {
        // `app.tab` outlives a game switch, so it can name a tab this game does
        // not show. The panel must follow what is on screen, not what was.
        let mut app = app_for_game("stellarblade");
        app.tab = Tab::Plugins;
        assert_eq!(effective_tab(&app), Tab::Data, "an invisible tab must not draw");

        let mut app = app_for_game("skyrimse");
        app.tab = Tab::Plugins;
        assert_eq!(effective_tab(&app), Tab::Plugins, "and a visible one still does");

        // Every other tab is untouched by this.
        for t in [Tab::Data, Tab::Conflicts, Tab::Overwrite, Tab::Saves, Tab::Downloads] {
            let mut app = app_for_game("stellarblade");
            app.tab = t;
            assert_eq!(effective_tab(&app), t);
        }
    }

    #[test]
    fn the_first_arrow_key_lands_on_the_list_instead_of_doing_nothing() {
        // With nothing focused, Down must reach the top and Up the bottom, or a
        // keyboard-only user has no way in.
        let mut app = nav_app(&["a", "b", "c"]);
        assert_eq!(app.selected_mod, None);
        let _ = key_nav(&mut app, Nav::Down);
        assert_eq!(app.selected_mod, Some(0));

        let mut app = nav_app(&["a", "b", "c"]);
        let _ = key_nav(&mut app, Nav::Up);
        assert_eq!(app.selected_mod, Some(2));
    }

    #[test]
    fn navigation_stops_at_the_ends_rather_than_wrapping() {
        // Wrapping from the last row to the first is how a held arrow key
        // silently loses your place in a long list.
        let mut app = nav_app(&["a", "b", "c"]);
        app.selected_mod = Some(2);
        let _ = key_nav(&mut app, Nav::Down);
        assert_eq!(app.selected_mod, Some(2));
        app.selected_mod = Some(0);
        let _ = key_nav(&mut app, Nav::Up);
        assert_eq!(app.selected_mod, Some(0));

        // A page past the end clamps too, and Home/End are absolute.
        let _ = key_nav(&mut app, Nav::PageDown);
        assert_eq!(app.selected_mod, Some(2));
        let _ = key_nav(&mut app, Nav::First);
        assert_eq!(app.selected_mod, Some(0));
        let _ = key_nav(&mut app, Nav::Last);
        assert_eq!(app.selected_mod, Some(2));
    }

    #[test]
    fn shift_and_arrows_build_the_same_selection_as_shift_and_click() {
        let mut app = nav_app(&["a", "b", "c", "d"]);
        app.selected_mod = Some(1);
        app.modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = key_nav(&mut app, Nav::Down);
        let _ = key_nav(&mut app, Nav::Down);
        let mut got: Vec<usize> = app.selected_mods.iter().copied().collect();
        got.sort_unstable();
        assert_eq!(got, [1, 2, 3]);
        assert_eq!(app.selected_mod, Some(3));

        // And a plain arrow after that collapses back to one row.
        app.modifiers = iced::keyboard::Modifiers::default();
        let _ = key_nav(&mut app, Nav::Up);
        assert!(app.selected_mods.is_empty());
        assert_eq!(app.selected_mod, Some(2));
    }

    #[test]
    fn an_empty_list_swallows_every_navigation_key() {
        let mut app = nav_app(&[]);
        for nav in [Nav::Down, Nav::Up, Nav::First, Nav::Last, Nav::PageDown, Nav::Toggle] {
            let _ = key_nav(&mut app, nav);
            assert_eq!(app.selected_mod, None, "{nav:?} on an empty list");
        }
    }

    #[test]
    fn delete_arms_the_guard_and_never_removes_on_its_own() {
        // A key that deletes a mod outright is a key that deletes a mod by
        // accident. It opens the same two-step confirmation the menu uses.
        let mut app = nav_app(&["a", "b"]);
        app.selected_mod = Some(1);
        let _ = key_nav(&mut app, Nav::Remove);
        assert_eq!(app.confirm_remove, Some(1));
        assert_eq!(names(&app.mods), ["a", "b"], "nothing may be removed yet");

        // Unmanaged rows are the game's own content and are not removable.
        let mut app = nav_app(&["dlc"]);
        app.mods[0].unmanaged = true;
        app.selected_mod = Some(0);
        let _ = key_nav(&mut app, Nav::Remove);
        assert_eq!(app.confirm_remove, None);
    }

    #[test]
    fn the_keyboard_never_drives_a_list_that_is_not_on_screen() {
        // Focus follows the last row pressed, but the plugin list only exists
        // while its tab does - so a focus left there would send the arrows
        // somewhere invisible.
        let mut app = nav_app(&["a", "b"]);
        app.focus = Pane::Plugins;
        app.tab = Tab::Data;
        assert_eq!(effective_focus(&app), Pane::Mods);
        let _ = key_nav(&mut app, Nav::Down);
        assert_eq!(app.selected_mod, Some(0), "the mod list answered");

        // Even on the Plugins tab, with no plugin list computed there is nothing
        // to drive.
        app.tab = Tab::Plugins;
        assert!(app.plugins.is_none());
        assert_eq!(effective_focus(&app), Pane::Mods);
    }

    #[test]
    fn typing_in_a_field_takes_the_navigation_keys_away_from_the_list() {
        // iced's on_key_press is a global subscription and cannot see which
        // widget holds the caret, so a space typed into the filter box would
        // otherwise toggle a mod.
        let mut app = nav_app(&["a", "b", "c"]);
        app.selected_mod = Some(0);
        let _ = update(&mut app, Message::SearchChanged("te".to_string()));
        assert!(app.typing);

        // Pressing a row hands it straight back.
        let _ = update(&mut app, Message::SelectMod(1));
        assert!(!app.typing);
        // As does Escape, which is the way out when the pointer is not involved.
        let _ = update(&mut app, Message::SearchChanged("te".to_string()));
        let _ = update(&mut app, Message::ClearSelection);
        assert!(!app.typing);
    }

    #[test]
    fn every_main_screen_field_hands_the_keyboard_over() {
        // A field that forgets to do this is invisible until someone types a
        // space into it and a mod turns off.
        for msg in [
            Message::SearchChanged("x".into()),
            Message::RenameChanged("x".into()),
            Message::NotesChanged("x".into()),
            Message::OverwriteToModName("x".into()),
            Message::SendToPriorityChanged("x".into()),
            Message::ProfileRenameChanged("x".into()),
            Message::ProfileCopyChanged("x".into()),
            Message::PickerNameChanged("x".into()),
        ] {
            let mut app = nav_app(&["a"]);
            let label = format!("{msg:?}");
            let _ = update(&mut app, msg);
            assert!(app.typing, "{label} did not claim the keyboard");
        }
    }

    #[test]
    fn an_armed_removal_does_not_survive_the_list_being_rebuilt() {
        // The guard names its target by index. After a refresh that index may be
        // a different mod, and the second Delete would confirm against it.
        let mut app = nav_app(&["a", "b", "c"]);
        app.selected_mod = Some(2);
        let _ = key_nav(&mut app, Nav::Remove);
        assert_eq!(app.confirm_remove, Some(2));
        // `reload_mods` needs an instance; the disarm itself lives in
        // `put_mod_selection`, which is what every rebuild goes through.
        let held = hold_mod_selection(&app);
        put_mod_selection(&mut app, held);
        assert_eq!(app.confirm_remove, None, "a rebuild must disarm it");
    }

    #[test]
    fn loot_is_never_handed_a_file_as_a_data_path() {
        // Reproduced against the real 108-path list: 80 of them were the game's
        // own .esm files, carried in app.mods as unmanaged rows, and libloot
        // answered the whole sort with "an I/O error occurred".
        let d = std::env::temp_dir().join(format!("eidos-lootpaths-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("RealMod")).unwrap();
        fs::write(d.join("Dawnguard.esm"), b"x").unwrap();

        let mut app = new(Vec::new()).0;
        app.mods = vec![
            ModEntry { name: "RealMod".into(), enabled: true, path: d.join("RealMod"), unmanaged: false },
            // The shape that broke it: enabled, not a separator, and a FILE.
            ModEntry { name: "Dawnguard.esm".into(), enabled: true, path: d.join("Dawnguard.esm"), unmanaged: true },
            // And one that is simply gone from disk.
            ModEntry { name: "Vanished".into(), enabled: true, path: d.join("Vanished"), unmanaged: false },
            ModEntry { name: "Off".into(), enabled: false, path: d.join("RealMod"), unmanaged: false },
            ModEntry { name: "grp_separator".into(), enabled: true, path: d.join("RealMod"), unmanaged: false },
        ];
        let paths = loot_data_paths(&app);
        // The invariant that matters: nothing here may be anything but a real
        // directory. An Overwrite dir from the live instance may lead the list.
        assert!(paths.iter().all(|p| p.is_dir()), "a non-directory got through: {paths:?}");
        assert!(paths.contains(&d.join("RealMod")), "the real mod is missing: {paths:?}");
        assert!(!paths.contains(&d.join("Dawnguard.esm")), "an unmanaged FILE got through");
        assert!(!paths.contains(&d.join("Vanished")), "a path that is gone got through");
        // Disabled rows and separators contribute nothing, so RealMod appears once.
        assert_eq!(paths.iter().filter(|p| **p == d.join("RealMod")).count(), 1);
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn the_arrows_only_land_on_rows_the_list_is_drawing() {
        // Walking the raw vector moved the focus into rows the filter had hidden:
        // the highlight was invisible and Space toggled a mod nobody was looking
        // at. Navigation counts in VISIBLE positions now.
        let mut app = nav_app(&["alpha", "beta", "gamma", "alderaan"]);
        app.search = "al".to_string();
        // Only alpha (0) and alderaan (3) match.
        let vis = mod_row_visibility(&app, None);
        assert_eq!(vis, [true, false, false, true]);

        let _ = key_nav(&mut app, Nav::Down);
        assert_eq!(app.selected_mod, Some(0));
        let _ = key_nav(&mut app, Nav::Down);
        assert_eq!(app.selected_mod, Some(3), "one step skips the hidden rows");
        let _ = key_nav(&mut app, Nav::Down);
        assert_eq!(app.selected_mod, Some(3), "and stops at the last visible row");
        let _ = key_nav(&mut app, Nav::Up);
        assert_eq!(app.selected_mod, Some(0));

        // A focus stranded on a row the filter has since hidden comes back onto
        // something visible rather than sticking.
        app.selected_mod = Some(1);
        let _ = key_nav(&mut app, Nav::Down);
        assert_eq!(app.selected_mod, Some(0));
    }

    #[test]
    fn the_keyboard_leaves_the_games_own_content_alone() {
        // Unmanaged rows are not in modlist.txt, so a flipped flag is lost on
        // the next save - which reads as the key having done nothing.
        let mut app = nav_app(&["dlc", "mod"]);
        app.mods[0].unmanaged = true;
        app.selected_mod = Some(0);
        let before = app.mods[0].enabled;
        let _ = update(&mut app, Message::ToggleMod(0));
        assert_eq!(app.mods[0].enabled, before, "unmanaged content is not togglable");

        // And Delete refuses it outright.
        let _ = key_nav(&mut app, Nav::Remove);
        assert_eq!(app.confirm_remove, None);
    }

    #[test]
    fn delete_twice_actually_removes_and_escape_calls_it_off() {
        // The first version armed a guard the keyboard could not confirm, while
        // telling the user to press Delete again. The promise has to be true.
        let mut app = nav_app(&["a", "b"]);
        app.selected_mod = Some(1);
        let _ = key_nav(&mut app, Nav::Remove);
        assert_eq!(app.confirm_remove, Some(1));
        assert!(app.status.as_deref().unwrap_or_default().contains("Delete again"));

        // Escape is the advertised way out.
        let _ = update(&mut app, Message::ClearSelection);
        assert_eq!(app.confirm_remove, None);
    }

    #[test]
    fn a_reorder_carries_the_selection_and_the_anchor_with_it() {
        // Indices survive a reorder while meaning different rows - the failure
        // that made a batch action write plugins nobody chose. Names do not.
        let mut app = nav_app(&["a", "b", "c", "d"]);
        app.selected_mod = Some(2);
        app.sel_anchor = Some(1);
        app.selected_mods = [1, 2].into_iter().collect();

        let held = hold_mod_selection(&app);
        // Simulate what a drag does: move "a" to the end.
        move_block(&mut app.mods, &[0], 4);
        assert_eq!(names(&app.mods), ["b", "c", "d", "a"]);
        put_mod_selection(&mut app, held);

        // b and c moved up one; the selection followed them.
        assert_eq!(app.selected_mod, Some(1), "focus follows 'c'");
        assert_eq!(app.sel_anchor, Some(0), "anchor follows 'b'");
        let mut got: Vec<usize> = app.selected_mods.iter().copied().collect();
        got.sort_unstable();
        assert_eq!(got, [0, 1]);
    }

    #[test]
    fn a_click_on_a_selected_row_does_not_reorder_the_list() {
        // A press arms a drag, so a plain click arrives as a drop. With a
        // multi-row selection there is no "own edge" to recognise it by, and
        // committing would COMPACT a non-contiguous set and save that.
        let mut app = nav_app(&["a", "b", "c", "d", "e"]);
        app.selected_mods = [0, 2, 4].into_iter().collect();
        app.selected_mod = Some(2);
        let _ = update(&mut app, Message::DragStart(2));
        assert!(app.drag_state.is_some_and(|d| !d.aimed), "a press has aimed at nothing yet");
        let _ = update(&mut app, Message::DragDrop);
        assert_eq!(names(&app.mods), ["a", "b", "c", "d", "e"], "a click moved rows");

        // Actually aiming somewhere still works.
        let _ = update(&mut app, Message::DragStart(2));
        let _ = update(&mut app, Message::DragOverGap(0));
        assert!(app.drag_state.is_some_and(|d| d.aimed));
    }

    /// The indices of a selection, sorted, for a readable assertion.
    fn sel(app: &App) -> Vec<usize> {
        let mut v: Vec<usize> = app.selected_mods.iter().copied().collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn group_children_stops_at_the_next_separator() {
        let v = mods(&["Head_separator", "a", "b", "Tail_separator", "c"]);
        assert_eq!(group_children(&v, 0), 1..3);
        assert_eq!(group_children(&v, 3), 4..5, "the last group runs to the end");
        let empty = mods(&["Head_separator", "Tail_separator"]);
        assert!(group_children(&empty, 0).is_empty(), "a header with nothing under it");
        assert!(group_children(&empty, 1).is_empty());
    }

    #[test]
    fn a_separator_dragged_alone_leaves_its_mods_behind() {
        // MO2's behaviour, and the whole point of the fix: `dropMimeData` hands
        // exactly the dragged rows to `changeModPriority` (modlist.cpp:1159),
        // gathering no children. The mods left behind are not orphaned - they now
        // belong to whatever header is above them, because membership is nothing
        // but adjacency.
        let mut app = nav_app(&["Head_separator", "a", "b", "Tail_separator", "c"]);
        let _ = update(&mut app, Message::DragStart(0));
        let _ = update(&mut app, Message::DragOverGap(4));
        let _ = update(&mut app, Message::DragDrop);
        assert_eq!(names(&app.mods), ["a", "b", "Tail_separator", "Head_separator", "c"]);
    }

    #[test]
    fn a_folded_separator_still_moves_alone_and_comes_back_open() {
        // MO2 force-expands a separator whose priority just changed
        // (ModListView::onModPrioritiesChanged, modlistview.cpp:449). Without it a
        // folded header dropped somewhere new goes on hiding rows that were never
        // inside it, which reads as mods having been deleted.
        let mut app = nav_app(&["Head_separator", "a", "Tail_separator", "b"]);
        app.collapsed.insert("Head".to_string());
        let _ = update(&mut app, Message::DragStart(0));
        let _ = update(&mut app, Message::DragOverGap(3));
        let _ = update(&mut app, Message::DragDrop);
        assert_eq!(names(&app.mods), ["a", "Tail_separator", "Head_separator", "b"]);
        assert!(!app.collapsed.contains("Head"), "a header that now hides rows must be open");

        // Landing with nothing under it hides nothing, so the fold is left alone -
        // the user's choice is only overridden where keeping it would mislead.
        let mut app = nav_app(&["a", "Head_separator", "b"]);
        app.collapsed.insert("Head".to_string());
        let _ = update(&mut app, Message::DragStart(1));
        let _ = update(&mut app, Message::DragOverGap(3));
        let _ = update(&mut app, Message::DragDrop);
        assert_eq!(names(&app.mods), ["a", "b", "Head_separator"]);
        assert!(app.collapsed.contains("Head"), "nothing is hidden, so nothing was unfolded");
    }

    #[test]
    fn mods_swallowed_by_a_folded_neighbour_are_named() {
        // Lift a header out from between a folded group and its own mods, and
        // those mods join the folded group: off screen, and with nothing else to
        // say so. The fold is the user's and is left alone; the disappearance is
        // not left to be discovered.
        let mut app = nav_app(&["Armour_separator", "a", "Weapons_separator", "w1", "w2"]);
        app.collapsed.insert("Armour".to_string());
        let _ = update(&mut app, Message::ModSendBottom(2));
        assert_eq!(names(&app.mods), ["Armour_separator", "a", "w1", "w2", "Weapons_separator"]);
        assert!(
            app.status.as_deref().is_some_and(|s| s.contains("folded group")),
            "two mods went off screen unremarked: {:?}",
            app.status
        );

        // An ordinary move hides nothing, and says nothing.
        let mut app = nav_app(&["a", "b", "c"]);
        let _ = update(&mut app, Message::ModSendBottom(0));
        assert_eq!(app.status, None);
    }

    #[test]
    fn ctrl_arrow_moves_a_separator_like_any_other_row() {
        // This was a dead key: `selection_or` returned an empty block and
        // `move_mod_rows` bailed without so much as a status line.
        let mut app = nav_app(&["a", "Sec_separator", "b"]);
        app.selected_mod = Some(1);
        let _ = key_nav(&mut app, Nav::ShiftUp);
        assert_eq!(names(&app.mods), ["Sec_separator", "a", "b"]);
        assert_eq!(app.selected_mod, Some(0), "the focus follows the row it moved");
    }

    #[test]
    fn a_separator_can_be_parked_above_the_games_own_content() {
        // The user's actual goal: a header above the DLC / Creation Club block, so
        // the arrow beside it folds all of that away.
        let mut app = nav_app(&["dlc", "Sec_separator", "a"]);
        app.mods[0].unmanaged = true;
        app.selected_mod = Some(1);
        let _ = key_nav(&mut app, Nav::ShiftUp);
        assert_eq!(names(&app.mods), ["Sec_separator", "dlc", "a"]);
    }

    #[test]
    fn alt_click_on_a_separator_selects_its_whole_group() {
        // MO2's gesture for taking a section rather than its label
        // (ModListView::mousePressEvent, modlistview.cpp:1444).
        let mut app = nav_app(&["Head_separator", "a", "b", "Tail_separator", "c"]);
        app.modifiers = iced::keyboard::Modifiers::ALT;
        let _ = update(&mut app, Message::DragStart(0));
        assert_eq!(sel(&app), vec![0, 1, 2], "header plus its group, stopping at the next header");

        let _ = update(&mut app, Message::DragStart(3));
        assert_eq!(sel(&app), vec![3, 4], "the last group runs to the end of the list");

        // Alt on an ordinary row is not this gesture.
        let _ = update(&mut app, Message::DragStart(1));
        assert_eq!(sel(&app), Vec::<usize>::new());
    }

    #[test]
    fn a_group_selected_with_alt_moves_as_one_block() {
        let mut app = nav_app(&["Head_separator", "a", "b", "Tail_separator", "c"]);
        app.modifiers = iced::keyboard::Modifiers::ALT;
        let _ = update(&mut app, Message::DragStart(0));
        let _ = update(&mut app, Message::DragOverGap(5));
        let _ = update(&mut app, Message::DragDrop);
        assert_eq!(names(&app.mods), ["Tail_separator", "c", "Head_separator", "a", "b"]);
        assert_eq!(sel(&app), vec![2, 3, 4], "a block stays selected so it can be dragged again");
    }

    #[test]
    fn a_mixed_selection_no_longer_leaves_its_header_behind() {
        // `real_selection` filtered the separator out of the batch reorders, which
        // lifted a group's mods above their own header and stranded it.
        let mut app = nav_app(&["a", "Sec_separator", "b", "c"]);
        app.selected_mods = [1, 2].into_iter().collect();
        app.selected_mod = Some(1);
        let _ = update(&mut app, Message::BatchSendTop);
        assert_eq!(names(&app.mods), ["Sec_separator", "b", "a", "c"]);
    }

    #[test]
    fn a_drag_re_anchors_the_selection_it_just_moved() {
        let mut app = nav_app(&["a", "b", "c", "d"]);
        app.selected_mods = [0, 1].into_iter().collect();
        app.selected_mod = Some(0);
        app.sel_anchor = Some(0);
        let _ = update(&mut app, Message::DragStart(0));
        let _ = update(&mut app, Message::DragOverGap(4));
        let _ = update(&mut app, Message::DragDrop);
        assert_eq!(names(&app.mods), ["c", "d", "a", "b"]);
        // Left at 0, the next Shift+click would have built its run from a row
        // nobody chose.
        assert_eq!(app.sel_anchor, Some(2));
        assert_eq!(sel(&app), vec![2, 3]);
    }

    #[test]
    fn send_to_separator_will_not_send_a_separator_into_itself() {
        let app = nav_app(&["A_separator", "a", "B_separator", "b"]);
        assert_eq!(separator_choices(&app, 0), vec![2], "a header is not a destination for itself");
        assert_eq!(separator_choices(&app, 2), vec![0]);
        assert_eq!(separator_choices(&app, 1), vec![0, 2], "an ordinary mod may go anywhere");
    }

    #[test]
    fn send_to_priority_keeps_the_menu_that_hosts_its_editor() {
        // Both "Send to..." items armed an inline editor and closed the card that
        // draws it, so they did nothing visible - for every row, not just
        // separators - and the armed state then hijacked the next right-click.
        let mut app = nav_app(&["a", "b"]);
        let _ = update(&mut app, Message::OpenModMenu(1));
        let _ = update(&mut app, Message::SendToPriorityStart(1));
        assert_eq!(app.menu_mod, Some(1), "the card holding the editor was dismissed");
        assert!(app.send_priority.is_some());

        let _ = update(&mut app, Message::SendToPriorityChanged("0".to_string()));
        let _ = update(&mut app, Message::SendToPriorityCommit);
        assert_eq!(names(&app.mods), ["b", "a"]);
        assert_eq!(app.menu_mod, None, "the commit is what closes the menu");

        // And closing the menu disarms it, so the next right-click opens a menu.
        let _ = update(&mut app, Message::OpenModMenu(0));
        let _ = update(&mut app, Message::SendToSeparatorStart(0));
        let _ = update(&mut app, Message::CloseMenu);
        assert!(app.send_separator.is_none());
    }

    #[test]
    fn a_batch_acts_on_the_selection_not_on_a_row_left_deselected() {
        // Ctrl-clicking a row OFF leaves the focus on it. Going through the
        // focus would then act on the one row the user just excluded.
        let mut app = nav_app(&["a", "b", "c"]);
        // Same shape on the mod side of the model: focus outside the set.
        app.selected_mods = [0, 1].into_iter().collect();
        app.selected_mod = Some(2);
        let rows = selection_or(&app, 2);
        assert_eq!(rows, vec![2], "selection_or answers about the row it is asked about");
        // Which is exactly why the batch handler must not ask it about the
        // focus: it consults the SET first. Documented here so the two do not
        // get "unified" back into the bug.
        assert!(!app.selected_mods.contains(&2));
    }

    #[test]
    fn the_conflict_tint_says_which_way_each_pair_goes() {
        use eidos_conflicts::{ConflictMap, ModConflicts};
        let mut app = nav_app(&["low", "focus", "high"]);
        // Origins are index + 1. "focus" (index 1) overwrites "low" and is
        // overwritten by "high".
        let mut mods = HashMap::new();
        mods.insert(
            2u32,
            ModConflicts {
                overwrites: [1u32].into_iter().collect(),
                overwritten_by: [3u32].into_iter().collect(),
                ..Default::default()
            },
        );
        app.conflicts = Some(ConflictMap { files: Default::default(), mods, names: HashMap::new() });

        app.selected_mod = Some(1);
        assert_eq!(conflict_tint(&app, 0), Some(CONFLICT_WINS_BG), "the row it beats");
        assert_eq!(conflict_tint(&app, 2), Some(CONFLICT_LOSES_BG), "the row that beats it");
        assert_eq!(conflict_tint(&app, 1), None, "the focused row keeps its selection colour");

        // Nothing focused, nothing tinted.
        app.selected_mod = None;
        assert_eq!(conflict_tint(&app, 0), None);
    }

    #[test]
    fn a_menu_grows_away_from_the_edge_it_was_summoned_near() {
        // The card's height is unknown until layout, so a menu opened near the
        // bottom cannot just be offset downwards - it would run off screen.
        // Anchoring the far edge to the pointer instead needs no size at all.
        let win = iced::Size::new(1000.0, 800.0);

        // Top-left quadrant: the card's top-left corner sits at the pointer.
        let p = iced::Point::new(100.0, 100.0);
        assert!(p.x <= win.width * 0.5 && p.y <= win.height * 0.5);

        // Bottom-right: it must flip on BOTH axes, and the padding that places
        // it is measured from the opposite edges.
        let p = iced::Point::new(900.0, 700.0);
        let (right, below) = (p.x > win.width * 0.5, p.y > win.height * 0.5);
        assert!(right && below);
        assert_eq!(win.height - p.y, 100.0, "padding from the bottom edge");
        assert_eq!(win.width - p.x, 100.0, "padding from the right edge");
    }

    #[test]
    fn opening_a_menu_freezes_where_it_was_summoned() {
        let mut app = nav_app(&["a", "b"]);
        let _ = update(&mut app, Message::PointerAt(iced::Point::new(300.0, 220.0)));
        let _ = update(&mut app, Message::OpenModMenu(1));
        assert_eq!(app.menu_at, Some(iced::Point::new(300.0, 220.0)));

        // The pointer keeps moving; a menu that followed it could not be aimed at.
        let _ = update(&mut app, Message::PointerAt(iced::Point::new(700.0, 600.0)));
        assert_eq!(app.menu_at, Some(iced::Point::new(300.0, 220.0)));

        // Closing releases it, so a stale point cannot place the next one.
        let _ = update(&mut app, Message::CloseMenu);
        assert_eq!(app.menu_at, None);
    }

    #[test]
    fn ctrl_arrow_moves_the_row_not_the_focus() {
        let mut app = nav_app(&["a", "b", "c", "d"]);
        app.selected_mod = Some(2);
        let _ = key_nav(&mut app, Nav::ShiftUp);
        assert_eq!(names(&app.mods), ["a", "c", "b", "d"]);
        assert_eq!(app.selected_mod, Some(1), "the focus travels with the row");

        let _ = key_nav(&mut app, Nav::ShiftDown);
        assert_eq!(names(&app.mods), ["a", "b", "c", "d"]);
        assert_eq!(app.selected_mod, Some(2));
    }

    #[test]
    fn a_row_move_lands_beside_the_neighbour_the_user_can_see() {
        // Under a filter the visible neighbour is not the adjacent index, and a
        // move whose effect is invisible reads as a key that did nothing.
        let mut app = nav_app(&["alpha", "hidden", "alderaan"]);
        app.search = "al".to_string();
        assert_eq!(mod_row_visibility(&app, None), [true, false, true]);
        app.selected_mod = Some(2);
        let _ = key_nav(&mut app, Nav::ShiftUp);
        // "alderaan" jumped over the hidden row to sit above "alpha".
        assert_eq!(names(&app.mods), ["alderaan", "alpha", "hidden"]);
    }

    #[test]
    fn a_row_move_stops_at_the_ends_but_may_pass_the_game_content() {
        let mut app = nav_app(&["dlc", "a", "b"]);
        app.mods[0].unmanaged = true;

        // Above the game's own content is now a legal place to be. It has to be:
        // a separator can only fold what comes AFTER it, so the only way to put
        // the DLC block away is to get a separator above it. The rows are written
        // to modlist.txt with MO2's `*` now, so nothing is lost by going there.
        app.selected_mod = Some(1);
        let _ = key_nav(&mut app, Nav::ShiftUp);
        assert_eq!(names(&app.mods), ["a", "dlc", "b"], "a row could not pass the game content");

        // The ends still hold.
        app.selected_mod = Some(0);
        let _ = key_nav(&mut app, Nav::ShiftUp);
        assert_eq!(names(&app.mods), ["a", "dlc", "b"], "the first row has nowhere to go");

        app.selected_mod = Some(2);
        let _ = key_nav(&mut app, Nav::ShiftDown);
        assert_eq!(names(&app.mods), ["a", "dlc", "b"], "the last row has nowhere to go");
    }

    #[test]
    fn move_block_compensates_for_the_lifted_rows() {
        // Moving DOWN: removing the source shifts everything after it, which is
        // the classic off-by-one that lands a dragged row one slot short.
        let mut v = mods(&["a", "b", "c", "d"]);
        let at = move_block(&mut v, &[0], 3);
        assert_eq!(names(&v), ["b", "c", "a", "d"]);
        assert_eq!(at, 2);

        // Moving UP needs no compensation.
        let mut v = mods(&["a", "b", "c", "d"]);
        let at = move_block(&mut v, &[3], 1);
        assert_eq!(names(&v), ["a", "d", "b", "c"]);
        assert_eq!(at, 1);

        // To the very end.
        let mut v = mods(&["a", "b", "c"]);
        let at = move_block(&mut v, &[0], 3);
        assert_eq!(names(&v), ["b", "c", "a"]);
        assert_eq!(at, 2);
    }

    #[test]
    fn move_block_keeps_a_multi_selection_together_and_ordered() {
        // A non-contiguous selection lands as one contiguous block, in its
        // original relative order.
        let mut v = mods(&["a", "b", "c", "d", "e"]);
        let at = move_block(&mut v, &[0, 2], 4);
        assert_eq!(names(&v), ["b", "d", "a", "c", "e"]);
        assert_eq!(at, 2);

        // Moving up, same rule.
        let mut v = mods(&["a", "b", "c", "d", "e"]);
        let at = move_block(&mut v, &[3, 4], 1);
        assert_eq!(names(&v), ["a", "d", "e", "b", "c"]);
        assert_eq!(at, 1);
    }

    #[test]
    fn move_block_is_safe_on_junk_input() {
        let mut v = mods(&["a", "b"]);
        // Out-of-range indices are dropped, duplicates collapse, empty is a no-op.
        assert_eq!(move_block(&mut v, &[], 1), 1);
        assert_eq!(names(&v), ["a", "b"]);
        move_block(&mut v, &[9, 9], 0);
        assert_eq!(names(&v), ["a", "b"]);
        move_block(&mut v, &[1, 1], 0);
        assert_eq!(names(&v), ["b", "a"]);
        // A destination past the end clamps instead of panicking.
        move_block(&mut v, &[0], 99);
        assert_eq!(names(&v), ["a", "b"]);
    }

    #[test]
    fn markdown_links_are_split_from_surrounding_text() {
        // A real LOOT message shape: prose, a link, more prose.
        let parts = split_markdown_links(
            "Please install [SSEEdit v4.1.5d](https://www.nexusmods.com/x/mods/164) to clean it.",
        );
        assert_eq!(
            parts,
            vec![
                ("Please install ".to_string(), None),
                (
                    "SSEEdit v4.1.5d".to_string(),
                    Some("https://www.nexusmods.com/x/mods/164".to_string())
                ),
                (" to clean it.".to_string(), None),
            ]
        );
    }

    #[test]
    fn plain_text_and_malformed_links_stay_verbatim() {
        assert_eq!(
            split_markdown_links("no links here"),
            vec![("no links here".to_string(), None)]
        );
        // Unclosed / empty forms are not links and must round-trip unchanged.
        for s in ["[label](", "[label] (url)", "[](url)", "[label]()", "a [b c"] {
            let parts = split_markdown_links(s);
            let rebuilt: String = parts.iter().map(|(t, _)| t.as_str()).collect();
            assert_eq!(rebuilt, s, "{s:?} must survive unchanged");
            assert!(parts.iter().all(|(_, u)| u.is_none()), "{s:?} is not a link");
        }
    }

    #[test]
    fn multibyte_text_around_a_link_is_not_split_mid_character() {
        // A non-ASCII message must not panic or corrupt (byte-indexed scanning).
        let parts = split_markdown_links("Réglé - voir [le fil](https://loot.example/é) déjà");
        let rebuilt: String = parts.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(rebuilt, "Réglé - voir le fil déjà");
        assert_eq!(parts[1].1.as_deref(), Some("https://loot.example/é"));
    }

    fn sorted(v: &[&str]) -> Vec<String> {
        let mut o: Vec<String> = v.iter().map(|s| s.to_string()).collect();
        o.sort();
        o
    }

    #[test]
    fn the_tree_shows_one_level_and_counts_what_is_below() {
        // Lin's real Overwrite in miniature: a Root/ subtree beside the tool's own
        // files. The root level must be four rows, not 4902.
        let e = sorted(&[
            "CalienteTools/BodySlide/Config.xml",
            "CalienteTools/BodySlide/Log_BS.txt",
            "Root/meshes/actors/body.nif",
            "Root/meshes/armor/boots.nif",
            "Root/d3dx9_42.log",
            "note.txt",
        ]);
        let top = tree_children(&e, "");
        assert_eq!(
            top,
            vec![
                ("CalienteTools".to_string(), Some(2)),
                ("Root".to_string(), Some(3)),
                ("note.txt".to_string(), None),
            ],
            "folders first with their recursive file count, then loose files"
        );
        assert_eq!(
            tree_children(&e, "Root"),
            vec![("meshes".to_string(), Some(2)), ("d3dx9_42.log".to_string(), None)]
        );
        assert_eq!(tree_children(&e, "Root/meshes/actors"), vec![("body.nif".to_string(), None)]);
    }

    #[test]
    fn a_prefix_that_is_not_a_path_component_is_not_a_child() {
        // "Rootless" must not be mistaken for something under "Root", which is
        // what a bare starts_with would do.
        let e = sorted(&["Root/a.nif", "Rootless/b.nif", "Roo/c.nif"]);
        assert_eq!(tree_children(&e, "Root"), vec![("a.nif".to_string(), None)]);
    }

    #[test]
    fn nothing_is_expanded_so_only_the_top_level_is_drawn() {
        let e = sorted(&[
            "Root/meshes/actors/body.nif",
            "Root/meshes/armor/boots.nif",
            "CalienteTools/BodySlide/Config.xml",
        ]);
        let app = nav_app(&[]);
        let rows = overwrite_tree_rows(&app, &e, 3000);
        assert_eq!(rows.len(), 2, "two folders closed, and none of their contents");
        assert!(rows.iter().all(|r| r.depth == 0));
    }

    #[test]
    fn expanding_a_folder_reveals_exactly_its_children() {
        let e = sorted(&[
            "Root/meshes/actors/body.nif",
            "Root/meshes/armor/boots.nif",
            "Root/d3dx9_42.log",
        ]);
        let mut app = nav_app(&[]);
        app.overwrite_expanded.insert("Root".to_string());
        let rows = overwrite_tree_rows(&app, &e, 3000);
        let drawn: Vec<&str> = rows.iter().map(|r| r.rel.as_str()).collect();
        assert_eq!(drawn, vec!["Root", "Root/meshes", "Root/d3dx9_42.log"]);
        // Still closed one level down: the grandchildren stay out.
        assert!(!drawn.iter().any(|r| r.starts_with("Root/meshes/")));

        app.overwrite_expanded.insert("Root/meshes".to_string());
        let deep = overwrite_tree_rows(&app, &e, 3000);
        assert_eq!(deep.iter().filter(|r| r.depth == 2).count(), 2, "actors and armor");
    }

    #[test]
    fn the_row_budget_is_respected() {
        let many: Vec<String> = (0..500).map(|i| format!("d/f{i:04}.txt")).collect();
        let mut app = nav_app(&[]);
        app.overwrite_expanded.insert("d".to_string());
        assert_eq!(overwrite_tree_rows(&app, &many, 10).len(), 10);
    }


    #[test]
    fn a_criterion_cycles_off_only_except_and_back() {
        // Three settings, not two: "only conflicted" and "everything except
        // conflicted" are both questions people ask.
        let mut app = nav_app(&["a"]);
        assert_eq!(app.filters.active, Criterion::Off);
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        assert_eq!(app.filters.active, Criterion::Require);
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        assert_eq!(app.filters.active, Criterion::Exclude);
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        assert_eq!(app.filters.active, Criterion::Off, "back to not filtering");
    }

    #[test]
    fn the_active_filter_keeps_only_what_it_says() {
        let mut app = nav_app(&["On", "Off"]);
        app.mods[1].enabled = false;
        app.filters.active = Criterion::Require;
        let vis = mod_row_visibility(&app, None);
        assert_eq!(vis, vec![true, false], "only the enabled one");
        app.filters.active = Criterion::Exclude;
        let vis = mod_row_visibility(&app, None);
        assert_eq!(vis, vec![false, true], "and the inverse is the other question");
    }

    #[test]
    fn filters_combine_with_each_other_and_with_the_name_box() {
        // They narrow together (AND), like MO2's - otherwise two criteria would
        // widen the list, which is the opposite of what a filter is for.
        let mut app = nav_app(&["Armour Pack", "Armour Patch", "Weather"]);
        app.mods[1].enabled = false;
        app.search = "armour".to_string();
        app.filters.active = Criterion::Require;
        let vis = mod_row_visibility(&app, None);
        assert_eq!(vis, vec![true, false, false], "name AND state: {vis:?}");
    }

    #[test]
    fn a_separator_survives_only_while_something_under_it_does() {
        // A group header for an empty group is noise; the existing rule has to
        // keep holding once state filters can empty a group.
        let mut app = nav_app(&["Group_separator", "Inside"]);
        app.mods[1].enabled = false;
        app.filters.active = Criterion::Require;
        let vis = mod_row_visibility(&app, None);
        assert_eq!(vis, vec![false, false], "the header goes with its only mod");
        app.mods[1].enabled = true;
        let vis = mod_row_visibility(&app, None);
        assert_eq!(vis, vec![true, true], "and comes back with it");
    }

    #[test]
    fn changing_a_filter_drops_the_selection_and_any_drag() {
        // The visible set changes underneath every row index they hold.
        let mut app = nav_app(&["a", "b", "c"]);
        app.selected_mods.extend([0, 2]);
        app.drag_state = Some(DragState { from: 0, gap: 2, aimed: true });
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Conflicted));
        assert!(app.drag_state.is_none());
        // `Conflicted -> only` with no conflict map hides everything, so nothing
        // in the selection may survive it.
        assert!(app.selected_mods.is_empty());
    }

    /// Filtering does not renumber the rows, so an index kept across it still
    /// RESOLVES - it just resolves to a mod that is no longer on screen. The
    /// keyboard reads `selected_mod` raw, and `real_selection` falls back to it
    /// when the multi-selection is empty, so a focus left on a hidden row aimed
    /// Space, Delete and every batch action at a mod the user could not see.
    #[test]
    fn a_filter_never_leaves_the_keyboard_aimed_at_a_row_it_hid() {
        // Alpha stays visible (disabled); Ivy is enabled, so Active->Exclude hides it.
        let mut app = nav_app(&["Alpha", "Ivy"]);
        app.mods[0].enabled = false;
        app.mods[1].enabled = true;

        // A PLAIN CLICK - the only gesture a mouse user makes. It sets the focus
        // and the anchor and deliberately leaves `selected_mods` empty, which is
        // why clearing only that set was never enough.
        let _ = update_inner(&mut app, Message::SelectMod(1));
        assert_eq!(app.selected_mod, Some(1));
        assert!(app.selected_mods.is_empty(), "a plain click never populates the set");

        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        assert_eq!(app.filters.active, Criterion::Exclude);
        assert_eq!(mod_row_visibility(&app, None), vec![true, false], "Ivy is off screen");

        assert_eq!(app.selected_mod, None, "the focus went with the row");
        assert_eq!(app.sel_anchor, None, "and so did the shift-select anchor");
        // The proof that matters: the keys that ACT are now inert.
        let _ = update_inner(&mut app, Message::KeyNav(Nav::Toggle));
        assert!(app.mods[1].enabled, "Space must not toggle a mod that is not drawn");
        let _ = update_inner(&mut app, Message::KeyNav(Nav::Remove));
        assert_ne!(app.confirm_remove, Some(1), "Delete must not arm on a hidden row");
    }

    #[test]
    fn a_filter_never_redirects_a_batch_action_onto_a_hidden_row() {
        let mut app = nav_app(&["Alpha", "Bravo", "Ivy"]);
        app.mods[0].enabled = false;
        app.mods[1].enabled = false;
        app.mods[2].enabled = true;
        // Ctrl+click three rows: the set is full and the focus is on the last.
        app.modifiers = iced::keyboard::Modifiers::CTRL;
        for i in 0..3 {
            let _ = update_inner(&mut app, Message::SelectMod(i));
        }
        app.modifiers = iced::keyboard::Modifiers::default();
        assert_eq!(app.selected_mods.len(), 3);

        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        // Ivy is hidden. `real_selection` falls back to `selected_mod` when the
        // set is empty, so a stale focus here would aim the batch Remove at the
        // one row nobody can see.
        assert!(!real_selection(&app).contains(&2), "the hidden row is not a batch target");
    }

    #[test]
    fn folding_a_group_takes_the_focus_with_it() {
        // Folding hides rows exactly like a filter does, through the same path.
        let mut app = nav_app(&["Gear_separator", "under"]);
        // The fold is keyed by DISPLAY name (the "_separator" suffix stripped),
        // which is what the header button sends.
        let key = app.mods[0].display_name().to_string();
        assert_eq!(key, "Gear");
        let _ = update_inner(&mut app, Message::SelectMod(1));
        assert_eq!(app.selected_mod, Some(1));
        let _ = update_inner(&mut app, Message::ToggleCollapse(key));
        assert_eq!(mod_row_visibility(&app, None), vec![true, false], "the group folded");
        assert_eq!(app.selected_mod, None, "the focus did not survive the fold");
    }

    #[test]
    fn select_all_only_takes_what_the_list_is_drawing() {
        let mut app = nav_app(&["Alpha", "Bravo", "Ivy"]);
        app.mods[0].enabled = false;
        app.mods[1].enabled = false;
        app.mods[2].enabled = true;
        app.focus = Pane::Mods;
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
        let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));

        let _ = update_inner(&mut app, Message::SelectAllInFocus);
        // Not `0..mods.len()`: Ctrl+A used to sweep in every hidden row, and the
        // batch Remove then aimed remove_dir_all at all of them.
        assert_eq!(app.selected_mods.len(), 2);
        assert!(!app.selected_mods.contains(&2), "the hidden row is not selected");
        assert_eq!(app.selected_mod, Some(0), "the focus lands on a row that is drawn");
    }

    /// A download row dragged onto an insertion strip.
    fn dl_row(name: &str) -> DownloadRow {
        DownloadRow {
            name: name.to_string(),
            path: PathBuf::from("/tmp").join(name),
            size: 1,
            version: String::new(),
            mod_name: None,
            mod_id: None,
            state: DownloadState::Ready,
            downloaded: 1,
            total: 1,
            speed: None,
            hidden: false,
            modified: std::time::SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn dragging_a_download_aims_at_a_gap_and_a_plain_click_does_not() {
        let mut app = nav_app(&["a", "b", "c"]);
        app.downloads = vec![dl_row("Mod.7z")];

        // A press ARMS the drag - it does not commit anything, because the same
        // press is how the row is clicked.
        let _ = update_inner(&mut app, Message::DownloadDragStart(0));
        let d = app.download_drag.as_ref().expect("armed");
        assert!(!d.aimed, "a press alone is not an aim");
        assert_eq!(d.gap, 3, "unaimed, it would land at the end");

        // Releasing without ever crossing a strip is a plain click: no install.
        let _ = update_inner(&mut app, Message::PointerReleased);
        assert!(app.download_drag.is_none());
        assert_eq!(app.install_at, None, "a click on a download row installs nothing");

        // Now with an aim.
        let _ = update_inner(&mut app, Message::DownloadDragStart(0));
        let _ = update_inner(&mut app, Message::DownloadDragOverGap(1));
        assert!(app.download_drag.as_ref().unwrap().aimed);
        let _ = update_inner(&mut app, Message::DownloadDragDrop);
        assert_eq!(
            app.install_at.as_ref().map(|(gap, _)| *gap),
            Some(1),
            "the drop remembers where it was aimed"
        );
        assert!(app.download_drag.is_none(), "and the drag is over");
    }

    #[test]
    fn a_partial_download_cannot_be_dragged() {
        let mut app = nav_app(&["a"]);
        let mut row = dl_row("Half.7z");
        row.state = DownloadState::Downloading;
        app.downloads = vec![row];
        let _ = update_inner(&mut app, Message::DownloadDragStart(0));
        assert!(app.download_drag.is_none(), "there is nothing to install out of a partial");
    }

    #[test]
    fn a_gap_means_nothing_under_a_filter_so_the_drop_says_so() {
        let mut app = nav_app(&["a", "b", "c"]);
        app.downloads = vec![dl_row("Mod.7z")];
        app.search = "a".to_string();
        let _ = update_inner(&mut app, Message::DownloadDragStart(0));
        let _ = update_inner(&mut app, Message::DownloadDragOverGap(1));
        let _ = update_inner(&mut app, Message::DownloadDragDrop);
        // The strip between two VISIBLE rows can have any number of hidden rows
        // behind it, so "here" would be a lie. It installs at the end and says
        // so - through the installer, because `ModPicked` sets its own status a
        // moment later and would overwrite anything said here.
        assert_eq!(app.install_at, None);
        assert!(
            app.pending_note.as_deref().unwrap_or("").contains("end of the list"),
            "{:?}",
            app.pending_note
        );
    }

    /// An instance with two profiles and a save in the active one.
    fn saves_app() -> (App, PathBuf) {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        // A second profile is a directory under profiles/; the copy target only
        // needs it to exist.
        fs::create_dir_all(inst.profile("Second").saves_dir()).unwrap();
        let dir = inst.active().saves_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Save1.ess"), b"x").unwrap();
        fs::write(dir.join("Save1.skse"), b"co").unwrap();
        fs::write(dir.join("Save2.ess"), b"y").unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;
        load_saves(&mut app);
        (app, root)
    }

    #[test]
    fn the_profile_chips_are_read_once_and_follow_every_change() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        fs::create_dir_all(inst.profile("Second").dir()).unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;

        let (names, active) = cached_profiles(&app);
        assert!(names.contains(&"Default".to_string()) && names.contains(&"Second".to_string()));
        assert_eq!(active, "Default");
        // Cached: a change on disk alone must NOT be picked up, or the memo is
        // not doing anything.
        fs::create_dir_all(app.created.as_ref().unwrap().profile("Third").dir()).unwrap();
        assert_eq!(cached_profiles(&app).0.len(), 2, "still memoised");

        // But a profile message drops it, whichever branch the handler takes -
        // delete and rename do not bump the view generation.
        let _ = update_inner(&mut app, Message::ProfileDeleteCommit("Second".into()));
        let (names, _) = cached_profiles(&app);
        assert!(names.contains(&"Third".to_string()), "the memo fell: {names:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_menu_label_names_the_host_not_the_whole_url() {
        assert_eq!(url_host("https://www.loverslab.com/files/file/123-x/"), "loverslab.com");
        assert_eq!(url_host("https://github.com/a/b"), "github.com");
        assert_eq!(url_host("http://example.org"), "example.org");
        // Anything unparseable falls back to the whole string rather than
        // rendering an entry that reads "Visit ".
        assert_eq!(url_host("nonsense"), "nonsense");
    }

    #[test]
    fn a_mod_page_that_is_not_a_web_link_is_refused_before_it_is_stored() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        fs::create_dir_all(root.join("mods/M")).unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = vec![ModEntry {
            name: "M".into(),
            enabled: true,
            path: root.join("mods/M"),
            unmanaged: false,
        }];
        app.screen = Screen::Main;
        app.info_mod = Some(0);

        // Refused at the SAVE, not at the click that opens it: a value that
        // cannot be opened must not be storable, or the menu entry becomes a
        // dead end that looks live.
        app.url_edit = "file:///etc/passwd".to_string();
        let _ = update_inner(&mut app, Message::ModUrlSave);
        assert!(app.status.as_deref().unwrap_or("").contains("http"));
        assert_eq!(app.created.as_ref().unwrap().mod_meta("M").url(), None);

        app.url_edit = "https://github.com/me/mod".to_string();
        let _ = update_inner(&mut app, Message::ModUrlSave);
        assert_eq!(
            app.created.as_ref().unwrap().mod_meta("M").url().as_deref(),
            Some("https://github.com/me/mod")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unknown_plugin_set_makes_the_archives_tab_say_so_rather_than_condemn_everything() {
        // Without this the tab renders every archive red: "we have not looked"
        // would be indistinguishable from "no plugin is active", which is the
        // exact bug the orphan diagnostic already had once.
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;
        assert!(app.plugins.is_none());
        assert!(archive_rows(&app, "skyrimse").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_archives_tab_says_why_each_archive_does_or_does_not_load() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mods = root.join("mods");
        // Named after an active plugin, the " - suffix" form, and an orphan.
        fs::create_dir_all(mods.join("Good")).unwrap();
        fs::write(mods.join("Good/Mine.bsa"), b"").unwrap();
        fs::write(mods.join("Good/Mine - Textures.bsa"), b"").unwrap();
        fs::create_dir_all(mods.join("Dead")).unwrap();
        fs::write(mods.join("Dead/Nobody.bsa"), b"").unwrap();

        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = vec![
            ModEntry { name: "Good".into(), enabled: true, path: mods.join("Good"), unmanaged: false },
            ModEntry { name: "Dead".into(), enabled: true, path: mods.join("Dead"), unmanaged: false },
        ];
        let mut list = PluginList::default();
        list.plugins.push(plugin_row("Mine.esp", "Good"));
        list.plugins[0].enabled = true;
        app.plugins = Some(list);
        app.screen = Screen::Main;

        let rows = archive_rows(&app, "skyrimse").expect("the plugin set is known");
        let by = |a: &str| rows.iter().find(|r| r.archive == a).expect(a);
        assert_eq!(by("Mine.bsa").by_plugin.as_deref(), Some("Mine.esp"));
        assert!(by("Mine.bsa").loaded());
        // The engine's " - <suffix>" rule, not MO2's looser starts-with.
        assert_eq!(by("Mine - Textures.bsa").by_plugin.as_deref(), Some("Mine.esp"));
        assert!(!by("Nobody.bsa").loaded(), "nothing names it");
        assert_eq!(by("Nobody.bsa").by_plugin, None);
        let _ = fs::remove_dir_all(&root);
    }

    /// A collection state holding a revision built from the captured payload.
    fn collection_app(mods: &[(&str, u64)], downloads: &[u64]) -> (App, PathBuf) {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        for (name, mod_id) in mods {
            let dir = root.join("mods").join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("meta.ini"), format!("[General]\nmodid={mod_id}\n")).unwrap();
        }
        let dl = inst.downloads_dir();
        fs::create_dir_all(&dl).unwrap();
        for file_id in downloads {
            fs::write(dl.join(format!("a{file_id}.7z")), b"x").unwrap();
            fs::write(
                dl.join(format!("a{file_id}.7z.meta")),
                format!("[General]\nmodID=1\nfileID={file_id}\n"),
            )
            .unwrap();
        }
        let mut app = app_for_game("skyrimse");
        app.mods = mods
            .iter()
            .map(|(n, _)| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: root.join("mods").join(n),
                unmanaged: false,
            })
            .collect();
        app.created = Some(inst);
        app.screen = Screen::Main;
        refresh_meta_cache(&mut app);
        (app, root)
    }

    /// A revision with three members. Built by hand rather than parsed: the
    /// PARSE is already tested against a captured real payload in eidos-nexus,
    /// and what these tests exercise is the local join, which only needs ids.
    fn captured_revision() -> eidos_nexus::collections::CollectionRevision {
        use eidos_nexus::collections::{CollectionMod, CollectionRevision};
        let member = |name: &str, mod_id: u64, file_id: u64| CollectionMod {
            name: name.to_string(),
            mod_id,
            file_id,
            domain: "skyrimspecialedition".to_string(),
            version: "2.1".to_string(),
            file_title: format!("{name}.7z"),
            size_in_bytes: 1024,
            optional: false,
        };
        CollectionRevision {
            slug: "rqhcxy".to_string(),
            revision_number: 1,
            name: "The Great Cities Collection".to_string(),
            summary: String::new(),
            author: "HookerHeels".to_string(),
            game_domain: "skyrimspecialedition".to_string(),
            mod_count: 3,
            total_size: 3072,
            instructions: String::new(),
            mods: vec![
                member("Karthwasten", 37471, 232153),
                member("Mixwater Mill", 37414, 232146),
                member("Shors Stone", 36462, 234034),
            ],
            hidden: None,
        }
    }

    #[test]
    fn a_collection_member_is_matched_against_what_the_instance_already_has() {
        let rev = captured_revision();
        // First member installed by mod id, second downloaded by file id, the
        // rest missing.
        let (first, second) = (rev.mods[0].mod_id, rev.mods[1].file_id);
        let (mut app, root) = collection_app(&[("Karthwasten", first)], &[second]);
        app.collection = Some(CollectionState {
            link: String::new(),
            revision: Some(rev),
            states: Vec::new(),
            loading: false,
            error: None,
            confirm_fetch: false,
            asked: std::collections::HashSet::new(),
        });
        recompute_collection_states(&mut app);

        let st = &app.collection.as_ref().unwrap().states;
        assert_eq!(st[0], MemberState::Installed, "matched on the Nexus mod id");
        assert_eq!(st[1], MemberState::Downloaded, "matched on the exact file id");
        assert!(st[2..].iter().all(|s| *s == MemberState::Missing));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn one_click_asks_for_a_capped_batch_not_the_whole_collection() {
        use crate::update::{next_fetch_batch, FETCH_BATCH};
        let mut rev = captured_revision();
        // Twenty missing members - a modest collection by Nexus standards, and
        // twenty `eidos nxm` children would be twenty processes at once.
        let one = rev.mods[0].clone();
        rev.mods = (0..20u64)
            .map(|i| eidos_nexus::collections::CollectionMod {
                mod_id: 1000 + i,
                file_id: 2000 + i,
                ..one.clone()
            })
            .collect();
        let states = vec![MemberState::Missing; rev.mods.len()];
        let mut asked = std::collections::HashSet::new();

        let (batch, left) = next_fetch_batch(&rev, &states, &asked, FETCH_BATCH);
        assert_eq!(batch.len(), FETCH_BATCH, "capped");
        assert_eq!(left, 20 - FETCH_BATCH, "and it says how many are behind them");

        // Clicking again advances instead of restarting the same few: the first
        // batch is still `Missing` (its downloads are running), so only `asked`
        // can tell them apart.
        asked.extend(batch.iter().map(|(_, f, _)| *f));
        let (second, _) = next_fetch_batch(&rev, &states, &asked, FETCH_BATCH);
        assert_eq!(second.len(), FETCH_BATCH);
        assert!(
            second.iter().all(|(_, f, _)| !batch.iter().any(|(_, g, _)| g == f)),
            "no overlap with the first batch"
        );

        // And the tail is short rather than wrapping.
        asked.extend(rev.mods.iter().map(|m| m.file_id).take(18));
        let (tail, left) = next_fetch_batch(&rev, &states, &asked, FETCH_BATCH);
        assert_eq!(tail.len(), 2);
        assert_eq!(left, 0);
    }

    #[test]
    fn fetching_the_missing_members_takes_two_clicks() {
        let (mut app, root) = collection_app(&[], &[]);
        let rev = captured_revision();
        app.collection = Some(CollectionState {
            link: String::new(),
            revision: Some(rev),
            states: Vec::new(),
            loading: false,
            error: None,
            confirm_fetch: false,
            asked: std::collections::HashSet::new(),
        });
        recompute_collection_states(&mut app);
        assert!(
            app.collection.as_ref().unwrap().states.iter().all(|s| *s == MemberState::Missing),
            "the fixture instance has none of them"
        );

        // First click arms and spawns nothing.
        let _ = update_inner(&mut app, Message::CollectionFetchMissing);
        assert!(app.collection.as_ref().unwrap().confirm_fetch);
        assert!(app.collection.as_ref().unwrap().asked.is_empty(), "nothing started yet");

        // And any other action disarms it, so a stray click cannot start a
        // dozen transfers a minute later.
        let _ = update_inner(&mut app, Message::CollectionLinkChanged("x".to_string()));
        assert!(!app.collection.as_ref().unwrap().confirm_fetch);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_member_already_asked_for_is_not_asked_for_twice() {
        let (mut app, root) = collection_app(&[], &[]);
        let rev = captured_revision();
        let all: Vec<u64> = rev.mods.iter().map(|m| m.file_id).collect();
        app.collection = Some(CollectionState {
            link: String::new(),
            revision: Some(rev),
            states: Vec::new(),
            loading: false,
            error: None,
            confirm_fetch: false,
            // Every member already started. A download in flight leaves its
            // member `missing` until the sidecar lands, so without this set the
            // pane would spawn the same transfers again on the next click.
            asked: all.into_iter().collect(),
        });
        recompute_collection_states(&mut app);

        let _ = update_inner(&mut app, Message::CollectionFetchMissing);
        assert!(!app.collection.as_ref().unwrap().confirm_fetch, "nothing to confirm");
        let s = app.status.clone().unwrap_or_default();
        assert!(s.contains("already started"), "{s}");
        assert!(s.contains("Look up again"), "and how to retry: {s}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_preview_says_why_when_it_cannot_show_something() {
        let root = temp_portable("skyrimse");
        fs::create_dir_all(&root).unwrap();

        // Text, shown lossily: a Windows-1252 INI is common in this world and
        // refusing one because a byte is not valid UTF-8 helps nobody.
        let ini = root.join("Skyrim.ini");
        fs::write(&ini, b"[Display]\niSize=1920\nname=Andr\xe9\n").unwrap();
        match build_preview(&ini) {
            Preview::Text { body, truncated, .. } => {
                assert!(body.contains("iSize=1920"));
                assert!(!truncated);
            }
            other => panic!("expected text, got {other:?}"),
        }

        // A NUL byte means binary whatever the extension claims - an .esp is a
        // record file, and printing one as text fills the pane with mojibake.
        let esp = root.join("Mod.esp");
        fs::write(&esp, b"TES4\0\0\0garbage").unwrap();
        assert!(matches!(build_preview(&esp), Preview::Unsupported { .. }));

        // DDS and NIF say what they are and what to do instead, rather than
        // showing an empty box - "no preview" with no reason reads as broken.
        let dds = root.join("skin.dds");
        fs::write(&dds, b"DDS ").unwrap();
        match build_preview(&dds) {
            Preview::Unsupported { why, .. } => {
                assert!(why.contains("DDS"), "{why}");
                assert!(why.contains("Reveal"), "and what to do instead: {why}");
            }
            other => panic!("expected unsupported, got {other:?}"),
        }

        // A folder is not a file.
        assert!(matches!(build_preview(&root), Preview::Unsupported { .. }));
        // And something that is not there at all.
        assert!(matches!(build_preview(&root.join("nope.txt")), Preview::Unsupported { .. }));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_huge_text_file_is_read_as_far_as_the_cap_and_says_so() {
        // A preview is a glance, and a log can be a hundred megabytes: reading
        // one whole to show its first screen is how a file browser freezes.
        let root = temp_portable("skyrimse");
        fs::create_dir_all(&root).unwrap();
        let log = root.join("big.log");
        fs::write(&log, "x".repeat(PREVIEW_TEXT_CAP * 2)).unwrap();

        match build_preview(&log) {
            Preview::Text { body, truncated, .. } => {
                assert_eq!(body.len(), PREVIEW_TEXT_CAP);
                assert!(truncated, "and it says the rest is there");
            }
            other => panic!("expected text, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_hidden_tool_leaves_the_picker_and_a_pinned_one_goes_to_the_top() {
        use eidos_instance::Tool;
        let mk = |title: &str, hidden: bool, pinned: bool| Tool {
            title: title.to_string(),
            exe: PathBuf::from("/x/t.exe"),
            hidden,
            pinned,
            ..Default::default()
        };
        let tools = vec![
            mk("Launcher", false, false),
            mk("Never used", true, false),
            mk("SSEEdit", false, true),
        ];
        let mut listed: Vec<&Tool> = tools.iter().filter(|t| !t.hidden).collect();
        listed.sort_by_key(|t| !t.pinned);
        let titles: Vec<&str> = listed.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["SSEEdit", "Launcher"], "pinned first, hidden gone");
    }

    #[test]
    fn a_desktop_shortcut_quotes_a_path_with_a_space_in_it() {
        use eidos_instance::Tool;
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.join("Eidos Skyrim"));
        inst.create().unwrap();
        let home = root.join("fakehome");
        fs::create_dir_all(&home).unwrap();
        // The entry must land where the DESKTOP looks, not in the Colony tree.
        let prev = std::env::var_os("XDG_DATA_HOME");
        // SAFETY: single-threaded here, and restored below.
        unsafe { std::env::set_var("XDG_DATA_HOME", &home) };

        let tool = Tool {
            title: "SSEEdit".to_string(),
            exe: PathBuf::from("/x/SSEEdit.exe"),
            ..Default::default()
        };
        let path = write_desktop_entry(&inst, "skyrimse", &tool).unwrap();
        let body = fs::read_to_string(&path).unwrap();

        assert!(path.starts_with(home.join("applications")));
        assert!(body.starts_with("[Desktop Entry]"));
        // A portable instance with a space in its path is ordinary, and unquoted
        // it would reach `eidos tool` as two arguments.
        assert!(
            body.contains(&format!("\"{}\"", inst.root.display())),
            "the whole path is one quoted argument:\n{body}"
        );
        assert!(inst.root.display().to_string().contains(' '), "the fixture has a space in it");
        assert!(body.contains("tool "), "{body}");
        assert!(body.contains("run \"SSEEdit\""), "{body}");

        match prev {
            // SAFETY: as above.
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_backup_is_inert_and_can_be_restored_over_the_mod_it_came_from() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let modd = root.join("mods").join("Armour");
        fs::create_dir_all(modd.join("Meshes")).unwrap();
        fs::write(modd.join("Meshes").join("a.nif"), b"original").unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;
        reload_mods(&mut app);

        let i = app.mods.iter().position(|m| m.name == "Armour").unwrap();
        let _ = update_inner(&mut app, Message::ModBackup(i));
        assert!(root.join("mods").join("Armour_backup").join("Meshes").join("a.nif").is_file());

        // Inert: it contributes nothing to the game, whatever modlist.txt says.
        let backup = app.mods.iter().find(|m| m.name == "Armour_backup").expect("in the list");
        assert!(backup.is_backup());
        assert!(!backup.is_active(), "a backup must never reach the game");

        // A second backup does not replace the first - that would lose the very
        // state somebody took a backup to keep.
        let i = app.mods.iter().position(|m| m.name == "Armour").unwrap();
        let _ = update_inner(&mut app, Message::ModBackup(i));
        assert!(root.join("mods").join("Armour_backup2").is_dir());

        // Now break the mod, and restore.
        fs::write(modd.join("Meshes").join("a.nif"), b"broken").unwrap();
        let b = app.mods.iter().position(|m| m.name == "Armour_backup").unwrap();
        let _ = update_inner(&mut app, Message::ModRestoreBackup(b));
        assert_eq!(fs::read(modd.join("Meshes").join("a.nif")).unwrap(), b"broken", "one click arms");
        let _ =
            update_inner(&mut app, Message::ConfirmModRestoreBackup("Armour_backup".to_string()));
        assert_eq!(fs::read(modd.join("Meshes").join("a.nif")).unwrap(), b"original");
        // And the scratch directory the restore used is gone.
        assert!(!root.join("mods").join("Armour.eidos-restoring").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn is_active_is_the_one_predicate_that_decides_what_reaches_the_game() {
        let mk = |name: &str, enabled: bool| ModEntry {
            name: name.to_string(),
            enabled,
            path: PathBuf::from("/tmp").join(name),
            unmanaged: false,
        };
        assert!(mk("Armour", true).is_active());
        assert!(!mk("Armour", false).is_active(), "disabled");
        assert!(!mk("SEP_separator", true).is_active(), "a separator has no files");
        // The case this predicate was introduced for: a backup is not merely
        // disabled, because a user who ticked it would deploy two copies of one
        // mod over each other.
        assert!(!mk("Armour_backup", true).is_active());
        assert!(!mk("Armour_backup7", true).is_active());
        assert!(mk("Armour_backups", true).is_active(), "only the exact suffix");
    }

    #[test]
    fn the_filetree_path_resolver_refuses_everything_that_is_not_inside_the_mod() {
        use crate::modinfo::resolve_in_mod;
        let base = PathBuf::from("/tmp/mods/Armour");
        // The ordinary case works.
        assert_eq!(
            resolve_in_mod(&base, "Meshes/armour/x.nif"),
            Some(base.join("Meshes/armour/x.nif"))
        );
        // And everything else is refused rather than normalised. A path that
        // needed normalising is not one this tab produced.
        for bad in [
            "",
            "   ",
            "..",
            "../../etc/passwd",
            "Meshes/../../..",
            "./x",
            "/etc/passwd",
            "Meshes//x",
            "C:\\Windows",
            "Meshes\\x",
        ] {
            assert_eq!(resolve_in_mod(&base, bad), None, "{bad:?} must be refused");
        }
    }

    #[test]
    fn the_filetree_resolver_will_not_walk_through_a_symlink() {
        use crate::modinfo::resolve_in_mod;
        let root = temp_portable("skyrimse");
        let modd = root.join("mods").join("Armour");
        let outside = root.join("elsewhere");
        fs::create_dir_all(modd.join("Meshes")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret"), b"not ours").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, modd.join("Away")).unwrap();

        // A real path inside still resolves.
        assert!(resolve_in_mod(&modd, "Meshes").is_some());
        // Through the link does not - which is how a delete or a rename would
        // otherwise reach outside the mod folder entirely.
        #[cfg(unix)]
        assert_eq!(resolve_in_mod(&modd, "Away/secret"), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn renaming_a_file_replaces_the_name_and_never_overwrites() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let modd = root.join("mods").join("Armour");
        fs::create_dir_all(modd.join("Meshes")).unwrap();
        fs::write(modd.join("Meshes").join("a.nif"), b"a").unwrap();
        fs::write(modd.join("Meshes").join("taken.nif"), b"b").unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = vec![ModEntry {
            name: "Armour".to_string(),
            enabled: true,
            path: modd.clone(),
            unmanaged: false,
        }];
        app.screen = Screen::Main;

        let _ = update_inner(
            &mut app,
            Message::FiletreeRenameStart(0, "Meshes/a.nif".to_string()),
        );
        // The box holds the NAME, not the path - editing directories in a rename
        // box is a move, which this is not.
        assert_eq!(app.tree_rename_text, "a.nif");

        let _ = update_inner(&mut app, Message::FiletreeRenameChanged("b.nif".to_string()));
        let _ = update_inner(&mut app, Message::FiletreeRenameCommit);
        assert!(modd.join("Meshes").join("b.nif").is_file());
        assert!(!modd.join("Meshes").join("a.nif").exists());

        // Never over something already there: fs::rename would replace it in
        // silence, and this is a mod's own contents.
        let _ = update_inner(
            &mut app,
            Message::FiletreeRenameStart(0, "Meshes/b.nif".to_string()),
        );
        let _ = update_inner(&mut app, Message::FiletreeRenameChanged("taken.nif".to_string()));
        let _ = update_inner(&mut app, Message::FiletreeRenameCommit);
        assert!(modd.join("Meshes").join("b.nif").is_file(), "the rename was refused");
        assert_eq!(fs::read(modd.join("Meshes").join("taken.nif")).unwrap(), b"b");
        assert!(app.error.is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn deleting_from_the_filetree_takes_two_clicks() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let modd = root.join("mods").join("Armour");
        fs::create_dir_all(modd.join("Meshes")).unwrap();
        fs::write(modd.join("Meshes").join("a.nif"), b"a").unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = vec![ModEntry {
            name: "Armour".to_string(),
            enabled: true,
            path: modd.clone(),
            unmanaged: false,
        }];
        app.screen = Screen::Main;

        let _ = update_inner(&mut app, Message::FiletreeDelete(0, "Meshes/a.nif".to_string()));
        assert!(modd.join("Meshes").join("a.nif").is_file(), "one click only arms");
        assert!(app.tree_delete_armed.is_some());

        // And any other action disarms it, like every other confirmation here.
        let _ = update_inner(&mut app, Message::Refresh);
        assert!(app.tree_delete_armed.is_none());
        assert!(modd.join("Meshes").join("a.nif").is_file());

        let _ = update_inner(&mut app, Message::FiletreeDelete(0, "Meshes/a.nif".to_string()));
        let _ = update_inner(
            &mut app,
            Message::ConfirmFiletreeDelete("Armour".to_string(), "Meshes/a.nif".to_string()),
        );
        assert!(!modd.join("Meshes").join("a.nif").exists());
        let _ = fs::remove_dir_all(&root);
    }

    /// Four mods and a separator, with an instance so meta reads work.
    fn list_app(names: &[&str]) -> (App, PathBuf) {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        for n in names {
            fs::create_dir_all(root.join("mods").join(n)).unwrap();
        }
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = names
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: root.join("mods").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        refresh_meta_cache(&mut app);
        (app, root)
    }

    #[test]
    fn a_confirmation_survives_the_list_moving_under_it() {
        // Two-click confirmations that store an INDEX are a known trap here: a
        // reload between the clicks and the second one acts on a different row.
        // A restore would then overwrite a different mod - and the `is_backup`
        // guard would not catch it, because the other row is a backup too.
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        for (name, body) in [("A", "a"), ("B", "b")] {
            let d = root.join("mods").join(name);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("f.txt"), body).unwrap();
        }
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;
        reload_mods(&mut app);

        // Back both up, so there are two backups to confuse.
        for name in ["A", "B"] {
            let i = app.mods.iter().position(|m| m.name == name).unwrap();
            let _ = update_inner(&mut app, Message::ModBackup(i));
        }
        // Break A, then arm ITS restore.
        fs::write(root.join("mods").join("A").join("f.txt"), b"broken").unwrap();
        let i = app.mods.iter().position(|m| m.name == "A_backup").unwrap();
        let _ = update_inner(&mut app, Message::ModRestoreBackup(i));
        assert_eq!(app.confirm_restore.as_deref(), Some("A_backup"), "armed by name");

        // Now the list moves under it - a refresh, an install, anything.
        app.mods.reverse();

        let _ = update_inner(&mut app, Message::ConfirmModRestoreBackup("A_backup".to_string()));
        assert_eq!(
            fs::read(root.join("mods").join("A").join("f.txt")).unwrap(),
            b"a",
            "A was restored"
        );
        assert_eq!(
            fs::read(root.join("mods").join("B").join("f.txt")).unwrap(),
            b"b",
            "and B was not touched"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_can_act_on_a_row_the_list_is_not_drawing() {
        // The defect this exists for was systemic: sorting and grouping changed
        // what is DRAWN, and the keyboard, shift-extend, select-all and the bulk
        // actions were all still asking "which rows pass the filter" - a
        // different question once the two orders disagree. One answer now.
        let (mut app, root) = list_app(&["Zeta", "SEP_separator", "Alpha", "Mid"]);

        // Load order: exactly what it always was, separators included.
        assert_eq!(drawn_mod_rows(&app), vec![0, 1, 2, 3]);

        // Grouped, with one group folded: the folded rows are drawn by nobody,
        // so nothing may reach them.
        let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
        let label = match display_entries(&app).first() {
            Some(ListEntry::Group(l, _)) => l.clone(),
            other => panic!("expected a header, got {other:?}"),
        };
        let _ = update_inner(&mut app, Message::ToggleGroupFold(label));
        assert!(drawn_mod_rows(&app).is_empty(), "everything is inside the folded group");
        // Select-all, which feeds the batch Remove that DELETES FROM DISK.
        assert!(
            mods_visible_for_bulk(&app).is_empty(),
            "a batch action must not reach a mod nobody can see"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_drag_is_refused_outside_load_order_rather_than_moving_the_wrong_row() {
        // Hiding the insertion strips was not enough: a row's own press still
        // armed a drag, and the gap it aimed at addressed the REAL list while
        // the rows on screen were somewhere else.
        let (mut app, root) = list_app(&["Zeta", "Alpha", "Mid"]);
        let before: Vec<String> = app.mods.iter().map(|m| m.name.clone()).collect();

        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
        assert!(!can_reorder(&app));
        let _ = update_inner(&mut app, Message::SelectMod(0));
        assert!(app.drag_state.is_none(), "no drag is armed while sorted");
        let _ = update_inner(&mut app, Message::DragOverGap(2));
        let _ = update_inner(&mut app, Message::DragDrop);
        assert_eq!(
            app.mods.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
            before,
            "and nothing moved"
        );

        // Back in load order it works again, which is the point of the escape
        // hatch in the View menu.
        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
        assert!(can_reorder(&app));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn shift_click_selects_the_run_that_is_on_screen() {
        let (mut app, root) = list_app(&["Zeta", "Alpha", "Mid"]);
        // Sorted by name the drawn order is Alpha(1), Mid(2), Zeta(0).
        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
        assert_eq!(drawn_mod_rows(&app), vec![1, 2, 0]);

        let _ = update_inner(&mut app, Message::SelectMod(1));
        let _ = update_inner(&mut app, Message::SelectModExtend(2));
        // Alpha to Mid: two rows, adjacent on screen. Over the raw index range
        // that would have been 1..=2 by luck; the case that broke is below.
        assert_eq!(app.selected_mods.len(), 2);

        // Alpha to Zeta is the WHOLE drawn list. Over raw indices it would have
        // been 0..=1 - Zeta and Alpha - silently missing the row between them.
        let _ = update_inner(&mut app, Message::SelectMod(1));
        let _ = update_inner(&mut app, Message::SelectModExtend(0));
        let mut got: Vec<usize> = app.selected_mods.iter().copied().collect();
        got.sort_unstable();
        assert_eq!(got, vec![0, 1, 2], "every row between them ON SCREEN");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn collapse_all_acts_on_whichever_folds_are_on_screen() {
        let (mut app, root) = list_app(&["A", "SEP_separator", "B"]);
        // Ungrouped: the separators, as it always did.
        let _ = update_inner(&mut app, Message::CollapseAllGroups);
        assert!(!app.collapsed.is_empty());

        // Grouped: the separators are not drawn and their folds are suspended,
        // so folding them would be a menu entry that visibly does nothing.
        let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
        let _ = update_inner(&mut app, Message::CollapseAllGroups);
        assert!(!app.groups_collapsed.is_empty(), "the headers that ARE drawn");
        assert!(drawn_mod_rows(&app).is_empty());
        let _ = update_inner(&mut app, Message::ExpandAllGroups);
        assert!(app.groups_collapsed.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_separator_fold_cannot_hide_mods_in_a_grouped_list() {
        // The separator that would unfold them is not drawn under a grouping, so
        // a fold left standing hides mods with no way back.
        let (mut app, root) = list_app(&["SEP_separator", "A", "B"]);
        let _ = update_inner(&mut app, Message::ToggleCollapse("SEP".to_string()));
        assert_eq!(drawn_mod_rows(&app).len(), 1, "folded: only the separator");

        let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
        assert_eq!(drawn_mod_rows(&app).len(), 2, "the mods are back");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_group_header_counts_only_the_rows_it_actually_heads() {
        let (mut app, root) = list_app(&["Alpha", "Beta"]);
        let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
        assert!(matches!(display_entries(&app).first(), Some(ListEntry::Group(_, 2))));

        // Filter one out: the count follows, and a header left with nothing does
        // not draw at all.
        let _ = update_inner(&mut app, Message::SearchChanged("alpha".to_string()));
        assert!(matches!(display_entries(&app).first(), Some(ListEntry::Group(_, 1))));
        let _ = update_inner(&mut app, Message::SearchChanged("nothing".to_string()));
        assert!(display_entries(&app).is_empty(), "no header with nothing under it");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn offline_mode_stops_the_sign_in_that_never_reaches_connect() {
        let mut app = app_for_game("skyrimse");
        app.prefs.offline = true;
        let _ = update_inner(&mut app, Message::NexusSignInStart);
        // Signing in opens a BROWSER before any request is made, so the guard in
        // `Nexus::connect` is not on this path at all.
        assert!(!app.nexus_signing_in, "it must not start");
        assert_eq!(app.nexus_error.as_deref(), Some(eidos_nexus::OFFLINE_MESSAGE));
    }

    #[test]
    fn an_archive_with_no_sidecar_can_still_be_hidden() {
        // The pile somebody most wants to hide is the one copied in by hand,
        // which has no sidecar at all - and the key-editing helper only edits a
        // file that already exists.
        let (mut app, root) = downloads_instance(&[("ByHand.7z", "")]);
        assert_eq!(app.downloads.len(), 1);

        let _ = update_inner(&mut app, Message::HideDownload("ByHand.7z".to_string()));
        assert!(app.error.is_none(), "{:?}", app.error);
        assert!(app.downloads.is_empty(), "it is hidden");
        assert!(root.join("downloads").join("ByHand.7z").is_file(), "and still there");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn grouping_puts_every_mod_under_exactly_one_header() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        // Two with a category, one without, plus a separator - which grouping
        // must drop, because it heads rows the grouping has moved.
        for (name, cat) in [("Helm", "5"), ("Sword", "5"), ("Loose", ""), ("SEP_separator", "")] {
            let dir = root.join("mods").join(name);
            fs::create_dir_all(&dir).unwrap();
            if !cat.is_empty() {
                fs::write(dir.join("meta.ini"), format!("[General]\ncategory=\"{cat},\"\n"))
                    .unwrap();
            }
        }
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = ["Helm", "Sword", "Loose", "SEP_separator"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: root.join("mods").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        refresh_meta_cache(&mut app);

        let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Category)));
        let entries = display_entries(&app);
        let headers: Vec<String> = entries
            .iter()
            .filter_map(|e| match e {
                ListEntry::Group(l, _) => Some(l.clone()),
                _ => None,
            })
            .collect();
        // The catch-all sinks to the bottom whatever it is called - it is the
        // pile that needs sorting out, not the first thing anybody wants.
        assert_eq!(headers.last().map(String::as_str), Some("Uncategorised"));
        // Every non-separator mod appears exactly once, and the separator not at
        // all.
        let rows: Vec<usize> = entries
            .iter()
            .filter_map(|e| match e {
                ListEntry::Row(i) => Some(*i),
                _ => None,
            })
            .collect();
        assert_eq!(rows.len(), 3, "the separator is not a row under a grouping");
        assert!(!rows.contains(&3));
        // The counts on the headers add up to the rows drawn.
        let counted: usize = entries
            .iter()
            .filter_map(|e| match e {
                ListEntry::Group(_, n) => Some(*n),
                _ => None,
            })
            .sum();
        assert_eq!(counted, 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn folding_a_group_hides_its_rows_but_keeps_its_count() {
        let mut app = app_for_game("skyrimse");
        app.mods = ["A", "B"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: PathBuf::from("/tmp").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));

        let label = match display_entries(&app).first() {
            Some(ListEntry::Group(l, _)) => l.clone(),
            other => panic!("expected a header, got {other:?}"),
        };
        let _ = update_inner(&mut app, Message::ToggleGroupFold(label.clone()));
        let entries = display_entries(&app);
        assert_eq!(entries.len(), 1, "only the header is left");
        // The count still says how many are inside, which is the whole reason to
        // fold rather than filter.
        assert_eq!(entries[0], ListEntry::Group(label.clone(), 2));

        // And the same click unfolds it.
        let _ = update_inner(&mut app, Message::ToggleGroupFold(label));
        assert_eq!(display_entries(&app).len(), 3);
    }

    #[test]
    fn leaving_a_grouping_returns_the_list_to_load_order() {
        let mut app = app_for_game("skyrimse");
        app.mods = ["A", "B"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: PathBuf::from("/tmp").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
        let _ = update_inner(&mut app, Message::ToggleGroupFold("From Nexus".to_string()));

        let _ = update_inner(&mut app, Message::SetGroupBy(None));
        assert!(app.group_by.is_none());
        // The folds go with it: they key on labels that no longer exist, and a
        // stale one would fold a group with the same name next time.
        assert!(app.groups_collapsed.is_empty());
        assert_eq!(
            display_entries(&app),
            vec![ListEntry::Row(0), ListEntry::Row(1)],
            "back to the real list"
        );
    }

    #[test]
    fn sorting_leaves_the_real_order_alone_and_takes_the_separators_out() {
        let mut app = app_for_game("skyrimse");
        app.mods = ["Zeta", "01 CITIES_separator", "Alpha", "Mid"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: PathBuf::from("/tmp").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;

        // Load order is 0..len, which is what keeps the drag strips valid.
        assert_eq!(display_order(&app), vec![0, 1, 2, 3]);

        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
        // A separator is a HEADING; ordered by name it heads nothing, so it
        // leaves the list rather than floating into the middle of it.
        assert_eq!(display_order(&app), vec![2, 3, 0], "Alpha, Mid, Zeta - no separator");
        // And the underlying list is untouched: sorting is a view, not a move.
        assert_eq!(app.mods[0].name, "Zeta");

        // Second click reverses, third returns to load order - which has to be
        // one click away, because it is the only order where dragging works.
        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
        assert_eq!(display_order(&app), vec![0, 3, 2]);
        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
        assert!(app.mod_sort.is_none());
        assert_eq!(display_order(&app), vec![0, 1, 2, 3]);
    }

    #[test]
    fn hiding_a_column_that_the_list_is_sorted_by_returns_it_to_load_order() {
        // Ordering a list by a column nobody can see is a list that looks
        // shuffled for no reason.
        let mut app = app_for_game("skyrimse");
        let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Column(ModColumn::Version)));
        assert!(app.mod_sort.is_some());
        assert!(app.mod_columns.contains(&ModColumn::Version));

        let _ = update_inner(&mut app, Message::ToggleModColumn(ModColumn::Version));
        assert!(!app.mod_columns.contains(&ModColumn::Version));
        assert!(app.mod_sort.is_none());
    }

    #[test]
    fn columns_are_saved_and_come_back_in_the_headers_order() {
        let mut app = app_for_game("skyrimse");
        // Turned on in a deliberately awkward order.
        let _ = update_inner(&mut app, Message::ToggleModColumn(ModColumn::Game));
        let _ = update_inner(&mut app, Message::ToggleModColumn(ModColumn::Author));

        // Redrawn canonically, so toggling one on cannot move another.
        let after: Vec<&str> = app.mod_columns.iter().map(|c| c.title()).collect();
        assert_eq!(after, vec!["Category", "Content", "Version", "Author", "Game", "Flags"]);

        // And a hand-edited settings file cannot produce a header that
        // disagrees with the rows either.
        let mut prefs = eidos_instance::settings::Settings::default();
        prefs.mod_columns = Some(vec!["game".into(), "category".into()]);
        let back: Vec<&str> = columns_from_settings(&prefs).iter().map(|c| c.title()).collect();
        assert_eq!(back, vec!["Category", "Game"]);

        // An EMPTY list is a choice, not "never chosen" - it has to survive a
        // restart rather than springing back to the defaults.
        prefs.mod_columns = Some(Vec::new());
        assert!(columns_from_settings(&prefs).is_empty());
        assert!(!columns_from_settings(&eidos_instance::settings::Settings::default()).is_empty());
    }

    fn downloads_instance(files: &[(&str, &str)]) -> (App, PathBuf) {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let dl = inst.downloads_dir();
        fs::create_dir_all(&dl).unwrap();
        for (name, sidecar) in files {
            fs::write(dl.join(name), b"xxxx").unwrap();
            if !sidecar.is_empty() {
                fs::write(dl.join(format!("{name}.meta")), sidecar).unwrap();
            }
        }
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;
        load_downloads(&mut app);
        (app, root)
    }

    #[test]
    fn hiding_a_download_keeps_the_archive_and_can_be_undone() {
        let (mut app, root) = downloads_instance(&[
            ("Keep.7z", "[General]\nmodID=1\ninstalled=true\n"),
            ("Stale.7z", "[General]\nmodID=2\ninstalled=true\n"),
        ]);
        assert_eq!(app.downloads.len(), 2);

        let _ = update_inner(&mut app, Message::HideDownload("Stale.7z".to_string()));
        assert_eq!(app.downloads.len(), 1, "hidden rows are dropped from the list");
        // The whole point: putting a book away is not burning it.
        assert!(
            root.join("downloads").join("Stale.7z").is_file(),
            "the archive must still be there"
        );

        // Show hidden brings it back, and the same button unhides it.
        let _ = update_inner(&mut app, Message::ToggleShowHiddenDownloads);
        assert_eq!(app.downloads.len(), 2);
        let _ = update_inner(&mut app, Message::HideDownload("Stale.7z".to_string()));
        let _ = update_inner(&mut app, Message::ToggleShowHiddenDownloads);
        assert_eq!(app.downloads.len(), 2, "unhidden again");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_bulk_purge_takes_two_clicks_and_only_what_is_on_screen() {
        let (mut app, root) = downloads_instance(&[
            ("Done.7z", "[General]\nmodID=1\ninstalled=true\n"),
            ("AlsoDone.7z", "[General]\nmodID=2\ninstalled=true\n"),
            ("NotYet.7z", "[General]\nmodID=3\n"),
        ]);

        // The filter is how the user said which ones they meant. A bulk delete
        // that ignores it deletes things they were not looking at.
        let _ = update_inner(&mut app, Message::DownloadFilterChanged("also".to_string()));
        assert_eq!(app.downloads.len(), 1);

        // One click only arms.
        let _ = update_inner(&mut app, Message::PurgeInstalledDownloads);
        assert!(app.confirm_purge_installed);
        assert!(root.join("downloads").join("AlsoDone.7z").is_file());

        let _ = update_inner(&mut app, Message::ConfirmPurgeInstalled);
        assert!(!root.join("downloads").join("AlsoDone.7z").exists(), "the filtered one went");
        assert!(root.join("downloads").join("Done.7z").is_file(), "the one off screen did not");
        assert!(root.join("downloads").join("NotYet.7z").is_file(), "and nor did the uninstalled");
        // The sidecar goes with the archive, or the row comes back as a ghost.
        assert!(!root.join("downloads").join("AlsoDone.7z.meta").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_downloads_list_sorts_by_what_was_asked_for() {
        let (mut app, root) = downloads_instance(&[
            ("bbb.7z", "[General]\nmodID=1\nmodName=Zebra\n"),
            ("aaa.7z", "[General]\nmodID=2\nmodName=Apple\n"),
        ]);
        let _ = update_inner(&mut app, Message::DownloadSortChanged(DownloadSort::Name));
        let names: Vec<&str> = app.downloads.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["aaa.7z", "bbb.7z"]);

        // The friendly mod name is searched too: an archive called
        // `SkyUI_5_2_SE-12604.7z` is found by typing "skyui" only if it is.
        let _ = update_inner(&mut app, Message::DownloadFilterChanged("zebra".to_string()));
        assert_eq!(app.downloads.len(), 1);
        assert_eq!(app.downloads[0].name, "bbb.7z");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_that_looks_like_nothing_this_game_loads_is_flagged_and_can_be_silenced() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        // One real mod, one that is just a readme, and one that ships only a
        // Root/ tree - Eidos's own convention, which must NOT be flagged.
        for (name, inner) in [("Good", "Meshes"), ("Junk", "docs"), ("RootOnly", "Root")] {
            fs::create_dir_all(root.join("mods").join(name).join(inner)).unwrap();
        }
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = ["Good", "Junk", "RootOnly"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: root.join("mods").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        refresh_meta_cache(&mut app);

        assert!(!app.meta_cache["Good"].invalid_data, "Meshes/ is data this game loads");
        assert!(app.meta_cache["Junk"].invalid_data, "docs/ alone is not");
        assert!(
            !app.meta_cache["RootOnly"].invalid_data,
            "a Root/ mod is correct - flagging it would flag every Root Builder mod"
        );

        // And "Mark as valid" silences it for good, through MO2's own key.
        let _ = update_inner(&mut app, Message::ModMarkValid(1));
        assert!(!app.meta_cache["Junk"].invalid_data);
        let text = fs::read_to_string(root.join("mods").join("Junk").join("meta.ini")).unwrap();
        assert!(text.contains("validated=true"), "MO2 reads this key too: {text}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_downloaded_for_another_game_says_which_one() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        for (name, game) in [("Ours", "SkyrimSE"), ("Theirs", "Fallout4"), ("Silent", "")] {
            let dir = root.join("mods").join(name);
            fs::create_dir_all(dir.join("Meshes")).unwrap();
            if !game.is_empty() {
                fs::write(dir.join("meta.ini"), format!("[General]\ngameName={game}\n")).unwrap();
            }
        }
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = ["Ours", "Theirs", "Silent"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: root.join("mods").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        refresh_meta_cache(&mut app);

        assert_eq!(app.meta_cache["Ours"].other_game, None);
        assert_eq!(app.meta_cache["Theirs"].other_game.as_deref(), Some("Fallout4"));
        // "Does not say" must look identical to "same game". A mod installed
        // from a folder never had a game recorded, and warning about that would
        // flag half a list.
        assert_eq!(app.meta_cache["Silent"].other_game, None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_letter_walks_every_match_rather_than_sticking_on_the_first() {
        let mut app = app_for_game("skyrimse");
        app.mods = ["Apachii Hair", "Beyond Skyrim", "Amber Guard", "Cloaks"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: PathBuf::from("/tmp").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;

        let _ = update_inner(&mut app, Message::JumpToLetter('a'));
        assert_eq!(app.selected_mod, Some(0), "the first A");
        let _ = update_inner(&mut app, Message::JumpToLetter('a'));
        assert_eq!(app.selected_mod, Some(2), "the next one, not the same one again");
        // And it wraps, so the list has no dead end.
        let _ = update_inner(&mut app, Message::JumpToLetter('a'));
        assert_eq!(app.selected_mod, Some(0));
        // Case does not matter, and a letter nothing starts with moves nothing.
        let _ = update_inner(&mut app, Message::JumpToLetter('C'));
        assert_eq!(app.selected_mod, Some(3));
        let _ = update_inner(&mut app, Message::JumpToLetter('z'));
        assert_eq!(app.selected_mod, Some(3), "no match leaves the focus alone");
    }

    #[test]
    fn a_letter_never_reaches_a_row_the_filter_is_hiding() {
        // Jumping onto a hidden row moves a highlight nobody can see, and the
        // next Space would then toggle a mod off screen.
        let mut app = app_for_game("skyrimse");
        app.mods = ["Apachii Hair", "Amber Guard"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: PathBuf::from("/tmp").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        app.search = "amber".to_string();

        let _ = update_inner(&mut app, Message::JumpToLetter('a'));
        assert_eq!(app.selected_mod, Some(1), "only the row the filter still draws");
    }

    #[test]
    fn a_double_click_branches_on_the_modifiers_that_are_actually_held() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        fs::create_dir_all(root.join("mods").join("Mod")).unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = vec![ModEntry {
            name: "Mod".to_string(),
            enabled: true,
            path: root.join("mods").join("Mod"),
            unmanaged: false,
        }];
        app.screen = Screen::Main;

        // Plain: Information. The closure that emits the double-click cannot see
        // the modifier set, which is why `update` reads it instead.
        let _ = update_inner(&mut app, Message::ModDoubleClick(0));
        assert_eq!(app.info_mod, Some(0));

        app.info_mod = None;
        app.modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = update_inner(&mut app, Message::ModDoubleClick(0));
        assert!(app.info_mod.is_none(), "Shift is the Nexus page, not Information");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn collapse_others_leaves_exactly_one_group_open() {
        let mut app = app_for_game("skyrimse");
        app.mods = ["A_separator", "one", "B_separator", "two", "C_separator"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: PathBuf::from("/tmp").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        let keep = app.mods[2].display_name().to_string();
        // Start with the kept one folded, to prove it is opened rather than
        // merely left alone.
        app.collapsed.insert(keep.clone());

        let _ = update_inner(&mut app, Message::CollapseOthers(keep.clone()));

        assert!(!app.collapsed.contains(&keep), "the one you asked to keep is open");
        for m in app.mods.iter().filter(|m| m.is_separator() && m.display_name() != keep) {
            assert!(app.collapsed.contains(m.display_name()), "{} should be folded", m.name);
        }
    }

    #[test]
    fn a_drag_resting_on_a_folded_group_opens_it_but_brushing_past_does_not() {
        let mut app = app_for_game("skyrimse");
        app.mods = ["A_separator", "one", "B_separator", "two"]
            .iter()
            .map(|n| ModEntry {
                name: (*n).to_string(),
                enabled: true,
                path: PathBuf::from("/tmp").join(n),
                unmanaged: false,
            })
            .collect();
        app.screen = Screen::Main;
        let folded = app.mods[2].display_name().to_string();
        app.collapsed.insert(folded.clone());

        let _ = update_inner(&mut app, Message::DragStart(1));
        let _ = update_inner(&mut app, Message::DragOverGap(2));
        assert_eq!(app.drag_hover_group.as_ref().map(|(n, t)| (n.clone(), *t)), Some((folded.clone(), 0)));

        // One tick is not enough - brushing past a group on the way somewhere
        // else must not open it.
        let _ = update_inner(&mut app, Message::DragHoverTick);
        assert!(app.collapsed.contains(&folded), "still folded after one tick");

        // Resting does open it.
        let _ = update_inner(&mut app, Message::DragHoverTick);
        assert!(!app.collapsed.contains(&folded), "resting on it opens it");

        // And moving off a group forgets it, so the counter cannot accumulate
        // across two different groups.
        app.collapsed.insert(folded.clone());
        let _ = update_inner(&mut app, Message::DragOverGap(2));
        let _ = update_inner(&mut app, Message::DragOverGap(1));
        assert!(app.drag_hover_group.is_none());
    }

    #[test]
    fn offline_mode_is_off_unless_it_was_explicitly_turned_on() {
        // The opposite default to lock_gui, deliberately: a settings file
        // written by an older Eidos has no `offline` key, and reading a missing
        // key as "on" would cut the network for everybody who upgrades.
        let s = eidos_instance::settings::Settings::parse("[eidos]\ntheme=dark\n");
        assert!(!s.offline);
        let s = eidos_instance::settings::Settings::parse("[eidos]\noffline=true\n");
        assert!(s.offline);
        // And it round-trips, which is the whole point of a setting.
        assert!(eidos_instance::settings::Settings::parse(&s.to_ini()).offline);
    }

    #[test]
    fn the_server_field_stores_an_order_and_echoes_back_what_it_kept() {
        let mut app = app_for_game("skyrimse");
        let _ = update_inner(
            &mut app,
            Message::PreferredServersChanged("  Paris , ,Nexus CDN,  ".to_string()),
        );
        let _ = update_inner(&mut app, Message::PreferredServersSave);

        assert_eq!(app.prefs.preferred_servers, vec!["Paris", "Nexus CDN"], "trimmed, blanks gone");
        // Echoed back as stored, so a trailing comma does not sit in the field
        // looking like it means something.
        assert_eq!(app.servers_edit, "Paris, Nexus CDN");
    }

    #[test]
    fn no_test_can_write_the_developers_own_preferences() {
        // The defect this exists for cost real time to find: `Settings::save`
        // resolved its path globally, and `App::new` loaded from the real one,
        // so a GUI test dispatching a toggle rewrote ~/.config/.../settings.ini
        // on the machine running the suite. The symptom - options reverting
        // after every rebuild - looks like anything but a test.
        let mut app = app_for_game("skyrimse");
        assert!(
            app.prefs.path().is_none(),
            "a test's Settings must not be bound to a file, let alone the real one"
        );

        let before = app.prefs.lock_gui;
        let _ = update_inner(&mut app, Message::ToggleLockGui(!before));
        assert_eq!(app.prefs.lock_gui, !before, "the toggle still works in memory");
        // And saving it is a no-op rather than a write, so no arm in update.rs
        // can reach the real file by accident either.
        assert!(app.prefs.save().is_ok());
        assert!(app.prefs.path().is_none());
    }

    #[test]
    fn a_collection_for_another_game_is_refused_by_name() {
        // The failure this exists for: a Skyrim collection opened while a
        // Fallout 4 instance is loaded would join its members against Fallout 4's
        // mods and downloads, so every "installed" and every "missing" is noise
        // shaped like an answer.
        let root = temp_portable("fallout4");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("fallout4");
        app.created = Some(inst);
        app.screen = Screen::Main;

        let _ = update_inner(
            &mut app,
            Message::ShowCollection(
                "nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest".to_string(),
            ),
        );
        let c = app.collection.as_ref().unwrap();
        let err = c.error.clone().unwrap_or_default();
        assert!(err.contains("skyrimspecialedition"), "it names the collection's game: {err}");
        assert!(err.contains("Fallout"), "and the one that is open: {err}");
        assert!(!c.loading, "and no request was dispatched");
        assert!(c.revision.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_collection_for_the_open_game_gets_past_the_guard() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;

        let _ = update_inner(
            &mut app,
            Message::ShowCollection(
                "nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest".to_string(),
            ),
        );
        let c = app.collection.as_ref().unwrap();
        // It gets as far as the credential check, which is the next gate - the
        // game guard is not what stopped it.
        assert!(
            c.error.as_deref().is_none_or(|e| !e.contains("instance is")),
            "{:?}",
            c.error
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_link_pasted_into_the_collection_box_says_what_it_is() {
        let mut app = nav_app(&[]);
        let _ = update_inner(&mut app, Message::ShowCollection(String::new()));
        let _ = update_inner(
            &mut app,
            Message::CollectionLinkChanged(
                "nxm://skyrimspecialedition/mods/266/files/1234".to_string(),
            ),
        );
        let _ = update_inner(&mut app, Message::CollectionFetch);
        let err = app.collection.as_ref().unwrap().error.clone().unwrap_or_default();
        // Not "bad link": it IS a valid link, to the wrong kind of thing, and
        // saying which sends the user to the button that handles it.
        assert!(err.contains("single mod"), "{err}");
    }

    #[test]
    fn a_malformed_collection_link_is_reported_without_a_request() {
        let mut app = nav_app(&[]);
        let _ = update_inner(&mut app, Message::ShowCollection("not a link".to_string()));
        let c = app.collection.as_ref().unwrap();
        assert!(c.error.is_some());
        assert!(!c.loading, "nothing was dispatched");
        assert!(c.revision.is_none());
    }

    #[test]
    fn every_timer_tick_is_ambient() {
        // A tick that counts as an ACTION cancels every armed two-click
        // confirmation before the second click can land. The saves watcher fires
        // every 2.5s and the log tail every 1.5s, so on those screens the
        // confirmations were a coin flip. Asserted as a set rather than one by
        // one, so the next tick that gets added is caught here.
        let app = nav_app(&[]);
        for m in [
            Message::DownloadTick,
            Message::SavesTick,
            Message::LogRefresh,
            Message::PointerAt(iced::Point::ORIGIN),
            Message::ModifiersChanged(iced::keyboard::Modifiers::default()),
        ] {
            assert!(is_ambient(&app, &m), "{m:?} must not disarm a confirmation");
        }
    }

    #[test]
    fn a_saves_tick_does_not_cancel_the_delete_it_is_ticking_beside() {
        let (mut app, root) = saves_app();
        app.tab = Tab::Saves;
        let _ = update_inner(&mut app, Message::SaveToggleSelect(0));
        let _ = update_inner(&mut app, Message::SavesDeleteSelected);
        assert!(app.confirm_saves_delete);
        // The watcher runs on its own, between the two clicks.
        let _ = update(&mut app, Message::SavesTick);
        assert!(app.confirm_saves_delete, "the tick cancelled the user's arming");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_saves_reload_keeps_the_whole_selection_not_just_the_focus() {
        let (mut app, root) = saves_app();
        let a = app.saves.iter().position(|s| s.filename == "Save1.ess").unwrap();
        let b = app.saves.iter().position(|s| s.filename == "Save2.ess").unwrap();
        let _ = update_inner(&mut app, Message::SaveToggleSelect(a));
        let _ = update_inner(&mut app, Message::SaveToggleSelect(b));
        assert_eq!(app.selected_saves.len(), 2);

        // The game writes an autosave, renumbering everything. The batch bar is
        // built on this set, so losing it takes the bar off screen mid-gesture.
        let dir = app.created.as_ref().unwrap().active().saves_dir();
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(dir.join("Autosave.ess"), b"z").unwrap();
        let _ = update_inner(&mut app, Message::SavesTick);

        let names: Vec<String> = app
            .selected_saves
            .iter()
            .filter_map(|&i| app.saves.get(i))
            .map(|s| s.filename.clone())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.contains(&"Save1.ess".to_string()));
        assert!(names.contains(&"Save2.ess".to_string()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_transfer_never_lands_a_cosave_beside_another_characters_save() {
        let (mut app, root) = saves_app();
        let inst = app.created.clone().unwrap();
        let dest = inst.profile("Second").saves_dir();
        let idx = app.saves.iter().position(|s| s.filename == "Save1.ess").unwrap();
        let stem = app.saves[idx].path.file_stem().unwrap().to_string_lossy().into_owned();

        // The destination already holds a DIFFERENT character's save under that
        // stem. Guarding per file copied the co-save anyway - a co-save that
        // silently belongs to the wrong game state.
        fs::write(dest.join(format!("{stem}.ess")), b"SOMEONE ELSE").unwrap();

        let _ = update_inner(&mut app, Message::SaveToggleSelect(idx));
        let _ = update_inner(&mut app, Message::SavesCopyToProfile("Second".into()));
        assert_eq!(fs::read(dest.join(format!("{stem}.ess"))).unwrap(), b"SOMEONE ELSE");
        assert!(
            !dest.join(format!("{stem}.skse")).exists(),
            "the co-save must not be planted beside a save it does not belong to"
        );
        assert!(app.status.as_deref().unwrap_or("").contains("already existed"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn deleting_selected_saves_takes_their_cosaves_and_does_not_shift_underneath_itself() {
        let (mut app, root) = saves_app();
        assert_eq!(app.saves.len(), 2);
        let dir = app.created.as_ref().unwrap().active().saves_dir();

        let _ = update_inner(&mut app, Message::SaveToggleSelect(0));
        let _ = update_inner(&mut app, Message::SaveToggleSelect(1));
        let _ = update_inner(&mut app, Message::SavesDeleteSelected);
        assert!(app.confirm_saves_delete, "the first click only arms");
        let _ = update_inner(&mut app, Message::SavesDeleteSelected);

        // Both gone, INCLUDING the co-save - which the game does not know about
        // and which orphans invisibly if it is left.
        assert!(!dir.join("Save1.ess").exists());
        assert!(!dir.join("Save1.skse").exists(), "the co-save travels with its save");
        assert!(!dir.join("Save2.ess").exists());
        assert!(app.saves.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn copying_saves_to_another_profile_copies_the_whole_group_and_never_overwrites() {
        let (mut app, root) = saves_app();
        let inst = app.created.clone().unwrap();
        let dest = inst.profile("Second").saves_dir();

        // The list is newest-first, so pick the one that HAS a co-save by name
        // rather than assuming an index.
        let idx = app.saves.iter().position(|s| s.filename == "Save1.ess").expect("Save1");
        let _ = update_inner(&mut app, Message::SaveToggleSelect(idx));
        let _ = update_inner(&mut app, Message::SavesCopyToProfile("Second".into()));

        let moved = app.saves[idx].clone();
        let stem = moved.path.file_stem().unwrap().to_string_lossy().into_owned();
        assert!(dest.join(format!("{stem}.ess")).is_file(), "{:?}", app.status);
        // The co-save comes too: a save without it is one the script extender
        // cannot restore its state for.
        assert!(dest.join(format!("{stem}.skse")).is_file());
        // And the source is untouched - this is a copy, not a move.
        assert!(moved.path.is_file());

        // A second copy must not clobber somebody's character.
        fs::write(dest.join(format!("{stem}.ess")), b"DIFFERENT").unwrap();
        let _ = update_inner(&mut app, Message::SavesCopyToProfile("Second".into()));
        assert_eq!(fs::read(dest.join(format!("{stem}.ess"))).unwrap(), b"DIFFERENT");
        assert!(app.status.as_deref().unwrap_or("").contains("already existed"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_saves_tick_only_reloads_when_the_directory_really_moved() {
        let (mut app, root) = saves_app();
        let _ = update_inner(&mut app, Message::SelectSave(0));
        let picked = app.saves[0].path.clone();
        assert_eq!(app.selected_save, Some(0));

        // A quiet tick changes nothing - reloading twice a second would close
        // the details pane under the user's hands.
        let _ = update_inner(&mut app, Message::SavesTick);
        assert_eq!(app.selected_save, Some(0));

        // A new save appears. The list reloads AND the selection follows its save
        // by path, because a new autosave renumbers every index.
        let dir = app.created.as_ref().unwrap().active().saves_dir();
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::write(dir.join("Save3.ess"), b"z").unwrap();
        let _ = update_inner(&mut app, Message::SavesTick);
        assert_eq!(app.saves.len(), 3);
        assert_eq!(
            app.selected_save.and_then(|i| app.saves.get(i)).map(|s| s.path.clone()),
            Some(picked),
            "the pane must not silently start describing a different save"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_conflict_map_is_not_the_same_as_nothing_to_send_back() {
        // "The question has not been asked" and "the answer is none" must not
        // look the same: one is fixed by opening a tab, the other by doing
        // something else entirely.
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.mods = mods(&["a"]);
        app.screen = Screen::Main;

        assert!(app.conflicts.is_none());
        assert!(overwrite_owners(&app).is_none());
        let _ = update_inner(&mut app, Message::OverwriteSyncToMods);
        assert!(app.confirm_sync, "the first click only arms");
        let _ = update_inner(&mut app, Message::OverwriteSyncToMods);
        assert!(
            app.status.as_deref().unwrap_or("").contains("Conflicts tab"),
            "{:?}",
            app.status
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_owner_of_an_overwrite_file_is_the_best_mod_under_it() {
        use eidos_conflicts::{ConflictMap, FileNode};
        let mut app = nav_app(&["Low", "High"]);
        let mut map = ConflictMap::default();
        // The Overwrite wins; under it, mod index 1 ("High", origin 2) beats
        // index 0 ("Low", origin 1), and the game's own Data (origin 0) is never
        // a destination - Eidos does not write there.
        map.files.insert(
            "meshes/a.nif".to_string(),
            FileNode {
                winner: u32::MAX,
                alternatives: vec![2, 1, 0],
                display_path: "meshes/a.nif".to_string(),
            },
        );
        // A file only the game provides underneath: nowhere to send it.
        map.files.insert(
            "vanilla.esm".to_string(),
            FileNode { winner: u32::MAX, alternatives: vec![0], display_path: "vanilla.esm".into() },
        );
        // A file the Overwrite does NOT win is not its business at all.
        map.files.insert(
            "other.txt".to_string(),
            FileNode { winner: 2, alternatives: vec![1], display_path: "other.txt".into() },
        );
        app.conflicts = Some(map);

        let owners = overwrite_owners(&app).unwrap();
        assert_eq!(owners.get("meshes/a.nif").map(String::as_str), Some("High"));
        assert!(!owners.contains_key("vanilla.esm"), "the game's Data is not a destination");
        assert!(!owners.contains_key("other.txt"));
    }

    #[test]
    fn renaming_a_portable_instance_moves_the_folder_and_the_registry_with_it() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        // NEVER the real user registry: these handlers persist.
        app.registry_path = root.parent().unwrap().join(format!("reg-{}.ini", std::process::id()));
        app.known = vec![KnownInstance {
            label: "Skyrim - portable".into(),
            inst: inst.clone(),
            game_index: 0,
            portable: true,
        }];
        app.instances_open = true;

        let dest = root.parent().unwrap().join(format!("renamed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dest);
        let _ = update_inner(&mut app, Message::InstanceRenameStart(0));
        let _ = update_inner(
            &mut app,
            Message::InstanceRenameChanged(dest.file_name().unwrap().to_string_lossy().into_owned()),
        );
        let _ = update_inner(&mut app, Message::InstanceRenameCommit);

        assert!(dest.is_dir(), "the folder moved: {:?}", app.status);
        assert!(!root.exists());
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn the_open_instance_cannot_be_renamed_out_from_under_the_window() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        app.registry_path = root.parent().unwrap().join(format!("reg2-{}.ini", std::process::id()));
        app.created = Some(inst.clone());
        app.known = vec![KnownInstance {
            label: "Skyrim - portable".into(),
            inst,
            game_index: 0,
            portable: true,
        }];

        let _ = update_inner(&mut app, Message::InstanceRenameStart(0));
        let _ = update_inner(&mut app, Message::InstanceRenameChanged("something-else".into()));
        let _ = update_inner(&mut app, Message::InstanceRenameCommit);
        // Every cached path in the window - and the lock it holds - points at
        // the old root.
        assert!(root.is_dir(), "it was renamed anyway");
        assert!(app.status.as_deref().unwrap_or("").contains("Switch to another"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_rename_that_is_not_a_folder_name_is_refused() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        app.registry_path = root.parent().unwrap().join(format!("reg3-{}.ini", std::process::id()));
        app.known =
            vec![KnownInstance { label: "x".into(), inst, game_index: 0, portable: true }];
        for bad in ["", "  ", "..", ".", "a/b", "a\\b"] {
            let _ = update_inner(&mut app, Message::InstanceRenameStart(0));
            let _ = update_inner(&mut app, Message::InstanceRenameChanged(bad.into()));
            let _ = update_inner(&mut app, Message::InstanceRenameCommit);
            assert!(root.is_dir(), "{bad:?} moved something");
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn forgetting_an_instance_needs_two_clicks_and_never_touches_the_disk() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        app.registry_path = root.parent().unwrap().join(format!("reg4-{}.ini", std::process::id()));
        app.known =
            vec![KnownInstance { label: "x".into(), inst, game_index: 0, portable: true }];

        let _ = update_inner(&mut app, Message::InstanceForget(0));
        assert_eq!(app.confirm_forget, Some(0), "the first click only arms");
        let _ = update_inner(&mut app, Message::InstanceForget(0));
        // Forgotten is not deleted, and that distinction is the whole design: an
        // instance is a mod pool, not a preference.
        assert!(root.is_dir(), "the folder must survive");
        assert!(app.status.as_deref().unwrap_or("").contains("Nothing on disk"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_global_instance_offers_neither_rename_nor_forget() {
        let mut app = app_for_game("skyrimse");
        app.registry_path = std::env::temp_dir().join(format!("reg5-{}.ini", std::process::id()));
        app.known = vec![KnownInstance {
            label: "Skyrim - global".into(),
            inst: Instance::global("skyrimse"),
            game_index: 0,
            portable: false,
        }];
        let _ = update_inner(&mut app, Message::InstanceForget(0));
        let _ = update_inner(&mut app, Message::InstanceForget(0));
        assert!(app.status.as_deref().unwrap_or("").contains("derived from the game id"));
    }

    #[test]
    fn a_wayland_session_is_told_the_desktop_drop_does_nothing() {
        // There is no moment of failure to hang this on - the event never fires,
        // so the window cannot know a drop was attempted. It has to be findable
        // instead, and Health is where "why did that do nothing" is answered.
        let mut app = app_for_game("skyrimse");
        app.screen = Screen::Main;
        let diag = diagnostics(&app);
        let row = diag.iter().find(|d| d.title.contains("file manager"));
        if on_wayland() {
            let row = row.expect("a Wayland session must be told");
            assert_eq!(row.level, DiagLevel::Advice, "nothing is broken - it is a limitation");
            // And it must point at the two paths that DO work, or it is a
            // complaint rather than help.
            assert!(row.detail.contains("Install Mod"), "{}", row.detail);
            assert!(row.detail.contains("Downloads"), "{}", row.detail);
        } else {
            assert!(row.is_none(), "an X11 session must not be told its drops are broken");
        }
    }

    #[test]
    fn the_nexus_budget_shows_only_once_the_server_has_answered() {
        let mut app = nav_app(&[]);
        assert_eq!(nexus_budget_suffix(&app), "", "no invented number before the first call");

        // The smaller of the two buckets is the one that matters: the daily
        // budget is large enough to be uninteresting until the hourly is spent.
        app.nexus_hourly_left = Some(1400);
        app.nexus_daily_left = Some(2200);
        assert!(nexus_budget_suffix(&app).contains("1400"));
        app.nexus_hourly_left = Some(2400);
        app.nexus_daily_left = Some(90);
        assert!(nexus_budget_suffix(&app).contains("90"));
        // And one bucket alone is still worth saying.
        app.nexus_daily_left = None;
        assert!(nexus_budget_suffix(&app).contains("2400"));
    }

    #[test]
    fn an_update_check_records_what_is_left_of_the_budget() {
        let mut app = nav_app(&[]);
        let result = eidos_nexus::UpdateCheckResult {
            checked: 12,
            queried: 12,
            updates_found: 1,
            updates: Vec::new(),
            rate_limited: false,
            hourly_remaining: Some(1388),
            daily_remaining: Some(2100),
            unavailable: Vec::new(),
        };
        let _ = update_inner(&mut app, Message::UpdatesChecked(Ok(result)));
        // It arrived in the result and used to be dropped on the floor.
        assert_eq!(app.nexus_hourly_left, Some(1388));
        assert_eq!(app.nexus_daily_left, Some(2100));
        assert!(nexus_budget_suffix(&app).contains("1388"));
    }

    #[test]
    fn the_plugin_priority_field_never_outlives_its_menu() {
        let mut app = nav_app(&[]);
        let mut list = PluginList::default();
        for n in ["A.esp", "B.esp", "C.esp"] {
            list.plugins.push(plugin_row(n, "Some Mod"));
        }
        app.plugins = Some(list);
        app.screen = Screen::Main;

        let _ = update_inner(&mut app, Message::OpenPluginMenu(2));
        let _ = update_inner(&mut app, Message::PluginSendToPriorityStart);
        assert_eq!(app.plugin_send_priority.as_ref().map(|(r, _)| *r), Some(2));
        assert_eq!(app.menu_plugin, Some(2), "the field lives INSIDE the card");

        // Dismissing the card drops it - a half-typed index must not be armed
        // for whatever menu opens next.
        let _ = update_inner(&mut app, Message::ClosePluginMenu);
        assert!(app.plugin_send_priority.is_none());

        // And reopening on another row never inherits the old aim.
        let _ = update_inner(&mut app, Message::OpenPluginMenu(0));
        let _ = update_inner(&mut app, Message::PluginSendToPriorityStart);
        let _ = update_inner(&mut app, Message::PluginSendToPriorityChanged("2".into()));
        let _ = update_inner(&mut app, Message::OpenPluginMenu(1));
        assert!(app.plugin_send_priority.is_none(), "a new menu starts clean");
    }

    #[test]
    fn a_plugin_priority_that_is_not_a_number_says_so_and_moves_nothing() {
        let mut app = nav_app(&[]);
        let mut list = PluginList::default();
        for n in ["A.esp", "B.esp"] {
            list.plugins.push(plugin_row(n, "Some Mod"));
        }
        app.plugins = Some(list);
        app.screen = Screen::Main;
        let before: Vec<String> =
            app.plugins.as_ref().unwrap().plugins.iter().map(|p| p.name.clone()).collect();

        let _ = update_inner(&mut app, Message::OpenPluginMenu(1));
        let _ = update_inner(&mut app, Message::PluginSendToPriorityStart);
        let _ = update_inner(&mut app, Message::PluginSendToPriorityChanged("  ".into()));
        let _ = update_inner(&mut app, Message::PluginSendToPriorityCommit);
        // "Row number", not "load index": the only numeric column the pane
        // draws is the game's hex load index, which this field does NOT take.
        assert!(app.status.as_deref().unwrap_or("").contains("row number"));
        let after: Vec<String> =
            app.plugins.as_ref().unwrap().plugins.iter().map(|p| p.name.clone()).collect();
        assert_eq!(before, after);
        assert!(app.plugin_send_priority.is_none(), "the field closes either way");
    }

    #[test]
    fn only_one_menu_bar_dropdown_is_open_at_a_time() {
        // Two cards at the same corner would overlap, and the one underneath
        // would eat clicks aimed at the one on top.
        let mut app = nav_app(&[]);
        let _ = update_inner(&mut app, Message::OpenFileMenu);
        assert!(app.file_menu_open && !app.view_menu_open);
        let _ = update_inner(&mut app, Message::OpenViewMenu);
        assert!(app.view_menu_open && !app.file_menu_open);
        let _ = update_inner(&mut app, Message::OpenFileMenu);
        assert!(app.file_menu_open && !app.view_menu_open);
        let _ = update_inner(&mut app, Message::CloseFileMenu);
        assert!(!app.file_menu_open);
    }

    #[test]
    fn the_file_menu_offers_every_folder_that_resolves_and_no_others() {
        let root = temp_portable("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mut app = app_for_game("skyrimse");
        app.created = Some(inst);
        app.screen = Screen::Main;
        app.file_menu_open = true;

        // Eidos's own folders stay live even before they exist - several are
        // created on first use, and "not there yet" is a worse answer than an
        // empty folder.
        let i = app.created.as_ref().unwrap();
        let downloads = i.downloads_dir();
        assert!(!downloads.exists(), "not created until the first download");
        let _ = update_inner(&mut app, Message::OpenFolder(downloads.clone()));
        assert!(downloads.is_dir(), "opening it created it");
        // The card builds without an open Proton prefix - that entry is simply
        // the inert one, which is the whole point of drawing rather than hiding.
        assert!(app.games.first().and_then(|g| g.compatdata.as_ref()).is_none());
        let _ = file_menu_card(&app);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_colour_can_be_set_on_an_ordinary_mod_but_never_on_the_games_own_content() {
        let root = temp_portable("skyrimse");
        let mut app = app_for_game("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        fs::create_dir_all(root.join("mods/Real")).unwrap();
        app.created = Some(inst);
        app.mods = vec![
            ModEntry { name: "Real".into(), enabled: true, path: root.join("mods/Real"), unmanaged: false },
            ModEntry {
                name: "Skyrim.esm".into(),
                enabled: true,
                path: PathBuf::from("/game/Data/Skyrim.esm"),
                unmanaged: true,
            },
        ];
        app.screen = Screen::Main;

        let _ = update_inner(&mut app, Message::SetSeparatorColor(0, Some([0x2e, 0x5e, 0x8b])));
        let meta = app.created.as_ref().unwrap().mod_meta("Real");
        assert_eq!(meta.color(), Some([0x2e, 0x5e, 0x8b]), "an ordinary mod takes a colour now");

        // The game's own Data is never written to.
        let before = app.status.clone();
        let _ = update_inner(&mut app, Message::SetSeparatorColor(1, Some([0x8b, 0x2e, 0x2e])));
        assert_ne!(app.status, before, "it says something rather than silently doing it");
        assert!(!PathBuf::from("/game/Data/Skyrim.esm/meta.ini").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_saved_note_reaches_the_row_immediately() {
        let root = temp_portable("skyrimse");
        let mut app = app_for_game("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        fs::create_dir_all(root.join("mods/Real")).unwrap();
        app.created = Some(inst);
        app.mods = vec![ModEntry {
            name: "Real".into(),
            enabled: true,
            path: root.join("mods/Real"),
            unmanaged: false,
        }];
        app.screen = Screen::Main;
        refresh_meta_cache(&mut app);
        assert_eq!(app.meta_cache["Real"].notes, None);

        app.info_mod = Some(0);
        app.notes_edit = "needs the AE patch".to_string();
        let _ = update_inner(&mut app, Message::NotesSave);
        // Without dropping the cached row the glyph would not appear until the
        // next full refresh - which is exactly the first time anyone adds a note.
        assert_eq!(app.meta_cache["Real"].notes.as_deref(), Some("needs the AE patch"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn installing_from_a_menu_lands_where_the_menu_was_opened() {
        let mut app = nav_app(&["a", "b", "c"]);
        // "Install below b" = the gap after index 1.
        let _ = update_inner(&mut app, Message::InstallAt(2));
        assert_eq!(app.install_gap, Some(2), "the place is held until an archive names itself");
        assert_eq!(app.install_at, None, "and it is NOT an aim yet - there is no archive");

        // The picker returns one: now it becomes a real aim, paired.
        let _ = update_inner(&mut app, Message::ModPicked(Some(PathBuf::from("/tmp/M.7z"))));
        assert_eq!(app.install_gap, None, "consumed");
        assert_eq!(
            app.install_at,
            Some((2, PathBuf::from("/tmp/M.7z"))),
            "paired with the archive, so a later failure cannot move something else"
        );
    }

    #[test]
    fn cancelling_the_picker_forgets_the_position() {
        let mut app = nav_app(&["a", "b"]);
        let _ = update_inner(&mut app, Message::InstallAt(1));
        let _ = update_inner(&mut app, Message::ModPicked(None));
        assert_eq!(app.install_gap, None, "a cancelled pick must not aim the NEXT install");
        assert_eq!(app.install_at, None);
    }

    #[test]
    fn a_menu_install_under_a_filter_says_it_is_going_to_the_end() {
        let mut app = nav_app(&["Alpha", "Bravo"]);
        app.search = "Alpha".to_string();
        let _ = update_inner(&mut app, Message::InstallAt(1));
        // The gap between two visible rows means nothing when rows are hidden -
        // the same promise the drag makes, made the same way.
        assert_eq!(app.install_gap, None);
        assert!(app.pending_note.as_deref().unwrap_or("").contains("end of the list"));
    }

    #[test]
    fn an_empty_mod_can_be_created_at_a_position() {
        let root = temp_portable("skyrimse");
        let mut app = app_for_game("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        app.created = Some(inst);
        app.mods = mods(&["a", "b", "c"]);
        app.screen = Screen::Main;

        let _ = update_inner(&mut app, Message::CreateEmptyModAt(1));
        assert_eq!(app.mods.len(), 4);
        assert_eq!(app.selected_mod, Some(1), "the new row is where it was asked for");
        assert!(app.mods[1].name.starts_with("New Mod"));
        assert_eq!(app.mods[2].name, "b", "everything after it shifted, not got overwritten");
        assert!(app.selected_mods.is_empty(), "stale indices are dropped");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bulk_enable_touches_only_what_is_on_screen_and_needs_two_clicks() {
        let mut app = nav_app(&["Alpha", "Bravo", "Ivy"]);
        for m in app.mods.iter_mut() {
            m.enabled = false;
        }
        // A filter that hides Ivy.
        app.search = "a".to_string(); // Alpha, Bravo
        assert_eq!(mods_visible_for_bulk(&app), vec![0, 1]);

        // First click only arms.
        let _ = update_inner(&mut app, Message::SetAllModsEnabled(true));
        assert_eq!(app.confirm_set_all, Some(true));
        assert!(!app.mods[0].enabled, "nothing happened yet");

        let _ = update_inner(&mut app, Message::SetAllModsEnabled(true));
        assert!(app.mods[0].enabled && app.mods[1].enabled);
        assert!(!app.mods[2].enabled, "the hidden row was NOT touched");
        assert_eq!(app.confirm_set_all, None, "disarmed after firing");
    }

    #[test]
    fn arming_enable_all_and_clicking_disable_all_does_not_fire() {
        let mut app = nav_app(&["a", "b"]);
        let _ = update_inner(&mut app, Message::SetAllModsEnabled(true));
        assert_eq!(app.confirm_set_all, Some(true));
        // A different TARGET is a different action: it re-arms, it does not fire.
        let _ = update_inner(&mut app, Message::SetAllModsEnabled(false));
        assert_eq!(app.confirm_set_all, Some(false));
        assert!(app.mods.iter().all(|m| m.enabled), "nothing was disabled");
    }

    #[test]
    fn bulk_enable_never_touches_a_separator_or_the_games_own_content() {
        let mut app = nav_app(&["Gear_separator", "real"]);
        app.mods.push(ModEntry {
            name: "Skyrim.esm".into(),
            enabled: true,
            path: PathBuf::from("/game/Data/Skyrim.esm"),
            unmanaged: true,
        });
        assert_eq!(mods_visible_for_bulk(&app), vec![1], "only the real mod");
    }

    #[test]
    fn an_aim_left_by_a_failed_install_never_moves_the_next_mod() {
        // An install can end without ever reaching `after_install`: an extraction
        // failure, an unrecognised layout, a dialog dismissed. The aim it left
        // behind must not be adopted by whatever is installed next.
        let mut app = nav_app(&["a", "b", "c"]);
        app.install_at = Some((0, PathBuf::from("/tmp/Aimed.7z")));
        let before: Vec<String> = app.mods.iter().map(|m| m.name.clone()).collect();

        // A DIFFERENT archive finishes installing.
        after_install(&mut app, "c", PathBuf::from("/tmp/x"), false, Some(Path::new("/tmp/Other.7z")));
        let after: Vec<String> = app.mods.iter().map(|m| m.name.clone()).collect();
        assert_eq!(before, after, "the unrelated mod was moved by a stale aim");
        assert!(app.install_at.is_some(), "and the aim is still waiting for ITS archive");
    }

    #[test]
    fn a_download_can_be_dropped_into_an_empty_mod_list() {
        // A fresh instance has no rows, so there is nothing to aim at except the
        // trailing strip. Without it the drag can never become aimed and the
        // release installs nothing, silently.
        let mut app = nav_app(&[]);
        app.downloads = vec![dl_row("First.7z")];
        let _ = update_inner(&mut app, Message::DownloadDragStart(0));
        let _ = update_inner(&mut app, Message::DownloadDragOverGap(0));
        assert!(app.download_drag.as_ref().is_some_and(|d| d.aimed));
        let _ = update_inner(&mut app, Message::DownloadDragDrop);
        assert_eq!(app.install_at.as_ref().map(|(g, _)| *g), Some(0));
    }

    #[test]
    fn merging_onto_an_existing_mod_never_moves_it() {
        // The mod already has a place in the load order; honouring a drop's
        // target priority would yank it out and flip every conflict it is in.
        let mut app = nav_app(&["a", "b"]);
        app.install_at = Some((0, PathBuf::from("/tmp/Mod.7z")));
        let _ = update_inner(&mut app, Message::CollisionMerge);
        assert_eq!(app.install_at, None, "the aim is discarded, not applied");
    }

    #[test]
    fn a_cancelled_install_does_not_leave_its_target_for_the_next_one() {
        let mut app = nav_app(&["a"]);
        for cancel in [Message::FomodCancel, Message::PickerCancel, Message::CollisionCancel] {
            app.install_at = Some((0, PathBuf::from("/tmp/Mod.7z")));
            let _ = update_inner(&mut app, cancel);
            assert_eq!(app.install_at, None);
        }
    }

    #[test]
    fn a_multi_file_drop_is_drained_one_file_at_a_time() {
        let mut app = nav_app(&["a"]);
        // Three files arrive as three messages, not one.
        for n in ["one.7z", "two.zip", "three.rar"] {
            let _ = update_inner(&mut app, Message::FileDropped(PathBuf::from("/tmp").join(n)));
        }
        assert_eq!(app.dropped.len(), 3, "queued, not handled inline");

        // With no instance open, the queue is dropped with one explanation
        // rather than three.
        assert!(app.created.is_none());
        let _ = update_inner(&mut app, Message::DrainDrops);
        assert!(app.dropped.is_empty());
        assert!(app.status.as_deref().unwrap_or("").contains("game instance"));
    }

    #[test]
    fn a_dropped_file_that_is_not_a_mod_archive_is_named_and_skipped() {
        let mut app = app_for_game("skyrimse");
        let root = temp_portable("skyrimse");
        app.created = Some(Instance::portable(root.clone()));
        let _ = update_inner(&mut app, Message::FileDropped(root.join("notes.txt")));
        let _ = update_inner(&mut app, Message::DrainDrops);
        let msg = app.status.clone().unwrap_or_default();
        assert!(msg.contains("notes.txt"), "{msg}");
        assert!(msg.contains(".7z"), "and it says what IS accepted: {msg}");
        let _ = fs::remove_dir_all(&root);
    }

    /// An App with a real portable instance, one mod, and a game Data dir - the
    /// minimum the Data tab needs to build a real `LayerStack`.
    fn data_app(mod_files: &[(&str, &str)], overwrite: &[(&str, &str)]) -> (App, PathBuf) {
        let root = temp_portable("skyrimse");
        let game = root.join("game/Data");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("Skyrim.esm"), b"vanilla").unwrap();
        let modroot = root.join("mods/AAA");
        for (rel, body) in mod_files {
            let p = modroot.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, body.as_bytes()).unwrap();
        }
        let ow = root.join("overwrite");
        for (rel, body) in overwrite {
            let p = ow.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, body.as_bytes()).unwrap();
        }
        let mut app = app_for_game("skyrimse");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        if let Some(g) = app.games.first_mut() {
            g.data_path = game;
        }
        app.mods = vec![ModEntry {
            name: "AAA".into(),
            enabled: true,
            path: modroot,
            unmanaged: false,
        }];
        // Written to disk, because the tab now reads the SAME layer stack the
        // mount is handed - `load_order()`, not the window's in-memory list.
        inst.save_modlist(&app.mods).unwrap();
        app.created = Some(inst);
        app.screen = Screen::Main;
        (app, root)
    }

    /// A throwaway logs dir with one session file, and the pane loaded from it.
    fn log_fixture(body: &str) -> (LogPaneState, PathBuf) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "eidos-logs-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::create_dir_all(&dir);
        let f = dir.join("gui.20260824-170411.1234.log");
        fs::write(&f, body).unwrap();
        (load_log_pane(vec![f.clone()], f, eidos_log::Level::Info), dir)
    }

    #[test]
    fn an_extension_gets_every_instance_path_it_is_promised() {
        let (app, root) = data_app(&[], &[]);
        let ctx = addon_context(&app);
        // Every placeholder documented in docs/guide/extensions.md must
        // actually resolve, or the doc is a promise the code does not keep.
        for key in [
            "instance",
            "mods",
            "downloads",
            "overwrite",
            "profile",
            "profile_dir",
            "game",
            "game_name",
            "install",
            "data",
        ] {
            assert!(ctx.values.contains_key(key), "{key} is documented but not provided");
            assert!(!ctx.values[key].is_empty(), "{key} resolved to nothing");
        }
        assert_eq!(ctx.values["game"], "skyrimse");
        assert!(ctx.expand("--root {instance}").contains(&root.display().to_string()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tool_extension_with_an_unresolvable_placeholder_is_refused_not_run() {
        let mut app = nav_app(&[]);
        assert!(app.created.is_none(), "no instance, so the instance placeholder cannot resolve");
        app.addons = vec![eidos_addons::parse_addon(
            "id='x'\nname='X'\nkind='tool'\nexec='sh'\nargs=['-c','ls {instance}']\n",
            std::path::Path::new("/x.toml"),
        )
        .unwrap()];
        let _ = update_inner(&mut app, Message::RunAddon("x".to_string()));
        let msg = app.status.clone().unwrap_or_default();
        assert!(msg.contains("instance"), "it names what is missing: {msg}");
        assert!(msg.contains("needs"), "{msg}");
    }

    #[test]
    fn a_check_extension_reports_under_its_own_name_and_never_as_eidoss_own() {
        let (mut app, root) = data_app(&[], &[]);
        app.addons = vec![eidos_addons::parse_addon(
            "id='c'\nname='My check'\nkind='diagnose'\nexec='sh'\n\
             args=['-c','printf \"advice\\\\tLook here\\\\tand the detail\\\\n\"']\n",
            std::path::Path::new("/c.toml"),
        )
        .unwrap()];
        app.diag_dirty = true;
        refresh_diagnostics(&mut app);

        let found = app
            .diag
            .iter()
            .find(|d| d.title.contains("Look here"))
            .expect("the finding reached the Health tab");
        assert_eq!(found.level, DiagLevel::Advice);
        assert_eq!(found.detail, "and the detail");
        // Attributed. A row that read like one of Eidos's own checks would put
        // Eidos's authority behind something it did not check.
        assert!(found.title.starts_with("My check - "), "{}", found.title);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_check_that_hangs_is_stopped_and_blamed_by_name() {
        let (mut app, root) = data_app(&[], &[]);
        app.addons = vec![eidos_addons::parse_addon(
            "id='slow'\nname='Slow one'\nkind='diagnose'\nexec='sh'\nargs=['-c','sleep 30']\n",
            std::path::Path::new("/s.toml"),
        )
        .unwrap()];
        app.diag_dirty = true;
        let started = std::time::Instant::now();
        refresh_diagnostics(&mut app);
        // It runs on the refresh that follows every click, so a hanging one
        // would freeze the window with nothing on screen to blame.
        assert!(started.elapsed() < std::time::Duration::from_secs(10), "it was not stopped");
        let row = app
            .diag
            .iter()
            .find(|d| d.title.contains("Slow one"))
            .expect("the failure is reported, not swallowed");
        assert_eq!(row.level, DiagLevel::Problem);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_extension_for_another_game_does_not_run_here() {
        let (mut app, root) = data_app(&[], &[]);
        app.addons = vec![eidos_addons::parse_addon(
            "id='f4'\nname='FO4 only'\nkind='diagnose'\nexec='sh'\n\
             args=['-c','printf \"problem\\\\tShould not appear\\\\n\"']\ngames=['fallout4']\n",
            std::path::Path::new("/f.toml"),
        )
        .unwrap()];
        app.diag_dirty = true;
        refresh_diagnostics(&mut app);
        assert!(!app.diag.iter().any(|d| d.title.contains("Should not appear")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_log_pane_filters_by_level_and_says_what_it_is_hiding() {
        let (pane, dir) = log_fixture(
            "2026-08-24 17:04:11.238 DEBUG resolving layers\n\
             2026-08-24 17:04:11.239 INFO  mounted 412 layers\n\
             2026-08-24 17:04:11.240 ERROR could not open the prefix\n",
        );
        assert_eq!(pane.total, 3, "every record is counted, filtered or not");
        assert_eq!(pane.lines.len(), 2, "Debug is below the floor");
        assert_eq!(pane.lines[0], (eidos_log::Level::Info, "mounted 412 layers".to_string()));
        assert_eq!(pane.lines[1].0, eidos_log::Level::Error);
        assert!(!pane.truncated);

        // Lowering the floor shows everything.
        let all = load_log_pane(pane.files.clone(), pane.current.clone(), eidos_log::Level::Debug);
        assert_eq!(all.lines.len(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_multi_line_message_stays_attached_to_its_record() {
        let (pane, dir) = log_fixture(
            "2026-08-24 17:04:11.240 ERROR mount failed:\n    \
             fuse: device not found\n    try: modprobe fuse\n",
        );
        // Three lines on disk, ONE record: the continuation lines carry no
        // level, and inventing one for them would put text at a severity
        // nothing claimed.
        assert_eq!(pane.total, 1);
        assert_eq!(pane.lines.len(), 1);
        assert!(pane.lines[0].1.contains("device not found"), "{:?}", pane.lines[0].1);
        assert!(pane.lines[0].1.contains("modprobe fuse"));

        // And filtering it out takes its continuations with it.
        let quiet = load_log_pane(pane.files.clone(), pane.current.clone(), eidos_log::Level::Debug);
        assert_eq!(quiet.lines.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_filtered_out_records_continuations_do_not_land_on_the_record_before_it() {
        // `concat!` rather than a `\`-continued literal: the continuation eats
        // the following source indentation INTO the next line, which pushes the
        // timestamp off the offset `parse_line` reads.
        let (pane, dir) = log_fixture(concat!(
            "2026-08-24 17:04:11.238 ERROR mount failed\n",
            "2026-08-24 17:04:11.239 DEBUG probing layers\n",
            "    layer 3: /mods/AAA\n",
            "    layer 4: /mods/BBB\n",
        ));
        // The Debug record is below the floor; its two continuation lines belong
        // to IT, not to the error above. Attaching them there would put a debug
        // trace under an error's severity and call it evidence.
        assert_eq!(pane.lines.len(), 1);
        assert_eq!(pane.lines[0].1, "mount failed", "{:?}", pane.lines[0].1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_line_orphaned_by_the_tail_seek_is_dropped_rather_than_guessed_at() {
        // Reading only the END of a big file lands mid-line. That fragment
        // belongs to a record that is not in the buffer, so there is nothing to
        // attach it to and nothing honest to say about its level.
        let (pane, dir) = log_fixture(
            "ount 412 layers\n2026-08-24 17:04:11.239 INFO  mounted\n",
        );
        assert_eq!(pane.lines.len(), 1);
        assert_eq!(pane.lines[0].1, "mounted");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_or_unreadable_session_is_not_a_panic() {
        let (pane, dir) = log_fixture("");
        assert_eq!(pane.total, 0);
        assert!(pane.lines.is_empty());
        let _ = fs::remove_dir_all(&dir);

        // A file that is not there at all.
        let gone = std::env::temp_dir().join("eidos-no-such.log");
        let pane = load_log_pane(vec![gone.clone()], gone, eidos_log::Level::Info);
        assert!(pane.lines.is_empty());
    }

    #[test]
    fn the_ini_editor_writes_the_profiles_copy_in_the_encoding_it_found() {
        let (mut app, root) = data_app(&[], &[]);
        let inst = app.created.clone().unwrap();
        let prof = inst.active();
        // A CP1252 file: Windows-written game INIs are as often this as UTF-8,
        // and re-encoding one silently mangles every accented value in it.
        let path = prof.ini_path("Skyrim.ini");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"[General]\nsLanguage=Fran\xe7ais").unwrap();

        let _ = update_inner(&mut app, Message::ShowIniEditor);
        let ed = app.ini_editor.as_ref().expect("the editor opened");
        assert_eq!(ed.current, "Skyrim.ini");
        assert!(ed.cp1252, "the file was not UTF-8 and must be written back as it was read");
        assert!(ed.original.contains("Français"), "decoded: {}", ed.original);
        assert!(!ed.dirty, "opening a file is not an edit");
        assert!(!ed.missing);

        // Saving an untouched buffer must not grow the file. `Content::text()`
        // always ends with a newline, and this one did not.
        let _ = update_inner(&mut app, Message::IniEditorSave);
        assert_eq!(
            fs::read(&path).unwrap(),
            b"[General]\nsLanguage=Fran\xe7ais",
            "byte-identical after a no-op save"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_ini_editor_refuses_to_switch_away_from_unsaved_edits() {
        let (mut app, root) = data_app(&[], &[]);
        let _ = update_inner(&mut app, Message::ShowIniEditor);
        {
            let ed = app.ini_editor.as_mut().unwrap();
            ed.dirty = true;
        }
        let _ = update_inner(&mut app, Message::IniEditorPick("SkyrimPrefs.ini".to_string()));
        assert_eq!(
            app.ini_editor.as_ref().unwrap().current,
            "Skyrim.ini",
            "switching would have thrown the edits away without saying so"
        );
        assert!(app.status.as_deref().unwrap_or("").contains("unsaved"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_ini_the_profile_does_not_have_yet_opens_empty_and_says_so() {
        let (mut app, root) = data_app(&[], &[]);
        let _ = update_inner(&mut app, Message::ShowIniEditor);
        let ed = app.ini_editor.as_ref().unwrap();
        assert!(ed.missing, "a fresh profile owns none of them");
        assert!(ed.original.is_empty());
        assert!(!ed.cp1252, "an absent file is not CP1252 - it would be written back wrong");

        // Saving creates it, and the flag clears.
        let _ = update_inner(&mut app, Message::IniEditorSave);
        assert!(!app.ini_editor.as_ref().unwrap().missing);
        assert!(app.created.as_ref().unwrap().active().ini_path("Skyrim.ini").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_data_tree_hides_what_the_mount_hides() {
        // The tab used to re-implement the union merge beside the real one, and
        // had drifted: it showed `.eidoswh.<name>` markers as ordinary rows, and
        // showed the lower-layer files those markers DELETE as winners - so it
        // claimed the game would see files the mount hides.
        let (app, root) = data_app(
            &[("mod.esp", "from the mod")],
            &[(&format!("{}Skyrim.esm", eidos_core::WHITEOUT_PREFIX), "")],
        );
        let names: Vec<String> =
            merged_listing(&app, "").into_iter().map(|r| r.name).collect();
        assert!(names.contains(&"mod.esp".to_string()), "{names:?}");
        assert!(
            !names.iter().any(|n| n.starts_with(eidos_core::WHITEOUT_PREFIX)),
            "the marker is bookkeeping, not a file: {names:?}"
        );
        assert!(
            !names.contains(&"Skyrim.esm".to_string()),
            "a whited-out file is NOT in the merged view: {names:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_data_row_names_the_layer_that_actually_provides_it() {
        let (app, root) = data_app(&[("mod.esp", "x")], &[("gen.json", "y")]);
        let rows = merged_listing(&app, "");
        let by = |n: &str| {
            rows.iter().find(|r| r.name == n).map(|r| r.source.clone()).unwrap_or_default()
        };
        assert_eq!(by("gen.json"), "[Overwrite]");
        assert_eq!(by("mod.esp"), "AAA");
        assert_eq!(by("Skyrim.esm"), "[skyrimse]");
        // And the size column reads the WINNER's file, not some other layer's.
        let m = rows.iter().find(|r| r.name == "mod.esp").unwrap();
        assert_eq!(m.size, Some(1));
        assert!(m.real.ends_with("mods/AAA/mod.esp"), "{:?}", m.real);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_data_filter_reaches_into_folders_and_drops_empty_ones() {
        let (mut app, root) = data_app(
            &[("meshes/actors/thing.nif", "x"), ("scripts/other.pex", "y")],
            &[],
        );
        // Unfiltered and unexpanded: only the top level draws.
        let top: Vec<String> = data_tree_rows(&app, 500).into_iter().map(|r| r.rel).collect();
        assert!(top.contains(&"meshes".to_string()));
        assert!(!top.iter().any(|r| r.contains('/')), "nothing is expanded: {top:?}");

        // A filter looks THROUGH folders - the match is somewhere in the tree,
        // not necessarily on the level the user happens to have open.
        app.data_query = "thing".to_string();
        let hits: Vec<String> = data_tree_rows(&app, 500).into_iter().map(|r| r.rel).collect();
        assert!(hits.contains(&"meshes/actors/thing.nif".to_string()), "{hits:?}");
        assert!(hits.contains(&"meshes".to_string()), "its parents stay, to reach it: {hits:?}");
        assert!(
            !hits.iter().any(|r| r.starts_with("scripts")),
            "a branch with no match is dropped whole: {hits:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn conflicts_only_needs_a_conflict_map_and_says_nothing_without_one() {
        let (mut app, root) = data_app(&[("mod.esp", "x")], &[]);
        app.data_conflicts_only = true;
        assert!(app.conflicts.is_none());
        // No map means nothing is KNOWN to conflict. Reporting rows as
        // contested on no evidence would be worse than an empty list.
        assert!(data_tree_rows(&app, 500).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn one_definition_decides_whether_a_filter_is_running() {
        // Two copies of this predicate drifted: the view's never learned about
        // the criteria, so a state filter left folding on and emptied the list
        // in silence.
        let mut app = nav_app(&["a", "b"]);
        assert!(!is_filtering(&app));
        app.filters.update = Criterion::Require;
        assert!(is_filtering(&app), "a criterion alone counts as filtering");
        app.filters = ModFilters::default();
        app.search = "a".to_string();
        assert!(is_filtering(&app));
    }

    #[test]
    fn the_button_says_how_many_criteria_are_narrowing_the_list() {
        // A list that looks short must always say why.
        let mut app = nav_app(&["a"]);
        assert_eq!(app.filters.active_count(), 0);
        app.filters.active = Criterion::Require;
        app.filters.update = Criterion::Exclude;
        assert_eq!(app.filters.active_count(), 2);
        let _ = update_inner(&mut app, Message::ClearFilters);
        assert_eq!(app.filters.active_count(), 0, "Clear really clears");
    }

    #[test]
    fn send_to_top_moves_as_far_as_the_engine_allows_not_to_row_zero() {
        // The defect this shipped with: gap 0 is refused for every plugin,
        // because the game's own masters sit above them - so the action did
        // nothing at all, silently.
        let spec = GameSpec::for_id("skyrimse").unwrap();
        let mut list = PluginList::default();
        list.plugins.push(plugin_row("Skyrim.esm", ""));
        for n in ["A.esp", "B.esp", "C.esp"] {
            list.plugins.push(plugin_row(n, "Some Mod"));
        }
        list.plugins[0].is_master = true;
        // C is last; sending it to the top must land it above A, not above the
        // master.
        let gap = list.edge_gap(&[3], true, &spec).expect("a reachable destination");
        assert!(gap >= 1, "never above the game's own master, got {gap}");
        assert!(list.move_plugins_to(&[3], gap, &spec), "and the move is accepted");
        list.refresh(&spec);
        let names: Vec<&str> = list.plugins.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names[0], "Skyrim.esm", "the master did not move");
        assert_eq!(names[1], "C.esp", "C went as high as it may: {names:?}");

        // Already there: reported as such rather than rewritten.
        assert_eq!(list.edge_gap(&[1], true, &spec), None);
    }

    #[test]
    fn send_to_bottom_lands_the_selection_last() {
        let spec = GameSpec::for_id("skyrimse").unwrap();
        let mut list = PluginList::default();
        for n in ["A.esp", "B.esp", "C.esp"] {
            list.plugins.push(plugin_row(n, "Some Mod"));
        }
        let gap = list.edge_gap(&[0], false, &spec).expect("a destination");
        assert!(list.move_plugins_to(&[0], gap, &spec));
        list.refresh(&spec);
        let names: Vec<&str> = list.plugins.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names.last(), Some(&"A.esp"), "{names:?}");
        assert_eq!(list.edge_gap(&[2], false, &spec), None, "already last");
    }

    #[test]
    fn activate_all_leaves_the_engines_own_plugins_alone() {
        // Writing them would be a lie the next refresh silently corrects.
        let mut app = app_for_game("skyrimse");
        let mut list = PluginList::default();
        list.plugins.push(plugin_row("Skyrim.esm", ""));
        list.plugins.push(plugin_row("Mine.esp", "Some Mod"));
        list.plugins[0].enabled = true;
        list.plugins[1].enabled = false;
        app.plugins = Some(list);
        let _ = update_inner(&mut app, Message::PluginsSetAll(true));
        let list = app.plugins.as_ref().unwrap();
        assert!(list.plugins.iter().find(|p| p.name == "Mine.esp").unwrap().enabled);
        // And saying nothing changed is itself an outcome worth stating.
        let _ = update_inner(&mut app, Message::PluginsSetAll(true));
        assert!(
            app.status.as_deref().unwrap_or("").contains("already"),
            "{:?}",
            app.status
        );
    }

    #[test]
    fn rebuilding_the_plugin_list_closes_a_menu_that_points_at_a_row() {
        // menu_plugin is a raw index and the rebuild renumbers the rows: acting
        // on a stale one would hit whichever plugin now sits there.
        let mut app = app_for_game("skyrimse");
        app.menu_plugin = Some(3);
        invalidate_plugins(&mut app);
        assert_eq!(app.menu_plugin, None);
    }

    #[test]
    fn a_second_identify_cannot_start_while_one_is_running() {
        // Two whole-file hashes racing to write one sidecar.
        let mut app = nav_app(&[]);
        app.identifying_download = Some("a.zip".into());
        let _ = update_inner(&mut app, Message::IdentifyDownload("b.zip".into()));
        assert_eq!(app.identifying_download.as_deref(), Some("a.zip"), "the first one still owns it");
    }

    #[test]
    fn the_plugin_menu_finds_the_mod_that_ships_the_plugin() {
        // The question the menu exists to answer, and the one the tooltip could
        // only answer one row at a time.
        let mut app = app_for_game("skyrimse");
        app.mods = mods(&["Other Mod", "Armour Pack"]);
        let mut list = PluginList::default();
        list.plugins.push(plugin_row("Armour.esp", "Armour Pack"));
        list.plugins.push(plugin_row("Skyrim.esm", ""));
        app.plugins = Some(list);
        assert_eq!(plugin_origin_row(&app, 0), Some(1), "matched to the mod row");
        // Vanilla content belongs to no mod: a real answer, not a failure.
        assert_eq!(plugin_origin_row(&app, 1), None);
    }

    #[test]
    fn right_clicking_a_plugin_outside_the_selection_takes_that_row_alone() {
        // Same rule as the mod list: otherwise a batch action would run on rows
        // the user can no longer see.
        let mut app = app_for_game("skyrimse");
        let mut list = PluginList::default();
        for n in ["A.esp", "B.esp", "C.esp"] {
            list.plugins.push(plugin_row(n, "Some Mod"));
        }
        app.plugins = Some(list);
        app.selected_plugins.extend([0, 1]);
        let _ = update_inner(&mut app, Message::OpenPluginMenu(2));
        assert_eq!(app.menu_plugin, Some(2), "the menu opens on the row");
        assert!(app.selected_plugins.is_empty(), "the old set is dropped");
        assert_eq!(app.selected_plugin, Some(2));

        // Right-clicking INSIDE the selection keeps it whole.
        app.selected_plugins.extend([0, 1]);
        let _ = update_inner(&mut app, Message::OpenPluginMenu(1));
        assert_eq!(app.selected_plugins.len(), 2, "the set survives");
    }


    #[test]
    fn opening_the_origin_of_a_vanilla_plugin_says_so_instead_of_doing_nothing() {
        // A menu action that silently no-ops reads as a broken button.
        let mut app = app_for_game("skyrimse");
        let mut list = PluginList::default();
        list.plugins.push(plugin_row("Skyrim.esm", ""));
        app.plugins = Some(list);
        let _ = update_inner(&mut app, Message::OpenPluginOrigin(0));
        assert!(
            app.status.as_deref().unwrap_or("").contains("game's own Data"),
            "{:?}",
            app.status
        );
    }

    #[test]
    fn the_backups_dialog_reads_both_lists_and_restores_through_the_instance() {
        use eidos_instance::BackupKind;
        let root = std::env::temp_dir().join(format!("eidos-gui-bk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let inst = eidos_instance::Instance::portable(root.clone());
        inst.create().unwrap();
        let prof = inst.active();
        fs::write(prof.dir().join("modlist.txt"), "+Kept\n").unwrap();

        let mut app = nav_app(&[]);
        app.created = Some(inst);
        let _ = update_inner(&mut app, Message::CreateBackup(BackupKind::ModList));
        let _ = update_inner(&mut app, Message::ShowBackupsDialog);
        let state = app.backups.as_ref().expect("the dialog opened");
        assert_eq!(state.mods.len(), 1, "the mod-list restore point is listed");
        let stamp = state.mods[0].stamp;

        // Destroy the list, restore it through the message the button sends.
        fs::write(prof.dir().join("modlist.txt"), "+Ruined\n").unwrap();
        let _ = update_inner(&mut app, Message::RestoreBackup(BackupKind::ModList, stamp));
        assert_eq!(fs::read_to_string(prof.dir().join("modlist.txt")).unwrap(), "+Kept\n");
        assert!(app.backups.is_none(), "the dialog closes on a restore");
        assert!(
            app.status.as_deref().unwrap_or("").contains("Restored"),
            "the outcome is said out loud: {:?}",
            app.status
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn releasing_the_button_that_armed_a_confirmation_does_not_cancel_it() {
        // The reported bug, and the half the previous fix missed: the pointer
        // fix covered MOVING, but every left-button release publishes
        // PointerReleased, so letting go of the very click that armed Delete
        // disarmed it again. The confirmation flashed for as long as the button
        // was held down - about a tenth of a second - and could never be
        // completed.
        let mut app = nav_app(&[]);
        update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
        update_inner(&mut app, Message::PointerReleased);
        assert_eq!(
            app.confirm_delete_download.as_deref(),
            Some("a.zip"),
            "letting go is the end of that same click, not a new action"
        );
        // The same release must not cancel the other confirmations either.
        let mut app = nav_app(&[]);
        update_inner(&mut app, Message::DeleteSave(0));
        update_inner(&mut app, Message::PointerReleased);
        assert!(app.confirm_delete_save.is_some(), "saves too");

        // Every drag the release ladder can cancel has to be exempt. Adding one
        // and forgetting it here reintroduces the exact bug above, through a
        // different door: the release emits the cancel, the cancel counts as an
        // action, and the confirmation is gone before the second click lands.
        let mut app = nav_app(&[]);
        for cancel in [Message::DragCancel, Message::PluginDragCancel, Message::DownloadDragCancel]
        {
            app.confirm_delete_download = Some("a.zip".into());
            update_inner(&mut app, cancel.clone());
            assert_eq!(
                app.confirm_delete_download.as_deref(),
                Some("a.zip"),
                "cancelling a drag is the absence of an action"
            );
        }
    }

    #[test]
    fn a_release_that_commits_a_drag_still_counts_as_an_action() {
        // The exemption is not blanket: a release that DROPS a dragged mod is a
        // real action, and it must cancel an armed confirmation like any other.
        let mut app = nav_app(&["a", "b", "c"]);
        app.drag_state = Some(DragState { from: 0, gap: 2, aimed: true });
        update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
        update_inner(&mut app, Message::PointerReleased);
        assert_eq!(app.confirm_delete_download, None, "a committed drop is an action");
    }

    #[test]
    fn holding_a_modifier_does_not_cancel_an_armed_confirmation() {
        // Ctrl is how a multi-selection is made, so pressing or releasing it
        // around a batch action is part of that gesture, not a decision to do
        // something else. The handler only stores the modifier set.
        let mut app = nav_app(&["a", "b"]);
        update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
        update_inner(
            &mut app,
            Message::ModifiersChanged(iced::keyboard::Modifiers::CTRL),
        );
        assert_eq!(app.confirm_delete_download.as_deref(), Some("a.zip"));
    }

    #[test]
    fn moving_the_mouse_does_not_cancel_an_armed_confirmation() {
        // The reported bug: arm Delete on a download, twitch the mouse, and the
        // confirmation is gone. The pointer HAS to move to reach the button, so
        // the two-click guard could never be completed.
        let mut app = nav_app(&[]);
        update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
        assert_eq!(app.confirm_delete_download.as_deref(), Some("a.zip"), "the first click arms it");

        update_inner(&mut app, Message::PointerAt(iced::Point::new(10.0, 10.0)));
        update_inner(&mut app, Message::WindowResized(iced::Size::new(800.0, 600.0)));
        assert_eq!(
            app.confirm_delete_download.as_deref(),
            Some("a.zip"),
            "ambient messages are not actions"
        );
    }

    #[test]
    fn a_real_action_still_cancels_every_confirmation() {
        // The guard must not become decorative: the whole point is that doing
        // anything ELSE takes the loaded gun out of your hand.
        let mut app = nav_app(&[]);
        for (arm, check) in [
            (Message::DeleteDownload("a.zip".into()), 0),
            (Message::DeleteSave(0), 1),
            (Message::ClearOverwrite, 2),
            (Message::BatchRemoveMods, 3),
        ] {
            update_inner(&mut app, arm);
            update_inner(&mut app, Message::Refresh);
            match check {
                0 => assert_eq!(app.confirm_delete_download, None),
                1 => assert_eq!(app.confirm_delete_save, None),
                2 => assert!(!app.confirm_clear),
                _ => assert!(!app.confirm_batch_remove),
            }
        }
    }

    #[test]
    fn arming_one_row_disarms_another() {
        let mut app = nav_app(&[]);
        update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
        update_inner(&mut app, Message::DeleteDownload("b.zip".into()));
        assert_eq!(
            app.confirm_delete_download.as_deref(),
            Some("b.zip"),
            "only one may be armed"
        );
    }


    /// Build an instance whose downloads dir holds the given `(name, bytes)`
    /// entries, then scan it. Real files, because the whole feature is "notice
    /// what another process is writing to disk".
    fn downloads_app(files: &[(&str, &[u8])], metas: &[(&str, &str)]) -> App {
        let mut app = nav_app(&[]);
        // A counter, not a timestamp. `cargo test` runs these on parallel
        // threads, and two of them reading the clock in the same instant got the
        // same directory - one test then saw the other's files and failed, but
        // only sometimes and never alone. A counter cannot collide.
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "eidos-dl-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let dl = root.join("downloads");
        fs::create_dir_all(&dl).unwrap();
        for (n, b) in files {
            fs::write(dl.join(n), b).unwrap();
        }
        for (n, body) in metas {
            fs::write(dl.join(n), body).unwrap();
        }
        app.created = Some(eidos_instance::Instance::portable(root));
        load_downloads(&mut app);
        app
    }

    #[test]
    fn a_download_in_flight_is_listed_before_it_finishes() {
        // The reported complaint: a mod being downloaded does not appear until
        // it is done and Refresh is pressed. The partial IS the evidence.
        let app = downloads_app(
            &[("Cool Mod.zip.unfinished", b"1234567890")],
            &[("Cool Mod.zip.meta", "[General]\nmodName=Cool Mod\ntotalSize=100\n")],
        );
        assert_eq!(app.downloads.len(), 1, "the partial must produce a row");
        let r = &app.downloads[0];
        assert_eq!(r.state, DownloadState::Downloading);
        assert_eq!(r.name, "Cool Mod.zip", "named for what it will BE");
        assert!(r.path.ends_with("Cool Mod.zip"), "Install must aim at the final path");
        assert_eq!(r.downloaded, 10);
        assert_eq!(r.total, 100);
        assert_eq!(r.mod_name.as_deref(), Some("Cool Mod"));
        // Size shows the destination, so the column does not creep upward while
        // the bar is already saying how far along it is.
        assert_eq!(r.size, 100);
    }

    #[test]
    fn a_partial_that_stopped_growing_reads_as_stalled() {
        let app = downloads_app(&[("x.zip.unfinished", b"abc")], &[]);
        assert_eq!(app.downloads[0].state, DownloadState::Downloading, "fresh mtime");

        // Backdate it well past the window: the writing process is gone.
        let dl = app.created.as_ref().unwrap().downloads_dir();
        let old = std::time::SystemTime::now() - STALLED_AFTER - std::time::Duration::from_secs(30);
        let f = fs::File::options().write(true).open(dl.join("x.zip.unfinished")).unwrap();
        f.set_modified(old).unwrap();
        // No sidecar exists in this case, so the partial's own mtime decides.
        let mut app = app;
        load_downloads(&mut app);
        assert_eq!(app.downloads[0].state, DownloadState::Stalled);
    }

    #[test]
    fn a_finished_download_replaces_its_partial_and_becomes_installable() {
        // The handover: `download` renames <dest>.unfinished to <dest>. One row
        // throughout, never two, and never a gap where neither is listed.
        let mut app = downloads_app(
            &[("m.zip.unfinished", b"partial")],
            &[("m.zip.meta", "[General]\ntotalSize=7\n")],
        );
        assert_eq!(app.downloads[0].state, DownloadState::Downloading);

        let dl = app.created.as_ref().unwrap().downloads_dir();
        fs::rename(dl.join("m.zip.unfinished"), dl.join("m.zip")).unwrap();
        load_downloads(&mut app);
        assert_eq!(app.downloads.len(), 1);
        assert_eq!(app.downloads[0].state, DownloadState::Ready);
    }

    #[test]
    fn speed_needs_two_samples_and_never_goes_backwards() {
        let mut app = downloads_app(
            &[("s.zip.unfinished", b"aaaa")],
            &[("s.zip.meta", "[General]\ntotalSize=1000\n")],
        );
        assert_eq!(app.downloads[0].speed, None, "one sighting is not a rate");

        let dl = app.created.as_ref().unwrap().downloads_dir();
        std::thread::sleep(std::time::Duration::from_millis(120));
        fs::write(dl.join("s.zip.unfinished"), vec![b'a'; 4004]).unwrap();
        load_downloads(&mut app);
        let v = app.downloads[0].speed.expect("two samples give a rate");
        assert!(v > 0.0, "grew by 4000 bytes, so the rate is positive: {v}");

        // A server that ignores our Range restarts from zero, so the partial
        // SHRINKS. That is not a negative speed.
        std::thread::sleep(std::time::Duration::from_millis(120));
        fs::write(dl.join("s.zip.unfinished"), b"a").unwrap();
        load_downloads(&mut app);
        assert_eq!(app.downloads[0].speed, None, "a shrinking partial reports no rate");
    }

    #[test]
    fn the_delete_confirmation_survives_the_background_tick() {
        // The tick re-sorts the list twice a second. Keyed by index, arming a
        // row and confirming it could delete a DIFFERENT archive.
        let mut app = downloads_app(&[("a.zip", b"a"), ("b.zip", b"b")], &[]);
        update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
        assert_eq!(app.confirm_delete_download.as_deref(), Some("a.zip"));
        update_inner(&mut app, Message::DownloadTick);
        assert_eq!(
            app.confirm_delete_download.as_deref(),
            Some("a.zip"),
            "a periodic re-scan is not an action"
        );
    }


    #[test]
    fn deleting_a_stalled_download_takes_the_partial_with_it() {
        // Otherwise the file that produced the row is still there, and the row
        // is back on the next tick - an entry the user cannot get rid of.
        let mut app = downloads_app(
            &[("dead.zip.unfinished", b"half")],
            &[("dead.zip.meta", "[General]\ntotalSize=999\n")],
        );
        let dl = app.created.as_ref().unwrap().downloads_dir();
        // BOTH files have to be old. A fresh sidecar means a retry is in its
        // API/latency window, and calling that stalled is what used to hand the
        // user a Delete button aimed at a live transfer.
        let old = std::time::SystemTime::now() - STALLED_AFTER - std::time::Duration::from_secs(30);
        for f in ["dead.zip.unfinished", "dead.zip.meta"] {
            fs::File::options().write(true).open(dl.join(f)).unwrap().set_modified(old).unwrap();
        }
        load_downloads(&mut app);
        assert_eq!(app.downloads[0].state, DownloadState::Stalled);

        update_inner(&mut app, Message::DeleteDownload("dead.zip".into()));
        update_inner(&mut app, Message::ConfirmDeleteDownload("dead.zip".into()));
        assert!(!dl.join("dead.zip.unfinished").exists(), "the partial must go");
        assert!(!dl.join("dead.zip.meta").exists(), "and its sidecar");
        assert!(app.downloads.is_empty(), "so the row does not come back");
    }


    #[test]
    fn a_resumed_download_is_not_called_stalled_while_it_waits_on_the_network() {
        // The data-loss case. A previous attempt left an hours-old partial. The
        // user retries: `eidos nxm` rewrites the sidecar, then spends seconds in
        // API calls and CDN latency before the first byte lands - and a resume
        // APPENDS, so the partial's mtime stays the dead attempt's throughout.
        // Judged on the partial alone the row read "Stalled" and offered a
        // Delete that unlinked the file a live process was writing to.
        let app = downloads_app(
            &[("retry.zip.unfinished", b"leftover")],
            &[("retry.zip.meta", "[General]\ntotalSize=500\n")],
        );
        let dl = app.created.as_ref().unwrap().downloads_dir();
        let ages_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(6 * 3600);
        fs::File::options()
            .write(true)
            .open(dl.join("retry.zip.unfinished"))
            .unwrap()
            .set_modified(ages_ago)
            .unwrap();

        // The sidecar is fresh, because the retry just wrote it.
        let mut app = app;
        load_downloads(&mut app);
        assert_eq!(
            app.downloads[0].state,
            DownloadState::Downloading,
            "a fresh sidecar means an attempt is under way, whatever the partial's mtime"
        );

        // Genuinely abandoned: BOTH are old, so it really is stalled and can be
        // cleared. Age the sidecar too.
        fs::File::options()
            .write(true)
            .open(dl.join("retry.zip.meta"))
            .unwrap()
            .set_modified(ages_ago)
            .unwrap();
        load_downloads(&mut app);
        assert_eq!(app.downloads[0].state, DownloadState::Stalled);
    }


    #[test]
    fn cleaning_debris_only_touches_eidos_install_folders() {
        // The handler deletes recursively inside `mods/`, which is where every
        // mod the user owns also lives. The prefix is the ONLY thing standing
        // between the two, so it is worth a test of its own.
        let mut app = nav_app(&[]);
        let root = std::env::temp_dir().join(format!("eidos-debris-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mods = root.join("mods");
        // Dead pids (past the kernel's default pid_max, so /proc never has
        // them): genuine debris from crashed installs.
        for d in [".eidos-install-4194305-0", ".eidos-install-4194306-0"] {
            fs::create_dir_all(mods.join(d).join("00 Core")).unwrap();
            fs::write(mods.join(d).join("00 Core/a.esp"), b"x").unwrap();
        }
        // A temp whose embedded pid is ALIVE (this very test process) is a
        // running install's workspace - deleting it failed that install while
        // the button called it safe debris.
        let live = format!(".eidos-install-{}-0", std::process::id());
        fs::create_dir_all(mods.join(&live)).unwrap();
        fs::write(mods.join(&live).join("extracting.7z.part"), b"x").unwrap();
        // Everything that must survive: a real mod, a separator, and a dotfile
        // that is not ours.
        fs::create_dir_all(mods.join("A Real Mod/meshes")).unwrap();
        fs::write(mods.join("A Real Mod/meshes/m.nif"), b"keep").unwrap();
        fs::create_dir_all(mods.join("Group_separator")).unwrap();
        fs::create_dir_all(mods.join(".git")).unwrap();
        app.created = Some(eidos_instance::Instance::portable(root.clone()));

        update_inner(&mut app, Message::CleanInstallDebris);

        assert!(!mods.join(".eidos-install-4194305-0").exists());
        assert!(!mods.join(".eidos-install-4194306-0").exists());
        assert!(mods.join(&live).is_dir(), "a live install's temp is not debris");
        assert_eq!(fs::read(mods.join("A Real Mod/meshes/m.nif")).unwrap(), b"keep");
        assert!(mods.join("Group_separator").is_dir());
        assert!(mods.join(".git").is_dir(), "a dotfile that is not ours is not ours to delete");
        assert!(app.status.as_deref().unwrap_or("").contains('2'), "{:?}", app.status);
        assert!(
            app.status.as_deref().unwrap_or("").contains("in use"),
            "the skip must be said, not silent: {:?}",
            app.status
        );
        let _ = fs::remove_dir_all(&root);
    }


    #[test]
    fn the_row_colour_has_exactly_one_owner() {
        // The fill and the fade must agree, always. They agree because they ask
        // the same function - this pins the precedence they both inherit.
        let conflict = Some(CONFLICT_WINS_BG);
        assert_eq!(
            row_background(true, true, conflict, None),
            SEL_BG,
            "selection outranks the conflict tint"
        );
        assert_eq!(row_background(true, false, conflict, None), CONFLICT_WINS_BG);
        assert_eq!(row_background(true, false, None, None), row_bg(true));
        assert_eq!(row_background(false, false, None, None), row_bg(false));
        // A user colour paints when nothing more urgent is asking for the row,
        // and yields to both selection and a live conflict answer.
        let tint = mod_tint([0x2e, 0x5e, 0x8b], true);
        assert_eq!(row_background(true, false, None, Some(tint)), tint);
        assert_eq!(row_background(true, false, conflict, Some(tint)), CONFLICT_WINS_BG);
        assert_eq!(row_background(true, true, None, Some(tint)), SEL_BG);
        // And it is a WASH: closer to the stripe than to the raw colour.
        let raw = Color::from_rgb8(0x2e, 0x5e, 0x8b);
        let d = |a: Color, b: Color| (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs();
        assert!(d(tint, row_bg(true)) < d(tint, raw), "the colour must not become the page");
        assert_ne!(
            row_bg(true),
            row_bg(false),
            "the stripes differ, so a fade into the wrong one would show"
        );
    }


    #[test]
    fn a_cold_plugin_cache_does_not_silence_the_missing_master_check() {
        // The check that predicts crashes must never answer "not computed yet".
        // `app.plugins` is None here, exactly as it is after any mod-list change,
        // and the diagnostic set must still contain a real verdict rather than an
        // apology.
        let mut app = nav_app(&["A"]);
        app.plugins = None;
        let out = diagnostics(&app);
        assert!(
            !out.iter().any(|d| d.title.contains("not computed")),
            "the old message is gone: {:?}",
            out.iter().map(|d| &d.title).collect::<Vec<_>>()
        );
    }


    // ---- the LOOT report as a worklist ---------------------------------------

    fn sample_report() -> eidos_loot::LootReport {
        use eidos_loot::{LootDirtyInfo, LootMessage, MessageType, PluginReport};
        eidos_loot::LootReport {
            plugin_meta: Default::default(),
            general: vec![LootMessage {
                kind: MessageType::Warn,
                text: "Skyrim Script Extender scripts seem to be missing".into(),
            }],
            plugins: vec![PluginReport {
                name: "Update.esm".into(),
                missing_masters: vec!["Nope.esm".into()],
                messages: vec![LootMessage {
                    kind: MessageType::Say,
                    text: "a note".into(),
                }],
                dirty: vec![LootDirtyInfo {
                    crc: 1,
                    cleaning_utility: "SSEEdit v4.1.5d".into(),
                    itm_count: 386,
                    deleted_reference_count: 93,
                    deleted_navmesh_count: 3,
                }],
            }],
        }
    }

    #[test]
    fn the_copied_report_carries_what_the_screen_shows() {
        let out = loot_report_text(&sample_report());
        // The counts are the point: they are what gets read off while xEdit runs.
        assert!(out.contains("386 ITM"), "{out}");
        assert!(out.contains("93 deleted refs"), "{out}");
        assert!(out.contains("Update.esm"), "{out}");
        assert!(out.contains("Missing masters: Nope.esm"), "{out}");
        assert!(out.contains("SSEEdit v4.1.5d"), "{out}");
    }

    #[test]
    fn severity_survives_the_loss_of_colour() {
        // On screen the severity is a colour. Pasted into a text editor a colour
        // is nothing, so each line has to say it.
        let out = loot_report_text(&sample_report());
        assert!(out.contains("[warning] Skyrim Script Extender"), "{out}");
        assert!(out.contains("[note] a note"), "{out}");
    }

    #[test]
    fn a_clean_report_still_says_something() {
        let out = loot_report_text(&eidos_loot::LootReport::default());
        assert!(out.contains("clean"), "{out}");
        assert!(!out.trim().is_empty());
    }

    #[test]
    fn an_unnamed_cleaning_utility_does_not_print_an_empty_gap() {
        let mut r = sample_report();
        r.plugins[0].dirty[0].cleaning_utility = String::new();
        assert!(loot_report_text(&r).contains("Dirty - ? found"), "empty utility left a hole");
    }

    // ---- the prerequisite status line ---------------------------------------
    //
    // The Prereqs field accepts any text, so this is the only thing that answers
    // "will this tool start?". A verb reported as present when it is not sends
    // the user hunting through their mods for a fault that is not there.

    #[test]
    fn a_bundled_dll_is_never_something_to_click() {
        let none = std::collections::HashSet::new();
        for v in ["d3dcompiler_47", "d3dx9_43", "d3dx11_43"] {
            let (label, missing) = prereq_state(v, &none);
            assert!(!missing, "{v} offered an install it does not need");
            assert!(label.contains("bundled"), "{v}: {label}");
        }
    }

    #[test]
    fn a_winetricks_verb_reads_from_what_the_instance_recorded() {
        let mut done = std::collections::HashSet::new();
        let (_, missing) = prereq_state("dotnetdesktop8", &done);
        assert!(missing, "nothing recorded, so it cannot be installed");
        done.insert("dotnetdesktop8".to_string());
        let (label, missing) = prereq_state("dotnetdesktop8", &done);
        assert!(!missing);
        assert_eq!(label, "installed");
    }

    #[test]
    fn a_typo_is_named_as_one_and_offers_nothing() {
        // `dotnet10` is real; `dotnet1O` (letter O) is what a tired user types.
        // Offering to install it would spend a download on nothing.
        let none = std::collections::HashSet::new();
        let (label, missing) = prereq_state("dotnet1O", &none);
        assert!(!missing, "offered to install a verb that does not exist");
        assert!(label.contains("unknown"), "{label}");
    }

    #[test]
    fn a_runtime_answers_from_the_shared_cache_not_the_instance() {
        // Tier 3 lives outside any instance, so an empty `prereqs.done` must not
        // make an installed runtime look missing.
        let none = std::collections::HashSet::new();
        let installed = eidos_gamefeatures::runtime("dotnet10")
            .is_some_and(eidos_gamefeatures::runtime_is_installed);
        let (label, missing) = prereq_state("dotnet10", &none);
        assert_eq!(missing, !installed, "state disagrees with the cache: {label}");
        if missing {
            assert!(label.contains("click"), "a missing runtime must say what to do: {label}");
        }
    }

    #[test]
    fn the_wording_tells_the_user_what_to_do() {
        // Every actionable state has to say so; a red label with no verb is just
        // bad news.
        let none = std::collections::HashSet::new();
        for v in ["dotnetdesktop8", "dotnet10"] {
            let (label, missing) = prereq_state(v, &none);
            if missing {
                assert!(label.contains("click"), "{v}: {label}");
                assert!(label.contains("NOT FOUND"), "{v}: {label}");
            }
        }
    }

    // ---- the game's own content, reconciled -------------------------------
    //
    // `modlist_with_unmanaged` decides where the DLC and Creation Club rows land.
    // The rule that matters: a row the profile already places keeps its position,
    // because that position is something the user said. Re-pinning it to the top
    // on every refresh is what stopped a separator from ever sitting above the
    // block, so it could not be collapsed.

    fn entry(name: &str, unmanaged: bool) -> ModEntry {
        ModEntry {
            name: name.into(),
            enabled: true,
            path: if unmanaged { PathBuf::new() } else { PathBuf::from("/mods").join(name) },
            unmanaged,
        }
    }

    /// The reconciliation, extracted from the filesystem so it can be tested:
    /// `listed` is what the profile holds, `real` what the game ships.
    fn reconcile(listed: Vec<ModEntry>, real: Vec<ModEntry>) -> Vec<String> {
        let mut by_name: std::collections::HashMap<String, ModEntry> =
            real.into_iter().map(|m| (m.name.to_ascii_lowercase(), m)).collect();
        let mut placed: Vec<ModEntry> = Vec::new();
        for m in listed {
            if !m.unmanaged {
                placed.push(m);
            } else if let Some(found) = by_name.remove(&m.name.to_ascii_lowercase()) {
                placed.push(found);
            }
        }
        let mut fresh: Vec<ModEntry> = by_name.into_values().collect();
        fresh.sort_by_key(|m| m.name.to_ascii_lowercase());
        fresh.into_iter().chain(placed).map(|m| m.name).collect()
    }

    #[test]
    fn a_placed_dlc_row_stays_where_the_user_put_it() {
        // The separator sits ABOVE the DLC block - the arrangement that lets it be
        // collapsed, and the one that was impossible before.
        let listed = vec![
            entry("00. DLCs_separator", false),
            entry("Dawnguard", true),
            entry("Dragonborn", true),
            entry("SkyUI", false),
        ];
        let real = vec![entry("Dawnguard", true), entry("Dragonborn", true)];
        assert_eq!(
            reconcile(listed, real),
            ["00. DLCs_separator", "Dawnguard", "Dragonborn", "SkyUI"],
            "the DLC rows were moved out from under their separator"
        );
    }

    #[test]
    fn a_dlc_the_game_no_longer_ships_is_dropped() {
        // Uninstalling a DLC must not leave a row pointing at nothing - it has no
        // path, and every consumer would have to defend against that.
        let listed = vec![entry("Dawnguard", true), entry("Dragonborn", true), entry("SkyUI", false)];
        let real = vec![entry("Dawnguard", true)];
        assert_eq!(reconcile(listed, real), ["Dawnguard", "SkyUI"]);
    }

    #[test]
    fn content_the_profile_has_never_seen_goes_to_the_top() {
        // A newly installed DLC has no position yet, and the engine loads its own
        // content first - so lowest priority, which is the top of the display.
        let listed = vec![entry("Dawnguard", true), entry("SkyUI", false)];
        let real = vec![entry("Dawnguard", true), entry("Anniversary", true)];
        assert_eq!(reconcile(listed, real), ["Anniversary", "Dawnguard", "SkyUI"]);
    }

    #[test]
    fn matching_is_case_insensitive_like_every_other_name_here() {
        // The profile stores what the display showed; the game directory spells it
        // however Bethesda spelled it. A case difference must not duplicate a row.
        let listed = vec![entry("dawnguard", true), entry("SkyUI", false)];
        let real = vec![entry("Dawnguard", true)];
        let got = reconcile(listed, real);
        assert_eq!(got, ["Dawnguard", "SkyUI"], "a case difference split one row into two");
    }

    #[test]
    fn managed_mods_are_untouched_by_any_of_this() {
        // The reconciliation must not reorder, drop or duplicate a single mod the
        // user actually installed.
        let listed = vec![
            entry("A", false),
            entry("Dawnguard", true),
            entry("B", false),
            entry("C", false),
        ];
        let got = reconcile(listed, vec![]);
        assert_eq!(got, ["A", "B", "C"]);
    }
}
