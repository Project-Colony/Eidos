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
pub use loadorder::{
    canonical_path, documents_my_games_dir, newest_variant, plugins_txt_dir, read_decoded,
};

/// Whether `name` is a plugin file by extension (`.esp`/`.esm`/`.esl`),
/// case-insensitively - MO2's plugin filter (`*.esp *.esm *.esl`).
pub fn is_plugin(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".esp") || n.ends_with(".esm") || n.ends_with(".esl")
}

/// Whether `name` loads as a master by its extension (`.esm`/`.esl`) - MO2's
/// `hasMasterExtension`; both sort above normal plugins.
pub fn is_master_ext(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".esm") || n.ends_with(".esl")
}

/// Whether `name` is a light (`.esl`) plugin by its extension.
pub fn is_light_ext(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".esl")
}

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
    /// plugin system. The mechanism, master plugins and My Games folder come from
    /// the shared `eidos-gamedef` descriptor; only the esplugin `GameId` (which
    /// belongs to the esplugin crate) is mapped here. Timestamp-ordered games
    /// (Oblivion/Morrowind) return `None` - not managed yet.
    pub fn for_id(eidos_game_id: &str) -> Option<GameSpec> {
        let def = eidos_gamedef::GameDef::for_id(eidos_game_id)?;
        let mechanism = match def.load_order {
            eidos_gamedef::LoadOrder::Asterisk => LoadOrderMechanism::Asterisk,
            eidos_gamedef::LoadOrder::PlainList => LoadOrderMechanism::PlainList,
            // FileTime (Oblivion/Morrowind) and None (generic games) have no Eidos-
            // managed plugins.txt, so there is no plugin spec to build.
            eidos_gamedef::LoadOrder::FileTime | eidos_gamedef::LoadOrder::None => return None,
        };
        let esplugin_id = match eidos_game_id {
            "skyrimse" | "skyrimvr" | "enderalse" => GameId::SkyrimSE,
            "skyrim" => GameId::Skyrim,
            "fallout4" | "fallout4vr" => GameId::Fallout4,
            "falloutnv" => GameId::FalloutNV,
            "fallout3" => GameId::Fallout3,
            "starfield" => GameId::Starfield,
            _ => return None,
        };
        Some(GameSpec {
            esplugin_id,
            mechanism,
            primary_plugins: def.primary_plugins.iter().map(|s| s.to_string()).collect(),
            local_dir: def.documents_dir.to_string(),
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
    /// Force-disabled: an `.esl` on an engine without light-plugin support
    /// (Skyrim LE / FO3 / FNV). It can never be activated (loading it would consume
    /// a normal index slot and shift every later plugin), so the UI must not offer a
    /// toggle - distinct from a merely default-inactive plugin the user CAN enable.
    pub force_disabled: bool,
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
    /// the header master flag or an `.esm`/`.esl` extension. MO2's hoisting
    /// predicate is `isMasterFlagged || hasMasterExtension || hasLightExtension`
    /// (`pluginlist.cpp`): the light *flag* (header 0x200) is deliberately NOT
    /// here, so a light-flagged `.esp` (an ESPFE patch) keeps its normal load
    /// position and only gets an `FE:` index (see `generate_indexes`).
    /// `is_master_ext` already covers `.esm`/`.esl`, so real masters still hoist.
    pub fn loads_as_master(&self) -> bool {
        self.is_master || is_master_ext(&self.name)
    }
}

/// The ordered plugin list.
#[derive(Debug, Clone, Default)]
pub struct PluginList {
    pub plugins: Vec<Plugin>,
    /// Plugins the ENGINE loads by itself, read from the game's `.ccc` file -
    /// lowercased. Populated by [`PluginList::discover`]; see [`implicit_plugins`].
    pub implicit: std::collections::HashSet<String>,
    /// Positions the user pinned, lowercased name -> the index it must occupy.
    /// MO2's `lockedorder.txt`, and keyed by NAME for the same reason: a lock has
    /// to survive the plugin being disabled, the list being rebuilt from disk, or
    /// LOOT reordering everything around it. An index-keyed lock would follow
    /// whatever moved into that slot instead.
    pub locked: std::collections::BTreeMap<String, usize>,
}


/// The plugins the ENGINE loads on its own, read from the game root's `.ccc` file
/// (`Skyrim.ccc`, `Fallout4.ccc`). Names are lowercased.
///
/// Creation Club content is loaded implicitly, exactly like `Skyrim.esm` and the
/// other primary masters, and the game therefore does NOT list it in
/// `plugins.txt` - a stock Anniversary install with no mods has an EMPTY
/// `plugins.txt`, header only. Writing those names in anyway makes the engine see
/// every Creation twice, and its answer is to blank the whole file the moment it
/// finishes loading. Everything that re-reads the list afterwards then believes
/// nothing is active: the save-game Creation check ("downloaded, but not
/// currently activated"), and any mod script that asks whether its own plugin is
/// present.
///
/// Measured on a 74-Creation install: Eidos wrote 77 active plugins, 75 of them
/// named in `Skyrim.ccc`, and the game blanked the file at `DataLoaded` - eleven
/// seconds before the session even ended. The correct file for that setup has
/// TWO lines, the two real mods.
///
/// Discovered by glob rather than a per-game constant: the file is named after
/// the game's master and sits beside the executable, so any title adopting the
/// convention works without a new table entry. A missing file yields an empty
/// list, which is right for every game that has no Creation Club.
///
/// Returned IN FILE ORDER, because that order is the engine's load order for this
/// content and the mod list wants to show it that way. Callers that only need
/// membership collect it into a set.
pub fn implicit_plugins(game_root: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(game_root) else { return Vec::new() };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("ccc"))
        })
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        out.extend(
            text.lines()
                .map(|l| l.trim().to_ascii_lowercase())
                .filter(|l| !l.is_empty() && !l.starts_with('#')),
        );
    }
    out
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
        // The first source is the game's own Data dir by contract, so the game
        // root - where the `.ccc` lives - is its parent.
        let implicit = sources
            .first()
            .and_then(|(_, data)| data.parent())
            .map(|root| implicit_plugins(root).into_iter().collect())
            .unwrap_or_default();

        for (origin, dir) in sources {
            let Ok(rd) = std::fs::read_dir(dir) else { continue };
            let mut found: Vec<(String, PathBuf)> = rd
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    is_plugin(&name).then(|| (name, e.path()))
                })
                .collect();
            found.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));

            for (name, path) in found {
                let key = name.to_ascii_lowercase();
                let (is_master, is_light, is_medium, masters) =
                    parse_header(&path, spec.esplugin_id).unwrap_or_else(|| {
                        // Unparseable: fall back to the extension.
                        (is_master_ext(&key), is_light_ext(&key), false, Vec::new())
                    });
                // An `.esl` on a no-light engine is force-disabled (a packaging error;
                // loading it would consume a normal index slot and shift every later
                // plugin's index).
                let force_disabled = is_light_ext(&key) && !spec.light_supported();
                if let Some(&i) = idx.get(&key) {
                    let p = &mut plugins[i];
                    p.path = path;
                    p.origin_mod = origin.clone();
                    p.is_master = is_master;
                    p.is_light = is_light;
                    p.is_medium = is_medium;
                    p.masters = masters;
                    p.force_disabled = force_disabled;
                    if force_disabled {
                        p.enabled = false;
                    }
                } else {
                    // A plugin from an ENABLED mod is active by default (MO2's opt-out
                    // model: enabling a mod activates its plugins; the user then
                    // unchecks the ones they don't want in the Plugins tab). Only a
                    // force-disabled .esl starts off. Plugins already recorded in the
                    // prefix's plugins.txt keep their state - apply_prefix_state runs
                    // after discover.
                    idx.insert(key, plugins.len());
                    plugins.push(Plugin {
                        name,
                        origin_mod: origin.clone(),
                        path,
                        enabled: !force_disabled,
                        force_disabled,
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
        // Discovery reads the game and the mods; the pins live in the profile and
        // are loaded over the top of this by the caller.
        PluginList { plugins, implicit, locked: Default::default() }
    }

    /// Re-sort to satisfy the ordering invariants, then assign mod indexes. Call
    /// after any change (enable/disable, reorder, discover).
    pub fn refresh(&mut self, spec: &GameSpec) {
        self.sort(spec);
        self.apply_locks(spec);
        self.generate_indexes(spec);
    }

    /// Put every pinned plugin back at the index the user pinned it to, then
    /// re-settle the master ordering around them. MO2's `lockedorder.txt`
    /// behaviour: a locked plugin holds its slot when LOOT sorts, when another
    /// plugin is dragged past it, and when the list is rebuilt from disk.
    ///
    /// The dependency invariant OUTRANKS the pin, always. A pin that would put a
    /// plugin above one of its own masters is a load the game cannot make, so
    /// the topological pass runs again afterwards and is allowed to undo it -
    /// [`violated_locks`] then reports which pins did not survive, because a lock
    /// that silently does nothing is worse than one that is refused out loud.
    pub fn apply_locks(&mut self, spec: &GameSpec) {
        if self.locked.is_empty() {
            return;
        }
        // Ascending target index, so each insert sees the slots below it already
        // filled and lands where the user pointed rather than being pushed along
        // by the pins that come after it.
        let mut pinned: Vec<(usize, Plugin)> = Vec::new();
        let mut rest: Vec<Plugin> = Vec::new();
        for p in std::mem::take(&mut self.plugins) {
            match self.locked.get(&p.name.to_ascii_lowercase()) {
                Some(&at) => pinned.push((at, p)),
                None => rest.push(p),
            }
        }
        pinned.sort_by_key(|(at, _)| *at);
        for (at, p) in pinned {
            let at = at.min(rest.len());
            rest.insert(at, p);
        }
        self.plugins = rest;

        // Primaries first and masters above plugins are engine rules, not
        // preferences, so re-impose them over the pins.
        let n = self.plugins.len();
        let primary_pos =
            |name: &str| spec.primary_plugins.iter().position(|p| p.eq_ignore_ascii_case(name));
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
        let mut settled: Vec<Plugin> = order.iter().map(|&i| self.plugins[i].clone()).collect();
        for (pos, p) in settled.iter_mut().enumerate() {
            p.priority = pos as i32;
        }
        self.plugins = settled;
    }

    /// Pins the engine rules overruled: the plugin is not at the index it was
    /// locked to. Returns `(name, wanted, actual)` so the UI can say which pin
    /// could not be honoured and where the plugin had to go instead.
    pub fn violated_locks(&self) -> Vec<(String, usize, usize)> {
        self.locked
            .iter()
            .filter_map(|(name, &want)| {
                let at = self.plugins.iter().position(|p| p.name.eq_ignore_ascii_case(name))?;
                (at != want).then(|| (name.clone(), want, at))
            })
            .collect()
    }

    /// Pin the plugin at `index` to where it currently sits, or release it.
    /// Returns whether anything changed.
    pub fn set_locked(&mut self, index: usize, locked: bool) -> bool {
        let Some(p) = self.plugins.get(index) else {
            return false;
        };
        let key = p.name.to_ascii_lowercase();
        if locked {
            self.locked.insert(key, index) != Some(index)
        } else {
            self.locked.remove(&key).is_some()
        }
    }

    /// Whether the plugin at `index` is pinned.
    pub fn is_locked(&self, index: usize) -> bool {
        self.plugins
            .get(index)
            .is_some_and(|p| self.locked.contains_key(&p.name.to_ascii_lowercase()))
    }

    /// Re-point the pins of the plugins named in `moved` at wherever they now
    /// sit, so a deliberate move of a pinned plugin sticks instead of being
    /// snapped straight back by its own lock on the next refresh.
    ///
    /// ONLY the plugins that were actually moved: a pin on a plugin something
    /// else was dropped past must NOT follow it along, or holding a slot would
    /// mean nothing the moment a neighbour shifted. And never after a LOOT sort,
    /// where resisting the sorter is the entire purpose of a pin.
    fn repin(&mut self, moved: &[String]) {
        if self.locked.is_empty() {
            return;
        }
        for name in moved {
            let key = name.to_ascii_lowercase();
            if !self.locked.contains_key(&key) {
                continue;
            }
            if let Some(i) = self.plugins.iter().position(|p| p.name.eq_ignore_ascii_case(name)) {
                self.locked.insert(key, i);
            }
        }
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

    /// Reorder plugins to follow `sorted` (a list of plugin file names, e.g. from
    /// LOOT). Names are matched case-insensitively. Plugins absent from `sorted`
    /// keep their relative order and are appended after the sorted ones (the sort is
    /// stable). Call `refresh` afterwards to recompute indexes and re-apply Eidos's
    /// master-before-dependent invariant, which always backstops the LOOT order.
    pub fn apply_sorted_order(&mut self, sorted: &[String]) {
        let rank: std::collections::HashMap<String, usize> = sorted
            .iter()
            .enumerate()
            .map(|(i, n)| (n.to_ascii_lowercase(), i))
            .collect();
        let tail = sorted.len();
        self.plugins
            .sort_by_key(|p| rank.get(&p.name.to_ascii_lowercase()).copied().unwrap_or(tail));
    }

    /// Move the plugin at `index` one slot towards the start (`up`) or the end of
    /// the load order, MO2's manual reorder. Returns whether anything moved -
    /// `refresh` afterwards re-applies the ordering invariants, which may pull the
    /// plugin back if the move would break masters-before-dependents.
    pub fn move_plugin(&mut self, index: usize, up: bool) -> bool {
        let target = if up {
            match index.checked_sub(1) {
                Some(t) => t,
                None => return false,
            }
        } else {
            index + 1
        };
        if index >= self.plugins.len() || target >= self.plugins.len() {
            return false;
        }
        let name = self.plugins[index].name.clone();
        self.plugins.swap(index, target);
        // Deliberate move: the pin follows. Done HERE rather than at the call
        // site because it has to happen between the move and the refresh, and a
        // caller that got that order wrong would leave a pinned plugin unable to
        // be moved at all - its own lock would snap it back every time.
        self.repin(&[name]);
        true
    }

    /// Move the plugins at `rows` so the block lands at the insertion point `gap`,
    /// keeping their relative order. `gap` counts BETWEEN rows: 0 is above the
    /// first plugin, `len()` is below the last - the same index the mod list's
    /// drop strips carry, so a drag reads identically in both panels.
    ///
    /// Lifting the sources shifts everything after them down, so a downward move
    /// compensates by however many moved rows sat before the target. Returns
    /// whether anything moved; `refresh` afterwards re-applies the engine
    /// ordering, which may pull the block back if the drop would load a plugin
    /// before one of its masters.
    pub fn move_plugins_to(&mut self, rows: &[usize], gap: usize) -> bool {
        let mut idx: Vec<usize> = rows.iter().copied().filter(|&i| i < self.plugins.len()).collect();
        idx.sort_unstable();
        idx.dedup();
        if idx.is_empty() {
            return false;
        }
        // A block dropped on either of its own edges has not gone anywhere; say so
        // rather than rewriting the load order and reporting a change.
        let contiguous = idx.last().unwrap() - idx.first().unwrap() + 1 == idx.len();
        if contiguous && (gap == idx[0] || gap == idx[idx.len() - 1] + 1) {
            return false;
        }
        let before = idx.iter().filter(|&&i| i < gap).count();
        let moved: Vec<String> = idx.iter().map(|&i| self.plugins[i].name.clone()).collect();
        let block: Vec<Plugin> = idx.iter().rev().map(|&i| self.plugins.remove(i)).collect();
        let at = gap.saturating_sub(before).min(self.plugins.len());
        // `block` came out highest-index-first, so re-insert in reverse to restore order.
        for p in block {
            self.plugins.insert(at, p);
        }
        self.repin(&moved);
        true
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
            force_disabled: false,
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
            implicit: Default::default(),
            locked: Default::default(),
        };
        list.sort(&se());
        assert_eq!(names(&list), vec!["Skyrim.esm", "Update.esm", "ZMod.esp"]);
    }

    #[test]
    fn masters_sort_above_normals_keeping_input_order() {
        let mut list = PluginList {
            plugins: vec![p("aaa.esp", &[]), p("zzz.esm", &[]), p("bbb.esp", &[]), p("mmm.esm", &[])],
            implicit: Default::default(),
            locked: Default::default(),
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
            implicit: Default::default(),
            locked: Default::default(),
        };
        list.sort(&se());
        let order = names(&list);
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("BaseMaster.esm") < pos("MidMaster.esm"));
        assert!(pos("MidMaster.esm") < pos("child.esp"));
    }

    #[test]
    fn esl_is_force_disabled_on_a_game_without_light_support() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("eidos-esl-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Patch.esl"), b"").unwrap();
        let sources = vec![(String::new(), dir.clone())];

        // Skyrim LE (PlainList, no light support): the .esl is FORCE-disabled - it
        // can never be activated, distinct from a merely default-inactive plugin.
        let le = GameSpec::for_id("skyrim").unwrap();
        let listed = PluginList::discover(&sources, &le);
        let le_p = listed.plugins.iter().find(|p| p.name == "Patch.esl").unwrap();
        assert!(le_p.force_disabled, ".esl must be force-disabled on a no-light game");
        assert!(!le_p.enabled);

        // Skyrim SE supports light plugins, so the same file is NOT force-disabled
        // and, coming from an enabled mod, is active by default (MO2's opt-out model).
        let se_list = PluginList::discover(&sources, &se());
        let se_p = se_list.plugins.iter().find(|p| p.name == "Patch.esl").unwrap();
        assert!(!se_p.force_disabled, ".esl is enableable on a light-capable game");
        assert!(se_p.enabled, "a plugin from an enabled mod is active by default");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_defaults_plugins_from_enabled_mods_active() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("eidos-def-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Skyrim.esm"), b"").unwrap(); // a primary master
        std::fs::write(dir.join("MyMod.esp"), b"").unwrap(); // a normal mod plugin
        let list = PluginList::discover(&[(String::new(), dir.clone())], &se());
        let esm = list.plugins.iter().find(|p| p.name.eq_ignore_ascii_case("Skyrim.esm")).unwrap();
        let esp = list.plugins.iter().find(|p| p.name == "MyMod.esp").unwrap();
        // MO2 opt-out model: a plugin from an enabled mod is active by default; the
        // user disables the ones they don't want.
        assert!(esm.enabled, "a game master defaults active");
        assert!(esp.enabled, "a plugin from an enabled mod defaults active");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn light_flagged_esp_stays_in_normal_tier_but_gets_fe_index() {
        // A light-FLAGGED .esp (ESPFE patch): .esp name, header 0x200 set
        // (is_light), NOT master-flagged. MO2's hoisting predicate excludes the
        // light flag, so it must NOT load as a master - it stays in the normal
        // section so the patch loads after what it patches.
        let espfe = {
            let mut x = p("Patch.esp", &[]);
            x.is_light = true;
            x
        };
        assert!(!espfe.loads_as_master(), "light-flagged .esp must stay in the normal tier");

        // A plain master and a real .esl still hoist (extension-based).
        assert!(p("Base.esm", &[]).loads_as_master());
        {
            let mut esl = p("Real.esl", &[]);
            esl.is_light = true;
            assert!(esl.loads_as_master(), ".esl extension must still hoist");
        }

        // Through the full pipeline: it sorts AFTER .esm/.esl masters, yet still
        // receives an FE: light index from generate_indexes.
        let mut list = PluginList {
            plugins: vec![
                espfe,
                {
                    let mut esl = p("Light.esl", &[]);
                    esl.is_light = true;
                    esl
                },
                p("Base.esm", &[]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        list.refresh(&se());
        let order = names(&list);
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("Base.esm") < pos("Patch.esp"), ".esm master loads before the ESPFE patch");
        assert!(pos("Light.esl") < pos("Patch.esp"), ".esl master loads before the ESPFE patch");

        // Base.esm is the only non-light plugin -> normal index 00; both lights
        // get FE: indexes, the patch sorting after Light.esl.
        let by = |n: &str| list.plugins.iter().find(|p| p.name == n).unwrap().index.clone();
        assert_eq!(by("Base.esm"), Some("00".to_string()));
        assert_eq!(by("Light.esl"), Some("FE:000".to_string()));
        assert_eq!(by("Patch.esp"), Some("FE:001".to_string()));
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
            implicit: Default::default(),
            locked: Default::default(),
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
            implicit: Default::default(),
            locked: Default::default(),
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

    #[test]
    fn extension_predicates() {
        // is_plugin: any of the three extensions, case-insensitive.
        assert!(is_plugin("Mod.esp") && is_plugin("Base.ESM") && is_plugin("Light.esl"));
        assert!(!is_plugin("readme.txt"));
        // is_master_ext (MO2 hasMasterExtension): .esm or .esl load above normals.
        assert!(is_master_ext("Base.esm") && is_master_ext("Light.ESL"));
        assert!(!is_master_ext("Mod.esp"));
        // is_light_ext: only .esl.
        assert!(is_light_ext("Light.esl") && !is_light_ext("Base.esm"));
    }

    /// A list of normal plugins, in the given order, ready to be moved around.
    fn esps(names: &[&str]) -> PluginList {
        PluginList {
            plugins: names.iter().map(|n| p(n, &[])).collect(),
            implicit: Default::default(),
            locked: Default::default(),
        }
    }

    #[test]
    fn a_dragged_plugin_lands_at_the_gap_it_was_dropped_on() {
        // Downward: lifting the source shifts everything after it, so the gap
        // index has to be compensated or the plugin lands one slot short.
        let mut l = esps(&["a.esp", "b.esp", "c.esp", "d.esp"]);
        assert!(l.move_plugins_to(&[0], 3));
        assert_eq!(names(&l), ["b.esp", "c.esp", "a.esp", "d.esp"]);

        // Upward needs no compensation.
        let mut l = esps(&["a.esp", "b.esp", "c.esp", "d.esp"]);
        assert!(l.move_plugins_to(&[3], 1));
        assert_eq!(names(&l), ["a.esp", "d.esp", "b.esp", "c.esp"]);

        // The gap past the last row is the only way to aim at the end.
        let mut l = esps(&["a.esp", "b.esp", "c.esp"]);
        assert!(l.move_plugins_to(&[0], 3));
        assert_eq!(names(&l), ["b.esp", "c.esp", "a.esp"]);
    }

    #[test]
    fn dropping_a_plugin_back_on_its_own_edges_changes_nothing() {
        // Both strips touching a row mean "leave it here". Reporting a move
        // would rewrite plugins.txt for a gesture that did nothing.
        let mut l = esps(&["a.esp", "b.esp", "c.esp"]);
        assert!(!l.move_plugins_to(&[1], 1));
        assert!(!l.move_plugins_to(&[1], 2));
        assert_eq!(names(&l), ["a.esp", "b.esp", "c.esp"]);
        // Out of range and empty are no-ops too, not panics.
        assert!(!l.move_plugins_to(&[], 0));
        assert!(!l.move_plugins_to(&[99], 0));
        assert_eq!(names(&l), ["a.esp", "b.esp", "c.esp"]);
    }

    #[test]
    fn a_pinned_plugin_holds_its_slot_when_the_order_is_resorted() {
        let mut l = esps(&["a.esp", "b.esp", "c.esp", "d.esp"]);
        // Pin d.esp to the top and re-settle: it must be there afterwards.
        l.locked.insert("d.esp".to_string(), 0);
        l.refresh(&se());
        assert_eq!(names(&l), ["d.esp", "a.esp", "b.esp", "c.esp"]);
        assert!(l.violated_locks().is_empty());

        // And it resists a move that would displace it: dragging a.esp to the
        // top pushes it into slot 1, because slot 0 is spoken for.
        assert!(l.move_plugins_to(&[1], 0));
        l.refresh(&se());
        assert_eq!(names(&l), ["d.esp", "a.esp", "b.esp", "c.esp"]);
    }

    #[test]
    fn several_pins_are_placed_low_slot_first() {
        // Applied in descending order, the later pin would shove the earlier one
        // along and neither would end up where it was asked for.
        let mut l = esps(&["a.esp", "b.esp", "c.esp", "d.esp"]);
        l.locked.insert("d.esp".to_string(), 0);
        l.locked.insert("c.esp".to_string(), 1);
        l.refresh(&se());
        assert_eq!(names(&l), ["d.esp", "c.esp", "a.esp", "b.esp"]);
        assert!(l.violated_locks().is_empty());
    }

    #[test]
    fn the_engine_rules_outrank_a_pin_and_say_so() {
        // Pinning a plugin above its own master is a load the game cannot make.
        // The pin loses - and is REPORTED, because a lock that silently does
        // nothing leaves the user believing a position is held when it is not.
        let mut l = PluginList {
            plugins: vec![p("Skyrim.esm", &[]), p("Patch.esp", &["Skyrim.esm"])],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.locked.insert("patch.esp".to_string(), 0);
        l.refresh(&se());
        assert_eq!(names(&l), ["Skyrim.esm", "Patch.esp"]);

        let bad = l.violated_locks();
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0], ("patch.esp".to_string(), 0, 1));
    }

    #[test]
    fn a_pin_survives_the_plugin_being_disabled_and_coming_back() {
        // Keyed by name, not by index: the whole point is that the slot is held
        // for THAT plugin across a rebuild of the list from disk.
        let mut l = esps(&["a.esp", "b.esp", "c.esp"]);
        assert!(l.set_locked(2, true));
        assert!(l.is_locked(2));

        // The list is rediscovered in a different order; the pin still applies.
        let mut rebuilt = esps(&["c.esp", "a.esp", "b.esp"]);
        rebuilt.locked = l.locked.clone();
        rebuilt.refresh(&se());
        assert_eq!(names(&rebuilt), ["a.esp", "b.esp", "c.esp"]);

        // Releasing it is idempotent, and reports whether it did anything.
        assert!(rebuilt.set_locked(2, false));
        assert!(!rebuilt.set_locked(2, false));
        assert!(!rebuilt.is_locked(2));
    }

    #[test]
    fn deliberately_moving_a_pinned_plugin_repins_it_where_it_landed() {
        // Otherwise a pinned plugin could never be moved again: its own lock
        // would snap it back on the next refresh and the drag would look broken.
        let mut l = esps(&["a.esp", "b.esp", "c.esp"]);
        l.set_locked(0, true);
        assert!(l.move_plugins_to(&[0], 3));
        l.refresh(&se());
        assert_eq!(names(&l), ["b.esp", "c.esp", "a.esp"]);
        assert_eq!(l.locked.get("a.esp"), Some(&2));

        // The arrow buttons are a deliberate move too, and must not be defeated
        // by the plugin's own pin.
        assert!(l.move_plugin(2, true));
        l.refresh(&se());
        assert_eq!(names(&l), ["b.esp", "a.esp", "c.esp"]);
        assert_eq!(l.locked.get("a.esp"), Some(&1));
    }

    #[test]
    fn a_pin_does_not_follow_a_plugin_dropped_past_it() {
        // The failure this pins: repinning everything after a move meant a pin
        // slid along whenever a neighbour shifted, so holding a slot held
        // nothing. Only the plugin that was actually dragged repins.
        let mut l = esps(&["a.esp", "b.esp", "c.esp", "d.esp"]);
        l.set_locked(2, true); // c.esp pinned to slot 2
        assert_eq!(l.locked.get("c.esp"), Some(&2));

        // Drop d.esp above a.esp: c.esp is pushed to 3, but its pin still says 2,
        // so the refresh pulls it back and the pin has done its job.
        assert!(l.move_plugins_to(&[3], 0));
        l.refresh(&se());
        assert_eq!(l.locked.get("c.esp"), Some(&2));
        assert_eq!(names(&l), ["d.esp", "a.esp", "c.esp", "b.esp"]);
    }
}
