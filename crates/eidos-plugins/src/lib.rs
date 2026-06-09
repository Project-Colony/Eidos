//! Plugin (ESP/ESM/ESL/ESH) load order for Eidos.
//!
//! This is the *second* load-order axis, independent of the mod-folder order the
//! FUSE union resolves: it decides which `.esp/.esm` records win and which FormID
//! mod-index each plugin gets. A plugin loaded before its master crashes the game,
//! so the ordering here is not cosmetic.
//!
//! It mirrors Mod Organizer 2's `PluginList` semantics - the three ordering
//! invariants (`fixPrimaryPlugins` / `fixPluginRelationships` / `fixPriorities`)
//! and the mod-index computation (`generatePluginIndexes`) - but as a pure,
//! game-agnostic model, with header parsing delegated to the `esplugin` crate
//! (the same one libloot uses). The on-disk reader/writer lives separately.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use esplugin::{GameId, ParseOptions, Plugin as EspPlugin};

mod loadorder;
pub use loadorder::{documents_my_games_dir, plugins_txt_dir};

/// How a game persists its plugin load order on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOrderMechanism {
    /// One `plugins.txt`: the active set, each active line prefixed with `*`,
    /// masters first. Skyrim SE/VR, Fallout 4, Starfield, Enderal SE.
    Asterisk,
    /// `plugins.txt` (active set) + `loadorder.txt` (the full order). Skyrim LE,
    /// Fallout 3, Fallout New Vegas.
    PlainList,
}

/// Per-game plugin handling: identity for esplugin, the on-disk format, and the
/// canonical primary (game master) plugins pinned to the top of the order.
#[derive(Debug, Clone)]
pub struct GameSpec {
    pub esplugin_id: GameId,
    pub mechanism: LoadOrderMechanism,
    pub primary_plugins: Vec<String>,
    /// The game's folder name under the prefix's `AppData/Local` and
    /// `Documents/My Games` (e.g. `Skyrim Special Edition`).
    pub local_dir: String,
}

impl GameSpec {
    /// Plugin spec for an Eidos game id, or `None` if the game has no (supported)
    /// plugin system. Oblivion/Morrowind (timestamp-ordered) are not covered yet.
    pub fn for_id(eidos_game_id: &str) -> Option<GameSpec> {
        use LoadOrderMechanism::*;
        const SKYRIM_MASTERS: &[&str] =
            &["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"];
        let (id, mech, primaries, local): (GameId, LoadOrderMechanism, &[&str], &str) =
            match eidos_game_id {
                "skyrimse" => (GameId::SkyrimSE, Asterisk, SKYRIM_MASTERS, "Skyrim Special Edition"),
                "skyrimvr" => (GameId::SkyrimSE, Asterisk, SKYRIM_MASTERS, "Skyrim VR"),
                "enderalse" => (GameId::SkyrimSE, Asterisk, SKYRIM_MASTERS, "Enderal Special Edition"),
                "skyrim" => (GameId::Skyrim, PlainList, &["Skyrim.esm", "Update.esm"], "Skyrim"),
                "fallout4" => (GameId::Fallout4, Asterisk, &["Fallout4.esm"], "Fallout4"),
                "falloutnv" => (GameId::FalloutNV, PlainList, &["FalloutNV.esm"], "FalloutNV"),
                "fallout3" => (GameId::Fallout3, PlainList, &["Fallout3.esm"], "Fallout3"),
                "starfield" => (
                    GameId::Starfield,
                    Asterisk,
                    &["Starfield.esm", "Constellation.esm", "OldMars.esm"],
                    "Starfield",
                ),
                _ => return None,
            };
        Some(GameSpec {
            esplugin_id: id,
            mechanism: mech,
            primary_plugins: primaries.iter().map(|s| s.to_string()).collect(),
            local_dir: local.to_string(),
        })
    }

    fn light_supported(&self) -> bool {
        self.esplugin_id.supports_light_plugins()
    }

    fn medium_supported(&self) -> bool {
        self.esplugin_id.supports_medium_plugins()
    }
}

