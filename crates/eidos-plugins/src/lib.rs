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

/// Where a plugin is allowed to sit in the load order, and what bounds it.
///
/// `lo`/`hi` are INSERTION points (gaps), inclusive, in the same numbering the
/// drop strips use. `after` and `before` name the plugins doing the bounding, so
/// the UI can say *why* rather than merely refusing.
///
/// Do NOT read `lo == hi` as "cannot move": a plugin's own two edges are both
/// legal and both no-ops, so a completely boxed-in plugin at index `i` reports
/// `lo == i, hi == i + 1`. Ask [`MovableRange::is_stuck`] instead - reading the
/// degenerate case for the stuck case is a real bug this comment used to cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovableRange {
    pub lo: usize,
    pub hi: usize,
    /// The last of its own masters that is present: it must load after this.
    pub after: Option<String>,
    /// The first plugin declaring it as a master: it must load before this.
    pub before: Option<String>,
    /// Insertion points inside `lo..=hi` that are nonetheless refused, because a
    /// PINNED plugin owns the slot they would land on.
    ///
    /// A pin is a hole, not a bound: a plugin may be dragged straight past a
    /// pinned neighbour - the pinned one is put back afterwards and everything
    /// flows around it - but it may not come to rest ON the pinned slot, because
    /// there the pin wins and the drop silently lands one row off.
    pub blocked: Vec<usize>,
}

