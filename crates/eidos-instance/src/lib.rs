//! Eidos instance model, shared by the CLI and the GUI so both create and read
//! instances identically.
//!
//! An instance is one modding setup for a game. Like Mod Organizer 2 it can be:
//! - **Global**: stored centrally at `$XDG_DATA_HOME/eidos/<game-id>/`, managed
//!   by Eidos.
//! - **Portable**: a self-contained folder the user chooses (movable, isolated).
//!
//! Either way the layout is the same:
//! ```text
//! <root>/mods/<name>/...   one folder per mod
//! <root>/modlist.txt       order + enabled state (MO2 style; top = highest)
//! <root>/overwrite/        the writable layer (saves, regenerated configs)
//! <root>/.base             bind-stash mountpoint for the pristine game files
//! ```

use std::fs;
use std::path::PathBuf;

mod manifest;
mod meta;
mod profile;
mod tools;
pub use manifest::Manifest;
pub use meta::ModMeta;
pub use profile::Profile;
pub use tools::{default_tools, merge_tools, read_tools, write_tools, Tool};

/// Where an instance is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceKind {
    /// Centrally under `$XDG_DATA_HOME/eidos/<id>`.
    Global,
    /// In a self-contained folder chosen by the user.
    Portable,
}

/// One mod in the list: a folder under `mods/`, with its enabled state. Order in
/// the returned vec is priority order, highest first (wins file conflicts).
#[derive(Debug, Clone)]
pub struct ModEntry {
    pub name: String,
    pub enabled: bool,
    pub path: PathBuf,
}

/// `$XDG_DATA_HOME`, or `$HOME/.local/share`.
pub fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
            home.join(".local/share")
        })
}

/// A modding instance rooted at a directory.
#[derive(Debug, Clone)]
pub struct Instance {
    pub root: PathBuf,
}

impl Instance {
    /// A global instance for a game id: `$XDG_DATA_HOME/eidos/<id>`.
    pub fn global(game_id: &str) -> Self {
        Instance { root: data_home().join("eidos").join(game_id) }
    }

    /// A portable instance at an explicit folder.
    pub fn portable(root: PathBuf) -> Self {
        Instance { root }
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.root.join("mods")
    }

    /// MO2-compatible metadata for a mod (`mods/<name>/meta.ini`); empty if none.
    pub fn mod_meta(&self, name: &str) -> ModMeta {
        ModMeta::read(&self.mods_dir().join(name).join("meta.ini"))
    }

    /// The instance manifest path (`<root>/eidos-instance.ini`).
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("eidos-instance.ini")
    }

    /// Read the instance manifest, if present.
    pub fn read_manifest(&self) -> Option<Manifest> {
        Manifest::read(&self.manifest_path())
    }

    /// Write the instance manifest if it is missing (so we don't churn one that
    /// already exists, e.g. on every launch).
    pub fn ensure_manifest(&self, game_id: &str, kind: InstanceKind) -> std::io::Result<()> {
        if self.manifest_path().exists() {
            return Ok(());
        }
        Manifest::new(game_id, kind).write(&self.manifest_path())
    }

    /// The instance's game id: from the manifest, else the last path component
    /// (correct for a global instance, whose folder is named after the game).
    pub fn game_id(&self) -> Option<String> {
        self.read_manifest()
            .map(|m| m.game_id)
            .or_else(|| self.root.file_name().map(|s| s.to_string_lossy().into_owned()))
    }

    pub fn overwrite_dir(&self) -> PathBuf {
        self.root.join("overwrite")
    }

    /// Bind-stash mountpoint for the pristine game files (used at launch).
    pub fn base_dir(&self) -> PathBuf {
        self.root.join(".base")
    }

    /// Downloaded mod archives land here (`<root>/downloads/`), each with its
    /// MO2-format `.meta` sidecar; shared by all profiles like `mods/`.
    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    /// The instance's tool list (`<root>/tools.ini`), user entries only - merge
    /// with per-game defaults via [`merge_tools`].
    pub fn tools(&self) -> Vec<Tool> {
        read_tools(&self.root.join("tools.ini"))
    }

    /// Persist the user's tool list.
    pub fn save_tools(&self, tools: &[Tool]) -> std::io::Result<()> {
        write_tools(&self.root.join("tools.ini"), tools)
    }

    pub fn exists(&self) -> bool {
        self.mods_dir().is_dir()
    }

    /// Create the `mods/` and `overwrite/` directories.
    pub fn create(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.mods_dir())?;
        fs::create_dir_all(self.overwrite_dir())?;
        self.ensure_profiles()?;
        Ok(())
    }

    /// The active profile's mod list (folders in the shared `mods/`, in priority
    /// order with enabled state). Top of the list = highest priority.
    pub fn modlist(&self) -> Vec<ModEntry> {
        self.active().modlist()
    }

    /// Persist the active profile's mod list.
    pub fn save_modlist(&self, mods: &[ModEntry]) -> std::io::Result<()> {
        self.active().save_modlist(mods)
    }

    /// Enabled mods of the active profile, highest priority first.
    pub fn load_order(&self) -> Vec<PathBuf> {
        self.active().load_order()
    }

    // ---- profiles ----

    /// `<root>/profiles/`.
    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    /// All profile names (at least `Default`).
    pub fn profiles(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(self.profiles_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        v.sort();
        if v.is_empty() {
            v.push("Default".to_string());
        }
        v
    }

    /// The profile of the given name (not necessarily existing on disk yet).
    pub fn profile(&self, name: &str) -> Profile {
        Profile { instance_root: self.root.clone(), name: name.to_string() }
    }

    /// The active profile name (from the manifest; `Default` if unset).
    pub fn active_profile(&self) -> String {
        self.read_manifest()
            .and_then(|m| m.selected_profile)
            .unwrap_or_else(|| "Default".to_string())
    }

    /// Set the active profile, persisted in the manifest (if one exists).
    pub fn set_active_profile(&self, name: &str) -> std::io::Result<()> {
        if let Some(mut m) = self.read_manifest() {
            m.selected_profile = Some(name.to_string());
            m.write(&self.manifest_path())?;
        }
        Ok(())
    }

    /// The active [`Profile`].
    pub fn active(&self) -> Profile {
        self.profile(&self.active_profile())
    }

    /// Ensure a `Default` profile exists, migrating a legacy flat `modlist.txt`
    /// (a pre-profiles instance) into it. Idempotent.
    pub fn ensure_profiles(&self) -> std::io::Result<()> {
        let default_dir = self.profiles_dir().join("Default");
        fs::create_dir_all(&default_dir)?;
        let legacy = self.root.join("modlist.txt");
        let migrated = default_dir.join("modlist.txt");
        if legacy.exists() && !migrated.exists() {
            fs::rename(&legacy, &migrated)?;
        }
        Ok(())
    }
}
