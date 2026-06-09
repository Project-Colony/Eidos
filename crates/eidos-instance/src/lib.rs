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

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

mod meta;
pub use meta::ModMeta;

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

    pub fn overwrite_dir(&self) -> PathBuf {
        self.root.join("overwrite")
    }

    /// Bind-stash mountpoint for the pristine game files (used at launch).
    pub fn base_dir(&self) -> PathBuf {
        self.root.join(".base")
    }

    pub fn exists(&self) -> bool {
        self.mods_dir().is_dir()
    }

    /// Create the `mods/` and `overwrite/` directories.
    pub fn create(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.mods_dir())?;
        fs::create_dir_all(self.overwrite_dir())?;
        Ok(())
    }

    /// The mod list: every folder in `mods/`, in priority order with enabled
    /// state, reconciled with `modlist.txt`. Folders not yet in the file are
    /// appended (enabled); file entries whose folder vanished are dropped.
    /// Top of the list = highest priority.
    pub fn modlist(&self) -> Vec<ModEntry> {
        let mut present: Vec<String> = fs::read_dir(self.mods_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        present.sort();

        let mut out: Vec<ModEntry> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        if let Ok(content) = fs::read_to_string(self.root.join("modlist.txt")) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let (enabled, name) = if let Some(n) = line.strip_prefix('+') {
                    (true, n)
                } else if let Some(n) = line.strip_prefix('-') {
                    (false, n)
                } else {
                    (true, line)
                };
                if present.iter().any(|p| p == name) && seen.insert(name.to_string()) {
                    out.push(ModEntry {
                        name: name.to_string(),
                        enabled,
                        path: self.mods_dir().join(name),
                    });
                }
            }
        }
        for name in present {
            if seen.insert(name.clone()) {
                out.push(ModEntry {
                    path: self.mods_dir().join(&name),
                    name,
                    enabled: true,
                });
            }
        }
        out
    }

    /// Persist the mod list to `modlist.txt` (`+Name` enabled, `-Name` disabled).
    pub fn save_modlist(&self, mods: &[ModEntry]) -> std::io::Result<()> {
        let mut s = String::new();
        for m in mods {
            s.push(if m.enabled { '+' } else { '-' });
            s.push_str(&m.name);
            s.push('\n');
        }
        fs::write(self.root.join("modlist.txt"), s)
    }

    /// Enabled mods, highest priority first: the layers to mount at launch.
    pub fn load_order(&self) -> Vec<PathBuf> {
        self.modlist().into_iter().filter(|m| m.enabled).map(|m| m.path).collect()
    }
}
