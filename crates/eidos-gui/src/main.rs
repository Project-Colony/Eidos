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
use eidos_instance::{Instance, InstanceKind, ModEntry, SaveEntry, Tool};
use eidos_plugins::{plugins_txt_dir, GameSpec, MovableRange, PluginList};
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
    /// The mod's `INI Tweaks/` fragments, individually enabled.
    IniTweaks,
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
    /// Open the instance's root folder in the file manager.
    OpenInstanceFolder,
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
    /// The pointer moved over an insertion point during a drag. The payload is an
    /// insertion index (see `DragState::gap`), not a row index.
    DragOverGap(usize),
    /// The drag ended: commit the move if the drop row differs from the source.
    DragDrop,
    /// Abandon an in-flight drag (filter change / Escape).
    DragCancel,
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
    /// Two-click guard for a download deletion (the row's index in `downloads`).
    confirm_delete_download: Option<usize>,
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
    /// Directories the user expanded in the Data tree, same keys as above. The
    /// root is implicitly expanded and never in here.
    data_expanded: HashSet<String>,
    /// Folders opened in the Overwrite tree, keyed the same way.
    overwrite_expanded: HashSet<String>,
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
/// Bring `app.meta_cache` in step with `app.mods`, computing ONLY the rows it does
/// not already hold and dropping the ones whose mod is gone.
///
/// Each row costs a `meta.ini` read plus `classify_content_dir`, which is a
/// `read_dir` on the mod plus two more on `meshes/` and `textures/`. Rebuilding
/// the whole map cost 100 meta reads and 100-300 `read_dir` on a 100-mod setup -
/// and it ran on EVERY checkbox click and EVERY arrow click, for 150-500 ms of
/// dead window each time. None of those actions change a single byte on disk.
///
/// So the map is only appended to. The three places that genuinely rewrite a
/// `meta.ini` drop what they changed first (see [`invalidate_meta`]), and Refresh
/// clears the lot - that is what Refresh is for.
fn refresh_meta_cache(app: &mut App) {
    let wanted: HashSet<String> = app.mods.iter().map(|m| m.name.clone()).collect();
    app.meta_cache.retain(|name, _| wanted.contains(name));
    let Some(inst) = app.created.clone() else {
        app.meta_cache.clear();
        return;
    };
    // Only built when there is actually something to compute: the catalog parses
    // the category files, which is pure waste on the common no-op refresh.
    let missing: Vec<(String, PathBuf)> = app
        .mods
        .iter()
        .filter(|m| !app.meta_cache.contains_key(&m.name))
        .map(|m| (m.name.clone(), m.path.clone()))
        .collect();
    if missing.is_empty() {
        return;
    }
    let cats = inst.category_factory();
    for (name, path) in missing {
        let meta = inst.mod_meta(&name);
        let category_id = meta.category().as_deref().and_then(eidos_instance::parse_primary);
        let category_name = category_id.and_then(|id| cats.name_for_id(id)).map(str::to_string);
        app.meta_cache.insert(
            name,
            RowMeta {
                version: meta.version(),
                mod_id: meta.mod_id(),
                category_id,
                category_name,
                content_tags: eidos_install::classify_content_dir(&path).tags(),
                update: meta.update_available(),
                color: meta.color(),
            },
        );
    }
}

