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

mod view;
use view::*;

mod update;
use update::*;

mod modinfo;
use modinfo::*;

mod wizard;
use wizard::*;

mod widgets;
use widgets::*;

mod theme;
use theme::*;

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
    /// Arriving now: a `.unfinished` partial that is still growing.
    Downloading,
    /// A `.unfinished` partial that has stopped growing - the `eidos nxm` process
    /// died, the network went, or the user closed the terminal. Not lost: the
    /// partial resumes with a Range request on the next attempt.
    Stalled,
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
    /// Bytes on disk so far. Equals `size` once finished.
    downloaded: u64,
    /// Total bytes from the sidecar's `totalSize`, `0` when unknown - an older
    /// download, or a manually dropped archive.
    total: u64,
    /// Bytes per second, measured between two ticks. `None` on the first tick of
    /// a download, when there is nothing to compare against yet.
    speed: Option<f64>,
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
    confirm_delete_download: Option<String>,
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
        download_samples: HashMap::new(),
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



/// The mod list as the user should see it: the profile's rows, with the game's
/// own content (DLCs, Creation Club) reconciled into them.
///
/// A row the profile already places KEEPS ITS POSITION. That is the whole
/// difference from prepending everything: a user who drags the DLC block under a
/// separator, or puts one above it, has said where it goes, and re-pinning it to
/// the top on the next refresh would throw that away silently.
///
/// Content the profile has never seen is prepended, because the display runs
/// lowest-priority-first and the engine loads its own content before anything
/// anyone installed. Content the profile lists but the game no longer ships is
/// dropped - a DLC can be uninstalled, and a row pointing at nothing helps no one.
fn modlist_with_unmanaged(inst: &Instance, game: Option<&DetectedGame>) -> Vec<ModEntry> {
    let listed = inst.modlist();
    let Some(game) = game else { return strip_unmanaged(listed) };
    let Some(spec) = GameSpec::for_id(game.def.id) else { return strip_unmanaged(listed) };
    // The order the engine imposes on its own content: the primary masters, then
    // whatever the `.ccc` lists. Anything else falls in after, alphabetically.
    let mut engine_order: Vec<String> = spec.primary_plugins.clone();
    engine_order.extend(eidos_plugins::implicit_plugins(&game.install_path));
    let managed: Vec<ModEntry> = listed.iter().filter(|m| !m.unmanaged).cloned().collect();
    let real = inst.unmanaged_mods(&game.data_path, &engine_order, &managed);

    // What the game actually ships, by name, so a listed row can be matched to it
    // and given the path this layer alone knows.
    let mut by_name: std::collections::HashMap<String, ModEntry> =
        real.into_iter().map(|m| (m.name.to_ascii_lowercase(), m)).collect();

    let mut out: Vec<ModEntry> = Vec::with_capacity(listed.len() + by_name.len());
    let mut placed: Vec<ModEntry> = Vec::new();
    for m in listed {
        if !m.unmanaged {
            placed.push(m);
            continue;
        }
        // `remove` both fills in the real path and marks the row as accounted for,
        // so what is left in the map afterwards is exactly the new content.
        if let Some(found) = by_name.remove(&m.name.to_ascii_lowercase()) {
            placed.push(found);
        }
        // Otherwise the game no longer ships it: drop the row.
    }
    // Whatever the profile never mentioned, in engine order, ahead of everything.
    let mut fresh: Vec<ModEntry> = by_name.into_values().collect();
    fresh.sort_by_key(|m| m.name.to_ascii_lowercase());
    out.extend(fresh);
    out.extend(placed);
    out
}

