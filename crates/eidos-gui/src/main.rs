//! Eidos GUI (iced) - MO2-style wizard + two-pane main window.
//!
//!   Welcome -> Instance type (portable/global) -> Game -> Name/location
//!           -> Summary -> [create] -> Main (MO2-style mod manager)
//!
//! The main window mirrors Mod Organizer 2: menu bar + toolbar + profile row,
//! left = the mod list (enable, priority, reorder) with an Overwrite entry,
//! right = Run + Data/Saves/Downloads tabs, plus a status bar. Colony parchment
//! / burgundy palette. Run with: `cargo run -p eidos-gui`

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{
    button, checkbox, container, image, mouse_area, pick_list, scrollable, text, text_input, Column,
    Row, Space, Stack,
};
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::settings::{Settings, Theme as PrefTheme};
use eidos_instance::{Instance, InstanceKind, ModEntry, SaveEntry, Tool};
use eidos_plugins::{plugins_txt_dir, GameSpec, PluginList};
use eidos_conflicts::{ConflictMap, ConflictState, Layer};

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
const IC_UP: &[u8] = include_bytes!("../assets/icons/go-up.png");
const IC_DOWN: &[u8] = include_bytes!("../assets/icons/go-down.png");

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
    Notes,
}

/// Tabs of the Preferences modal (MO2's Settings dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    General,
    Nexus,
}

#[derive(Debug, Clone)]
enum Message {
    Next,
    Back,
    PickKind(InstanceKind),
    PickGame(usize),
    NameChanged(String),
    PortableChanged(String),
    Finish,
    Restart,
    ToolPicked(String),
    ToggleMod(usize),
    MoveUp(usize),
    MoveDown(usize),
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
    /// Reorder shortcuts (MO2 sendModsToTop / sendModsToBottom).
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
    /// Enable/disable an ESP/ESM in the Plugins tab, persisting plugins.txt.
    TogglePlugin(usize),
    /// Run LOOT's graph sort over the discovered plugins (MO2's "Sort" button).
    SortPlugins,
    /// LOOT finished: the optimised plugin-name order plus the (advisory) report,
    /// or an error. The inner `Result` is the report: it may fail without losing the
    /// successfully-computed order.
    PluginsSorted(Result<(Vec<String>, Result<eidos_loot::LootReport, String>), String>),
    /// Dismiss the LOOT report modal.
    CloseLootReport,
    // ---- per-mod information dialog (MO2 modinfodialog) ----
    ShowModInfo(usize),
    CloseInfo,
    InfoSelectTab(InfoTab),
    NotesChanged(String),
    NotesSave,
    // ---- toolbar ----
    /// Re-open the game picker to switch the managed game (MO2 switch-instance).
    ChangeGame,
    /// Open the current game's Nexus page in the browser.
    OpenNexusGame,
    /// Open the instance's root folder in the file manager.
    OpenInstanceFolder,
    /// Install the modding tools' runtime prerequisites into the prefix
    /// (`eidos prereqs <id> --install`); the Tier-2 verbs download from Microsoft.
    SetupPrereqs,
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
    ApiKeyChanged(String),
    /// Validate + persist the entered Nexus API key.
    ApiKeyValidateStart,
    /// The key validation finished: the account on success, else an error.
    /// Carries the key that was actually VALIDATED, so an edit made to the field
    /// during the round-trip is never saved as if it had been checked.
    ApiKeyValidateResult(String, Result<eidos_nexus::Account, String>),
    /// Set the preferred colour theme.
    ThemeChanged(PrefTheme),
    /// Set the default game id to open (`None` = none).
    DefaultGameChanged(Option<String>),
    /// Toggle "lock the GUI while a game/tool runs" (MO2's `lock_gui`).
    ToggleLockGui(bool),
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
    CreateEmptyMod,
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
    /// Re-scan the downloads directory + reload each archive's `.meta` status.
    RefreshDownloads,
    /// Delete a downloaded archive and its `.meta` sidecar (two-click confirm).
    DeleteDownload(usize),
    /// Second click: actually delete the armed download.
    ConfirmDeleteDownload(usize),
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
    DragOver(usize),
    /// The drag ended: commit the move if the drop row differs from the source.
    DragDrop,
    /// Abandon an in-flight drag (filter change / Escape).
    DragCancel,
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
    /// Move the Overwrite's contents into that mod.
    OverwriteToModCommit,
    /// Dismiss the prompt.
    OverwriteToModCancel,
    // ---- MO2 profile import ----
    /// Open the folder picker for an existing MO2 profile directory.
    ImportMo2Pick,
    /// The picked MO2 profile directory (`None` = cancelled).
    ImportMo2Picked(Option<PathBuf>),
    /// Open a URL in the user's browser (LOOT advice links in the report).
    OpenUrl(String),
    // ---- manual plugin reorder (MO2 lets the load order be dragged by hand) ----
    /// Move the plugin at this index one slot earlier / later in the load order.
    PluginMoveUp(usize),
    PluginMoveDown(usize),
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
            }
            None => {
                self.title.clear();
                self.exe.clear();
                self.workdir.clear();
                self.args.clear();
                self.prereqs.clear();
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
    }
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
}

/// The install status of a downloaded archive, derived from its `.meta` sidecar
/// (MO2's downloads-list state column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadState {
    /// No `.meta` sidecar (a manually dropped archive) - status unknown.
    Untracked,
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
    /// The derived install status.
    state: DownloadState,
}

/// An in-flight mod-row drag (MO2's drag-to-reorder). `from` is the grabbed row's
/// index in `app.mods`; `hover_over` is the row the pointer is currently over. The
/// move is only applied on release, and only when `from != hover_over`.
#[derive(Debug, Clone, Copy)]
struct DragState {
    from: usize,
    hover_over: usize,
}

/// One Data-tab row: entry name, the layer providing it, and whether it is a
/// folder (the merged view as the FUSE union would serve it).
type DataRow = (String, String, bool);

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
    // ---- Settings / Nexus account (the status bar + endorse/update read these) ----
    /// The Preferences modal is open.
    settings_open: bool,
    /// The active Preferences tab.
    settings_tab: SettingsTab,
    /// The editable Nexus API key field.
    settings_api_key: String,
    /// The validated Nexus account, if the stored key checked out (or was cached).
    nexus_account: Option<eidos_nexus::Account>,
    /// A key validation is in flight (guards the button + concurrent validations).
    api_key_validating: bool,
    /// The last key-validation error, shown inline in the dialog.
    api_key_error: Option<String>,
    /// The persisted app-global preferences (theme, default game).
    prefs: Settings,
    // ---- Executables dialog ----
    /// The open Executables editor, if any (None = closed).
    executables: Option<ExecutablesDialogState>,
    // ---- Endorse / update in-flight + counts ----
    /// The mod index whose Nexus endorse is in flight (greys the toolbar button).
    endorsing: Option<usize>,
    /// Enabled mods that are endorsed (recomputed in `mods_changed`).
    endorsed_count: usize,
    /// Enabled mods with a Nexus update available (recomputed in `mods_changed`).
    updated_count: usize,
    /// A Nexus mod-update check is in flight (guards the Update button).
    update_in_progress: bool,
    // ---- menu-bar UI toggles + About ----
    /// The toolbar / status bar are visible (View menu toggles).
    ui_toolbar_visible: bool,
    ui_statusbar_visible: bool,
    /// The View dropdown is open (iced has no native menu, so it's a floating card).
    view_menu_open: bool,
    /// The About box is open.
    about_open: bool,
    // ---- Saves tab ----
    /// The active profile's save files (newest first), lazily loaded.
    saves: Vec<SaveEntry>,
    /// Two-click guard for a save deletion (the save's index in `saves`).
    confirm_delete_save: Option<usize>,
    // ---- Downloads manager ----
    /// The completed downloads (cached so the panel does not re-scan on redraw).
    downloads: Vec<DownloadRow>,
    /// Two-click guard for a download deletion (the row's index in `downloads`).
    confirm_delete_download: Option<usize>,
    // ---- multi-select + batch actions ----
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
    /// The `eidos` binary lacks CAP_SYS_ADMIN (setcap wiped by a rebuild): FUSE
    /// passthrough will be off and SKSE plugin DLLs may fail to load. Drives the
    /// persistent warning banner; rechecked on Refresh and after every run.
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
    /// Memoised Data-tab merged listing, with the generation it was built at.
    data_listing: std::cell::RefCell<Option<(u64, Vec<DataRow>)>>,
    /// Memoised recursive file listings per directory (the Overwrite tab and the
    /// mod-info file tree), each with the generation it was built at.
    listing_cache: std::cell::RefCell<HashMap<PathBuf, (u64, Vec<String>)>>,
    // ---- LOOT report (MO2's post-sort report dialog) ----
    /// The report from the last LOOT sort, shown as a modal so the user sees
    /// missing masters / messages / dirty-plugin advice. `None` = no report open.
    loot_report: Option<eidos_loot::LootReport>,
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
    /// A separator's display colour (MO2's `color=@Variant(...)`), if set.
    color: Option<[u8; 3]>,
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

/// Build the per-mod metadata cache for the open instance's mod list.
fn build_meta_cache(app: &App) -> HashMap<String, RowMeta> {
    let mut out = HashMap::new();
    if let Some(inst) = &app.created {
        // The category catalog (resolves `category=` ids to names); built once.
        let cats = inst.category_factory();
        for m in &app.mods {
            let meta = inst.mod_meta(&m.name);
            let category_id = meta.category().as_deref().and_then(eidos_instance::parse_primary);
            let category_name = category_id.and_then(|id| cats.name_for_id(id)).map(str::to_string);
            out.insert(
                m.name.clone(),
                RowMeta {
                    version: meta.version(),
                    mod_id: meta.mod_id(),
                    category_id,
                    category_name,
                    content_tags: eidos_install::classify_content_dir(&m.path).tags(),
                    update: meta.update_available(),
                    color: meta.color(),
                },
            );
        }
    }
    out
}

/// The run-target picker entry meaning "the game itself".
const RUN_GAME: &str = "Game (Steam command)";

fn new(launch_command: Vec<String>) -> (App, Task<Message>) {
    let games = detect(&home());
    // If Steam launched us with the game's command (`eidos-gui %command%`),
    // identify the game and open straight to its instance, like MO2 does.
    let auto = identify_game(&games, &launch_command);
    let mut app = App {
        screen: Screen::Welcome,
        games,
        kind: InstanceKind::Global,
        portable_path: String::new(),
        selected: None,
        name: String::new(),
        created: None,
        error: None,
        mods: Vec::new(),
        plugins: None,
        conflicts: None,
        tab: Tab::Data,
        status: None,
        confirm_clear: false,
        overwrite_to_mod: None,
        send_priority: None,
        send_separator: None,
        launch_command,
        fomod: None,
        collision: None,
        tools: Vec::new(),
        tool_choice: None,
        search: String::new(),
        selected_mod: None,
        menu_mod: None,
        rename: None,
        meta_cache: HashMap::new(),
        confirm_remove: None,
        info_mod: None,
        info_tab: InfoTab::General,
        notes_edit: String::new(),
        collapsed: HashSet::new(),
        category_filter: None,
        settings_open: false,
        settings_tab: SettingsTab::Nexus,
        // Prefill the key field from the shared store (the same key `eidos nexus
        // key` writes), so it survives across sessions without a network round trip.
        settings_api_key: eidos_instance::settings::load_nexus_key().unwrap_or_default(),
        nexus_account: None,
        api_key_validating: false,
        api_key_error: None,
        prefs: Settings::load(),
        executables: None,
        endorsing: None,
        endorsed_count: 0,
        updated_count: 0,
        update_in_progress: false,
        ui_toolbar_visible: true,
        ui_statusbar_visible: true,
        view_menu_open: false,
        about_open: false,
        saves: Vec::new(),
        confirm_delete_save: None,
        downloads: Vec::new(),
        confirm_delete_download: None,
        selected_mods: HashSet::new(),
        confirm_batch_remove: false,
        modifiers: iced::keyboard::Modifiers::default(),
        drag_state: None,
        profile_menu: None,
        profile_rename: None,
        profile_copy: None,
        profile_delete_confirm: None,
        running: None,
        cap_missing: !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary()),
        files_cache: std::cell::RefCell::new(HashMap::new()),
        view_generation: std::cell::Cell::new(0),
        data_listing: std::cell::RefCell::new(None),
        listing_cache: std::cell::RefCell::new(HashMap::new()),
        loot_report: None,
    };
    if let Some(i) = auto {
        app.selected = Some(i);
        let inst = Instance::global(app.games[i].def.id);
        if inst.exists() {
            let _ = inst.ensure_manifest(app.games[i].def.id, InstanceKind::Global);
            let _ = inst.ensure_profiles();
            app.mods = inst.modlist();
            app.created = Some(inst);
            app.screen = Screen::Main;
            app.status =
                Some("Launched from Steam. Click Run to start the game through Eidos.".to_string());
        }
    } else {
        // Standalone: open the first detected game that already has an instance,
        // so `eidos-gui` lands on your existing setup instead of the wizard.
        for (i, g) in app.games.iter().enumerate() {
            let inst = Instance::global(g.def.id);
            if inst.exists() {
                let _ = inst.ensure_manifest(g.def.id, InstanceKind::Global);
                let _ = inst.ensure_profiles();
                app.selected = Some(i);
                app.mods = inst.modlist();
                app.created = Some(inst);
                app.screen = Screen::Main;
                break;
            }
        }
    }
    load_tools(&mut app);
    // Conflicts feed the mod-list emblems, so compute them as soon as the
    // instance opens instead of waiting for the Conflicts tab.
    app.conflicts = compute_conflicts(&app);
    app.meta_cache = build_meta_cache(&app);
    app.collapsed = load_collapsed(&app);
    recompute_counts(&mut app);
    // A stored key means the user IS connected: validate it in the background so
    // the status bar shows the account instead of "not logged in" every session.
    let startup = match load_nexus_api_key() {
        Some(key) => Task::perform(
            async move {
                let result = eidos_nexus::Nexus::new(&key).validate();
                (key, result)
            },
            |(key, result)| Message::ApiKeyValidateResult(key, result),
        ),
        None => Task::none(),
    };
    (app, startup)
}

/// Reload the tool list for the open instance (user `tools.ini` + per-game
/// defaults), keeping the current pick when it still exists.
/// The auto-detectable executables for a game (launcher, binary, script extender),
/// from its `GameDef` - fed to `default_tools` for MO2-style file-existence detection.
fn game_executables(g: &eidos_games::DetectedGame) -> eidos_instance::GameExecutables<'_> {
    eidos_instance::GameExecutables {
        game_name: g.def.name,
        launcher: g.def.script_extender.as_ref().map(|se| se.launcher),
        binary: Some(g.def.game_binary),
        script_extender: g.def.script_extender.as_ref().map(|se| se.loader),
    }
}

fn load_tools(app: &mut App) {
    let merged = match (selected_game(app), &app.created) {
        (Some(g), Some(inst)) => eidos_instance::merge_tools(
            inst.tools(),
            eidos_instance::default_tools_in(
                game_executables(g),
                &g.install_path,
                &app.created.as_ref().map(|i| i.root_layers()).unwrap_or_default(),
            ),
        ),
        _ => Vec::new(),
    };
    if let Some(t) = &app.tool_choice {
        if !merged.iter().any(|x| x.title.eq_ignore_ascii_case(t)) {
            app.tool_choice = None;
        }
    }
    app.tools = merged;
}

/// The stored Nexus API key (the same key the CLI's `eidos nexus key` writes),
/// shared via `eidos-instance`'s settings store so the key never diverges.
fn load_nexus_api_key() -> Option<String> {
    eidos_instance::settings::load_nexus_key()
}

/// Build the Executables editor state for the open instance: the user's tools.ini
/// entries (editable) followed by the per-game defaults (read-only). Recomputed
/// every open so a game switch picks up the right script-extender defaults; `None`
/// when no instance is open.
fn open_executables_dialog(app: &App) -> Option<ExecutablesDialogState> {
    let (game, inst) = (selected_game(app)?, app.created.as_ref()?);
    let user = inst.tools();
    let user_len = user.len();
    // Widened to enabled mods' Root/ dirs, so a script extender installed as a
    // mod is detected; the root union puts it on the game root at launch.
    let roots = app.created.as_ref().map(|i| i.root_layers()).unwrap_or_default();
    let defaults =
        eidos_instance::default_tools_in(game_executables(game), &game.install_path, &roots);
    let merged = eidos_instance::merge_tools(user, defaults);
    let mut state = ExecutablesDialogState {
        merged,
        user_len,
        selected: None,
        title: String::new(),
        exe: String::new(),
        workdir: String::new(),
        args: String::new(),
        prereqs: String::new(),
    };
    // Select the first user tool, if any, so the editor opens with something.
    if user_len > 0 {
        state.selected = Some(0);
        state.load_buffers();
    }
    Some(state)
}

/// The `eidos tool <id> run <title>` command: the CLI resolves the tool + Proton
/// and runs it through the merged view (same single-process requirement as
/// `play`). Returned unspawned so `start_run` can route its output to a log.
fn tool_command(game_id: &str, title: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(find_eidos_binary());
    cmd.arg("tool").arg(game_id).arg("run").arg(title);
    cmd
}

/// Install the tools' runtime prerequisites into the prefix (`eidos prereqs <id>
/// --install`). The Tier-2 winetricks step downloads from Microsoft and can take a
/// while; its output is redirected to `log` (the GUI has no terminal when launched
/// from Steam) so the user can follow progress and read any error.
fn run_prereqs_setup(game_id: &str, log: &Path) -> std::io::Result<()> {
    let out = std::fs::File::create(log)?;
    let err = out.try_clone()?;
    std::process::Command::new(find_eidos_binary())
        .arg("prereqs")
        .arg(game_id)
        .arg("--install")
        .stdout(std::process::Stdio::from(out))
        .stderr(std::process::Stdio::from(err))
        .spawn()
        .map(|_| ())
}

/// Identify which detected game a Steam `%command%` is launching, by matching
/// each game's install directory against the command's arguments.
fn identify_game(games: &[DetectedGame], command: &[String]) -> Option<usize> {
    for arg in command {
        for (i, g) in games.iter().enumerate() {
            if let Some(dir) = g.data_path.parent() {
                if arg.contains(&*dir.to_string_lossy()) {
                    return Some(i);
                }
            }
        }
    }
    None
}

/// The (launcher exe, script-extender loader) swap for a game: launching through
/// Eidos runs the script extender (SKSE/F4SE/...) instead of the vanilla
/// launcher, matching how a modded game is actually played.
fn script_extender_swap(game_id: &str) -> Option<(&'static str, &'static str)> {
    eidos_games::GameDef::for_id(game_id)
        .and_then(|g| g.script_extender)
        .map(|se| (se.launcher, se.loader))
}

/// Find the `eidos` CLI that drives the namespaced launch. The GUI is
/// multi-threaded, so it cannot enter a user namespace itself; the single-process
/// `eidos` binary can. Prefer a sibling of this binary, then `~/.cargo/bin`, then
/// `PATH`.
fn find_eidos_binary() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let sib = exe.with_file_name("eidos");
        if sib.is_file() {
            return sib;
        }
    }
    let cargo = home().join(".cargo").join("bin").join("eidos");
    if cargo.is_file() {
        return cargo;
    }
    PathBuf::from("eidos")
}

/// Launch the game through Eidos: spawn `eidos play <id> -- <command>` (with the
/// script-extender swap applied), which mounts the merged mods over the game's
/// Data dir in a private namespace and runs the command through it.
/// Build the `eidos play` command, swapping the vanilla launcher for the script
/// extender's loader - but only if the loader actually exists on disk (a swap to
/// a missing skse64_loader.exe would just make Proton fail cryptically). Returns
/// the command plus a warning to surface when the extender is not installed.
fn play_command(game_id: &str, command: &[String]) -> (std::process::Command, Option<String>) {
    let mut swapped: Vec<String> = command.to_vec();
    let mut warning = None;
    if let Some((from, to)) = script_extender_swap(game_id) {
        for a in swapped.iter_mut() {
            if a.contains(from) {
                let candidate = a.replace(from, to);
                if Path::new(&candidate).is_file() {
                    *a = candidate;
                } else {
                    warning = Some(format!(
                        "{to} is not installed - launching the vanilla launcher (script-extender mods will not load)."
                    ));
                }
            }
        }
    }
    let mut cmd = std::process::Command::new(find_eidos_binary());
    cmd.arg("play").arg(game_id).arg("--").args(&swapped);
    (cmd, warning)
}

/// Spawn a launch and start tracking it: the child's stdout+stderr go to a
/// per-run log under the instance (the GUI has no terminal when started from
/// Steam), a detached thread `wait()`s it (reaping it, so no zombie) and records
/// its exit status, and the poll subscription refreshes on exit. When `lock_gui`
/// is set the lock overlay also comes up; otherwise the run is tracked without
/// blocking the window.
fn start_run(app: &mut App, title: String, mut cmd: std::process::Command) {
    use std::sync::atomic::Ordering;
    let log = app.created.as_ref().and_then(|inst| {
        let dir = inst.root.join("logs");
        std::fs::create_dir_all(&dir).ok()?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(dir.join(format!("run-{stamp}.log")))
    });
    if let Some(p) = &log {
        if let Ok(f) = std::fs::File::create(p) {
            if let Ok(out) = f.try_clone() {
                cmd.stdout(std::process::Stdio::from(out));
            }
            cmd.stderr(std::process::Stdio::from(f));
        }
    }
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            app.status = Some(format!("Launch failed: {e}"));
            return;
        }
    };
    let pid = child.id();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let outcome = std::sync::Arc::new(std::sync::Mutex::new(None));
    let (signal, slot) = (done.clone(), outcome.clone());
    std::thread::spawn(move || {
        let mut child = child;
        let status = child.wait().ok();
        if let Ok(mut s) = slot.lock() {
            *s = status;
        }
        signal.store(true, Ordering::SeqCst);
    });
    let lock = app.prefs.lock_gui;
    app.running = Some(RunningState { title: title.clone(), pid, done, outcome, log, lock });
    app.status = Some(if lock {
        format!("Running {title} - Eidos is locked until it exits (or click Unlock).")
    } else {
        format!("Running {title}...")
    });
}

/// Clear the run lock and refresh the way MO2's `afterRun` does: the game may have
/// rewritten plugins.txt / loadorder.txt while playing, so re-read the mod list,
/// load order and conflicts. Called from the exit poll once the child exits.
/// A non-zero exit is reported with the run log's path so failures are diagnosable.
fn finish_run(app: &mut App) {
    let run = app.running.take();
    if let Some(inst) = &app.created {
        app.mods = inst.modlist();
        // The session wrote into the Overwrite (and tools may have edited mods).
        drop_files_cache(app, None);
        invalidate_plugins(app);
        app.conflicts = compute_conflicts(app);
        app.meta_cache = build_meta_cache(app);
        recompute_counts(app);
        app.selected_mods.clear();
        app.drag_state = None;
    }
    if app.created.is_some() {
        // The session may have written new saves; the Saves tab must not go stale
        // exactly when they appear.
        load_saves(app);
    }
    // A rebuild may have wiped the launch capability while we played; re-check so
    // the warning banner is current for the next run.
    app.cap_missing = !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary());
    let Some(run) = run else {
        app.status = Some("Application exited. Refreshed plugins and load order.".to_string());
        return;
    };
    let failed = run
        .outcome
        .lock()
        .ok()
        .and_then(|s| *s)
        .map(|st| !st.success())
        .unwrap_or(false);
    app.status = Some(if failed {
        match &run.log {
            Some(p) => format!(
                "{} exited with an error - see the log: {}",
                run.title,
                p.display()
            ),
            None => format!("{} exited with an error.", run.title),
        }
    } else {
        format!("{} exited. Refreshed plugins and load order.", run.title)
    });
}

fn selected_game(app: &App) -> Option<&DetectedGame> {
    app.selected.and_then(|i| app.games.get(i))
}

fn planned_instance(app: &App) -> Option<Instance> {
    let game = selected_game(app)?;
    Some(match app.kind {
        InstanceKind::Global => Instance::global(game.def.id),
        InstanceKind::Portable => {
            let root = if app.portable_path.trim().is_empty() {
                home().join("Eidos").join(game.def.id)
            } else {
                PathBuf::from(app.portable_path.trim())
            };
            Instance::portable(root)
        }
    })
}

/// The rows a row-targeted action should act on: the whole multi-selection when
/// the clicked row belongs to it, otherwise just that row. Separators are never
/// moved by these actions - they define the groups.
fn selection_or(app: &App, row: usize) -> Vec<usize> {
    let mut v: Vec<usize> = if app.selected_mods.contains(&row) && app.selected_mods.len() > 1 {
        app.selected_mods.iter().copied().collect()
    } else {
        vec![row]
    };
    v.retain(|&i| app.mods.get(i).is_some_and(|m| !m.is_separator()));
    v.sort_unstable();
    v
}

/// Move `targets` (indices into `mods`) so the block lands at `dest`, preserving
/// their relative order. Returns the destination index of the first moved row.
///
/// Removing the sources shifts everything after them down, so a downward move has
/// to compensate; getting this wrong is the classic off-by-one that lands a
/// dragged mod one slot short. Every reorder - drag-drop, send to top/bottom, and
/// the targeted sends - goes through here so the correction exists in one place.
fn move_block(mods: &mut Vec<ModEntry>, targets: &[usize], dest: usize) -> usize {
    let mut idx: Vec<usize> = targets.iter().copied().filter(|&i| i < mods.len()).collect();
    idx.sort_unstable();
    idx.dedup();
    if idx.is_empty() {
        return dest.min(mods.len());
    }
    // How many of the moved rows sat before the destination: the block lands that
    // much earlier once they are lifted out.
    let before = idx.iter().filter(|&&i| i < dest).count();
    let block: Vec<ModEntry> = idx.iter().rev().map(|&i| mods.remove(i)).collect();
    let at = dest.saturating_sub(before).min(mods.len());
    // `block` came out highest-index-first, so re-insert in reverse to restore order.
    for m in block {
        mods.insert(at, m);
    }
    at
}

/// Persist the mod list, surfacing a failure instead of losing it silently (a
/// full disk or permission problem would otherwise revert the user's changes on
/// the next restart with no warning). Returns the error text, if any.
fn save_mods(app: &App) -> Option<String> {
    let inst = app.created.as_ref()?;
    inst.save_modlist(&app.mods).err().map(|e| format!("Could not save the mod list: {e}"))
}

/// Invalidate every memoised view listing. Cheap: the listings rebuild lazily on
/// the next redraw that needs them. The stored entries are dropped rather than
/// left to accumulate one stale copy per directory ever viewed.
fn bump_views(app: &App) {
    app.view_generation.set(app.view_generation.get().wrapping_add(1));
    app.data_listing.borrow_mut().take();
    app.listing_cache.borrow_mut().clear();
}

/// Drop cached per-layer file walks: one layer by name (a mod whose contents
/// just changed), or every layer (`None`) when anything might have moved. Also
/// invalidates the memoised view listings, which derive from the same trees.
fn drop_files_cache(app: &App, layer: Option<&str>) {
    let mut cache = app.files_cache.borrow_mut();
    match layer {
        Some(name) => {
            cache.remove(name);
        }
        None => cache.clear(),
    }
    drop(cache);
    bump_views(app);
}

