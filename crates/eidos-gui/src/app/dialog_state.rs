//! What each open dialog holds while it is up: the executables editor, the
//! export, log, INI, backups, categories, pack and unpack dialogs, and the
//! install picker and collision prompt.

use crate::*;

/// An open Executables editor (MO2's Modify Executables dialog). The list shown is
/// the user's `tools.ini` entries (editable, movable, deletable) followed by the
/// per-game defaults (read-only); the first `user_len` rows are the user's.
pub(crate) struct ExecutablesDialogState {
    /// Display order: user tools first, then read-only per-game defaults.
    pub(crate) merged: Vec<Tool>,
    /// How many leading `merged` entries are the user's (editable) tools.
    pub(crate) user_len: usize,
    /// The selected row, if any.
    pub(crate) selected: Option<usize>,
    // Edit buffers mirroring the selected tool (committed back into `merged`).
    pub(crate) title: String,
    pub(crate) exe: String,
    pub(crate) workdir: String,
    /// Arguments, one per line. A `text_editor` rather than a String, because
    /// the field has to hold a newline for the label to be true.
    pub(crate) args_editor: iced::widget::text_editor::Content,
    /// Prerequisite verbs, comma-separated in the editor.
    pub(crate) prereqs: String,
    /// The mod this tool's output is captured into (empty = the Overwrite).
    pub(crate) output_mod: String,
    /// The mods that can be picked as a target, read when the dialog opens.
    pub(crate) mod_names: Vec<String>,
    /// A Steam AppID to launch this tool under, as typed (empty = the game's).
    pub(crate) app_id: String,
}

impl ExecutablesDialogState {
    /// Load the buffers from the selected tool (or clear them when nothing is set).
    pub(crate) fn load_buffers(&mut self) {
        match self.selected.and_then(|i| self.merged.get(i)) {
            Some(t) => {
                self.title = t.title.clone();
                self.exe = t.exe.display().to_string();
                self.workdir = t
                    .workdir
                    .as_ref()
                    .map(|w| w.display().to_string())
                    .unwrap_or_default();
                self.args_editor =
                    iced::widget::text_editor::Content::with_text(&t.args.join("\n"));
                self.prereqs = t.prereqs.join(", ");
                self.output_mod = t.output_mod.clone().unwrap_or_default();
                self.app_id = t.app_id.map(|n| n.to_string()).unwrap_or_default();
            }
            None => {
                self.title.clear();
                self.exe.clear();
                self.workdir.clear();
                self.args_editor = iced::widget::text_editor::Content::new();
                self.prereqs.clear();
                self.output_mod.clear();
                self.app_id.clear();
            }
        }
    }

    /// Whether the selected row is an editable user tool (vs a read-only default).
    pub(crate) fn selected_is_user(&self) -> bool {
        matches!(self.selected, Some(i) if i < self.user_len)
    }

