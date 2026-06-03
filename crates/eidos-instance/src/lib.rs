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
//! <root>/mods/<name>/...   one folder per mod (load order)
//! <root>/overwrite/        the writable layer (saves, regenerated configs)
//! <root>/.base             bind-stash mountpoint for the pristine game files
//! ```

use std::fs;
use std::path::PathBuf;

/// Where an instance is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceKind {
    /// Centrally under `$XDG_DATA_HOME/eidos/<id>`.
    Global,
    /// In a self-contained folder chosen by the user.
    Portable,
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

    /// Mod folders, highest priority first. Honours a `load_order.txt` (top line
    /// wins); otherwise alphabetical.
    pub fn load_order(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = fs::read_dir(self.mods_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();

        match fs::read_to_string(self.root.join("load_order.txt")) {
            Ok(content) => {
                let order: Vec<&str> = content.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
                dirs.sort_by_key(|p| {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    order.iter().position(|o| *o == name).unwrap_or(usize::MAX)
                });
            }
            Err(_) => dirs.sort(),
        }
        dirs
    }
}