/// One plugin in the load order.
#[derive(Debug, Clone)]
pub struct Plugin {
    /// File name, e.g. `Skyrim.esm`.
    pub name: String,
    /// The mod folder providing it (empty for the game's own Data).
    pub origin_mod: String,
    /// The real file on disk.
    pub path: PathBuf,
    pub enabled: bool,
    /// Header master flag (0x01).
    pub is_master: bool,
    /// Light plugin (`.esl` or the 0x200 flag).
    pub is_light: bool,
    /// Medium plugin (Starfield ESH).
    pub is_medium: bool,
    /// MAST subrecords: the plugins this one depends on.
    pub masters: Vec<String>,
    /// Contiguous user-order priority (0 = first), assigned by `sort`.
    pub priority: i32,
    /// Game-visible mod index (`00`, `FE:001`, `FD:00`); `None` when disabled.
    pub index: Option<String>,
}

impl Plugin {
    /// Whether the game loads this as a master (so it sorts above normal plugins):
    /// the header master flag, a light plugin, or an `.esm`/`.esl` extension.
    pub fn loads_as_master(&self) -> bool {
        if self.is_master || self.is_light {
            return true;
        }
        let lower = self.name.to_ascii_lowercase();
        lower.ends_with(".esm") || lower.ends_with(".esl")
    }
}

/// The ordered plugin list.
#[derive(Debug, Clone, Default)]
pub struct PluginList {
    pub plugins: Vec<Plugin>,
}

impl PluginList {
    /// Discover plugins from a set of sources - `(origin_mod, dir)` pairs in
    /// ascending plugin-priority order (earliest = lowest index; pass the game's
    /// own Data dir first with an empty origin). A later source providing a plugin
    /// of the same name wins it (higher-priority mod), like the file union. Each
    /// plugin's header is parsed with esplugin.
    pub fn discover(sources: &[(String, PathBuf)], spec: &GameSpec) -> PluginList {
        let mut plugins: Vec<Plugin> = Vec::new();
        let mut idx: HashMap<String, usize> = HashMap::new();

        for (origin, dir) in sources {
            let Ok(rd) = std::fs::read_dir(dir) else { continue };
            let mut found: Vec<(String, PathBuf)> = rd
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let lower = name.to_ascii_lowercase();
                    let is_plugin = lower.ends_with(".esp")
                        || lower.ends_with(".esm")
                        || lower.ends_with(".esl");
                    is_plugin.then(|| (name, e.path()))
                })
                .collect();
            found.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));

            for (name, path) in found {
                let key = name.to_ascii_lowercase();
                let (is_master, is_light, is_medium, masters) =
                    parse_header(&path, spec.esplugin_id).unwrap_or_else(|| {
                        // Unparseable: fall back to the extension.
                        let m = key.ends_with(".esm") || key.ends_with(".esl");
                        (m, key.ends_with(".esl"), false, Vec::new())
                    });
                if let Some(&i) = idx.get(&key) {
                    let p = &mut plugins[i];
                    p.path = path;
                    p.origin_mod = origin.clone();
                    p.is_master = is_master;
                    p.is_light = is_light;
                    p.is_medium = is_medium;
                    p.masters = masters;
                } else {
                    idx.insert(key, plugins.len());
                    plugins.push(Plugin {
                        name,
                        origin_mod: origin.clone(),
                        path,
                        enabled: true,
                        is_master,
                        is_light,
                        is_medium,
                        masters,
                        priority: -1,
                        index: None,
                    });
                }
            }
        }
        PluginList { plugins }
    }

    /// Re-sort to satisfy the ordering invariants, then assign mod indexes. Call
    /// after any change (enable/disable, reorder, discover).
    pub fn refresh(&mut self, spec: &GameSpec) {
        self.sort(spec);
        self.generate_indexes(spec);
    }

    /// Order the plugins to satisfy MO2's three invariants and assign contiguous
    /// `priority` values. Semantically equivalent to MO2's
    /// `fixPrimaryPlugins` + `fixPluginRelationships` + `fixPriorities`, expressed
    /// as a deterministic stable sort plus a stable topological pass:
    /// 1. primary (game master) plugins first, in their canonical order;
    /// 2. all masters above all normal plugins;
    /// 3. within that, the input (mod-priority) order is preserved;
    /// 4. every plugin after all of its own masters.
    pub fn sort(&mut self, spec: &GameSpec) {
        let n = self.plugins.len();

        // Tier + primary sub-order key; `i` (input position) is the stable tiebreak.
        let primary_pos = |name: &str| {
            spec.primary_plugins.iter().position(|p| p.eq_ignore_ascii_case(name))
        };
        let mut base: Vec<usize> = (0..n).collect();
        base.sort_by_key(|&i| {
            let p = &self.plugins[i];
            let prim = primary_pos(&p.name);
            let tier: u8 = if prim.is_some() {
                0
            } else if p.loads_as_master() {
                1
            } else {
                2
            };
            (tier, prim.unwrap_or(usize::MAX), i)
        });

        let order = topo_stable(&self.plugins, &base);
        let mut sorted: Vec<Plugin> = order.iter().map(|&i| self.plugins[i].clone()).collect();
        for (pos, p) in sorted.iter_mut().enumerate() {
            p.priority = pos as i32;
        }
        self.plugins = sorted;
    }

    /// Assign each enabled plugin its game-visible mod index, iterating in
    /// priority order. Ported from MO2's `generatePluginIndexes`:
    /// normal = sequential 2-hex; light = `FE:xxx` (254 + n/4096, slot n%4096);
    /// medium = `FD:xx` (253 + n/256, slot n%256). Disabled plugins get no index.
    pub fn generate_indexes(&mut self, spec: &GameSpec) {
        let (light_ok, medium_ok) = (spec.light_supported(), spec.medium_supported());
        let (mut esl, mut esh, mut normal): (u32, u32, u32) = (0, 0, 0);
        for p in &mut self.plugins {
            if !p.enabled {
                p.index = None;
                continue;
            }
            if medium_ok && p.is_medium {
                p.index = Some(format!("{:02X}:{:02X}", 253 + esh / 256, esh % 256));
                esh += 1;
            } else if light_ok && p.is_light {
                p.index = Some(format!("{:02X}:{:03X}", 254 + esl / 4096, esl % 4096));
                esl += 1;
            } else {
                p.index = Some(format!("{:02X}", normal));
                normal += 1;
            }
        }
    }

    /// Enabled plugins whose masters are missing from the enabled set - a reliable
    /// crash predictor (MO2's `testMasters`). Returns `(plugin, missing_master)`.
    pub fn missing_masters(&self) -> Vec<(String, String)> {
        let active: std::collections::HashSet<String> = self
            .plugins
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.to_ascii_lowercase())
            .collect();
        let mut out = Vec::new();
        for p in self.plugins.iter().filter(|p| p.enabled) {
            for m in &p.masters {
                if !active.contains(&m.to_ascii_lowercase()) {
                    out.push((p.name.clone(), m.clone()));
                }
            }
        }
        out
    }

    /// Set a plugin's enabled state by name (case-insensitive). Returns whether it
    /// matched. Call `refresh` afterwards to recompute indexes.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(p) = self.plugins.iter_mut().find(|p| p.name.eq_ignore_ascii_case(name)) {
            p.enabled = enabled;
            true
        } else {
            false
        }
    }
}