/// Drop the plugin-order cache - and, when the Plugins tab is open, recompute it
/// immediately so the pane updates in place instead of blanking to the
/// placeholder until the user leaves and re-enters the tab.
fn invalidate_plugins(app: &mut App) {
    app.plugins = None;
    if app.tab == Tab::Plugins && app.created.is_some() {
        app.plugins = compute_plugins(app);
    }
}

/// Persist the mod list and invalidate everything derived from it (plugin order,
/// conflict emblems, the per-mod metadata cache).
fn mods_changed(app: &mut App) {
    if let Some(err) = save_mods(app) {
        app.status = Some(err);
    }
    // The merged view depends on which mods are enabled and in what order, not
    // just on their contents.
    bump_views(app);
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    app.meta_cache = build_meta_cache(app);
    recompute_counts(app);
}

/// Make `name` the active profile and reload all per-profile view state (mod list,
/// plugin/conflict caches, collapsed groups, saves), clearing any transient
/// selection / menu / drag. Shared by the profile switch, copy, rename, and delete
/// flows so they can never drift apart.
fn switch_to_profile(app: &mut App, name: &str) {
    if let Some(inst) = &app.created {
        let _ = inst.set_active_profile(name);
        app.mods = inst.profile(name).modlist();
    }
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    app.meta_cache = build_meta_cache(app);
    app.collapsed = load_collapsed(app);
    recompute_counts(app);
    app.selected_mod = None;
    app.selected_mods.clear();
    app.drag_state = None;
    app.menu_mod = None;
    // Saves are per-profile; drop the cache so the Saves tab reloads.
    app.saves = Vec::new();
    app.confirm_delete_save = None;
}

/// Recompute the profile-row Endorsed / Updated counts (MO2 surfaces these). Only
/// real, enabled mods count; separators and disabled mods do not.
fn recompute_counts(app: &mut App) {
    let mut endorsed = 0usize;
    let mut updated = 0usize;
    if let Some(inst) = &app.created {
        for m in &app.mods {
            if !m.enabled || m.is_separator() {
                continue;
            }
            let meta = inst.mod_meta(&m.name);
            if meta.endorsed() {
                endorsed += 1;
            }
            if meta.update_available() {
                updated += 1;
            }
        }
    }
    app.endorsed_count = endorsed;
    app.updated_count = updated;
}

/// The active profile's collapsed-separators file (MO2 keeps this per-profile, out
/// of `modlist.txt`/`meta.ini` so the load order stays clean).
fn collapsed_path(app: &App) -> Option<PathBuf> {
    app.created.as_ref().map(|inst| inst.active().dir().join("collapsed_separators.txt"))
}

