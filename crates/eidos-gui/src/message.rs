//! Every message the window can receive.
//!
//! One enum for the whole program, as iced wants it. The groups inside follow
//! MO2's dialogs and panels; `update` is where each one is handled.

use crate::*;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Installer(installers::Action),
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
    InstallerArchiveResolved {
        request: u64,
        target: Option<CollectionTarget>,
        name: String,
        result: Result<Option<PathBuf>, String>,
    },
    ModPickedFor {
        fresh: bool,
        request: u64,
        target: Option<CollectionTarget>,
        name: Option<String>,
        path: Option<PathBuf>,
    },
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
    ModReinstallFresh(usize),
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
    /// The shared tools folder: typed, browsed, and saved.
    ToolsDirChanged(String),
    ToolsDirSave,
    BrowseToolsDir,
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
    /// Drop the sort AND the grouping in one go - the way back to load order,
    /// which is the only order in which the list can be reordered.
    ClearListOrder,
    /// Fold or unfold one synthetic group header.
    ToggleGroupFold(String),
    /// Preview a file from a tree, in a pane over the window.
    PreviewFile(PathBuf),
    RunFileExtension(eidos_addons::protocol::Operation, PathBuf),
    ChooseExtensionFile(eidos_addons::protocol::Operation),
    ExtensionFilePicked {
        request: u64,
        target: Option<CollectionTarget>,
        epoch: u64,
        operation: eidos_addons::protocol::Operation,
        path: Option<PathBuf>,
    },
    PreviewReady(u64, Preview),
    PreviewDdsSelection(dds_preview::Selection),
    PreviewNifView(nif_render::View),
    ArchiveProviderAction {
        epoch: u64,
        member: String,
        provider: eidos_conflicts::AssetProvider,
        export: bool,
    },
    ArchiveFilterChanged(String),
    ArchivePage(usize),
    ArchiveExportDestination(u64, Option<PathBuf>),
    ArchiveExportFinished(u64, Result<PathBuf, String>),
    ClosePreview,
    /// Executables editor: the AppID field, the two flags, and the shortcut.
    ExecAppIdChanged(String),
    ToolArgsAction(iced::widget::text_editor::Action),
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
    /// Carries the mod, so the folder lands in the tree it was started from
    /// even if the panel has moved on.
    FiletreeNewFolderStart(usize),
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
    CollisionBackupChanged(bool),
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
    /// Pick a palette: a `(family, variant)` pair from the shared catalogue, or
    /// Eidos's own parchment.
    ThemeChanged(String, String),
    /// Pick an accent override, or `None` to go back to the theme's own.
    AccentChanged(Option<String>),
    /// Boost the separation between surfaces and text on whatever theme is on.
    ToggleHighContrast(bool),
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
    /// Turn the window's animations on or off (Preferences -> Appearance).
    ToggleMotion(bool),
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
    ModEndorsed(PathBuf, String, u64, Result<bool, String>),
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
    /// The divider between the mod list and the right pane was grabbed.
    SplitGrab,
    /// One animation frame. Carries nothing: every animated value is derived
    /// from elapsed time, so the tick's only job is to make iced redraw.
    ///
    /// Only subscribed while something is actually moving - see `subscription`.
    AnimationTick,
    ArchivePoll,
    /// A frame while an archive extracts: check whether the worker finished.
    InstallPoll,
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
    CollectionFetched(
        PathBuf,
        String,
        Result<eidos_nexus::collections::CollectionRevision, String>,
    ),
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
    /// Where iced laid the File / View / Filters button out. The menu opens ON
    /// this rather than before it, so it is never drawn in the wrong place and
    /// then corrected a frame later.
    FileMenuAt(Option<iced::Rectangle>),
    ViewMenuAt(Option<iced::Rectangle>),
    FiltersAt(Option<iced::Rectangle>),
    /// Open a URL in the user's browser (LOOT advice links in the report).
    OpenUrl(String),
    // ---- move an instance to another machine (eidos pack / eidos unpack) ----
    /// Install the collection the pane is showing, on a worker thread.
    CollectionInstall,
    CollectionInstallChecked {
        request: u64,
        target: CollectionTarget,
        link: String,
        result: Result<eidos_collections::recipe::RuntimeCheck, String>,
    },
    /// Open the Pack dialog, which previews what would go into the file.
    ShowPackDialog,
    ClosePackDialog,
    /// The destination typed into the dialog.
    PackDestChanged(String),
    /// Include `downloads/` (the archives the mods were installed from).
    PackToggleDownloads,
    /// One of 7-Zip's compression levels.
    PackLevelChanged(u8),
    /// Open a native save dialog for the destination.
    PackBrowse,
    /// Where it will be written (`None` = cancelled).
    PackDestPicked(Option<PathBuf>),
    /// Start packing on a worker thread.
    PackRun,
    /// Open the Unpack dialog. Reachable with NO instance open: restoring a
    /// backup onto a fresh machine is the whole point of the file.
    ShowUnpackDialog,
    CloseUnpackDialog,
    /// Choose the `.eidos` file, and what reading its manifest said.
    UnpackBrowseArchive,
    UnpackArchivePicked(Option<PathBuf>),
    /// Choose the folder it goes into.
    UnpackBrowseDest,
    UnpackDestPicked(Option<PathBuf>),
    UnpackDestChanged(String),
    /// Unpack into a folder that is not empty.
    UnpackToggleForce,
    /// Start unpacking on a worker thread.
    UnpackRun,
    /// The 60 Hz look at a running pack or unpack. Reached from `AnimationTick`,
    /// like `InstallPoll`, because iced's timer needs a non-capturing closure.
    TransferPoll,
    /// Dismiss the card showing what a finished pack or unpack did.
    CloseTransferResult,
    // ---- manual plugin reorder (MO2 lets the load order be dragged by hand) ----
    /// Move the plugin at this index one slot earlier / later in the load order.
    Noop,
}