    /// Write the current edit buffers back into the selected user tool.
    pub(crate) fn commit_buffers(&mut self) {
        if !self.selected_is_user() {
            return;
        }
        let Some(i) = self.selected else { return };
        let Some(t) = self.merged.get_mut(i) else {
            return;
        };
        t.title = self.title.trim().to_string();
        t.exe = PathBuf::from(self.exe.trim());
        t.workdir = {
            let w = self.workdir.trim();
            if w.is_empty() {
                None
            } else {
                Some(PathBuf::from(w))
            }
        };
        t.args = self
            .args_editor
            .text()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        t.prereqs = self
            .prereqs
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
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

/// The Export dialog (MO2's Export to csv).
pub(crate) struct ExportDialogState {
    pub(crate) scope: ExportScope,
    /// One flag per `Column::ALL`, in that order. A Vec rather than a set so the
    /// dialog can render in MO2's column order without sorting anything.
    pub(crate) columns: Vec<bool>,
}

impl ExportDialogState {
    /// The columns actually ticked, in MO2's order.
    pub(crate) fn picked(&self) -> Vec<eidos_instance::Column> {
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
pub(crate) struct LogPaneState {
    /// Every session file, newest first.
    pub(crate) files: Vec<PathBuf>,
    /// The one being read.
    pub(crate) current: PathBuf,
    /// Records at or above `level`, oldest first.
    pub(crate) lines: Vec<(eidos_log::Level, String)>,
    /// The floor for what is shown.
    pub(crate) level: eidos_log::Level,
    /// How many records the file held before filtering, so the pane can say what
    /// a level switch is hiding.
    pub(crate) total: usize,
    /// True when the file was longer than the read budget and only its tail is
    /// shown - a launch log runs to megabytes.
    pub(crate) truncated: bool,
}

/// The INI editor (MO2 ships one as a tool plugin).
///
/// It edits the PROFILE's copy, which is the only durable one: the copy in the
/// Proton prefix is overwritten from the profile at every launch and captured
/// back after, so editing that one is either pointless or a race. That makes the
/// editor worth more here than in MO2 - on Linux the prefix copy is buried in
/// `steamapps/compatdata/<id>/pfx/drive_c/users/steamuser/Documents/My Games/...`,
/// which is not a path anyone finds by accident.
pub(crate) struct IniEditorState {
    /// The INI files this game has, in `GameDef` order.
    pub(crate) files: Vec<String>,
    /// Which one is being edited.
    pub(crate) current: String,
    /// The editable buffer.
    pub(crate) content: iced::widget::text_editor::Content,
    /// Whether the file on disk was Windows-1252, so it is written back the same
    /// way. Game INIs are as often CP1252 as UTF-8, and re-encoding one silently
    /// mangles every accented value in it.
    pub(crate) cp1252: bool,
    /// Whether the buffer differs from what was read.
    pub(crate) dirty: bool,
    /// The text as read, so Revert costs no disk round-trip and works even if
    /// the file has since been deleted.
    pub(crate) original: String,
    /// Absent from disk: a profile that has never had this INI. Saving creates it.
    pub(crate) missing: bool,
    /// Present but unreadable. Distinct from `missing`, because saving an empty
    /// buffer over a file that exists destroys it - and a permission error is
    /// exactly when that would happen.
    pub(crate) unreadable: bool,
    /// A hash of the file's bytes as read, so a save can tell that another
    /// process rewrote it since (see `IniEditorSave`).
    pub(crate) on_disk: u64,
}

/// A mod-install name collision: `mods/<name>/` already exists, so the user picks
/// Merge / Replace / Rename / Cancel - MO2's QueryOverwriteDialog.
pub(crate) struct CollisionPrompt {
    pub(crate) target: CollectionTarget,
    pub(crate) backup: bool,
    pub(crate) archive: PathBuf,
    /// The colliding (already sanitized) mod name.
    pub(crate) name: String,
    pub(crate) game_id: String,
    /// Editable target for the Rename option (defaults to a free suggestion).
    pub(crate) rename_to: String,
    /// The prompt guards an in-progress FOMOD wizard (still open in `app.fomod`
    /// with the user's choices): resolve via `finish_fomod`, not a re-extract.
    pub(crate) fomod: bool,
    /// The archive already extracted, kept alive so resolving the collision costs
    /// no second 7-Zip pass. `None` only for a prompt raised without one.
    pub(crate) tree: Option<eidos_install::ExtractedTree>,
    /// The BAIN / manual picks that produced this install, if any. A picker
    /// install cannot be replayed from the tree alone - re-running it without the
    /// selection would install the wrong sub-packages.
    pub(crate) pick: Option<PickerChoice>,
}

/// MO2's manual / BAIN install dialogs, which is where an archive lands when the
/// simple and FOMOD heuristics both decline it. Holds the extracted tree so
/// nothing is unpacked twice, whatever the user picks.
pub(crate) struct InstallPicker {
    pub(crate) target: CollectionTarget,
    pub(crate) archive: PathBuf,
    /// The mod name to install under, editable (MO2 lets you rename here).
    pub(crate) name: String,
    pub(crate) game_id: String,
    pub(crate) tree: eidos_install::ExtractedTree,
    /// The parsed archive tree, computed ONCE at construction. The Manual mode's
    /// validity label used to rebuild it from disk inside view() - a full
    /// recursive walk of the extraction, stat per entry, per redraw - while this
    /// struct's own `rows` comment explains why that must not happen. The tree
    /// cannot change while the dialog is open, so both `rows` and the validity
    /// check read this one.
    pub(crate) archive_tree: eidos_install::ArchiveTree,
    /// The archive's contents as flat depth-first rows, computed once - the tree
    /// does not change while the dialog is open.
    pub(crate) rows: Vec<eidos_install::TreeRow>,
    pub(crate) mode: PickerMode,
}

/// Which of the two dialogs is showing.
pub(crate) enum PickerMode {
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
pub(crate) enum PickerChoice {
    /// The ticked sub-package names, in merge order.
    Bain(Vec<String>),
    /// The chosen data root, relative to the archive.
    Manual(String),
}

/// What the Backups dialog shows: the restore points of each list, newest
/// first, as they were when the dialog opened.
pub(crate) struct BackupsDialogState {
    pub(crate) mods: Vec<eidos_instance::Backup>,
    pub(crate) order: Vec<eidos_instance::Backup>,
}

/// The Categories dialog. Two modes in one card: assigning categories to the
/// selected mods, and editing the catalog those categories come from.
///
/// The pending choice lives here rather than being written on every click,
/// because a `meta.ini` write per checkbox would rewrite the file (and invalidate
/// the meta cache) a dozen times while the user makes up their mind.
pub(crate) struct CategoriesDialogState {
    /// The mods this applies to, BY NAME. More than one = MO2's batch assign.
    ///
    /// Names, not row indices: the mod list can be rebuilt behind an open dialog
    /// (a refresh, a Nexus check, a drag), and indices captured before that would
    /// then point at different mods than the ones the user opened it on.
    pub(crate) names: Vec<String>,
    /// Checked ids, primary FIRST (that is what the on-disk order means).
    pub(crate) chosen: Vec<i32>,
    /// The catalog being edited. Loaded once with the dialog; saved on Apply.
    pub(crate) catalog: eidos_instance::CategoryFactory,
    /// True while the catalog editor is showing instead of the picker.
    pub(crate) editing: bool,
    /// The name box in the catalog editor.
    pub(crate) new_name: String,
    /// The parent for a category about to be created (0 = top level).
    pub(crate) new_parent: i32,
    /// The catalog row being renamed, and its pending name.
    pub(crate) rename: Option<(i32, String)>,
    /// Two-click guard for deleting a catalog row.
    pub(crate) confirm_delete: Option<i32>,
    /// Free-text filter over the category tree.
    pub(crate) query: String,
}

/// The Pack dialog: where the file goes, what goes in it, and the preview of
/// what that means - which is `eidos pack --dry-run`, drawn.
pub(crate) struct PackDialogState {
    /// The destination, as typed or as the picker returned it.
    pub(crate) dest: String,
    /// Include `downloads/`.
    pub(crate) downloads: bool,
    /// 7-Zip's `-mx` level.
    pub(crate) level: u8,
    /// What the walk found, so the dialog can say how many files and how big
    /// BEFORE the user commits twenty minutes to it. Recomputed when an option
    /// changes, because `downloads` moves the numbers.
    pub(crate) plan: eidos_transfer::Plan,
}

/// The Unpack dialog: which backup, where it goes, and what its manifest says.
pub(crate) struct UnpackDialogState {
    /// The `.eidos` file.
    pub(crate) archive: String,
    /// Where the instance will be put back.
    pub(crate) dest: String,
    /// What the archive says about itself, once it has been read. `None` before
    /// a file is chosen; the error is kept separately so a file that is not a
    /// backup can say so in the dialog rather than through a status bar the
    /// welcome screen does not draw.
    pub(crate) manifest: Option<eidos_transfer::BackupManifest>,
    pub(crate) error: Option<String>,
    /// Unpack into a folder that is not empty.
    pub(crate) force: bool,
}