/// Load the collapsed-separator set for the active profile.
fn load_collapsed(app: &App) -> HashSet<String> {
    collapsed_path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

/// Persist the collapsed-separator set (one display name per line).
fn save_collapsed(app: &App) {
    if let Some(p) = collapsed_path(app) {
        let body: String = app.collapsed.iter().map(|n| format!("{n}\n")).collect();
        let _ = fs::write(p, body);
    }
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    // Any action other than a second Clear click cancels the clear confirmation.
    if !matches!(message, Message::ClearOverwrite) {
        app.confirm_clear = false;
    }
    // A pending save/download deletion is armed by the first Delete click; any
    // other action (including arming a different row) cancels the previous one.
    if !matches!(message, Message::DeleteSave(_) | Message::ConfirmDeleteSave(_)) {
        app.confirm_delete_save = None;
    }
    if !matches!(message, Message::DeleteDownload(_) | Message::ConfirmDeleteDownload(_)) {
        app.confirm_delete_download = None;
    }
    // The batch-remove confirmation is armed by the first click; any other action
    // (including merely re-rendering on a modifier change) cancels it.
    if !matches!(message, Message::BatchRemoveMods | Message::ConfirmBatchRemove) {
        app.confirm_batch_remove = false;
    }
    match message {
        Message::Next => {
            app.screen = match app.screen {
                Screen::Welcome => Screen::Kind,
                Screen::Kind => Screen::Game,
                Screen::Game => Screen::NameLoc,
                Screen::NameLoc => Screen::Summary,
                other => other,
            };
        }
        Message::Back => {
            app.screen = match app.screen {
                Screen::Kind => Screen::Welcome,
                Screen::Game => Screen::Kind,
                Screen::NameLoc => Screen::Game,
                Screen::Summary => Screen::NameLoc,
                other => other,
            };
        }
        Message::PickKind(k) => app.kind = k,
        Message::PickGame(i) => {
            app.selected = Some(i);
            if app.name.trim().is_empty() {
                if let Some(g) = app.games.get(i) {
                    app.name = g.def.name.to_string();
                }
            }
        }
        Message::NameChanged(s) => app.name = s,
        Message::PortableChanged(s) => app.portable_path = s,
        Message::Finish => {
            if let Some(inst) = planned_instance(app) {
                let game_id = selected_game(app).map(|g| g.def.id.to_string());
                let kind = app.kind;
                match inst.create() {
                    Ok(()) => {
                        if let Some(id) = &game_id {
                            let _ = inst.ensure_manifest(id, kind);
                        }
                        app.mods = inst.modlist();
                        app.created = Some(inst);
                        app.tab = Tab::Data;
                        app.error = None;
                        app.screen = Screen::Main;
                        load_tools(app);
                        app.conflicts = compute_conflicts(app);
                        app.meta_cache = build_meta_cache(app);
                        // Everything cached from a previously-open instance is
                        // stale for this one: plugin order, saves, downloads,
                        // selection and counts all belong to the old instance.
                        app.plugins = None;
                        app.saves = Vec::new();
                        app.confirm_delete_save = None;
                        app.downloads = Vec::new();
                        app.confirm_delete_download = None;
                        app.selected_mod = None;
                        app.selected_mods.clear();
                        app.drag_state = None;
                        app.menu_mod = None;
                        app.collapsed = load_collapsed(app);
                        recompute_counts(app);
                    }
                    Err(e) => app.error = Some(e.to_string()),
                }
            }
        }
        Message::Restart => {
            app.selected = None;
            app.name.clear();
            app.portable_path.clear();
            app.created = None;
            app.error = None;
            app.mods.clear();
            app.status = None;
            app.kind = InstanceKind::Global;
            app.fomod = None;
            app.screen = Screen::Welcome;
        }
        Message::ToggleMod(i) => {
            // A separator is a group divider, not content - it has no toggle (MO2's
            // canBeEnabled() == false).
            if app.mods.get(i).is_some_and(|m| m.is_separator()) {
                return Task::none();
            }
            if let Some(m) = app.mods.get_mut(i) {
                m.enabled = !m.enabled;
            }
            mods_changed(app);
        }
        Message::MoveUp(i) => {
            if i > 0 && i < app.mods.len() {
                app.mods.swap(i - 1, i);
                if app.selected_mod == Some(i) {
                    app.selected_mod = Some(i - 1);
                }
                swap_in_selection(app, i - 1, i);
                mods_changed(app);
            }
        }
        Message::MoveDown(i) => {
            if i + 1 < app.mods.len() {
                app.mods.swap(i, i + 1);
                if app.selected_mod == Some(i) {
                    app.selected_mod = Some(i + 1);
                }
                swap_in_selection(app, i, i + 1);
                mods_changed(app);
            }
        }
        Message::SelectTab(t) => {
            app.tab = t;
            if t == Tab::Plugins && app.plugins.is_none() {
                app.plugins = compute_plugins(app);
            }
            if t == Tab::Conflicts && app.conflicts.is_none() {
                app.conflicts = compute_conflicts(app);
            }
            // Lazily fill the Saves / Downloads caches the first time each tab opens.
            if t == Tab::Saves && app.saves.is_empty() {
                load_saves(app);
            }
            if t == Tab::Downloads && app.downloads.is_empty() {
                load_downloads(app);
            }
        }
        Message::SwitchProfile(name) => {
            // One shared path (switch_to_profile) so the reload steps - incl.
            // recompute_counts, which this handler used to skip - never drift.
            if app.created.is_some() {
                switch_to_profile(app, &name);
                app.status = Some(format!("Switched to profile '{name}'."));
            }
        }
        Message::NewProfile => {
            let created = app.created.as_ref().map(|inst| {
                let existing = inst.profiles();
                let mut n = existing.len() + 1;
                let mut name = format!("Profile {n}");
                while existing.contains(&name) {
                    n += 1;
                    name = format!("Profile {n}");
                }
                let src = inst.active();
                let ok = inst.profile(&name).create_from(&src).is_ok();
                (name, src.name, ok)
            });
            if let Some((name, src_name, true)) = created {
                switch_to_profile(app, &name);
                app.status = Some(format!("Created '{name}' (copy of '{src_name}')."));
            }
        }
        // ---- profile management (rename / delete / named copy) --------------
        Message::ProfileMenuOpen(name) => {
            app.profile_menu = Some(name);
            app.profile_rename = None;
            app.profile_copy = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileCloseMenu => {
            app.profile_menu = None;
            app.profile_rename = None;
            app.profile_copy = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileRenameStart(name) => {
            app.profile_rename = Some((name.clone(), name));
            app.profile_copy = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileRenameChanged(s) => {
            if let Some((_, edited)) = &mut app.profile_rename {
                *edited = s;
            }
        }
        Message::ProfileRenameCommit => {
            if let (Some(inst), Some((old, edited))) = (&app.created, app.profile_rename.clone()) {
                let new = edited.trim().to_string();
                if new.is_empty() || new.contains('/') || new.contains('\\') {
                    app.status = Some("Invalid profile name.".to_string());
                } else if new == old {
                    // no-op: just close the editor
                    app.profile_rename = None;
                    app.profile_menu = None;
                } else {
                    let was_active = inst.active_profile() == old;
                    match inst.rename_profile(&old, &new) {
                        Ok(()) => {
                            app.profile_rename = None;
                            app.profile_menu = None;
                            // rename_profile already followed the active pointer; reload
                            // the view when the renamed profile was the active one.
                            if was_active {
                                switch_to_profile(app, &new);
                            }
                            app.status = Some(format!("Renamed profile to '{new}'."));
                        }
                        // Keep the editor open on a collision so the user can retype.
                        Err(e) => app.status = Some(format!("Rename failed: {e}")),
                    }
                }
            }
        }
        Message::ProfileCopyStart(name) => {
            // Prefill a free "<name> Copy" target so the editor never collides at once.
            let suggested = app
                .created
                .as_ref()
                .map(|inst| suggest_free_profile_name(inst, &format!("{name} Copy")))
                .unwrap_or_else(|| format!("{name} Copy"));
            app.profile_copy = Some((name, suggested));
            app.profile_rename = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileCopyChanged(s) => {
            if let Some((_, edited)) = &mut app.profile_copy {
                *edited = s;
            }
        }
        Message::ProfileCopyCommit => {
            if let (Some(inst), Some((src_name, edited))) = (&app.created, app.profile_copy.clone()) {
                let new = edited.trim().to_string();
                if new.is_empty() || new.contains('/') || new.contains('\\') {
                    app.status = Some("Invalid profile name.".to_string());
                } else if inst.profile(&new).dir().exists() {
                    app.status = Some(format!("Profile '{new}' already exists."));
                } else {
                    let src = inst.profile(&src_name);
                    let dest = inst.profile(&new);
                    match dest.create_from(&src) {
                        Ok(()) => {
                            app.profile_copy = None;
                            app.profile_menu = None;
                            switch_to_profile(app, &new);
                            app.status = Some(format!("Created '{new}' (copy of '{src_name}')."));
                        }
                        Err(e) => app.status = Some(format!("Copy failed: {e}")),
                    }
                }
            }
        }
        Message::ProfileDeleteConfirm(name) => {
            // First click arms; clicking the same profile again commits.
            app.profile_delete_confirm = Some(name);
            app.profile_rename = None;
            app.profile_copy = None;
        }
        Message::ProfileDeleteCommit(name) => {
            app.profile_delete_confirm = None;
            if let Some(inst) = &app.created {
                match inst.delete_profile(&name) {
                    Ok(()) => {
                        app.profile_menu = None;
                        app.status = Some(format!("Deleted profile '{name}'."));
                    }
                    // Backend guards the active / last profile; surface its reason.
                    Err(e) => app.status = Some(format!("Delete failed: {e}")),
                }
            }
        }
        Message::InstallMod => {
            // Open a native file picker off-thread; the result comes back as ModPicked.
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("Mod archives", &["7z", "zip", "rar"])
                    .set_title("Select a mod archive to install")
                    .pick_file(),
                |handle| Message::ModPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ModPicked(picked) => {
            let Some(path) = picked else { return Task::none() };
            let game_id = selected_game(app).map(|g| g.def.id.to_string());
            let mods_dir = app.created.as_ref().map(|i| i.mods_dir());
            let (Some(gid), Some(mods_dir)) = (game_id, mods_dir) else {
                return Task::none();
            };
            let name = eidos_install::mod_name_for(&path);
            // One extraction, then classify: a plain archive installs straight from
            // the extracted tree instead of being unpacked a second time.
            match eidos_install::open_archive(&path, &mods_dir, &name) {
                Ok(eidos_install::Opened::Fomod(session)) => {
                    let enabled_roots: Vec<std::path::PathBuf> =
                        app.mods.iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
                    let disabled_roots: Vec<std::path::PathBuf> =
                        app.mods.iter().filter(|m| !m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
                    let ctx = match selected_game(app) {
                        Some(g) => eidos_install::fomod_context(&g.data_path, &enabled_roots, &disabled_roots),
                        None => eidos_fomod::Context::default(),
                    };
                    let session = *session;
                    // MO2 refuses a FOMOD whose <moduleDependencies> are unmet before
                    // showing the wizard - tell the user what is missing and stop.
                    if let Some(req) = session.unmet_dependencies(&ctx) {
                        app.status = Some(format!("Cannot install: this mod requires {req}."));
                    } else {
                        let selection = eidos_fomod::default_selection(&session.config, &ctx);
                        app.fomod =
                            Some(FomodWizard { session, step: 0, selection, game_id: gid, archive: path, ctx });
                        app.status = Some("FOMOD installer: choose your options, then Install.".to_string());
                    }
                }
                Ok(eidos_install::Opened::Simple(tree)) => {
                    let ctx = eidos_fomod::Context::default();
                    match eidos_install::install_extracted(
                        &tree,
                        &path,
                        &mods_dir,
                        &name,
                        &gid,
                        eidos_install::OverwritePolicy::Fail,
                        &ctx,
                    ) {
                        Ok(r) => after_install(app, &r.name, r.dest, r.fomod, Some(&path)),
                        Err(eidos_install::InstallError::Exists(_)) => {
                            // MO2's QueryOverwriteDialog: let the user Merge/Replace/
                            // Rename. The extracted tree rides along so resolving it
                            // needs no re-extract.
                            let rename_to = suggest_free_name(&mods_dir, &name);
                            app.collision = Some(CollisionPrompt {
                                archive: path,
                                name: name.clone(),
                                game_id: gid,
                                rename_to,
                                fomod: false,
                                tree: Some(tree),
                            });
                            app.status = Some(format!("'{name}' already exists - choose how to install."));
                        }
                        Err(e) => app.status = Some(format!("Install failed: {e}")),
                    }
                }
                Err(e) => app.status = Some(format!("Install failed: {e}")),
            }
        }
        Message::FomodToggle(gi, pi) => {
            if let Some(w) = &mut app.fomod {
                let si = w.step;
                let gtype =
                    w.session.config.steps.get(si).and_then(|s| s.groups.get(gi)).map(|g| g.group_type);
                if let (Some(gtype), Some(g)) =
                    (gtype, w.selection.get_mut(si).and_then(|s| s.get_mut(gi)))
                {
                    use eidos_fomod::GroupType::*;
                    match gtype {
                        SelectAll => {}
                        SelectExactlyOne => {
                            g.iter_mut().for_each(|x| *x = false);
                            if let Some(s) = g.get_mut(pi) {
                                *s = true;
                            }
                        }
                        SelectAtMostOne => {
                            let was = g.get(pi).copied().unwrap_or(false);
                            g.iter_mut().for_each(|x| *x = false);
                            if let Some(s) = g.get_mut(pi) {
                                *s = !was;
                            }
                        }
                        _ => {
                            if let Some(s) = g.get_mut(pi) {
                                *s = !*s;
                            }
                        }
                    }
                }
            }
        }
        Message::FomodNext => {
            if let Some(w) = &mut app.fomod {
                let vis = eidos_fomod::visible_steps(
                    &w.session.config,
                    &w.selection,
                    &w.ctx,
                );
                let mut s = w.step + 1;
                while s < vis.len() && !vis[s] {
                    s += 1;
                }
                if s < vis.len() {
                    w.step = s;
                }
            }
        }
        Message::FomodBack => {
            if let Some(w) = &mut app.fomod {
                let vis = eidos_fomod::visible_steps(
                    &w.session.config,
                    &w.selection,
                    &w.ctx,
                );
                let mut s = w.step;
                while s > 0 {
                    s -= 1;
                    if vis.get(s).copied().unwrap_or(true) {
                        w.step = s;
                        break;
                    }
                }
            }
        }
        Message::FomodInstall => {
            let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) else {
                return Task::none();
            };
            // Collision check BEFORE consuming the wizard: a reinstall must offer
            // Merge/Replace/Rename (MO2's QueryOverwriteDialog) with the user's
            // choices intact, not dead-end and discard them.
            if let Some(w) = app.fomod.as_ref() {
                if let Some(name) = eidos_install::collision_name(&mods_dir, w.session.mod_name()) {
                    let rename_to = suggest_free_name(&mods_dir, &name);
                    app.collision = Some(CollisionPrompt {
                        archive: w.archive.clone(),
                        name: name.clone(),
                        game_id: w.game_id.clone(),
                        rename_to,
                        fomod: true,
                        // The wizard (still open) owns the extracted tree.
                        tree: None,
                    });
                    app.status = Some(format!("'{name}' already exists - choose how to install."));
                    return Task::none();
                }
            }
            if let Some(w) = app.fomod.take() {
                let archive = w.archive.clone();
                match eidos_install::finish_fomod(
                    w.session,
                    &w.selection,
                    &mods_dir,
                    &w.game_id,
                    &w.ctx,
                    eidos_install::OverwritePolicy::Fail,
                ) {
                    Ok(r) => after_install(app, &r.name, r.dest, true, Some(&archive)),
                    Err(e) => app.status = Some(format!("Install failed: {e}")),
                }
            }
        }
        Message::FomodCancel => {
            app.fomod = None;
            app.status = Some("FOMOD install cancelled.".to_string());
        }
        Message::ToolPicked(choice) => {
            app.tool_choice = (choice != RUN_GAME).then_some(choice);
        }
        Message::Run => {
            if app.running.is_some() {
                // Already waiting on a launched application (MO2 won't launch a
                // second one while locked); ignore the repeat Run.
                app.status = Some("An application is already running. Unlock first to launch another.".to_string());
            } else if let Some(title) = app.tool_choice.clone() {
                // A tool: the CLI resolves Proton itself, no Steam command needed.
                // `id` is Copy, so the immutable `game` borrow ends before `start_run`.
                if let Some(id) = selected_game(app).map(|g| g.def.id) {
                    let cmd = tool_command(id, &title);
                    start_run(app, title, cmd);
                } else {
                    app.status = Some("Create or open an instance first.".to_string());
                }
            } else if app.launch_command.is_empty() {
                // Standalone: we don't have Steam's Proton command, so we cannot
                // build the launch environment. Point the user at the option, with
                // this binary's absolute path (Steam's launch options don't see
                // ~/.cargo/bin on PATH) and native d3dcompiler forced so the game's
                // shader compilation works under Proton. Eidos merges that with any
                // mod-shipped DLL overrides at launch.
                let exe = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.to_str().map(str::to_string))
                    .unwrap_or_else(|| "eidos-gui".to_string());
                app.status = Some(format!(
                    "Set the game's Steam launch option to:  WINEDLLOVERRIDES=\"d3dcompiler_47=n\" {exe} %command%  then press Play in Steam (Eidos opens, then click Run)."
                ));
            } else if let (Some(game), Some(inst)) = (selected_game(app), &app.created) {
                let id = game.def.id;
                let game_name = game.def.name.to_string();
                // Soft advisory if an ENB (game root) and Community Shaders (an
                // enabled mod) are both active - prepended to the launch status,
                // never blocking. The CLI emits the same note to stderr, which the
                // GUI does not surface, so we recompute it here.
                let cs_roots: Vec<std::path::PathBuf> = inst
                    .modlist()
                    .into_iter()
                    .filter(|m| m.enabled && !m.is_separator())
                    .map(|m| m.path)
                    .collect();
                let both_active = eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots);
                // `game`/`inst` are no longer used below; their borrows end here so
                // `start_run` can take `&mut app`.
                let (cmd, se_warning) = play_command(id, &app.launch_command);
                start_run(app, game_name, cmd);
                // Prepend advisories to whatever status start_run set.
                for note in [se_warning, both_active.then(|| {
                    "Note: ENB and Community Shaders are both active (if visuals look wrong, disable one in its INI).".to_string()
                })].into_iter().flatten() {
                    if let Some(s) = app.status.take() {
                        app.status = Some(format!("{note} {s}"));
                    }
                }
            } else {
                app.status = Some("Create or open an instance first.".to_string());
            }
        }
        Message::PollRunning => {
            // The poll subscription fires while a launch is being waited on; once
            // the wait thread reports the child exited, unlock and refresh.
            let exited = app
                .running
                .as_ref()
                .map(|r| r.done.load(std::sync::atomic::Ordering::SeqCst))
                .unwrap_or(false);
            if exited {
                finish_run(app);
            }
        }
        Message::ForceUnlock => {
            // Drop the overlay but KEEP tracking (MO2 stops waiting entirely; we
            // keep the exit poll so the afterRun refresh still happens and the
            // game's own plugins.txt rewrite is never clobbered by stale GUI state).
            // The game is never killed.
            if let Some(r) = app.running.as_mut() {
                r.lock = false;
                let title = r.title.clone();
                app.status = Some(format!("Unlocked - {title} is still running."));
            }
        }
        Message::CloseLootReport => {
            app.loot_report = None;
        }
        Message::SendToFirstConflict(i) | Message::SendToLastConflict(i) => {
            let first = matches!(message, Message::SendToFirstConflict(_));
            app.menu_mod = None;
            let targets = selection_or(app, i);
            // The conflict sets are already computed for the emblems; reuse them.
            // Origins are `index + 1`, so BASE_ORIGIN (0, the game data) and the
            // Overwrite pseudo-layer (u32::MAX) are not rows and must be dropped.
            let mut related: Vec<usize> = Vec::new();
            if let Some(map) = app.conflicts.as_ref() {
                for &t in &targets {
                    let origin = (t + 1) as u32;
                    if let Some(mc) = map.mods.get(&origin) {
                        let set = if first { &mc.overwrites } else { &mc.overwritten_by };
                        related.extend(
                            set.iter()
                                .filter(|&&o| o != 0 && o != u32::MAX)
                                .map(|&o| (o - 1) as usize),
                        );
                    }
                }
            }
            let dest = if first { related.iter().min() } else { related.iter().max() };
            let Some(&dest) = dest else {
                app.status = Some(
                    if first { "This mod overrides nothing." } else { "Nothing overrides this mod." }
                        .to_string(),
                );
                return Task::none();
            };
            // "Just below the last mod that overrides it" is one slot past it.
            let dest = if first { dest } else { (dest + 1).min(app.mods.len()) };
            let at = move_block(&mut app.mods, &targets, dest);
            app.selected_mod = Some(at);
            app.selected_mods.clear();
            mods_changed(app);
        }
        Message::SendToPriorityStart(i) => {
            app.menu_mod = None;
            app.send_separator = None;
            app.send_priority = Some((i, i.to_string()));
        }
        Message::SendToPriorityChanged(text) => {
            if let Some((_, t)) = app.send_priority.as_mut() {
                *t = text;
            }
        }
        Message::SendToPriorityCommit => {
            let Some((i, text)) = app.send_priority.take() else { return Task::none() };
            let Ok(dest) = text.trim().parse::<usize>() else {
                app.status = Some("Enter a priority number.".to_string());
                return Task::none();
            };
            let targets = selection_or(app, i);
            let dest = dest.min(app.mods.len());
            let at = move_block(&mut app.mods, &targets, dest);
            app.selected_mod = Some(at);
            app.selected_mods.clear();
            mods_changed(app);
            app.status = Some(format!("Moved to priority {at}."));
        }
        Message::SendToSeparatorStart(i) => {
            app.menu_mod = None;
            app.send_priority = None;
            app.send_separator = Some(i);
        }
        Message::SendToSeparatorPick(sep) => {
            let Some(i) = app.send_separator.take() else { return Task::none() };
            let targets = selection_or(app, i);
            // Land in the chosen separator's GROUP: the slot just before the next
            // separator, or the end of the list when it is the last group.
            let dest = app
                .mods
                .iter()
                .enumerate()
                .skip(sep + 1)
                .find(|(_, m)| m.is_separator())
                .map(|(idx, _)| idx)
                .unwrap_or(app.mods.len());
            let at = move_block(&mut app.mods, &targets, dest);
            app.selected_mod = Some(at);
            app.selected_mods.clear();
            mods_changed(app);
        }
        Message::SendToTargetCancel => {
            app.send_priority = None;
            app.send_separator = None;
        }
        Message::ClearStatus => {
            app.status = None;
        }
        Message::OverwriteToModStart => {
            if app.created.as_ref().is_some_and(|i| i.overwrite_is_empty()) {
                app.status = Some("The Overwrite is empty - nothing to turn into a mod.".to_string());
            } else {
                // Default to a fresh, non-colliding name, like the installer does.
                let suggestion = app
                    .created
                    .as_ref()
                    .map(|i| suggest_free_name(&i.mods_dir(), "Overwrite output"))
                    .unwrap_or_else(|| "Overwrite output".to_string());
                app.overwrite_to_mod = Some(suggestion);
            }
        }
        Message::OverwriteToModName(s) => {
            if app.overwrite_to_mod.is_some() {
                app.overwrite_to_mod = Some(s);
            }
        }
        Message::OverwriteToModCancel => {
            app.overwrite_to_mod = None;
        }
        Message::OpenUrl(url) => {
            // Only ever hand a real web link to the browser.
            if url.starts_with("https://") || url.starts_with("http://") {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                app.status = Some(format!("Opened {url}"));
            }
        }
        Message::PluginMoveUp(i) | Message::PluginMoveDown(i) => {
            let up = matches!(message, Message::PluginMoveUp(_));
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            let mut moved = false;
            if let Some(list) = app.plugins.as_mut() {
                moved = list.move_plugin(i, up);
                if moved {
                    // refresh() re-applies masters-before-dependents, so an illegal
                    // move is corrected rather than written out.
                    list.refresh(&spec);
                }
            }
            if !moved {
                return Task::none();
            }
            let written =
                app.plugins.as_ref().map(|list| write_plugin_state(app, list, &spec)).transpose();
            if let Err(e) = written {
                app.status = Some(format!("Could not write the load order: {e}"));
            }
        }
        Message::ImportMo2Pick => {
            if app.created.is_none() {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            }
            app.profile_menu = None;
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title("Select the MO2 profile folder (the one holding modlist.txt)")
                    .pick_folder(),
                |h| Message::ImportMo2Picked(h.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ImportMo2Picked(picked) => {
            let Some(dir) = picked else { return Task::none() };
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            match inst.import_mo2_profile(&dir) {
                Ok(r) => {
                    app.mods = inst.modlist();
                    drop_files_cache(app, None);
                    invalidate_plugins(app);
                    app.conflicts = compute_conflicts(app);
                    app.meta_cache = build_meta_cache(app);
                    recompute_counts(app);
                    app.selected_mod = None;
                    app.selected_mods.clear();
                    let mut s = format!("Imported {} mod(s) from MO2.", r.matched);
                    if r.plugin_files > 0 {
                        s.push_str(" Load order imported.");
                    }
                    if !r.missing.is_empty() {
                        s.push_str(&format!(
                            " {} mod(s) MO2 listed are not installed here (install them, then import again).",
                            r.missing.len()
                        ));
                    }
                    app.status = Some(s);
                }
                Err(e) => app.status = Some(format!("MO2 import failed: {e}")),
            }
        }
        Message::OverwriteToModCommit => {
            let Some(name) = app.overwrite_to_mod.take().map(|s| s.trim().to_string()) else {
                return Task::none();
            };
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let existing = inst.mods_dir().join(&name).exists();
            match inst.overwrite_into_mod(&name) {
                Ok(dest) => {
                    // Highest priority (the end of the display order), which is where
                    // the Overwrite's content effectively sat.
                    if !app.mods.iter().any(|m| m.name == name) {
                        app.mods.push(ModEntry { name: name.clone(), enabled: true, path: dest });
                    }
                    drop_files_cache(app, None);
                    mods_changed(app);
                    app.status = Some(if existing {
                        format!("Moved the Overwrite into '{name}'.")
                    } else {
                        format!("Created mod '{name}' from the Overwrite.")
                    });
                }
                Err(e) => {
                    app.status = Some(format!("Could not create the mod: {e}"));
                    // Keep the prompt open so the name can be fixed.
                    app.overwrite_to_mod = Some(name);
                }
            }
        }
        Message::Refresh => {
            if let Some(inst) = &app.created {
                app.mods = inst.modlist();
                // F5 = full re-scan: every cached file walk may be stale.
                drop_files_cache(app, None);
                invalidate_plugins(app);
                app.conflicts = compute_conflicts(app);
                app.meta_cache = build_meta_cache(app);
                recompute_counts(app);
                // The list was rebuilt; selection / drag indices no longer hold.
                app.selected_mods.clear();
                app.drag_state = None;
                app.status = Some("Refreshed mod list.".to_string());
            }
            load_tools(app);
            // F5 is also the "I just ran setcap" recheck for the warning banner.
            app.cap_missing = !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary());
        }
        Message::OpenFolder(p) => {
            let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
            app.status = Some(format!("Opened {} in your file manager.", p.display()));
        }
        Message::ClearOverwrite => {
            if let Some(inst) = &app.created {
                let dir = inst.overwrite_dir();
                if app.confirm_clear {
                    app.confirm_clear = false;
                    app.status = Some(match clear_dir_contents(&dir) {
                        Ok(()) => "Overwrite cleared.".to_string(),
                        Err(e) => format!("Clear failed: {e}"),
                    });
                    drop_files_cache(app, Some("Overwrite"));
                    app.conflicts = compute_conflicts(app);
                } else {
                    app.confirm_clear = true;
                    app.status = Some(
                        "Click Clear again to confirm - this permanently deletes everything the game wrote to the Overwrite (configs, new saves, generated files)."
                            .to_string(),
                    );
                }
            }
        }
        Message::SearchChanged(q) => {
            app.search = q;
            // A filter change can hide the menu's target row; keep it simple.
            app.menu_mod = None;
            app.rename = None;
            app.drag_state = None;
        }
        Message::CategoryFilterChanged(id) => {
            app.category_filter = id;
            app.menu_mod = None;
            app.rename = None;
            app.drag_state = None;
        }
        Message::SelectMod(i) => {
            // A held modifier turns a plain click into a multi-select gesture (iced
            // can only fire a fixed `on_press` message, so we branch on the live
            // modifier state captured by the keyboard subscription).
            if app.modifiers.control() || app.modifiers.command() {
                return update(app, Message::SelectModToggle(i));
            }
            if app.modifiers.shift() {
                return update(app, Message::SelectModExtend(i));
            }
            // Plain click: single focus + collapse the multi-selection to just it,
            // and arm a potential drag from this row (committed only if it moves).
            app.selected_mod = Some(i);
            app.selected_mods.clear();
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = Some(DragState { from: i, hover_over: i });
        }
        Message::SelectModToggle(i) => {
            // Ctrl+click: flip this row's membership; the first toggle also seeds the
            // set from the current focus so the anchor row stays selected.
            if app.selected_mods.is_empty() {
                if let Some(f) = app.selected_mod {
                    app.selected_mods.insert(f);
                }
            }
            if !app.selected_mods.remove(&i) {
                app.selected_mods.insert(i);
            }
            app.selected_mod = Some(i);
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
        }
        Message::SelectModExtend(i) => {
            // Shift+click: select the contiguous run from the focus anchor to `i`.
            // With no anchor yet, behaves like a plain single select.
            let anchor = app.selected_mod.unwrap_or(i);
            let (lo, hi) = (anchor.min(i), anchor.max(i));
            app.selected_mods.clear();
            for idx in lo..=hi {
                if idx < app.mods.len() {
                    app.selected_mods.insert(idx);
                }
            }
            app.selected_mod = Some(i);
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
        }
        Message::ClearSelection => {
            app.selected_mods.clear();
            app.drag_state = None;
            app.menu_mod = None;
        }
        Message::OpenModMenu(i) => {
            // Right-clicking a row already in the multi-selection keeps the whole
            // set (MO2 batch context menu); right-clicking outside it selects just
            // that row first.
            if !app.selected_mods.contains(&i) {
                app.selected_mods.clear();
            }
            app.selected_mod = Some(i);
            app.menu_mod = Some(i);
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
        }
        Message::CloseMenu => {
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
        }
        Message::ModSendTop(i) => {
            if i < app.mods.len() {
                let at = move_block(&mut app.mods, &[i], 0);
                app.selected_mod = Some(at);
                // Every other row's index shifted: a stale multi-selection here
                // could feed the wrong rows into a batch remove.
                app.selected_mods.clear();
                mods_changed(app);
            }
            app.menu_mod = None;
        }
        Message::ModSendBottom(i) => {
            if i < app.mods.len() {
                let end = app.mods.len();
                let at = move_block(&mut app.mods, &[i], end);
                app.selected_mod = Some(at);
                app.selected_mods.clear();
                mods_changed(app);
            }
            app.menu_mod = None;
        }
        Message::ModOpenFolder(i) => {
            app.menu_mod = None;
            if let Some(m) = app.mods.get(i) {
                let _ = std::process::Command::new("xdg-open").arg(&m.path).spawn();
                app.status = Some(format!("Opened '{}' in your file manager.", m.name));
            }
        }
        Message::ModVisitNexus(i) => {
            app.menu_mod = None;
            let domain = selected_game(app).map(|g| g.def.nexus_game).filter(|s| !s.is_empty());
            let mod_id = app.mods.get(i).and_then(|m| app.meta_cache.get(&m.name)).and_then(|r| r.mod_id);
            match (domain, mod_id) {
                (Some(domain), Some(id)) => {
                    let url = format!("https://www.nexusmods.com/{domain}/mods/{id}");
                    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                    app.status = Some(format!("Opening {url}"));
                }
                _ => {
                    app.status =
                        Some("No Nexus mod id on record for this mod (install it from Nexus to link it).".to_string());
                }
            }
        }
        Message::ModReinstall(i) => {
            app.menu_mod = None;
            if let Some(m) = app.mods.get(i) {
                app.status = Some(format!(
                    "Reinstalling '{}': pick the archive to install over it.",
                    m.name
                ));
            }
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("Mod archives", &["7z", "zip", "rar"])
                    .set_title("Select the archive to reinstall")
                    .pick_file(),
                |handle| Message::ModPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ModRemove(i) => {
            if app.confirm_remove == Some(i) {
                app.confirm_remove = None;
                app.menu_mod = None;
                if let Some(m) = app.mods.get(i).cloned() {
                    match fs::remove_dir_all(&m.path) {
                        Ok(()) => {
                            app.mods.remove(i);
                            app.selected_mod = None;
                            app.selected_mods.clear();
                            app.drag_state = None;
                            drop_files_cache(app, Some(&m.name));
                            mods_changed(app);
                            app.status = Some(format!("Removed '{}'.", m.name));
                        }
                        Err(e) => app.status = Some(format!("Remove failed: {e}")),
                    }
                }
            } else {
                app.confirm_remove = Some(i);
                if let Some(m) = app.mods.get(i) {
                    app.status =
                        Some(format!("Click Remove again to permanently delete '{}' from disk.", m.name));
                }
            }
        }
        Message::RenameStart(i) => {
            if let Some(m) = app.mods.get(i) {
                // Edit the display name; a separator's `_separator` suffix is stripped
                // for editing and re-applied on commit (MO2 getDisplayName/makeInternalName).
                app.rename = Some((i, m.display_name().to_string()));
                app.menu_mod = Some(i);
                app.confirm_remove = None;
            }
        }
        Message::RenameChanged(s) => {
            if let Some((_, name)) = &mut app.rename {
                *name = s;
            }
        }
        Message::RenameCommit => {
            if let Some((i, typed)) = app.rename.take() {
                app.menu_mod = None;
                let typed = typed.trim().to_string();
                let old = app.mods.get(i).cloned();
                if let Some(old) = old {
                    // A separator keeps its `_separator` suffix on disk + in modlist.txt.
                    let new_name =
                        if old.is_separator() { format!("{typed}_separator") } else { typed.clone() };
                    if typed.is_empty() || typed.contains('/') || typed.contains('\\') {
                        app.status = Some("Invalid name.".to_string());
                    } else if new_name == old.name {
                        // no-op
                    } else if let Some(mods_dir) = app.created.as_ref().map(|inst| inst.mods_dir()) {
                        let dest = mods_dir.join(&new_name);
                        if dest.exists() {
                            app.status = Some(format!("'{typed}' already exists."));
                        } else {
                            match fs::rename(&old.path, &dest) {
                                Ok(()) => {
                                    if let Some(m) = app.mods.get_mut(i) {
                                        m.name = new_name.clone();
                                        m.path = dest;
                                    }
                                    // The cache is keyed by name; the old key is stale.
                                    drop_files_cache(app, Some(&old.name));
                                    mods_changed(app);
                                    app.status = Some(format!("Renamed to '{typed}'."));
                                }
                                Err(e) => app.status = Some(format!("Rename failed: {e}")),
                            }
                        }
                    }
                }
            }
        }
        Message::AddSeparator(i) => {
            app.menu_mod = None;
            let mods_dir = app.created.as_ref().map(|inst| inst.mods_dir());
            if let Some(mods_dir) = mods_dir {
                // A unique "Separator N" display name -> folder "<name>_separator".
                let mut n = 1usize;
                let mut display = "Separator".to_string();
                while mods_dir.join(format!("{display}_separator")).exists() {
                    n += 1;
                    display = format!("Separator {n}");
                }
                let folder = format!("{display}_separator");
                let dest = mods_dir.join(&folder);
                match fs::create_dir_all(&dest) {
                    Ok(()) => {
                        // Minimal meta.ini, mirroring MO2's createMod.
                        let _ = fs::write(dest.join("meta.ini"), "[General]\nmodid=0\nversion=\n");
                        let idx = i.min(app.mods.len());
                        app.mods.insert(idx, ModEntry { name: folder, enabled: true, path: dest });
                        // Indices at/after the insertion point shifted.
                        app.selected_mods.clear();
                        mods_changed(app);
                        app.selected_mod = Some(idx);
                        // Open its rename editor so the user names it straight away.
                        app.rename = Some((idx, display));
                        app.menu_mod = Some(idx);
                    }
                    Err(e) => app.status = Some(format!("Could not create separator: {e}")),
                }
            }
        }
        Message::SetSeparatorColor(i, rgb) => {
            app.menu_mod = None;
            let result = match (app.mods.get(i).cloned(), app.created.as_ref()) {
                (Some(m), Some(inst)) if m.is_separator() => {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_color(rgb);
                    Some((m.display_name().to_string(), meta.write(&inst.meta_path(&m.name))))
                }
                _ => None,
            };
            if let Some((display, r)) = result {
                match r {
                    Ok(()) => {
                        app.meta_cache = build_meta_cache(app); // pick up the new colour
                        app.status = Some(format!("Set the colour for '{display}'."));
                    }
                    Err(e) => app.status = Some(format!("Could not set colour: {e}")),
                }
            }
        }
        Message::ToggleCollapse(name) => {
            if !app.collapsed.remove(&name) {
                app.collapsed.insert(name);
            }
            save_collapsed(app);
        }
        Message::TogglePlugin(i) => {
            // Compute the spec + prefix dir up front (immutable borrows of `app`)
            // before mutating `app.plugins`.
            let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
            let name = app.plugins.as_ref().and_then(|l| l.plugins.get(i)).map(|p| p.name.clone());
            let forced = app.plugins.as_ref().and_then(|l| l.plugins.get(i)).map(|p| p.force_disabled).unwrap_or(false);
            if let (Some(spec), Some(name)) = (spec, name) {
                // Base-game masters are implicit and always loaded; refuse to toggle.
                if spec.primary_plugins.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
                    app.status = Some(format!("{name} is a base-game master and is always loaded."));
                } else if forced {
                    app.status =
                        Some(format!("{name} is a light plugin this game can't load and stays off."));
                } else if app.plugins.is_some() {
                    let mut now = false;
                    if let Some(list) = app.plugins.as_mut() {
                        now = list.plugins.get(i).map(|p| p.enabled).unwrap_or(false);
                        list.set_enabled(&name, !now);
                        list.refresh(&spec);
                    }
                    // Persist to the profile (which owns the order) and the prefix.
                    // Both borrows below are shared, so this is fine after the
                    // mutation above has ended.
                    let written = app
                        .plugins
                        .as_ref()
                        .map(|list| write_plugin_state(app, list, &spec))
                        .transpose();
                    app.status = Some(match written {
                        Ok(_) => format!("{} {name}.", if now { "Disabled" } else { "Enabled" }),
                        Err(e) => format!("Could not write the load order: {e}"),
                    });
                }
            }
        }
        Message::SortPlugins => {
            // Gather everything the (static) async closure needs, cloned out of
            // `app`, then run the masterlist fetch + LOOT sort off the UI thread.
            let Some(game) = selected_game(app) else { return Task::none() };
            let id = game.def.id;
            if !eidos_loot::is_supported(id) {
                app.status = Some(format!("LOOT sorting is not available for {id}."));
                return Task::none();
            }
            let Some(spec) = GameSpec::for_id(id) else { return Task::none() };
            let Some(cd) = game.compatdata.as_ref() else {
                app.status =
                    Some("Launch the game once through Steam first so its prefix exists.".to_string());
                return Task::none();
            };
            let Some(list) = app.plugins.as_ref() else {
                app.status = Some("No plugins computed yet.".to_string());
                return Task::none();
            };
            let id = id.to_string();
            let install = game.install_path.clone();
            let local_dir = plugins_txt_dir(&cd.join("pfx"), &spec);
            let cache = app
                .created
                .as_ref()
                .map(|i| i.root.join("loot"))
                .unwrap_or_else(|| eidos_instance::Instance::global(&id).root.join("loot"));
            let plugins: Vec<(String, PathBuf)> =
                list.plugins.iter().map(|p| (p.name.clone(), p.path.clone())).collect();
            // The enabled (active) plugin names, lowercased - drives which plugins the
            // LOOT report covers and what counts as a missing master.
            let enabled_lower: std::collections::HashSet<String> = list
                .plugins
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.name.to_ascii_lowercase())
                .collect();
            app.status = Some("Sorting plugins with LOOT...".to_string());
            return Task::perform(
                async move {
                    // `is_supported(id)` was checked above and loot_support is a pure
                    // map, so this is always Some here; handle None gracefully anyway
                    // rather than unwrap (robust to any future refactor of the guard).
                    let repo = match eidos_loot::loot_support(&id) {
                        Some((_, repo)) => repo,
                        None => return Err(format!("LOOT sorting is not available for {id}.")),
                    };
                    // Refresh the masterlist on every sort, like MO2/LOOT; a failed
                    // download falls back to the cached copy.
                    let (ml, pre) = eidos_loot::ensure_masterlist(repo, &cache, true)
                        .map_err(|e| e.to_string())?;
                    let userlist = cache.join("userlist.yaml");
                    let order = eidos_loot::sort(&id, &install, &local_dir, &plugins, &ml, &pre, Some(&userlist))
                        .map_err(|e| e.to_string())?;
                    // Build the post-sort report (general messages + per-plugin
                    // missing masters / messages / dirty info) for the modal, the
                    // same way MO2 shows its LOOT dialog after a sort. This is
                    // advisory: a report failure must NOT discard the successful
                    // sort, so it is an inner Result the handler tolerates.
                    let report = eidos_loot::report(
                        &id, &install, &local_dir, &plugins, &enabled_lower, &ml, &pre, Some(&userlist),
                    )
                    .map_err(|e| e.to_string());
                    Ok((order, report))
                },
                Message::PluginsSorted,
            );
        }
        Message::PluginsSorted(result) => {
            let (sorted, report_res) = match result {
                Ok(x) => x,
                Err(e) => {
                    app.status = Some(format!("LOOT sort failed: {e}"));
                    return Task::none();
                }
            };
            // Recompute spec + prefix dir (immutable borrows) before mutating plugins.
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            // A refresh may have invalidated the cache while LOOT ran off-thread;
            // recompute instead of silently discarding the sort (the report would
            // otherwise pop over an unsorted list).
            if app.plugins.is_none() {
                app.plugins = compute_plugins(app);
            }
            if let Some(list) = app.plugins.as_mut() {
                list.apply_sorted_order(&sorted);
                list.refresh(&spec);
            }
            let written =
                app.plugins.as_ref().map(|list| write_plugin_state(app, list, &spec)).transpose();
            app.status = Some(match written {
                Ok(_) => format!("LOOT sorted {} plugins.", sorted.len()),
                Err(e) => format!("Sorted, but writing the load order failed: {e}"),
            });
            // Show the LOOT report (MO2 always pops its dialog after a sort), so the
            // user sees missing masters / warnings / cleaning advice - or a clean bill.
            // The order was already applied above; a report failure only costs the
            // dialog, never the sort.
            match report_res {
                Ok(report) => app.loot_report = Some(report),
                Err(e) => {
                    let base = app.status.take().unwrap_or_default();
                    app.status = Some(format!("{base} (LOOT report unavailable: {e})"));
                }
            }
        }
        Message::CollisionMerge => run_collision_install(app, eidos_install::OverwritePolicy::Merge),
        Message::CollisionReplace => run_collision_install(app, eidos_install::OverwritePolicy::Replace),
        Message::CollisionRenameChanged(s) => {
            if let Some(c) = &mut app.collision {
                c.rename_to = s;
            }
        }
        Message::CollisionRenameCommit => {
            if let Some(new) = app.collision.as_ref().map(|c| c.rename_to.trim().to_string()) {
                if new.is_empty() {
                    app.status = Some("Enter a name to install under.".to_string());
                } else {
                    run_collision_install(app, eidos_install::OverwritePolicy::Rename(new));
                }
            }
        }
        Message::CollisionCancel => {
            app.collision = None;
            app.status = Some("Install cancelled.".to_string());
        }
        Message::ChangeGame => {
            // Re-open the game picker; keep detection and any selection.
            app.menu_mod = None;
            app.info_mod = None;
            app.executables = None;
            app.selected_mod = None;
            app.selected_mods.clear();
            app.drag_state = None;
            app.profile_menu = None;
            app.profile_rename = None;
            app.profile_copy = None;
            app.profile_delete_confirm = None;
            app.error = None;
            app.screen = Screen::Game;
        }
        Message::OpenNexusGame => {
            let domain = selected_game(app).map(|g| g.def.nexus_game).filter(|s| !s.is_empty());
            let url = match domain {
                Some(d) => format!("https://www.nexusmods.com/{d}"),
                None => "https://www.nexusmods.com".to_string(),
            };
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            app.status = Some(format!("Opening {url}"));
        }
        Message::OpenInstanceFolder => {
            if let Some(inst) = &app.created {
                let _ = std::process::Command::new("xdg-open").arg(&inst.root).spawn();
                app.status = Some(format!("Opened {}", inst.root.display()));
            }
        }
        Message::SetupPrereqs => {
            let id = selected_game(app).map(|g| g.def.id);
            let has_prefix = selected_game(app).and_then(|g| g.compatdata.as_ref()).is_some();
            let log = app.created.as_ref().map(|i| i.root.join("prereqs.log"));
            match (id, log) {
                (Some(_), _) if !has_prefix => {
                    app.status = Some(
                        "Launch the game once through Steam first so its Proton prefix exists, then run Tool Setup."
                            .to_string(),
                    );
                }
                (Some(id), Some(log)) => match run_prereqs_setup(id, &log) {
                    Ok(()) => {
                        app.status = Some(format!(
                            "Installing tool prerequisites: bundled DLLs copy now; .NET/vcrun download via winetricks. Progress + errors -> {}",
                            log.display()
                        ));
                    }
                    Err(e) => app.status = Some(format!("Could not start prereq setup: {e}")),
                },
                _ => app.status = Some("Open a game instance first.".to_string()),
            }
        }
        Message::ShowModInfo(i) => {
            app.menu_mod = None;
            let notes = match (app.created.as_ref(), app.mods.get(i)) {
                (Some(inst), Some(m)) => Some(inst.mod_meta(&m.name).notes().unwrap_or_default()),
                _ => None,
            };
            if let Some(notes) = notes {
                app.notes_edit = notes;
                app.info_mod = Some(i);
                app.info_tab = InfoTab::General;
            }
        }
        Message::CloseInfo => app.info_mod = None,
        Message::InfoSelectTab(t) => app.info_tab = t,
        Message::NotesChanged(s) => app.notes_edit = s,
        Message::NotesSave => {
            let result = match (app.info_mod, app.created.as_ref()) {
                (Some(i), Some(inst)) => app.mods.get(i).map(|m| {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_notes(&app.notes_edit);
                    (m.name.clone(), meta.write(&inst.meta_path(&m.name)))
                }),
                _ => None,
            };
            if let Some((name, r)) = result {
                app.status = Some(match r {
                    Ok(()) => format!("Saved notes for '{name}'."),
                    Err(e) => format!("Could not save notes: {e}"),
                });
            }
        }
        // ---- Settings / Preferences ------------------------------------------
        Message::OpenSettings => {
            app.menu_mod = None;
            app.api_key_error = None;
            // Re-read the stored key so the field reflects what's on disk.
            app.settings_api_key = eidos_instance::settings::load_nexus_key().unwrap_or_default();
            app.settings_open = true;
        }
        Message::CloseSettings => {
            app.settings_open = false;
            app.api_key_error = None;
        }
        Message::SettingsTabSelected(t) => app.settings_tab = t,
        Message::ApiKeyChanged(s) => {
            app.settings_api_key = s;
            app.api_key_error = None;
        }
        Message::ApiKeyValidateStart => {
            let key = app.settings_api_key.trim().to_string();
            if key.is_empty() {
                app.api_key_error = Some("Enter your personal Nexus API key.".to_string());
                return Task::none();
            }
            if app.api_key_validating {
                return Task::none();
            }
            app.api_key_validating = true;
            app.api_key_error = None;
            // Blocking ureq inside the async closure, like SortPlugins.
            return Task::perform(
                async move {
                    let result = eidos_nexus::Nexus::new(&key).validate();
                    (key, result)
                },
                |(key, result)| Message::ApiKeyValidateResult(key, result),
            );
        }
        Message::ApiKeyValidateResult(key, result) => {
            app.api_key_validating = false;
            match result {
                Ok(account) => {
                    // Persist the key that was validated (the field may have been
                    // edited during the round-trip) so the CLI and a relaunch see it.
                    let saved = eidos_instance::settings::save_nexus_key(&key);
                    app.status = Some(match &saved {
                        Ok(()) => format!("Connected to Nexus as {}.", account.name),
                        Err(e) => format!("Validated, but could not save the key: {e}"),
                    });
                    app.nexus_account = Some(account);
                }
                Err(e) => {
                    app.api_key_error = Some(e);
                }
            }
        }
        Message::ThemeChanged(t) => {
            app.prefs.theme = t;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::DefaultGameChanged(g) => {
            app.prefs.default_game = g;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToggleLockGui(v) => {
            app.prefs.lock_gui = v;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        // ---- Executables dialog ----------------------------------------------
        Message::ShowExecutablesDialog => {
            app.menu_mod = None;
            match open_executables_dialog(app) {
                Some(state) => app.executables = Some(state),
                None => app.status = Some("Open a game instance first.".to_string()),
            }
        }
        Message::CloseExecutablesDialog => app.executables = None,
        Message::SelectExecutableTool(i) => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                state.selected = Some(i);
                state.load_buffers();
            }
        }
        Message::AddExecutableTool => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                let tool = Tool {
                    title: "New Tool".to_string(),
                    exe: PathBuf::new(),
                    args: Vec::new(),
                    workdir: None,
                    prereqs: Vec::new(),
                };
                // User tools sit at the front, ahead of the read-only defaults.
                state.merged.insert(state.user_len, tool);
                state.selected = Some(state.user_len);
                state.user_len += 1;
                state.load_buffers();
            }
        }
        Message::DeleteExecutableTool => {
            if let Some(state) = &mut app.executables {
                if state.selected_is_user() {
                    if let Some(i) = state.selected {
                        state.merged.remove(i);
                        state.user_len -= 1;
                        state.selected = None;
                        state.load_buffers();
                    }
                }
            }
        }
        Message::MoveExecutableUp => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                if let Some(i) = state.selected {
                    if i > 0 && i < state.user_len {
                        state.merged.swap(i, i - 1);
                        state.selected = Some(i - 1);
                    }
                }
            }
        }
        Message::MoveExecutableDown => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                if let Some(i) = state.selected {
                    if i + 1 < state.user_len {
                        state.merged.swap(i, i + 1);
                        state.selected = Some(i + 1);
                    }
                }
            }
        }
        Message::ToolTitleChanged(s) => {
            if let Some(state) = &mut app.executables {
                // Auto-seed prereqs from the title for known tools (e.g. BodySlide ->
                // d3dx9_43, d3dcompiler_47), mirroring the CLI, but only when the user
                // has not entered any prereqs yet (never clobber their edit).
                if state.prereqs.trim().is_empty() {
                    let seeded = eidos_instance::default_prereqs(&s).join(", ");
                    if !seeded.is_empty() {
                        state.prereqs = seeded;
                    }
                }
                state.title = s;
            }
        }
        Message::ToolExeChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.exe = s;
            }
        }
        Message::ToolWorkdirChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.workdir = s;
            }
        }
        Message::ToolArgsChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.args = s;
            }
        }
        Message::ToolPrereqsChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.prereqs = s;
            }
        }
        Message::BrowseToolExe => {
            // Start the picker in the game install dir (where tool exes usually live).
            let start = selected_game(app).map(|g| g.install_path.clone());
            let mut dlg = rfd::AsyncFileDialog::new()
                .add_filter("Executables", &["exe"])
                .set_title("Select the tool executable");
            if let Some(dir) = start {
                dlg = dlg.set_directory(dir);
            }
            return Task::perform(dlg.pick_file(), |h| match h {
                Some(h) => Message::ToolExeChanged(h.path().display().to_string()),
                None => Message::Noop,
            });
        }
        Message::BrowseToolWorkdir => {
            let start = selected_game(app).map(|g| g.install_path.clone());
            let mut dlg = rfd::AsyncFileDialog::new().set_title("Select the working directory");
            if let Some(dir) = start {
                dlg = dlg.set_directory(dir);
            }
            return Task::perform(dlg.pick_folder(), |h| match h {
                Some(h) => Message::ToolWorkdirChanged(h.path().display().to_string()),
                None => Message::Noop,
            });
        }
        Message::SaveExecutablesDialog => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                // Reject a blank or control-char title up front (write_tools would
                // silently drop it, losing the user's edit without warning).
                let bad = state.merged[..state.user_len].iter().find(|t| {
                    let title = t.title.trim();
                    title.is_empty() || title.chars().any(char::is_control)
                });
                if bad.is_some() {
                    app.status = Some("Every tool needs a non-empty, single-line title.".to_string());
                    return Task::none();
                }
                let user_tools: Vec<Tool> = state.merged[..state.user_len].to_vec();
                if let Some(inst) = &app.created {
                    match inst.save_tools(&user_tools) {
                        Ok(()) => {
                            app.executables = None;
                            load_tools(app); // refresh the run-target picker
                            app.status = Some("Saved executables.".to_string());
                        }
                        Err(e) => app.status = Some(format!("Could not save executables: {e}")),
                    }
                }
            }
        }
        // ---- Endorse ---------------------------------------------------------
        Message::ModEndorse(i) => {
            if app.endorsing.is_some() {
                return Task::none();
            }
            let Some(key) = load_nexus_api_key() else {
                app.status = Some(
                    "Connect a Nexus account first (Settings, or `eidos nexus key <KEY>`).".to_string(),
                );
                return Task::none();
            };
            let domain = selected_game(app).map(|g| g.def.nexus_game.to_string());
            let folder = app.mods.get(i).map(|m| m.name.clone()).unwrap_or_default();
            let info = app.created.as_ref().zip(app.mods.get(i)).filter(|(_, m)| !m.is_separator()).map(
                |(inst, m)| {
                    let meta = inst.mod_meta(&m.name);
                    (meta.mod_id(), meta.version().unwrap_or_default(), meta.endorsed())
                },
            );
            let (Some(domain), Some((Some(mod_id), version, endorsed))) = (domain, info) else {
                app.status = Some("This mod has no Nexus id to endorse.".to_string());
                return Task::none();
            };
            // Toggle: endorse when not yet endorsed, abstain when already endorsed.
            let endorse = !endorsed;
            app.endorsing = Some(i);
            app.status = Some(
                if endorse { "Endorsing on Nexus...".to_string() } else { "Abstaining on Nexus...".to_string() },
            );
            return Task::perform(
                async move {
                    eidos_nexus::Nexus::new(&key).set_endorsed(&domain, mod_id, &version, endorse)
                },
                move |r| Message::ModEndorsed(folder.clone(), r),
            );
        }
        Message::ModEndorsed(folder, result) => {
            app.endorsing = None;
            match result {
                Ok(now_endorsed) => {
                    // Persist by folder name: the row index from before the network
                    // round-trip may point at a different mod by now.
                    if let (Some(inst), Some(m)) =
                        (app.created.as_ref(), app.mods.iter().find(|m| m.name == folder))
                    {
                        let mut meta = inst.mod_meta(&m.name);
                        meta.set("endorsed", if now_endorsed { "1" } else { "0" });
                        let _ = meta.write(&inst.meta_path(&m.name));
                        app.status = Some(format!(
                            "{} '{}' on Nexus.",
                            if now_endorsed { "Endorsed" } else { "Abstained from" },
                            m.display_name()
                        ));
                    }
                    recompute_counts(app);
                }
                Err(e) => app.status = Some(format!("Endorse failed: {e}")),
            }
        }
        // ---- per-mod local flags (Track / Ignore update) --------------------
        Message::ModTrack(i) => {
            app.menu_mod = None;
            if let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) {
                if !m.is_separator() {
                    let mut meta = inst.mod_meta(&m.name);
                    let now = !meta.tracked();
                    meta.set_tracked(now);
                    let _ = meta.write(&inst.meta_path(&m.name));
                    app.status = Some(format!(
                        "{} '{}'.",
                        if now { "Tracking" } else { "Untracked" },
                        m.display_name()
                    ));
                }
            }
        }
        Message::ModIgnoreUpdate(i) => {
            app.menu_mod = None;
            if let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) {
                if !m.is_separator() {
                    let mut meta = inst.mod_meta(&m.name);
                    let now = !meta.ignore_update();
                    meta.set_ignore_update(now);
                    let _ = meta.write(&inst.meta_path(&m.name));
                    app.status = Some(format!(
                        "{} updates for '{}'.",
                        if now { "Ignoring" } else { "Checking" },
                        m.display_name()
                    ));
                    // The update markers + count exclude ignored mods, so refresh.
                    app.meta_cache = build_meta_cache(app);
                    recompute_counts(app);
                }
            }
        }
        // ---- mod creation (Create empty mod / Install from folder) ----------
        Message::CreateEmptyMod => {
            app.menu_mod = None;
            if let Some(inst) = &app.created {
                // A unique "New Mod N" name, never colliding on disk.
                let mut n = 1usize;
                let mut name = "New Mod".to_string();
                while inst.mods_dir().join(&name).exists() {
                    n += 1;
                    name = format!("New Mod {n}");
                }
                match inst.create_empty_mod(&name) {
                    Ok(entry) => {
                        // New mods land at the TOP of the list (highest priority,
                        // matching where a fresh install goes) - index = end of vec.
                        let idx = app.mods.len();
                        app.mods.push(entry);
                        mods_changed(app);
                        app.selected_mod = Some(idx);
                        app.selected_mods.clear();
                        // Open its rename editor so the user names it straight away.
                        app.rename = Some((idx, name));
                        app.menu_mod = Some(idx);
                    }
                    Err(e) => app.status = Some(format!("Could not create mod: {e}")),
                }
            }
        }
        Message::InstallFromFolder => {
            app.menu_mod = None;
            // Pick an already-unpacked mod directory off-thread.
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title("Select an unpacked mod folder to install")
                    .pick_folder(),
                |handle| Message::FolderPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::FolderPicked(picked) => {
            let Some(src) = picked else { return Task::none() };
            let mods_dir = app.created.as_ref().map(|i| i.mods_dir());
            let Some(mods_dir) = mods_dir else {
                return Task::none();
            };
            // Name the new mod after the chosen folder (sanitized + de-duplicated).
            let raw = src.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let base = eidos_install::fix_directory_name(&raw).unwrap_or_else(|| "New Mod".to_string());
            let name = suggest_free_name(&mods_dir, &base);
            let dest = mods_dir.join(&name);
            // Copy the folder's CONTENTS into mods/<name>/ (not the folder itself),
            // mirroring how an archive's root is laid out.
            match copy_dir_contents(&src, &dest) {
                Ok(()) => after_install(app, &name, dest, false, None),
                Err(e) => {
                    let _ = fs::remove_dir_all(&dest);
                    app.status = Some(format!("Install from folder failed: {e}"));
                }
            }
        }
        // ---- Mod update check ------------------------------------------------
        Message::CheckUpdates => {
            if app.update_in_progress {
                return Task::none();
            }
            let Some(key) = load_nexus_api_key() else {
                app.status = Some(
                    "Connect a Nexus account first (Settings, or `eidos nexus key <KEY>`).".to_string(),
                );
                return Task::none();
            };
            let Some(domain) = selected_game(app).map(|g| g.def.nexus_game.to_string()) else {
                return Task::none();
            };
            let Some(inst) = app.created.clone() else {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            };
            app.update_in_progress = true;
            app.status = Some("Checking Nexus for mod updates...".to_string());
            return Task::perform(
                async move {
                    let nexus = eidos_nexus::Nexus::new(&key);
                    eidos_nexus::check_updates(&nexus, &inst, &domain)
                },
                Message::UpdatesChecked,
            );
        }
        Message::UpdatesChecked(result) => {
            app.update_in_progress = false;
            match result {
                Ok(r) => {
                    // Re-read meta so the `^` markers + counts pick up the writes.
                    app.meta_cache = build_meta_cache(app);
                    recompute_counts(app);
                    let mut msg = format!(
                        "Update check: {} mods checked, {} update(s) found.",
                        r.checked, r.updates_found
                    );
                    if r.rate_limited {
                        msg.push_str(" Hourly Nexus limit reached - some mods were left unchecked.");
                    }
                    app.status = Some(msg);
                }
                Err(e) => app.status = Some(format!("Update check failed: {e}")),
            }
        }
        // ---- menu bar --------------------------------------------------------
        Message::ShowAbout => {
            app.menu_mod = None;
            app.about_open = true;
        }
        Message::CloseAbout => app.about_open = false,
        Message::OpenViewMenu => app.view_menu_open = true,
        Message::CloseViewMenu => app.view_menu_open = false,
        Message::ToggleToolbar => {
            app.ui_toolbar_visible = !app.ui_toolbar_visible;
            app.view_menu_open = false;
        }
        Message::ToggleStatusBar => {
            app.ui_statusbar_visible = !app.ui_statusbar_visible;
            app.view_menu_open = false;
        }
        Message::CollapseAllGroups => {
            // Collapse every separator's group (key by display name, like MO2).
            for m in &app.mods {
                if m.is_separator() {
                    app.collapsed.insert(m.display_name().to_string());
                }
            }
            save_collapsed(app);
            app.view_menu_open = false;
        }
        Message::ExpandAllGroups => {
            app.collapsed.clear();
            save_collapsed(app);
            app.view_menu_open = false;
        }
        // ---- Saves tab ----
        Message::RefreshSaves => {
            load_saves(app);
            app.status = Some(format!("Found {} save file(s).", app.saves.len()));
        }
        Message::DeleteSave(i) => {
            // First click arms the confirm; clicking a different row re-arms it.
            app.confirm_delete_save = Some(i);
        }
        Message::ConfirmDeleteSave(i) => {
            // Only act on the armed row, and re-check the index (the list may have
            // shifted if the file vanished out from under us).
            if app.confirm_delete_save == Some(i) {
                if let Some(save) = app.saves.get(i) {
                    let name = save.filename.clone();
                    match std::fs::remove_file(&save.path) {
                        Ok(()) => app.status = Some(format!("Deleted save '{name}'.")),
                        // Already gone is success enough; surface real errors.
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.status = Some(format!("Save '{name}' was already gone."));
                        }
                        Err(e) => app.status = Some(format!("Could not delete '{name}': {e}")),
                    }
                }
                load_saves(app);
            }
        }
        // ---- Downloads manager ----
        Message::RefreshDownloads => {
            load_downloads(app);
            app.status = Some(format!("Found {} download(s).", app.downloads.len()));
        }
        Message::DeleteDownload(i) => {
            app.confirm_delete_download = Some(i);
        }
        Message::ConfirmDeleteDownload(i) => {
            if app.confirm_delete_download == Some(i) {
                if let Some(row) = app.downloads.get(i) {
                    let name = row.name.clone();
                    // Remove the archive and its `.meta` sidecar together (MO2 keeps
                    // them paired). A missing sidecar is fine.
                    let meta = PathBuf::from(format!("{}.meta", row.path.display()));
                    let archive_res = std::fs::remove_file(&row.path);
                    let _ = std::fs::remove_file(&meta);
                    match archive_res {
                        Ok(()) => app.status = Some(format!("Deleted download '{name}'.")),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.status = Some(format!("Download '{name}' was already gone."));
                        }
                        Err(e) => app.status = Some(format!("Could not delete '{name}': {e}")),
                    }
                }
                load_downloads(app);
            }
        }
        Message::BatchToggleMods => {
            // MO2-style batch enable/disable: if any selected real mod is enabled,
            // the whole selection is disabled; otherwise the whole selection is
            // enabled. Separators carry no toggle and are skipped.
            let targets: Vec<usize> = real_selection(app);
            if targets.is_empty() {
                app.status = Some("Select one or more mods first.".to_string());
                return Task::none();
            }
            let any_on = targets.iter().any(|&i| app.mods.get(i).is_some_and(|m| m.enabled));
            let new_state = !any_on;
            for &i in &targets {
                if let Some(m) = app.mods.get_mut(i) {
                    m.enabled = new_state;
                }
            }
            mods_changed(app);
            app.menu_mod = None;
            app.status = Some(format!(
                "{} {} mod(s).",
                if new_state { "Enabled" } else { "Disabled" },
                targets.len()
            ));
        }
        Message::BatchRemoveMods => {
            let n = real_selection(app).len();
            if n == 0 {
                app.status = Some("Select one or more mods first.".to_string());
                return Task::none();
            }
            app.confirm_batch_remove = true;
            app.status =
                Some(format!("Click Remove again to permanently delete {n} mod(s) from disk."));
        }
        Message::ConfirmBatchRemove => {
            app.confirm_batch_remove = false;
            app.menu_mod = None;
            // Delete from the highest index down so the lower indices stay valid.
            let mut targets = real_selection(app);
            targets.sort_unstable();
            let mut removed = 0usize;
            let mut failed = 0usize;
            for &i in targets.iter().rev() {
                if let Some(m) = app.mods.get(i).cloned() {
                    match fs::remove_dir_all(&m.path) {
                        Ok(()) => {
                            app.mods.remove(i);
                            drop_files_cache(app, Some(&m.name));
                            removed += 1;
                        }
                        Err(_) => failed += 1,
                    }
                }
            }
            app.selected_mods.clear();
            app.selected_mod = None;
            mods_changed(app);
            app.status = Some(if failed == 0 {
                format!("Removed {removed} mod(s).")
            } else {
                format!("Removed {removed} mod(s); {failed} could not be deleted.")
            });
        }
        Message::BatchSendTop => {
            // Lift the whole selection (keeping its relative order) to the top.
            let mut targets = real_selection(app);
            if targets.is_empty() {
                return Task::none();
            }
            targets.sort_unstable();
            let at = move_block(&mut app.mods, &targets, 0);
            // The selection is now a contiguous block at the destination.
            app.selected_mods = (at..at + targets.len()).collect();
            app.selected_mod = Some(at);
            mods_changed(app);
            app.menu_mod = None;
        }
        Message::BatchSendBottom => {
            let mut targets = real_selection(app);
            if targets.is_empty() {
                return Task::none();
            }
            targets.sort_unstable();
            let end = app.mods.len();
            let at = move_block(&mut app.mods, &targets, end);
            app.selected_mods = (at..at + targets.len()).collect();
            app.selected_mod = Some(at);
            mods_changed(app);
            app.menu_mod = None;
        }
        Message::DragStart(i) => {
            // Arm a drag and (re)select the row, unless a modifier means the click
            // was a multi-select gesture (then leave the existing selection alone).
            if app.modifiers.control()
                || app.modifiers.command()
                || app.modifiers.shift()
            {
                return update(app, Message::SelectMod(i));
            }
            app.selected_mod = Some(i);
            // Pressing a row that is NOT already in the multi-selection collapses the
            // selection to it; pressing one that IS keeps the group (so a mis-press
            // does not wipe a careful Ctrl/Shift selection). Drag still moves one row.
            if !app.selected_mods.contains(&i) {
                app.selected_mods.clear();
            }
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = Some(DragState { from: i, hover_over: i });
        }
        Message::DragOver(i) => {
            if let Some(d) = &mut app.drag_state {
                d.hover_over = i;
            }
        }
        Message::DragDrop => {
            if let Some(d) = app.drag_state.take() {
                if d.from != d.hover_over && d.from < app.mods.len() && d.hover_over < app.mods.len()
                {
                    let to = move_block(&mut app.mods, &[d.from], d.hover_over);
                    app.selected_mod = Some(to);
                    app.selected_mods.clear();
                    mods_changed(app);
                }
            }
        }
        Message::DragCancel => {
            app.drag_state = None;
        }
        Message::ModifiersChanged(mods) => {
            app.modifiers = mods;
        }
        Message::Noop => {}
    }
    Task::none()
}

