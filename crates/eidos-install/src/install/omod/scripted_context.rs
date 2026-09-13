use super::*;
use eidos_core::LayerStack;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct Source {
    pub path: PathBuf,
    stamp: [u64; 7],
}
impl Source {
    pub fn capture(path: PathBuf) -> Result<Self, InstallError> {
        // Reject links in every component, including directory links selected by
        // the virtual tree. No script may read through an unowned redirection.
        let path = checked_destination(Path::new("/"), &path)?;
        let m = fs::symlink_metadata(&path)?;
        if !m.is_file() {
            return Err(bad("context source is not a regular file"));
        }
        Ok(Self {
            path,
            stamp: [
                m.dev(),
                m.ino(),
                m.len(),
                m.mtime() as u64,
                m.mtime_nsec() as u64,
                m.ctime() as u64,
                m.ctime_nsec() as u64,
            ],
        })
    }
    pub fn verify(&self) -> Result<(), InstallError> {
        if Self::capture(self.path.clone())? != *self {
            return Err(bad("virtual source changed; restart the OMOD review"));
        }
        Ok(())
    }
    pub fn open(&self) -> Result<fs::File, InstallError> {
        self.verify()?;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&self.path)?;
        let m = file.metadata()?;
        let stamp = [
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime() as u64,
            m.mtime_nsec() as u64,
            m.ctime() as u64,
            m.ctime_nsec() as u64,
        ];
        if !m.is_file() || stamp != self.stamp {
            return Err(bad("source changed while opening"));
        }
        Ok(file)
    }
    pub fn read(&self, max: u64, cancel: &AtomicBool) -> Result<Vec<u8>, InstallError> {
        self.verify()?;
        if self.stamp[2] > max {
            return Err(bad("source exceeds the requested byte bound"));
        }
        let mut file = self.open()?;
        let mut bytes = Vec::new();
        let mut buf = [0; 65536];
        loop {
            check_cancel(cancel)?;
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if bytes.len() as u64 + n as u64 > max {
                return Err(bad("source grew beyond its byte bound"));
            }
            bytes.extend_from_slice(&buf[..n]);
        }
        self.verify()?;
        Ok(bytes)
    }
}

fn real_directory(path: &Path) -> Result<(), InstallError> {
    if !path.is_absolute() || path.parent().is_none() || !path.is_dir() {
        return Err(bad("selected installation and instance paths must be existing absolute non-root directories"));
    }
    checked_destination(Path::new("/"), path)?;
    Ok(())
}
fn ini_map(text: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut map = BTreeMap::<String, BTreeMap<String, String>>::new();
    let mut section = String::new();
    for line in text.lines() {
        if let Some(s) = eidos_ini::section_header(line) {
            section = s.to_ascii_lowercase();
        } else if !line.trim_start().starts_with([';', '#']) {
            if let Some((k, v)) = eidos_ini::key_value(line) {
                map.entry(section.clone())
                    .or_default()
                    .entry(k.to_ascii_lowercase())
                    .or_insert_with(|| v.trim().to_string());
            }
        }
    }
    map
}
fn tree(stack: &LayerStack, cancel: &AtomicBool) -> Result<BTreeMap<String, Source>, InstallError> {
    let mut out = BTreeMap::new();
    let mut dirs = vec![(String::new(), 0)];
    let mut entries = 0usize;
    while let Some((parent, depth)) = dirs.pop() {
        check_cancel(cancel)?;
        if depth > 128 {
            return Err(bad("virtual context exceeds 128 directory levels"));
        }
        for (name, path, kind) in stack.list_dir_typed(&parent) {
            entries += 1;
            if entries > 500_000 {
                return Err(bad("virtual context exceeds 500000 entries"));
            }
            let name = if parent.is_empty() {
                name
            } else {
                format!("{parent}/{name}")
            };
            let kind = kind.ok_or_else(|| bad("virtual context changed during enumeration"))?;
            if kind.is_dir() {
                dirs.push((name, depth + 1));
            } else if kind.is_file() {
                out.insert(name.to_ascii_lowercase(), Source::capture(path)?);
            } else {
                return Err(bad(format!(
                    "unsupported linked or special virtual source: {name}"
                )));
            }
        }
    }
    Ok(out)
}
fn optional_source(
    path: Option<PathBuf>,
    cancel: &AtomicBool,
) -> Result<Option<(Source, Vec<u8>)>, InstallError> {
    path.map(|p| {
        let s = Source::capture(p)?;
        let b = s.read(MAX_EFFECT_FILE, cancel)?;
        Ok((s, b))
    })
    .transpose()
}

