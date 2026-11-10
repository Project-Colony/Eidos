//! The instance manifest: `<root>/eidos-instance.ini`.
//!
//! A tiny self-describing header so an instance knows its game - essential for
//! portable instances, whose folder name does not encode the game id - and so the
//! on-disk format can evolve (`schema_version`). Deliberately our own minimal
//! `key=value` format (not MO2's), distinct from the per-mod `meta.ini`.

use std::fs;
use std::io;
use std::path::Path;

use crate::InstanceKind;

/// Current on-disk layout version. Bump when the instance layout changes.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub schema_version: u32,
    pub game_id: String,
    pub kind: InstanceKind,
    /// The active profile (reserved for the profiles feature; `None` for now).
    pub selected_profile: Option<String>,
    /// Exact store/install identity selected when this instance was created.
    pub installation: Option<String>,
    /// The folder under the instance root the game runs from (`None` = the
    /// store install).
    pub base_game: Option<String>,
    /// The folder Eidos made by freezing the game, the only copy it may delete.
    pub frozen_copy: Option<String>,
}

impl Manifest {
    pub fn new(game_id: &str, kind: InstanceKind) -> Manifest {
        Manifest {
            schema_version: SCHEMA_VERSION,
            game_id: game_id.to_string(),
            kind,
            selected_profile: None,
            installation: None,
            base_game: None,
            frozen_copy: None,
        }
    }

    /// Parse a manifest file. Returns `None` if absent, unreadable, or missing the
    /// required `game_id`.
    pub fn read(path: &Path) -> Option<Manifest> {
        Self::read_checked(path).ok().flatten()
    }

    /// Read without treating a damaged or unreadable manifest as a new instance.
    pub fn read_checked(path: &Path) -> io::Result<Option<Manifest>> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut game_id: Option<String> = None;
        let mut schema_version = SCHEMA_VERSION;
        let mut kind = InstanceKind::Global;
        let mut selected_profile = None;
        let mut installation = None;
        let mut base_game = None;
        let mut frozen_copy = None;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('[') || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = eidos_ini::key_value(line) else {
                continue;
            };
            let v = v.trim();
            match k {
                "schema_version" => schema_version = v.parse().unwrap_or(SCHEMA_VERSION),
                "game_id" => game_id = Some(v.to_string()),
                "kind" => {
                    kind = if v.eq_ignore_ascii_case("portable") {
                        InstanceKind::Portable
                    } else {
                        InstanceKind::Global
                    }
                }
                "selected_profile" if !v.is_empty() => selected_profile = Some(v.to_string()),
                "installation" if !v.is_empty() => installation = Some(v.to_string()),
                "base_game" if !v.is_empty() => base_game = Some(copy_folder(v)?),
                "frozen_copy" if !v.is_empty() => frozen_copy = Some(copy_folder(v)?),
                _ => {}
            }
        }
        Ok(Some(Manifest {
            schema_version,
            game_id: game_id.filter(|id| !id.is_empty()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "instance manifest is missing game_id",
                )
            })?,
            kind,
            selected_profile,
            installation,
            base_game,
            frozen_copy,
        }))
    }

    pub fn write(&self, path: &Path) -> io::Result<()> {
        let kind = match self.kind {
            InstanceKind::Global => "global",
            InstanceKind::Portable => "portable",
        };
        let mut out = format!(
            "[eidos]\nschema_version={}\ngame_id={}\nkind={}\n",
            self.schema_version, self.game_id, kind
        );
        if let Some(p) = &self.selected_profile {
            out.push_str("selected_profile=");
            out.push_str(p);
            out.push('\n');
        }
        if let Some(selection) = &self.installation {
            if selection.contains(['\n', '\r', '\0']) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid installation identity",
                ));
            }
            out.push_str("installation=");
            out.push_str(selection);
            out.push('\n');
        }
        for (key, folder) in [
            ("base_game", &self.base_game),
            ("frozen_copy", &self.frozen_copy),
        ] {
            let Some(folder) = folder else { continue };
            if !crate::is_copy_folder_name(folder) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid game copy folder",
                ));
            }
            out.push_str(&format!("{key}={folder}\n"));
        }
        // Atomic replace (tmp + rename): a crash mid-write must never leave a torn
        // manifest - a manifest that fails to parse makes the whole instance look
        // uninitialised on the next start.
        crate::write_atomic(path, out.as_bytes())
    }
}