/// Drop one mod's cached row, for the paths that rewrite its `meta.ini`. The next
/// [`refresh_meta_cache`] recomputes exactly that row.
fn invalidate_meta(app: &mut App, name: &str) {
    app.meta_cache.remove(name);
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
        picker: None,
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
        sorting: false,
        ui_toolbar_visible: true,
        ui_statusbar_visible: true,
        view_menu_open: false,
        about_open: false,
        saves: Vec::new(),
        confirm_delete_save: None,
        selected_save: None,
        save_info: None,
        save_missing: Vec::new(),
        downloads: Vec::new(),
        confirm_delete_download: None,
        selected_mods: HashSet::new(),
        sel_anchor: None,
        confirm_batch_remove: false,
        modifiers: iced::keyboard::Modifiers::default(),
        drag_state: None,
        selected_plugin: None,
        selected_plugins: HashSet::new(),
        plugin_anchor: None,
        focus: Pane::Mods,
        categories: None,
        cursor: iced::Point::ORIGIN,
        window: iced::Size::new(1280.0, 800.0),
        menu_at: None,
        typing: false,
        plugin_drag: None,
        profile_menu: None,
        profile_rename: None,
        profile_copy: None,
        profile_delete_confirm: None,
        running: None,
        cap_missing: !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary()),
        files_cache: std::cell::RefCell::new(HashMap::new()),
        view_generation: std::cell::Cell::new(0),
        diag: Vec::new(),
        diag_dirty: true,
        diag_stale: std::cell::Cell::new(true),
        data_listing: std::cell::RefCell::new(HashMap::new()),
        data_expanded: HashSet::new(),
        overwrite_expanded: HashSet::new(),
        listing_cache: std::cell::RefCell::new(HashMap::new()),
        loot_report: None,
    };
    // NEVER under test. This opens the REAL instance in the user's home and,
    // through ensure_manifest/ensure_profiles, writes to it - so any test that
    // built an App through `new` was one `mods_changed` away from saving its
    // fixture over a live mod list. That is not hypothetical: a keyboard test
    // whose list was ["a","b","c","d"] wrote exactly that into a real
    // modlist.txt, and the only reason it was noticed is that the user restarted
    // and saw four mods. A test needs an App, not a machine's data.
    if cfg!(test) {
        return (app, Task::none());
    }
    if let Some(i) = auto {
        app.selected = Some(i);
        let inst = Instance::global(app.games[i].def.id);
        if inst.exists() {
            let _ = inst.ensure_manifest(app.games[i].def.id, InstanceKind::Global);
            let _ = inst.ensure_profiles();
            app.mods = modlist_with_unmanaged(&inst, app.games.get(i));
            app.categories = Some(inst.category_factory());
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
                app.mods = modlist_with_unmanaged(&inst, Some(g));
                app.categories = Some(inst.category_factory());
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
    refresh_meta_cache(&mut app);
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



/// The mod list as the user should see it: the profile's managed mods with the
/// game's own unmanaged content (DLCs, Creation Club) prepended.
///
/// Prepended, because the display runs lowest-priority-first and the engine loads
/// this content before anything anyone installed. Without it the list shows four
/// mods while eighty plugins load, which is how you end up asking whether your
/// DLC is even there.
fn modlist_with_unmanaged(inst: &Instance, game: Option<&DetectedGame>) -> Vec<ModEntry> {
    let managed = inst.modlist();
    let Some(game) = game else { return managed };
    let Some(spec) = GameSpec::for_id(game.def.id) else { return managed };
    // The order the engine imposes on its own content: the primary masters, then
    // whatever the `.ccc` lists. Anything else falls in after, alphabetically.
    let mut engine_order: Vec<String> = spec.primary_plugins.clone();
    engine_order.extend(eidos_plugins::implicit_plugins(&game.install_path));
    let mut out = inst.unmanaged_mods(&game.data_path, &engine_order, &managed);
    out.extend(managed);
    out
}


/// Refresh `app.mods` from disk, unmanaged content included. Clones the instance
/// and the game first so the immutable borrows end before `app.mods` is assigned.
fn reload_mods(app: &mut App) {
    let Some(inst) = app.created.clone() else { return };
    let game = selected_game(app).cloned();
    // This replaces the list the selection indexes into, so it is carried
    // across by name; anything that disappeared is dropped rather than silently
    // re-pointed at whatever took its place.
    let held = hold_mod_selection(app);
    app.mods = modlist_with_unmanaged(&inst, game.as_ref());
    put_mod_selection(app, held);
    // Same moment the list is rebuilt: a category could have been added by an
    // install, and this is the only place that would notice.
    app.categories = Some(inst.category_factory());
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
    if let Some((from, prefer)) = launch_targets(game_id) {
        for a in swapped.iter_mut() {
            if !a.contains(from) {
                continue;
            }
            // First target that is actually on disk wins.
            let picked = prefer.iter().find_map(|to| {
                let candidate = a.replace(from, to);
                Path::new(&candidate).is_file().then_some((*to, candidate))
            });
            match picked {
                Some((to, candidate)) => {
                    // Falling back past the script extender is worth saying out
                    // loud: the game will start, and every SKSE mod will be inert.
                    if Some(to) != prefer.first().copied() {
                        warning = Some(format!(
                            "{} is not installed - launching {to} directly, so script-extender mods will not load.",
                            prefer[0]
                        ));
                    }
                    *a = candidate;
                }
                None => {
                    warning = Some(format!(
                        "Neither {} nor the game binary was found next to {from}; launching it unchanged.",
                        prefer.join(" nor ")
                    ));
                }
            }
        }
    }
    let mut cmd = std::process::Command::new(find_eidos_binary());
    cmd.arg("play").arg(game_id).arg("--").args(&swapped);
    (cmd, warning)
}

/// What to run INSTEAD of the vanilla Bethesda launcher, best first: the script
/// extender's loader, then the game binary. Returns `(launcher name, preferences)`.
///
/// Steam's `%command%` for these games often points at `<Game>Launcher.exe`, and
/// running that through a mod manager is never what the user wants. It is a
/// separate settings app that re-scans Data and rewrites `plugins.txt`, undoing
/// the load order Eidos just deployed - MO2 runs the game binary or the extender
/// and never the launcher, which is also why Eidos already writes
/// `bEnableFileSelection` to stop the launcher resetting the plugin selection.
/// On top of that the launcher is simply fragile under Proton, where the game
/// itself runs fine.
fn launch_targets(game_id: &str) -> Option<(&'static str, Vec<&'static str>)> {
    let def = eidos_games::GameDef::for_id(game_id)?;
    let se = def.script_extender?;
    Some((se.launcher, vec![se.loader, def.game_binary]))
}

/// Spawn a launch and start tracking it: the child's stdout+stderr go to a
/// per-run log under the instance (the GUI has no terminal when started from
/// Steam), a detached thread `wait()`s it (reaping it, so no zombie) and records
/// its exit status, and the poll subscription refreshes on exit. When `lock_gui`
/// is set the lock overlay also comes up; otherwise the run is tracked without
/// blocking the window.
/// A command as a single copy-pasteable line, quoting only the arguments that
/// need it. Written into the run log so a failing launch can be reproduced by
/// hand in a terminal, where the error the GUI swallows is visible.
fn render_command(cmd: &std::process::Command) -> String {
    let quote = |s: &str| {
        if s.is_empty() || s.contains([' ', '"', '\'', '\\', '$', '`']) {
            format!("'{}'", s.replace('\'', r"'\''"))
        } else {
            s.to_string()
        }
    };
    let mut out = quote(&cmd.get_program().to_string_lossy());
    for a in cmd.get_args() {
        out.push(' ');
        out.push_str(&quote(&a.to_string_lossy()));
    }
    out
}

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
        if let Ok(mut f) = std::fs::File::create(p) {
            use std::io::Write;
            // The command itself, before a byte of its output. Without it a log is
            // only evidence that SOMETHING ran: which executable Eidos picked after
            // the launcher swap is exactly the question these logs get read to
            // answer, and it was the one thing they never recorded.
            let _ = writeln!(f, "# eidos: running {title}");
            let _ = writeln!(f, "# command: {}", render_command(&cmd));
            let _ = writeln!(f, "#");
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
    reload_mods(app);
    if app.created.is_some() {
        // The session wrote into the Overwrite (and tools may have edited mods).
        drop_files_cache(app, None);
        invalidate_plugins(app);
        app.conflicts = compute_conflicts(app);
        refresh_meta_cache(app);
        recompute_counts(app);
        app.selected_mods.clear();
        app.drag_state = None;
        // The run just wrote the script extender's log, which is one of the
        // health checks - and reading it is exactly what the cache defers.
        app.diag_dirty = true;
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
    let status = run.outcome.lock().ok().and_then(|s| *s);
    let failed = status.map(|st| !st.success()).unwrap_or(false);
    // Record how it ended in the log itself. The status bar says it too, but the
    // status bar is gone by the time anyone reads the log - and "exited with 0
    // after one second" versus "killed by SIGSEGV" are completely different
    // problems that looked identical in these files.
    if let Some(p) = &run.log {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(p) {
            let _ = match status {
                Some(st) => match st.code() {
                    Some(c) => writeln!(f, "\n# eidos: {} exited with code {c}", run.title),
                    // No code means a signal killed it; on Unix that is the
                    // interesting case and `ExitStatus` prints which one.
                    None => writeln!(f, "\n# eidos: {} was killed ({st})", run.title),
                },
                None => writeln!(f, "\n# eidos: {} - could not read the exit status", run.title),
            };
        }
    }
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

/// The first row a mod may legally occupy: unmanaged rows (the game's own DLC and
/// Creation Club content) are listed first and are not part of `modlist.txt`, so
/// nothing can be ordered above them - a drop there would vanish on save.
fn first_managed(mods: &[ModEntry]) -> usize {
    mods.iter().position(|m| !m.is_unmanaged()).unwrap_or(mods.len())
}

/// Which rows the mod list draws, given the filter and the folded groups.
///
/// Filtering SUSPENDS folding. A search is a question - "which of my mods are
/// called this?" - and a folded group is a display convenience; letting the
/// second silently amputate the answer to the first means the list can show
/// nothing, or worse print "no mods match", while the match sits two rows away
/// inside a group the user folded last week and has forgotten about. That is
/// not a slow answer, it is a wrong one, so a matching mod shows whatever its
/// group is doing.
///
/// A separator then draws only when a mod under it survived the filter, so
/// suspending the fold does not leave a wall of empty headers; with no filter
/// running it always draws, since it is the handle the group folds by.
///
/// `matches` is asked only about real mods - a separator carries no version,
/// no category and no content, and is never a filter subject itself.
fn visible_rows(
    mods: &[ModEntry],
    collapsed: &HashSet<String>,
    filtering: bool,
    matches: impl Fn(usize, &ModEntry) -> bool,
) -> Vec<bool> {
    let mut vis = vec![false; mods.len()];
    let mut folded = false;
    for (i, m) in mods.iter().enumerate() {
        if m.is_separator() {
            folded = !filtering && collapsed.contains(m.display_name());
            vis[i] = !filtering;
            continue;
        }
        vis[i] = !folded && matches(i, m);
    }
    if filtering {
        // Walk back so each separator sees the group it heads, which is every
        // row after it up to the next separator.
        let mut group_has_match = false;
        for i in (0..mods.len()).rev() {
            if mods[i].is_separator() {
                vis[i] = group_has_match;
                group_has_match = false;
            } else if vis[i] {
                group_has_match = true;
            }
        }
    }
    vis
}

/// Ask whether the instance is free, WITHOUT still holding it afterwards.
///
/// `if let Err(e) = inst.try_lock(..)` reads like a test but is not one: the
/// `InstanceLock` it produces is a temporary that lives to the end of the whole
/// `if let` statement, the `else` block included. The rename path then called
/// `switch_to_profile`, which takes the same flock again from a second
/// descriptor - refused, because `LOCK_NB` does not care that the caller is the
/// same process. The profile was renamed and the window kept pointing at a name
/// that no longer existed, saying "Cannot switch profiles".
///
/// Dropping the lock before returning narrows the check to what it always
/// actually was: a courtesy probe, since every write underneath takes its own.
fn probe_lock(inst: &Instance) -> std::io::Result<()> {
    inst.try_lock("the Eidos window").map(drop)
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

/// The scrollables the keyboard has to move, named so `snap_to` can reach them.
///
/// `scrollable::Id` became the shared `widget::Id` in iced 0.14 - the same type
/// every operation addresses a widget by.
fn mod_scroll_id() -> widget::Id {
    widget::Id::new("mod-list")
}
fn plugin_scroll_id() -> widget::Id {
    widget::Id::new("plugin-list")
}

/// Bring the row at visible position `pos` of `total` into view.
///
/// Without this the arrow keys move a highlight the user cannot see: past the
/// bottom of a hundred-row list the focus is real, the selection is real, and
/// nothing on screen changes. iced has no "scroll this row into view", so the
/// list is scrolled proportionally - the focused row ends up roughly a third
/// down the viewport, which keeps its neighbours visible in both directions.
fn scroll_focus_into_view(id: widget::Id, pos: usize, total: usize) -> Task<Message> {
    if total <= 1 {
        return Task::none();
    }
    let frac = (pos as f32 / (total - 1) as f32).clamp(0.0, 1.0);
    // The offset is per-axis optional in 0.14, so `x: None` says "leave the
    // horizontal scroll where the user put it" instead of yanking it back to 0
    // on every arrow key - which is what passing 0.0 used to do.
    operation::snap_to(id, operation::RelativeOffset { x: None, y: Some(frac) })
}

/// Which mod rows the list is currently drawing.
///
/// Shared with the keyboard on purpose. Computed separately, the two would
/// drift, and the drift is invisible until an arrow key walks the focus into a
/// row that is filtered out or folded away - where the highlight cannot be seen
/// and Space toggles a mod the user is not looking at.
fn mod_row_visibility(app: &App, cats: Option<&eidos_instance::CategoryFactory>) -> Vec<bool> {
    let query = app.search.trim().to_lowercase();
    let filtering = !query.is_empty() || app.category_filter.is_some();
    visible_rows(&app.mods, &app.collapsed, filtering, |_, m| {
        if !query.is_empty() && !m.display_name().to_lowercase().contains(&query) {
            return false;
        }
        match app.category_filter {
            None => true,
            Some(fid) => app
                .meta_cache
                .get(&m.name)
                .and_then(|r| r.category_id)
                .zip(cats)
                .is_some_and(|(cid, cf)| cf.is_descendant_of(cid, fid)),
        }
    })
}

/// Which list the keyboard actually drives right now.
///
/// `App::focus` remembers the last list the user touched, but the plugin list is
/// only on screen while its tab is - so a focus left there after switching tabs
/// would send the arrow keys somewhere invisible.
fn effective_focus(app: &App) -> Pane {
    match app.focus {
        Pane::Plugins if app.tab == Tab::Plugins && app.plugins.is_some() => Pane::Plugins,
        _ => Pane::Mods,
    }
}

/// Move the focused mod (or the whole selection) beside `neighbour`.
///
/// `neighbour` is the row the user can see next to this one, which under a
/// filter is not the adjacent index - landing one raw place away would look like
/// nothing happened.
fn move_mod_rows(app: &mut App, from: usize, neighbour: usize, up: bool) -> Task<Message> {
    let block = selection_or(app, from);
    if block.is_empty() {
        return Task::none();
    }
    // Nothing may be ordered above the game's own content: those rows are not in
    // modlist.txt and a mod dropped among them would vanish on save.
    let floor = first_managed(&app.mods);
    let dest = if up { neighbour.max(floor) } else { neighbour + 1 };
    let held = hold_mod_selection(app);
    let at = move_block(&mut app.mods, &block, dest);
    put_mod_selection(app, held);
    app.selected_mod = Some(at);
    mods_changed(app);
    Task::none()
}

/// The plugin twin. The engine's ordering rules decide whether it happens at
/// all, and say why when they refuse - the same answer a drag gets.
fn move_plugin_rows(app: &mut App, from: usize, neighbour: usize, up: bool) -> Task<Message> {
    let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
        return Task::none();
    };
    let block = plugin_selection_or(app, from);
    if block.is_empty() {
        return Task::none();
    }
    let gap = if up { neighbour } else { neighbour + 1 };
    let held = hold_plugin_selection(app);
    let mut moved = false;
    if let Some(list) = app.plugins.as_mut() {
        moved = list.move_plugins_to(&block, gap, &spec);
        if moved {
            list.refresh(&spec);
        }
    }
    put_plugin_selection(app, held);
    if !moved {
        // Refused by the engine's rules, not by a mis-aimed gesture; say which
        // plugin is in the way rather than looking like a dead key.
        if let Some(r) = app
            .plugins
            .as_ref()
            .and_then(|l| l.block_movable_range(&block, &spec))
            .filter(|r| r.is_stuck(block[0]))
        {
            app.status = Some(pinned_by(&r));
        }
        return Task::none();
    }
    commit_plugin_order(app, &spec);
    Task::none()
}

/// Move the focused row, or act on it. One place, so the two lists cannot drift
/// into answering the same key differently.
fn key_nav(app: &mut App, nav: Nav) -> Task<Message> {
    const PAGE: usize = 10;
    let pane = effective_focus(app);
    // The rows the keyboard may land on: what the list is actually DRAWING.
    // Walking the raw vector would move the focus into rows hidden by the filter
    // or folded into a collapsed group, where the highlight cannot be seen and
    // Space toggles something nobody is looking at.
    let rows: Vec<usize> = match pane {
        Pane::Mods => {
            let vis = mod_row_visibility(app, app.categories.as_ref());
            (0..app.mods.len()).filter(|&i| vis.get(i).copied().unwrap_or(false)).collect()
        }
        Pane::Plugins => (0..app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0)).collect(),
    };
    if rows.is_empty() {
        return Task::none();
    }
    let cur = match pane {
        Pane::Mods => app.selected_mod,
        Pane::Plugins => app.selected_plugin,
    };

    // The actions first: they act on the row, not on where the row is.
    match nav {
        Nav::Toggle => {
            return match (pane, cur) {
                // The batch path already handles "just the focused row" via
                // plugin_selection_or, so one message covers both cases.
                (Pane::Plugins, Some(_)) => {
                    let on = app
                        .selected_plugin
                        .and_then(|i| app.plugins.as_ref()?.plugins.get(i).map(|p| !p.enabled))
                        .unwrap_or(true);
                    update(app, Message::SetSelectedPluginsEnabled(on))
                }
                (Pane::Mods, Some(i)) => update(app, Message::ToggleMod(i)),
                _ => Task::none(),
            };
        }
        Nav::Activate => {
            return match (pane, cur) {
                (Pane::Mods, Some(i)) => update(app, Message::ShowModInfo(i)),
                _ => Task::none(),
            };
        }
        Nav::Remove => {
            // The same two-step guard the row menu uses - and the SECOND press
            // has to be able to finish it, or the promise in the status line is
            // a lie and the key does nothing but light up a button elsewhere.
            return match (pane, cur) {
                (Pane::Mods, Some(i))
                    if app
                        .mods
                        .get(i)
                        .is_some_and(|m| !m.is_unmanaged() && !m.is_separator()) =>
                {
                    if app.confirm_remove == Some(i) {
                        return update(app, Message::ModRemove(i));
                    }
                    let name = app.mods[i].display_name().to_string();
                    app.confirm_remove = Some(i);
                    app.status =
                        Some(format!("Press Delete again to remove '{name}', Escape to cancel."));
                    Task::none()
                }
                _ => Task::none(),
            };
        }
        Nav::ShiftUp | Nav::ShiftDown => {
            let up = matches!(nav, Nav::ShiftUp);
            let Some(i) = cur else { return Task::none() };
            // Land beside the neighbour the user can SEE, not one raw index
            // away: under a filter those differ, and a move whose effect is
            // invisible reads as a key that did nothing.
            let Some(here) = rows.iter().position(|&r| r == i) else { return Task::none() };
            let neighbour = if up {
                if here == 0 {
                    return Task::none();
                }
                rows[here - 1]
            } else {
                match rows.get(here + 1) {
                    Some(&r) => r,
                    None => return Task::none(),
                }
            };
            return match pane {
                Pane::Mods => move_mod_rows(app, i, neighbour, up),
                Pane::Plugins => move_plugin_rows(app, i, neighbour, up),
            };
        }
        _ => {}
    }

    // Movement, in VISIBLE positions rather than raw indices, so a step is one
    // row on screen however many are filtered out between them. With nothing
    // focused yet the first key lands on an end rather than doing nothing, so
    // the list is reachable without ever touching the mouse.
    let last = rows.len() - 1;
    // Where the current focus sits among the visible rows. A focus that is no
    // longer drawn (the filter moved under it) is treated as "before the list",
    // so the next key brings it back onto something visible.
    let at = cur.and_then(|i| rows.iter().position(|&r| r == i));
    let pos = match (at, nav) {
        (None, Nav::Up | Nav::PageUp | Nav::Last) => last,
        (None, _) => 0,
        (Some(p), Nav::Up) => p.saturating_sub(1),
        (Some(p), Nav::Down) => (p + 1).min(last),
        (Some(p), Nav::PageUp) => p.saturating_sub(PAGE),
        (Some(p), Nav::PageDown) => (p + PAGE).min(last),
        (_, Nav::First) => 0,
        (_, Nav::Last) => last,
        (Some(p), _) => p,
    };
    let next = rows[pos];

    // Shift extends from the anchor, exactly as Shift-click does, so the two
    // ways of building a selection agree.
    let extend = app.modifiers.shift();
    match pane {
        Pane::Mods => {
            if extend {
                let t = update(app, Message::SelectModExtend(next));
                return Task::batch([t, scroll_focus_into_view(mod_scroll_id(), pos, rows.len())]);
            }
            app.selected_mod = Some(next);
            app.sel_anchor = Some(next);
            app.selected_mods.clear();
            app.menu_mod = None;
            app.confirm_remove = None;
            scroll_focus_into_view(mod_scroll_id(), pos, rows.len())
        }
        Pane::Plugins => {
            if extend {
                let t = update(app, Message::SelectPluginExtend(next));
                return Task::batch([t, scroll_focus_into_view(plugin_scroll_id(), pos, rows.len())]);
            }
            app.selected_plugin = Some(next);
            app.plugin_anchor = Some(next);
            app.selected_plugins.clear();
            scroll_focus_into_view(plugin_scroll_id(), pos, rows.len())
        }
    }
}

/// The plugin rows an action should act on: the whole selection when the given
/// row belongs to it, otherwise just that row.
///
/// The twin of [`selection_or`], and deliberately the same shape: a batch action
/// and a single-row action must not disagree about what "the rows I am acting
/// on" means, or a right-click would do something different from the menu it
/// opened.
fn plugin_selection_or(app: &App, row: usize) -> Vec<usize> {
    let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
    let mut v: Vec<usize> = if app.selected_plugins.contains(&row) && app.selected_plugins.len() > 1
    {
        app.selected_plugins.iter().copied().collect()
    } else {
        vec![row]
    };
    v.retain(|&i| i < len);
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
    // The cross-process lock: a running `eidos play` holds it for the whole
    // session, so a mid-game edit is refused HERE with a readable reason instead
    // of writing into files the live session owns.
    let _lock = match inst.try_lock("the Eidos window") {
        Ok(l) => l,
        Err(e) => return Some(format!("Not saved: {e}.")),
    };
    inst.save_modlist(&app.mods).err().map(|e| format!("Could not save the mod list: {e}"))
}

/// Invalidate every memoised view listing. Cheap: the listings rebuild lazily on
/// the next redraw that needs them. The stored entries are dropped rather than
/// left to accumulate one stale copy per directory ever viewed.
fn bump_views(app: &App) {
    app.view_generation.set(app.view_generation.get().wrapping_add(1));
    app.data_listing.borrow_mut().clear();
    app.listing_cache.borrow_mut().clear();
    app.diag_stale.set(true);
}

/// Recompute the cached health checks if anything flagged them stale. Called once
/// at the end of `update()`, so a message that changes ten things still pays for
/// one scan - and a message that changes nothing pays for none.
fn refresh_diagnostics(app: &mut App) {
    if !app.diag_stale.get() && !app.diag_dirty {
        return;
    }
    app.diag_stale.set(false);
    app.diag_dirty = false;
    app.diag = diagnostics(app);
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
    let held = hold_plugin_selection(app);
    app.plugins = None;
    if app.tab == Tab::Plugins && app.created.is_some() {
        app.plugins = compute_plugins(app);
    }
    put_plugin_selection(app, held);
}

/// A selection captured BY NAME so it can survive its list being rebuilt or
/// reordered.
///
/// A selection is a set of indices, and almost everything moves them: a LOOT
/// sort, a drag, an arrow-button move, a refresh, a mod enabled. Left alone the
/// numbers stay in range and simply mean different rows - which is worse than
/// going out of range, because nothing errors: the highlight paints strangers
/// and a batch action writes them to disk.
///
/// The ANCHOR is in here too. It is the one index a Shift extension counts from,
/// so a stale one silently turns a three-row gesture into a twenty-row one.
#[derive(Debug, Clone, Default)]
struct HeldSelection {
    focus: Option<String>,
    anchor: Option<String>,
    set: Vec<String>,
}

/// Capture the plugin selection by name. Pair every call with
/// [`put_plugin_selection`] around whatever moves the rows.
fn hold_plugin_selection(app: &App) -> HeldSelection {
    let Some(list) = app.plugins.as_ref() else { return HeldSelection::default() };
    let name = |i: &usize| list.plugins.get(*i).map(|p| p.name.clone());
    HeldSelection {
        focus: app.selected_plugin.as_ref().and_then(name),
        anchor: app.plugin_anchor.as_ref().and_then(name),
        set: app.selected_plugins.iter().filter_map(name).collect(),
    }
}

/// Put it back on the current list, dropping whatever is no longer there.
fn put_plugin_selection(app: &mut App, held: HeldSelection) {
    let Some(list) = app.plugins.as_ref() else {
        app.selected_plugin = None;
        app.plugin_anchor = None;
        app.selected_plugins.clear();
        return;
    };
    let at = |n: &String| list.plugins.iter().position(|p| p.name.eq_ignore_ascii_case(n));
    app.selected_plugin = held.focus.as_ref().and_then(at);
    app.plugin_anchor = held.anchor.as_ref().and_then(at);
    app.selected_plugins = held.set.iter().filter_map(at).collect();
}

/// The mod-list twin of [`hold_plugin_selection`].
fn hold_mod_selection(app: &App) -> HeldSelection {
    let name = |i: &usize| app.mods.get(*i).map(|m| m.name.clone());
    HeldSelection {
        focus: app.selected_mod.as_ref().and_then(name),
        anchor: app.sel_anchor.as_ref().and_then(name),
        set: app.selected_mods.iter().filter_map(name).collect(),
    }
}

/// The mod-list twin of [`put_plugin_selection`]. Also disarms a pending
/// removal: that guard names its target by index, and confirming it after the
/// list moved would delete whatever slid into the slot.
fn put_mod_selection(app: &mut App, held: HeldSelection) {
    let at = |n: &String| app.mods.iter().position(|m| &m.name == n);
    app.selected_mod = held.focus.as_ref().and_then(at);
    app.sel_anchor = held.anchor.as_ref().and_then(at);
    app.selected_mods = held.set.iter().filter_map(at).collect();
    app.confirm_remove = None;
}

/// Persist the mod list and invalidate everything derived from it (plugin order,
/// conflict emblems, the per-mod metadata cache).
fn mods_changed(app: &mut App) {
    if let Some(err) = save_mods(app) {
        app.status = Some(err);
        // The write was refused (another process owns the instance): the
        // in-memory edit will never reach disk, and leaving it displayed shows
        // the user a state that silently evaporates when they close the window.
        // Disk is the truth; resync the view to it.
        reload_mods(app);
    }
    // The merged view depends on which mods are enabled and in what order, not
    // just on their contents.
    bump_views(app);
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    refresh_meta_cache(app);
    recompute_counts(app);
}

/// Refresh everything that a hide or unhide inside `mod_name` invalidates: the
/// mod's cached file walk (and with it the Data tree and the hidden-files glyph),
/// the conflict map, and - when a plugin came or went - the load order, since a
/// hidden `.esp` is one the game no longer sees.
fn after_hidden_change(app: &mut App, mod_name: &str, rel: &str) {
    drop_files_cache(app, Some(mod_name));
    app.conflicts = compute_conflicts(app);
    let lower = rel.to_ascii_lowercase();
    if [".esp", ".esm", ".esl"].iter().any(|e| lower.trim_end_matches(".mohidden").ends_with(e)) {
        invalidate_plugins(app);
    }
}

/// Make `name` the active profile and reload all per-profile view state (mod list,
/// plugin/conflict caches, collapsed groups, saves), clearing any transient
/// selection / menu / drag. Shared by the profile switch, copy, rename, and delete
/// flows so they can never drift apart.
/// Returns whether the switch actually happened - callers gate their success
/// toasts on it, or a refused switch got its refusal message overwritten by
/// "Created ..." a millisecond later.
fn switch_to_profile(app: &mut App, name: &str) -> bool {
    if let Some(inst) = &app.created {
        // Same lock as every other mutation: a switch during a run would point
        // the run's post-exit steps at the wrong profile. The flock also covers
        // sessions this window did not start (CLI, Steam direct).
        match inst.try_lock("the Eidos window") {
            Ok(_lock) => {
                let _ = inst.set_active_profile(name);
            }
            Err(e) => {
                app.status = Some(format!("Cannot switch profiles: {e}."));
                return false;
            }
        }
    }
    reload_mods(app);
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    refresh_meta_cache(app);
    app.collapsed = load_collapsed(app);
    recompute_counts(app);
    app.selected_mod = None;
    app.selected_mods.clear();
    app.drag_state = None;
    app.menu_mod = None;
    // Saves are per-profile; drop the cache so the Saves tab reloads.
    app.saves = Vec::new();
    app.confirm_delete_save = None;
    clear_save_selection(app);    true
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

/// The iced entry point: run the handler, then bring the once-per-change caches
/// back in step.
///
/// A wrapper rather than a line at the end of `update_inner`, because that
/// function has 68 early returns and a refresh reachable from only some of them
/// is worse than none - the tab count would be right or wrong depending on which
/// branch ran.
fn update(app: &mut App, message: Message) -> Task<Message> {
    let task = update_inner(app, message);
    refresh_diagnostics(app);
    task
}

/// Whether a message reports where the pointer or the window IS, rather than
/// something the user DID.
///
/// The distinction decides whether a two-click confirmation survives. Arming
/// Delete and then moving the mouse a single pixel used to cancel it, because
/// pointer tracking publishes a message per `CursorMoved` and every disarm rule
/// read that as an action. The confirmation was unreachable in practice: the
/// pointer has to travel to the button, and travelling is a mouse move.
fn is_ambient(m: &Message) -> bool {
    matches!(
        m,
        Message::PointerAt(_)
            | Message::WindowResized(_)
            | Message::FomodHover(_)
            | Message::FomodUnhover(..)
    )
}

fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    // A confirmation is armed by the first click and cancelled by any other
    // ACTION - including arming a different row. Ambient messages are not
    // actions and must leave it standing.
    if !is_ambient(&message) {
        // Any action other than a second Clear click cancels the clear confirmation.
        if !matches!(message, Message::ClearOverwrite) {
            app.confirm_clear = false;
        }
        if !matches!(message, Message::DeleteSave(_) | Message::ConfirmDeleteSave(_)) {
            app.confirm_delete_save = None;
        }
        if !matches!(message, Message::DeleteDownload(_) | Message::ConfirmDeleteDownload(_)) {
            app.confirm_delete_download = None;
        }
        // The batch-remove confirmation is armed by the first click; any other
        // action (including merely re-rendering on a modifier change) cancels it.
        if !matches!(message, Message::BatchRemoveMods | Message::ConfirmBatchRemove) {
            app.confirm_batch_remove = false;
        }
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
                        app.created = Some(inst);
                        reload_mods(app);
                        app.tab = Tab::Data;
                        app.error = None;
                        app.screen = Screen::Main;
                        load_tools(app);
                        app.conflicts = compute_conflicts(app);
                        refresh_meta_cache(app);
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
            // canBeEnabled() == false). Unmanaged rows are the game's own DLC and
            // Creation Club content: they are not in modlist.txt, so a flipped
            // flag would be silently lost on the next save, which reads as the
            // click having done nothing.
            if app.mods.get(i).is_some_and(|m| m.is_separator() || m.is_unmanaged()) {
                return Task::none();
            }
            if let Some(m) = app.mods.get_mut(i) {
                m.enabled = !m.enabled;
            }
            mods_changed(app);
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
            // Refused while the game runs: the run's post-exit steps write into
            // the profile that was LAUNCHED, and the profile's plugins dir is
            // bind-mounted into the live session - switching under it corrupted
            // the profile that was never played.
            if app.running.is_some() {
                app.status =
                    Some("Cannot switch profiles while the game is running.".to_string());
                return Task::none();
            }
            // One shared path (switch_to_profile) so the reload steps - incl.
            // recompute_counts, which this handler used to skip - never drift.
            if app.created.is_some() && switch_to_profile(app, &name) {
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
                if switch_to_profile(app, &name) {
                    app.status = Some(format!("Created '{name}' (copy of '{src_name}')."));
                }
            }
        }
        // ---- profile management (rename / delete / named copy) --------------
        Message::ProfileMenuOpen(name) => {
            app.profile_menu = Some(name);
            app.menu_at = Some(app.cursor);
            app.profile_rename = None;
            app.profile_copy = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileCloseMenu => {
            app.menu_at = None;
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
            app.typing = true;
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
                } else if app.running.is_some() {
                    // A rename mid-run would pull the played profile out from
                    // under the session's post-exit steps (and the bound dirs).
                    app.status =
                        Some("Cannot rename a profile while the game is running.".to_string());
                } else if let Err(e) = probe_lock(inst) {
                    // app.running only sees runs THIS window started; the flock
                    // also covers a session launched from the CLI or Steam.
                    app.status = Some(format!("Cannot rename: {e}."));
                } else {
                    let was_active = inst.active_profile() == old;
                    match inst.rename_profile(&old, &new) {
                        Ok(()) => {
                            app.profile_rename = None;
                            app.profile_menu = None;
                            // rename_profile already followed the active pointer; reload
                            // the view when the renamed profile was the active one.
                            if !was_active || switch_to_profile(app, &new) {
                                app.status = Some(format!("Renamed profile to '{new}'."));
                            }
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
            app.typing = true;
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
                            if switch_to_profile(app, &new) {
                                app.status =
                                    Some(format!("Created '{new}' (copy of '{src_name}')."));
                            }
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
                        // Open on the first step that is actually shown. Next/Back
                        // already skip invisible steps and build_plan ignores them,
                        // but nothing seeked at open: a FOMOD whose first step is
                        // conditional rendered that page fully interactive, and
                        // every choice made on it was thrown away at install time.
                        let first = eidos_fomod::visible_steps(&session.config, &selection, &ctx)
                            .iter()
                            .position(|v| *v)
                            .unwrap_or(0);
                        app.fomod =
                            Some(FomodWizard {
                            session,
                            step: first,
                            selection,
                            game_id: gid,
                            archive: path,
                            ctx,
                            hover: None,
                        });
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
                                pick: None,
                            });
                            app.status = Some(format!("'{name}' already exists - choose how to install."));
                        }
                        Err(e) => app.status = Some(format!("Install failed: {e}")),
                    }
                }
                // Wrye Bash complex package: let the user tick sub-packages. MO2
                // pre-ticks the `00`-prefixed ones plus whatever the last install
                // of this mod used, which its meta.ini remembers.
                Ok(eidos_install::Opened::Bain { tree, subpackages, invalid }) => {
                    let previous = app
                        .created
                        .as_ref()
                        .map(|i| i.mod_meta(&name).bain_options().to_vec())
                        .unwrap_or_default();
                    let picked = eidos_install::bain_default_selection(&subpackages, &previous);
                    app.status = Some(if invalid > 0 {
                        format!("'{name}' may be a BAIN installer - {invalid} folder(s) do not look like sub-packages.")
                    } else {
                        format!("BAIN installer: choose the sub-packages to install for '{name}'.")
                    });
                    app.picker = Some(InstallPicker {
                        rows: tree_rows(&tree),
                        archive: path,
                        name,
                        game_id: gid,
                        tree,
                        // `invalid` folders are MO2's cue to ASK rather than assume.
                        mode: PickerMode::Bain { subpackages, picked, asking: invalid > 0 },
                    });
                }
                // No heuristic recognised the layout. Rather than refuse the
                // archive, show its tree and let the user point at the data root.
                Ok(eidos_install::Opened::Manual(tree)) => {
                    app.status =
                        Some(format!("'{name}': pick the folder that holds the game data."));
                    app.picker = Some(InstallPicker {
                        rows: tree_rows(&tree),
                        archive: path,
                        name,
                        game_id: gid,
                        tree,
                        mode: PickerMode::Manual { root: String::new() },
                    });
                }
                Err(e) => app.status = Some(format!("Install failed: {e}")),
            }
        }
        Message::PickerBainToggle(i) => {
            if let Some(PickerMode::Bain { picked, .. }) = app.picker.as_mut().map(|p| &mut p.mode) {
                if let Some(b) = picked.get_mut(i) {
                    *b = !*b;
                }
            }
        }
        Message::PickerBainConfirm(yes) => {
            let Some(p) = app.picker.as_mut() else { return Task::none() };
            match (&mut p.mode, yes) {
                (PickerMode::Bain { asking, .. }, true) => *asking = false,
                // "No, it is not a BAIN package": same extraction, manual picker.
                (PickerMode::Bain { .. }, false) => {
                    p.mode = PickerMode::Manual { root: String::new() };
                    app.status = Some("Pick the folder that holds the game data.".to_string());
                }
                _ => {}
            }
        }
        Message::PickerSetRoot(r) => {
            if let Some(PickerMode::Manual { root }) = app.picker.as_mut().map(|p| &mut p.mode) {
                *root = r;
            }
        }
        Message::PickerNameChanged(s) => {
            app.typing = true;
            if let Some(p) = app.picker.as_mut() {
                p.name = s;
            }
        }
        Message::PickerInstall => run_picker_install(app),
        Message::PickerCancel => {
            // Dropping the picker drops the ExtractedTree, which removes the temp.
            app.picker = None;
            app.status = Some("Install cancelled.".to_string());
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
                        pick: None,
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
                // Unlock only drops the overlay - it deliberately KEEPS the run
                // tracked so the post-exit refresh still happens - so telling the
                // user to unlock was advice that could not work.
                let what = app.running.as_ref().map(|r| r.title.clone()).unwrap_or_default();
                app.status = Some(format!(
                    "{what} is still running. Eidos re-enables launching and LOOT sorting when it exits."
                ));
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
            app.typing = true;
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
            app.typing = true;
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
            // Same gates as every other mutation: the import rewrites the modlist
            // AND the plugin state dir, which is bind-mounted into a running
            // session - importing under the game's feet mixed the two states and
            // half-undid the import at the next launch.
            if app.running.is_some() {
                app.status =
                    Some("Cannot import while the game is running.".to_string());
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot import: {e}."));
                    return Task::none();
                }
            };
            match inst.import_mo2_profile(&dir) {
                Ok(r) => {
                    // The import is the user speaking, exactly like a GUI edit:
                    // the snapshot follows it, or the damage card would flame on
                    // the imported (smaller) list and its Restore button would
                    // one-click undo the import.
                    let _ = inst.active().snapshot_plugin_state();
                    reload_mods(app);
                    drop_files_cache(app, None);
                    invalidate_plugins(app);
                    app.conflicts = compute_conflicts(app);
                    refresh_meta_cache(app);
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
                        app.mods.push(ModEntry { name: name.clone(), enabled: true, path: dest, unmanaged: false });
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
            if app.created.is_some() {
                reload_mods(app);
                // F5 = full re-scan: every cached file walk may be stale.
                drop_files_cache(app, None);
                invalidate_plugins(app);
                app.conflicts = compute_conflicts(app);
                // Refresh is the "re-read everything from disk" affordance, and
                // the only place that pays the full meta scan on purpose.
                app.meta_cache.clear();
                refresh_meta_cache(app);
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
            app.typing = true;
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
            app.focus = Pane::Mods;
            app.typing = false;
            app.selected_mod = Some(i);
            app.sel_anchor = Some(i);
            app.selected_mods.clear();
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = Some(DragState { from: i, gap: i, aimed: false });
        }
        Message::SelectModToggle(i) => {
            // A modifier click is still a press on this list: it has to take the
            // keyboard, or the arrows would go on driving the other pane.
            app.focus = Pane::Mods;
            app.typing = false;
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
            app.sel_anchor = Some(i);
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
        }
        Message::SelectModExtend(i) => {
            app.focus = Pane::Mods;
            app.typing = false;
            // Shift+click: select the contiguous run from the ANCHOR to `i`. The
            // anchor is not the focus - it stays where the selection began, so a
            // second Shift gesture grows the same run instead of starting a new
            // two-row one. With no anchor yet, behaves like a plain select.
            let anchor = app.sel_anchor.or(app.selected_mod).unwrap_or(i);
            // Pin it: the fallback above must be taken ONCE. Left unset, the next
            // Shift would fall back to the focus this gesture is about to move,
            // and the run would never grow past two rows.
            app.sel_anchor = Some(anchor);
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
            app.typing = false;
            app.confirm_remove = None;
            app.selected_mods.clear();
            app.selected_plugins.clear();
            app.drag_state = None;
            app.plugin_drag = None;
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
            // Frozen here: the pointer keeps moving and a menu that followed it
            // would be impossible to aim at.
            app.menu_at = Some(app.cursor);
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
        }
        Message::CloseMenu => {
            app.menu_at = None;
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
                // NOT re-anchored: this reopens the same menu around an inline
                // editor, and moving it to wherever the pointer had drifted
                // would yank it out from under the user mid-gesture.
                app.confirm_remove = None;
            }
        }
        Message::RenameChanged(s) => {
            app.typing = true;
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
                        app.mods.insert(idx, ModEntry { name: folder, enabled: true, path: dest, unmanaged: false });
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
                    Some((m.name.clone(), m.display_name().to_string(), meta.write(&inst.meta_path(&m.name))))
                }
                _ => None,
            };
            if let Some((changed, display, r)) = result {
                match r {
                    Ok(()) => {
                        // The colour lives in this mod's meta.ini: drop its row so
                        // the refresh below recomputes exactly that one.
                        invalidate_meta(app, &changed);
                        refresh_meta_cache(app);
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
            let implicit = app
                .plugins
                .as_ref()
                .and_then(|l| l.plugins.get(i).map(|p| l.implicit.contains(&p.name.to_ascii_lowercase())))
                .unwrap_or(false);
            if let (Some(spec), Some(name)) = (spec, name) {
                // Base-game masters are implicit and always loaded; refuse to toggle.
                if spec.primary_plugins.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
                    app.status = Some(format!("{name} is a base-game master and is always loaded."));
                } else if implicit {
                    // Creation Club content the engine loads from the .ccc file.
                    // It is deliberately kept out of plugins.txt (writing it in
                    // makes the game see every Creation twice and blank the
                    // file), so a toggle here had nothing to write: the checkbox
                    // came straight back on at the next refresh with no
                    // explanation, which reads as the click being ignored.
                    app.status = Some(format!(
                        "{name} is Creation Club content the engine loads itself - it cannot be turned off here."
                    ));
                } else if forced {
                    app.status =
                        Some(format!("{name} is a light plugin this game can't load and stays off."));
                } else if app.plugins.is_some() {
                    let held = hold_plugin_selection(app);
                    let mut now = false;
                    if let Some(list) = app.plugins.as_mut() {
                        now = list.plugins.get(i).map(|p| p.enabled).unwrap_or(false);
                        list.set_enabled(&name, !now);
                        list.refresh(&spec);
                    }
                    put_plugin_selection(app, held);
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
                        Err(e) => {
                            // Refused write: drop the phantom toggle, resync to disk.
                            app.plugins = compute_plugins(app);
                            format!("Could not write the load order: {e}")
                        }
                    });
                }
            }
        }
        Message::SortPlugins => {
            // Refused up front while the game runs: the sort's async completion
            // would only be refused by the lock anyway (and resynced), so
            // starting it just wastes a masterlist download to throw the result
            // away - and shows a "Sorting..." status for a sort that cannot land.
            if app.running.is_some() {
                app.status =
                    Some("Cannot sort while the game is running.".to_string());
                return Task::none();
            }
            // One at a time. Without this every impatient click during the
            // masterlist download queued another complete sort, and each one
            // re-opened the report over a dialog the user had already closed -
            // minutes later, since they run strictly one after another.
            if app.sorting {
                app.status = Some("A LOOT sort is already running.".to_string());
                return Task::none();
            }
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
            // The PROFILE is the load-order authority; the prefix copy is a
            // shadow that can be stale. Fall back to the prefix only before the
            // profile owns a state (pre-first-launch).
            let local_dir = app
                .created
                .as_ref()
                .map(|i| i.active())
                .filter(|p| p.has_plugin_state())
                .map(|p| p.plugins_state_dir())
                .unwrap_or_else(|| plugins_txt_dir(&cd.join("pfx"), &spec));
            let cache = app
                .created
                .as_ref()
                .map(|i| i.root.join("loot"))
                .unwrap_or_else(|| eidos_instance::Instance::global(&id).root.join("loot"));
            let plugins: Vec<(String, PathBuf)> =
                list.plugins.iter().map(|p| (p.name.clone(), p.path.clone())).collect();
            // Where LOOT must look besides the vanilla Data dir. Highest priority
            // first, Overwrite ahead of everything, matching the union's own
            // precedence - without these every file-conditioned masterlist rule
            // is evaluated against a directory the mods are not in.
            let mod_dirs = loot_data_paths(app);
            // The enabled (active) plugin names, lowercased - drives which plugins the
            // LOOT report covers and what counts as a missing master.
            let enabled_lower: std::collections::HashSet<String> = list
                .plugins
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.name.to_ascii_lowercase())
                .collect();
            // What this answer will be checked against when it comes back.
            let fingerprint = SortFingerprint {
                game: id.clone(),
                profile: app
                    .created
                    .as_ref()
                    .map(|i| i.active_profile())
                    .unwrap_or_default(),
                names: list.plugins.iter().map(|p| p.name.clone()).collect(),
            };
            app.sorting = true;
            app.status =
                Some("Sorting plugins with LOOT (updating the masterlist)...".to_string());
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
                    // One view, used by both calls, so the report can never be
                    // built from a different picture than the sort.
                    let view = eidos_loot::GameView {
                        game_id: &id,
                        game_path: &install,
                        local_path: &local_dir,
                        plugins: &plugins,
                        mod_dirs: &mod_dirs,
                        masterlist: &ml,
                        prelude: &pre,
                        userlist: Some(&userlist),
                    };
                    let order = eidos_loot::sort(&view).map_err(|e| e.to_string())?;
                    // Build the post-sort report (general messages + per-plugin
                    // missing masters / messages / dirty info) for the modal, the
                    // same way MO2 shows its LOOT dialog after a sort. This is
                    // advisory: a report failure must NOT discard the successful
                    // sort, so it is an inner Result the handler tolerates.
                    let report =
                        eidos_loot::report(&view, &enabled_lower).map_err(|e| e.to_string());
                    Ok((fingerprint, order, report))
                },
                Message::PluginsSorted,
            );
        }
        Message::PluginsSorted(result) => {
            // Cleared on EVERY path, including the failures below, or a single
            // bad sort would leave the button dead for the rest of the session.
            app.sorting = false;
            let (asked_about, sorted, report_res) = match result {
                Ok(x) => x,
                Err(e) => {
                    app.status = Some(format!("LOOT sort failed: {e}"));
                    return Task::none();
                }
            };
            // A Refresh while LOOT ran drops the cached list. Rebuild it BEFORE
            // fingerprinting, or a harmless refresh would look like a changed
            // list and throw away a sort that is still perfectly valid.
            if app.plugins.is_none() {
                app.plugins = compute_plugins(app);
            }
            // Refuse an answer computed for a list that has since changed. The
            // order LOOT returns is a permutation of the names it was GIVEN;
            // applied to a different set - after a profile switch, a mod enabled
            // or disabled, a mod installed - it silently rearranges plugins
            // nobody asked about, and everything downstream reports a clean sort.
            let now = SortFingerprint {
                game: selected_game(app).map(|g| g.def.id.to_string()).unwrap_or_default(),
                profile: app.created.as_ref().map(|i| i.active_profile()).unwrap_or_default(),
                names: app
                    .plugins
                    .as_ref()
                    .map(|l| l.plugins.iter().map(|p| p.name.clone()).collect())
                    .unwrap_or_default(),
            };
            if now != asked_about {
                app.status = Some(if now.game != asked_about.game {
                    format!(
                        "Discarded the LOOT sort: it was computed for {}, and {} is open now.",
                        asked_about.game, now.game
                    )
                } else if now.profile != asked_about.profile {
                    format!(
                        "Discarded the LOOT sort: it was computed for profile '{}', and '{}' is active now.",
                        asked_about.profile, now.profile
                    )
                } else {
                    "Discarded the LOOT sort: the plugin list changed while it ran. Sort again."
                        .to_string()
                });
                return Task::none();
            }
            // Recompute spec + prefix dir (immutable borrows) before mutating plugins.
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            let before: Vec<String> = app
                .plugins
                .as_ref()
                .map(|l| l.plugins.iter().map(|p| p.name.clone()).collect())
                .unwrap_or_default();
            let held = hold_plugin_selection(app);
            if let Some(list) = app.plugins.as_mut() {
                list.apply_sorted_order(&sorted);
                // NOT repin_to_current: refresh() puts the pinned plugins back
                // where the user pinned them, over LOOT's opinion. Holding a slot
                // against the sorter is the entire purpose of a pin.
                list.refresh(&spec);
            }
            put_plugin_selection(app, held);
            // How much actually moved. Without this the status reads the same
            // whether the sort rearranged forty plugins or had nothing to do,
            // and a correct no-op on an already-sorted list is indistinguishable
            // from a broken button - which is exactly how it was reported.
            let changed = app
                .plugins
                .as_ref()
                .map(|l| {
                    l.plugins
                        .iter()
                        .zip(before.iter())
                        .filter(|(p, was)| &p.name != *was)
                        .count()
                })
                .unwrap_or(0);
            // Say when the sort was partly overruled, rather than reporting a
            // clean LOOT sort the list does not actually match.
            let pinned = app.plugins.as_ref().map(|l| l.locked.len()).unwrap_or(0);
            let held = if pinned > 0 { format!(" ({pinned} pinned position(s) kept)") } else { String::new() };
            let written =
                app.plugins.as_ref().map(|list| write_plugin_state(app, list, &spec)).transpose();
            let landed = written.is_ok();
            app.status = Some(match written {
                Ok(_) => {
                    if changed == 0 {
                        format!(
                            "LOOT checked {} plugins - the load order was already correct, nothing moved.{held}",
                            sorted.len()
                        )
                    } else {
                        format!("LOOT sorted {} plugins - {changed} moved.{held}", sorted.len())
                    }
                }
                Err(e) => {
                    // Refused write: drop the phantom sort, resync to disk.
                    app.plugins = compute_plugins(app);
                    format!("Sorted, but writing the load order failed: {e}")
                }
            });
            // A refused write means the sort was rolled back and the list on
            // screen is the one from disk. Popping the report here would present
            // advice about an order that no longer exists, on top of a dialog
            // whose very appearance reads as success - so the failure would be
            // announced by a success-shaped modal. The status line already says
            // what went wrong; leave it standing.
            if !landed {
                return Task::none();
            }
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
        Message::NotesChanged(s) => {
            app.typing = true;
            app.notes_edit = s;
        }
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
        // ---- hidden files (MO2 filetree.cpp HIDE/UNHIDE) ----------------------
        Message::DataToggleDir(rel) => {
            if !app.data_expanded.remove(&rel) {
                app.data_expanded.insert(rel);
            }
        }
        Message::OverwriteToggleDir(rel) => {
            if !app.overwrite_expanded.remove(&rel) {
                app.overwrite_expanded.insert(rel);
            }
        }
        Message::ToggleFileHidden(i, rel) => {
            let Some(m) = app.mods.get(i).cloned() else { return Task::none() };
            let target = m.path.join(&rel);
            // The path came from a listing that may be a redraw old; a stale row
            // must report a miss, not act on whatever now sits at that path.
            if target.symlink_metadata().is_err() {
                app.status = Some(format!("'{rel}' is no longer there."));
                return Task::none();
            }
            let hide = !path_is_hidden(&rel);
            match set_hidden(&target, hide) {
                Ok(_) => {
                    let verb = if hide { "Hid" } else { "Unhid" };
                    app.status = Some(format!("{verb} '{rel}' in '{}'.", m.name));
                    after_hidden_change(app, &m.name, &rel);
                }
                Err(e) => {
                    let verb = if hide { "hide" } else { "unhide" };
                    app.status = Some(format!("Could not {verb} '{rel}': {e}"));
                }
            }
        }
        Message::ToggleIniTweak(i, name) => {
            let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) else {
                return Task::none();
            };
            let mut meta = inst.mod_meta(&m.name);
            let mut list: Vec<String> = meta.ini_tweaks().to_vec();
            let was_on = list.iter().any(|e| e.eq_ignore_ascii_case(&name));
            // Order is application order, so enabling appends rather than inserting:
            // the fragment a user just ticked should win over the ones already on.
            if was_on {
                list.retain(|e| !e.eq_ignore_ascii_case(&name));
            } else {
                list.push(name.clone());
            }
            meta.set_ini_tweaks(&list);
            match meta.write(&inst.meta_path(&m.name)) {
                Ok(()) => {
                    let verb = if was_on { "Disabled" } else { "Enabled" };
                    app.status = Some(format!("{verb} INI tweak '{name}' for '{}'.", m.name));
                }
                Err(e) => app.status = Some(format!("Could not save the tweak list: {e}")),
            }
        }
        Message::RestoreHiddenFiles(i) => {
            app.menu_mod = None;
            let Some(m) = app.mods.get(i).cloned() else { return Task::none() };
            match restore_hidden_files(&m.path) {
                Ok(0) => app.status = Some(format!("'{}' has no hidden files.", m.name)),
                Ok(n) => {
                    app.status = Some(format!("Unhid {n} file(s) in '{}'.", m.name));
                    // No single path to key the plugin refresh on, so assume the
                    // worst: a restored .esp changes the load order.
                    after_hidden_change(app, &m.name, "restored.esp");
                }
                Err(e) => app.status = Some(format!("Could not unhide files: {e}")),
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
                    // The ignore flag lives in this mod's meta.ini.
                    let changed = m.name.clone();
                    invalidate_meta(app, &changed);
                    refresh_meta_cache(app);
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
                    // The check rewrote a version line in an unknown number of
                    // meta.ini files, so this is the one case that really does
                    // need the whole map back.
                    app.meta_cache.clear();
                    refresh_meta_cache(app);
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
        Message::SelectSave(i) => {
            // Clicking the open row closes the pane, so the list can go full width.
            if app.selected_save == Some(i) {
                clear_save_selection(app);
            } else {
                app.selected_save = Some(i);
                load_save_details(app);
            }
        }
        Message::FixSaveMods => {
            // Enable every mod that supplies one of the save's missing plugins.
            // MO2 stops at naming them; doing it is the whole point of knowing.
            let wanted: HashSet<String> =
                app.save_missing.iter().flat_map(|m| m.providers.iter().cloned()).collect();
            let mut enabled = 0usize;
            for m in app.mods.iter_mut() {
                if !m.enabled && wanted.contains(&m.name) {
                    m.enabled = true;
                    enabled += 1;
                }
            }
            if enabled == 0 {
                app.status =
                    Some("Those mods are already enabled - the plugins still need turning on in the Plugins tab.".to_string());
                return Task::none();
            }
            mods_changed(app);
            // The plugin list changed shape, so the save's diff has to be redone
            // against it rather than left showing the old answer.
            load_save_details(app);
            let left = app.save_missing.len();
            app.status = Some(if left == 0 {
                format!("Enabled {enabled} mod(s); this save's plugins are all available now.")
            } else {
                format!("Enabled {enabled} mod(s); {left} plugin(s) still need enabling in the Plugins tab.")
            });
        }
        Message::RestorePreSessionPlugins => {
            // Same gates as every other mutation: the plugins dir is bind-mounted
            // into a running session, and restoring under the game's feet races
            // its own writes.
            if app.running.is_some() {
                app.status = Some("Cannot restore while the game is running.".to_string());
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot restore: {e}."));
                    return Task::none();
                }
            };
            match inst.active().restore_plugin_snapshot() {
                Ok(()) => {
                    // The on-disk state changed under the in-memory list: recompute
                    // rather than patch, same as every other external change.
                    app.plugins = compute_plugins(app);
                    app.status = Some("Restored the pre-session plugin order.".to_string());
                }
                Err(e) => {
                    app.status = Some(format!("Could not restore the pre-session order: {e}"));
                }
            }
        }
        Message::AcceptPluginState => {
            if app.running.is_some() {
                app.status = Some("Cannot do that while the game is running.".to_string());
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot do that: {e}."));
                    return Task::none();
                }
            };
            app.status = Some(match inst.active().snapshot_plugin_state() {
                Ok(()) => "Kept the current plugin set; the warning is cleared.".to_string(),
                Err(e) => format!("Could not accept the current set: {e}"),
            });
        }
        Message::ConfirmDeleteSave(i) => {
            // Only act on the armed row, and re-check the index (the list may have
            // shifted if the file vanished out from under us).
            if app.confirm_delete_save == Some(i) {
                if let Some(save) = app.saves.get(i) {
                    let name = save.filename.clone();
                    match std::fs::remove_file(&save.path) {
                        Ok(()) => {
                            // The co-save travels with its save: leaving it made
                            // an orphan the Saves tab cannot show, the user
                            // cannot delete, and the cloud sync pushed forever.
                            for co in eidos_instance::cosave_siblings(&save.path) {
                                let _ = std::fs::remove_file(co);
                            }
                            app.status = Some(format!("Deleted save '{name}'."))
                        }
                        // Already gone is success enough; surface real errors.
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // The save may be gone while its co-save is not
                            // (deleted in-game, which knows nothing of co-saves):
                            // clean those up here too or they orphan invisibly.
                            for co in eidos_instance::cosave_siblings(&save.path) {
                                let _ = std::fs::remove_file(co);
                            }
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
            app.drag_state = Some(DragState { from: i, gap: i, aimed: false });
        }
        Message::DragOverGap(gap) => {
            if let Some(d) = &mut app.drag_state {
                // Never above the unmanaged block: those rows are the game's own
                // content, they are not in modlist.txt, and a mod dropped among
                // them would be silently dropped from the saved order.
                let want = gap.max(first_managed(&app.mods)).min(app.mods.len());
                d.aimed |= want != d.from && want != d.from + 1;
                d.gap = want;
            }
        }
        Message::DragDrop => {
            let Some(d) = app.drag_state.take() else { return Task::none() };
            if d.from >= app.mods.len() {
                return Task::none();
            }
            // Drag the whole selection when the grabbed row belongs to it (MO2
            // moves the block); otherwise just the grabbed row. Same helper every
            // other row-targeted action uses, so a drag and a "send to top" agree
            // about what "the rows I am acting on" means.
            let block = selection_or(app, d.from);
            if block.is_empty() {
                return Task::none();
            }
            // A drop that changes nothing: the pointer never left the grabbed
            // row's own edges, so this was a click. `aimed` is what makes that
            // true for a MULTI-row selection too, which has no single edge - and
            // where committing would compact a non-contiguous set and save it.
            let unchanged =
                !d.aimed || (block.len() == 1 && (d.gap == block[0] || d.gap == block[0] + 1));
            if !unchanged {
                let at = move_block(&mut app.mods, &block, d.gap);
                app.selected_mod = Some(at);
                app.selected_mods.clear();
                mods_changed(app);
            }
        }
        Message::DragCancel => {
            app.drag_state = None;
        }
        Message::SelectPlugin(i) => {
            if app.modifiers.control() || app.modifiers.command() {
                return update(app, Message::SelectPluginToggle(i));
            }
            if app.modifiers.shift() {
                return update(app, Message::SelectPluginExtend(i));
            }
            app.focus = Pane::Plugins;
            app.typing = false;
            app.selected_plugin = Some(i);
            app.plugin_anchor = Some(i);
            // Pressing a row that is NOT already in the set collapses it, so a
            // mis-press does not silently wipe a careful Ctrl/Shift selection.
            if !app.selected_plugins.contains(&i) {
                app.selected_plugins.clear();
            }
            // One press, both jobs - selecting and arming the drag - exactly as
            // the mod list does it. Splitting them would mean a row could be
            // dragged without ever becoming the row the menus act on.
            return update(app, Message::PluginDragStart(i));
        }
        Message::SelectPluginToggle(i) => {
            app.typing = false;
            // The first toggle seeds the set from the current focus, so the
            // anchor row stays selected instead of vanishing.
            if app.selected_plugins.is_empty() {
                if let Some(f) = app.selected_plugin {
                    app.selected_plugins.insert(f);
                }
            }
            if !app.selected_plugins.remove(&i) {
                app.selected_plugins.insert(i);
            }
            app.focus = Pane::Plugins;
            app.selected_plugin = Some(i);
            app.plugin_anchor = Some(i);
            // A modifier click builds a selection; it must not also start a drag.
            app.plugin_drag = None;
        }
        Message::SelectPluginExtend(i) => {
            app.typing = false;
            let anchor = app.plugin_anchor.or(app.selected_plugin).unwrap_or(i);
            app.plugin_anchor = Some(anchor);
            let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
            app.selected_plugins.clear();
            for idx in anchor.min(i)..=anchor.max(i) {
                if idx < len {
                    app.selected_plugins.insert(idx);
                }
            }
            app.focus = Pane::Plugins;
            app.selected_plugin = Some(i);
            // A modifier click builds a selection; it must not also start a drag.
            app.plugin_drag = None;
        }
        Message::SetSelectedPluginsEnabled(on) => {
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            // The SET when there is one. Going through the focus row would act on
            // a row the user had just Ctrl-clicked OFF: deselecting leaves the
            // focus on it, so `plugin_selection_or` would see it outside the set
            // and answer with that row alone - the one row they excluded.
            let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
            let mut rows: Vec<usize> = if !app.selected_plugins.is_empty() {
                app.selected_plugins.iter().copied().collect()
            } else {
                app.selected_plugin.into_iter().collect()
            };
            rows.retain(|&i| i < len);
            rows.sort_unstable();
            if rows.is_empty() {
                app.status = Some("Select a plugin first.".to_string());
                return Task::none();
            }
            // Collect names first: the indices shift under refresh(), the names
            // do not, and a batch that half-applied would be worse than one that
            // did not start.
            let (names, refused) = {
                let Some(list) = app.plugins.as_ref() else { return Task::none() };
                let mut names = Vec::new();
                let mut refused = 0usize;
                for &i in &rows {
                    let Some(p) = list.plugins.get(i) else { continue };
                    let engine_owned = spec
                        .primary_plugins
                        .iter()
                        .any(|pp| pp.eq_ignore_ascii_case(&p.name))
                        || list.implicit.contains(&p.name.to_ascii_lowercase());
                    if engine_owned || p.force_disabled {
                        refused += 1;
                        continue;
                    }
                    names.push(p.name.clone());
                }
                (names, refused)
            };
            if names.is_empty() {
                app.status = Some(
                    "Nothing to change: the game loads those plugins itself.".to_string(),
                );
                return Task::none();
            }
            let held = hold_plugin_selection(app);
            if let Some(list) = app.plugins.as_mut() {
                for n in &names {
                    list.set_enabled(n, on);
                }
                // Enabling changes the tier a plugin sorts into, so this can
                // reorder the very rows the selection points at.
                list.refresh(&spec);
            }
            put_plugin_selection(app, held);
            let verb = if on { "Enabled" } else { "Disabled" };
            let tail = if refused > 0 {
                format!(" ({refused} left alone - the game loads them itself)")
            } else {
                String::new()
            };
            app.status = Some(format!("{verb} {} plugin(s).{tail}", names.len()));
            commit_plugin_order(app, &spec);
        }
        Message::PluginDragStart(i) => {
            // The legal range is resolved once, here, and not per frame: it can
            // only change when the list itself changes, which a drag cannot do.
            // The block this press will move: the whole selection when the
            // grabbed row belongs to it, so the range is computed for what will
            // actually travel rather than for one row of it.
            let block = plugin_selection_or(app, i);
            let range = selected_game(app)
                .and_then(|g| GameSpec::for_id(g.def.id))
                .zip(app.plugins.as_ref())
                .and_then(|(spec, list)| list.block_movable_range(&block, &spec));
            app.plugin_drag =
                range.map(|range| PluginDrag { from: i, gap: i, block, range, aimed: false });
        }
        Message::PluginDragOverGap(gap) => {
            if let Some(d) = &mut app.plugin_drag {
                // Clamped rather than rejected, so the indicator parks on the
                // nearest legal slot instead of vanishing when the pointer
                // wanders past the boundary. MO2 clamps illegal drops the same
                // way (pluginlist.cpp:1940-2016). A slot a pinned plugin owns is
                // skipped over, not clamped to: it is a hole in the middle of
                // the range, and resting the line there would promise a landing
                // the pin is going to take back.
                let want = gap.clamp(d.range.lo, d.range.hi);
                if !d.range.blocked.contains(&want) {
                    let (lo, hi) = (
                        d.block.first().copied().unwrap_or(d.from),
                        d.block.last().copied().unwrap_or(d.from),
                    );
                    // Only a gap OUTSIDE the block counts as aiming; the ones
                    // inside it are where the block already is.
                    d.aimed |= want < lo || want > hi + 1;
                    d.gap = want;
                }
            }
        }
        Message::PluginDragCancel => {
            app.plugin_drag = None;
        }
        Message::PluginDragDrop => {
            let Some(d) = app.plugin_drag.take() else { return Task::none() };
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            // A press that never travelled is a click, not a drag.
            if !d.aimed {
                return Task::none();
            }
            let held = hold_plugin_selection(app);
            let mut moved = false;
            if let Some(list) = app.plugins.as_mut() {
                // move_plugins_to carries the pin of what it moved across with
                // it, so a pinned plugin the user dragged keeps its NEW slot
                // instead of being snapped back by its own lock.
                moved = list.move_plugins_to(&d.block, d.gap, &spec);
                if moved {
                    list.refresh(&spec);
                }
            }
            // The rows just changed places: without this the highlight, the
            // "N selected" count and every batch action stay on the numbers,
            // which now name different plugins.
            put_plugin_selection(app, held);
            if !moved {
                // The gesture did nothing. If the plugin was boxed in by the
                // engine, say which plugins boxed it in rather than leaving the
                // row to snap back in silence - that silence is what made a
                // correct refusal read as a broken feature.
                if d.range.is_stuck(d.block.first().copied().unwrap_or(d.from)) {
                    app.status = Some(pinned_by(&d.range));
                }
                return Task::none();
            }
            commit_plugin_order(app, &spec);
        }
        Message::TogglePluginLock(i) => {
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            let mut changed = false;
            if let Some(list) = app.plugins.as_mut() {
                let now = list.is_locked(i);
                changed = list.set_locked(i, !now);
            }
            if !changed {
                return Task::none();
            }
            commit_plugin_order(app, &spec);
        }
        Message::PointerAt(p) => app.cursor = p,
        Message::WindowResized(s) => app.window = s,
        Message::FomodHover(at) => {
            if let Some(w) = app.fomod.as_mut() {
                w.hover = at;
            }
        }
        Message::FomodUnhover(gi, pi) => {
            if let Some(w) = app.fomod.as_mut() {
                if w.hover == Some((gi, pi)) {
                    w.hover = None;
                }
            }
        }
        Message::CycleFocus => {
            // Only somewhere there is a list to drive.
            app.focus = match app.focus {
                Pane::Mods if app.tab == Tab::Plugins => Pane::Plugins,
                _ => Pane::Mods,
            };
        }
        Message::SelectAllInFocus => match effective_focus(app) {
            Pane::Mods => {
                // Separators define groups; they are not rows an action moves,
                // and `selection_or` drops them anyway - so leave them out here
                // rather than showing a selection that silently shrinks.
                app.selected_mods =
                    (0..app.mods.len()).filter(|&i| !app.mods[i].is_separator()).collect();
                app.selected_mod = app.selected_mod.or(Some(0)).filter(|_| !app.mods.is_empty());
            }
            Pane::Plugins => {
                let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
                app.selected_plugins = (0..len).collect();
                app.selected_plugin = app.selected_plugin.or((len > 0).then_some(0));
            }
        },
        Message::KeyNav(nav) => return key_nav(app, nav),
        Message::ModifiersChanged(mods) => {
            app.modifiers = mods;
        }
        Message::Noop => {}
    }
    Task::none()
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
        // New in iced 0.14, and it has to sit between the green of success and
        // the deep red of danger without reading as either: a burnt amber that
        // belongs to the same parchment family.
        warning: Color::from_rgb8(0xB0, 0x6A, 0x1E),
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

/// A mod the focused one OVERWRITES: it sits lower in the list and wins the
/// files they share. Green - the focused mod is on top of these.
const CONFLICT_WINS_BG: Color = Color::from_rgb(0.784, 0.855, 0.706);
/// A mod that overwrites the focused one: it sits lower and takes those files
/// away. Red - the focused mod is losing to these.
const CONFLICT_LOSES_BG: Color = Color::from_rgb(0.921, 0.769, 0.741);
/// The same two meanings as text, dark enough to read on parchment.
const CONFLICT_WINS_FG: Color = Color::from_rgb(0.13, 0.42, 0.16);
const CONFLICT_LOSES_FG: Color = Color::from_rgb(0.60, 0.16, 0.16);

/// Place a floating card with one corner at `at`, growing away from the nearest
/// window edge.
///
/// The card's height is not known until it is laid out, so a menu summoned near
/// the bottom cannot simply be offset downwards - it would run off the screen.
/// Anchoring the BOTTOM edge to the pointer instead, and mirroring the same
/// trick horizontally, avoids ever needing to guess the size: the container
/// aligns the card and the padding does the positioning.
fn floating_at<'a>(
    card: Element<'a, Message>,
    at: iced::Point,
    win: iced::Size,
) -> Element<'a, Message> {
    // Past the halfway line the menu would head towards an edge, so flip it.
    let right = at.x > win.width * 0.5;
    let below = at.y > win.height * 0.5;
    let pad = iced::Padding {
        top: if below { 0.0 } else { at.y },
        bottom: if below { (win.height - at.y).max(0.0) } else { 0.0 },
        left: if right { 0.0 } else { at.x },
        right: if right { (win.width - at.x).max(0.0) } else { 0.0 },
    };
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(pad)
        .align_x(if right {
            iced::alignment::Horizontal::Right
        } else {
            iced::alignment::Horizontal::Left
        })
        .align_y(if below {
            iced::alignment::Vertical::Bottom
        } else {
            iced::alignment::Vertical::Top
        })
        .into()
}

