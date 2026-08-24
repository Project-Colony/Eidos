//! The instance registry: `~/.config/Colony/Eidos/instances.ini`.
//!
//! A global instance needs no registry - its path is a pure function of the
//! game id. A portable instance is the opposite: the user chose the folder,
//! and nothing can re-derive that choice. Without a record of it, the folder
//! is orphaned the moment the process exits - the wizard greets its owner
//! like a stranger and every CLI command quietly operates on the XDG-global
//! path instead. This file is that record: the known portable roots, plus
//! which instance was opened last, so startup and the `nxm://` handler land
//! on the setup the user actually plays.
//!
//! Deliberately its OWN file rather than keys in `settings.ini`:
//! `Settings::save()` rewrites that file from its struct and drops unknown
//! keys, so the first time an older Eidos touched a setting it would have
//! silently destroyed a registry stored there. A file the old versions never
//! open survives them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::settings::config_home;
use crate::Instance;

/// A persistable reference to an instance - the two ways one can be named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceRef {
    /// The central instance for a game id (`$XDG_DATA_HOME/eidos/<id>`).
    Global(String),
    /// A portable instance at an explicit root.
    Portable(PathBuf),
}

impl InstanceRef {
    /// The instance this reference names.
    pub fn instance(&self) -> Instance {
        match self {
            InstanceRef::Global(id) => Instance::global(id),
            InstanceRef::Portable(root) => Instance::portable(root.clone()),
        }
    }

    /// The on-disk token: `global:<id>` or `portable:<path>`. The tag comes
    /// first so a path containing `:` parses unambiguously.
    fn to_token(&self) -> String {
        match self {
            InstanceRef::Global(id) => format!("global:{id}"),
            InstanceRef::Portable(root) => format!("portable:{}", root.display()),
        }
    }

    fn parse(token: &str) -> Option<InstanceRef> {
        let (tag, rest) = token.split_once(':')?;
        let rest = rest.trim();
        if rest.is_empty() {
            return None;
        }
        match tag.trim() {
            "global" => Some(InstanceRef::Global(rest.to_string())),
            "portable" => Some(InstanceRef::Portable(PathBuf::from(rest))),
            _ => None,
        }
    }
}

/// The persisted registry. `portables` is most-recently-used first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    pub portables: Vec<PathBuf>,
    pub last: Option<InstanceRef>,
}

/// `~/.config/Colony/Eidos/instances.ini`.
pub fn registry_path() -> PathBuf {
    config_home().join("instances.ini")
}

impl Registry {
    /// Load the persisted registry, or an empty one if the file is absent or
    /// unreadable - an unreadable registry must degrade to "no portables
    /// known", never to a startup failure.
    pub fn load() -> Registry {
        Registry::load_from(&registry_path())
    }

    /// [`Registry::load`] from an explicit file. The path is a parameter so a
    /// caller can be tested without writing the real user config - and so a
    /// sandbox or a second profile can point somewhere else deliberately.
    pub fn load_from(path: &Path) -> Registry {
        match fs::read_to_string(path) {
            Ok(text) => Registry::parse(&text),
            Err(_) => Registry::default(),
        }
    }