/// Keep the multi-selection consistent when two rows are swapped (MoveUp/MoveDown):
/// a selected index follows its row to the new slot.
fn swap_in_selection(app: &mut App, a: usize, b: usize) {
    let has_a = app.selected_mods.contains(&a);
    let has_b = app.selected_mods.contains(&b);
    if has_a == has_b {
        return; // both or neither selected: the set is unchanged by the swap.
    }
    if has_a {
        app.selected_mods.remove(&a);
        app.selected_mods.insert(b);
    } else {
        app.selected_mods.remove(&b);
        app.selected_mods.insert(a);
    }
}

/// The real (non-separator) mods in the current multi-selection, as indices into
/// `app.mods`. Falls back to the single focus row when the set is empty, so a batch
/// action invoked with just one row selected still does the obvious thing.
fn real_selection(app: &App) -> Vec<usize> {
    let mut set = app.selected_mods.clone();
    if set.is_empty() {
        if let Some(f) = app.selected_mod {
            set.insert(f);
        }
    }
    set.into_iter()
        .filter(|&i| app.mods.get(i).is_some_and(|m| !m.is_separator()))
        .collect()
}

// ---- theme -------------------------------------------------------------------

fn palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: Color::from_rgb8(0xEC, 0xDF, 0xC2),
        text: Color::from_rgb8(0x2B, 0x20, 0x18),
        primary: Color::from_rgb8(0x7A, 0x1F, 0x2B),
        success: Color::from_rgb8(0x4A, 0x6B, 0x3A),
        danger: Color::from_rgb8(0x8A, 0x2A, 0x2A),
    }
}

fn theme(_app: &App) -> Theme {
    Theme::custom("Eidos".to_string(), palette())
}

fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
        border: Border { color: Color::from_rgb8(0x7A, 0x1F, 0x2B), width: 1.5, radius: 8.0.into() },
        ..Default::default()
    }
}

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
        border: Border { color: Color::from_rgb8(0x7A, 0x1F, 0x2B), width: 1.0, radius: 3.0.into() },
        ..Default::default()
    }
}

fn bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xE3, 0xD6, 0xB6))),
        border: Border { color: Color::from_rgb8(0xC9, 0xB8, 0x90), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

fn row_bg(even: bool) -> Color {
    if even {
        Color::from_rgb8(0xF3, 0xEA, 0xD3)
    } else {
        Color::from_rgb8(0xEA, 0xDD, 0xBF)
    }
}

/// Wrap a row with an alternating background (MO2-style row striping).
fn striped<'a>(content: Element<'a, Message>, even: bool) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .padding(2)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(row_bg(even))),
            ..Default::default()
        })
        .into()
}

/// The highlight behind the selected mod row.
const SEL_BG: Color = Color::from_rgb(0.812, 0.722, 0.525); // tan, distinct from the stripes

/// A mod-list row background that also reflects selection (MO2's blue highlight,
/// here a parchment-tan so it reads on the burgundy theme).
fn list_row<'a>(
    content: Element<'a, Message>,
    even: bool,
    selected: bool,
    drop_target: bool,
) -> Element<'a, Message> {
    let bg = if selected { SEL_BG } else { row_bg(even) };
    container(content)
        .width(Length::Fill)
        .padding(2)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            // During a drag, the hovered drop target gets a burgundy accent border
            // (iced 0.13 has no native drag image, so this is the drop-position cue).
            border: Border {
                color: if drop_target {
                    Color::from_rgb8(0x6E, 0x24, 0x2E)
                } else {
                    Color::TRANSPARENT
                },
                width: if drop_target { 2.0 } else { 0.0 },
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ---- shared widgets ----------------------------------------------------------

fn nav<'a>(label: &'a str, msg: Option<Message>, primary: bool) -> Element<'a, Message> {
    let mut b = button(text(label).size(13.0)).padding(8);
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    if primary {
        b.style(button::primary).into()
    } else {
        b.style(button::secondary).into()
    }
}

fn tool_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0)).padding(6).on_press(msg).style(button::secondary).into()
}

/// A flat, menu/toolbar-style button (no chrome until hovered).
fn flat_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(13.0)).padding(6).on_press(msg).style(button::text).into()
}

fn icon<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    image(image::Handle::from_bytes(bytes.to_vec()))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

/// A flat toolbar button: icon + label (MO2's icons-and-text mode).
fn icon_text_btn<'a>(bytes: &'static [u8], label: &'a str, msg: Message) -> Element<'a, Message> {
    let content = Row::new()
        .spacing(5)
        .push(icon(bytes, 16.0))
        .push(text(label).size(12.0));
    button(content).padding(5).on_press(msg).style(button::text).into()
}

/// A flat icon-only button (toolbar right group, row arrows).
fn icon_btn<'a>(bytes: &'static [u8], size: f32, msg: Option<Message>) -> Element<'a, Message> {
    let mut b = button(icon(bytes, size)).padding(3).style(button::text);
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
}

// ---- wizard ------------------------------------------------------------------

fn frame<'a>(
    step: &'a str,
    title: &'a str,
    content: Element<'a, Message>,
    back: Option<Message>,
    next_label: &'a str,
    next_msg: Option<Message>,
) -> Element<'a, Message> {
    let header = Column::new()
        .spacing(2)
        .push(text("Eidos").size(26.0))
        .push(text(step).size(12.0));

    let card = container(content).width(Length::Fill).padding(18).style(card_style);

    let footer = Row::new()
        .push(nav("Back", back, false))
        .push(Space::with_width(Length::Fill))
        .push(nav(next_label, next_msg, true));

    Column::new()
        .spacing(16)
        .push(header)
        .push(text(title).size(20.0))
        .push(card)
        .push(Space::with_height(Length::Fill))
        .push(footer)
        .into()
}

fn welcome<'a>() -> Element<'a, Message> {
    let content = Column::new()
        .spacing(10)
        .push(text("Eidos creates an isolated modding setup for your game,").size(15.0))
        .push(text("mounting your mods over the game without touching its files.").size(15.0))
        .push(text("Let's set up an instance.").size(13.0));
    frame("Step 1 of 5", "Welcome", content.into(), None, "Next", Some(Message::Next))
}

fn kind_card<'a>(label: &'a str, desc: &'a str, selected: bool, msg: Message) -> Element<'a, Message> {
    let inner = Column::new()
        .spacing(4)
        .push(text(label).size(16.0))
        .push(text(desc).size(12.0));
    button(inner)
        .width(Length::Fill)
        .padding(12)
        .on_press(msg)
        .style(if selected { button::primary } else { button::secondary })
        .into()
}

fn kind_screen<'a>(app: &App) -> Element<'a, Message> {
    let content = Column::new()
        .spacing(10)
        .push(kind_card(
            "Global",
            "Stored centrally in ~/.local/share/eidos, managed by Eidos. Recommended.",
            app.kind == InstanceKind::Global,
            Message::PickKind(InstanceKind::Global),
        ))
        .push(kind_card(
            "Portable",
            "A self-contained folder you choose. Movable and isolated.",
            app.kind == InstanceKind::Portable,
            Message::PickKind(InstanceKind::Portable),
        ));
    frame("Step 2 of 5", "Instance type", content.into(), Some(Message::Back), "Next", Some(Message::Next))
}

fn game_screen<'a>(app: &App) -> Element<'a, Message> {
    let content: Element<Message> = if app.games.is_empty() {
        Column::new()
            .push(text("No supported games detected.").size(15.0))
            .push(text("Install a supported game via Steam, then restart Eidos.").size(12.0))
            .into()
    } else {
        let mut list = Column::new().spacing(6);
        for (i, g) in app.games.iter().enumerate() {
            list = list.push(
                button(text(format!("{}  ({})", g.def.name, g.steam_name)).size(14.0))
                    .width(Length::Fill)
                    .padding(10)
                    .on_press(Message::PickGame(i))
                    .style(if app.selected == Some(i) { button::primary } else { button::secondary }),
            );
        }
        scrollable(list).height(Length::Fixed(240.0)).into()
    };
    let next = app.selected.map(|_| Message::Next);
    frame("Step 3 of 5", "Choose the game to mod", content, Some(Message::Back), "Next", next)
}

fn nameloc_screen<'a>(app: &App) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(8)
        .push(text("Instance name").size(13.0))
        .push(text_input("My Skyrim setup", &app.name).on_input(Message::NameChanged).padding(8));
    if app.kind == InstanceKind::Portable {
        content = content.push(text("Portable folder").size(13.0)).push(
            text_input("~/Eidos/skyrimse", &app.portable_path).on_input(Message::PortableChanged).padding(8),
        );
    }
    let next = (!app.name.trim().is_empty()).then_some(Message::Next);
    frame("Step 4 of 5", "Name and location", content.into(), Some(Message::Back), "Next", next)
}

fn summary_screen<'a>(app: &App) -> Element<'a, Message> {
    let kind = match app.kind {
        InstanceKind::Global => "Global",
        InstanceKind::Portable => "Portable",
    };
    let game = selected_game(app);
    let location = planned_instance(app).map(|i| i.root.display().to_string()).unwrap_or_default();

    let mut content = Column::new()
        .spacing(8)
        .push(text(format!("Name:     {}", app.name)).size(14.0))
        .push(text(format!("Type:     {kind}")).size(14.0))
        .push(text(format!("Game:     {}", game.map(|g| g.def.name).unwrap_or("(none)"))).size(14.0))
        .push(text(format!("Location: {location}")).size(13.0));
    if let Some(g) = game {
        content = content.push(text(format!("Game data: {}", g.data_path.display())).size(12.0));
    }
    if let Some(err) = &app.error {
        content = content.push(text(format!("Error: {err}")).size(13.0));
    }
    frame("Step 5 of 5", "Review and create", content.into(), Some(Message::Back), "Create instance", Some(Message::Finish))
}

// ---- main window (MO2 layout) ------------------------------------------------

const C_CHECK: Length = Length::Fixed(36.0);
const C_PRIO: Length = Length::Fixed(26.0);
const C_FLAGS: Length = Length::Fixed(46.0);
const C_VERSION: Length = Length::Fixed(64.0);
const C_CATEGORY: Length = Length::Fixed(96.0);
const C_CONTENT: Length = Length::Fixed(78.0);
const C_MOVE: Length = Length::Fixed(70.0);

/// Every file in the Overwrite as `/`-joined paths relative to it (recursive).
/// [`overwrite_entries`] memoised against the view generation: the Overwrite tab
/// and the mod-info file tree re-render constantly, and each render used to walk
/// the whole tree again. Rebuilds only after something changes on disk.
fn cached_entries(app: &App, dir: &Path) -> Vec<String> {
    let gen = app.view_generation.get();
    if let Some((at, entries)) = app.listing_cache.borrow().get(dir) {
        if *at == gen {
            return entries.clone();
        }
    }
    let entries = overwrite_entries(dir);
    app.listing_cache.borrow_mut().insert(dir.to_path_buf(), (gen, entries.clone()));
    entries
}