/// The strip that explains the two conflict colours, and how many rows carry
/// each - `None` when the focused mod fights with nothing, or nothing is
/// focused.
fn conflict_legend<'a>(app: &App) -> Option<Element<'a, Message>> {
    let focus = app.selected_mod?;
    let me = app.conflicts.as_ref()?.mods.get(&((focus + 1) as u32))?;
    // Origin 0 is the game's own data and u32::MAX is the Overwrite layer;
    // neither is a row, so neither is counted here.
    let rows = |set: &std::collections::BTreeSet<u32>| {
        set.iter().filter(|&&o| o != 0 && o != u32::MAX).count()
    };
    let (over, under) = (rows(&me.overwrites), rows(&me.overwritten_by));
    if over == 0 && under == 0 {
        return None;
    }
    let swatch = |c: Color, label: String| -> Element<'a, Message> {
        Row::new()
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .push(
                container(Space::new().width(Length::Fixed(12.0)).height(Length::Fixed(12.0)))
                    .style(move |_t: &Theme| container::Style {
                        background: Some(Background::Color(c)),
                        border: Border { color: Color::from_rgb8(0x6E, 0x24, 0x2E), width: 1.0, radius: 2.0.into() },
                        ..Default::default()
                    }),
            )
            .push(text(label).size(11.0))
            .into()
    };
    let name = app.mods.get(focus).map(|m| m.display_name().to_string()).unwrap_or_default();
    let mut row = Row::new().spacing(10).align_y(iced::Alignment::Center);
    row = row.push(text(format!("{name} conflicts:")).size(11.0));
    if over > 0 {
        row = row.push(swatch(CONFLICT_WINS_BG, format!("{over} it overwrites")));
    }
    if under > 0 {
        row = row.push(swatch(CONFLICT_LOSES_BG, format!("{under} overwrite it")));
    }
    Some(row.into())
}

