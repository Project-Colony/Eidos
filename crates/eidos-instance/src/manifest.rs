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
}

impl Manifest {
    pub fn new(game_id: &str, kind: InstanceKind) -> Manifest {
        Manifest {
            schema_version: SCHEMA_VERSION,
            game_id: game_id.to_string(),
            kind,
            selected_profile: None,
        }
    }

    /// Parse a manifest file. Returns `None` if absent, unreadable, or missing the
    /// required `game_id`.
    pub fn read(path: &Path) -> Option<Manifest> {
        let text = fs::read_to_string(path).ok()?;
        let mut game_id: Option<String> = None;
        let mut schema_version = SCHEMA_VERSION;
        let mut kind = InstanceKind::Global;
        let mut selected_profile = None;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('[') || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = eidos_ini::key_value(line) else { continue };
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
                _ => {}
            }
        }
        Some(Manifest { schema_version, game_id: game_id?, kind, selected_profile })
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
        // Atomic replace (tmp + rename): a crash mid-write must never leave a torn
        // manifest - a manifest that fails to parse makes the whole instance look
        // uninitialised on the next start.
        crate::write_atomic(path, out.as_bytes())
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
        Manifest::new("skyrimse", InstanceKind::Global).write(&p).unwrap();
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
    fn missing_is_none() {
        assert!(Manifest::read(Path::new("/no/such/eidos-instance.ini")).is_none());
    }
}