fn overwrite_entries(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Delete everything inside a directory, keeping the directory itself.
fn clear_dir_contents(dir: &Path) -> std::io::Result<()> {
    for e in fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            fs::remove_dir_all(&p)?;
        } else {
            fs::remove_file(&p)?;
        }
    }
    Ok(())
}

/// [`merged_listing`] memoised against the view generation - it read every
/// enabled mod's directory on each redraw of the Data tab.
fn cached_merged_listing(app: &App) -> Vec<DataRow> {
    let gen = app.view_generation.get();
    if let Some((at, entries)) = app.data_listing.borrow().as_ref() {
        if *at == gen {
            return entries.clone();
        }
    }
    let entries = merged_listing(app);
    *app.data_listing.borrow_mut() = Some((gen, entries.clone()));
    entries
}

/// Top-level entries of the merged view: each name, the source providing it
/// (highest-priority enabled mod, or the game data), and whether it's a folder.
/// Winner attribution matches what the FUSE layer actually serves: Overwrite
/// first, then mods from HIGHEST display priority down, then the game data.
fn merged_listing(app: &App) -> Vec<DataRow> {
    let mut seen = HashSet::new();
    let mut out: Vec<DataRow> = Vec::new();
    if let Some(inst) = app.created.as_ref() {
        if let Ok(rd) = fs::read_dir(inst.overwrite_dir()) {
            for e in rd.flatten() {
                if let Ok(name) = e.file_name().into_string() {
                    if seen.insert(name.clone()) {
                        out.push((name, "[Overwrite]".to_string(), e.path().is_dir()));
                    }
                }
            }
        }
    }
    // `app.mods` is display order = lowest priority first; the merged view's
    // winner is the highest, so walk it in reverse.
    for m in app.mods.iter().rev().filter(|m| m.enabled && !m.is_separator()) {
        if let Ok(rd) = fs::read_dir(&m.path) {
            for e in rd.flatten() {
                if let Ok(name) = e.file_name().into_string() {
                    if seen.insert(name.clone()) {
                        out.push((name, m.name.clone(), e.path().is_dir()));
                    }
                }
            }
        }
    }
    if let Some(g) = selected_game(app) {
        if let Ok(rd) = fs::read_dir(&g.data_path) {
            for e in rd.flatten() {
                if let Ok(name) = e.file_name().into_string() {
                    if seen.insert(name.clone()) {
                        out.push((name, format!("[{}]", g.def.id), e.path().is_dir()));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

/// The menu bar. iced 0.13 has no native dropdown widget, so most top-level items
/// fire the single most useful action (MO2's most-used per menu): File -> open the
/// instance folder, Tools -> Executables, Run -> run the current target, Help ->
/// About. View opens a small floating menu (it has several toggles to host).
fn menu_bar<'a>() -> Element<'a, Message> {
    let row = Row::new()
        .spacing(0)
        .push(flat_btn("File", Message::OpenInstanceFolder))
        .push(flat_btn("View", Message::OpenViewMenu))
        .push(flat_btn("Tools", Message::ShowExecutablesDialog))
        // Shortcut hints inline, MO2-style (the keys are wired in `subscription`).
        .push(flat_btn("Run (Ctrl+R)", Message::Run))
        .push(flat_btn("Refresh (F5)", Message::Refresh))
        .push(flat_btn("Help", Message::ShowAbout));
    container(row).width(Length::Fill).padding(1).style(bar_style).into()
}

/// The View dropdown's contents (floats over the window via the Stack, dismissed
/// by a click outside). Hosts the toolbar/status-bar toggles + collapse/expand-all.
fn view_menu_card<'a>(app: &App) -> Element<'a, Message> {
    let toolbar_label = if app.ui_toolbar_visible { "Hide toolbar" } else { "Show toolbar" };
    let status_label = if app.ui_statusbar_visible { "Hide status bar" } else { "Show status bar" };
    let col = Column::new()
        .spacing(1)
        .push(menu_item(toolbar_label, Message::ToggleToolbar))
        .push(menu_item(status_label, Message::ToggleStatusBar))
        .push(menu_sep())
        .push(menu_item("Collapse all groups", Message::CollapseAllGroups))
        .push(menu_item("Expand all groups", Message::ExpandAllGroups));
    menu_frame(col.into())
}

fn toolbar<'a>(app: &App) -> Element<'a, Message> {
    // Greyed (no on_press) while a Nexus call for that action is in flight; the
    // first selected mod is the endorse target (MO2's toolbar endorse button); it
    // must be a real mod with a Nexus id to act on.
    let endorse_target = app.selected_mod.filter(|&i| {
        app.mods.get(i).is_some_and(|m| {
            !m.is_separator()
                && app.meta_cache.get(&m.name).and_then(|r| r.mod_id).is_some()
        })
    });
    let endorse_msg = (app.endorsing.is_none()).then(|| endorse_target.map(Message::ModEndorse)).flatten();
    let update_msg = (!app.update_in_progress).then_some(Message::CheckUpdates);
    let row = Row::new()
        .spacing(2)
        .push(icon_text_btn(IC_INSTALL, "Install Mod", Message::InstallMod))
        .push(icon_text_btn(IC_NEXUS, "Nexus", Message::OpenNexusGame))
        .push(icon_text_btn(IC_CHANGE_GAME, "Change Game", Message::ChangeGame))
        .push(icon_text_btn(IC_REFRESH, "Refresh", Message::Refresh))
        .push(icon_text_btn(IC_EXECUTABLES, "Executables", Message::ShowExecutablesDialog))
        .push(icon_text_btn(IC_TOOLS, "Tool Setup", Message::SetupPrereqs))
        .push(icon_text_btn(IC_SETTINGS, "Settings", Message::OpenSettings))
        .push(Space::with_width(Length::Fill))
        .push(icon_btn(IC_ENDORSE, 20.0, endorse_msg))
        .push(icon_btn(IC_UPDATE, 20.0, update_msg))
        .push(icon_btn(IC_HELP, 20.0, Some(Message::ShowAbout)));
    container(row).width(Length::Fill).padding(2).style(bar_style).into()
}

#[allow(clippy::too_many_arguments)]
fn mod_row<'a>(
    i: usize,
    m: &ModEntry,
    len: usize,
    meta: Option<&RowMeta>,
    flag_icon: Option<&'static [u8]>,
    hidden_icon: Option<&'static [u8]>,
) -> Element<'a, Message> {
    let up = icon_btn(IC_UP, 14.0, (i > 0).then_some(Message::MoveUp(i)));
    let dn = icon_btn(IC_DOWN, 14.0, (i + 1 < len).then_some(Message::MoveDown(i)));
    // MO2's left-hand checkbox: a real square box, checked when the mod is enabled.
    let toggle = checkbox("", m.enabled).on_toggle(move |_| Message::ToggleMod(i)).size(16);

    // MO2's conflict emblem plus an optional hidden-files glyph (a mod can be both).
    let mut flags = Row::new().spacing(2);
    if let Some(bytes) = flag_icon {
        flags = flags.push(icon(bytes, 14.0));
    }
    if let Some(bytes) = hidden_icon {
        flags = flags.push(icon(bytes, 14.0));
    }
    let flag_cell: Element<'a, Message> = container(flags).width(C_FLAGS).into();

    // MO2's Version column; an update marker prefixes it when Nexus has a newer one.
    let version = meta.and_then(|r| r.version.clone()).unwrap_or_default();
    let version = match meta {
        Some(r) if r.update => format!("^ {version}"),
        _ => version,
    };
    // MO2's Category column: the mod's primary category, resolved to a name.
    let category = meta.and_then(|r| r.category_name.clone()).unwrap_or_default();
    // MO2's Content column: a compact letters summary of what the mod ships.
    let content = meta.map(|r| r.content_tags.clone()).unwrap_or_default();

    let row = Row::new()
        .spacing(6)
        .push(container(toggle).width(C_CHECK))
        .push(text(format!("{:>2}", i + 1)).size(12.0).width(C_PRIO))
        .push(text(m.name.clone()).size(13.0).width(Length::Fill))
        .push(text(category).size(11.0).width(C_CATEGORY))
        .push(text(content).size(10.0).width(C_CONTENT))
        .push(text(version).size(11.0).width(C_VERSION))
        .push(flag_cell)
        .push(Row::new().spacing(2).push(up).push(dn).width(C_MOVE));

    // Left-press selects + arms a drag, entering during a drag retargets the drop,
    // release commits it; right-click opens the action menu (MO2's context menu).
    // Inner buttons still get their own clicks; the mouse_area catches the rest.
    mouse_area(row)
        .on_press(Message::DragStart(i))
        .on_enter(Message::DragOver(i))
        .on_release(Message::DragDrop)
        .on_right_press(Message::OpenModMenu(i))
        .into()
}

/// Default separator bar colour when its `meta.ini` carries none (a parchment tan,
/// #C8B895).
const SEPARATOR_ACCENT: Color = Color::from_rgb(0.784, 0.722, 0.584);

/// A separator (group divider) row, MO2-style: a full-width coloured bar with the
/// display name centred, no checkbox / version / conflict flags, but still movable.
fn separator_row<'a>(
    i: usize,
    m: &ModEntry,
    len: usize,
    color: Option<[u8; 3]>,
    collapsed: bool,
    selected: bool,
) -> Element<'a, Message> {
    let up = icon_btn(IC_UP, 14.0, (i > 0).then_some(Message::MoveUp(i)));
    let dn = icon_btn(IC_DOWN, 14.0, (i + 1 < len).then_some(Message::MoveDown(i)));
    let bg = color.map(|[r, g, b]| Color::from_rgb8(r, g, b)).unwrap_or(SEPARATOR_ACCENT);

    // The collapse/expand toggle sits in the checkbox column (a separator has no
    // checkbox); it hides/shows the mods grouped beneath this separator.
    let collapse = button(text(if collapsed { "[+]" } else { "[-]" }).size(11.0))
        .padding([1, 4])
        .on_press(Message::ToggleCollapse(m.display_name().to_string()))
        .style(button::text);

    let name = container(text(m.display_name().to_string()).size(13.0))
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center);

    let row = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(container(collapse).width(C_CHECK))
        .push(text(format!("{:>2}", i + 1)).size(12.0).width(C_PRIO))
        .push(name)
        .push(Row::new().spacing(2).push(up).push(dn).width(C_MOVE));

    container(
        mouse_area(row)
            .on_press(Message::DragStart(i))
            .on_enter(Message::DragOver(i))
            .on_release(Message::DragDrop)
            .on_right_press(Message::OpenModMenu(i)),
    )
    .width(Length::Fill)
    .padding(3)
    .style(move |_t: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: if selected { Color::from_rgb8(0x6E, 0x24, 0x2E) } else { bg },
            width: if selected { 2.0 } else { 0.0 },
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn modlist_pane<'a>(app: &App) -> Element<'a, Message> {
    let active = app.mods.iter().filter(|m| m.enabled && !m.is_separator()).count();
    let active_name = app.created.as_ref().map(|i| i.active_profile()).unwrap_or_default();
    let mut profile = Row::new().spacing(6).push(text("Profile:").size(12.0));
    if let Some(inst) = &app.created {
        for name in inst.profiles() {
            let selected = name == active_name;
            // Left-click switches (MO2's profile selector); right-click opens the
            // rename / copy / delete menu (MO2's Profiles dialog actions).
            let chip = button(text(name.clone()).size(12.0))
                .padding(4)
                .on_press(Message::SwitchProfile(name.clone()))
                .style(if selected { button::primary } else { button::secondary });
            profile = profile
                .push(mouse_area(chip).on_right_press(Message::ProfileMenuOpen(name.clone())));
        }
    }
    let profile = profile
        .push(tool_btn("+ New", Message::NewProfile))
        .push(Space::with_width(Length::Fill))
        .push(
            text(format!(
                "Active: {active}  |  Endorsed: {}  |  Updates: {}",
                app.endorsed_count, app.updated_count
            ))
            .size(12.0),
        );

    // The category catalog (resolves ids -> names; drives the filter + the column).
    let cats = app.created.as_ref().map(|i| i.category_factory());

    // Category-filter dropdown: "All" + the top-level categories actually in use.
    let mut choices = vec![CategoryChoice { id: None, label: "All categories".to_string() }];
    if let Some(cf) = &cats {
        let mut used: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
        for r in app.meta_cache.values() {
            if let Some(mut cur) = r.category_id {
                for _ in 0..32 {
                    match cf.parent_id(cur) {
                        Some(p) if p != 0 && p != cur => cur = p,
                        _ => break,
                    }
                }
                used.insert(cur);
            }
        }
        for (id, name) in cf.all_top_level() {
            if used.contains(&id) {
                choices.push(CategoryChoice { id: Some(id), label: name.to_string() });
            }
        }
    }
    let selected = choices.iter().find(|c| c.id == app.category_filter).cloned();

    // MO2's mod-list filter box + a category dropdown + a button to drop a separator.
    let search = Row::new()
        .spacing(6)
        .push(
            text_input("Filter mods by name...", &app.search)
                .on_input(Message::SearchChanged)
                .padding(5)
                .size(12.0),
        )
        .push(
            pick_list(choices, selected, |c: CategoryChoice| Message::CategoryFilterChanged(c.id))
                .text_size(12.0)
                .padding(5),
        )
        .push(tool_btn("+ Separator", Message::AddSeparator(0)))
        .push(tool_btn("+ Empty mod", Message::CreateEmptyMod))
        .push(tool_btn("Install folder", Message::InstallFromFolder));

    let header = Row::new()
        .spacing(6)
        .push(text("").width(C_CHECK))
        .push(text("#").size(11.0).width(C_PRIO))
        .push(text("Mod Name").size(11.0).width(Length::Fill))
        .push(text("Category").size(11.0).width(C_CATEGORY))
        .push(text("Content").size(11.0).width(C_CONTENT))
        .push(text("Version").size(11.0).width(C_VERSION))
        .push(text("Flags").size(11.0).width(C_FLAGS))
        .push(text("").width(C_MOVE));

    let len = app.mods.len();
    let query = app.search.trim().to_lowercase();
    let mut list = Column::new().spacing(1);
    let mut shown = 0usize;
    if app.mods.is_empty() {
        list = list.push(text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0));
    }
    // Tracks whether the current separator's group is collapsed, so its mods hide.
    let mut in_collapsed = false;
    // The live drag's drop target, if any, so its row shows a feedback border.
    let drop_target = app.drag_state.map(|d| d.hover_over);
    for (i, m) in app.mods.iter().enumerate() {
        // A row is highlighted when it is the focus row or in the multi-selection.
        let selected = app.selected_mod == Some(i) || app.selected_mods.contains(&i);
        // A separator renders as a full-width group header - no checkbox, version,
        // conflict flags, or content (it never queries the ConflictMap). It always
        // shows (even under a filter, and even when its own group is collapsed).
        if m.is_separator() {
            // A category filter is about content; hide separators (no category) while
            // it's active, otherwise they show as group anchors.
            if app.category_filter.is_some() {
                continue;
            }
            let collapsed = app.collapsed.contains(m.display_name());
            in_collapsed = collapsed;
            shown += 1;
            let color = app.meta_cache.get(&m.name).and_then(|r| r.color);
            list = list.push(separator_row(i, m, len, color, collapsed, selected));
            continue;
        }
        // Mods under a collapsed separator are hidden; otherwise filter by name + category.
        if in_collapsed {
            continue;
        }
        if !query.is_empty() && !m.display_name().to_lowercase().contains(&query) {
            continue;
        }
        if let Some(fid) = app.category_filter {
            let matches = app
                .meta_cache
                .get(&m.name)
                .and_then(|r| r.category_id)
                .zip(cats.as_ref())
                .is_some_and(|(cid, cf)| cf.is_descendant_of(cid, fid));
            if !matches {
                continue;
            }
        }
        shown += 1;
        // MO2's conflict emblems; a disabled mod shows none (the checkbox says it).
        let flag_icon = if !m.enabled {
            None
        } else if let Some(c) = &app.conflicts {
            match c.state((i + 1) as u32) {
                ConflictState::Overwrites => Some(IC_CONFLICT_OVERWRITE),
                ConflictState::Overwritten => Some(IC_CONFLICT_OVERWRITTEN),
                ConflictState::Mixed => Some(IC_CONFLICT_MIXED),
                ConflictState::Redundant => Some(IC_CONFLICT_REDUNDANT),
                ConflictState::None => None,
            }
        } else {
            None
        };
        // A separate hidden-files glyph (MO2's FLAG_HIDDEN_FILES), shown alongside.
        let hidden_icon = if m.enabled {
            app.conflicts
                .as_ref()
                .and_then(|c| c.mods.get(&((i + 1) as u32)))
                .filter(|mc| mc.has_hidden)
                .map(|_| IC_CONFLICT_HIDDEN)
        } else {
            None
        };
        let meta = app.meta_cache.get(&m.name);
        // Show the drop-target border only while a drag is genuinely in motion (the
        // hovered row differs from the grabbed one), so a plain click never flashes.
        let is_drop = drop_target == Some(i)
            && app.drag_state.is_some_and(|d| d.from != d.hover_over);
        list = list.push(list_row(
            mod_row(i, m, len, meta, flag_icon, hidden_icon),
            i % 2 == 0,
            selected,
            is_drop,
        ));
    }
    if !app.mods.is_empty() && shown == 0 {
        list = list.push(text(format!("No mods match \"{}\".", app.search.trim())).size(12.0));
    }

    let overwrite = button(
        Row::new()
            .spacing(6)
            .push(text("").width(C_CHECK))
            .push(text("").width(C_PRIO))
            .push(text("Overwrite").size(13.0).width(Length::Fill)),
    )
    .padding(2)
    .on_press(Message::SelectTab(Tab::Overwrite))
    .style(button::text);

    // Wrap the list so the pointer leaving its bounds during a drag cancels it
    // (MO2 drops nothing when you release outside the list).
    let list_area = mouse_area(scrollable(list).height(Length::Fill)).on_exit(Message::DragCancel);

    let inner = Column::new()
        .spacing(6)
        .push(profile)
        .push(search)
        .push(header)
        .push(list_area)
        .push(overwrite);

    container(inner).width(Length::FillPortion(3)).height(Length::Fill).padding(8).style(panel_style).into()
}

/// A single left-aligned action in the mod context menu.
/// A menu row whose label is owned, so the resulting element borrows nothing.
fn menu_item_owned<'a>(label: String, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(msg)
        .style(button::text)
        .into()
}

fn menu_item<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(msg)
        .style(button::text)
        .into()
}

/// A small separator line inside the context menu.
fn menu_sep<'a>() -> Element<'a, Message> {
    container(Space::new(Length::Fill, Length::Fixed(1.0)))
        .padding([2, 6])
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xC8, 0xB8, 0x95))),
            ..Default::default()
        })
        .into()
}

/// MO2's right-click mod menu, rendered as a floating card (the action set from
/// modlistviewactions.cpp: enable/disable, send-to-top/bottom, explorer, Nexus,
/// reinstall, rename, remove). Shows the rename editor when a rename is in flight.
fn mod_menu_card<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(m) = app.mods.get(i) else {
        return Space::new(Length::Shrink, Length::Shrink).into();
    };

    // When more than one mod is selected, the right-click menu becomes a batch menu
    // (MO2 swaps the per-mod actions for selection-wide ones).
    if app.selected_mods.len() > 1 {
        return batch_mod_menu_card(app);
    }

    let title = Row::new()
        .spacing(6)
        .push(text(m.display_name().to_string()).size(13.0).width(Length::Fill))
        .push(
            button(text("x").size(13.0)).padding([1, 6]).on_press(Message::CloseMenu).style(button::text),
        );

    let mut col = Column::new().spacing(1).push(title);

    // A read-only info line (MO2 surfaces version/category/Nexus id on the row).
    if let Some(r) = app.meta_cache.get(&m.name) {
        let mut bits: Vec<String> = Vec::new();
        if let Some(v) = &r.version {
            bits.push(format!("v{v}"));
        }
        if let Some(c) = &r.category_name {
            bits.push(format!("cat {c}"));
        }
        if let Some(id) = r.mod_id {
            bits.push(format!("Nexus #{id}"));
        }
        if !bits.is_empty() {
            col = col.push(text(bits.join("  ·  ")).size(10.0));
        }
    }

    col = col.push(menu_sep());

    // Inline rename editor (MO2 renameMod) takes over the card while active.
    if let Some((ri, name)) = &app.rename {
        if *ri == i {
            let editor = text_input("New name", name)
                .on_input(Message::RenameChanged)
                .on_submit(Message::RenameCommit)
                .padding(5)
                .size(12.0);
            let actions = Row::new()
                .spacing(6)
                .push(tool_btn("Save", Message::RenameCommit))
                .push(tool_btn("Cancel", Message::CloseMenu));
            col = col.push(editor).push(actions);
            return menu_frame(col.into());
        }
    }

    // A separator gets a reduced menu: rename, colour, reorder, add-above, remove
    // (no enable/disable, information, reinstall, or Nexus - MO2 parity).
    if m.is_separator() {
        let current = app.meta_cache.get(&m.name).and_then(|r| r.color);
        col = col
            .push(menu_item("Rename", Message::RenameStart(i)))
            .push(separator_swatches(i, current))
            .push(menu_sep())
            .push(menu_item("Send to Top", Message::ModSendTop(i)))
            .push(menu_item("Send to Bottom", Message::ModSendBottom(i)))
            .push(menu_item("Add separator above", Message::AddSeparator(i)))
            .push(menu_item("Open in Explorer", Message::ModOpenFolder(i)))
            .push(menu_sep())
            .push(remove_button(app, i));
        return menu_frame(col.into());
    }

    col = col
        .push(menu_item("Information...", Message::ShowModInfo(i)))
        .push(menu_sep())
        .push(menu_item(if m.enabled { "Disable" } else { "Enable" }, Message::ToggleMod(i)))
        .push(menu_sep())
        .push(menu_item("Send to Top", Message::ModSendTop(i)))
        .push(menu_item("Send to Bottom", Message::ModSendBottom(i)))
        .push(send_to_targets(app, i))
        .push(menu_sep())
        .push(menu_item("Open in Explorer", Message::ModOpenFolder(i)));

    // Visit on Nexus + Endorse + Track only when we have a mod id to act on. The
    // Endorse / Track labels reflect the current state (MO2 toggles them).
    let meta = app.created.as_ref().map(|inst| inst.mod_meta(&m.name));
    let has_nexus = app.meta_cache.get(&m.name).and_then(|r| r.mod_id).is_some();
    if has_nexus {
        col = col.push(menu_item("Visit on Nexus", Message::ModVisitNexus(i)));
        let endorsed = meta.as_ref().is_some_and(|mm| mm.endorsed());
        let endorse_label = if endorsed { "Abstain (un-endorse)" } else { "Endorse" };
        col = col.push(menu_item(endorse_label, Message::ModEndorse(i)));
        let tracked = meta.as_ref().is_some_and(|mm| mm.tracked());
        let track_label = if tracked { "Untrack" } else { "Track" };
        col = col.push(menu_item(track_label, Message::ModTrack(i)));
    }
    // Ignore update is a local flag (MO2 shows it for any mod, Nexus id or not).
    let ignored = meta.as_ref().is_some_and(|mm| mm.ignore_update());
    let ignore_label = if ignored { "Check for updates" } else { "Ignore updates" };
    col = col.push(menu_item(ignore_label, Message::ModIgnoreUpdate(i)));

    col = col
        .push(menu_sep())
        .push(menu_item("Reinstall Mod", Message::ModReinstall(i)))
        .push(menu_item("Rename", Message::RenameStart(i)))
        .push(menu_item("Add separator above", Message::AddSeparator(i)))
        .push(menu_sep())
        .push(remove_button(app, i));

    menu_frame(col.into())
}

/// The batch context menu shown when several mods are selected at once (MO2's
/// multi-row right-click): enable/disable, send-to-top/bottom, and a two-click
/// Remove that wipes the whole selection from disk.
fn batch_mod_menu_card<'a>(app: &App) -> Element<'a, Message> {
    let targets = real_selection(app);
    let n = targets.len();
    // Mirror the batch toggle's decision so the label reads true ("Disable" when
    // any selected mod is on, else "Enable").
    let any_on = targets.iter().any(|&i| app.mods.get(i).is_some_and(|m| m.enabled));
    let toggle_label = if any_on { "Disable selected" } else { "Enable selected" };

    let title = Row::new()
        .spacing(6)
        .push(text(format!("{n} mods selected")).size(13.0).width(Length::Fill))
        .push(
            button(text("x").size(13.0))
                .padding([1, 6])
                .on_press(Message::CloseMenu)
                .style(button::text),
        );

    // Two-click guard: the first click arms (BatchRemoveMods), the second executes
    // (ConfirmBatchRemove). The label + danger style flip once armed.
    let (remove_label, remove_msg) = if app.confirm_batch_remove {
        ("Confirm remove?", Message::ConfirmBatchRemove)
    } else {
        ("Remove selected", Message::BatchRemoveMods)
    };
    let remove = button(text(remove_label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(remove_msg)
        .style(if app.confirm_batch_remove { button::danger } else { button::text });

    let col = Column::new()
        .spacing(1)
        .push(title)
        .push(menu_sep())
        .push(menu_item(toggle_label, Message::BatchToggleMods))
        .push(menu_sep())
        .push(menu_item("Send to Top", Message::BatchSendTop))
        .push(menu_item("Send to Bottom", Message::BatchSendBottom))
        .push(menu_sep())
        .push(remove);
    menu_frame(col.into())
}

/// The two-click Remove button shared by the mod and separator menus.
fn remove_button<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let label = if app.confirm_remove == Some(i) { "Confirm remove?" } else { "Remove" };
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(Message::ModRemove(i))
        .style(if app.confirm_remove == Some(i) { button::danger } else { button::text })
        .into()
}

/// A small palette of colour swatches for a separator (iced has no native colour
/// dialog), plus an "x" to clear back to the default.
fn separator_swatches<'a>(i: usize, current: Option<[u8; 3]>) -> Element<'a, Message> {
    const PALETTE: &[[u8; 3]] = &[
        [0x8b, 0x2e, 0x2e],
        [0x8b, 0x5e, 0x2e],
        [0x6e, 0x6e, 0x2e],
        [0x2e, 0x6e, 0x3e],
        [0x2e, 0x5e, 0x8b],
        [0x5e, 0x2e, 0x8b],
        [0x55, 0x55, 0x55],
    ];
    let mut row = Row::new().spacing(3).align_y(iced::Alignment::Center).push(text("Colour").size(10.0));
    for &rgb in PALETTE {
        let [r, g, b] = rgb;
        let sel = current == Some(rgb);
        let sw = button(Space::new(Length::Fixed(15.0), Length::Fixed(13.0)))
            .padding(0)
            .on_press(Message::SetSeparatorColor(i, Some(rgb)))
            .style(move |_t: &Theme, _s: button::Status| button::Style {
                background: Some(Background::Color(Color::from_rgb8(r, g, b))),
                border: Border {
                    color: Color::from_rgb8(0x3a, 0x2a, 0x1a),
                    width: if sel { 2.0 } else { 0.5 },
                    radius: 2.0.into(),
                },
                ..Default::default()
            });
        row = row.push(sw);
    }
    row.push(
        button(text("x").size(10.0))
            .padding([1, 4])
            .on_press(Message::SetSeparatorColor(i, None))
            .style(button::text),
    )
    .into()
}

/// The bordered card chrome around the context menu's contents.
fn menu_frame<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fixed(210.0))
        .padding(6)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
            border: Border {
                color: Color::from_rgb8(0x6E, 0x24, 0x2E),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ---- per-mod information dialog (MO2 modinfodialog) -------------------------

fn info_tab_btn<'a>(label: &'a str, tab: InfoTab, active: bool) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding([4, 10])
        .on_press(Message::InfoSelectTab(tab))
        .style(if active { button::primary } else { button::secondary })
        .into()
}