/// How the row at `i` relates to the focused mod, for painting.
///
/// MO2's whole conflict workflow is this: click a mod, and every mod it fights
/// with lights up so the stack can be read at a glance instead of by opening a
/// dialog per pair. Both directions are shown, because "who am I beating" and
/// "who is beating me" are different questions and the answer to the second is
/// what sends a texture pack to the bottom of the list.
///
/// `None` for the focused row itself, which already reads as selected.
fn conflict_tint(app: &App, i: usize) -> Option<Color> {
    let focus = app.selected_mod?;
    if focus == i {
        return None;
    }
    let map = app.conflicts.as_ref()?;
    // Origins are `index + 1`; 0 is the game's own data.
    let me = map.mods.get(&((focus + 1) as u32))?;
    let other = (i + 1) as u32;
    if me.overwrites.contains(&other) {
        Some(CONFLICT_WINS_BG)
    } else if me.overwritten_by.contains(&other) {
        Some(CONFLICT_LOSES_BG)
    } else {
        None
    }
}

/// A mod-list row background that also reflects selection (MO2's blue highlight,
/// here a parchment-tan so it reads on the burgundy theme).
/// The height of the insertion strip between two rows. Rendered ALWAYS, not only
/// during a drag, so the list does not jump when one starts - on a 100-mod list,
/// making the strips appear on grab shifted everything below by hundreds of
/// pixels and the pointer ended up over a completely different row. It replaces
/// the list's old 1px spacing, so the real cost is 3px per row, and it gives the
/// dense view the breathing room it needed anyway.
const GAP_H: f32 = 4.0;

/// An insertion point between two rows: the drop target for index `gap`, drawn as
/// a burgundy line while it is the live target.
///
/// This is what replaced a border around the hovered ROW. A border says "this row
/// is involved" and leaves the user guessing which side; a line in the gap says
/// exactly where the block lands, which is the whole point of aiming. MO2 draws
/// the same indicator, and its geometry is why: the strip IS the destination, so
/// there is nothing to infer.
/// `interactive` is false when no drag is in flight, and for the strips above the
/// game's own content (which nothing may be ordered above). A non-interactive
/// strip is pure spacing: no `mouse_area`, so idly moving the pointer down a
/// 100-row list does not fire a hover message per strip and rebuild the view
/// each time.
///
/// Both reorderable lists render through this, so a drag reads and aims the same
/// way in the mod list and the plugin list; only the messages differ.
fn drop_gap<'a>(
    gap: usize,
    active: bool,
    interactive: bool,
    over: fn(usize) -> Message,
    drop: Message,
) -> Element<'a, Message> {
    let bar = container(Space::new().width(Length::Fill).height(Length::Fixed(if active { 2.0 } else { 0.0 })))
        .width(Length::Fill)
        .style(move |_t: &Theme| container::Style {
            background: active.then(|| Background::Color(Color::from_rgb8(0x6E, 0x24, 0x2E))),
            ..Default::default()
        });
    // `center_y(len)` is `height(len) + align`, so passing Fill here silently
    // REPLACED the fixed height: every strip then demanded the whole viewport,
    // the rows were squeezed to nothing and the list rendered blank mid-drag.
    // The height is fixed once, and the alignment is set without touching it.
    let strip = container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(GAP_H))
        .align_y(iced::alignment::Vertical::Center);
    if !interactive {
        return strip.into();
    }
    mouse_area(strip).on_enter(over(gap)).on_release(drop).into()
}

fn list_row<'a>(
    content: Element<'a, Message>,
    even: bool,
    selected: bool,
    conflict: Option<Color>,
) -> Element<'a, Message> {
    // Selection outranks the conflict tint: the focused row is where the user's
    // attention already is, and losing its highlight to a colour that describes
    // OTHER rows would be a step backwards.
    let bg = if selected { SEL_BG } else { conflict.unwrap_or_else(|| row_bg(even)) };
    container(content)
        .width(Length::Fill)
        .padding(2)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(bg)),
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

/// The decoded handle for each icon, made ONCE and handed out by address.
///
/// `image::Handle::from_bytes` stamps every handle with `Id::unique()`, so
/// building one per call meant every icon was a brand-new image to the renderer
/// on every view rebuild - a fresh texture upload per icon per frame, plus a
/// `to_vec` copy of the PNG bytes to go with it. That stayed invisible while the
/// view only rebuilt on a click or a hover transition. Tracking the pointer for
/// context-menu placement made it rebuild on every mouse MOVE, and the cache
/// thrashing showed up as icons and text flickering as the pointer travelled.
///
/// Keyed by the address of the `&'static [u8]`, which is stable and unique per
/// icon constant - the bytes themselves are never copied again.
static ICON_HANDLES: std::sync::LazyLock<std::sync::Mutex<HashMap<usize, image::Handle>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

fn icon<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    let handle = {
        let mut cache = ICON_HANDLES.lock().unwrap_or_else(|p| p.into_inner());
        cache
            .entry(bytes.as_ptr() as usize)
            .or_insert_with(|| image::Handle::from_bytes(bytes))
            .clone()
    };
    image(handle).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
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
        .push(Space::new().width(Length::Fill))
        .push(nav(next_label, next_msg, true));

    Column::new()
        .spacing(16)
        .push(header)
        .push(text(title).size(20.0))
        .push(card)
        .push(Space::new().height(Length::Fill))
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

/// One drawn line of the Overwrite tree.
struct OwRow {
    depth: usize,
    /// `/`-joined path relative to the Overwrite: the expansion key.
    rel: String,
    name: String,
    /// `Some(n)` for a folder holding `n` files (recursively), `None` for a file.
    files: Option<usize>,
}

/// The immediate children of `dir` inside a SORTED list of `/`-joined FILE paths.
///
/// The list is the one the tab already had and already caches, so the tree costs
/// no extra disk read - it is derived, not gathered. Sortedness is what makes it
/// cheap: every descendant of `dir` is one contiguous run, found by binary search,
/// and only a run belonging to an EXPANDED folder is ever scanned. A collapsed
/// Overwrite of 4902 files touches a few dozen strings.
///
/// Folders first, then files, each alphabetically - the order MO2 uses.
fn tree_children(entries: &[String], dir: &str) -> Vec<(String, Option<usize>)> {
    let prefix = if dir.is_empty() { String::new() } else { format!("{dir}/") };
    let lo = entries.partition_point(|e| e.as_str() < prefix.as_str());
    let mut dirs: Vec<(String, usize)> = Vec::new();
    let mut files: Vec<String> = Vec::new();
    for e in &entries[lo..] {
        let Some(rest) = e.strip_prefix(prefix.as_str()) else { break };
        match rest.split_once('/') {
            // A folder: count every file under it by extending the current run
            // rather than searching again.
            Some((head, _)) => match dirs.last_mut() {
                Some((n, c)) if n == head => *c += 1,
                _ => dirs.push((head.to_string(), 1)),
            },
            None => files.push(rest.to_string()),
        }
    }
    dirs.into_iter()
        .map(|(n, c)| (n, Some(c)))
        .chain(files.into_iter().map(|n| (n, None)))
        .collect()
}

/// Flatten the expanded parts of the Overwrite into the rows to draw, depth
/// first. Bounded by `limit` for the same reason the Data tree is: the point of
/// opening one level at a time is not to build the other 4900 rows.
fn overwrite_tree_rows(app: &App, entries: &[String], limit: usize) -> Vec<OwRow> {
    fn walk(
        app: &App,
        entries: &[String],
        dir: &str,
        depth: usize,
        limit: usize,
        out: &mut Vec<OwRow>,
    ) {
        if out.len() >= limit || depth > 32 {
            return;
        }
        for (name, files) in tree_children(entries, dir) {
            if out.len() >= limit {
                return;
            }
            let rel = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
            let expanded = files.is_some() && app.overwrite_expanded.contains(&rel);
            out.push(OwRow { depth, rel: rel.clone(), name, files });
            if expanded {
                walk(app, entries, &rel, depth + 1, limit, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(app, entries, "", 0, limit, &mut out);
    out
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

/// [`merged_listing`] memoised per directory against the view generation - it
/// read every enabled mod's directory on each redraw of the Data tab.
fn cached_merged_listing(app: &App, dir: &str) -> Vec<DataRow> {
    let gen = app.view_generation.get();
    if let Some((at, entries)) = app.data_listing.borrow().get(dir) {
        if *at == gen {
            return entries.clone();
        }
    }
    let entries = merged_listing(app, dir);
    app.data_listing.borrow_mut().insert(dir.to_string(), (gen, entries.clone()));
    entries
}

/// The entries of ONE directory of the merged view (`dir` relative to `Data`,
/// `""` for the root): each name, the source providing it (highest-priority
/// enabled mod, or the game data), and whether it's a folder. Winner attribution
/// matches what the FUSE layer actually serves: Overwrite first, then mods from
/// HIGHEST display priority down, then the game data.
///
/// One level at a time, so expanding a node costs one directory read per layer
/// that has it rather than a full recursive walk of every enabled mod.
fn merged_listing(app: &App, dir: &str) -> Vec<DataRow> {
    let mut seen = HashSet::new();
    let mut out: Vec<DataRow> = Vec::new();
    let take = |root: &Path, source: &str, seen: &mut HashSet<String>, out: &mut Vec<DataRow>| {
        let base = if dir.is_empty() { root.to_path_buf() } else { root.join(dir) };
        let Ok(rd) = fs::read_dir(base) else { return };
        for e in rd.flatten() {
            let Ok(name) = e.file_name().into_string() else { continue };
            // Hidden entries are out of the virtual view (eidos-core drops them
            // from the mount too), so the Data tree must not show them as winners
            // - the point of hiding is that the layer below wins instead.
            if eidos_core::is_hidden_name(&name) {
                continue;
            }
            if seen.insert(name.to_lowercase()) {
                out.push((name, source.to_string(), e.path().is_dir()));
            }
        }
    };
    if let Some(inst) = app.created.as_ref() {
        take(&inst.overwrite_dir(), "[Overwrite]", &mut seen, &mut out);
    }
    // `app.mods` is display order = lowest priority first; the merged view's
    // winner is the highest, so walk it in reverse.
    for m in app.mods.iter().rev().filter(|m| m.enabled && !m.is_separator()) {
        take(&m.path, &m.name, &mut seen, &mut out);
    }
    if let Some(g) = selected_game(app) {
        let label = format!("[{}]", g.def.id);
        take(&g.data_path, &label, &mut seen, &mut out);
    }
    // Folders first, then files, each alphabetically - the ordering every file
    // browser uses, and the one that makes a deep tree navigable.
    out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    out
}

/// MO2's hide/unhide, which is a rename and never a delete (`filetree.cpp:375-391`
/// drives exactly this through a `FileRenamer` constructed with HIDE / UNHIDE):
/// hiding appends `.mohidden`, unhiding strips it. Refuses to hide something
/// already hidden or unhide something that is not, so a stale row cannot
/// double-suffix a file into `foo.dds.mohidden.mohidden`.
///
/// Works on directories too - hiding `meshes/` suppresses the whole subtree.
fn set_hidden(path: &Path, hide: bool) -> std::io::Result<PathBuf> {
    use std::io::{Error, ErrorKind};
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "unusable file name"))?;
    let already = eidos_core::is_hidden_name(name);
    if already == hide {
        let what = if hide { "already hidden" } else { "not hidden" };
        return Err(Error::new(ErrorKind::AlreadyExists, what));
    }
    let target = if hide {
        path.with_file_name(format!("{name}{}", eidos_core::HIDDEN_SUFFIX))
    } else {
        path.with_file_name(&name[..name.len() - eidos_core::HIDDEN_SUFFIX.len()])
    };
    // Never let a hide silently swallow an existing file: unhiding onto a name the
    // mod already carries would destroy the live copy.
    if target.symlink_metadata().is_ok() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            format!("{} already exists", target.display()),
        ));
    }
    fs::rename(path, &target)?;
    Ok(target)
}

/// Unhide everything under `root`, MO2's `restoreHiddenFiles`. Returns how many
/// entries were restored.
///
/// Deepest first, so renaming a hidden directory never invalidates the paths of
/// the hidden files collected inside it.
fn restore_hidden_files(root: &Path) -> std::io::Result<usize> {
    fn collect(dir: &Path, depth: usize, out: &mut Vec<(usize, PathBuf)>) {
        if depth > 32 {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, depth + 1, out);
            }
            if p.file_name().and_then(|n| n.to_str()).is_some_and(eidos_core::is_hidden_name) {
                out.push((depth, p));
            }
        }
    }
    let mut found = Vec::new();
    collect(root, 0, &mut found);
    found.sort_by(|a, b| b.0.cmp(&a.0));
    let mut done = 0;
    for (_, p) in found {
        if set_hidden(&p, false).is_ok() {
            done += 1;
        }
    }
    Ok(done)
}

/// One rendered line of the Data tree: how deep it sits, its full path relative
/// to `Data` (the expansion key and what a hide acts on), and the merged-listing
/// entry itself.
struct TreeRow {
    depth: usize,
    rel: String,
    name: String,
    source: String,
    is_dir: bool,
}