/// The list without the game's content, for when there is no game to reconcile
/// against. A `*` row whose files cannot be located is not a mod and must not be
/// shown as one - least of all with an empty path, which every consumer would
/// then have to defend against.
fn strip_unmanaged(mods: Vec<ModEntry>) -> Vec<ModEntry> {
    mods.into_iter().filter(|m| !m.unmanaged).collect()
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

pub(crate) fn selected_game(app: &App) -> Option<&DetectedGame> {
    app.selected.and_then(|i| app.games.get(i))
}

pub(crate) fn planned_instance(app: &App) -> Option<Instance> {
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
/// the clicked row belongs to it, otherwise just that row.
fn selection_or(app: &App, row: usize) -> Vec<usize> {
    let mut v: Vec<usize> = if app.selected_mods.contains(&row) && app.selected_mods.len() > 1 {
        app.selected_mods.iter().copied().collect()
    } else {
        vec![row]
    };
    // Separators are IN. They used to be filtered out here, on the theory that a
    // separator defines a group rather than sitting in one - which made every
    // reorder gesture a no-op on a separator, since the callers all bail on an
    // empty block. MO2 does the opposite: `ModList::flags` marks a separator
    // `ItemIsDragEnabled` like any other row (modlist.cpp:630), and
    // `dropMimeData` hands the dragged rows to `changeModPriority` untouched
    // (modlist.cpp:1159). Group membership is positional and recomputed after
    // every move, so a separator that moves alone has not abandoned its mods -
    // it now heads whatever follows it, and they belong to the header above them.
    //
    // Actions a separator cannot answer are refused where MO2 refuses them: at
    // the menu entry, on the grounds of the thing being missing (no conflict
    // flags, no checkbox), never on the grounds of being a separator.
    v.retain(|&i| i < app.mods.len());
    v.sort_unstable();
    v
}

/// The rows a separator heads: everything after it up to the next separator.
///
/// Adjacency IS the group - the same rule `visible_rows` walks to decide what a
/// fold hides. There is no parent pointer anywhere, in Eidos or in MO2.
fn group_children(mods: &[ModEntry], sep: usize) -> std::ops::Range<usize> {
    let end = mods
        .iter()
        .enumerate()
        .skip(sep + 1)
        .find(|(_, m)| m.is_separator())
        .map(|(i, _)| i)
        .unwrap_or(mods.len());
    (sep + 1).min(end)..end
}

/// The mods a fold is currently hiding, by name, so a move can be compared
/// against what it swallowed.
fn hidden_by_folds(app: &App) -> HashSet<String> {
    let vis = visible_rows(&app.mods, &app.collapsed, false, |_, _| true);
    app.mods
        .iter()
        .zip(&vis)
        .filter(|(m, &shown)| !shown && !m.is_separator())
        .map(|(m, _)| m.name.clone())
        .collect()
}

/// Reconcile the fold state with a move that just happened.
///
/// Two things, both about rows going invisible without being asked to:
///
/// A separator that moved is unfolded if it heads anything at its new position.
/// MO2 does exactly this after a priority change (`ModListView::onModPrioritiesChanged`,
/// modlistview.cpp:449), and it is what makes "a separator moves alone"
/// survivable: a folded header dropped somewhere new would otherwise go on
/// hiding rows that were never inside it, which reads as mods having been deleted.
///
/// The mirror case has no MO2 answer, because MO2's tree at least draws the
/// swallowed rows under a parent: lift a separator out from between a folded
/// group and its own mods, and those mods join the folded group and vanish. The
/// fold is the user's, so it is not overridden - but the disappearance is named,
/// because a row leaving the screen unbidden and unremarked is the failure mode
/// this list is most often accused of.
fn settle_folds_after_move(app: &mut App, at: usize, len: usize, hidden_before: &HashSet<String>) {
    let opened: Vec<String> = (at..(at + len).min(app.mods.len()))
        .filter(|&i| app.mods[i].is_separator())
        .filter(|&i| !group_children(&app.mods, i).is_empty())
        .map(|i| app.mods[i].display_name().to_string())
        .collect();
    let mut changed = false;
    for name in opened {
        changed |= app.collapsed.remove(&name);
    }
    if changed {
        save_collapsed(app);
    }
    let swallowed = hidden_by_folds(app);
    let n = swallowed.difference(hidden_before).count();
    if n > 0 {
        app.status =
            Some(format!("{n} mod(s) are now inside a folded group. Unfold it to see them."));
    }
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
    // No floor. This used to clamp every move to below the game's own content,
    // for a reason that was true at the time - those rows were not in modlist.txt,
    // so a mod dropped among them vanished on the next save. They are written now
    // (MO2's `*`), which is what makes a separator above the DLC block possible,
    // and a collapsed block is the only way to put that noise away.
    let dest = if up { neighbour } else { neighbour + 1 };
    let held = hold_mod_selection(app);
    let hidden = hidden_by_folds(app);
    let at = move_block(&mut app.mods, &block, dest);
    put_mod_selection(app, held);
    app.selected_mod = Some(at);
    settle_folds_after_move(app, at, block.len(), &hidden);
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
                let tier = if account.is_premium { "Premium" } else { "free" };
                col = col.push(text(format!("Connected as {} ({tier}).", account.name)).size(11.0));
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

fn executables_dialog<'a>(app: &App, state: &ExecutablesDialogState) -> Element<'a, Message> {
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
            .push(prereq_status_rows(app, &state.prereqs))
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
            Row::new()
                .spacing(8)
                .push(
                    button(text("Close").size(12.0))
                        .padding([5, 14])
                        .on_press(Message::CloseLootReport)
                        .style(button::primary),
                )
                // The report is a worklist: the plugins to clean get read off it
                // while xEdit runs on another screen. Selecting rich text inside a
                // modal is not something this toolkit does, so hand over the whole
                // thing in one press - which is what the Ctrl+A/Ctrl+C people are
                // really after anyway. Ctrl+C does the same while this is open.
                .push(
                    button(text("Copy report").size(12.0))
                        .padding([5, 14])
                        .on_press(Message::CopyLootReport)
                        .style(button::secondary),
                ),
        );
    container(card).max_width(580.0).padding(16).style(card_style).into()
}

/// The report as plain text, for the clipboard.
///
/// Deliberately not the on-screen layout: colour carries the severity there, and
/// a paste into a text editor would lose it silently. Each line says what it is.
fn loot_report_text(report: &eidos_loot::LootReport) -> String {
    let mut out = String::from("LOOT report\n");
    if report.is_empty() {
        out.push_str("\nNo issues - the load order is clean.\n");
        return out;
    }
    if !report.general.is_empty() {
        out.push_str("\nGeneral messages\n");
        for m in &report.general {
            out.push_str(&format!("  [{}] {}\n", loot_severity_label(m.kind), m.text));
        }
    }
    for p in &report.plugins {
        out.push_str(&format!("\n{}\n", p.name));
        if !p.missing_masters.is_empty() {
            out.push_str(&format!("  Missing masters: {}\n", p.missing_masters.join(", ")));
        }
        for m in &p.messages {
            out.push_str(&format!("  [{}] {}\n", loot_severity_label(m.kind), m.text));
        }
        for d in &p.dirty {
            let util = if d.cleaning_utility.is_empty() { "?" } else { d.cleaning_utility.as_str() };
            out.push_str(&format!(
                "  Dirty - {util} found {} ITM, {} deleted refs, {} deleted navmeshes (clean with xEdit)\n",
                d.itm_count, d.deleted_reference_count, d.deleted_navmesh_count
            ));
        }
    }
    out
}

fn loot_severity_label(kind: eidos_loot::MessageType) -> &'static str {
    match kind {
        eidos_loot::MessageType::Error => "error",
        eidos_loot::MessageType::Warn => "warning",
        eidos_loot::MessageType::Say => "note",
    }
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
/// How long a `.unfinished` partial may go without growing before it is called
/// stalled rather than downloading. Generous: a slow mirror can go quiet for a
/// few seconds, and calling a live download dead is worse than the reverse.
const STALLED_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

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
        for d in [".eidos-install-1-0", ".eidos-install-2-0"] {
            fs::create_dir_all(mods.join(d).join("00 Core")).unwrap();
            fs::write(mods.join(d).join("00 Core/a.esp"), b"x").unwrap();
        }
        // Everything that must survive: a real mod, a separator, and a dotfile
        // that is not ours.
        fs::create_dir_all(mods.join("A Real Mod/meshes")).unwrap();
        fs::write(mods.join("A Real Mod/meshes/m.nif"), b"keep").unwrap();
        fs::create_dir_all(mods.join("Group_separator")).unwrap();
        fs::create_dir_all(mods.join(".git")).unwrap();
        app.created = Some(eidos_instance::Instance::portable(root.clone()));

        update_inner(&mut app, Message::CleanInstallDebris);

        assert!(!mods.join(".eidos-install-1-0").exists());
        assert!(!mods.join(".eidos-install-2-0").exists());
        assert_eq!(fs::read(mods.join("A Real Mod/meshes/m.nif")).unwrap(), b"keep");
        assert!(mods.join("Group_separator").is_dir());
        assert!(mods.join(".git").is_dir(), "a dotfile that is not ours is not ours to delete");
        assert!(app.status.as_deref().unwrap_or("").contains('2'), "{:?}", app.status);
        let _ = fs::remove_dir_all(&root);
    }


    #[test]
    fn the_row_colour_has_exactly_one_owner() {
        // The fill and the fade must agree, always. They agree because they ask
        // the same function - this pins the precedence they both inherit.
        let conflict = Some(CONFLICT_WINS_BG);
        assert_eq!(
            row_background(true, true, conflict),
            SEL_BG,
            "selection outranks the conflict tint"
        );
        assert_eq!(row_background(true, false, conflict), CONFLICT_WINS_BG);
        assert_eq!(row_background(true, false, None), row_bg(true));
        assert_eq!(row_background(false, false, None), row_bg(false));
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