fn info_kv<'a>(k: &'a str, v: String) -> Element<'a, Message> {
    Row::new()
        .spacing(8)
        .push(text(k).size(12.0).width(Length::Fixed(120.0)))
        .push(text(v).size(12.0).width(Length::Fill))
        .into()
}

/// General tab: name/version/category/Nexus id/source/endorsed/tracked + counts.
fn info_general<'a>(app: &App, m: &ModEntry) -> Element<'a, Message> {
    let meta = app.created.as_ref().map(|inst| inst.mod_meta(&m.name));
    let files = cached_entries(app, &m.path).len();
    let mut col = Column::new().spacing(4).push(info_kv("Name", m.name.clone()));
    if let Some(meta) = &meta {
        if let Some(v) = meta.version() {
            col = col.push(info_kv("Version", v));
        }
        if let Some(nv) = meta.newest_version() {
            col = col.push(info_kv("Newest", nv));
        }
        if let Some(c) = app.meta_cache.get(&m.name).and_then(|r| r.category_name.clone()) {
            col = col.push(info_kv("Category", c));
        }
        if let Some(id) = meta.mod_id() {
            col = col.push(info_kv("Nexus id", id.to_string()));
        }
        if let Some(src) = meta.installation_file() {
            col = col.push(info_kv("Installed from", src));
        }
        col = col
            .push(info_kv("Endorsed", if meta.endorsed() { "yes".into() } else { "no".into() }))
            .push(info_kv("Tracked", if meta.tracked() { "yes".into() } else { "no".into() }));
    }
    col.push(info_kv("Enabled", if m.enabled { "yes".into() } else { "no".into() }))
        .push(info_kv("Files", files.to_string()))
        .push(info_kv("Folder", m.path.display().to_string()))
        .into()
}

/// Conflicts tab: which files this mod overrides, and which it loses, by mod name.
fn info_conflicts<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(cmap) = &app.conflicts else {
        return text("Conflicts not computed yet.").size(12.0).into();
    };
    let origin = (i + 1) as u32;
    let mut wins: Vec<(String, String)> = Vec::new();
    let mut loses: Vec<(String, String)> = Vec::new();
    for node in cmap.files.values() {
        if node.winner == origin && node.is_conflicted() {
            let losers: Vec<&str> =
                node.alternatives.iter().filter(|&&a| a != 0).map(|&a| cmap.name(a)).collect();
            wins.push((node.display_path.clone(), losers.join(", ")));
        } else if node.winner != origin && node.winner != 0 && node.alternatives.contains(&origin) {
            loses.push((node.display_path.clone(), cmap.name(node.winner).to_string()));
        }
    }
    let mut col = Column::new().spacing(2);
    col = col.push(text(format!("Overrides ({}):", wins.len())).size(13.0));
    if wins.is_empty() {
        col = col.push(text("  (none)").size(11.0));
    }
    for (p, who) in wins.iter().take(300) {
        col = col.push(text(format!("  {p}   >   {who}")).size(11.0));
    }
    col = col
        .push(Space::with_height(Length::Fixed(8.0)))
        .push(text(format!("Overridden by ({}):", loses.len())).size(13.0));
    if loses.is_empty() {
        col = col.push(text("  (none)").size(11.0));
    }
    for (p, who) in loses.iter().take(300) {
        col = col.push(text(format!("  {p}   <   {who}")).size(11.0));
    }
    col.into()
}

/// Filetree tab: every file the mod ships, relative to its root.
fn info_filetree<'a>(app: &App, m: &ModEntry) -> Element<'a, Message> {
    let entries = cached_entries(app, &m.path);
    let mut col = Column::new().spacing(1).push(text(format!("{} file(s):", entries.len())).size(12.0));
    for e in entries.into_iter().take(2000) {
        col = col.push(text(e).size(11.0));
    }
    col.into()
}

/// Notes tab: an editable note persisted to the mod's meta.ini.
fn info_notes<'a>(app: &App) -> Element<'a, Message> {
    Column::new()
        .spacing(8)
        .push(text("Note (saved to the mod's meta.ini):").size(12.0))
        .push(
            text_input("Add a note...", &app.notes_edit)
                .on_input(Message::NotesChanged)
                .on_submit(Message::NotesSave)
                .padding(6)
                .size(12.0),
        )
        .push(tool_btn("Save note", Message::NotesSave))
        .into()
}

/// MO2's per-mod info dialog: a centered modal with General / Conflicts /
/// Filetree / Notes tabs.
fn mod_info_dialog<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(m) = app.mods.get(i) else {
        return Space::new(Length::Shrink, Length::Shrink).into();
    };

    let title = Row::new()
        .spacing(8)
        .push(text(m.name.clone()).size(16.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([3, 10])
                .on_press(Message::CloseInfo)
                .style(button::secondary),
        );

    let tabs = Row::new()
        .spacing(4)
        .push(info_tab_btn("General", InfoTab::General, app.info_tab == InfoTab::General))
        .push(info_tab_btn("Conflicts", InfoTab::Conflicts, app.info_tab == InfoTab::Conflicts))
        .push(info_tab_btn("Filetree", InfoTab::Filetree, app.info_tab == InfoTab::Filetree))
        .push(info_tab_btn("Notes", InfoTab::Notes, app.info_tab == InfoTab::Notes));

    let content = match app.info_tab {
        InfoTab::General => info_general(app, m),
        InfoTab::Conflicts => info_conflicts(app, i),
        InfoTab::Filetree => info_filetree(app, m),
        InfoTab::Notes => info_notes(app),
    };

    let card = Column::new()
        .spacing(10)
        .push(title)
        .push(tabs)
        .push(scrollable(content).height(Length::Fill));

    container(card)
        .width(Length::Fixed(660.0))
        .height(Length::Fixed(460.0))
        .padding(16)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xEC, 0xDF, 0xC2))),
            border: Border {
                color: Color::from_rgb8(0x6E, 0x24, 0x2E),
                width: 2.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn data_panel<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .push(text("Name").size(11.0).width(Length::FillPortion(3)))
        .push(text("Mod").size(11.0).width(Length::FillPortion(2)))
        .push(text("Type").size(11.0).width(Length::Fixed(70.0)));

    let mut list = Column::new().spacing(1);
    let entries = cached_merged_listing(app);
    if entries.is_empty() {
        list = list.push(text("(empty)").size(12.0));
    }
    for (idx, (name, source, is_dir)) in entries.into_iter().take(500).enumerate() {
        let row = Row::new()
            .spacing(6)
            .push(text(name).size(12.0).width(Length::FillPortion(3)))
            .push(text(source).size(12.0).width(Length::FillPortion(2)))
            .push(text(if is_dir { "Folder" } else { "File" }).size(12.0).width(Length::Fixed(70.0)));
        list = list.push(striped(row.into(), idx % 2 == 0));
    }
    Column::new().spacing(4).push(header).push(scrollable(list).height(Length::Fill)).into()
}

fn overwrite_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.overwrite_dir();
    let actions = Row::new()
        .spacing(6)
        .push(
            text("Everything the game writes (configs, new saves, generated files) lands here.")
                .size(12.0)
                .width(Length::Fill),
        )
        // MO2's central Overwrite workflow: turn what the game/tools generated into
        // a real, orderable mod instead of only being able to delete it.
        .push(tool_btn("Create mod...", Message::OverwriteToModStart))
        .push(tool_btn("Open folder", Message::OpenFolder(dir.clone())))
        .push(
            button(text(if app.confirm_clear { "Confirm clear?" } else { "Clear" }).size(12.0))
                .padding(5)
                .on_press(Message::ClearOverwrite)
                .style(if app.confirm_clear { button::danger } else { button::secondary }),
        );

    // The inline name prompt, shown while "Create mod..." is armed. Typing an
    // existing mod's name merges into it (MO2's "move content to mod").
    let prompt: Option<Element<'a, Message>> = app.overwrite_to_mod.as_ref().map(|name| {
        let exists = inst.mods_dir().join(name.trim()).exists();
        let hint = if exists {
            "merges into that existing mod"
        } else {
            "creates a new mod at the top of the priority order"
        };
        Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text("Mod name").size(12.0))
            .push(
                text_input("Mod name", name)
                    .on_input(Message::OverwriteToModName)
                    .on_submit(Message::OverwriteToModCommit)
                    .padding(5)
                    .size(12.0)
                    .width(Length::Fixed(260.0)),
            )
            .push(text(hint).size(10.0).width(Length::Fill))
            .push(
                button(text("Create").size(12.0))
                    .padding([4, 12])
                    .on_press(Message::OverwriteToModCommit)
                    .style(button::primary),
            )
            .push(tool_btn("Cancel", Message::OverwriteToModCancel))
            .into()
    });

    let entries = cached_entries(app, &dir);
    let mut c = Column::new().spacing(2);
    if entries.is_empty() {
        c = c.push(text("(empty)").size(12.0));
    } else {
        c = c.push(text(format!("{} file(s):", entries.len())).size(11.0));
    }
    for e in entries.into_iter().take(500) {
        c = c.push(text(e).size(11.0));
    }

    let mut col = Column::new().spacing(8).push(actions);
    if let Some(p) = prompt {
        col = col.push(p);
    }
    col.push(scrollable(c).height(Length::Fill)).into()
}

/// Format a file's modified time as `YYYY-MM-DD HH:MM` (UTC), with only std - no
/// chrono. Used for the Saves "Date" column.
fn format_mtime(t: std::time::SystemTime) -> String {
    let Ok(dur) = t.duration_since(std::time::UNIX_EPOCH) else {
        return "-".to_string();
    };
    let secs = dur.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (hh, mm) = ((tod / 3600) % 24, (tod % 3600) / 60);
    // Civil date from a day count (Howard Hinnant's algorithm), days since epoch.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

/// Human-readable byte size for the Saves / Downloads size columns.
fn format_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MiB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.0} KiB", b / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// The Saves tab: the active profile's save files (name / date / size) plus
/// open-folder + per-save delete. MO2's savegame list.
fn saves_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.active().saves_dir();

    let header = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text("Save").size(13.0))
        .push(Space::with_width(Length::Fill))
        .push(button(text("Open folder").size(11.0)).padding(4).on_press(Message::OpenFolder(dir.clone())))
        .push(button(text("Refresh").size(11.0)).padding(4).on_press(Message::RefreshSaves));

    let col_header = Row::new()
        .spacing(8)
        .push(text("Name").size(11.0).width(Length::Fill))
        .push(text("Date").size(11.0).width(Length::Fixed(130.0)))
        .push(text("Size").size(11.0).width(Length::Fixed(80.0)))
        .push(Space::with_width(Length::Fixed(80.0)));

    let mut rows = Column::new().spacing(2);
    if app.saves.is_empty() {
        rows = rows.push(
            text("(no saves yet) Saves your game writes for this profile appear here.")
                .size(12.0),
        );
    }
    for (i, save) in app.saves.iter().take(SAVES_LIST_CAP).enumerate() {
        let armed = app.confirm_delete_save == Some(i);
        let del = button(text(if armed { "Confirm?" } else { "Delete" }).size(11.0))
            .padding(4)
            .on_press(if armed { Message::ConfirmDeleteSave(i) } else { Message::DeleteSave(i) })
            .style(if armed { button::danger } else { button::secondary });
        let row = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(text(save.filename.clone()).size(12.0).width(Length::Fill))
            .push(text(format_mtime(save.mtime)).size(11.0).width(Length::Fixed(130.0)))
            .push(text(format_size(save.size)).size(11.0).width(Length::Fixed(80.0)))
            .push(container(del).width(Length::Fixed(80.0)));
        rows = rows.push(striped(container(row).padding(3).into(), i % 2 == 0));
    }

    Column::new()
        .spacing(6)
        .push(header)
        .push(text(dir.display().to_string()).size(10.0))
        .push(col_header)
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

/// A short status label for a download row (MO2's downloads state column).
fn download_state_label(state: DownloadState) -> &'static str {
    match state {
        DownloadState::Untracked => "-",
        DownloadState::Ready => "Ready",
        DownloadState::Installed => "Installed",
        DownloadState::Uninstalled => "Uninstalled",
    }
}

fn downloads_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.downloads_dir();

    let header = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text("Downloads").size(13.0))
        .push(Space::with_width(Length::Fill))
        .push(button(text("Open folder").size(11.0)).padding(4).on_press(Message::OpenFolder(dir.clone())))
        .push(button(text("Refresh").size(11.0)).padding(4).on_press(Message::RefreshDownloads));

    let col_header = Row::new()
        .spacing(8)
        .push(text("Name").size(11.0).width(Length::Fill))
        .push(text("Version").size(11.0).width(Length::Fixed(80.0)))
        .push(text("Size").size(11.0).width(Length::Fixed(80.0)))
        .push(text("Status").size(11.0).width(Length::Fixed(90.0)))
        .push(Space::with_width(Length::Fixed(150.0)));

    let mut rows = Column::new().spacing(2);
    if app.downloads.is_empty() {
        rows = rows.push(
            text("No downloads yet. On Nexus, use \"Mod Manager Download\" once the handler is registered (eidos nxm --register), or drop archives here.")
                .size(11.0),
        );
    }
    for (i, row) in app.downloads.iter().enumerate() {
        let armed = app.confirm_delete_download == Some(i);
        // Two action buttons: Install (re-run the installer) and Delete.
        let install = button(text("Install").size(11.0))
            .padding(4)
            .on_press(Message::ModPicked(Some(row.path.clone())))
            .style(button::primary);
        let del = button(text(if armed { "Confirm?" } else { "Delete" }).size(11.0))
            .padding(4)
            .on_press(if armed { Message::ConfirmDeleteDownload(i) } else { Message::DeleteDownload(i) })
            .style(if armed { button::danger } else { button::secondary });
        // Prefer the friendly Nexus mod name when present, else the file name.
        let display = row.mod_name.clone().unwrap_or_else(|| row.name.clone());
        let r = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(text(display).size(12.0).width(Length::Fill))
            .push(text(row.version.clone()).size(11.0).width(Length::Fixed(80.0)))
            .push(text(format_size(row.size)).size(11.0).width(Length::Fixed(80.0)))
            .push(text(download_state_label(row.state)).size(11.0).width(Length::Fixed(90.0)))
            .push(
                Row::new()
                    .spacing(4)
                    .width(Length::Fixed(150.0))
                    .push(install)
                    .push(del),
            );
        rows = rows.push(striped(container(r).padding(3).into(), i % 2 == 0));
    }

    Column::new()
        .spacing(6)
        .push(header)
        .push(text(dir.display().to_string()).size(10.0))
        .push(col_header)
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

/// How serious a diagnostic is: `Problem` needs action (it will break or lose
/// something), `Advice` is worth knowing, `Ok` is a passing check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagLevel {
    Problem,
    Advice,
    Ok,
}

/// One health check: what it found, and what to do about it.
struct Diagnostic {
    level: DiagLevel,
    title: String,
    detail: String,
}

/// Run every health check for the current setup - MO2's problems panel, plus the
/// Linux-specific ones MO2 never needed (the launch capability above all, which
/// silently disables FUSE passthrough after each rebuild).
fn diagnostics(app: &App) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

    if app.cap_missing {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: "FUSE passthrough is off (launch capability missing)".to_string(),
            detail: format!(
                "Script-extender plugin DLLs may fail to load in-game. Run:  sudo setcap cap_sys_admin+ep {}  then press F5. Every rebuild of that binary wipes it.",
                find_eidos_binary().display()
            ),
        });
    } else {
        out.push(Diagnostic {
            level: DiagLevel::Ok,
            title: "FUSE passthrough available".to_string(),
            detail: "The launch binary carries CAP_SYS_ADMIN, so reads and DLL mapping go through the kernel.".to_string(),
        });
    }

    // Missing masters: the single most reliable crash predictor.
    match app.plugins.as_ref() {
        Some(list) => {
            let missing = list.missing_masters();
            if missing.is_empty() {
                out.push(Diagnostic {
                    level: DiagLevel::Ok,
                    title: "No missing masters".to_string(),
                    detail: format!("All {} plugins have their masters enabled.", list.plugins.len()),
                });
            } else {
                let mut detail = missing
                    .iter()
                    .take(8)
                    .map(|(p, m)| format!("{p} needs {m}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                if missing.len() > 8 {
                    detail.push_str(&format!("; and {} more", missing.len() - 8));
                }
                out.push(Diagnostic {
                    level: DiagLevel::Problem,
                    title: format!("{} plugin(s) are missing a master", missing.len()),
                    detail: format!("{detail}. The game will crash on load - enable or install them."),
                });
            }
        }
        None => out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: "Load order not computed yet".to_string(),
            detail: "Open the Plugins tab to analyse the load order.".to_string(),
        }),
    }

    // ENB + Community Shaders both injecting into D3D11.
    if let (Some(game), Some(inst)) = (selected_game(app), app.created.as_ref()) {
        let cs_roots: Vec<PathBuf> = inst
            .modlist()
            .into_iter()
            .filter(|m| m.enabled && !m.is_separator())
            .map(|m| m.path)
            .collect();
        if eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: "ENB and Community Shaders are both active".to_string(),
                detail: "They can run together, but if visuals look wrong disable one in its INI."
                    .to_string(),
            });
        }
    }

    // A non-empty Overwrite is generated content sitting outside any mod.
    if let Some(inst) = app.created.as_ref() {
        if !inst.overwrite_is_empty() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: "The Overwrite holds generated files".to_string(),
                detail: "Tool output (xEdit, DynDOLOD, Nemesis) is sitting outside any mod. Turn it into one from the Overwrite tab so it can be ordered and disabled.".to_string(),
            });
        }
        // Debris from an interrupted install.
        let debris: Vec<String> = fs::read_dir(inst.mods_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with(".eidos-install"))
            .collect();
        if !debris.is_empty() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: format!("{} leftover extraction folder(s)", debris.len()),
                detail: format!(
                    "An install was interrupted. They are ignored by the mod list and safe to delete from {}.",
                    inst.mods_dir().display()
                ),
            });
        }
    }

    // The game rewrites its own load order; a profile that never captured one is
    // still riding on the prefix's copy.
    if let Some(inst) = app.created.as_ref() {
        let prof = inst.active();
        if !prof.has_plugin_state() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: format!("Profile '{}' has no load order of its own yet", prof.name),
                detail: "It will adopt the current one on the next launch, after which switching profiles switches load orders.".to_string(),
            });
        }
    }

    // LOOT coverage for this game.
    if let Some(game) = selected_game(app) {
        if !eidos_loot::is_supported(game.def.id) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: format!("LOOT cannot sort {}", game.def.name),
                detail: "This game orders plugins by file timestamp; sort it by hand in the Plugins tab.".to_string(),
            });
        }
        // A Flatpak-Steam Proton runs from the host here, which can fail to resolve
        // its sandbox libraries. Eidos will not re-launch through `flatpak run`:
        // that would put the game in Flatpak's sandbox, blind to the FUSE union in
        // our private namespace, and it would silently play vanilla.
        if let Some(cd) = game.compatdata.as_ref() {
            let flatpak = eidos_games::proton_command(
                &home(),
                game.def.steam_app_id,
                cd,
                &game.install_path,
            )
            .is_some_and(|r| r.flatpak);
            if flatpak {
                out.push(Diagnostic {
                    level: DiagLevel::Problem,
                    title: "Proton comes from the Flatpak Steam install".to_string(),
                    detail: "It ships its runtime and steamclient libraries inside the sandbox, so running it from the host may fail. Install a Proton in ~/.steam/root/compatibilitytools.d/ and select it for this game.".to_string(),
                });
            }
        }
        if game.compatdata.is_none() {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: "No Proton prefix found".to_string(),
                detail: "Launch the game once through Steam so its prefix exists; until then the load order and INIs cannot be deployed.".to_string(),
            });
        }
    }

    out
}

/// The Diagnostics tab label, carrying the count of things needing attention.
fn diagnostics_tab_label(app: &App) -> String {
    let n = diagnostics(app).iter().filter(|d| d.level == DiagLevel::Problem).count();
    if n > 0 {
        format!("Diagnostics ({n})")
    } else {
        "Diagnostics".to_string()
    }
}

fn diagnostics_panel<'a>(app: &App) -> Element<'a, Message> {
    let checks = diagnostics(app);
    let problems = checks.iter().filter(|d| d.level == DiagLevel::Problem).count();
    let summary = if problems == 0 {
        "No problems found.".to_string()
    } else {
        format!("{problems} problem(s) need attention.")
    };
    let mut col = Column::new()
        .spacing(8)
        .push(text("Diagnostics").size(13.0))
        .push(text(summary).size(12.0));
    for d in checks {
        let (tag, color) = match d.level {
            DiagLevel::Problem => ("PROBLEM", Color::from_rgb8(0x8A, 0x2A, 0x2A)),
            DiagLevel::Advice => ("ADVICE", Color::from_rgb8(0xB0, 0x6A, 0x10)),
            DiagLevel::Ok => ("OK", Color::from_rgb8(0x3E, 0x73, 0x50)),
        };
        let card = Column::new()
            .spacing(2)
            .push(
                Row::new()
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .push(text(tag).size(9.0).color(color).width(Length::Fixed(58.0)))
                    .push(text(d.title).size(12.0).width(Length::Fill)),
            )
            .push(text(d.detail).size(10.5).color(Color::from_rgb8(0x6A, 0x5A, 0x40)));
        col = col.push(container(card).padding([4, 6]).width(Length::Fill).style(card_style));
    }
    scrollable(col).height(Length::Fill).into()
}

fn tab_btn<'a>(label: String, t: Tab, selected: bool) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding(6)
        .on_press(Message::SelectTab(t))
        .style(if selected { button::primary } else { button::secondary })
        .into()
}

/// Compute the ESP/ESM load order for the Plugins tab: discover from the selected
/// game's Data plus the enabled mods, preserve any existing prefix order, and
/// validate. `None` if there is no game with a plugin system.
fn compute_plugins(app: &App) -> Option<PluginList> {
    let game = selected_game(app)?;
    let spec = GameSpec::for_id(game.def.id)?;
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game.data_path.clone())];
    // app.mods is MO2 display order (lowest priority first) = the ascending order
    // plugin discovery wants, so feed it through as-is.
    let enabled = app.mods.iter().filter(|m| m.enabled && !m.is_separator());
    sources.extend(enabled.map(|m| (m.name.clone(), m.path.clone())));
    // The Overwrite layer is a plugin source too (a cleaned/generated .esp lands
    // there) - the launch path includes it, so the GUI must agree.
    if let Some(inst) = app.created.as_ref() {
        sources.push(("overwrite".to_string(), inst.overwrite_dir()));
    }

    let mut list = PluginList::discover(&sources, &spec);
    // The load order is per-profile: read the active profile's own copy once it
    // has one, and otherwise the prefix's (which the profile adopts on first
    // launch). Same primitive as the launch path, so for PlainList games this also
    // keeps "in loadorder.txt but not plugins.txt" DISABLED instead of silently
    // re-enabling plugins the user turned off.
    let profile_state = app
        .created
        .as_ref()
        .map(|i| i.active())
        .filter(|p| p.has_plugin_state())
        .map(|p| p.dir());
    match profile_state {
        Some(dir) => list.apply_prefix_state(&dir, &spec),
        None => {
            if let Some(cd) = game.compatdata.as_ref() {
                let dir = plugins_txt_dir(&cd.join("pfx"), &spec);
                list.apply_prefix_state(&dir, &spec);
            }
        }
    }
    list.refresh(&spec);
    Some(list)
}

/// Persist the plugin load order: into the active profile (which owns it) AND
/// into the prefix the game reads, so a profile switch swaps load orders and the
/// game still sees the current one without waiting for the next launch.
fn write_plugin_state(app: &App, list: &PluginList, spec: &GameSpec) -> std::io::Result<()> {
    if let Some(inst) = app.created.as_ref() {
        list.write_load_order(&inst.active().dir(), spec)?;
    }
    if let Some(cd) = selected_game(app).and_then(|g| g.compatdata.as_ref()) {
        list.write_load_order(&plugins_txt_dir(&cd.join("pfx"), spec), spec)?;
    }
    Ok(())
}

fn plugins_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(list) = &app.plugins else {
        return Column::new()
            .spacing(4)
            .push(text("Plugins (ESP / ESM / ESL load order)").size(13.0))
            .push(text("Open a game instance to compute the plugin load order.").size(12.0))
            .into();
    };

    let active = list.plugins.iter().filter(|p| p.enabled).count();
    let missing = list.missing_masters();

    // Top row: the plugin count plus a "Sort with LOOT" action (MO2's Sort button),
    // shown only for games LOOT can sort.
    let loot_ok = selected_game(app).map(|g| eidos_loot::is_supported(g.def.id)).unwrap_or(false);
    let mut top = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text(format!("{} plugins - {active} active", list.plugins.len())).size(12.0));
    if loot_ok {
        top = top.push(
            button(text("Sort with LOOT").size(11.0))
                .padding([3, 8])
                .on_press(Message::SortPlugins)
                .style(button::secondary),
        );
    }
    let mut head = Column::new().spacing(2).push(top);
    if !missing.is_empty() {
        head = head.push(
            text(format!("! {} missing master(s) - the game would crash", missing.len())).size(12.0),
        );
    }

    let header = Row::new()
        .spacing(6)
        .push(text("Index").size(11.0).width(Length::Fixed(52.0)))
        .push(text("On").size(11.0).width(Length::Fixed(28.0)))
        .push(text("Plugin").size(11.0).width(Length::Fill))
        .push(text("Type").size(11.0).width(Length::Fixed(36.0)));

    // Base-game masters are implicit/always-on; show them as forced, not togglable.
    let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
    let mut rows = Column::new().spacing(1);
    let total = list.plugins.len();
    for (i, p) in list.plugins.iter().enumerate() {
        let idx = p.index.clone().unwrap_or_else(|| "--".to_string());
        let kind = if p.is_light {
            "ESL"
        } else if p.loads_as_master() {
            "ESM"
        } else {
            "esp"
        };
        let is_primary = spec
            .as_ref()
            .map(|s| s.primary_plugins.iter().any(|pp| pp.eq_ignore_ascii_case(&p.name)))
            .unwrap_or(false);
        // MO2-style checkbox. A checkbox with no `on_toggle` renders disabled/greyed,
        // which is exactly the look for the non-togglable cases.
        let toggle: Element<'a, Message> = if is_primary {
            // A forced game master: always on, never togglable (checked + greyed).
            checkbox("", true).size(15).into()
        } else if p.force_disabled {
            // An .esl on a no-light engine: can never load (unchecked + greyed).
            checkbox("", false).size(15).into()
        } else {
            checkbox("", p.enabled).on_toggle(move |_| Message::TogglePlugin(i)).size(15).into()
        };
        // Manual reorder (MO2 lets the load order be moved by hand, not only
        // LOOT-sorted). refresh() re-applies the invariants after each move, so an
        // illegal position is corrected rather than persisted.
        let mut up = button(text("^").size(10.0)).padding([0, 5]).style(button::text);
        if i > 0 {
            up = up.on_press(Message::PluginMoveUp(i));
        }
        let mut down = button(text("v").size(10.0)).padding([0, 5]).style(button::text);
        if i + 1 < total {
            down = down.on_press(Message::PluginMoveDown(i));
        }
        let row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(idx).size(11.0).width(Length::Fixed(52.0)))
            .push(container(toggle).width(Length::Fixed(28.0)))
            .push(text(p.name.clone()).size(12.0).width(Length::Fill))
            .push(text(kind).size(10.0).width(Length::Fixed(36.0)))
            .push(up)
            .push(down);
        rows = rows.push(striped(row.into(), i % 2 == 0));
    }

    Column::new()
        .spacing(6)
        .push(head)
        .push(header)
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