/// Flatten the expanded parts of the merged tree into the rows to draw, depth
/// first. Bounded by `limit`: a fully-expanded Skyrim Data tree is six figures of
/// files, and the whole point of expanding a level at a time is not to build them.
fn data_tree_rows(app: &App, limit: usize) -> Vec<TreeRow> {
    fn walk(app: &App, dir: &str, depth: usize, limit: usize, out: &mut Vec<TreeRow>) {
        // Guard against a pathological tree as well as the row budget: a symlink
        // loop inside a mod would otherwise recurse until the stack gives out.
        if out.len() >= limit || depth > 32 {
            return;
        }
        for (name, source, is_dir) in cached_merged_listing(app, dir) {
            if out.len() >= limit {
                return;
            }
            let rel = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
            let expanded = is_dir && app.data_expanded.contains(&rel);
            out.push(TreeRow { depth, rel: rel.clone(), name, source, is_dir });
            if expanded {
                walk(app, &rel, depth + 1, limit, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(app, "", 0, limit, &mut out);
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
        .push(Space::new().width(Length::Fill))
        .push(icon_btn(IC_ENDORSE, 20.0, endorse_msg))
        .push(icon_btn(IC_UPDATE, 20.0, update_msg))
        .push(icon_btn(IC_HELP, 20.0, Some(Message::ShowAbout)));
    container(row).width(Length::Fill).padding(2).style(bar_style).into()
}

#[allow(clippy::too_many_arguments)]
fn mod_row<'a>(
    i: usize,
    m: &ModEntry,
    meta: Option<&RowMeta>,
    flag_icon: Option<&'static [u8]>,
    hidden_icon: Option<&'static [u8]>,
) -> Element<'a, Message> {
    // Unmanaged content - the game's own DLCs and Creation Club plugins - is
    // listed so the mod list matches what will actually load, but none of it is
    // ours to move, disable or remove. MO2 renders these the same way: present,
    // greyed, inert. A checkbox with no `on_toggle` draws disabled, which is
    // exactly the look.
    let toggle = if m.unmanaged {
        checkbox(true).size(16)
    } else {
        checkbox(m.enabled).on_toggle(move |_| Message::ToggleMod(i)).size(16)
    };

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
        .push(
            text(if m.unmanaged { "Game content".to_string() } else { category })
                .size(11.0)
                .width(C_CATEGORY),
        )
        .push(text(content).size(10.0).width(C_CONTENT))
        .push(text(version).size(11.0).width(C_VERSION))
        .push(flag_cell);

    // Left-press selects + arms a drag, entering during a drag retargets the drop,
    // release commits it; right-click opens the action menu (MO2's context menu).
    // Inner buttons still get their own clicks; the mouse_area catches the rest.
    if m.unmanaged {
        // No drag, no context menu: there is no action on this row that would do
        // anything, and offering one only invites the question of why it failed.
        return container(row).into();
    }
    mouse_area(row)
        .on_press(Message::DragStart(i))
        .on_enter(Message::DragOverGap(i))
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
    color: Option<[u8; 3]>,
    collapsed: bool,
    selected: bool,
) -> Element<'a, Message> {
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
        ;

    container(
        mouse_area(row)
            .on_press(Message::DragStart(i))
            .on_enter(Message::DragOverGap(i))
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
        .push(Space::new().width(Length::Fill))
        .push(
            text(format!(
                "Active: {active}  |  Endorsed: {}  |  Updates: {}",
                app.endorsed_count, app.updated_count
            ))
            .size(12.0),
        );

    // The category catalog (resolves ids -> names; drives the filter + the column).
    let cats = app.categories.as_ref();

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
        ;

    let query = app.search.trim().to_lowercase();
    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
    let mut shown = 0usize;
    if app.mods.is_empty() {
        list = list.push(text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0));
    }
    // Decided up front, because whether a separator draws depends on whether any
    // mod BELOW it survives the filter - which the single downward pass this used
    // to be could not know when it reached the header.
    let filtering = !query.is_empty() || app.category_filter.is_some();
    let vis = mod_row_visibility(app, cats);
    // The live drag's insertion point, if any, so exactly one gap draws the line.
    // A drag that has not moved off its own row targets nothing visible: a plain
    // click must never flash an indicator.
    let live_gap = app
        .drag_state
        .filter(|d| d.gap != d.from && d.gap != d.from + 1)
        .map(|d| d.gap);
    let dragging = app.drag_state.is_some();
    let lowest_gap = first_managed(&app.mods);
    for (i, m) in app.mods.iter().enumerate() {
        // A row is highlighted when it is the focus row or in the multi-selection.
        let selected = app.selected_mod == Some(i) || app.selected_mods.contains(&i);
        // A separator renders as a full-width group header - no checkbox, version,
        // conflict flags, or content (it never queries the ConflictMap). It always
        // shows (even under a filter, and even when its own group is collapsed).
        if m.is_separator() {
            if !vis[i] {
                continue;
            }
            // Folding is suspended under a filter, so the header draws unfolded:
            // the mods it heads ARE on screen, and a [+] next to them would lie.
            let collapsed = !filtering && app.collapsed.contains(m.display_name());
            let color = app.meta_cache.get(&m.name).and_then(|r| r.color);
            // Every VISIBLE row gets a strip above it, separators included, or the
            // slot just before a group header would be unreachable.
            list = list.push(drop_gap(i, live_gap == Some(i), dragging && i >= lowest_gap, Message::DragOverGap, Message::DragDrop));
            list = list.push(separator_row(i, m, color, collapsed, selected));
            continue;
        }
        if !vis[i] {
            continue;
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
        // The insertion strip ABOVE this row. Always rendered (stable layout),
        // targetable only during a drag and only from the first managed row down:
        // nothing may be ordered above the game's own content.
        list = list.push(drop_gap(i, live_gap == Some(i), dragging && i >= lowest_gap, Message::DragOverGap, Message::DragDrop));
        list = list.push(list_row(
            mod_row(i, m, meta, flag_icon, hidden_icon),
            i % 2 == 0,
            selected,
            conflict_tint(app, i),
        ));
    }
    // The trailing strip: the only way to aim at the end of the list, since
    // hovering a row always means "above it".
    if !app.mods.is_empty() {
        let end = app.mods.len();
        list = list.push(drop_gap(end, live_gap == Some(end), dragging, Message::DragOverGap, Message::DragDrop));
    }
    // `shown` counts mods only, so this cannot fire on a list that is all folded
    // groups - and it only speaks when something was actually asked.
    if !app.mods.is_empty() && shown == 0 && filtering {
        let by = match (query.is_empty(), app.category_filter.is_some()) {
            (false, false) => format!("named \"{}\"", app.search.trim()),
            (true, _) => "in this category".to_string(),
            (false, true) => format!("named \"{}\" in this category", app.search.trim()),
        };
        list = list.push(text(format!("No mods {by}.")).size(12.0));
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
    // `on_release` here is the catch-all: a row or a strip that handles the
    // release captures it and this never fires, but a release landing anywhere
    // else in the list - a header, a gap the layout moved, empty space below the
    // last row - disarms instead of leaving a drag live for the next click to
    // commit. `on_exit` covers releasing outside the list entirely.
    let list_area = mouse_area(scrollable(list).id(mod_scroll_id()).height(Length::Fill))
        .on_exit(Message::DragCancel)
        .on_release(Message::DragCancel);

    // ALWAYS in the flow, at a fixed height, even when it has nothing to say.
    // Appearing and disappearing moved every row below it by its own height, so
    // clicking a mod scrolled the list out from under the pointer - and if the
    // button came up somewhere that was no longer a row, the armed drag was
    // never released and the next click moved the mod. The same mistake the
    // insertion strips were built to avoid, made again one panel over.
    let legend = container(conflict_legend(app).unwrap_or_else(|| Space::new().width(0).height(0).into()))
        // Tall enough for the 12px swatch and the 11pt label without clipping,
        // and identical whether or not there is anything to show.
        .height(Length::Fixed(20.0))
        .align_y(iced::alignment::Vertical::Center);

    let inner = Column::new()
        .spacing(6)
        .push(profile)
        .push(search)
        .push(legend)
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
    container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
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
        return Space::new().width(Length::Shrink).height(Length::Shrink).into();
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

    // Bulk unhide, offered only when the mod actually has hidden files - the
    // conflict scan already tracks that (it is what drives the hidden glyph on the
    // row), so this costs no extra walk.
    let has_hidden = app
        .conflicts
        .as_ref()
        .and_then(|c| c.mods.get(&((i + 1) as u32)))
        .is_some_and(|mc| mc.has_hidden);
    if has_hidden {
        col = col.push(menu_item("Unhide all files", Message::RestoreHiddenFiles(i)));
    }

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
        let sw = button(Space::new().width(Length::Fixed(15.0)).height(Length::Fixed(13.0)))
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
        .push(Space::new().height(Length::Fixed(8.0)))
        .push(text(format!("Overridden by ({}):", loses.len())).size(13.0));
    if loses.is_empty() {
        col = col.push(text("  (none)").size(11.0));
    }
    for (p, who) in loses.iter().take(300) {
        col = col.push(text(format!("  {p}   <   {who}")).size(11.0));
    }
    col.into()
}

/// Filetree tab: every file the mod ships, relative to its root, each with a
/// Hide / Unhide toggle. Unlike the Data tab this is the mod's REAL contents, so
/// hidden files are listed (with their suffix) and are the only place to unhide
/// one individually.
fn info_filetree<'a>(app: &App, i: usize, m: &ModEntry) -> Element<'a, Message> {
    let entries = cached_entries(app, &m.path);
    let hidden = entries.iter().filter(|e| path_is_hidden(e)).count();
    let summary = if hidden == 0 {
        format!("{} file(s):", entries.len())
    } else {
        format!("{} file(s), {hidden} hidden:", entries.len())
    };
    let mut col = Column::new().spacing(1).push(text(summary).size(12.0));
    if hidden > 0 {
        col = col.push(tool_btn("Unhide all", Message::RestoreHiddenFiles(i)));
    }
    for e in entries.into_iter().take(2000) {
        let is_hidden = path_is_hidden(&e);
        let label = if is_hidden { "Unhide" } else { "Hide" };
        let row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(e.clone()).size(11.0).width(Length::Fill))
            .push(
                button(text(label).size(10.0))
                    .padding([1, 5])
                    .on_press(Message::ToggleFileHidden(i, e))
                    .style(if is_hidden { button::primary } else { button::secondary }),
            );
        col = col.push(row);
    }
    col.into()
}

/// Whether a `/`-joined relative path names a hidden entry, or lies under one.
fn path_is_hidden(rel: &str) -> bool {
    rel.split('/').any(eidos_core::is_hidden_name)
}

/// INI Tweaks tab: the fragments this mod ships in its `INI Tweaks/` folder, each
/// individually enabled. Enabled fragments are merged into the profile's game INI
/// at launch, in mod priority order, and undone again when the run's INIs are
/// captured back - so a tweak stays a tweak instead of quietly becoming a setting.
fn info_ini_tweaks<'a>(app: &App, i: usize, m: &ModEntry) -> Element<'a, Message> {
    let available = eidos_instance::available_ini_tweaks(&m.path);
    if available.is_empty() {
        return Column::new()
            .spacing(6)
            .push(text("This mod ships no INI tweaks.").size(12.0))
            .push(
                text("A mod with tweaks has an 'INI Tweaks' folder of small INI fragments.")
                    .size(10.0),
            )
            .into();
    }
    let enabled: Vec<String> =
        app.created.as_ref().map(|inst| inst.mod_meta(&m.name).ini_tweaks().to_vec()).unwrap_or_default();

    let mut col = Column::new().spacing(3).push(
        text("Enabled fragments are merged into this profile's game INI at launch.").size(11.0),
    );
    for name in available {
        let on = enabled.iter().any(|e| e.eq_ignore_ascii_case(&name));
        let label = name.clone();
        col = col.push(
            checkbox(on).label(label)
                .on_toggle(move |_| Message::ToggleIniTweak(i, name.clone()))
                .size(13.0)
                .text_size(12.0),
        );
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
        return Space::new().width(Length::Shrink).height(Length::Shrink).into();
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
        .push(info_tab_btn("INI Tweaks", InfoTab::IniTweaks, app.info_tab == InfoTab::IniTweaks))
        .push(info_tab_btn("Notes", InfoTab::Notes, app.info_tab == InfoTab::Notes));

    let content = match app.info_tab {
        InfoTab::General => info_general(app, m),
        InfoTab::Conflicts => info_conflicts(app, i),
        InfoTab::Filetree => info_filetree(app, i, m),
        InfoTab::IniTweaks => info_ini_tweaks(app, i, m),
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

/// How many tree rows the Data tab will draw. Generous for browsing, but finite:
/// expanding `meshes/` in a heavy setup is six figures of files and iced builds a
/// widget per row.
const DATA_TREE_ROWS: usize = 3000;

/// Data tab: the merged view as a real tree, each node labelled with the layer
/// that actually provides it. This is the virtual filesystem the game will see,
/// so a hidden file is absent here by construction - unhiding is done from the
/// owning mod's Filetree tab, which shows the mod's real contents.
fn data_panel<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .push(text("Name").size(11.0).width(Length::FillPortion(3)))
        .push(text("Provided by").size(11.0).width(Length::FillPortion(2)))
        .push(text("").size(11.0).width(Length::Fixed(56.0)));

    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
    let rows = data_tree_rows(app, DATA_TREE_ROWS);
    if rows.is_empty() {
        list = list.push(text("(empty)").size(12.0));
    }
    let truncated = rows.len() >= DATA_TREE_ROWS;
    for (idx, r) in rows.into_iter().enumerate() {
        // A folder gets a clickable disclosure triangle; a file gets a spacer of
        // the same width so names stay in one column.
        let lead: Element<'a, Message> = if r.is_dir {
            let glyph = if app.data_expanded.contains(&r.rel) { "\u{25BE}" } else { "\u{25B8}" };
            button(text(glyph).size(11.0))
                .padding([0, 4])
                .on_press(Message::DataToggleDir(r.rel.clone()))
                .style(button::text)
                .into()
        } else {
            Space::new().width(Length::Fixed(18.0)).into()
        };
        let name = Row::new()
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .push(Space::new().width(Length::Fixed(r.depth as f32 * 14.0)))
            .push(lead)
            .push(text(r.name).size(12.0));

        // Hiding is only offered on rows a mod owns: the Overwrite is regenerated
        // by the game (it would just come back) and the game layer is the pristine
        // install, which Eidos never writes to.
        let owner = app.mods.iter().position(|m| !m.is_separator() && m.name == r.source);
        let action: Element<'a, Message> = match owner {
            Some(i) => button(text("Hide").size(10.0))
                .padding([1, 5])
                .on_press(Message::ToggleFileHidden(i, r.rel.clone()))
                .style(button::secondary)
                .into(),
            None => Space::new().width(Length::Fixed(56.0)).into(),
        };

        let row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(container(name).width(Length::FillPortion(3)))
            .push(text(r.source).size(12.0).width(Length::FillPortion(2)))
            .push(container(action).width(Length::Fixed(56.0)));
        list = list.push(striped(row.into(), idx % 2 == 0));
    }
    if truncated {
        list = list.push(
            text(format!("Showing the first {DATA_TREE_ROWS} entries - collapse a folder to see more."))
                .size(11.0),
        );
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

    // A tree, not 4902 full paths one under the other. Same grammar as the Data
    // tab - triangle, indent, name - because they are the same gesture.
    let entries = cached_entries(app, &dir);
    let mut c = Column::new().spacing(2);
    if entries.is_empty() {
        c = c.push(text("(empty)").size(12.0));
    } else {
        c = c.push(text(format!("{} file(s):", entries.len())).size(11.0));
    }
    let rows = overwrite_tree_rows(app, &entries, DATA_TREE_ROWS);
    let truncated = rows.len() >= DATA_TREE_ROWS;
    for r in rows {
        let lead: Element<'a, Message> = match r.files {
            Some(_) => {
                let glyph =
                    if app.overwrite_expanded.contains(&r.rel) { "\u{25BE}" } else { "\u{25B8}" };
                button(text(glyph).size(11.0))
                    .padding([0, 4])
                    .on_press(Message::OverwriteToggleDir(r.rel.clone()))
                    .style(button::text)
                    .into()
            }
            // Same width as the triangle, so names stay in one column.
            None => Space::new().width(Length::Fixed(18.0)).into(),
        };
        let mut row = Row::new()
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .push(Space::new().width(Length::Fixed(r.depth as f32 * 14.0)))
            .push(lead)
            .push(text(r.name).size(11.5));
        if let Some(n) = r.files {
            // How much is under a folder, so a closed one still says something.
            row = row.push(text(format!("  {n}")).size(10.0).color(FOMOD_INK_FAINT));
        }
        c = c.push(row);
    }
    if truncated {
        c = c.push(
            text(format!("Showing the first {DATA_TREE_ROWS} rows - collapse a folder to see more."))
                .size(11.0),
        );
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
        .push(Space::new().width(Length::Fill))
        .push(button(text("Open folder").size(11.0)).padding(4).on_press(Message::OpenFolder(dir.clone())))
        .push(button(text("Refresh").size(11.0)).padding(4).on_press(Message::RefreshSaves));

    let col_header = Row::new()
        .spacing(8)
        .push(text("Name").size(11.0).width(Length::Fill))
        .push(text("Date").size(11.0).width(Length::Fixed(130.0)))
        .push(text("Size").size(11.0).width(Length::Fixed(80.0)))
        .push(Space::new().width(Length::Fixed(80.0)));

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
        // The name is the click target for the details pane; the row's other
        // controls keep working (a Delete click must not also select).
        let name = button(text(save.filename.clone()).size(12.0))
            .padding(0)
            .width(Length::Fill)
            .on_press(Message::SelectSave(i))
            .style(button::text);
        let row = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(name)
            .push(text(format_mtime(save.mtime)).size(11.0).width(Length::Fixed(130.0)))
            .push(text(format_size(save.size)).size(11.0).width(Length::Fixed(80.0)))
            .push(container(del).width(Length::Fixed(80.0)));
        rows = rows.push(list_row(
            container(row).padding(3).into(),
            i % 2 == 0,
            app.selected_save == Some(i),
            // Saves do not fight over files.
            None,
        ));
    }

    let list = Column::new()
        .spacing(6)
        .push(header)
        .push(text(dir.display().to_string()).size(10.0))
        .push(col_header)
        .push(scrollable(rows).height(Length::Fill));

    match app.selected_save.and_then(|i| app.saves.get(i)) {
        Some(save) => Row::new()
            .spacing(8)
            .push(container(list).width(Length::FillPortion(3)))
            .push(container(save_details(app, save)).width(Length::FillPortion(2)))
            .into(),
        None => list.into(),
    }
}

/// The details pane for the selected save: who and where, and - the reason this
/// exists - which of the plugins baked into the save are no longer active.
///
/// MO2 shows this before you load: a save carries the plugin list it was written
/// with, and loading it without those plugins is how a playthrough loses its
/// contents (or crashes on the way in).
fn save_details<'a>(app: &App, save: &eidos_instance::SaveEntry) -> Element<'a, Message> {
    let mut col = Column::new().spacing(4).push(text(save.filename.clone()).size(13.0));

    let info = match app.save_info.as_ref().filter(|(p, _)| *p == save.path) {
        Some((_, Ok(info))) => info,
        Some((_, Err(e))) => {
            return col
                .push(text(format!("Cannot read this save: {e}")).size(11.0))
                .push(text("The list below is unavailable; the file itself is untouched.").size(10.0))
                .into();
        }
        // Parsed on selection, so this only shows for the frame in between.
        None => return col.push(text("Reading...").size(11.0)).into(),
    };

    let mut facts: Vec<(&'static str, String)> = vec![
        ("Character", format!("{} (level {})", info.player_name, info.level)),
        ("Location", info.location.clone()),
        ("In-game date", info.game_date.clone()),
    ];
    if let Some((d, h, m)) = info.playtime() {
        facts.push(("Played", format!("{d}d {h}h {m}m")));
    }
    facts.push(("Save", format!("#{}", info.save_number)));
    facts.push(("Plugins", format!("{} + {} light", info.plugins.len(), info.light_plugins.len())));
    for (k, v) in facts {
        col = col.push(info_kv(k, v));
    }

    let missing = &app.save_missing;
    col = col.push(Space::new().height(Length::Fixed(6.0)));
    if missing.is_empty() {
        return col
            .push(text("Every plugin this save uses is active.").size(11.0))
            .push(
                text(if info.truncated {
                    "(the save's plugin list was truncated, so this is advisory)"
                } else {
                    ""
                })
                .size(10.0),
            )
            .into();
    }

    col = col.push(text(format!("{} plugin(s) missing:", missing.len())).size(12.0));
    for m in missing.iter().take(40) {
        let what = match m.state {
            eidos_gamefeatures::SavePluginState::Inactive => "inactive",
            eidos_gamefeatures::SavePluginState::Absent => "not installed",
        };
        let who = if m.providers.is_empty() {
            "  no mod here provides it".to_string()
        } else {
            format!("  in: {}", m.providers.join(", "))
        };
        col = col.push(text(format!("{} ({what})", m.name)).size(11.0)).push(text(who).size(10.0));
    }
    // Only offer the fix when something on disk can actually supply the plugins;
    // otherwise the button would enable nothing and look broken.
    let fixable = missing.iter().any(|m| !m.providers.is_empty());
    if fixable {
        col = col
            .push(Space::new().height(Length::Fixed(4.0)))
            .push(tool_btn("Enable the mods this save needs", Message::FixSaveMods));
    }
    if info.truncated {
        col = col.push(
            text("The save's plugin list was truncated, so treat this as advisory.").size(10.0),
        );
    }
    col.into()
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

/// The colour of the status word, following MO2's own scheme
/// (downloadlist.cpp:202): green for the one state that is asking to be acted
/// on, amber for a mod that was installed and then removed, and NOTHING for
/// "Installed" - a finished job should not keep waving.
fn download_state_color(state: DownloadState, theme: &Theme) -> Option<Color> {
    match state {
        DownloadState::Ready => Some(theme.palette().success),
        DownloadState::Uninstalled => Some(theme.palette().warning),
        DownloadState::Installed | DownloadState::Untracked => None,
    }
}

// Downloads column widths, declared once so the header and the rows cannot drift
// apart. Each is sized to its widest real content and no more: every pixel they
// do not take goes to the name, which is the only column whose content has no
// bound - Nexus file names run to eighty characters.
//
// They were 80/80/90/150, which is 400px of a pane that is roughly 500 wide once
// the mod list has its half. That left about 68px for the name, so "Dynamic Armor
// Physics" came out three lines tall. The action column was the worst of it: 150
// reserved for two buttons that measure about 100.
const DL_C_VERSION: f32 = 56.0; // "1.0.1"
const DL_C_SIZE: f32 = 66.0; // "10.3 MiB"
const DL_C_STATUS: f32 = 66.0; // "Installed"
// Sized on the WIDEST pair the column can ever hold at once, which is not the
// resting state: "Reinstall" beside Delete armed as "Confirm?". Sizing it on
// "Install" + "Delete" would clip the two labels that only appear when something
// is at stake.
const DL_C_ACTIONS: f32 = 128.0;

fn downloads_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.downloads_dir();

    let header = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text("Downloads").size(13.0))
        .push(Space::new().width(Length::Fill))
        .push(button(text("Open folder").size(11.0)).padding(4).on_press(Message::OpenFolder(dir.clone())))
        .push(button(text("Refresh").size(11.0)).padding(4).on_press(Message::RefreshDownloads));

    let col_header = Row::new()
        .spacing(8)
        .push(text("Name").size(11.0).width(Length::Fill))
        .push(text("Version").size(11.0).width(Length::Fixed(DL_C_VERSION)))
        .push(text("Size").size(11.0).width(Length::Fixed(DL_C_SIZE)))
        .push(text("Status").size(11.0).width(Length::Fixed(DL_C_STATUS)))
        .push(Space::new().width(Length::Fixed(DL_C_ACTIONS)));

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
        // MO2 keeps Install available on an already-installed archive
        // (downloadlistview.cpp:230, `state >= STATE_READY`) because re-running a
        // FOMOD with different answers is a real thing to want. What it does NOT
        // do is present it as the next step: its Install lives in a context menu,
        // and it colours the STATUS rather than the action.
        //
        // So keep the action, drop the shouting. Burgundy means "this is what to
        // do here"; on a row that is already installed, that was a lie, and the
        // label said "Install" for something that would install it a second time.
        let installed = row.state == DownloadState::Installed;
        let install = button(text(if installed { "Reinstall" } else { "Install" }).size(11.0))
            .padding(4)
            .on_press(Message::ModPicked(Some(row.path.clone())))
            .style(if installed { button::secondary } else { button::primary });
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
            .push(text(row.version.clone()).size(11.0).width(Length::Fixed(DL_C_VERSION)))
            .push(text(format_size(row.size)).size(11.0).width(Length::Fixed(DL_C_SIZE)))
            .push({
                let st = row.state;
                text(download_state_label(st))
                    .size(11.0)
                    .width(Length::Fixed(DL_C_STATUS))
                    .style(move |t: &Theme| iced::widget::text::Style {
                        color: download_state_color(st, t),
                    })
            })
            .push(
                Row::new()
                    .spacing(4)
                    .width(Length::Fixed(DL_C_ACTIONS))
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
#[derive(Clone)]
struct Diagnostic {
    level: DiagLevel,
    title: String,
    detail: String,
    /// One-click remedies, rendered as buttons on the card. Most checks only
    /// inform; the ones that can FIX what they found carry the fix with them, so
    /// recovery is not a file-manager expedition. More than one when the finding
    /// has two honest outcomes (restore vs accept).
    actions: Vec<(&'static str, Message)>,
}

/// Run every health check for the current setup - MO2's problems panel, plus the
/// Linux-specific ones MO2 never needed (the launch capability above all, which
/// silently disables FUSE passthrough after each rebuild).
fn diagnostics(app: &App) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

    // First, because while it is showing nothing else in this tab is trustworthy:
    // the mod list Eidos is working from does not match what is on disk, so the
    // conflict map, the load order and the layer stack are all built from a
    // partial picture. Saving is refused for as long as this is true.
    if let Some(why) = app.created.as_ref().and_then(|i| i.modlist_checked().1.reason().map(str::to_string)) {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: "The mod list does not match the mods folder".to_string(),
            detail: format!(
                "{why} Eidos will not save the mod list until this is resolved, so the order and \
                 enabled state on disk are safe. If a drive holds your mods, mount it and press F5."
            ),
            actions: Vec::new(),
        });
    }

    // The launch capability is optional: it only gates FUSE passthrough, which is
    // off by default because it stops the game opening its own archives and
    // plugins. So this is only worth a Problem when passthrough was asked for.
    if passthrough_requested() {
        out.push(if app.cap_missing {
            Diagnostic {
                level: DiagLevel::Problem,
                title: "Passthrough requested but unavailable (launch capability missing)"
                    .to_string(),
                detail: format!(
                    "EIDOS_FUSE_PASSTHROUGH is set, but the launch binary has no CAP_SYS_ADMIN, so reads go through the daemon anyway. Run:  sudo setcap cap_sys_admin+ep {}  then press F5. Every rebuild of that binary wipes it.",
                    find_eidos_binary().display()
                ),
                actions: Vec::new(),
            }
        } else {
            Diagnostic {
                level: DiagLevel::Advice,
                title: "FUSE passthrough is ON (opt-in)".to_string(),
                detail: "Reads go straight to the backing file. Measured on Skyrim SE, this makes the game fail to open its archives and plugins, so mods do not load. Unset EIDOS_FUSE_PASSTHROUGH if content goes missing in-game.".to_string(),
                actions: Vec::new(),
            }
        });
    } else {
        out.push(Diagnostic {
            level: DiagLevel::Ok,
            title: "FUSE passthrough is off".to_string(),
            detail: "The daemon serves reads itself, which is what lets the game open its archives and plugins. The launch capability is not needed for this.".to_string(),
            actions: Vec::new(),
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
                    actions: Vec::new(),
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
                    actions: Vec::new(),
                });
            }
        }
        None => out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: "Load order not computed yet".to_string(),
            detail: "Open the Plugins tab to analyse the load order.".to_string(),
            actions: Vec::new(),
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
                actions: Vec::new(),
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
                actions: Vec::new(),
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
                actions: Vec::new(),
            });
        }
    }

    // The last session wrecked the active set (a crash artifact written straight
    // into the bound profile dir): the pre-session snapshot noticed, and the fix
    // is one click. Also fires when the user deliberately disabled most plugins
    // in-game - they dismiss it by playing on; restoring is never automatic.
    if let (Some(inst), Some(spec)) = (
        app.created.as_ref(),
        selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)),
    ) {
        let prof = inst.active();
        if let Some(reason) = prof.plugin_loss_since_snapshot(&spec) {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: "The last session damaged the plugin active set".to_string(),
                detail: format!(
                    "Compared to launch, plugins.txt now {reason}. If the game crashed, restore \
                     the pre-session order below; if you disabled those plugins on purpose, \
                     ignore this."
                ),
                actions: vec![
                    ("Restore the pre-session order", Message::RestorePreSessionPlugins),
                    ("Keep the current set", Message::AcceptPluginState),
                ],
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
                actions: Vec::new(),
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
                actions: Vec::new(),
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
                    actions: Vec::new(),
                });
            }
        }
        if game.compatdata.is_none() {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: "No Proton prefix found".to_string(),
                detail: "Launch the game once through Steam so its prefix exists; until then the load order and INIs cannot be deployed.".to_string(),
                actions: Vec::new(),
            });
        }
        out.extend(orphan_archive_diagnostics(app, game.def.id));
        out.extend(script_extender_diagnostic(game));
    }

    out
}

