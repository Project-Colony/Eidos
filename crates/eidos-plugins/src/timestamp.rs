//! Timestamp engines keep physical source files untouched; launch projects this order.
use crate::{
    newest_variant, plugins_txt_dir, read_decoded, GameSpec, LoadOrderMechanism, PluginList,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
impl GameSpec {
    pub fn active_file(&self) -> &'static str {
        if self.esplugin_id == esplugin::GameId::Morrowind {
            "Morrowind.ini"
        } else {
            "plugins.txt"
        }
    }
}
/// Location to READ the game's existing activation state. Writers use profile storage.
pub fn plugin_state_dir(prefix: &Path, game_root: &Path, spec: &GameSpec) -> PathBuf {
    if spec.esplugin_id == esplugin::GameId::Morrowind
        || spec.esplugin_id == esplugin::GameId::Oblivion
            && newest_variant(game_root, "Oblivion.ini")
                .and_then(|p| read_decoded(&p))
                .is_some_and(|s| ini_value(&s, "General", "bUseMyGamesDirectory") == Some("0"))
    {
        game_root.to_path_buf()
    } else {
        plugins_txt_dir(prefix, spec)
    }
}
fn ini_value<'a>(text: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut inside = false;
    let mut value = None;
    for line in text.lines().map(str::trim) {
        if let Some(s) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            inside = s.eq_ignore_ascii_case(section);
        } else if inside {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    value = Some(v.trim());
                }
            }
        }
    }
    value
}
pub fn morrowind_active(text: &str) -> Vec<String> {
    let mut inside = false;
    let mut entries = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if let Some(s) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            inside = s.eq_ignore_ascii_case("Game Files");
        } else if inside {
            if let Some((key, value)) = line.split_once('=') {
                if let Some(index) = key
                    .trim()
                    .to_ascii_lowercase()
                    .strip_prefix("gamefile")
                    .and_then(|n| n.parse::<u32>().ok())
                {
                    if !value.trim().is_empty() {
                        entries.insert(index, value.trim().to_string());
                    }
                }
            }
        }
    }
    entries.into_values().collect()
}
pub(crate) fn write_morrowind_active<'a>(
    text: &str,
    names: impl Iterator<Item = &'a str>,
) -> String {
    let entries = names
        .enumerate()
        .map(|(i, n)| format!("GameFile{i}={n}\r\n"))
        .collect::<String>();
    let mut output = String::new();
    let mut inside = false;
    let mut inserted = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(section) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            inside = section.eq_ignore_ascii_case("Game Files");
            output.push_str(line);
            output.push_str("\r\n");
            if inside && !inserted {
                output.push_str(&entries);
                inserted = true;
            }
            continue;
        }
        if inside
            && trimmed.split_once('=').is_some_and(|(k, _)| {
                k.trim()
                    .to_ascii_lowercase()
                    .strip_prefix("gamefile")
                    .is_some_and(|n| n.parse::<u32>().is_ok())
            })
        {
            continue;
        }
        output.push_str(line);
        output.push_str("\r\n");
    }
    if !inserted {
        output.push_str("[Game Files]\r\n");
        output.push_str(&entries);
    }
    output
}
impl PluginList {
    /// Allocate the same unique source mtimes, padding duplicates by 60 seconds.
    pub fn virtual_mtimes(&self, spec: &GameSpec) -> io::Result<BTreeMap<String, SystemTime>> {
        if spec.mechanism != LoadOrderMechanism::Timestamp {
            return Ok(BTreeMap::new());
        }
        let mut times = self
            .plugins
            .iter()
            .map(|p| std::fs::metadata(&p.path)?.modified())
            .collect::<io::Result<BTreeSet<_>>>()?
            .into_iter()
            .collect::<Vec<_>>();
        while times.len() < self.plugins.len() {
            times.push(
                times
                    .last()
                    .copied()
                    .unwrap_or(UNIX_EPOCH)
                    .checked_add(Duration::from_secs(60))
                    .ok_or_else(|| io::Error::other("Plugin timestamp overflow"))?,
            );
        }
        Ok(self
            .plugins
            .iter()
            .zip(times)
            .map(|(p, t)| (p.name.to_ascii_lowercase(), t))
            .collect())
    }
    pub fn apply_mtime_order(&mut self, times: &BTreeMap<String, SystemTime>, spec: &GameSpec) {
        if spec.mechanism != LoadOrderMechanism::Timestamp {
            return;
        }
        self.plugins.sort_by_cached_key(|p| {
            (
                !p.loads_as_master(),
                times
                    .get(&p.name.to_ascii_lowercase())
                    .copied()
                    .or_else(|| std::fs::metadata(&p.path).ok()?.modified().ok())
                    .unwrap_or(UNIX_EPOCH),
                std::cmp::Reverse(p.name.to_uppercase()),
            )
        });
        self.refresh(spec);
    }
}