/// Analyse file conflicts across the enabled mods (+ the game data) for the
/// Conflicts tab and the mod-row flags. Highest-priority mod first; game data
/// last as origin 0. `None` if there is no game.
fn compute_conflicts(app: &App) -> Option<ConflictMap> {
    let game = selected_game(app)?;
    // app.mods is MO2 display order (lowest priority first); the conflict crate wants
    // layers highest-priority first, so reverse. The origin stays the app.mods index
    // + 1 (NOT the layer position), so conflicts_panel's `origin = i + 1` lookup over
    // app.mods still maps to the same mod.
    let mut layers: Vec<Layer> = app
        .mods
        .iter()
        .enumerate()
        .filter(|(_, m)| m.enabled && !m.is_separator())
        .map(|(i, m)| Layer {
            origin: (i + 1) as u32,
            name: m.name.clone(),
            root: m.path.clone(),
        })
        .rev()
        .collect();
    // MO2's Overwrite is an always-active, top-priority pseudo-mod (xEdit / Bashed
    // Patch output lands there); include it at the front so the mods it shadows get
    // the overwritten emblem. Its whiteout markers are skipped by collect_files, and
    // its reserved origin (u32::MAX) keeps it distinct from BASE_ORIGIN (0).
    if let Some(inst) = app.created.as_ref() {
        let ow = inst.overwrite_dir();
        if ow.is_dir() {
            layers.insert(0, Layer { origin: u32::MAX, name: "Overwrite".to_string(), root: ow });
        }
    }
    layers.push(Layer {
        origin: 0,
        name: format!("[{}]", game.def.id),
        root: game.data_path.clone(),
    });
    Some(build_conflicts_cached(app, layers))
}

/// Build the conflict map from cached per-layer file walks: only layers missing
/// from the cache touch the filesystem, so a toggle/reorder (same set of mods)
/// re-derives winners entirely in memory. The cache is keyed by layer name
/// (mod folder names are unique; the game/Overwrite pseudo-layers use their
/// bracketed display names).
fn build_conflicts_cached(app: &App, layers: Vec<Layer>) -> ConflictMap {
    let mut cache = app.files_cache.borrow_mut();
    let parts: Vec<(Layer, (Vec<String>, bool))> = layers
        .into_iter()
        .map(|l| {
            let files = cache
                .entry(l.name.clone())
                .or_insert_with(|| eidos_conflicts::collect_files(&l.root))
                .clone();
            (l, files)
        })
        .collect();
    ConflictMap::build_from(&parts)
}

fn conflicts_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(map) = &app.conflicts else {
        return Column::new()
            .spacing(4)
            .push(text("Conflicts").size(13.0))
            .push(text("Open a game instance to analyse file conflicts across your mods.").size(12.0))
            .into();
    };

    let mut counts = (0usize, 0usize, 0usize, 0usize); // overwrites, overwritten, mixed, redundant
    let mut rows = Column::new().spacing(1);
    for (i, m) in app.mods.iter().enumerate().filter(|(_, m)| m.enabled && !m.is_separator()) {
        let origin = (i + 1) as u32;
        let tag = match map.state(origin) {
            ConflictState::Overwrites => {
                counts.0 += 1;
                "overwrites others"
            }
            ConflictState::Overwritten => {
                counts.1 += 1;
                "overwritten"
            }
            ConflictState::Mixed => {
                counts.2 += 1;
                "mixed"
            }
            ConflictState::Redundant => {
                counts.3 += 1;
                "redundant - wins nothing"
            }
            ConflictState::None => continue,
        };
        let detail = map
            .mods
            .get(&origin)
            .map(|c| format!("{}/{} won", c.won, c.total))
            .unwrap_or_default();
        let row = Row::new()
            .spacing(6)
            .push(text(m.name.clone()).size(12.0).width(Length::Fill))
            .push(text(tag).size(11.0).width(Length::Fixed(160.0)))
            .push(text(detail).size(10.0).width(Length::Fixed(80.0)));
        rows = rows.push(striped(row.into(), i % 2 == 0));
    }

    let summary = format!(
        "{} overwrite - {} overwritten - {} mixed - {} redundant",
        counts.0, counts.1, counts.2, counts.3
    );
    Column::new()
        .spacing(6)
        .push(text(format!("Conflicts: {summary}")).size(12.0))
        .push(text("(only conflicting mods shown; flags also appear in the mod list)").size(10.0))
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

fn right_pane<'a>(app: &App) -> Element<'a, Message> {
    // Run-target picker (MO2's executables combo): the game, or any tool run
    // through the same merged view. The game's launcher/binary + script extender are
    // auto-detected as tools, so they show up here alongside the user's tools.
    let run_options: Vec<String> = std::iter::once(RUN_GAME.to_string())
        .chain(app.tools.iter().map(|t| t.title.clone()))
        .collect();
    let run_choice = app.tool_choice.clone().unwrap_or_else(|| RUN_GAME.to_string());

    let top = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text("Run:").size(13.0))
        .push(pick_list(run_options, Some(run_choice), Message::ToolPicked).text_size(13.0).padding(8))
        .push(Space::with_width(Length::Fill))
        .push(
            button(Row::new().spacing(6).push(icon(IC_RUN, 18.0)).push(text("Run").size(15.0)))
                .padding(10)
                .on_press(Message::Run)
                .style(button::primary),
        );

    let tabs = Row::new()
        .spacing(4)
        .push(tab_btn("Data".to_string(), Tab::Data, app.tab == Tab::Data))
        .push(tab_btn("Plugins".to_string(), Tab::Plugins, app.tab == Tab::Plugins))
        .push(tab_btn("Conflicts".to_string(), Tab::Conflicts, app.tab == Tab::Conflicts))
        .push(tab_btn("Overwrite".to_string(), Tab::Overwrite, app.tab == Tab::Overwrite))
        .push(tab_btn("Saves".to_string(), Tab::Saves, app.tab == Tab::Saves))
        .push(tab_btn("Downloads".to_string(), Tab::Downloads, app.tab == Tab::Downloads))
        .push(tab_btn(diagnostics_tab_label(app), Tab::Diagnostics, app.tab == Tab::Diagnostics));

    let content = match app.tab {
        Tab::Data => data_panel(app),
        Tab::Plugins => plugins_panel(app),
        Tab::Conflicts => conflicts_panel(app),
        Tab::Overwrite => overwrite_panel(app),
        Tab::Saves => saves_panel(app),
        Tab::Downloads => downloads_panel(app),
        Tab::Diagnostics => diagnostics_panel(app),
    };

    let inner = Column::new().spacing(8).push(top).push(tabs).push(content);
    container(inner).width(Length::FillPortion(2)).height(Length::Fill).padding(8).style(panel_style).into()
}

fn status_bar<'a>(app: &App) -> Element<'a, Message> {
    let kind = match app.kind {
        InstanceKind::Global => "Global",
        InstanceKind::Portable => "Portable",
    };
    let game = selected_game(app).map(|g| g.def.name).unwrap_or("Instance");
    // A live multi-selection count takes the left slot (MO2's "N selected"), unless a
    // transient status message is showing; otherwise the instance summary.
    let showing_status = app.status.is_some();
    let left = if let Some(s) = app.status.clone() {
        s
    } else if app.selected_mods.len() > 1 {
        format!("{} mods selected", app.selected_mods.len())
    } else {
        let profile = app
            .created
            .as_ref()
            .map(|i| i.active().name)
            .unwrap_or_else(|| "Default".to_string());
        format!("{game} - {kind} - {profile}")
    };
    // The Nexus account, if connected this session (MO2's status-bar login state).
    let account = match &app.nexus_account {
        Some(a) if a.is_premium => format!("Nexus: {} (Premium)", a.name),
        Some(a) => format!("Nexus: {}", a.name),
        None => "not logged in".to_string(),
    };
    let mut row = Row::new()
        .align_y(iced::Alignment::Center)
        .push(text(left).size(11.0).width(Length::Fill));
    if showing_status {
        // A tiny dismiss so a stale message stops masking the selection count and
        // instance summary.
        row = row.push(
            button(text("x").size(10.0))
                .padding([0, 6])
                .on_press(Message::ClearStatus)
                .style(button::text),
        );
    }
    row = row.push(text(account).size(11.0));
    container(row).width(Length::Fill).padding(4).style(bar_style).into()
}

fn main_screen<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(10)
        .push(text("Eidos").size(20.0))
        .push(Space::with_width(Length::Fill))
        .push(tool_btn("New instance", Message::Restart));

    let body = Row::new()
        .spacing(8)
        .height(Length::Fill)
        .push(modlist_pane(app))
        .push(right_pane(app));

    let mut base = Column::new().spacing(4).padding(4).push(header).push(menu_bar());
    if app.ui_toolbar_visible {
        base = base.push(toolbar(app));
    }
    // Persistent warning while the eidos binary lacks CAP_SYS_ADMIN: launches
    // still work but FUSE passthrough is off and SKSE plugin DLLs may fail to
    // load. Every rebuild wipes the capability, so this fires often enough that
    // silence cost real debugging time.
    if app.cap_missing {
        base = base.push(cap_warning_banner());
    }
    base = base.push(body);
    if app.ui_statusbar_visible {
        base = base.push(status_bar(app));
    }

    let mut layers = Stack::new().push(base);

    // The right-click action menu floats over the window (MO2's context menu).
    // A full-window catcher behind it dismisses on a click outside the card.
    if let Some(i) = app.menu_mod {
        if i < app.mods.len() {
            let catcher =
                mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseMenu);
            let card = container(mod_menu_card(app, i))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding { top: 150.0, right: 0.0, bottom: 0.0, left: 40.0 })
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Top);
            layers = layers.push(catcher).push(card);
        }
    }

    // The per-mod info dialog is a centered modal (MO2's modinfodialog).
    if let Some(i) = app.info_mod {
        if i < app.mods.len() {
            let scrim =
                mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseInfo);
            let dialog = container(mod_info_dialog(app, i)).center(Length::Fill);
            layers = layers.push(scrim).push(dialog);
        }
    }

    // The install-collision chooser is a centered modal (MO2's QueryOverwriteDialog).
    if let Some(c) = &app.collision {
        let scrim =
            mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CollisionCancel);
        let dialog = container(collision_dialog(c)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Preferences modal (MO2's Settings dialog).
    if app.settings_open {
        let scrim =
            mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseSettings);
        let dialog = container(settings_dialog(app)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Executables editor (MO2's Modify Executables dialog).
    if let Some(state) = &app.executables {
        let scrim = mouse_area(Space::new(Length::Fill, Length::Fill))
            .on_press(Message::CloseExecutablesDialog);
        let dialog = container(executables_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The About box (Help menu).
    if app.about_open {
        let scrim = mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseAbout);
        let dialog = container(about_dialog()).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The View dropdown floats just under the menu bar, near the View item.
    if app.view_menu_open {
        let catcher =
            mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseViewMenu);
        let card = container(view_menu_card(app))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 44.0, right: 0.0, bottom: 0.0, left: 44.0 })
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top);
        layers = layers.push(catcher).push(card);
    }

    // The per-profile menu (rename / copy / delete), opened by right-clicking a
    // profile chip. A catcher behind it dismisses on an outside click.
    if let Some(name) = app.profile_menu.clone() {
        let catcher =
            mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::ProfileCloseMenu);
        let card = container(profile_menu_card(app, &name))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 120.0, right: 0.0, bottom: 0.0, left: 60.0 })
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top);
        layers = layers.push(catcher).push(card);
    }

    // The LOOT report (MO2's post-sort dialog): a centered modal listing general
    // messages + per-plugin missing masters / messages / dirty advice.
    if let Some(report) = &app.loot_report {
        let scrim =
            mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseLootReport);
        let dialog = container(loot_report_dialog(report)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The run lock (MO2's "lock GUI while the application runs"): a full-window
    // overlay that blocks everything beneath it until the game exits or the user
    // clicks Unlock. Added last so it sits on top of every other layer. A tracked
    // run with `lock` off (setting disabled, or force-unlocked) shows no overlay.
    if let Some(run) = app.running.as_ref().filter(|r| r.lock) {
        // A backdrop that swallows EVERY pointer event (press/release/right/scroll)
        // so nothing beneath it is reachable - clicks, context menus and the modlist
        // scroll wheel are all inert while locked. `interaction` also tells the Stack
        // to mark lower layers unavailable for scroll.
        let scrim = mouse_area(
            container(Space::new(Length::Fill, Length::Fill)).style(|_| container::Style {
                background: Some(iced::Color { a: 0.55, ..iced::Color::BLACK }.into()),
                ..Default::default()
            }),
        )
        .on_press(Message::Noop)
        .on_release(Message::Noop)
        .on_right_press(Message::Noop)
        .on_scroll(|_| Message::Noop)
        .interaction(iced::mouse::Interaction::NotAllowed);
        let dialog = container(running_lock_card(run)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    layers.into()
}

/// MO2's targeted "Send to" actions, below the blunt top/bottom pair.
///
/// The two conflict-relative moves are the ones people actually reach for -
/// "put this just above the mod it is overriding" - and both are gated on the
/// relevant set being non-empty, so the menu never offers a move that would do
/// nothing. Priority and separator open an inline editor rather than a modal,
/// matching how rename already works in this menu.
fn send_to_targets<'a>(app: &App, i: usize) -> Element<'a, Message> {
    // Same origin convention as the emblems: index + 1, with the game (0) and the
    // Overwrite pseudo-layer (u32::MAX) excluded because they are not rows.
    let real = |set: &std::collections::BTreeSet<u32>| {
        set.iter().any(|&o| o != 0 && o != u32::MAX)
    };
    let mc = app.conflicts.as_ref().and_then(|m| m.mods.get(&((i + 1) as u32)));
    let mut col = Column::new().spacing(1);

    if let Some((row, text)) = app.send_priority.as_ref().filter(|(r, _)| *r == i) {
        let _ = row;
        col = col.push(
            text_input("Priority", text)
                .on_input(Message::SendToPriorityChanged)
                .on_submit(Message::SendToPriorityCommit)
                .padding(5)
                .size(12.0),
        );
        return col.into();
    }
    if app.send_separator == Some(i) {
        // An inline chooser of the separators, scrollable because a big load
        // order has plenty of them.
        let mut list = Column::new().spacing(1);
        for (idx, sep) in app.mods.iter().enumerate().filter(|(_, m)| m.is_separator()) {
            // Owned, so the Element does not borrow from `app`.
            let label = sep.display_name().to_string();
            list = list.push(menu_item_owned(label, Message::SendToSeparatorPick(idx)));
        }
        col = col
            .push(text("Move into group:").size(11.0))
            .push(scrollable(list).height(Length::Fixed(160.0)))
            .push(menu_item("Cancel", Message::SendToTargetCancel));
        return col.into();
    }

    if mc.is_some_and(|m| real(&m.overwrites)) {
        col = col.push(menu_item("Send above first conflict", Message::SendToFirstConflict(i)));
    }
    if mc.is_some_and(|m| real(&m.overwritten_by)) {
        col = col.push(menu_item("Send below last conflict", Message::SendToLastConflict(i)));
    }
    col = col
        .push(menu_item("Send to priority...", Message::SendToPriorityStart(i)))
        .push(menu_item("Send to separator...", Message::SendToSeparatorStart(i)));
    col.into()
}

/// The per-profile context menu (MO2's profile manager actions), opened by
/// right-clicking a profile chip: rename, copy-to-new, delete (two-click confirm).
fn profile_menu_card<'a>(app: &App, name: &str) -> Element<'a, Message> {
    let title = Row::new()
        .spacing(6)
        .push(text(format!("Profile: {name}")).size(13.0).width(Length::Fill))
        .push(
            button(text("x").size(13.0))
                .padding([1, 6])
                .on_press(Message::ProfileCloseMenu)
                .style(button::text),
        );
    let mut col = Column::new().spacing(1).push(title).push(menu_sep());

    // Rename: an inline editor when armed, else a menu item that arms it.
    match &app.profile_rename {
        Some((orig, edited)) if orig == name => {
            col = col.push(
                text_input("New name", edited)
                    .on_input(Message::ProfileRenameChanged)
                    .on_submit(Message::ProfileRenameCommit)
                    .padding(5)
                    .size(12.0),
            );
        }
        _ => col = col.push(menu_item("Rename", Message::ProfileRenameStart(name.to_string()))),
    }

    // Copy to a new profile: an inline editor when armed, else a menu item.
    match &app.profile_copy {
        Some((src, edited)) if src == name => {
            col = col.push(
                text_input("Copy name", edited)
                    .on_input(Message::ProfileCopyChanged)
                    .on_submit(Message::ProfileCopyCommit)
                    .padding(5)
                    .size(12.0),
            );
        }
        _ => col = col.push(menu_item("Copy to new...", Message::ProfileCopyStart(name.to_string()))),
    }

    col = col.push(menu_sep());
    // Take over an existing MO2 profile's mod order + load order, so a migrating
    // user does not re-tick dozens of mods and plugins by hand.
    col = col.push(menu_item("Import from MO2...", Message::ImportMo2Pick));

    col = col.push(menu_sep());
    // Delete: two-click confirm (backend refuses the active / last profile).
    let delete: Element<'a, Message> = if app.profile_delete_confirm.as_deref() == Some(name) {
        button(text("Click again to delete").size(12.0))
            .padding([2, 6])
            .width(Length::Fill)
            .on_press(Message::ProfileDeleteCommit(name.to_string()))
            .style(button::danger)
            .into()
    } else {
        menu_item("Delete", Message::ProfileDeleteConfirm(name.to_string()))
    };
    col = col.push(delete);

    container(col).max_width(240.0).padding(8).style(card_style).into()
}

/// Suggest a free profile name near `base` (`base`, `base 2`, `base 3`, ...) so the
/// copy editor never starts on a name that already collides.
fn suggest_free_profile_name(inst: &Instance, base: &str) -> String {
    if !inst.profile(base).dir().exists() {
        return base.to_string();
    }
    (2..1000)
        .map(|n| format!("{base} {n}"))
        .find(|cand| !inst.profile(cand).dir().exists())
        .unwrap_or_else(|| base.to_string())
}

/// Suggest a free mod-folder name near `name` (`name (2)`, `name (3)`, ...) for the
/// Rename option, so the prefilled value doesn't immediately collide again.
fn suggest_free_name(mods_dir: &std::path::Path, name: &str) -> String {
    if !mods_dir.join(name).exists() {
        return name.to_string();
    }
    (2..1000)
        .map(|n| format!("{name} ({n})"))
        .find(|cand| !mods_dir.join(cand).exists())
        .unwrap_or_else(|| name.to_string())
}

/// Retry the pending collision install under `policy`. Reuses the same discovery as
/// a normal install (rebuilds the FOMOD context in case the archive turns out to be
/// a FOMOD). A Rename that collides again re-opens the prompt.
fn run_collision_install(app: &mut App, policy: eidos_install::OverwritePolicy) {
    let Some(c) = app.collision.take() else { return };
    // A FOMOD reinstall: the wizard (with the user's choices) is still open in
    // app.fomod - resolve through finish_fomod, never by re-extracting with
    // default selections.
    if c.fomod {
        let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) else { return };
        // A Rename onto another existing mod re-opens the prompt BEFORE the
        // session is consumed (its drop would delete the extracted tree).
        if let eidos_install::OverwritePolicy::Rename(new) = &policy {
            if eidos_install::collision_name(&mods_dir, new).is_some() {
                app.status = Some("That name also exists - pick another.".to_string());
                app.collision = Some(c);
                return;
            }
        }
        let Some(w) = app.fomod.take() else { return };
        let archive = w.archive.clone();
        match eidos_install::finish_fomod(w.session, &w.selection, &mods_dir, &w.game_id, &w.ctx, policy)
        {
            Ok(r) => after_install(app, &r.name, r.dest, true, Some(&archive)),
            Err(e) => app.status = Some(format!("Install failed: {e}")),
        }
        return;
    }
    let (Some(inst), Some(game)) = (app.created.as_ref(), selected_game(app)) else {
        app.status = Some("Open a game instance first.".to_string());
        return;
    };
    let mods_dir = inst.mods_dir();
    let enabled_roots: Vec<std::path::PathBuf> =
        app.mods.iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
    let disabled_roots: Vec<std::path::PathBuf> =
        app.mods.iter().filter(|m| !m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
    let ctx = eidos_install::fomod_context(&game.data_path, &enabled_roots, &disabled_roots);
    let archive = c.archive.clone();
    // Reuse the tree extracted when the collision was raised; only fall back to a
    // fresh extraction if it is gone.
    let result = match c.tree.as_ref() {
        Some(tree) => eidos_install::install_extracted(
            tree,
            &c.archive,
            &mods_dir,
            &c.name,
            &c.game_id,
            policy,
            &ctx,
        ),
        None => eidos_install::install_archive_with_policy(
            &c.archive,
            &mods_dir,
            &c.name,
            &c.game_id,
            policy,
            &ctx,
        ),
    };
    match result {
        Ok(r) => after_install(app, &r.name, r.dest, r.fomod, Some(&archive)),
        Err(eidos_install::InstallError::Exists(_)) => {
            // A Rename target that also exists: keep the prompt open for another try.
            app.status = Some("That name also exists - pick another.".to_string());
            app.collision = Some(c);
        }
        Err(e) => app.status = Some(format!("Install failed: {e}")),
    }
}

/// Cap on the rows the Saves / Downloads panels render (matches the 500-entry
/// cap on the Data / Overwrite listings).
const SAVES_LIST_CAP: usize = 500;

/// Re-scan the active profile's save directory into `app.saves`.
fn load_saves(app: &mut App) {
    app.saves = match &app.created {
        Some(inst) => inst.savegames(),
        None => Vec::new(),
    };
    app.confirm_delete_save = None;
}

/// Re-scan the downloads directory into `app.downloads`, reading each archive's
/// `.meta` sidecar for its version + install status. Newest first.
fn load_downloads(app: &mut App) {
    let Some(inst) = &app.created else {
        app.downloads = Vec::new();
        app.confirm_delete_download = None;
        return;
    };
    let dir = inst.downloads_dir();
    let mut entries: Vec<(DownloadRow, std::time::SystemTime)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_string_lossy().into_owned();
            let lower = name.to_ascii_lowercase();
            let is_archive =
                lower.ends_with(".7z") || lower.ends_with(".zip") || lower.ends_with(".rar");
            if !is_archive {
                return None;
            }
            let md = e.metadata().ok()?;
            // Version + install status from the MO2-format `.meta` sidecar.
            let meta =
                eidos_instance::ModMeta::read(&PathBuf::from(format!("{}.meta", p.display())));
            let has_meta =
                std::fs::metadata(PathBuf::from(format!("{}.meta", p.display()))).is_ok();
            let state = if !has_meta {
                DownloadState::Untracked
            } else if meta.uninstalled() {
                DownloadState::Uninstalled
            } else if meta.installed() {
                DownloadState::Installed
            } else {
                DownloadState::Ready
            };
            let row = DownloadRow {
                name,
                path: p,
                size: md.len(),
                version: meta.version().unwrap_or_default(),
                mod_name: meta.mod_name(),
                state,
            };
            Some((row, md.modified().ok()?))
        })
        .collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    app.downloads = entries.into_iter().map(|(r, _)| r).take(SAVES_LIST_CAP).collect();
    app.confirm_delete_download = None;
}

/// Recursively copy the CONTENTS of `src` into `dst` (creating `dst`), MO2's
/// "Install from folder": the new mod folder mirrors the chosen directory's root
/// rather than nesting the directory inside itself.
fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Shared post-install step: give the new mod the highest priority (wins conflicts
/// by default, like MO2), reload the list, and invalidate the plugin + conflict
/// caches. modlist() is lowest-priority-first, so highest = the END of the list.
fn after_install(app: &mut App, name: &str, dest: PathBuf, fomod: bool, archive: Option<&Path>) {
    if let Some(inst) = &app.created {
        let mut ml = inst.modlist();
        ml.retain(|m| m.name != name);
        ml.push(ModEntry { name: name.to_string(), enabled: true, path: dest });
        let _ = inst.save_modlist(&ml);
        app.mods = inst.modlist();
    }
    // Flip the source archive's `.meta` status to installed (MO2 marks the
    // download), so the Downloads manager shows it as installed. Best-effort: a
    // manually dropped archive with no sidecar is a no-op.
    if let Some(a) = archive {
        let _ = eidos_nexus::mark_installed(a);
    }
    // The installed mod's tree changed (and a FOMOD may have replaced it wholesale).
    drop_files_cache(app, Some(name));
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    app.meta_cache = build_meta_cache(app);
    // Refresh the cached downloads only if they were already loaded, so the
    // status column reflects the new install without a full re-scan otherwise.
    if !app.downloads.is_empty() {
        load_downloads(app);
    }
    app.status = Some(if fomod {
        format!("Installed '{name}' via FOMOD.")
    } else {
        format!("Installed '{name}'.")
    });
}

/// The install-collision chooser card (MO2's QueryOverwriteDialog): Merge / Replace
/// / Rename / Cancel for an already-existing `mods/<name>/`.
fn collision_dialog<'a>(c: &CollisionPrompt) -> Element<'a, Message> {
    let buttons = Row::new()
        .spacing(8)
        .push(
            button(text("Merge").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionMerge)
                .style(button::secondary),
        )
        .push(
            button(text("Replace").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionReplace)
                .style(button::danger),
        );
    let rename = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Rename:").size(12.0))
        .push(
            text_input("new name", &c.rename_to)
                .on_input(Message::CollisionRenameChanged)
                .on_submit(Message::CollisionRenameCommit)
                .padding(5)
                .size(12.0)
                .width(Length::Fill),
        )
        .push(
            button(text("Install").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionRenameCommit)
                .style(button::primary),
        );
    let card = Column::new()
        .spacing(10)
        .push(text(format!("\"{}\" already exists", c.name)).size(15.0))
        .push(text("A mod with this name is already installed. Choose how to install it:").size(12.0))
        .push(buttons)
        .push(
            text("Merge installs over the existing files. Replace wipes the mod and reinstalls (your endorsement and category are kept).")
                .size(10.0),
        )
        .push(rename)
        .push(
            button(text("Cancel").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionCancel)
                .style(button::text),
        );
    container(card).max_width(460.0).padding(16).style(card_style).into()
}

// ---- Preferences modal (MO2's Settings dialog) -----------------------------

/// A wrapped game id for the default-game `pick_list` (so it has a Display label).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DefaultGameChoice {
    /// `None` = "(none)".
    id: Option<String>,
    label: String,
}