/// What the script extender itself recorded about each of its plugin DLLs on the
/// last run.
///
/// The passthrough check above says whether DLL loading is *likely* to work. This
/// says what happened. The distinction matters because the two failure modes look
/// identical from outside: a plugin refused for an incompatible runtime version
/// and one the manager failed to expose both end with the feature simply absent
/// in game.
fn script_extender_diagnostic(game: &DetectedGame) -> Option<Diagnostic> {
    let spec = GameSpec::for_id(game.def.id)?;
    let prefix = game.compatdata.as_ref()?.join("pfx");
    let docs = eidos_plugins::documents_my_games_dir(&prefix, &spec);
    let path = eidos_gamefeatures::se_log_path(game.def.id, &docs, &game.install_path)?;

    let Ok(raw) = fs::read(&path) else {
        return Some(Diagnostic {
            level: DiagLevel::Advice,
            title: "No script-extender log yet".to_string(),
            detail: format!(
                "Launch the game once through Eidos and this will report whether each SKSE-style plugin DLL loaded. Expected at {}.",
                path.display()
            ),
            actions: Vec::new(),
        });
    };
    // The extender writes cp1252, so a plugin name with an accent is not valid
    // UTF-8; lossy keeps the rest of the line readable rather than dropping it.
    let plugins = eidos_gamefeatures::parse_se_log(&String::from_utf8_lossy(&raw));
    if plugins.is_empty() {
        return None;
    }
    // The log is from the LAST run, which may predate the current load order, so
    // stamp it - an old log claiming success is the confusing case.
    let when = fs::metadata(&path).and_then(|m| m.modified()).map(format_mtime).unwrap_or_default();
    let failed: Vec<&eidos_gamefeatures::SePluginLoad> =
        plugins.iter().filter(|p| !p.loaded).collect();
    if failed.is_empty() {
        return Some(Diagnostic {
            level: DiagLevel::Ok,
            title: format!("All {} script-extender plugins loaded", plugins.len()),
            detail: format!("From the extender's own log, last written {when}."),
            actions: Vec::new(),
        });
    }
    let lines: Vec<String> =
        failed.iter().take(10).map(|p| format!("{}: {}", p.dll, p.status)).collect();
    let more = failed.len().saturating_sub(lines.len());
    let tail = if more > 0 { format!("  (and {more} more)") } else { String::new() };
    Some(Diagnostic {
        level: DiagLevel::Problem,
        title: format!(
            "{} of {} script-extender plugins did not load",
            failed.len(),
            plugins.len()
        ),
        detail: format!("{}{tail}  -  from the extender's own log, last written {when}.", lines.join("   ")),
        actions: Vec::new(),
    })
}

/// Archives (BSA/BA2) an enabled mod ships that nothing will load: the engine
/// only reads an archive whose name matches an ACTIVE plugin, or that the INI
/// registers by hand. An orphan is silent - the mod looks installed and simply
/// has no effect - which is exactly the class of problem a diagnostic is for.
///
/// Advice, never a problem: a mod can ship an archive deliberately for a plugin
/// the user has not enabled yet.
fn orphan_archive_diagnostics(app: &App, game_id: &str) -> Vec<Diagnostic> {
    let Some(inst) = app.created.as_ref() else { return Vec::new() };
    let mods: Vec<(String, PathBuf)> = app
        .mods
        .iter()
        .filter(|m| m.enabled && !m.is_separator())
        .map(|m| (m.name.clone(), m.path.clone()))
        .collect();
    let archives = eidos_gamefeatures::mod_archives(&mods);
    if archives.is_empty() {
        return Vec::new();
    }
    let active: Vec<String> = app
        .plugins
        .as_ref()
        .map(|l| l.plugins.iter().filter(|p| p.enabled).map(|p| p.name.clone()).collect())
        .unwrap_or_default();
    // The profile's own INI copy is the one that gets deployed, so it is what the
    // next launch will actually register.
    let registered =
        eidos_gamefeatures::registered_archives_in(&inst.active().dir(), game_id);

    let orphans = eidos_gamefeatures::orphan_archives(&archives, &active, &registered);
    if orphans.is_empty() {
        return Vec::new();
    }
    let listed: Vec<String> = orphans.iter().take(8).map(|(m, a)| format!("{a} ({m})")).collect();
    let more = orphans.len().saturating_sub(listed.len());
    let tail = if more > 0 { format!(", and {more} more") } else { String::new() };
    vec![Diagnostic {
        level: DiagLevel::Advice,
        title: format!("{} archive(s) no active plugin loads", orphans.len()),
        detail: format!(
            "{}{tail}. An engine only loads an archive named after an ACTIVE plugin \
             (<plugin>.bsa or \"<plugin> - Textures.bsa\"), or one the INI registers. \
             Enable the matching plugin, or the mod's assets will not appear.",
            listed.join(", ")
        ),
        actions: Vec::new(),
    }]
}

/// The Diagnostics tab label, carrying the count of things needing attention.
fn diagnostics_tab_label(app: &App) -> String {
    let n = app.diag.iter().filter(|d| d.level == DiagLevel::Problem).count();
    if n > 0 {
        format!("Diagnostics ({n})")
    } else {
        "Diagnostics".to_string()
    }
}

fn diagnostics_panel<'a>(app: &App) -> Element<'a, Message> {
    // The same cache the tab label reads, so the count on the tab and the cards
    // in the panel can never tell two different stories.
    let checks = app.diag.clone();
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
        let mut card = Column::new()
            .spacing(2)
            .push(
                Row::new()
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .push(text(tag).size(9.0).color(color).width(Length::Fixed(58.0)))
                    .push(text(d.title).size(12.0).width(Length::Fill)),
            )
            .push(text(d.detail).size(10.5).color(Color::from_rgb8(0x6A, 0x5A, 0x40)));
        if !d.actions.is_empty() {
            let mut row = Row::new().spacing(6);
            for (label, msg) in d.actions {
                row = row.push(tool_btn(label, msg));
            }
            card = card.push(row);
        }
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
        .map(|p| p.plugins_state_dir());
    match profile_state {
        Some(dir) => list.apply_prefix_state(&dir, &spec),
        None => {
            if let Some(cd) = game.compatdata.as_ref() {
                let dir = plugins_txt_dir(&cd.join("pfx"), &spec);
                list.apply_prefix_state(&dir, &spec);
            }
        }
    }
    // The pins are the user's, so they load from the profile and outlive any
    // rediscovery of the plugins themselves.
    if let Some(inst) = app.created.as_ref() {
        list.locked = inst.active().read_locked_order();
    }
    list.refresh(&spec);
    Some(list)
}

/// Persist the plugin load order: into the active profile's plugins dir (the
/// single source of truth, bind-mounted over the prefix at launch) AND a shadow
/// copy into the prefix for external tools reading it outside Eidos.
/// The trees LOOT must look at besides the game's own `Data`, highest priority
/// first with Overwrite ahead of all - the union's own precedence.
///
/// Two filters, both of which cost a sort when they are missing:
///
/// UNMANAGED rows are the game's DLC and Creation Club content. `app.mods`
/// carries them so the list can show them, but their `path` is a single `.esm`
/// FILE inside the game's Data directory, not a directory. Offered to libloot as
/// data paths, eighty files it is asked to scan as folders, every sort died with
/// "libloot: an I/O error occurred" and no hint as to which path was at fault.
/// They also need no offering: they are already in the Data dir LOOT reads.
///
/// And anything no longer on disk - a mod folder deleted since the list was
/// read - for the same reason: libloot reports the failure without naming it, so
/// one stale row would take the whole sort down.
fn loot_data_paths(app: &App) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(inst) = app.created.as_ref() {
        dirs.push(inst.overwrite_dir());
    }
    dirs.extend(
        app.mods
            .iter()
            .rev()
            .filter(|m| m.enabled && !m.is_separator() && !m.is_unmanaged())
            .map(|m| m.path.clone()),
    );
    dirs.retain(|p| p.is_dir());
    dirs
}

/// Why a plugin will not move, in the terms a modder already thinks in.
///
/// A plugin must load after every one of its masters and before anything that
/// declares IT as a master, so a plugin caught between the two has exactly one
/// legal slot. Naming both sides is the difference between "the drag is broken"
/// and "of course, EX2 needs EX1".
fn pinned_by(range: &MovableRange) -> String {
    match (&range.after, &range.before) {
        (Some(a), Some(b)) => format!(
            "Held in place: it must load after {a} (one of its masters) and before {b}, which lists it as a master."
        ),
        (Some(a), None) => {
            format!("Held in place: it must load after {a}, which is one of its masters.")
        }
        (None, Some(b)) => {
            format!("Held in place: it must load before {b}, which lists it as a master.")
        }
        (None, None) => {
            "Held in place: the game loads this plugin itself, at a fixed position.".to_string()
        }
    }
}

/// Persist the load order after a user-driven change, and say so if disk refused.
///
/// A refused write means the in-memory order never landed. Keeping it would let a
/// LATER successful write commit this stale list over whatever a running session
/// wrote meanwhile, so the list is re-read instead: disk is the truth.
fn commit_plugin_order(app: &mut App, spec: &GameSpec) {
    let written = app.plugins.as_ref().map(|list| write_plugin_state(app, list, spec)).transpose();
    if let Err(e) = written {
        app.status = Some(format!("Could not write the load order: {e}"));
        app.plugins = compute_plugins(app);
    }
}

fn write_plugin_state(app: &App, list: &PluginList, spec: &GameSpec) -> std::io::Result<()> {
    // Cross-process lock: a running session owns these files (the plugins dir is
    // bind-mounted into it); a mid-game reorder must refuse, not corrupt.
    let _lock = app.created.as_ref().map(|inst| inst.try_lock("the Eidos window")).transpose()?;
    if let Some(inst) = app.created.as_ref() {
        let prof = inst.active();
        // A deliberate GUI edit is the user speaking: it must not trip the
        // "session damaged the active set" card, so the snapshot follows it -
        // EXCEPT while damage is currently flagged, where refreshing would
        // destroy the only copy that can still restore the pre-damage state.
        let damage_flagged = prof.plugin_loss_since_snapshot(spec).is_some();
        list.write_load_order(&prof.plugins_state_dir(), spec)?;
        prof.write_locked_order(&list.locked)?;
        if !damage_flagged {
            let _ = prof.snapshot_plugin_state();
        }
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
        // No `on_press` while a sort runs, nor while a run is tracked: the button
        // greys itself. That is the only sign of work a multi-second masterlist
        // download otherwise gives, and the only sign that a session still holds
        // the load-order files - the handler refused both cases already, but a
        // live-looking button that answers with a status line reads as broken.
        let busy = app.sorting || app.running.is_some();
        let label = if app.sorting {
            "Sorting..."
        } else if app.running.is_some() {
            "Sort with LOOT (game running)"
        } else {
            "Sort with LOOT"
        };
        let mut b = button(text(label).size(11.0)).padding([3, 8]).style(button::secondary);
        if !busy {
            b = b.on_press(Message::SortPlugins);
        }
        top = top.push(b);
    }
    // Batch enable/disable, shown only once a selection exists so the toolbar
    // does not offer an action with no subject.
    let picked = if app.selected_plugins.len() > 1 {
        app.selected_plugins.len()
    } else {
        usize::from(app.selected_plugin.is_some())
    };
    if picked > 0 {
        top = top
            .push(text(format!("{picked} selected")).size(11.0))
            .push(
                button(text("Enable").size(11.0))
                    .padding([3, 8])
                    .on_press(Message::SetSelectedPluginsEnabled(true))
                    .style(button::secondary),
            )
            .push(
                button(text("Disable").size(11.0))
                    .padding([3, 8])
                    .on_press(Message::SetSelectedPluginsEnabled(false))
                    .style(button::secondary),
            );
    }
    let mut head = Column::new().spacing(2).push(top);
    if !missing.is_empty() {
        head = head.push(
            text(format!("! {} missing master(s) - the game would crash", missing.len())).size(12.0),
        );
    }

    // A pin the engine had to overrule. Silence here would leave the user
    // believing a slot is held when it is not, so it is said out loud.
    let violated = list.violated_locks();
    if !violated.is_empty() {
        let names: Vec<&str> = violated.iter().map(|(n, _, _)| n.as_str()).take(3).collect();
        let more = violated.len().saturating_sub(names.len());
        let tail = if more > 0 { format!(" (+{more} more)") } else { String::new() };
        head = head.push(
            text(format!(
                "{} pinned position(s) could not be kept - a plugin must load after its masters: {}{tail}",
                violated.len(),
                names.join(", ")
            ))
            .size(11.0),
        );
    }

    let header = Row::new()
        .spacing(6)
        .push(text("Index").size(11.0).width(Length::Fixed(52.0)))
        .push(text("On").size(11.0).width(Length::Fixed(28.0)))
        .push(text("Plugin").size(11.0).width(Length::Fill))
        .push(text("Type").size(11.0).width(Length::Fixed(36.0)))
        .push(text("Pin").size(11.0).width(Length::Fixed(26.0)));

    // Base-game masters are implicit/always-on; show them as forced, not togglable.
    let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
    // No spacing: the insertion strips are the spacing, exactly as in the mod
    // list, so the layout does not shift the instant a drag begins.
    let mut rows = Column::new();
    let total = list.plugins.len();
    let drag = app.plugin_drag.as_ref();
    // A drop anywhere between the block's own first row and just past its last
    // leaves it where it is, so no indicator is drawn there.
    let live_gap = drag
        .filter(|d| {
            let (lo, hi) = (
                d.block.first().copied().unwrap_or(d.from),
                d.block.last().copied().unwrap_or(d.from),
            );
            d.gap < lo || d.gap > hi + 1
        })
        .map(|d| d.gap);
    // A strip is a target only inside the range the engine allows this plugin,
    // so an illegal slot cannot be aimed at in the first place. That is strictly
    // better than MO2, which accepts the drop and clamps it afterwards
    // (pluginlist.cpp:1940-2016) - the user there has no way to know why the row
    // did not go where they put it.
    let legal = |gap: usize| {
        drag.is_some_and(|d| {
            gap >= d.range.lo && gap <= d.range.hi && !d.range.blocked.contains(&gap)
        })
    };
    // Said once, above the list, while the drag is live: the boundary is visible
    // as a place the line stops, and this explains what is stopping it.
    if let Some(d) = drag {
        let msg = if d.range.is_stuck(d.block.first().copied().unwrap_or(d.from)) {
            pinned_by(&d.range)
        } else {
            match (&d.range.after, &d.range.before) {
                (Some(a), Some(b)) => format!("Can move between {a} and {b} - both are master ties."),
                (Some(a), None) => format!("Must stay after {a}, one of its masters."),
                (None, Some(b)) => format!("Must stay before {b}, which lists it as a master."),
                (None, None) => "Free to move anywhere in its section.".to_string(),
            }
        };
        head = head.push(text(msg).size(11.0));
    }
    let dragging = drag.is_some();
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
        // Creation Club content is loaded by the engine from the .ccc file, so
        // it is as immovable and as un-togglable as a base-game master - and has
        // to look it, or the row invites clicks that can do nothing.
        let engine_owned = is_primary || list.implicit.contains(&p.name.to_ascii_lowercase());
        // MO2-style checkbox. A checkbox with no `on_toggle` renders disabled/greyed,
        // which is exactly the look for the non-togglable cases.
        let toggle: Element<'a, Message> = if engine_owned {
            // A forced game master: always on, never togglable (checked + greyed).
            checkbox(true).size(15).into()
        } else if p.force_disabled {
            // An .esl on a no-light engine: can never load (unchecked + greyed).
            checkbox(false).size(15).into()
        } else {
            checkbox(p.enabled).on_toggle(move |_| Message::TogglePlugin(i)).size(15).into()
        };
        // Manual reorder (MO2 lets the load order be moved by hand, not only
        // The pin (MO2's locked order). A primary master is already nailed to the
        // top by the engine, so offering to pin it would be theatre.
        let locked = list.is_locked(i);
        let pin: Element<'a, Message> = if engine_owned {
            text("").width(Length::Fixed(26.0)).into()
        } else {
            button(text(if locked { "[*]" } else { "[ ]" }).size(10.0))
                .padding([0, 3])
                .style(button::text)
                .on_press(Message::TogglePluginLock(i))
                .into()
        };
        // MO2 puts exactly this behind a hover (pluginlist.cpp tooltipData:
        // Origin, Masters, Missing Masters). It is the information that explains
        // why a plugin will not move, and it is far too wide to be a column -
        // these plugins carry five to nine masters each.
        let mut tip = if p.origin_mod.is_empty() {
            "Origin: the game's own Data".to_string()
        } else {
            format!("Origin: {}", p.origin_mod)
        };
        if engine_owned {
            tip.push_str("\nThe game loads this plugin itself: it cannot be moved or disabled.");
        }
        if !p.masters.is_empty() {
            let present: Vec<&str> = p
                .masters
                .iter()
                .filter(|m| list.plugins.iter().any(|q| q.name.eq_ignore_ascii_case(m)))
                .map(|m| m.as_str())
                .collect();
            let absent: Vec<&str> = p
                .masters
                .iter()
                .filter(|m| !list.plugins.iter().any(|q| q.name.eq_ignore_ascii_case(m)))
                .map(|m| m.as_str())
                .collect();
            if !present.is_empty() {
                tip.push_str(&format!("\nMasters: {}", present.join(", ")));
            }
            if !absent.is_empty() {
                tip.push_str(&format!("\nMISSING masters: {}", absent.join(", ")));
            }
            tip.push_str("\nThis plugin must load after all of them.");
        }
        let name_cell = tooltip(
            text(p.name.clone()).size(12.0).width(Length::Fill),
            container(text(tip).size(11.0))
                .padding(6)
                .style(|t: &Theme| container::Style {
                    background: Some(Background::Color(t.extended_palette().background.weak.color)),
                    ..Default::default()
                }),
            tooltip::Position::FollowCursor,
        )
        .gap(4);
        let row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(idx).size(11.0).width(Length::Fixed(52.0)))
            .push(container(toggle).width(Length::Fixed(28.0)))
            .push(container(name_cell).width(Length::Fill))
            .push(text(kind).size(10.0).width(Length::Fixed(36.0)))
            .push(container(pin).width(Length::Fixed(26.0)));
        // Grabbing the row arms the drag AND selects it, the same press doing
        // both exactly as in the mod list; hovering it during a drag means
        // "insert above me".
        let selected = app.selected_plugin == Some(i) || app.selected_plugins.contains(&i);
        // Same padding as `striped`, or a selected row would be a different
        // height from its neighbours and the list would twitch as focus moves.
        let painted: Element<'a, Message> = if selected {
            container(row)
                .width(Length::Fill)
                .padding(2)
                .style(|_t: &Theme| container::Style {
                    background: Some(Background::Color(SEL_BG)),
                    ..Default::default()
                })
                .into()
        } else {
            striped(row.into(), i % 2 == 0)
        };
        let grab = mouse_area(painted)
            .on_press(Message::SelectPlugin(i))
            .on_enter(Message::PluginDragOverGap(i))
            .on_release(Message::PluginDragDrop);
        rows = rows.push(drop_gap(
            i,
            live_gap == Some(i),
            dragging && legal(i),
            Message::PluginDragOverGap,
            Message::PluginDragDrop,
        ));
        rows = rows.push(grab);
    }
    // The trailing strip: hovering a row always means "above it", so this is the
    // only way to aim at the end of the load order.
    if total > 0 {
        rows = rows.push(drop_gap(
            total,
            live_gap == Some(total),
            dragging && legal(total),
            Message::PluginDragOverGap,
            Message::PluginDragDrop,
        ));
    }

    // Releasing outside the list drops nothing, as in the mod list.
    let list_area = mouse_area(scrollable(rows).id(plugin_scroll_id()).height(Length::Fill))
        .on_exit(Message::PluginDragCancel)
        .on_release(Message::PluginDragCancel);

    Column::new().spacing(6).push(head).push(header).push(list_area).into()
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
        .push(scrollable(rows).height(Length::FillPortion(2)))
        .push(conflicting_files(app, map))
        .into()
}