impl ScriptedContext {
    pub fn capture(
        instance: &eidos_instance::Instance,
        game: &ScriptedGame,
        cancel: &AtomicBool,
    ) -> Result<Self, InstallError> {
        if game.game_id != "oblivion" {
            return Err(bad("Scripted OMOD installers require Oblivion"));
        }
        check_cancel(cancel)?;
        real_directory(&instance.root)?;
        real_directory(&game.install_path)?;
        real_directory(&game.data_path)?;
        if !game.data_path.starts_with(&game.install_path) {
            return Err(bad(
                "selected Data directory is outside the game installation",
            ));
        }
        if let Some(p) = &game.prefix {
            real_directory(p)?;
        }
        let _lock = instance.try_lock("capturing scripted OMOD context")?;
        let profile = instance.active();
        real_directory(&profile.dir())?;
        let spec = eidos_plugins::GameSpec::for_id("oblivion").unwrap();
        let routed = eidos_plugins::plugin_state_dir(
            game.prefix.as_deref().unwrap_or(Path::new("")),
            &game.install_path,
            &spec,
        );
        let root_mode = routed == game.install_path;
        let fallback_plugins = (root_mode || game.prefix.is_some()).then_some(routed);
        let fallback_inis = if root_mode {
            Some(game.install_path.clone())
        } else {
            game.prefix
                .as_ref()
                .map(|p| eidos_plugins::documents_my_games_dir(p, &spec))
        };
        let mods = profile.modlist();
        let mut layers: Vec<_> = mods
            .iter()
            .rev()
            .filter(|m| m.is_active())
            .map(|m| m.path.clone())
            .collect();
        layers.push(game.data_path.clone());
        let stack = LayerStack::new(layers, instance.overwrite_dir());
        let sources = tree(&stack, cancel)?;
        let mut interpreter = ObmmContext {
            files: sources.keys().cloned().collect(),
            active_mods: mods
                .iter()
                .filter(|m| m.is_active())
                .map(|m| m.name.clone())
                .collect(),
            versions: game.observed_versions.clone(),
            ..Default::default()
        };
        // Native script compatibility target, never a claim that Windows OBMM is
        // installed. Fixed UI scripts require 1.1.12's vocabulary and semantics.
        interpreter.versions.insert("OBMM".into(), "1.1.12".into());
        let plugin_list = instance
            .plugin_list_for_profile(
                &game.data_path,
                "oblivion",
                fallback_plugins.as_deref(),
                &profile,
            )
            .ok_or_else(|| bad("missing Oblivion plugin specification"))?;
        interpreter.plugins = plugin_list
            .plugins
            .iter()
            .map(|p| (p.name.clone(), p.enabled))
            .collect();
        let mut control_sources = BTreeMap::new();
        let ini_path =
            eidos_plugins::newest_variant(&profile.dir(), "Oblivion.ini").or_else(|| {
                fallback_inis
                    .as_ref()
                    .and_then(|p| eidos_plugins::newest_variant(p, "Oblivion.ini"))
            });
        let ini = optional_source(ini_path, cancel)?;
        let (ini_text, ini_cp1252) = if let Some((_, bytes)) = &ini {
            match std::str::from_utf8(bytes) {
                Ok(text) => (text.to_string(), false),
                Err(_) => (encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(), true),
            }
        } else {
            (String::new(), false)
        };
        interpreter.ini = ini_map(&ini_text);
        if let Some((s, b)) = ini {
            control_sources.insert("ini".to_string(), (s, digest(&b)));
        }
        if let Some(dir) = &fallback_inis {
            if let Some((s, b)) = optional_source(
                eidos_plugins::newest_variant(dir, "RendererInfo.txt"),
                cancel,
            )? {
                let text = String::from_utf8_lossy(&b);
                interpreter.renderer = text
                    .lines()
                    .filter_map(|l| l.split_once(':').or_else(|| l.split_once('=')))
                    .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                    .collect();
                control_sources.insert("renderer".into(), (s, digest(&b)));
            }
        }
        let mut roots = instance.root_layers();
        roots.push(game.install_path.clone());
        let relative_data = game.data_path.strip_prefix(&game.install_path).ok();
        let root = LayerStack::new_with_readonly_overwrite(
            roots,
            if root_mode {
                profile.dir().join("runtime-root")
            } else {
                instance.root_overwrite_dir()
            },
            relative_data,
            root_mode.then(|| instance.root_overwrite_dir()),
        );
        for name in [
            "obse_loader.exe",
            "obse_1_2_416.dll",
            "obse_editor_1_2.dll",
            "oblivion.exe",
        ] {
            if let Some(p) = root.resolve_read(name) {
                control_sources.insert(name.into(), (Source::capture(p)?, String::new()));
            }
        }
        interpreter.script_extender_present =
            control_sources.keys().any(|k| k.starts_with("obse_"));
        interpreter.graphics_extender_present = sources
            .keys()
            .any(|p| p.starts_with("obse/plugins/obge") && p.ends_with(".dll"));
        for path in [
            instance.manifest_path(),
            profile.dir().join("settings.ini"),
            profile.dir().join("lockedorder.txt"),
        ] {
            if path.exists() {
                let s = Source::capture(path.clone())?;
                let b = s.read(MAX_EFFECT_FILE, cancel)?;
                control_sources.insert(path.to_string_lossy().into_owned(), (s, digest(&b)));
            }
        }
        for dir in [Some(profile.plugins_state_dir()), fallback_plugins.clone()]
            .into_iter()
            .flatten()
        {
            for name in ["plugins.txt", "loadorder.txt"] {
                if let Some((s, b)) =
                    optional_source(eidos_plugins::newest_variant(&dir, name), cancel)?
                {
                    control_sources.insert(s.path.to_string_lossy().into_owned(), (s, digest(&b)));
                }
            }
        }
        // Collection pause may serialize an already visible inactive reservation.
        // Bind effective rows, not whitespace or an equivalent modlist rewrite.
        let mod_rows: Vec<_> = mods
            .iter()
            .map(|m| (&m.name, &m.path, m.is_active(), m.is_separator()))
            .collect();
        let snapshot = serde_json::to_vec(&(
            game,
            &mod_rows,
            &profile.name,
            &interpreter,
            &sources,
            &control_sources,
            root_mode,
        ))
        .map_err(json_error)?;
        for source in sources
            .values()
            .chain(control_sources.values().map(|(s, _)| s))
        {
            source.verify()?;
        }
        Ok(Self {
            origin: ScriptedOrigin {
                instance_root: instance.root.clone(),
                profile: profile.name,
                game: game.clone(),
                context_sha256: digest(&snapshot),
            },
            interpreter,
            sources,
            ini_text,
            ini_cp1252,
            plugin_list,
        })
    }
}
