//! Read-only Heroic/Legendary installed manifests. No account or library-cache reads.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{catalog, DetectedGame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    Gog,
    Epic,
}

impl Store {
    pub fn name(self) -> &'static str {
        match self {
            Self::Gog => "GOG",
            Self::Epic => "Epic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GameSource {
    #[default]
    Steam,
    /// Discovery records the actual prefix, but does not guess a host-compatible runner.
    External {
        store: Store,
        app_id: String,
        prefix: Option<PathBuf>,
        heroic: bool,
    },
}

impl DetectedGame {
    /// Stable source and install identity for an instance; duplicate copies stay distinct.
    pub fn selection_id(&self) -> String {
        let store = match &self.source {
            GameSource::Steam => format!("steam:{}", self.def.steam_app_id),
            GameSource::External { store, app_id, .. } => format!("{}:{app_id}", store.name()),
        };
        serde_json::json!([store, self.install_path.to_string_lossy()]).to_string()
    }

    pub fn source_name(&self) -> &'static str {
        match &self.source {
            GameSource::Steam => "Steam",
            GameSource::External {
                store: Store::Gog, ..
            } => "GOG / Heroic",
            GameSource::External { heroic: true, .. } => "Epic / Heroic",
            GameSource::External { .. } => "Epic / Legendary",
        }
    }

    pub fn is_steam(&self) -> bool {
        matches!(self.source, GameSource::Steam)
    }

    /// Store variants can use different profile locations despite sharing an engine.
    pub fn plugin_spec(&self) -> Option<eidos_plugins::GameSpec> {
        let mut spec = eidos_plugins::GameSpec::for_id(self.def.id)?;
        match (&self.source, self.def.id) {
            (
                GameSource::External {
                    store: Store::Gog, ..
                },
                "skyrimse",
            ) => spec.local_dir = "Skyrim Special Edition GOG".into(),
            (
                GameSource::External {
                    store: Store::Epic, ..
                },
                "skyrimse",
            ) => spec.local_dir = "Skyrim Special Edition EPIC".into(),
            (
                GameSource::External {
                    store: Store::Epic, ..
                },
                "falloutnv",
            ) => spec.local_dir = "FalloutNV_Epic".into(),
            _ => {}
        }
        Some(spec)
    }

    /// The actual Wine prefix root, never a fabricated Steam compatdata directory.
    pub fn prefix(&self) -> Option<PathBuf> {
        match &self.source {
            GameSource::Steam => self.compatdata.as_ref().map(|p| p.join("pfx")),
            GameSource::External { prefix, .. } => prefix.clone(),
        }
    }

    pub fn plugin_state_dir(&self) -> Option<PathBuf> {
        let spec = self.plugin_spec()?;
        if self.def.id == "morrowind" {
            return Some(self.install_path.clone());
        }
        Some(eidos_plugins::plugin_state_dir(
            &self.prefix()?,
            &self.install_path,
            &spec,
        ))
    }
}

/// Select a saved installation exactly. Legacy instances only select unambiguous copies.
pub fn select_installation<'a>(
    games: &'a [DetectedGame],
    game_id: &str,
    saved: Option<&str>,
) -> Option<&'a DetectedGame> {
    let mut matches = games
        .iter()
        .filter(|g| g.def.id == game_id && saved.is_none_or(|key| g.selection_id() == key));
    let selected = matches.next()?;
    matches.next().is_none().then_some(selected)
}

fn json(path: &Path) -> Option<Value> {
    const LIMIT: u64 = 16 * 1024 * 1024;
    let file = File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= LIMIT)
        .then(|| serde_json::from_slice(&bytes).ok())
        .flatten()
}

fn prefix(root: &Path, id: &str, native: bool) -> Option<PathBuf> {
    if native
        || id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return None;
    }
    let settings = json(&root.join("GamesConfig").join(format!("{id}.json")))?;
    let settings = settings.get(id).unwrap_or(&settings);
    let configured = Path::new(settings.get("winePrefix")?.as_str()?);
    if !configured.is_absolute() {
        return None;
    }
    let path = match settings.get("wineVersion")?.get("type")?.as_str()? {
        "proton" => configured.join("pfx"),
        "wine" => configured.to_path_buf(),
        _ => return None,
    };
    path.is_dir().then(|| path.canonicalize().ok()).flatten()
}

fn append_manifest(out: &mut Vec<DetectedGame>, file: &Path, store: Store, heroic: Option<&Path>) {
    let Some(value) = json(file) else { return };
    let entries: Vec<(String, &Value)> = match store {
        Store::Gog => value
            .get("installed")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| Some((v.get("appName")?.as_str()?.to_owned(), v)))
            .collect(),
        Store::Epic => value
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, v)| (key.clone(), v))
            .collect(),
    };
    for (id, entry) in entries {
        if entry
            .get("is_dlc")
            .or_else(|| entry.get("isDlc"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            continue;
        }
        if store == Store::Epic
            && entry
                .get("app_name")
                .and_then(Value::as_str)
                .is_some_and(|name| name != id)
        {
            continue;
        }
        let Some(def) = catalog().iter().find(|d| match store {
            Store::Gog => d.gog_ids.contains(&id.as_str()),
            Store::Epic => d.epic_ids.contains(&id.as_str()),
        }) else {
            continue;
        };
        let Some(raw) = entry.get("install_path").and_then(Value::as_str) else {
            continue;
        };
        let path = Path::new(raw);
        if !path.is_absolute() || !path.is_dir() {
            continue;
        }
        let Ok(install_path) = path.canonicalize() else {
            continue;
        };
        let game = DetectedGame {
            def,
            data_path: install_path.join(def.data_dir),
            install_path,
            compatdata: None,
            steam_name: entry
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(def.name)
                .to_owned(),
            source: GameSource::External {
                store,
                prefix: heroic.and_then(|root| {
                    prefix(
                        root,
                        &id,
                        entry.get("platform").and_then(Value::as_str) == Some("linux"),
                    )
                }),
                app_id: id,
                heroic: heroic.is_some(),
            },
        };
        if !out.iter().any(|g| g.selection_id() == game.selection_id()) {
            out.push(game);
        }
    }
}

pub(super) fn detect_stores(home: &Path) -> Vec<DetectedGame> {
    let mut out = Vec::new();
    let roots = [
        home.join(".config/heroic"),
        home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ];
    for root in &roots {
        append_manifest(
            &mut out,
            &root.join("gog_store/installed.json"),
            Store::Gog,
            Some(root),
        );
        append_manifest(
            &mut out,
            &root.join("legendaryConfig/legendary/installed.json"),
            Store::Epic,
            Some(root),
        );
        if let Some(parent) = root.parent() {
            append_manifest(
                &mut out,
                &parent.join("legendary/installed.json"),
                Store::Epic,
                root.is_dir().then_some(root),
            );
        }
    }
    out
}