/// Parse a plugin header with esplugin: `(is_master, is_light, is_medium, masters)`.
fn parse_header(path: &Path, game_id: GameId) -> Option<(bool, bool, bool, Vec<String>)> {
    let mut p = EspPlugin::new(game_id, path);
    p.parse_file(ParseOptions::header_only()).ok()?;
    Some((
        p.is_master_file(),
        p.is_light_plugin(),
        p.is_medium_plugin(),
        p.masters().unwrap_or_default(),
    ))
}

/// Stable topological order: respect `base` (the tier ordering) as the tiebreak,
/// but never place a plugin before one of its present masters. O(n^2), fine for
/// realistic plugin counts. A dependency cycle (should not occur) falls back to
/// `base` order for the offending nodes.
fn topo_stable(plugins: &[Plugin], base: &[usize]) -> Vec<usize> {
    let n = plugins.len();
    let by_name: HashMap<String, usize> = plugins
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.to_ascii_lowercase(), i))
        .collect();

    let mut indeg = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, p) in plugins.iter().enumerate() {
        for m in &p.masters {
            if let Some(&mi) = by_name.get(&m.to_ascii_lowercase()) {
                if mi != i {
                    indeg[i] += 1;
                    dependents[mi].push(i);
                }
            }
        }
    }

    let mut base_pos = vec![0usize; n];
    for (pos, &i) in base.iter().enumerate() {
        base_pos[i] = pos;
    }

    let mut placed = vec![false; n];
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        // The available (in-degree 0) node earliest in `base`.
        let mut best: Option<usize> = None;
        for i in 0..n {
            if !placed[i] && indeg[i] == 0 && best.is_none_or(|b| base_pos[i] < base_pos[b]) {
                best = Some(i);
            }
        }
        let Some(i) = best else { break };
        placed[i] = true;
        result.push(i);
        for &d in &dependents[i] {
            indeg[d] = indeg[d].saturating_sub(1);
        }
    }
    // Any cycle leftovers, in base order.
    for &i in base {
        if !placed[i] {
            result.push(i);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, masters: &[&str]) -> Plugin {
        let lower = name.to_ascii_lowercase();
        Plugin {
            name: name.to_string(),
            origin_mod: String::new(),
            path: PathBuf::from(name),
            enabled: true,
            is_master: lower.ends_with(".esm"),
            is_light: lower.ends_with(".esl"),
            is_medium: false,
            masters: masters.iter().map(|s| s.to_string()).collect(),
            priority: -1,
            index: None,
        }
    }

    fn names(list: &PluginList) -> Vec<String> {
        list.plugins.iter().map(|p| p.name.clone()).collect()
    }

    fn se() -> GameSpec {
        GameSpec::for_id("skyrimse").unwrap()
    }

    #[test]
    fn primaries_pinned_first_in_canonical_order() {
        let mut list = PluginList {
            plugins: vec![p("ZMod.esp", &[]), p("Update.esm", &["Skyrim.esm"]), p("Skyrim.esm", &[])],
        };
        list.sort(&se());
        assert_eq!(names(&list), vec!["Skyrim.esm", "Update.esm", "ZMod.esp"]);
    }

    #[test]
    fn masters_sort_above_normals_keeping_input_order() {
        let mut list = PluginList {
            plugins: vec![p("aaa.esp", &[]), p("zzz.esm", &[]), p("bbb.esp", &[]), p("mmm.esm", &[])],
        };
        list.sort(&se());
        // masters first (input order zzz, mmm), then normals (aaa, bbb).
        assert_eq!(names(&list), vec!["zzz.esm", "mmm.esm", "aaa.esp", "bbb.esp"]);
    }

    #[test]
    fn dependent_sorts_after_its_master() {
        // child.esp (normal) depends on a master that itself depends on another.
        let mut list = PluginList {
            plugins: vec![
                p("child.esp", &["MidMaster.esm"]),
                p("MidMaster.esm", &["BaseMaster.esm"]),
                p("BaseMaster.esm", &[]),
            ],
        };
        list.sort(&se());
        let order = names(&list);
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("BaseMaster.esm") < pos("MidMaster.esm"));
        assert!(pos("MidMaster.esm") < pos("child.esp"));
    }

    #[test]
    fn indexes_normal_light_and_disabled() {
        let mut list = PluginList {
            plugins: vec![
                p("Skyrim.esm", &[]),
                {
                    let mut x = p("Light.esl", &[]);
                    x.is_light = true;
                    x
                },
                p("Normal.esp", &[]),
                {
                    let mut x = p("Off.esp", &[]);
                    x.enabled = false;
                    x
                },
            ],
        };
        list.refresh(&se());
        let by = |n: &str| list.plugins.iter().find(|p| p.name == n).unwrap().index.clone();
        assert_eq!(by("Skyrim.esm"), Some("00".to_string())); // first normal/master
        assert_eq!(by("Normal.esp"), Some("01".to_string())); // second normal
        assert_eq!(by("Light.esl"), Some("FE:000".to_string())); // first light
        assert_eq!(by("Off.esp"), None); // disabled
    }

    #[test]
    fn missing_masters_are_flagged() {
        let mut list = PluginList {
            plugins: vec![p("Skyrim.esm", &[]), p("Patch.esp", &["Skyrim.esm", "Ghost.esm"])],
        };
        list.refresh(&se());
        let missing = list.missing_masters();
        assert_eq!(missing, vec![("Patch.esp".to_string(), "Ghost.esm".to_string())]);
    }

    #[test]
    fn game_spec_mechanisms() {
        assert_eq!(se().mechanism, LoadOrderMechanism::Asterisk);
        assert_eq!(GameSpec::for_id("skyrim").unwrap().mechanism, LoadOrderMechanism::PlainList);
        assert!(GameSpec::for_id("nonsuch").is_none());
        assert!(se().light_supported());
        assert!(!se().medium_supported());
    }
}