impl std::fmt::Display for DefaultGameChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// A wrapped theme for the theme `pick_list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThemeChoice(PrefTheme);

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            PrefTheme::System => "Follow system",
            PrefTheme::Light => "Light",
            PrefTheme::Dark => "Dark",
        })
    }
}

fn settings_tab_btn<'a>(label: &'a str, tab: SettingsTab, active: bool) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding([4, 10])
        .on_press(Message::SettingsTabSelected(tab))
        .style(if active { button::primary } else { button::secondary })
        .into()
}

fn settings_dialog<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .push(text("Settings").size(16.0).width(Length::Fill))
        .push(button(text("x").size(14.0)).padding([1, 8]).on_press(Message::CloseSettings).style(button::text));

    let tabs = Row::new()
        .spacing(4)
        .push(settings_tab_btn("General", SettingsTab::General, app.settings_tab == SettingsTab::General))
        .push(settings_tab_btn("Nexus", SettingsTab::Nexus, app.settings_tab == SettingsTab::Nexus));

    let body: Element<'a, Message> = match app.settings_tab {
        SettingsTab::Nexus => {
            // The validate/connect button greys out while a check is in flight.
            let connect_label = if app.api_key_validating { "Checking..." } else { "Validate & Save" };
            let mut connect = button(text(connect_label).size(12.0)).padding([5, 12]).style(button::primary);
            if !app.api_key_validating {
                connect = connect.on_press(Message::ApiKeyValidateStart);
            }
            let field = text_input("Personal API key", &app.settings_api_key)
                .on_input(Message::ApiKeyChanged)
                .on_submit(Message::ApiKeyValidateStart)
                .padding(6)
                .size(12.0)
                .width(Length::Fill);

            let mut col = Column::new()
                .spacing(8)
                .push(text("Personal Nexus Mods API key").size(13.0))
                .push(
                    text("Get it from nexusmods.com -> Account -> API Keys (Personal API Key). It is stored at ~/.config/eidos/nexus.ini and shared with the CLI.")
                        .size(10.0),
                )
                .push(Row::new().spacing(8).push(field).push(connect));

            if let Some(account) = &app.nexus_account {
                let suffix = if account.is_premium { " (Premium)" } else { "" };
                col = col.push(text(format!("Connected as {}{}.", account.name, suffix)).size(11.0));
            }
            if let Some(err) = &app.api_key_error {
                col = col.push(text(format!("Error: {err}")).size(11.0).color(Color::from_rgb8(0x8A, 0x2A, 0x2A)));
            }
            col.into()
        }
        SettingsTab::General => {
            let themes = vec![
                ThemeChoice(PrefTheme::System),
                ThemeChoice(PrefTheme::Light),
                ThemeChoice(PrefTheme::Dark),
            ];
            let theme_row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Theme").size(12.0).width(Length::Fixed(120.0)))
                .push(
                    pick_list(themes, Some(ThemeChoice(app.prefs.theme)), |c: ThemeChoice| {
                        Message::ThemeChanged(c.0)
                    })
                    .text_size(12.0)
                    .padding(6),
                );

            // Default-game dropdown: "(none)" plus every supported game.
            let mut games = vec![DefaultGameChoice { id: None, label: "(none)".to_string() }];
            for g in eidos_games::catalog() {
                games.push(DefaultGameChoice { id: Some(g.id.to_string()), label: g.name.to_string() });
            }
            let selected_game = games
                .iter()
                .find(|c| c.id == app.prefs.default_game)
                .cloned()
                .unwrap_or_else(|| DefaultGameChoice { id: None, label: "(none)".to_string() });
            let game_row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Default game").size(12.0).width(Length::Fixed(120.0)))
                .push(
                    pick_list(games, Some(selected_game), |c: DefaultGameChoice| {
                        Message::DefaultGameChanged(c.id)
                    })
                    .text_size(12.0)
                    .padding(6),
                );

            // MO2's "lock GUI while an executable runs" toggle.
            let lock_row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Run behaviour").size(12.0).width(Length::Fixed(120.0)))
                .push(
                    checkbox("Lock the window while a game or tool is running", app.prefs.lock_gui)
                        .on_toggle(Message::ToggleLockGui)
                        .size(16)
                        .text_size(12.0),
                );

            Column::new()
                .spacing(10)
                .push(theme_row)
                .push(game_row)
                .push(lock_row)
                .push(text("Saved to ~/.config/eidos/settings.ini.").size(10.0))
                .into()
        }
    };

    let card = Column::new()
        .spacing(12)
        .push(header)
        .push(tabs)
        .push(container(body).width(Length::Fill).padding(4));
    container(card).max_width(560.0).padding(16).style(card_style).into()
}

// ---- Executables editor (MO2's Modify Executables) --------------------------

fn executables_dialog<'a>(state: &ExecutablesDialogState) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .push(text("Executables").size(16.0).width(Length::Fill))
        .push(
            button(text("x").size(14.0))
                .padding([1, 8])
                .on_press(Message::CloseExecutablesDialog)
                .style(button::text),
        );

    // The tool list: user tools first (editable), then a "(defaults)" divider and
    // the read-only per-game defaults.
    let mut list = Column::new().spacing(1);
    for (i, t) in state.merged.iter().enumerate() {
        if i == state.user_len && i < state.merged.len() {
            list = list.push(text("Defaults (read-only)").size(10.0));
        }
        let selected = state.selected == Some(i);
        let label = if t.title.trim().is_empty() { "(unnamed)" } else { t.title.trim() };
        let is_default = i >= state.user_len;
        let display = if is_default { format!("{label}  (default)") } else { label.to_string() };
        list = list.push(
            button(text(display).size(12.0))
                .width(Length::Fill)
                .padding([3, 6])
                .on_press(Message::SelectExecutableTool(i))
                .style(if selected { button::primary } else { button::text }),
        );
    }
    if state.merged.is_empty() {
        list = list.push(text("No tools yet. Click Add to create one.").size(11.0));
    }
    let list_pane = container(scrollable(list).height(Length::Fill))
        .width(Length::Fixed(200.0))
        .height(Length::Fixed(280.0))
        .padding(4)
        .style(panel_style);

    let list_actions = Row::new()
        .spacing(4)
        .push(tool_btn("Add", Message::AddExecutableTool))
        .push(del_button(state))
        .push(move_button("Up", Message::MoveExecutableUp, can_move_up(state)))
        .push(move_button("Down", Message::MoveExecutableDown, can_move_down(state)));

    let left = Column::new().spacing(6).push(list_pane).push(list_actions);

    // The editor pane (only meaningful for a selected user tool).
    let editor: Element<'a, Message> = if state.selected_is_user() {
        Column::new()
            .spacing(8)
            .push(exe_field("Title", &state.title, Message::ToolTitleChanged))
            .push(exe_field_browse(
                "Executable (path)",
                &state.exe,
                Message::ToolExeChanged,
                Message::BrowseToolExe,
            ))
            .push(exe_field_browse(
                "Working dir (optional)",
                &state.workdir,
                Message::ToolWorkdirChanged,
                Message::BrowseToolWorkdir,
            ))
            .push(text("Arguments (one per line)").size(11.0))
            .push(
                text_input("", &state.args)
                    .on_input(Message::ToolArgsChanged)
                    .padding(6)
                    .size(12.0)
                    .width(Length::Fill),
            )
            .push(exe_field("Prereqs (comma-separated)", &state.prereqs, Message::ToolPrereqsChanged))
            .into()
    } else if state.selected.is_some() {
        Column::new()
            .spacing(8)
            .push(text("This is a per-game default and cannot be edited.").size(12.0))
            .push(text("Add a user tool with the same title to override it.").size(10.0))
            .into()
    } else {
        Column::new()
            .spacing(8)
            .push(text("Select a tool to edit, or click Add to create one.").size(12.0))
            .into()
    };
    let right = container(editor).width(Length::Fill).padding(4);

    let panes = Row::new().spacing(12).push(left).push(right);

    let footer = Row::new()
        .spacing(8)
        .push(Space::with_width(Length::Fill))
        .push(tool_btn("Cancel", Message::CloseExecutablesDialog))
        .push(
            button(text("Save").size(12.0))
                .padding([5, 14])
                .on_press(Message::SaveExecutablesDialog)
                .style(button::primary),
        );

    let card = Column::new().spacing(12).push(header).push(panes).push(footer);
    container(card).max_width(720.0).padding(16).style(card_style).into()
}

/// A labelled single-line field for the Executables editor.
fn exe_field<'a>(
    label: &'a str,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    Column::new()
        .spacing(2)
        .push(text(label).size(11.0))
        .push(text_input("", value).on_input(on_input).padding(6).size(12.0).width(Length::Fill))
        .into()
}

/// Like [`exe_field`] but with a Browse button that opens a native file/folder
/// picker (`browse` message), so the user can pick the path instead of typing it.
fn exe_field_browse<'a>(
    label: &'a str,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
    browse: Message,
) -> Element<'a, Message> {
    let row = Row::new()
        .spacing(4)
        .push(text_input("", value).on_input(on_input).padding(6).size(12.0).width(Length::Fill))
        .push(button(text("Browse...").size(11.0)).padding([5, 8]).on_press(browse).style(button::secondary));
    Column::new().spacing(2).push(text(label).size(11.0)).push(row).into()
}

/// The Delete button: active only when a user tool is selected.
fn del_button<'a>(state: &ExecutablesDialogState) -> Element<'a, Message> {
    let mut b = button(text("Delete").size(12.0)).padding(6).style(button::danger);
    if state.selected_is_user() {
        b = b.on_press(Message::DeleteExecutableTool);
    }
    b.into()
}

/// A reorder button, greyed when the move is not possible.
fn move_button<'a>(label: &'a str, msg: Message, enabled: bool) -> Element<'a, Message> {
    let mut b = button(text(label).size(12.0)).padding(6).style(button::secondary);
    if enabled {
        b = b.on_press(msg);
    }
    b.into()
}

fn can_move_up(state: &ExecutablesDialogState) -> bool {
    matches!(state.selected, Some(i) if i > 0 && i < state.user_len)
}

fn can_move_down(state: &ExecutablesDialogState) -> bool {
    matches!(state.selected, Some(i) if i + 1 < state.user_len)
}

// ---- About box --------------------------------------------------------------

fn about_dialog<'a>() -> Element<'a, Message> {
    let card = Column::new()
        .spacing(8)
        .push(text("Eidos").size(20.0))
        .push(text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(12.0))
        .push(
            text("A Linux-native mod manager modelled on Mod Organizer 2: isolated instances, a virtual file system over the game, FOMOD installs, LOOT sorting, and Nexus integration.")
                .size(12.0),
        )
        .push(Space::with_height(Length::Fixed(6.0)))
        .push(text("Shortcuts").size(13.0))
        .push(
            text("Ctrl+R run   ·   F5 refresh   ·   Ctrl+click multi-select   ·   Shift+click range   ·   Esc clear   ·   drag a row to reorder")
                .size(11.0),
        )
        .push(Space::with_height(Length::Fixed(6.0)))
        .push(
            button(text("Close").size(12.0))
                .padding([5, 14])
                .on_press(Message::CloseAbout)
                .style(button::primary),
        );
    container(card).max_width(440.0).padding(16).style(card_style).into()
}

/// The persistent CAP_SYS_ADMIN warning banner: the launch binary lost its file
/// capability (every rebuild wipes it), so FUSE passthrough is off and
/// script-extender DLLs may fail to image-map in-game. Shows the exact fix
/// command; F5 rechecks after running it.
fn cap_warning_banner<'a>() -> Element<'a, Message> {
    let cmd = format!(
        "sudo setcap cap_sys_admin+ep {}",
        find_eidos_binary().display()
    );
    let row = Row::new()
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .push(text("FUSE passthrough is OFF (capability lost after a rebuild): SKSE plugin DLLs may fail to load. Fix, then press F5:").size(11.0))
        .push(
            container(text(cmd).size(11.0))
                .padding([2, 8])
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
                    border: Border {
                        color: Color::from_rgb8(0xB0, 0x6A, 0x10),
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .push(Space::with_width(Length::Fill))
        .push(flat_btn("Re-check (F5)", Message::Refresh));
    container(row)
        .width(Length::Fill)
        .padding([4, 8])
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xF6, 0xE3, 0xC0))),
            border: Border {
                color: Color::from_rgb8(0xB0, 0x6A, 0x10),
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Some(Color::from_rgb8(0x6B, 0x42, 0x0A)),
            ..Default::default()
        })
        .into()
}

/// MO2's run-lock overlay card: the GUI is locked while the launched application
/// runs. Shows what is running and offers Unlock (which stops waiting but leaves
/// the game running - MO2's force-unlock never kills the process).
fn running_lock_card<'a>(run: &RunningState) -> Element<'a, Message> {
    let card = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(text("Eidos is locked while the application runs").size(18.0))
        .push(text(format!("{}  (pid {})", run.title, run.pid)).size(13.0))
        .push(
            text("It is being run through the merged mod view. Loading a save or starting a new game writes the load order; Eidos refreshes when it exits.")
                .size(11.0),
        )
        .push(Space::with_height(Length::Fixed(6.0)))
        .push(
            button(text("Unlock").size(13.0))
                .padding([6, 22])
                .on_press(Message::ForceUnlock)
                .style(button::primary),
        )
        .push(
            text("Unlock re-enables the GUI but leaves the game running.")
                .size(10.0)
                .color(Color::from_rgb8(0x6A, 0x5A, 0x40)),
        );
    container(card).max_width(470.0).padding(20).style(card_style).into()
}

/// Split a CommonMark string into plain runs and `[label](url)` links, in order.
/// LOOT's messages are markdown, and rendering them verbatim showed the bracket
/// syntax to the user; this keeps the label and hands back the URL to open.
fn split_markdown_links(text: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let bytes = text.as_bytes();
    let mut plain = String::new();
    let mut i = 0;
    while i < bytes.len() {
        // A link is `[label](url)` with no nested bracket in the label.
        if bytes[i] == b'[' {
            if let Some(close) = text[i + 1..].find(']').map(|p| i + 1 + p) {
                if text.as_bytes().get(close + 1) == Some(&b'(') {
                    if let Some(end) = text[close + 2..].find(')').map(|p| close + 2 + p) {
                        let label = &text[i + 1..close];
                        let url = &text[close + 2..end];
                        if !label.is_empty() && !url.is_empty() {
                            if !plain.is_empty() {
                                out.push((std::mem::take(&mut plain), None));
                            }
                            out.push((label.to_string(), Some(url.to_string())));
                            i = end + 1;
                            continue;
                        }
                    }
                }
            }
        }
        plain.push(text[i..].chars().next().unwrap_or('\0'));
        i += text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
    }
    if !plain.is_empty() {
        out.push((plain, None));
    }
    out
}

/// One LOOT message rendered with MO2's severity prefix and a severity colour
/// (Error red, Warning amber, Say muted). Markdown links become clickable
/// buttons that open in the browser, instead of showing raw `[label](url)`.
fn loot_message_row<'a>(m: &eidos_loot::LootMessage) -> Element<'a, Message> {
    use eidos_loot::MessageType;
    let (prefix, color) = match m.kind {
        MessageType::Error => ("Error: ", Color::from_rgb8(0x8A, 0x2A, 0x2A)),
        MessageType::Warn => ("Warning: ", Color::from_rgb8(0xB0, 0x6A, 0x10)),
        MessageType::Say => ("", Color::from_rgb8(0x4A, 0x40, 0x30)),
    };
    let parts = split_markdown_links(&m.text);
    if parts.iter().all(|(_, url)| url.is_none()) {
        return text(format!("{prefix}{}", m.text)).size(11.0).color(color).into();
    }
    let mut row = Row::new().spacing(0).align_y(iced::Alignment::Center);
    if !prefix.is_empty() {
        row = row.push(text(prefix).size(11.0).color(color));
    }
    for (label, url) in parts {
        row = match url {
            Some(u) => row.push(
                button(text(label).size(11.0).color(Color::from_rgb8(0x2B, 0x4F, 0x8A)))
                    .padding(0)
                    .on_press(Message::OpenUrl(u))
                    .style(button::text),
            ),
            None => row.push(text(label).size(11.0).color(color)),
        };
    }
    // `wrap` keeps a long advisory readable instead of running off the dialog.
    row.wrap().into()
}

/// MO2's post-sort LOOT report dialog: a summary line, then LOOT's general messages
/// and a per-plugin list of problems (missing masters, messages, dirty-plugin
/// cleaning advice). Shown after every sort, like MO2's LOOT dialog.
fn loot_report_dialog<'a>(report: &eidos_loot::LootReport) -> Element<'a, Message> {
    let summary = if report.is_empty() {
        "LOOT found no issues - your load order is clean.".to_string()
    } else {
        let mut parts: Vec<String> = Vec::new();
        if report.error_count() > 0 {
            parts.push(format!("{} error(s)", report.error_count()));
        }
        if report.warning_count() > 0 {
            parts.push(format!("{} warning(s)", report.warning_count()));
        }
        if report.missing_master_count() > 0 {
            parts.push(format!("{} with missing masters", report.missing_master_count()));
        }
        if report.dirty_count() > 0 {
            parts.push(format!("{} need cleaning", report.dirty_count()));
        }
        if parts.is_empty() {
            "LOOT messages".to_string()
        } else {
            parts.join(", ")
        }
    };

    let mut body = Column::new().spacing(12);

    if !report.general.is_empty() {
        let mut sec = Column::new().spacing(3).push(text("General messages").size(14.0));
        for m in &report.general {
            sec = sec.push(loot_message_row(m));
        }
        body = body.push(sec);
    }

    for p in &report.plugins {
        let mut sec = Column::new().spacing(2).push(text(p.name.clone()).size(13.0));
        if !p.missing_masters.is_empty() {
            sec = sec.push(
                text(format!("Missing masters: {}", p.missing_masters.join(", ")))
                    .size(11.0)
                    .color(Color::from_rgb8(0x8A, 0x2A, 0x2A)),
            );
        }
        for m in &p.messages {
            sec = sec.push(loot_message_row(m));
        }
        for d in &p.dirty {
            let util = if d.cleaning_utility.is_empty() { "?" } else { d.cleaning_utility.as_str() };
            sec = sec.push(
                text(format!(
                    "Dirty - {util} found {} ITM, {} deleted refs, {} deleted navmeshes (clean with xEdit)",
                    d.itm_count, d.deleted_reference_count, d.deleted_navmesh_count
                ))
                .size(11.0)
                .color(Color::from_rgb8(0xB0, 0x6A, 0x10)),
            );
        }
        body = body.push(sec);
    }

    let card = Column::new()
        .spacing(10)
        .push(text("LOOT report").size(20.0))
        .push(text(summary).size(12.0))
        .push(scrollable(body).height(Length::Fixed(360.0)))
        .push(
            button(text("Close").size(12.0))
                .padding([5, 14])
                .on_press(Message::CloseLootReport)
                .style(button::primary),
        );
    container(card).max_width(580.0).padding(16).style(card_style).into()
}

fn group_type_label(t: eidos_fomod::GroupType) -> &'static str {
    use eidos_fomod::GroupType::*;
    match t {
        SelectExactlyOne => "choose one",
        SelectAtMostOne => "choose at most one",
        SelectAtLeastOne => "choose at least one",
        SelectAny => "choose any",
        SelectAll => "all included",
    }
}

/// Whether the wizard's current step satisfies its group constraints: a "choose
/// one" group needs exactly one selected, a "choose at least one" needs >= 1.
fn step_valid(w: &FomodWizard) -> bool {
    use eidos_fomod::GroupType::*;
    let Some(step) = w.session.config.steps.get(w.step) else {
        return true;
    };
    let Some(sel) = w.selection.get(w.step) else {
        return true;
    };
    for (gi, group) in step.groups.iter().enumerate() {
        let count = sel.get(gi).map(|g| g.iter().filter(|&&x| x).count()).unwrap_or(0);
        let ok = match group.group_type {
            SelectExactlyOne => count == 1,
            SelectAtLeastOne => count >= 1,
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

/// The FOMOD installer wizard: the current step's groups as selectable options,
/// with Back / Cancel / Next / Install.
fn fomod_wizard_view(w: &FomodWizard) -> Element<'_, Message> {
    use eidos_fomod::PluginType;
    let config = &w.session.config;
    let total = config.steps.len();
    // Effective option types for this step (re-evaluated against the choices so far).
    let types = eidos_fomod::step_types(config, &w.selection, &w.ctx, w.step);

    let mut col = Column::new().spacing(8).padding(12);
    col = col.push(text(format!("{}  -  FOMOD installer", config.module_name)).size(20.0));
    if let Some(banner) = config.module_image.as_ref().and_then(|p| w.session.resolve(p)) {
        col = col.push(image(image::Handle::from_path(banner)).width(Length::Fixed(360.0)));
    }
    if let Some(step) = config.steps.get(w.step) {
        col = col.push(text(format!("Step {}/{}: {}", w.step + 1, total, step.name)).size(14.0));
        for (gi, group) in step.groups.iter().enumerate() {
            col = col.push(
                text(format!("{}  ({})", group.name, group_type_label(group.group_type))).size(13.0),
            );
            for (pi, plugin) in group.plugins.iter().enumerate() {
                let on = w
                    .selection
                    .get(w.step)
                    .and_then(|s| s.get(gi))
                    .and_then(|g| g.get(pi))
                    .copied()
                    .unwrap_or(false);
                let ptype = types.get(gi).and_then(|g| g.get(pi)).copied().unwrap_or(PluginType::Optional);
                let usable = ptype != PluginType::NotUsable;
                let mark = if on {
                    "[x]  "
                } else if usable {
                    "[  ]  "
                } else {
                    "[-]  "
                };
                let tag = match ptype {
                    PluginType::Required => "   - required",
                    PluginType::Recommended => "   - recommended",
                    PluginType::NotUsable => "   - not usable",
                    _ => "",
                };
                let mut b = button(text(format!("{mark}{}{tag}", plugin.name)).size(13.0))
                    .padding(4)
                    .width(Length::Fill)
                    .style(if on { button::primary } else { button::secondary });
                if usable {
                    b = b.on_press(Message::FomodToggle(gi, pi));
                }
                col = col.push(b);
                if !plugin.description.is_empty() {
                    col = col.push(text(plugin.description.clone()).size(11.0));
                }
                if let Some(img) = plugin.image.as_ref().and_then(|p| w.session.resolve(p)) {
                    col = col.push(image(image::Handle::from_path(img)).width(Length::Fixed(220.0)));
                }
            }
        }
    }
    let vis = eidos_fomod::visible_steps(config, &w.selection, &w.ctx);
    let has_prev = (0..w.step).any(|i| vis.get(i).copied().unwrap_or(false));
    let has_next = (w.step + 1..vis.len()).any(|i| vis[i]);
    let valid = step_valid(w);

    let mut nav = Row::new().spacing(8);
    if has_prev {
        nav = nav.push(tool_btn("Back", Message::FomodBack));
    }
    nav = nav.push(tool_btn("Cancel", Message::FomodCancel));
    nav = nav.push(Space::with_width(Length::Fill));
    let (label, msg) = if has_next {
        ("Next", Message::FomodNext)
    } else {
        ("Install", Message::FomodInstall)
    };
    if valid {
        nav = nav.push(tool_btn(label, msg));
    } else {
        // A constraint is unmet (e.g. a "choose one" group with nothing picked).
        nav = nav.push(button(text(label).size(13.0)).padding(6).style(button::secondary));
    }

    let mut bottom = Column::new().spacing(4);
    if !valid {
        bottom = bottom.push(text("Select the required option(s) to continue.").size(11.0));
    }
    bottom = bottom.push(nav);

    Column::new()
        .spacing(8)
        .padding(8)
        .push(scrollable(col).height(Length::Fill))
        .push(bottom)
        .into()
}

fn view(app: &App) -> Element<'_, Message> {
    if let Some(w) = &app.fomod {
        let base = fomod_wizard_view(w);
        // A reinstall collision raised from inside the wizard must be able to
        // show over it (the wizard replaces the whole view).
        if let Some(c) = &app.collision {
            let scrim =
                mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CollisionCancel);
            let dialog = container(collision_dialog(c)).center(Length::Fill);
            return Stack::new().push(base).push(scrim).push(dialog).into();
        }
        return base;
    }
    if app.screen == Screen::Main {
        return main_screen(app);
    }
    let inner = match app.screen {
        Screen::Welcome => welcome(),
        Screen::Kind => kind_screen(app),
        Screen::Game => game_screen(app),
        Screen::NameLoc => nameloc_screen(app),
        Screen::Summary => summary_screen(app),
        Screen::Main => welcome(),
    };
    container(inner).width(Length::Fill).height(Length::Fill).padding(20).into()
}

/// Keyboard subscription: surface global shortcuts and keep the live modifier state
/// in sync so a plain mod-row click can branch to Ctrl-toggle / Shift-extend.
///
/// Shortcuts only fire on the main screen, and only when no modal / inline editor is
/// stealing input, so they never clobber typing into a text field. Mirrors MO2's
/// global accelerators: F5 (Refresh) and Ctrl+R (Run).
fn subscription(app: &App) -> iced::Subscription<Message> {
    use iced::keyboard::{self, key::Named, Key};

    // Track held modifiers from every key press AND release (a release with no
    // remaining keys still carries the updated modifier set).
    let track_press = keyboard::on_key_press(|_key, mods| Some(Message::ModifiersChanged(mods)));
    let track_release =
        keyboard::on_key_release(|_key, mods| Some(Message::ModifiersChanged(mods)));

    // App shortcuts. `on_key_press` takes a plain `fn`, so it cannot read `app`;
    // the handlers themselves no-op off the main screen / while a modal is open.
    let shortcuts = keyboard::on_key_press(|key, mods| match key.as_ref() {
        Key::Named(Named::F5) => Some(Message::Refresh),
        // Ctrl+R launches the current run target (MO2's Run accelerator).
        Key::Character("r") if mods.control() => Some(Message::Run),
        Key::Named(Named::Escape) => Some(Message::ClearSelection),
        _ => None,
    });

    // The shortcut stream is gated on the main screen (the wizard/FOMOD views have
    // their own focus); modifier tracking always runs so the set is never stale.
    let mut subs = vec![track_press, track_release];
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
    {
        subs.push(shortcuts);
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
    iced::application("Eidos", update, view)
        .theme(theme)
        .subscription(subscription)
        .run_with(move || new(launch_command.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(names: &[&str]) -> Vec<ModEntry> {
        names
            .iter()
            .map(|n| ModEntry { name: n.to_string(), enabled: true, path: PathBuf::new() })
            .collect()
    }
    fn names(v: &[ModEntry]) -> Vec<&str> {
        v.iter().map(|m| m.name.as_str()).collect()
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
}