impl MovableRange {
    /// Whether the plugin grabbed at row `from` has nowhere to go: every gap the
    /// range allows is one of its own two edges, which both leave it exactly
    /// where it is.
    ///
    /// This is the honest immovability test. `lo == hi` is NOT: it holds only
    /// for a primary game master or a contradictory rule set, so a plugin
    /// wedged between its master and its dependent - the common case, and the
    /// one users hit - slipped through it and got told it was free to move.
    pub fn is_stuck(&self, from: usize) -> bool {
        self.lo >= from && self.hi <= from + 1
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
        let (base, tier) = tier_order(&self.plugins, spec);
        let order = topo_stable(&self.plugins, &base, &tier);
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
        // A pin past the end of a list that merely got shorter - the mod
        // providing the plugins below it was disabled - is still honoured: the
        // plugin is as late as it can be. Comparing against the raw index would
        // report every such pin as overruled by the engine, which is simply
        // false, and the banner blames masters that had nothing to do with it.
        let last = self.plugins.len().saturating_sub(1);
        self.locked
            .iter()
            .filter_map(|(name, &want)| {
                let at = self.plugins.iter().position(|p| p.name.eq_ignore_ascii_case(name))?;
                let reachable = want.min(last);
                (at != reachable).then(|| (name.clone(), want, at))
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
        // Tier + primary sub-order key; input position is the stable tiebreak.
        let (base, tier) = tier_order(&self.plugins, spec);
        let order = topo_stable(&self.plugins, &base, &tier);
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

    /// Whether a ONE-SLOT move is legal, checked against the immediate neighbour
    /// only - which is all a single step can violate, and costs O(masters)
    /// instead of the O(n) [`movable_range`] needs. The arrow buttons ask this
    /// per row on every frame, so the difference matters.
    pub fn can_move(&self, index: usize, up: bool, spec: &GameSpec) -> bool {
        let Some(me) = self.plugins.get(index) else { return false };
        let is_primary = |n: &str| {
            spec.primary_plugins.iter().any(|pp| pp.eq_ignore_ascii_case(n))
                || self.implicit.contains(&n.to_ascii_lowercase())
        };
        if is_primary(&me.name) {
            return false;
        }
        let Some(other) = (if up {
            index.checked_sub(1).and_then(|i| self.plugins.get(i))
        } else {
            self.plugins.get(index + 1)
        }) else {
            return false;
        };
        // A pinned neighbour owns its slot; stepping onto it would be undone by
        // apply_locks on the very next refresh.
        if self.locked.contains_key(&other.name.to_ascii_lowercase()) {
            return false;
        }
        if up {
            // Cannot climb over one of my own masters, over the primary block,
            // or - if I am a normal plugin - into the master block at all.
            if is_primary(&other.name) {
                return false;
            }
            if me.masters.iter().any(|m| m.eq_ignore_ascii_case(&other.name)) {
                return false;
            }
            if !me.loads_as_master() && other.loads_as_master() {
                return false;
            }
        } else {
            // Cannot sink below something that declares me as its master, and a
            // master cannot sink into the normal block.
            if other.masters.iter().any(|m| m.eq_ignore_ascii_case(&me.name)) {
                return false;
            }
            if me.loads_as_master() && !other.loads_as_master() {
                return false;
            }
        }
        true
    }

    /// Move the plugin at `index` one slot towards the start (`up`) or the end of
    /// the load order, MO2's manual reorder. Returns whether anything moved.
    ///
    /// An illegal step is refused HERE rather than being made and then undone by
    /// the next `refresh`. The difference is not cosmetic: the move carries the
    /// plugin's pin with it, so a move that is silently reverted would leave the
    /// pin recording a slot the plugin cannot occupy - permanently violated, and
    /// re-applied on every later refresh.
    pub fn move_plugin(&mut self, index: usize, up: bool, spec: &GameSpec) -> bool {
        if !self.can_move(index, up, spec) {
            return false;
        }
        self.move_plugin_unchecked(index, up)
    }

    fn move_plugin_unchecked(&mut self, index: usize, up: bool) -> bool {
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

    /// The insertion points the plugin at `index` may legally be dropped on, as an
    /// inclusive gap range `(lo, hi)`, plus the two plugins that bound it.
    ///
    /// Every ordering rule the engine imposes is an index constraint, and each one
    /// cuts the same range from one side, so the legal region is always ONE
    /// contiguous interval - which is what makes this worth computing: the UI can
    /// simply refuse to offer a strip outside it, instead of accepting a drop and
    /// silently undoing it afterwards. That silence is what made a correct refusal
    /// read as a broken feature.
    ///
    /// The bounds, in the order they bind:
    /// - after the last of its own masters that is present (a plugin loaded before
    ///   its master resolves every FormID it borrows against the wrong record);
    /// - before the first plugin that declares IT as a master, symmetrically;
    /// - inside its tier, since masters all load above normal plugins;
    /// - and a primary game master does not move at all.
    ///
    /// Assumes the list is already sorted (tiers contiguous), which is true after
    /// any `refresh`. Use [`MovableRange::is_stuck`] to ask whether the result
    /// leaves the plugin anywhere to go.
    pub fn movable_range(&self, index: usize, spec: &GameSpec) -> Option<MovableRange> {
        self.range_excluding(index, spec, &[index])
    }

    /// [`movable_range`](Self::movable_range) for a plugin moving as part of a
    /// BLOCK: ties to the other rows in `block` are ignored, because they travel
    /// with it and their relative order is preserved.
    ///
    /// Without this, a master and its own dependent selected together could
    /// never be moved down past anything - the master's "must load before its
    /// dependent" bound would point at a row that is moving too, and a perfectly
    /// legal move would be refused.
    fn range_excluding(
        &self,
        index: usize,
        spec: &GameSpec,
        block: &[usize],
    ) -> Option<MovableRange> {
        let me = self.plugins.get(index)?;
        let len = self.plugins.len();
        // Creation Club content is loaded by the ENGINE from the .ccc file, at a
        // position Eidos does not choose and never writes. Treating it as
        // ordinary let the user drag it, accepted the drop, wrote it out, and
        // then the next discovery put it back where the engine says - a move
        // that looked like it worked and silently was not.
        let is_primary = |n: &str| {
            spec.primary_plugins.iter().any(|pp| pp.eq_ignore_ascii_case(n))
                || self.implicit.contains(&n.to_ascii_lowercase())
        };
        if is_primary(&me.name) {
            return Some(MovableRange {
                lo: index,
                hi: index,
                blocked: Vec::new(),
                after: None,
                before: None,
            });
        }
        // Tier boundaries. The list is sorted, so both blocks are contiguous.
        let primaries_end = self.plugins.iter().position(|p| !is_primary(&p.name)).unwrap_or(len);
        let normals_start = self
            .plugins
            .iter()
            .position(|p| !is_primary(&p.name) && !p.loads_as_master())
            .unwrap_or(len);

        // The last master of mine that is actually present: I must land after it.
        let mut after: Option<(usize, String)> = None;
        for (i, p) in self.plugins.iter().enumerate() {
            if !block.contains(&i) && me.masters.iter().any(|m| m.eq_ignore_ascii_case(&p.name)) {
                after = Some((i, p.name.clone()));
            }
        }
        // The first plugin that declares me as ITS master: I must land before it.
        let before = self
            .plugins
            .iter()
            .enumerate()
            .find(|(i, p)| {
                !block.contains(i) && p.masters.iter().any(|m| m.eq_ignore_ascii_case(&me.name))
            })
            .map(|(i, p)| (i, p.name.clone()));

        let (mut lo, mut hi) = (0usize, len);
        if let Some((i, _)) = &after {
            lo = i + 1;
        }
        if let Some((i, _)) = &before {
            hi = *i;
        }
        // Where a pin would steal the landing. Lifting the row out shifts
        // everything after it down by one, so the gap that lands ON slot `k` is
        // `k` when the pin sits above the grabbed row and `k + 1` when it sits
        // below. The plugin's own pin is not an obstacle to itself - a
        // deliberate move re-points it.
        let mut blocked: Vec<usize> = Vec::new();
        for (i, p) in self.plugins.iter().enumerate() {
            if block.contains(&i) || !self.locked.contains_key(&p.name.to_ascii_lowercase()) {
                continue;
            }
            blocked.push(if i < index { i } else { i + 1 });
        }
        blocked.retain(|g| *g >= lo && *g <= hi);
        blocked.sort_unstable();
        blocked.dedup();
        if me.loads_as_master() {
            lo = lo.max(primaries_end);
            hi = hi.min(normals_start);
        } else {
            lo = lo.max(normals_start);
        }
        // A contradictory set of rules must not produce an inverted range; pin the
        // plugin where it is rather than handing the UI something nonsensical.
        if lo > hi {
            lo = index;
            hi = index;
            blocked.clear();
        }
        Some(MovableRange {
            lo,
            hi,
            blocked,
            after: after.map(|(_, n)| n),
            before: before.map(|(_, n)| n),
        })
    }

    /// Where a BLOCK of plugins may legally land, as the intersection of what
    /// each of its rows allows once ties internal to the block are discounted.
    ///
    /// A block travels with its relative order intact, so a master and its own
    /// dependent selected together constrain each other not at all - only what
    /// is OUTSIDE the block can bound it. The bounding names come from whichever
    /// row binds tightest, so the explanation the UI shows names the plugin that
    /// is actually in the way.
    pub fn block_movable_range(&self, rows: &[usize], spec: &GameSpec) -> Option<MovableRange> {
        let mut idx: Vec<usize> = rows.iter().copied().filter(|&i| i < self.plugins.len()).collect();
        idx.sort_unstable();
        idx.dedup();
        let (&first, &last) = (idx.first()?, idx.last()?);
        let mut out = MovableRange {
            lo: 0,
            hi: self.plugins.len(),
            blocked: Vec::new(),
            after: None,
            before: None,
        };
        for &i in &idx {
            let r = self.range_excluding(i, spec, &idx)?;
            if r.lo > out.lo {
                out.lo = r.lo;
                out.after = r.after.clone();
            }
            if r.hi < out.hi {
                out.hi = r.hi;
                out.before = r.before.clone();
            }
            out.blocked.extend(r.blocked);
        }
        out.blocked.retain(|g| *g >= out.lo && *g <= out.hi);
        out.blocked.sort_unstable();
        out.blocked.dedup();
        // A contradictory intersection must not hand the UI an inverted range;
        // pin the block where it already is.
        if out.lo > out.hi {
            out.lo = first;
            out.hi = last + 1;
            out.blocked.clear();
        }
        Some(out)
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
    pub fn move_plugins_to(&mut self, rows: &[usize], gap: usize, spec: &GameSpec) -> bool {
        let mut idx: Vec<usize> = rows.iter().copied().filter(|&i| i < self.plugins.len()).collect();
        idx.sort_unstable();
        idx.dedup();
        if idx.is_empty() {
            return false;
        }
        // Refuse a destination the engine forbids, instead of moving there and
        // letting the next refresh quietly undo it - which would strand the
        // plugin's pin on a slot it can never occupy. For a block, every row's
        // range must allow the gap: a move that would be half-undone is refused
        // whole rather than landing somewhere nobody asked for.
        for &i in &idx {
            match self.range_excluding(i, spec, &idx) {
                Some(r) if gap >= r.lo && gap <= r.hi && !r.blocked.contains(&gap) => {}
                _ => return false,
            }
        }
        // A contiguous block dropped anywhere between its own first row and just
        // past its last has not gone anywhere - the interior gaps are inside the
        // block being lifted, so they all resolve to the position it already
        // holds. Reporting a move there would rewrite the load order and raise a
        // status message for a gesture that changed nothing.
        let contiguous = idx.last().unwrap() - idx.first().unwrap() + 1 == idx.len();
        if contiguous && gap >= idx[0] && gap <= idx[idx.len() - 1] + 1 {
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
/// The engine's tier for each plugin - 0 a primary game master, 1 anything that
/// loads as a master, 2 a normal plugin - together with the indices sorted into
/// that order: primaries in their canonical sequence, and the input order
/// preserved inside each tier.
///
/// The two are returned together because they have to agree. `topo_stable` reads
/// the tiers to decide which rows it may emit next, and deriving them a second
/// time from a second copy of the same closure is exactly how the sort and the
/// pin pass would drift apart.
fn tier_order(plugins: &[Plugin], spec: &GameSpec) -> (Vec<usize>, Vec<u8>) {
    let primary_pos =
        |name: &str| spec.primary_plugins.iter().position(|p| p.eq_ignore_ascii_case(name));
    let tier: Vec<u8> = plugins
        .iter()
        .map(|p| {
            if primary_pos(&p.name).is_some() {
                0
            } else if p.loads_as_master() {
                1
            } else {
                2
            }
        })
        .collect();
    let mut base: Vec<usize> = (0..plugins.len()).collect();
    base.sort_by_key(|&i| (tier[i], primary_pos(&plugins[i].name).unwrap_or(usize::MAX), i));
    (base, tier)
}

fn topo_stable(plugins: &[Plugin], base: &[usize], tier: &[u8]) -> Vec<usize> {
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
        // Only the earliest tier still holding rows may be emitted. `base` is
        // tier-sorted, so the head of the unplaced remainder identifies it - and
        // restricting candidates to it is what makes the tier rule (all masters
        // above all normal plugins) survive a dependency graph that cannot be
        // satisfied. Without the restriction, a normal plugin that happened to
        // be unblocked was emitted while two mutually-mastering .esm files were
        // stuck, and both masters ended up BELOW it: a load order the engine
        // cannot honour, produced in silence.
        let Some(head) = base.iter().copied().find(|&i| !placed[i]) else { break };
        let tier_end = base
            .iter()
            .position(|&i| !placed[i] && tier[i] != tier[head])
            .unwrap_or(base.len());
        let mut best: Option<usize> = None;
        for &i in &base[..tier_end] {
            if !placed[i] && indeg[i] == 0 && best.is_none_or(|b| base_pos[i] < base_pos[b]) {
                best = Some(i);
            }
        }
        // Nothing available inside the tier means a cycle within it (A masters
        // B, B masters A - malformed, but hand-edited plugins do it). Break it
        // at the earliest row rather than stalling: only the cyclic edge is
        // given up, and the ordering everything else depends on is kept.
        let Some(i) = best.or(Some(head)) else { break };
        placed[i] = true;
        result.push(i);
        for &d in &dependents[i] {
            indeg[d] = indeg[d].saturating_sub(1);
        }
    }
    // Nothing should be left now, but stay total rather than dropping rows.
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
        assert!(l.move_plugins_to(&[0], 3, &se()));
        assert_eq!(names(&l), ["b.esp", "c.esp", "a.esp", "d.esp"]);

        // Upward needs no compensation.
        let mut l = esps(&["a.esp", "b.esp", "c.esp", "d.esp"]);
        assert!(l.move_plugins_to(&[3], 1, &se()));
        assert_eq!(names(&l), ["a.esp", "d.esp", "b.esp", "c.esp"]);

        // The gap past the last row is the only way to aim at the end.
        let mut l = esps(&["a.esp", "b.esp", "c.esp"]);
        assert!(l.move_plugins_to(&[0], 3, &se()));
        assert_eq!(names(&l), ["b.esp", "c.esp", "a.esp"]);
    }

    #[test]
    fn dropping_a_plugin_back_on_its_own_edges_changes_nothing() {
        // Both strips touching a row mean "leave it here". Reporting a move
        // would rewrite plugins.txt for a gesture that did nothing.
        let mut l = esps(&["a.esp", "b.esp", "c.esp"]);
        assert!(!l.move_plugins_to(&[1], 1, &se()));
        assert!(!l.move_plugins_to(&[1], 2, &se()));
        assert_eq!(names(&l), ["a.esp", "b.esp", "c.esp"]);
        // Out of range and empty are no-ops too, not panics.
        assert!(!l.move_plugins_to(&[], 0, &se()));
        assert!(!l.move_plugins_to(&[99], 0, &se()));
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

        // And slot 0 is spoken for: dropping a.esp there is refused outright,
        // rather than accepted and then quietly undone by d.esp's own pin - the
        // range reports that gap as blocked, so it is never offered either.
        assert!(l.movable_range(1, &se()).unwrap().blocked.contains(&0));
        assert!(!l.move_plugins_to(&[1], 0, &se()));
        assert_eq!(names(&l), ["d.esp", "a.esp", "b.esp", "c.esp"]);

        // Crossing a pin is still allowed, though - only landing ON it is not.
        // c.esp travels from the bottom to slot 1 and d.esp stays pinned at 0.
        assert!(l.move_plugins_to(&[3], 1, &se()));
        l.refresh(&se());
        assert_eq!(names(&l), ["d.esp", "c.esp", "a.esp", "b.esp"]);
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
        assert!(l.move_plugins_to(&[0], 3, &se()));
        l.refresh(&se());
        assert_eq!(names(&l), ["b.esp", "c.esp", "a.esp"]);
        assert_eq!(l.locked.get("a.esp"), Some(&2));

        // The arrow buttons are a deliberate move too, and must not be defeated
        // by the plugin's own pin.
        assert!(l.move_plugin(2, true, &se()));
        l.refresh(&se());
        assert_eq!(names(&l), ["b.esp", "a.esp", "c.esp"]);
        assert_eq!(l.locked.get("a.esp"), Some(&1));
    }

    #[test]
    fn a_chain_of_masters_cannot_be_reordered_and_that_is_correct() {
        // Reported as "I cannot move these four, the drag is broken". It is not:
        // Kurone Soul Tomb ships five plugins in a strict master chain, each
        // declaring the previous one in its MAST records (read off the real
        // files). A plugin loaded before its own master resolves every FormID it
        // borrows against the wrong record, so the engine forbids it and so does
        // Eidos. The move is refused, correctly - what is missing is saying so.
        let mut l = PluginList {
            plugins: vec![
                p("KuroneSoulTomb.esp", &[]),
                p("KuroneSoulTomb_EX1.esp", &["KuroneSoulTomb.esp"]),
                p("KuroneSoulTomb_EX2.esp", &["KuroneSoulTomb.esp", "KuroneSoulTomb_EX1.esp"]),
                p(
                    "KuroneSoulTomb_EX3.esp",
                    &["KuroneSoulTomb.esp", "KuroneSoulTomb_EX1.esp", "KuroneSoulTomb_EX2.esp"],
                ),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        let before = names(&l);

        // Drag EX2 above EX1: refused outright, because EX1 is one of EX2's
        // masters. Refused rather than made-and-reverted, so nothing downstream
        // (the pin, the disk write) ever sees an order the engine forbids.
        assert!(!l.move_plugins_to(&[2], 1, &se()));
        assert_eq!(names(&l), before);

        // Same through the arrow button, same answer, and cheaply: can_move only
        // has to look at the one neighbour a single step can cross.
        assert!(!l.can_move(2, true, &se()));
        assert!(!l.move_plugin(2, true, &se()));
        assert_eq!(names(&l), before);

        // Downward is refused too, from the other side of the tie: EX3 declares
        // EX2 as ITS master, so EX2 cannot sink past it.
        assert!(!l.can_move(2, false, &se()));

        // A plugin with no such tie moves freely, so the refusal is the master
        // rule and not a broken reorder.
        l.plugins.push(p("Free.esp", &[]));
        l.refresh(&se());
        assert!(l.move_plugins_to(&[4], 0, &se()));
        l.refresh(&se());
        assert_eq!(names(&l)[0], "Free.esp");
    }

    #[test]
    fn the_legal_range_names_the_plugins_that_bound_it() {
        // The same Kurone chain. Every link in it has exactly one legal slot, so
        // the range collapses - and it can say WHICH plugin closed it from each
        // side, which is the whole point: a refusal the user can read.
        let mut l = PluginList {
            plugins: vec![
                p("Skyrim.esm", &[]),
                p("KuroneSoulTomb.esp", &["Skyrim.esm"]),
                p("KuroneSoulTomb_EX1.esp", &["KuroneSoulTomb.esp"]),
                p("KuroneSoulTomb_EX2.esp", &["KuroneSoulTomb_EX1.esp"]),
                p("Free.esp", &[]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        assert_eq!(
            names(&l),
            [
                "Skyrim.esm",
                "KuroneSoulTomb.esp",
                "KuroneSoulTomb_EX1.esp",
                "KuroneSoulTomb_EX2.esp",
                "Free.esp"
            ]
        );

        // EX1 is boxed in on both sides: after KuroneSoulTomb.esp, before EX2.
        let r = l.movable_range(2, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (2, 3));
        assert_eq!(r.after.as_deref(), Some("KuroneSoulTomb.esp"));
        assert_eq!(r.before.as_deref(), Some("KuroneSoulTomb_EX2.esp"));

        // EX2 has nothing depending on it, so it is free below its master.
        let r = l.movable_range(3, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (3, 5));
        assert_eq!(r.after.as_deref(), Some("KuroneSoulTomb_EX1.esp"));
        assert_eq!(r.before, None);

        // A plugin with no ties may go anywhere below the master block.
        let r = l.movable_range(4, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (1, 5));
        assert_eq!(r.after, None);

        // And a primary game master does not move at all.
        let r = l.movable_range(0, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (0, 0));
    }

    #[test]
    fn a_normal_plugin_may_not_climb_into_the_master_block() {
        // Not a dependency rule - the engine loads every master above every
        // normal plugin, so the tier boundary bounds the range too.
        let mut l = PluginList {
            plugins: vec![
                p("Skyrim.esm", &[]),
                p("Big.esm", &[]),
                p("One.esp", &[]),
                p("Two.esp", &[]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        // Two.esp cannot go above index 2, where the normal plugins start.
        let r = l.movable_range(3, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (2, 4));
        assert_eq!(r.after, None);
        // Big.esm is stuck between the primary block and the normal block.
        let r = l.movable_range(1, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (1, 2));
    }

    #[test]
    fn a_shorter_list_is_not_the_engine_overruling_a_pin() {
        // Pin the last plugin, then disable the mod supplying the three above
        // it. The pin's recorded index is now past the end - but the plugin is
        // still last, exactly as pinned. Comparing raw indexes reported it as
        // overruled, and the banner blamed master ordering that had nothing to
        // do with it: a false alarm on a completely healthy setup.
        let mut l = esps(&["a.esp", "b.esp", "x.esp", "y.esp", "z.esp", "c.esp"]);
        assert!(l.set_locked(5, true));
        l.refresh(&se());
        assert!(l.violated_locks().is_empty());

        let mut shorter = esps(&["a.esp", "b.esp", "c.esp"]);
        shorter.locked = l.locked.clone();
        shorter.refresh(&se());
        assert_eq!(names(&shorter), ["a.esp", "b.esp", "c.esp"]);
        assert!(shorter.violated_locks().is_empty(), "{:?}", shorter.violated_locks());
    }

    #[test]
    fn a_block_dropped_inside_itself_reports_no_move() {
        // Every gap between a contiguous block's first row and just past its
        // last is INSIDE the block being lifted, so they all resolve to the
        // position it already holds. Only the two edges were caught, so an
        // interior gap rewrote plugins.txt and announced a move that never
        // happened. Not reachable while the UI drags one row, but it will be
        // the moment plugins get multi-select.
        let mut l = esps(&["a.esp", "b.esp", "c.esp", "d.esp", "e.esp"]);
        let before = names(&l);
        for gap in 1..=4 {
            assert!(!l.move_plugins_to(&[1, 2, 3], gap, &se()), "gap {gap}");
            assert_eq!(names(&l), before, "gap {gap}");
        }
        // Just outside, it does move.
        assert!(l.move_plugins_to(&[1, 2, 3], 5, &se()));
        assert_eq!(names(&l), ["a.esp", "e.esp", "b.esp", "c.esp", "d.esp"]);
    }

    #[test]
    fn a_master_and_its_dependent_travel_together() {
        // Ties INSIDE the moving block are not obstacles: the two rows keep
        // their relative order, so the master still loads before its dependent
        // wherever they land. Judging each row against the other refused the
        // whole move.
        let mut l = PluginList {
            plugins: vec![
                p("Base.esm", &[]),
                p("Patch.esp", &["Base.esm"]),
                p("One.esp", &[]),
                p("Two.esp", &[]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        // Patch.esp and the two free plugins are all normal; move Patch down
        // past both. Base.esm is a master and stays in the master block.
        assert!(l.move_plugins_to(&[1], 4, &se()));
        l.refresh(&se());
        assert_eq!(names(&l), ["Base.esm", "One.esp", "Two.esp", "Patch.esp"]);
    }

    #[test]
    fn a_master_cycle_does_not_sink_the_masters_below_the_plugins() {
        // Two masters each declaring the other - malformed, but it exists on
        // hand-edited plugins. The topological pass cannot order them, and it
        // used to dump both at the END of the list, under every .esp: a load
        // order the engine cannot honour, produced in silence. The cycle is
        // broken at the earliest row instead, so the tier invariant survives.
        let mut l = PluginList {
            plugins: vec![
                p("Skyrim.esm", &[]),
                p("A.esm", &["Skyrim.esm", "B.esm"]),
                p("B.esm", &["Skyrim.esm", "A.esm"]),
                p("Z.esp", &["Skyrim.esm"]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        let pos = |n: &str| names(&l).iter().position(|x| x == n).unwrap();
        assert!(pos("A.esm") < pos("Z.esp"), "{:?}", names(&l));
        assert!(pos("B.esm") < pos("Z.esp"), "{:?}", names(&l));
        // And every plugin is still present exactly once.
        assert_eq!(l.plugins.len(), 4);
    }

    #[test]
    fn a_boxed_in_plugin_reports_itself_as_stuck() {
        // The bug this pins, found by the audit against the real Kurone chain:
        // a plugin's own two edges are both legal gaps, so a completely wedged
        // plugin at index i reports lo == i and hi == i + 1 - NOT lo == hi. The
        // GUI tested lo == hi, so the wedged case fell into the "can move
        // between A and B" branch and the panel told the user a plugin was free
        // to move while offering nowhere to move it. A false statement is worse
        // than the silence it replaced.
        let mut l = PluginList {
            plugins: vec![
                p("Skyrim.esm", &[]),
                p("KuroneSoulTomb.esp", &["Skyrim.esm"]),
                p("KuroneSoulTomb_EX1.esp", &["KuroneSoulTomb.esp"]),
                p("KuroneSoulTomb_EX2.esp", &["KuroneSoulTomb_EX1.esp"]),
                p("KuroneSoulTomb_EX3.esp", &["KuroneSoulTomb_EX2.esp"]),
                p("Free.esp", &[]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());

        // EX1 at index 2 is wedged between its master and its dependent.
        let r = l.movable_range(2, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (2, 3));
        assert_ne!(r.lo, r.hi, "the degenerate test is exactly what was wrong");
        assert!(r.is_stuck(2));

        // A primary master is stuck too, through the other path.
        assert!(l.movable_range(0, &se()).unwrap().is_stuck(0));

        // And the free plugin is NOT stuck - it may go anywhere below the master
        // block. A test that called everything stuck would be no better than the
        // one that called nothing stuck.
        let r = l.movable_range(5, &se()).unwrap();
        assert!(!r.is_stuck(5), "lo {} hi {}", r.lo, r.hi);
        // EX2 is wedged the same way as EX1, and reports the same shape.
        let r = l.movable_range(3, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (3, 4));
        assert!(r.is_stuck(3));

        // The end of the chain can still sink past the free plugin, so it is
        // NOT stuck - the range has to distinguish the two.
        let r = l.movable_range(4, &se()).unwrap();
        assert_eq!((r.lo, r.hi), (4, 6));
        assert!(!r.is_stuck(4));
    }

    #[test]
    fn creation_club_content_cannot_be_dragged_anywhere() {
        // The engine loads these from the .ccc file and Eidos deliberately keeps
        // them OUT of plugins.txt, so there is no position for a drag to record.
        // Treating them as ordinary rows accepted the drop, wrote the order out,
        // and let the next discovery quietly put them back: a move that looked
        // like it worked and never did.
        let mut l = PluginList {
            plugins: vec![
                p("Skyrim.esm", &[]),
                p("ccBGSSSE001-Fish.esm", &[]),
                p("Mod.esp", &[]),
                p("Other.esp", &[]),
            ],
            implicit: ["ccbgssse001-fish.esm".to_string()].into_iter().collect(),
            locked: Default::default(),
        };
        l.refresh(&se());
        let r = l.movable_range(1, &se()).unwrap();
        assert!(r.is_stuck(1));
        assert!(!l.can_move(1, true, &se()));
        assert!(!l.can_move(1, false, &se()));
        assert!(!l.move_plugins_to(&[1], 0, &se()));
        assert!(!l.move_plugins_to(&[1], 3, &se()));
        assert_eq!(names(&l), ["Skyrim.esm", "ccBGSSSE001-Fish.esm", "Mod.esp", "Other.esp"]);

        // Real mods next to it are still free to move around each other.
        assert!(l.movable_range(2, &se()).is_some_and(|r| !r.is_stuck(2)));
        assert!(l.move_plugins_to(&[2], 4, &se()));
        l.refresh(&se());
        assert_eq!(names(&l), ["Skyrim.esm", "ccBGSSSE001-Fish.esm", "Other.esp", "Mod.esp"]);
    }

    #[test]
    fn a_master_and_its_dependent_move_together_as_a_block() {
        // Judged one row at a time, the master's "must load before its
        // dependent" bound points at a row that is travelling with it, and the
        // whole legal move is refused. A block carries its relative order, so
        // ties inside it constrain nothing.
        let mut l = PluginList {
            plugins: vec![
                p("Base.esm", &[]),
                p("Pair1.esp", &[]),
                p("Pair2.esp", &["Pair1.esp"]),
                p("Free1.esp", &[]),
                p("Free2.esp", &[]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        assert_eq!(names(&l), ["Base.esm", "Pair1.esp", "Pair2.esp", "Free1.esp", "Free2.esp"]);

        // Pair1 alone cannot pass Pair2 - it is its master.
        assert_eq!(l.movable_range(1, &se()).unwrap().hi, 2);
        // The two together can go to the end.
        let r = l.block_movable_range(&[1, 2], &se()).unwrap();
        assert_eq!((r.lo, r.hi), (1, 5));
        assert!(l.move_plugins_to(&[1, 2], 5, &se()));
        l.refresh(&se());
        assert_eq!(names(&l), ["Base.esm", "Free1.esp", "Free2.esp", "Pair1.esp", "Pair2.esp"]);
    }

    #[test]
    fn a_block_range_still_answers_to_what_is_outside_it() {
        // Discounting internal ties must not discount external ones.
        let mut l = PluginList {
            plugins: vec![
                p("Base.esm", &[]),
                p("Anchor.esp", &[]),
                p("Dep.esp", &["Anchor.esp"]),
                p("Tail.esp", &["Dep.esp"]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        // Moving Anchor+Dep as a block: still bounded by Tail, which depends on
        // Dep and is NOT travelling.
        let r = l.block_movable_range(&[1, 2], &se()).unwrap();
        assert_eq!(r.hi, 3);
        assert_eq!(r.before.as_deref(), Some("Tail.esp"));
        assert!(!l.move_plugins_to(&[1, 2], 4, &se()));
        assert_eq!(names(&l), ["Base.esm", "Anchor.esp", "Dep.esp", "Tail.esp"]);
    }

    #[test]
    fn a_refused_move_leaves_the_pin_alone() {
        // The defect this pins: a move used to be made, repinned, and only THEN
        // undone by refresh's topological pass. The order came out right, so it
        // looked harmless - but the pin was left recording a slot the plugin can
        // never occupy, so it counted as violated forever and was re-applied on
        // every later refresh, shuffling its neighbours each time. Refusing the
        // move up front is what makes that unreachable.
        let mut l = PluginList {
            plugins: vec![
                p("Base.esp", &[]),
                p("Patch.esp", &["Base.esp"]),
                p("Other.esp", &[]),
            ],
            implicit: Default::default(),
            locked: Default::default(),
        };
        l.refresh(&se());
        assert!(l.set_locked(1, true));
        assert_eq!(l.locked.get("patch.esp"), Some(&1));

        // Try to drag Patch.esp above its own master, and to arrow it up.
        assert!(!l.move_plugins_to(&[1], 0, &se()));
        assert!(!l.move_plugin(1, true, &se()));

        // The pin still points at the slot the plugin actually holds, so nothing
        // is reported as violated and no later refresh will disturb the list.
        assert_eq!(l.locked.get("patch.esp"), Some(&1));
        l.refresh(&se());
        assert!(l.violated_locks().is_empty());
        assert_eq!(names(&l), ["Base.esp", "Patch.esp", "Other.esp"]);
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
        assert!(l.move_plugins_to(&[3], 0, &se()));
        l.refresh(&se());
        assert_eq!(l.locked.get("c.esp"), Some(&2));
        assert_eq!(names(&l), ["d.esp", "a.esp", "c.esp", "b.esp"]);
    }
}