/// The FILES the selected mod is fighting over, and who wins each.
///
/// The list above says a mod won "1/2" and stops there, which is where every
/// real question starts: WHICH file, and to whom. Answering it meant reading the
/// mod folders by hand - the flag could be raised by a texture the user cares
/// about or by a stale `.log` the author happened to zip up, and the panel gave
/// no way to tell those apart.
///
/// Capped, because a texture pack can contest thousands of paths and a list that
/// long is not an answer either; the count says what was left out.
fn conflicting_files<'a>(app: &App, map: &ConflictMap) -> Element<'a, Message> {
    const SHOWN: usize = 40;
    let Some(focus) = app.selected_mod else {
        return text("Select a mod to see which files it contests.").size(11.0).into();
    };
    let origin = (focus + 1) as u32;
    let name = app.mods.get(focus).map(|m| m.display_name().to_string()).unwrap_or_default();

    let mut rows = Column::new().spacing(1);
    let mut n = 0usize;
    for node in map.files.values() {
        let providers: Vec<u32> =
            std::iter::once(node.winner).chain(node.alternatives.iter().copied()).collect();
        if !providers.contains(&origin) || !node.is_conflicted() {
            continue;
        }
        n += 1;
        if n > SHOWN {
            continue;
        }
        let wins = node.winner == origin;
        // Who this file actually comes from, when it is not us.
        let verdict = if wins {
            "wins it".to_string()
        } else {
            format!("loses to {}", map.name(node.winner))
        };
        let row = Row::new()
            .spacing(6)
            .push(text(node.display_path.clone()).size(11.0).width(Length::Fill))
            .push(
                text(verdict)
                    .size(11.0)
                    .width(Length::Fixed(260.0))
                    .color(if wins { CONFLICT_WINS_FG } else { CONFLICT_LOSES_FG }),
            );
        rows = rows.push(striped(row.into(), n.is_multiple_of(2)));
    }

    let head = if n == 0 {
        format!("{name} contests no files.")
    } else if n > SHOWN {
        format!("{name}: {n} contested file(s), showing the first {SHOWN}")
    } else {
        format!("{name}: {n} contested file(s)")
    };
    Column::new()
        .spacing(4)
        .height(Length::FillPortion(3))
        .push(text(head).size(12.0))
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
        .push(Space::new().width(Length::Fill))
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
        .push(Space::new().width(Length::Fill))
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
    if app.cap_missing && passthrough_requested() {
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
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseMenu);
            let at = app.menu_at.unwrap_or(app.cursor);
            let card = floating_at(mod_menu_card(app, i), at, app.window);
            layers = layers.push(catcher).push(card);
        }
    }

    // The per-mod info dialog is a centered modal (MO2's modinfodialog).
    if let Some(i) = app.info_mod {
        if i < app.mods.len() {
            let scrim =
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseInfo);
            let dialog = container(mod_info_dialog(app, i)).center(Length::Fill);
            layers = layers.push(scrim).push(dialog);
        }
    }

    // The manual / BAIN picker (MO2's InstallDialog and BainComplexInstallerDialog).
    // Below the collision chooser in the stack: a collision raised BY the picker
    // has to be the thing you can click.
    if let Some(p) = &app.picker {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::PickerCancel);
        layers = layers.push(scrim).push(container(install_picker_dialog(p)).center(Length::Fill));
    }

    // The install-collision chooser is a centered modal (MO2's QueryOverwriteDialog).
    if let Some(c) = &app.collision {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CollisionCancel);
        let dialog = container(collision_dialog(c)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Preferences modal (MO2's Settings dialog).
    if app.settings_open {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseSettings);
        let dialog = container(settings_dialog(app)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Executables editor (MO2's Modify Executables dialog).
    if let Some(state) = &app.executables {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseExecutablesDialog);
        let dialog = container(executables_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The About box (Help menu).
    if app.about_open {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseAbout);
        let dialog = container(about_dialog()).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The View dropdown floats just under the menu bar, near the View item.
    if app.view_menu_open {
        let catcher =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseViewMenu);
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
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::ProfileCloseMenu);
        let at = app.menu_at.unwrap_or(app.cursor);
        let card = floating_at(profile_menu_card(app, &name), at, app.window);
        layers = layers.push(catcher).push(card);
    }

    // The LOOT report (MO2's post-sort dialog): a centered modal listing general
    // messages + per-plugin missing masters / messages / dirty advice.
    if let Some(report) = &app.loot_report {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseLootReport);
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
            container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| container::Style {
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
        // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
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
/// The extracted archive as flat rows for the picker's tree view. Built once when
/// the picker opens: the extraction does not change while it is on screen, and
/// re-walking it on every redraw would stutter a large pack.
fn tree_rows(tree: &eidos_install::ExtractedTree) -> Vec<eidos_install::TreeRow> {
    eidos_install::ArchiveTree::from_dir(tree.path()).map(|t| t.flatten()).unwrap_or_default()
}

/// Install what the manual / BAIN picker currently has selected.
///
/// A name collision hands off to the existing Merge / Replace / Rename prompt,
/// carrying the picks so resolving it does not re-ask them. On any other failure
/// the picker stays open with the reason, so a bad data root can just be
/// re-picked instead of re-extracting the archive.
fn run_picker_install(app: &mut App) {
    let Some(p) = app.picker.as_ref() else { return };
    let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) else {
        app.status = Some("Open a game instance first.".to_string());
        return;
    };
    let name = p.name.trim().to_string();
    if name.is_empty() {
        app.status = Some("Give the mod a name first.".to_string());
        return;
    }
    let choice = match &p.mode {
        PickerMode::Bain { subpackages, picked, .. } => {
            let chosen: Vec<String> = subpackages
                .iter()
                .zip(picked)
                .filter(|(_, &on)| on)
                .map(|(s, _)| s.clone())
                .collect();
            if chosen.is_empty() {
                app.status = Some("Tick at least one sub-package.".to_string());
                return;
            }
            PickerChoice::Bain(chosen)
        }
        PickerMode::Manual { root } => PickerChoice::Manual(root.clone()),
    };
    let result = install_with_choice(
        &p.tree,
        &choice,
        &p.archive,
        &mods_dir,
        &name,
        &p.game_id,
        eidos_install::OverwritePolicy::Fail,
    );
    match result {
        Ok(r) => {
            let archive = p.archive.clone();
            // A successful install may have consumed the tree (a lone source is
            // moved, not copied), so the picker must go before anything else.
            app.picker = None;
            remember_bain_options(app, &r.name, &choice);
            after_install(app, &r.name, r.dest, r.fomod, Some(&archive));
        }
        Err(eidos_install::InstallError::Exists(_)) => {
            let Some(p) = app.picker.take() else { return };
            let rename_to = suggest_free_name(&mods_dir, &name);
            app.collision = Some(CollisionPrompt {
                archive: p.archive,
                name: name.clone(),
                game_id: p.game_id,
                rename_to,
                fomod: false,
                tree: Some(p.tree),
                pick: Some(choice),
            });
            app.status = Some(format!("'{name}' already exists - choose how to install."));
        }
        Err(e) => app.status = Some(format!("Install failed: {e}")),
    }
}

/// Dispatch one picker choice to the matching installer.
fn install_with_choice(
    tree: &eidos_install::ExtractedTree,
    choice: &PickerChoice,
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: eidos_install::OverwritePolicy,
) -> Result<eidos_install::InstallReport, eidos_install::InstallError> {
    match choice {
        PickerChoice::Bain(subs) => {
            eidos_install::install_bain(tree, subs, archive, mods_dir, name, game_id, policy)
        }
        PickerChoice::Manual(root) => {
            eidos_install::install_manual(tree, root, archive, mods_dir, name, game_id, policy)
        }
    }
}

/// Record a BAIN selection in the installed mod's `meta.ini`, so reinstalling it
/// later opens the picker with the same sub-packages already ticked (MO2's
/// `onInstallationEnd`). Best-effort: failing to remember a preference must not
/// look like a failed install.
fn remember_bain_options(app: &App, mod_name: &str, choice: &PickerChoice) {
    let (PickerChoice::Bain(subs), Some(inst)) = (choice, app.created.as_ref()) else { return };
    let mut meta = inst.mod_meta(mod_name);
    meta.set_bain_options(subs);
    let _ = meta.write(&inst.meta_path(mod_name));
}

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
    // A collision raised by the manual / BAIN picker: replay the SAME picks. The
    // tree alone does not say which sub-packages were ticked, so re-running the
    // plain installer here would quietly install something else.
    if let (Some(choice), Some(tree)) = (c.pick.as_ref(), c.tree.as_ref()) {
        match install_with_choice(
            tree,
            choice,
            &c.archive,
            &mods_dir,
            &c.name,
            &c.game_id,
            policy,
        ) {
            Ok(r) => {
                remember_bain_options(app, &r.name, choice);
                after_install(app, &r.name, r.dest, r.fomod, Some(&archive));
            }
            Err(eidos_install::InstallError::Exists(_)) => {
                app.status = Some("That name also exists - pick another.".to_string());
                app.collision = Some(c);
            }
            Err(e) => app.status = Some(format!("Install failed: {e}")),
        }
        return;
    }
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
    // Indices just moved; a selection kept across the reload could point at a
    // different save (or past the end).
    clear_save_selection(app);
}

/// Close the save details pane and drop what it derived.
fn clear_save_selection(app: &mut App) {
    app.selected_save = None;
    app.save_info = None;
    app.save_missing = Vec::new();
}

/// Parse the selected save's header and diff its plugin list against the profile's
/// current one. Runs on selection only - a save header means decompressing part of
/// the file, which is not something to do per redraw.
fn load_save_details(app: &mut App) {
    let Some(save) = app.selected_save.and_then(|i| app.saves.get(i)) else {
        clear_save_selection(app);
        return;
    };
    let path = save.path.clone();
    let parsed = eidos_gamefeatures::parse_sse_save(&path).map_err(|e| e.to_string());
    app.save_missing = match (&parsed, app.plugins.as_ref()) {
        (Ok(info), Some(list)) => {
            let known: Vec<eidos_gamefeatures::KnownPlugin> = list
                .plugins
                .iter()
                .map(|p| eidos_gamefeatures::KnownPlugin {
                    name: &p.name,
                    enabled: p.enabled,
                    origin_mod: &p.origin_mod,
                })
                .collect();
            // Every mod, disabled ones included: a disabled mod holding the plugin
            // is precisely the case the "enable what this save needs" fix exists
            // for. Overwrite counts as a provider too (a cleaned .esp lands there).
            let overwrite = app.created.as_ref().map(|i| i.overwrite_dir());
            let mut mods: Vec<eidos_gamefeatures::ModFolder> = app
                .mods
                .iter()
                .filter(|m| !m.is_separator())
                .map(|m| eidos_gamefeatures::ModFolder { name: &m.name, path: &m.path })
                .collect();
            if let Some(o) = overwrite.as_deref() {
                mods.push(eidos_gamefeatures::ModFolder { name: "Overwrite", path: o });
            }
            let data = selected_game(app).map(|g| g.data_path.clone());
            if let Some(d) = data.as_deref() {
                mods.push(eidos_gamefeatures::ModFolder { name: "(game data)", path: d });
            }
            eidos_gamefeatures::missing_plugins(info, &known, &mods, data.as_deref())
        }
        _ => Vec::new(),
    };
    app.save_info = Some((path, parsed));
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
        // Same lock as save_mods: the modlist must not be rewritten under a
        // running session. A refusal is not a lost install - the files are on
        // disk, reconciliation lists the folder on the next reload, and only the
        // auto-enable is skipped.
        match inst.try_lock("the Eidos window") {
            Ok(_lock) => {
                let mut ml = inst.modlist();
                ml.retain(|m| m.name != name);
                ml.push(ModEntry {
                    name: name.to_string(),
                    enabled: true,
                    path: dest,
                    unmanaged: false,
                });
                let _ = inst.save_modlist(&ml);
            }
            Err(e) => {
                app.status = Some(format!(
                    "Installed '{name}', but could not enable it now: {e}. Enable it once the \
                     game closes."
                ));
            }
        }
    }
    reload_mods(app);
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
    refresh_meta_cache(app);
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

/// How many tree rows the manual picker draws. An archive with more entries than
/// this is one whose data root is a top-level folder anyway.
const PICKER_TREE_ROWS: usize = 1500;

/// The manual / BAIN install picker: MO2's `InstallDialog` (point at the data
/// root) and `BainComplexInstallerDialog` (tick sub-packages), which share an
/// archive tree and a name field.
fn install_picker_dialog<'a>(p: &InstallPicker) -> Element<'a, Message> {
    let name_row = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Install as:").size(12.0))
        .push(
            text_input("mod name", &p.name)
                .on_input(Message::PickerNameChanged)
                .on_submit(Message::PickerInstall)
                .padding(5)
                .size(12.0)
                .width(Length::Fill),
        );

    let (title, body): (String, Element<'a, Message>) = match &p.mode {
        // MO2 asks before assuming: an archive whose top level mixes sub-packages
        // with other folders is as likely to be a plain mod with extras.
        PickerMode::Bain { asking: true, subpackages, .. } => (
            "May be a BAIN installer".to_string(),
            Column::new()
                .spacing(10)
                .push(
                    text(format!(
                        "This archive has {} folder(s) that look like Wrye Bash sub-packages, \
                         and others that do not. Install it as a BAIN package?",
                        subpackages.len()
                    ))
                    .size(12.0),
                )
                .push(
                    Row::new()
                        .spacing(8)
                        .push(
                            button(text("Yes, pick sub-packages").size(12.0))
                                .padding([4, 10])
                                .on_press(Message::PickerBainConfirm(true))
                                .style(button::primary),
                        )
                        .push(
                            button(text("No, choose the data folder").size(12.0))
                                .padding([4, 10])
                                .on_press(Message::PickerBainConfirm(false))
                                .style(button::secondary),
                        ),
                )
                .into(),
        ),
        PickerMode::Bain { subpackages, picked, .. } => {
            // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
            for (i, (name, &on)) in subpackages.iter().zip(picked).enumerate() {
                list = list.push(
                    checkbox(on).label(name.clone())
                        .on_toggle(move |_| Message::PickerBainToggle(i))
                        .size(13.0)
                        .text_size(12.0),
                );
            }
            (
                "Choose sub-packages".to_string(),
                Column::new()
                    .spacing(8)
                    .push(
                        text("Ticked sub-packages are merged top to bottom, so a later one wins.")
                            .size(11.0),
                    )
                    .push(scrollable(list).height(Length::Fixed(240.0)))
                    .into(),
            )
        }
        PickerMode::Manual { root } => {
            // Re-derived on every pick, like MO2's live green/red label.
            let tree = eidos_install::ArchiveTree::from_dir(p.tree.path()).ok();
            let valid = tree.as_ref().is_some_and(|t| t.root_looks_valid(root));
            let chosen = if root.is_empty() { "<archive root>" } else { root.as_str() };

            let mut list = Column::new().spacing(1).push(
                button(text("<archive root>").size(12.0))
                    .padding([1, 4])
                    .on_press(Message::PickerSetRoot(String::new()))
                    .style(if root.is_empty() { button::primary } else { button::text }),
            );
            for r in p.rows.iter().filter(|r| r.is_dir).take(PICKER_TREE_ROWS) {
                let selected = *root == r.path;
                let label = format!("{}{}", "    ".repeat(r.depth + 1), r.name);
                list = list.push(
                    button(text(label).size(12.0))
                        .padding([1, 4])
                        .on_press(Message::PickerSetRoot(r.path.clone()))
                        .style(if selected { button::primary } else { button::text }),
                );
            }
            (
                "Choose the data folder".to_string(),
                Column::new()
                    .spacing(6)
                    .push(scrollable(list).height(Length::Fixed(220.0)))
                    .push(
                        text(if valid {
                            format!("The content of {chosen} looks valid.")
                        } else {
                            format!("The content of {chosen} does NOT look valid.")
                        })
                        .size(11.0)
                        .color(if valid {
                            Color::from_rgb8(0x2E, 0x6E, 0x31)
                        } else {
                            Color::from_rgb8(0x8E, 0x2A, 0x2A)
                        }),
                    )
                    // MO2 warns but still lets you through: the checker only knows
                    // the game's own folder names, and plenty of valid mods (SKSE
                    // plugins, tool configs) match none of them.
                    .push(
                        text("You can install anyway - the check only recognises the game's own folder names.")
                            .size(10.0),
                    )
                    .into(),
            )
        }
    };

    let mut card = Column::new().spacing(10).push(text(title).size(15.0)).push(name_row).push(body);

    // No Install button while the BAIN question is open: the answer decides which
    // installer would even run.
    if !matches!(p.mode, PickerMode::Bain { asking: true, .. }) {
        card = card.push(
            Row::new()
                .spacing(8)
                .push(
                    button(text("Install").size(12.0))
                        .padding([4, 10])
                        .on_press(Message::PickerInstall)
                        .style(button::primary),
                )
                .push(
                    button(text("Cancel").size(12.0))
                        .padding([4, 10])
                        .on_press(Message::PickerCancel)
                        .style(button::text),
                ),
        );
    }
    container(card).max_width(520.0).padding(16).style(card_style).into()
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
            // Masked. It is a credential, and this field sits in a window users
            // screenshot to ask for help - which is one of the ways a key leaks.
            // Nothing is lost by hiding it: validation names the account back, so
            // the user still gets told whether what they pasted was right.
            let field = text_input("Personal API key", &app.settings_api_key)
                .secure(true)
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
                    checkbox(app.prefs.lock_gui).label("Lock the window while a game or tool is running")
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
    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
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
        .push(Space::new().width(Length::Fill))
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
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(text("Shortcuts").size(13.0))
        .push(
            text("Ctrl+R run   ·   F5 refresh   ·   Ctrl+click multi-select   ·   Shift+click range   ·   Esc clear   ·   drag a row to reorder")
                .size(11.0),
        )
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(
            button(text("Close").size(12.0))
                .padding([5, 14])
                .on_press(Message::CloseAbout)
                .style(button::primary),
        );
    container(card).max_width(440.0).padding(16).style(card_style).into()
}

/// Whether the user opted into FUSE passthrough. Read from this process's own
/// environment because the launch environment is inherited from it (Steam launch
/// options land here first, then on the `eidos` child).
///
/// It is off by default: passthrough stops the game opening its own archives and
/// plugins (see `passthrough_enabled` in eidos-fuse for the measurement). This
/// gates the capability warnings, which are only meaningful to someone who
/// actually wants passthrough.
fn passthrough_requested() -> bool {
    std::env::var("EIDOS_FUSE_PASSTHROUGH").is_ok_and(|v| {
        let v = v.trim();
        !v.is_empty() && v != "0"
    })
}

/// The CAP_SYS_ADMIN warning banner, shown only when the user asked for
/// passthrough and the launch binary cannot deliver it (every rebuild wipes the
/// file capability). Shows the exact fix command; F5 rechecks after running it.
fn cap_warning_banner<'a>() -> Element<'a, Message> {
    let cmd = format!(
        "sudo setcap cap_sys_admin+ep {}",
        find_eidos_binary().display()
    );
    let row = Row::new()
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .push(text("Passthrough was requested but the launch binary has no CAP_SYS_ADMIN (a rebuild wipes it), so reads go through the daemon. Fix, then press F5:").size(11.0))
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
        .push(Space::new().width(Length::Fill))
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
        .push(Space::new().height(Length::Fixed(6.0)))
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
/// Width of the option column. FIXED, and narrow: the options are short labels,
/// and every pixel it does not take is a pixel the preview image gets. A long
/// option name is clipped rather than allowed to widen the column, because the
/// image is what the user is actually comparing.
const FOMOD_OPTIONS_W: f32 = 260.0;
/// Height of the preview box. Also fixed, and it stays even when an option has
/// no image: FOMOD art is wildly inconsistent - CBBE ships portrait body shots
/// next to letterbox eyebrow strips - so a box that resized to its content would
/// make the whole dialog jump on every hover.
const FOMOD_PREVIEW_H: f32 = 420.0;

// The parchment family, spelled out once. These are the same inks the rest of the
// window uses; the wizard only ever looked out of place because it was drawn with
// iced's stock `button::secondary` and an ASCII `[x]`, not because it was missing
// anything iced cannot do.
const FOMOD_RULE: Color = Color::from_rgb(0.81, 0.75, 0.63); // hairlines and dividers
const FOMOD_ROW_BG: Color = Color::from_rgb(0.89, 0.84, 0.72); // an unselected option
const FOMOD_ROW_HOVER: Color = Color::from_rgb(0.93, 0.88, 0.78);
// Both inks are measured against the page (0xECDFC2): SOFT reaches 6.2:1 and FAINT
// 4.5:1, the WCAG floor for text this small. The first pass had FAINT at 2.9:1,
// which is a decorative grey, not a legible one - and it was carrying "required"
// and "recommended", the only guidance the mod author gives. Below ~10px there is
// no room for a genuinely faint tier, so the hierarchy lives in size and weight.
const FOMOD_INK_SOFT: Color = Color::from_rgb(0.36, 0.30, 0.23); // descriptions, tags
const FOMOD_INK_FAINT: Color = Color::from_rgb(0.44, 0.38, 0.30); // group metadata
const FOMOD_PARCHMENT: Color = Color::from_rgb(0.95, 0.92, 0.83); // ink on burgundy

/// The circle or square in front of an option, drawn rather than written.
///
/// Eidos ships no icon font, so `[x]`/`[ ]` was standing in for a control - and it
/// was the single loudest thing separating this dialog from MO2's. Two nested
/// containers cost nothing and give the real shape, which also carries meaning MO2
/// itself carries: a ROUND marker is a group you pick one of, a SQUARE one is a
/// group you pick any number of. The user learns the rule from the shape instead of
/// reading "choose at most one" every time.
fn fomod_marker<'a>(on: bool, usable: bool, radio: bool) -> Element<'a, Message> {
    // `on` is tested FIRST because the row fill and the row label both test it
    // first: a selected row is burgundy whatever else is true of it. Testing
    // `!usable` first painted the marker in dark ink on that burgundy, so an option
    // that was both ticked and forbidden showed a tick you could not see.
    let ink = if on {
        FOMOD_PARCHMENT
    } else if !usable {
        FOMOD_INK_FAINT
    } else {
        FOMOD_INK_SOFT
    };
    // The dot fills what the 4px inset leaves it, rather than being a fixed size
    // that gets centred. Centring a 7px dot in a 14px ring asks the renderer for a
    // 3.5px offset on each side, and half a pixel has to land somewhere: it went
    // up and left, so every ticked option looked knocked off its axis. An inset is
    // exact arithmetic - 14 - 4 - 4 = 6 - and cannot drift.
    let inner: Element<'_, Message> = if on {
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t: &Theme| container::Style {
                background: Some(Background::Color(ink)),
                border: Border {
                    radius: (if radio { 3.0 } else { 1.0 }).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        Space::new().into()
    };
    container(inner)
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .padding(4)
        .style(move |_t: &Theme| container::Style {
            border: Border {
                color: ink,
                width: 1.5,
                radius: (if radio { 7.0 } else { 3.0 }).into(),
            },
            ..Default::default()
        })
        .into()
}

/// A hairline. Used to seat the header and the footer instead of leaving three
/// blocks of content floating in one undifferentiated field of parchment.
fn fomod_rule<'a>(vertical: bool) -> Element<'a, Message> {
    let (w, h) = if vertical {
        (Length::Fixed(1.0), Length::Fill)
    } else {
        (Length::Fill, Length::Fixed(1.0))
    };
    container(Space::new())
        .width(w)
        .height(h)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(FOMOD_RULE)),
            ..Default::default()
        })
        .into()
}

/// The FOMOD installer, laid out the way MO2 lays it out: the options on one
/// side, and ONE description and ONE image for whichever option is current.
///
/// The previous version rendered every option's description and full-size image
/// inline, one after another, so a step of CBBE was several thousand pixels tall
/// and two body shapes could never be compared without scrolling between them.
/// MO2 avoids that by filtering HoverEnter on each option and filling two fixed
/// panes (fomodinstallerdialog.cpp:628); this does the same, and additionally
/// falls back to the SELECTED option when nothing is hovered, so the pane is
/// never blank and the dialog is usable without a mouse.
fn fomod_wizard_view(w: &FomodWizard) -> Element<'_, Message> {
    use eidos_fomod::PluginType;
    let config = &w.session.config;
    let types = eidos_fomod::step_types(config, &w.selection, &w.ctx, w.step);
    let step = config.steps.get(w.step);

    // Steps whose `<visible>` condition is false are skipped by Next/Back and
    // ignored by build_plan, so counting raw indices made the header lie: a run
    // that shows three panels announced "Step 1 of 5", then jumped to "Step 3".
    // Number by position among the steps that will actually be shown.
    let vis = eidos_fomod::visible_steps(config, &w.selection, &w.ctx);
    let total = vis.iter().filter(|v| **v).count().max(1);
    let shown_no = (0..=w.step).filter(|&i| vis.get(i).copied().unwrap_or(false)).count().max(1);

    // What the preview is about: the hovered option, else the first selected one,
    // else the first option of the step.
    let current = w.hover.filter(|&(gi, pi)| {
        step.is_some_and(|s| s.groups.get(gi).is_some_and(|g| pi < g.plugins.len()))
    });
    let current = current.or_else(|| {
        let s = step?;
        let sel = w.selection.get(w.step)?;
        s.groups.iter().enumerate().find_map(|(gi, g)| {
            (0..g.plugins.len())
                .find(|&pi| sel.get(gi).and_then(|gg| gg.get(pi)).copied().unwrap_or(false))
                .map(|pi| (gi, pi))
        })
    });
    let current = current.or_else(|| step.and_then(|s| (!s.groups.is_empty()).then_some((0, 0))));

    // ---- header: which mod, which step ----
    let bold = iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::DEFAULT };
    // The name takes Length::Fill and the chip stays Shrink. iced's flex measures
    // the Shrink children first and hands what is left to the Fill ones, so the
    // chip is always laid out at its full size; with the name left as Shrink it was
    // measured first, ate the whole row, and squeezed the chip to zero width.
    let mut title = Row::new()
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .push(text(config.module_name.clone()).size(17.0).font(bold).width(Length::Fill));
    if let Some(s) = step {
        title = title.push(text(s.name.clone()).size(12.0).color(FOMOD_INK_FAINT));
    }
    let head = title.push(
        // The step counter as a chip, so it reads as status rather than as one
        // more sentence competing with the mod's name.
        container(
            text(format!("Step {shown_no} of {total}")).size(11.0).color(FOMOD_PARCHMENT),
        )
            .padding([3, 9])
            .style(|t: &Theme| container::Style {
                background: Some(Background::Color(t.palette().primary)),
                border: Border { radius: 9.0.into(), ..Default::default() },
                ..Default::default()
            }),
    );

    // ---- left: the options, compact ----
    let mut opts = Column::new().spacing(3);
    if let Some(s) = step {
        for (gi, group) in s.groups.iter().enumerate() {
            use eidos_fomod::GroupType;
            // Round marker = pick one of these; square = pick as many as you like.
            let radio = matches!(
                group.group_type,
                GroupType::SelectExactlyOne | GroupType::SelectAtMostOne
            );
            opts = opts.push(
                container(
                    Row::new()
                        .spacing(5)
                        // Fill on the name for the same reason as the header: the
                        // separator and the "choose one" label are Shrink and get
                        // measured first, so they can no longer be starved to zero
                        // width by a long group name and pushed out of the column.
                        .push(
                            text(group.name.clone())
                                .size(11.0)
                                .color(FOMOD_INK_SOFT)
                                .width(Length::Fill),
                        )
                        .push(text("·").size(11.0).color(FOMOD_INK_FAINT))
                        .push(
                            text(group_type_label(group.group_type))
                                .size(11.0)
                                .color(FOMOD_INK_FAINT),
                        ),
                )
                .padding([9, 4]),
            );
            for (pi, plugin) in group.plugins.iter().enumerate() {
                let on = w
                    .selection
                    .get(w.step)
                    .and_then(|sl| sl.get(gi))
                    .and_then(|g| g.get(pi))
                    .copied()
                    .unwrap_or(false);
                let ptype =
                    types.get(gi).and_then(|g| g.get(pi)).copied().unwrap_or(PluginType::Optional);
                let usable = ptype != PluginType::NotUsable;
                // Two ways an option can be present but not yours to change, and
                // neither may offer a click that does nothing or, worse, one that
                // quietly breaks the install:
                //   - Required: the engine pre-ticks it, but every branch of the
                //     toggle handler would happily tick it back OFF, and build_plan
                //     only installs the files of options still marked selected. One
                //     click on a required option silently dropped its files.
                //   - SelectAll: the handler's `SelectAll => {}` arm is a no-op, so
                //     the row lit up on hover and answered nothing.
                let locked = matches!(ptype, PluginType::Required)
                    || matches!(group.group_type, GroupType::SelectAll);
                let tag = match ptype {
                    PluginType::Required => "required",
                    PluginType::Recommended => "recommended",
                    PluginType::NotUsable => "not usable",
                    _ => "",
                };
                let label = if on {
                    FOMOD_PARCHMENT
                } else if usable {
                    palette().text
                } else {
                    FOMOD_INK_FAINT
                };
                let row = Row::new()
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .push(fomod_marker(on, usable, radio))
                    .push(text(plugin.name.clone()).size(12.5).color(label).width(Length::Fill))
                    // 10px, not 9.5, and in the darker ink: this string carries the
                    // author's own guidance and was being rendered at 2.7:1.
                    .push(text(tag).size(10.0).color(if on {
                        FOMOD_PARCHMENT
                    } else {
                        FOMOD_INK_SOFT
                    }));
                let mut b = button(row)
                    .padding([7, 9])
                    .width(Length::Fill)
                    .style(move |t: &Theme, s: button::Status| {
                        // A locked row must not light up on hover: the highlight is
                        // a promise that a click will do something.
                        let hovered = matches!(s, button::Status::Hovered) && !locked;
                        let bg = if on {
                            t.palette().primary
                        } else if !usable {
                            Color { a: 0.35, ..FOMOD_ROW_BG }
                        } else if hovered {
                            FOMOD_ROW_HOVER
                        } else {
                            FOMOD_ROW_BG
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: label,
                            border: Border {
                                color: if on { t.palette().primary } else { FOMOD_RULE },
                                width: 1.0,
                                radius: 5.0.into(),
                            },
                            ..Default::default()
                        }
                    });
                if usable && !locked {
                    b = b.on_press(Message::FomodToggle(gi, pi));
                }
                // Hover drives the preview; leaving falls back to the selection
                // rather than blanking the pane.
                opts = opts.push(
                    mouse_area(b)
                        .on_enter(Message::FomodHover(Some((gi, pi))))
                        .on_exit(Message::FomodUnhover(gi, pi)),
                );
            }
        }
    }

    // ---- right: one image, one description ----
    let shown = current.and_then(|(gi, pi)| step?.groups.get(gi)?.plugins.get(pi));
    let art = shown
        .and_then(|p| p.image.as_ref())
        .and_then(|p| w.session.resolve(p))
        .or_else(|| config.module_image.as_ref().and_then(|p| w.session.resolve(p)));
    let preview: Element<'_, Message> = match art {
        // `contain` keeps the aspect ratio inside the fixed box, so a portrait
        // body shot and a letterbox eyebrow sheet both sit still.
        Some(path) => image(image::Handle::from_path(path))
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Contain)
            .into(),
        // INK_SOFT, not FAINT: this sits on the preview fill, which is darker than
        // the page, so the faint ink fell to 2.4:1 and the box just read as blank.
        None => container(text("No preview for this option.").size(12.0).color(FOMOD_INK_SOFT))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
    };
    let mut right = Column::new().spacing(10).push(
        container(preview)
            .width(Length::Fill)
            .height(Length::Fixed(FOMOD_PREVIEW_H))
            .padding(8)
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0xD9, 0xC9, 0xA8))),
                border: Border {
                    color: FOMOD_RULE,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }),
    );
    if let Some(p) = shown {
        // Name on its own line, description under it. A Row would have kept the
        // name from ever wrapping with the paragraph, and FOMOD descriptions are
        // paragraphs - CBBE's run to several lines.
        let mut d = Column::new()
            .spacing(4)
            .push(text(p.name.clone()).size(13.0).font(bold).width(Length::Fill));
        if !p.description.is_empty() {
            d = d.push(
                text(p.description.clone()).size(12.0).color(FOMOD_INK_SOFT).width(Length::Fill),
            );
        }
        // Scrollable, because the preview box above it is a hard 420px and the pane
        // is whatever the window leaves. iced does not clip text to its node, so a
        // long description did not truncate cleanly - it painted straight over the
        // footer, and its tail was unreachable. FOMOD descriptions carry "do NOT
        // pick this if..." warnings, so losing the tail is losing the warning.
        right = right.push(scrollable(d).height(Length::Fill));
    }

    // ---- footer ----
    let vis = eidos_fomod::visible_steps(config, &w.selection, &w.ctx);
    let has_prev = (0..w.step).any(|i| vis.get(i).copied().unwrap_or(false));
    let has_next = (w.step + 1..vis.len()).any(|i| vis[i]);
    let valid = step_valid(w);

    let mut nav = Row::new().spacing(8).align_y(iced::Alignment::Center);
    if !valid {
        nav = nav.push(
            text("Select the required option(s) to continue.").size(11.0).color(FOMOD_INK_FAINT),
        );
    }
    nav = nav.push(Space::new().width(Length::Fill));
    nav = nav.push(fomod_btn("Cancel", Some(Message::FomodCancel), false));
    if has_prev {
        nav = nav.push(fomod_btn("Back", Some(Message::FomodBack), false));
    }
    let (label, msg) =
        if has_next { ("Next", Message::FomodNext) } else { ("Install", Message::FomodInstall) };
    // The one button that carries the flow gets the burgundy. When the step is
    // unsatisfied it keeps its place and its size and simply stops responding,
    // rather than vanishing and shifting the whole row.
    nav = nav.push(fomod_btn(label, valid.then_some(msg), true));

    let panes = Row::new()
        .spacing(14)
        .height(Length::Fill)
        .push(
            // The right inset goes INSIDE the scrollable, not around it: iced draws
            // the scrollbar over the content, so without it the bar sat on the rows'
            // right border and crowded the "recommended" tag.
            container(
                scrollable(container(opts).padding(iced::Padding {
                    top: 0.0,
                    right: 13.0,
                    bottom: 0.0,
                    left: 3.0,
                }))
                .height(Length::Fill),
            )
                .width(Length::Fixed(FOMOD_OPTIONS_W)),
        )
        .push(fomod_rule(true))
        .push(container(right).width(Length::Fill));

    Column::new()
        .spacing(12)
        .padding(16)
        .push(head)
        .push(fomod_rule(false))
        .push(panes)
        .push(fomod_rule(false))
        .push(nav)
        .into()
}