/// A copy folder value read back from the manifest, refused unless it is one
/// plain name: a hand-edited `../elsewhere` must not move the game files.
fn copy_folder(v: &str) -> io::Result<String> {
    if crate::is_copy_folder_name(v) {
        Ok(v.to_string())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "instance manifest names an invalid game copy folder",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("eidos-manifest-{}-{}.ini", std::process::id(), n))
    }

    #[test]
    fn round_trips_global() {
        let p = tmp();
        Manifest::new("skyrimse", InstanceKind::Global)
            .write(&p)
            .unwrap();
        let m = Manifest::read(&p).unwrap();
        assert_eq!(m.schema_version, SCHEMA_VERSION);
        assert_eq!(m.game_id, "skyrimse");
        assert_eq!(m.kind, InstanceKind::Global);
        assert_eq!(m.selected_profile, None);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn round_trips_portable_with_profile() {
        let p = tmp();
        let mut man = Manifest::new("fallout4", InstanceKind::Portable);
        man.selected_profile = Some("Ultra".to_string());
        man.write(&p).unwrap();
        let m = Manifest::read(&p).unwrap();
        assert_eq!(m.game_id, "fallout4");
        assert_eq!(m.kind, InstanceKind::Portable);
        assert_eq!(m.selected_profile.as_deref(), Some("Ultra"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn selected_installation_survives_a_manifest_update() {
        let p = tmp();
        fs::write(
            &p,
            "[eidos]\ngame_id=skyrimse\ninstallation=[\"GOG:1711230643\",\"/games/Skyrim\"]\n",
        )
        .unwrap();
        let mut m = Manifest::read_checked(&p).unwrap().unwrap();
        m.selected_profile = Some("Second".into());
        m.write(&p).unwrap();
        assert!(fs::read_to_string(&p)
            .unwrap()
            .contains("installation=[\"GOG:1711230643\",\"/games/Skyrim\"]"));
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn game_copy_keys_round_trip_and_survive_a_profile_change() {
        let p = tmp();
        let mut m = Manifest::new("skyrimse", InstanceKind::Portable);
        m.base_game = Some("Stock Game".into());
        m.frozen_copy = Some("Stock Game".into());
        m.write(&p).unwrap();
        let mut back = Manifest::read_checked(&p).unwrap().unwrap();
        assert_eq!(
            (back.base_game.as_deref(), back.frozen_copy.as_deref()),
            (Some("Stock Game"), Some("Stock Game"))
        );
        back.selected_profile = Some("Other".into());
        back.write(&p).unwrap();
        assert_eq!(
            Manifest::read(&p).unwrap().base_game.as_deref(),
            Some("Stock Game")
        );
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn a_copy_folder_must_be_one_plain_name() {
        for bad in ["", ".", "..", "a/b", "../Stock Game", "/abs", "x\ny", " "] {
            let mut m = Manifest::new("skyrimse", InstanceKind::Portable);
            m.base_game = Some(bad.into());
            assert_eq!(
                m.write(&tmp()).unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
                "{bad:?}"
            );
        }
        let p = tmp();
        fs::write(&p, "[eidos]\ngame_id=skyrimse\nbase_game=../outside\n").unwrap();
        assert_eq!(
            Manifest::read_checked(&p).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn missing_is_none() {
        assert!(Manifest::read(Path::new("/no/such/eidos-instance.ini")).is_none());
    }
}
