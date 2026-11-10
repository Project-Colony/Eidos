//! Where an instance reads the game's own files: the store install, or a copy
//! of the game folder kept in the instance (a Wabbajack list's `Stock Game`, or
//! one Eidos made to freeze the game version). The manifest names the copy.

use std::path::{Path, PathBuf};

/// Whether `name` can name a game copy folder: one plain path component, so a
/// manifest can never point the game files outside the instance root. The
/// manifest trims its values, so a name with edge whitespace would not read back.
pub fn is_copy_folder_name(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name != "."
        && name != ".."
        && !name.contains(['/', '\0', '\n', '\r'])
}

/// The game folder and data directory that readers of the game's own files use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameFiles {
    pub root: PathBuf,
    pub data: PathBuf,
    /// The copy folder under the instance root, `None` for the store install.
    pub copy: Option<String>,
}

impl GameFiles {
    /// A data directory outside the install (7 Days to Die's user `Mods`) stays
    /// where it is with a copy.
    pub fn new(install: &Path, data: &Path, instance_root: &Path, copy: Option<&str>) -> GameFiles {
        let Some(copy) = copy else {
            return GameFiles {
                root: install.to_path_buf(),
                data: data.to_path_buf(),
                copy: None,
            };
        };
        let root = instance_root.join(copy);
        let data = match data.strip_prefix(install) {
            // `join("")` would leave a trailing slash on a data-is-root game.
            Ok(rel) if rel.as_os_str().is_empty() => root.clone(),
            Ok(rel) => root.join(rel),
            Err(_) => data.to_path_buf(),
        };
        GameFiles {
            root,
            data,
            copy: Some(copy.to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_copy_the_game_files_are_the_install() {
        let f = GameFiles::new(Path::new("/g"), Path::new("/g/Data"), Path::new("/i"), None);
        assert_eq!(
            f,
            GameFiles {
                root: "/g".into(),
                data: "/g/Data".into(),
                copy: None
            }
        );
    }

    #[test]
    fn a_copy_moves_root_and_data_but_not_a_data_dir_outside_the_install() {
        let f = GameFiles::new(
            Path::new("/g"),
            Path::new("/g/SB/Content/Paks/~mods"),
            Path::new("/i"),
            Some("Stock Game"),
        );
        assert_eq!(f.root, Path::new("/i/Stock Game"));
        assert_eq!(f.data, Path::new("/i/Stock Game/SB/Content/Paks/~mods"));
        let outside = GameFiles::new(
            Path::new("/g"),
            Path::new("/u/Mods"),
            Path::new("/i"),
            Some("Stock Game"),
        );
        assert_eq!(outside.data, Path::new("/u/Mods"));
        // A game whose data directory is the install itself.
        let same = GameFiles::new(
            Path::new("/g"),
            Path::new("/g"),
            Path::new("/i"),
            Some("Stock Game"),
        );
        assert_eq!(same.data.as_os_str(), "/i/Stock Game");
    }
}