    /// Parse a registry body. Unknown keys are ignored so the format can grow.
    pub fn parse(text: &str) -> Registry {
        let mut reg = Registry::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('[') || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = eidos_ini::key_value(line) else { continue };
            let v = v.trim();
            match k {
                "portable" if !v.is_empty() => {
                    let p = PathBuf::from(v);
                    if !reg.portables.contains(&p) {
                        reg.portables.push(p);
                    }
                }
                "last" => reg.last = InstanceRef::parse(v),
                _ => {}
            }
        }
        reg
    }

    pub fn to_ini(&self) -> String {
        let mut out = String::from("[instances]\n");
        if let Some(last) = &self.last {
            out.push_str(&format!("last={}\n", last.to_token()));
        }
        for p in &self.portables {
            out.push_str(&format!("portable={}\n", p.display()));
        }
        out
    }

    /// Persist the registry. Atomic (tmp + rename) like the manifest: a torn
    /// registry would orphan every portable instance at once.
    pub fn save(&self) -> io::Result<()> {
        self.save_to(&registry_path())
    }

    /// [`Registry::save`] to an explicit file.
    ///
    /// Through the shared atomic writer: the window and any `eidos` process both
    /// save this file, and a FIXED temp name let two of them interleave into one
    /// - which is how a half-written `able=` line got into a real registry.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        crate::write_atomic(path, self.to_ini().as_bytes())
    }

    /// Record a portable root, moving it to the front (most recently used).
    pub fn remember_portable(&mut self, root: &Path) {
        self.portables.retain(|p| p != root);
        self.portables.insert(0, root.to_path_buf());
    }

    /// Drop a portable root the user no longer wants offered.
    pub fn forget_portable(&mut self, root: &Path) {
        self.portables.retain(|p| p != root);
        if self.last == Some(InstanceRef::Portable(root.to_path_buf())) {
            self.last = None;
        }
    }

    /// Record the instance that was just opened. A portable one is also
    /// remembered in the MRU list - opening IS using.
    pub fn set_last(&mut self, r: InstanceRef) {
        if let InstanceRef::Portable(root) = &r {
            let root = root.clone();
            self.remember_portable(&root);
        }
        self.last = Some(r);
    }

    /// Every known instance of `game_id` worth trying, most preferred first:
    /// the last-used instance when it belongs to this game, then the known
    /// portable roots whose manifest names this game (MRU order), then the
    /// global path. Entries whose folder is missing are NOT dropped from the
    /// registry - an unmounted drive is not a deleted instance - the caller
    /// skips non-existing candidates instead.
    pub fn candidates_for(&self, game_id: &str) -> Vec<Instance> {
        let mut out: Vec<Instance> = Vec::new();
        let mut push = |inst: Instance| {
            if !out.iter().any(|i| i.root == inst.root) {
                out.push(inst);
            }
        };
        if let Some(last) = &self.last {
            let inst = last.instance();
            let matches = match last {
                InstanceRef::Global(id) => id == game_id,
                InstanceRef::Portable(_) => {
                    inst.read_manifest().is_some_and(|m| m.game_id == game_id)
                }
            };
            if matches {
                push(inst);
            }
        }
        for root in &self.portables {
            let inst = Instance::portable(root.clone());
            if inst.read_manifest().is_some_and(|m| m.game_id == game_id) {
                push(inst);
            }
        }
        push(Instance::global(game_id));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_portables_and_last() {
        let mut reg = Registry::default();
        reg.remember_portable(Path::new("/mnt/games/EidosSkyrim"));
        reg.remember_portable(Path::new("/home/u/Eidos/fallout4"));
        reg.set_last(InstanceRef::Global("skyrimse".into()));
        let back = Registry::parse(&reg.to_ini());
        assert_eq!(back, reg);

        reg.set_last(InstanceRef::Portable("/mnt/games/EidosSkyrim".into()));
        let back = Registry::parse(&reg.to_ini());
        assert_eq!(back, reg);
    }

    #[test]
    fn remember_is_mru_and_dedups() {
        let mut reg = Registry::default();
        reg.remember_portable(Path::new("/a"));
        reg.remember_portable(Path::new("/b"));
        reg.remember_portable(Path::new("/a"));
        assert_eq!(reg.portables, vec![PathBuf::from("/a"), PathBuf::from("/b")]);
    }

    #[test]
    fn forget_also_clears_a_last_that_pointed_there() {
        let mut reg = Registry::default();
        reg.set_last(InstanceRef::Portable("/a".into()));
        assert_eq!(reg.portables, vec![PathBuf::from("/a")], "set_last must remember too");
        reg.forget_portable(Path::new("/a"));
        assert!(reg.portables.is_empty());
        assert_eq!(reg.last, None, "a forgotten root must not linger as the default");
    }

    #[test]
    fn a_path_containing_colons_survives_the_last_token() {
        let mut reg = Registry::default();
        reg.set_last(InstanceRef::Portable("/mnt/we:ird/Eidos".into()));
        let back = Registry::parse(&reg.to_ini());
        assert_eq!(back.last, Some(InstanceRef::Portable("/mnt/we:ird/Eidos".into())));
    }

    #[test]
    fn candidates_prefer_last_then_mru_then_global() {
        use crate::{InstanceKind, Manifest};
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let mk = |game: &str| {
            let root = std::env::temp_dir().join(format!(
                "eidos-reg-cand-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Manifest::new(game, InstanceKind::Portable)
                .write(&root.join("eidos-instance.ini"))
                .unwrap();
            root
        };
        let a = mk("skyrimse");
        let b = mk("skyrimse");
        let other = mk("fallout4");
        let mut reg = Registry::default();
        reg.remember_portable(&a);
        reg.remember_portable(&other);
        reg.set_last(InstanceRef::Portable(b.clone()));

        let roots: Vec<PathBuf> =
            reg.candidates_for("skyrimse").into_iter().map(|i| i.root).collect();
        assert_eq!(roots[0], b, "the last-used instance leads");
        assert_eq!(roots[1], a, "then the MRU portables of the same game");
        assert_eq!(
            roots.last(),
            Some(&Instance::global("skyrimse").root),
            "the global path closes the list as the fallback"
        );
        assert!(!roots.contains(&other), "another game's portable is never a candidate");
        for r in [a, b, other] {
            let _ = fs::remove_dir_all(&r);
        }
    }

    #[test]
    fn unknown_keys_and_junk_are_ignored() {
        let reg = Registry::parse(
            "[instances]\n# comment\nfuture_key=whatever\nlast=badtag:/x\nportable=\nportable=/ok\n",
        );
        assert_eq!(reg.portables, vec![PathBuf::from("/ok")]);
        assert_eq!(reg.last, None, "an unknown tag is not a reference");
    }
}