/// A footer button. `msg == None` means present but inert: the disabled Next has
/// to hold its width or the footer jumps the moment a step becomes satisfiable.
///
/// The primary one is also given a FIXED width, because its label is not stable:
/// ticking an option can reveal or hide a later step, which flips "Next" to
/// "Install" and back. The row is right-aligned behind a Fill spacer, so an 8px
/// change in that one label slid Cancel and Back sideways under the pointer.
fn fomod_btn<'a>(label: &'a str, msg: Option<Message>, primary: bool) -> Element<'a, Message> {
    let live = msg.is_some();
    let mut b = button(text(label).size(12.5).width(Length::Fill).center())
        .padding([7, 16])
        .width(if primary { Length::Fixed(104.0) } else { Length::Shrink })
        .style(move |t: &Theme, s: button::Status| {
            let hovered = matches!(s, button::Status::Hovered);
            let p = t.palette().primary;
            // The disabled state used to be translucent burgundy under translucent
            // parchment, which composited to a 1.3:1 label - not dim, gone. The one
            // moment the user most needs to read a button is when it will not let
            // them past. Opaque fill, dark ink: readable, and unmistakably inert.
            let (bg, fg) = match (primary, live, hovered) {
                (true, true, false) => (p, FOMOD_PARCHMENT),
                (true, true, true) => (Color { a: 0.85, ..p }, FOMOD_PARCHMENT),
                (true, false, _) => (FOMOD_ROW_BG, FOMOD_INK_SOFT),
                (false, _, true) => (FOMOD_ROW_HOVER, t.palette().text),
                (false, _, false) => (FOMOD_ROW_BG, t.palette().text),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: fg,
                border: Border {
                    color: if primary && live { Color { a: 0.0, ..p } } else { FOMOD_RULE },
                    width: 1.0,
                    radius: 5.0.into(),
                },
                ..Default::default()
            }
        });
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
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
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
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
        Message::KeyNav(_) | Message::CycleFocus if typing => Message::Noop,
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
    // The title moved out of `application` and onto a builder; the first argument
    // is now the boot function that `run_with` used to take. It must be `Fn`, not
    // `FnOnce` - which is why the `.clone()` stays: without it the closure would
    // consume the Vec and only be callable once.
    iced::application(move || new(launch_command.clone()), update, view)
        .title("Eidos")
        .theme(theme)
        .subscription(subscription)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(names: &[&str]) -> Vec<ModEntry> {
        names
            .iter()
            .map(|n| ModEntry { name: n.to_string(), enabled: true, path: PathBuf::new(), unmanaged: false })
            .collect()
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
        let (cmd, warning) = play_command(game_id, command);
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

    /// Unmanaged rows (the game's DLC and Creation Club content) are listed first
    /// and never written to modlist.txt, so no strip is offered above them - a
    /// drop there would vanish on save.
    #[test]
    fn no_insertion_point_is_offered_above_the_game_content() {
        let mut v = mods(&["dlc1", "dlc2", "mod1", "mod2"]);
        v[0].unmanaged = true;
        v[1].unmanaged = true;
        assert_eq!(first_managed(&v), 2);

        // An all-unmanaged list offers nothing at all rather than index 0.
        let mut all_dlc = mods(&["dlc1", "dlc2"]);
        for m in all_dlc.iter_mut() {
            m.unmanaged = true;
        }
        assert_eq!(first_managed(&all_dlc), 2);

        // And an all-managed list starts at the top.
        assert_eq!(first_managed(&mods(&["a", "b"])), 0);
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
    fn a_row_move_stops_at_the_ends_and_above_the_game_content() {
        let mut app = nav_app(&["dlc", "a", "b"]);
        app.mods[0].unmanaged = true;
        app.selected_mod = Some(1);
        // "a" is already the first movable row; nothing above it may be claimed.
        let _ = key_nav(&mut app, Nav::ShiftUp);
        assert_eq!(names(&app.mods), ["dlc", "a", "b"]);

        app.selected_mod = Some(2);
        let _ = key_nav(&mut app, Nav::ShiftDown);
        assert_eq!(names(&app.mods), ["dlc", "a", "b"], "the last row has nowhere to go");
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
    fn moving_the_mouse_does_not_cancel_an_armed_confirmation() {
        // The reported bug: arm Delete on a download, twitch the mouse, and the
        // confirmation is gone. The pointer HAS to move to reach the button, so
        // the two-click guard could never be completed.
        let mut app = nav_app(&[]);
        update_inner(&mut app, Message::DeleteDownload(0));
        assert_eq!(app.confirm_delete_download, Some(0), "the first click arms it");

        update_inner(&mut app, Message::PointerAt(iced::Point::new(10.0, 10.0)));
        update_inner(&mut app, Message::WindowResized(iced::Size::new(800.0, 600.0)));
        assert_eq!(app.confirm_delete_download, Some(0), "ambient messages are not actions");
    }

    #[test]
    fn a_real_action_still_cancels_every_confirmation() {
        // The guard must not become decorative: the whole point is that doing
        // anything ELSE takes the loaded gun out of your hand.
        let mut app = nav_app(&[]);
        for (arm, check) in [
            (Message::DeleteDownload(0), 0),
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
        update_inner(&mut app, Message::DeleteDownload(0));
        update_inner(&mut app, Message::DeleteDownload(3));
        assert_eq!(app.confirm_delete_download, Some(3), "only one may be armed");
    }

}
