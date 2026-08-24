//! Mod categories, MO2-compatible. A mod's `meta.ini` stores `category=<primaryId>,
//! <otherIds>` - a comma-terminated list of integer ids, the first being the
//! "primary" category (MO2 `ModInfoRegular`). The id -> name mapping is NOT stored
//! per mod; it is resolved at display time through a catalog (MO2's
//! `CategoryFactory`), seeded from MO2's ~60 built-in defaults or an optional
//! per-instance `categories.dat`.
//!
//! The catalog is editable (MO2's Categories dialog). It lives in TWO files, as
//! MO2 splits it (`CategoryFactory::loadCategories`/`saveCategories`):
//!
//! * `categories.dat` - `id|name|parentID`. MO2 also READS a legacy 4-cell form
//!   (`id|name|nexusIDs|parentID`) but never writes it, so neither do we: a
//!   4-cell file written here would be silently rewritten to 3 cells the next
//!   time MO2 saved, taking the mappings with it.
//! * `nexuscatmap.dat` - `categoryID|nexusName|nexusID`, one line per REMOTE
//!   category. This is what turns a downloaded mod's Nexus category into a local
//!   one.

use std::collections::HashMap;
use std::path::Path;

/// MO2's built-in default categories, ported verbatim from
/// `CategoryFactory::loadDefaultCategories` (`(id, name, parent_id)`, parent 0 =
/// top-level). MO2 reuses id 39 (Voice, then Tattoos); the map is last-write-wins,
/// so the table order is reproduced exactly to match MO2's resolved names.
const DEFAULTS: &[(i32, &str, i32)] = &[
    (1, "Animations", 0),
    (52, "Poses", 1),
    (2, "Armour", 0),
    (53, "Power Armor", 2),
    (3, "Audio", 0),
    (38, "Music", 0),
    (39, "Voice", 0),
    (5, "Clothing", 0),
    (41, "Jewelry", 5),
    (42, "Backpacks", 5),
    (6, "Collectables", 0),
    (28, "Companions", 0),
    (7, "Creatures, Mounts, & Vehicles", 0),
    (8, "Factions", 0),
    (9, "Gameplay", 0),
    (27, "Combat", 9),
    (43, "Crafting", 9),
    (48, "Overhauls", 9),
    (49, "Perks", 9),
    (54, "Radio", 9),
    (55, "Shouts", 9),
    (22, "Skills & Levelling", 9),
    (58, "Weather & Lighting", 9),
    (44, "Equipment", 43),
    (45, "Home/Settlement", 43),
    (10, "Body, Face, & Hair", 0),
    (39, "Tattoos", 10),
    (40, "Character Presets", 0),
    (11, "Items", 0),
    (32, "Mercantile", 0),
    (37, "Ammo", 11),
    (19, "Weapons", 11),
    (36, "Weapon & Armour Sets", 11),
    (23, "Player Homes", 0),
    (25, "Castles & Mansions", 23),
    (51, "Settlements", 23),
    (12, "Locations", 0),
    (4, "Cities", 12),
    (31, "Landscape Changes", 0),
    (29, "Environment", 0),
    (30, "Immersion", 0),
    (20, "Magic", 0),
    (21, "Models & Textures", 0),
    (33, "Modders resources", 0),
    (13, "NPCs", 0),
    (24, "Bugfixes", 0),
    (14, "Patches", 24),
    (35, "Utilities", 0),
    (26, "Cheats", 0),
    (15, "Quests", 0),
    (16, "Races & Classes", 0),
    (34, "Stealth", 0),
    (17, "UI", 0),
    (18, "Visuals", 0),
    (50, "Pip-Boy", 18),
    (46, "Shader Presets", 0),
    (47, "Miscellaneous", 0),
];

/// One row of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// The MO2 category id, as it appears in a mod's `category=` list.
    pub id: i32,
    /// Display name.
    pub name: String,
    /// Parent id; `0` = top-level.
    pub parent: i32,
    /// The Nexus categories mapped onto this one, used to turn a downloaded
    /// mod's remote category into a local one. Empty for most rows.
    pub nexus: Vec<NexusCategory>,
}

/// A REMOTE (Nexus) category, as recorded in `nexuscatmap.dat`. The name is
/// whatever Nexus calls it; MO2 stores `Unknown` for entries recovered from the
/// legacy 4-cell `categories.dat`, and so do we.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusCategory {
    /// The Nexus category id, as it appears in a download's `.meta` `category=`.
    pub id: i32,
    /// The remote name.
    pub name: String,
}

/// The category catalog: maps a category id to its name, parent and Nexus ids.
///
/// Rows are kept in insertion order so a saved `categories.dat` reproduces the
/// order it was read in - MO2's own file is hand-ordered, and reshuffling it on
/// every save would make an Eidos/MO2 shared instance churn in git.
#[derive(Debug, Clone)]
pub struct CategoryFactory {
    rows: Vec<Category>,
    by_id: HashMap<i32, usize>,
}

impl Default for CategoryFactory {
    fn default() -> Self {
        Self::defaults()
    }
}

impl CategoryFactory {
    /// MO2's built-in default catalog.
    pub fn defaults() -> Self {
        let mut f = CategoryFactory { rows: Vec::new(), by_id: HashMap::new() };
        // Insertion order matters: MO2's id collisions resolve last-write-wins.
        for &(id, name, parent) in DEFAULTS {
            f.put(Category { id, name: name.to_string(), parent, nexus: Vec::new() });
        }
        f
    }

    /// Insert or overwrite a row, keeping `by_id` and the row order in step.
    fn put(&mut self, row: Category) {
        match self.by_id.get(&row.id) {
            Some(&at) => self.rows[at] = row,
            None => {
                self.by_id.insert(row.id, self.rows.len());
                self.rows.push(row);
            }
        }
    }

    /// Load the catalog from an instance root: `categories.dat` if present (MO2
    /// format), else the built-in defaults, plus `nexuscatmap.dat`'s remote
    /// mappings if that file is there.
    pub fn load(root: &Path) -> Self {
        let mut f = match std::fs::read_to_string(root.join(CATEGORIES_DAT)) {
            Ok(text) => Self::parse_dat(&text),
            Err(_) => Self::defaults(),
        };
        if let Ok(text) = std::fs::read_to_string(root.join(NEXUS_MAP_DAT)) {
            f.merge_nexus_map(&text);
        }
        f
    }

    /// Parse MO2's `categories.dat`: pipe-separated, either `id|name|parentID` (3
    /// cells, what MO2 writes) or the legacy `id|name|nexusIDs|parentID` (4 cells,
    /// which MO2 still reads - the ids it recovers get the name `Unknown`).
    fn parse_dat(text: &str) -> Self {
        let mut f = CategoryFactory { rows: Vec::new(), by_id: HashMap::new() };
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let cells: Vec<&str> = line.split('|').collect();
            let (id, name, nexus, parent) = match cells.as_slice() {
                [id, name, parent] => (id, name, "", parent),
                [id, name, nexus, parent] => (id, name, *nexus, parent),
                _ => continue,
            };
            if let (Ok(id), Ok(parent)) = (id.trim().parse::<i32>(), parent.trim().parse::<i32>()) {
                let nexus = parse_id_list(nexus)
                    .into_iter()
                    .map(|id| NexusCategory { id, name: "Unknown".to_string() })
                    .collect();
                f.put(Category { id, name: name.trim().to_string(), parent, nexus });
            }
        }
        if f.rows.is_empty() {
            Self::defaults()
        } else {
            f
        }
    }

    /// Merge `nexuscatmap.dat` (`categoryID|nexusName|nexusID`) onto the catalog.
    /// A mapping whose local category is not in the catalog is DROPPED, matching
    /// MO2's `resolveNexusID`, which checks the id map before answering.
    fn merge_nexus_map(&mut self, text: &str) {
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let [cat, name, nexus] = line.split('|').collect::<Vec<_>>()[..] else { continue };
            let (Ok(cat), Ok(nexus)) = (cat.trim().parse::<i32>(), nexus.trim().parse::<i32>())
            else {
                continue;
            };
            // A nexus id maps to exactly ONE local category (MO2's map is keyed
            // by nexus id), so re-pointing it means clearing it elsewhere first.
            for row in &mut self.rows {
                row.nexus.retain(|n| n.id != nexus);
            }
            if let Some(&at) = self.by_id.get(&cat) {
                self.rows[at].nexus.push(NexusCategory { id: nexus, name: name.trim().to_string() });
            }
        }
    }

    /// Serialise to MO2's `categories.dat` text (the 3-cell form MO2 writes).
    pub fn to_dat(&self) -> String {
        let mut out = String::new();
        for c in &self.rows {
            // MO2 skips id 0 ("None"), its no-parent sentinel, on save.
            if c.id == 0 {
                continue;
            }
            out.push_str(&format!("{}|{}|{}\n", c.id, c.name, c.parent));
        }
        out
    }

    /// Serialise the remote mappings to MO2's `nexuscatmap.dat` text.
    pub fn to_nexus_map(&self) -> String {
        let mut out = String::new();
        for c in &self.rows {
            for n in &c.nexus {
                out.push_str(&format!("{}|{}|{}\n", c.id, n.name, n.id));
            }
        }
        out
    }

    /// Write both catalog files into an instance root, atomically (tmp + rename):
    /// `categories.dat` is the only place the id -> name mapping lives, and a torn
    /// write would leave every mod in the instance showing a bare number.
    pub fn save(&self, root: &Path) -> std::io::Result<()> {
        write_atomic(&root.join(CATEGORIES_DAT), &self.to_dat())?;
        // Only touch the mapping file when there is something to say, so an
        // instance that never used Nexus categories does not grow an empty file
        // MO2 would then read as "mappings loaded, none of them".
        let map = self.to_nexus_map();
        if !map.is_empty() || root.join(NEXUS_MAP_DAT).exists() {
            write_atomic(&root.join(NEXUS_MAP_DAT), &map)?;
        }
        Ok(())
    }

    /// Every row, in file order.
    pub fn all(&self) -> &[Category] {
        &self.rows
    }

    /// The row for an id, if known.
    pub fn get(&self, id: i32) -> Option<&Category> {
        self.by_id.get(&id).map(|&at| &self.rows[at])
    }

    /// The display name for a category id, if known.
    pub fn name_for_id(&self, id: i32) -> Option<&str> {
        self.get(id).map(|c| c.name.as_str())
    }

    /// The parent id of a category (0 = top-level), if known.
    pub fn parent_id(&self, id: i32) -> Option<i32> {
        self.get(id).map(|c| c.parent)
    }

    /// Top-level categories (parent 0), sorted by name - for the filter dropdown.
    pub fn all_top_level(&self) -> Vec<(i32, &str)> {
        let mut v: Vec<(i32, &str)> = self
            .rows
            .iter()
            .filter(|c| c.parent == 0)
            .map(|c| (c.id, c.name.as_str()))
            .collect();
        v.sort_by(|a, b| a.1.cmp(b.1));
        v
    }

    /// Every category as `(id, indented name, depth)`, parents before children,
    /// each sibling group sorted by name - the shape a tree picker needs.
    ///
    /// Rows whose parent is missing from the catalog are emitted at top level
    /// rather than dropped: a `categories.dat` hand-edited to delete a parent
    /// would otherwise make its children invisible AND unassignable, with no way
    /// back short of editing the file again.
    pub fn tree(&self) -> Vec<(i32, String, usize)> {
        let known = |p: i32| p != 0 && self.by_id.contains_key(&p);
        let mut out = Vec::new();
        let mut roots: Vec<&Category> = self.rows.iter().filter(|c| !known(c.parent)).collect();
        roots.sort_by(|a, b| a.name.cmp(&b.name));
        for r in roots {
            self.walk(r.id, 0, &mut out);
        }
        out
    }

    fn walk(&self, id: i32, depth: usize, out: &mut Vec<(i32, String, usize)>) {
        // Depth guard: a cycle in a hand-edited categories.dat would otherwise
        // recurse until the stack runs out.
        if depth > 16 || out.iter().any(|(seen, _, _)| *seen == id) {
            return;
        }
        if let Some(c) = self.get(id) {
            out.push((id, c.name.clone(), depth));
        }
        let mut kids: Vec<&Category> = self.rows.iter().filter(|c| c.parent == id && c.id != id).collect();
        kids.sort_by(|a, b| a.name.cmp(&b.name));
        for k in kids {
            self.walk(k.id, depth + 1, out);
        }
    }

    /// Whether `child` is `ancestor` or descends from it (MO2's `isDescendantOf`,
    /// used so a filter on a parent category also matches its children).
    pub fn is_descendant_of(&self, child: i32, ancestor: i32) -> bool {
        let mut cur = child;
        for _ in 0..32 {
            // depth guard against a malformed cycle
            if cur == ancestor {
                return true;
            }
            match self.parent_id(cur) {
                Some(p) if p != 0 && p != cur => cur = p,
                _ => return cur == ancestor,
            }
        }
        false
    }

    /// The local category a Nexus category id maps to (MO2's `resolveNexusID`).
    pub fn for_nexus_id(&self, nexus_id: i32) -> Option<i32> {
        self.rows.iter().find(|c| c.nexus.iter().any(|n| n.id == nexus_id)).map(|c| c.id)
    }

    /// Point a Nexus category id at a local category, returning true if that
    /// changed anything. Re-points rather than duplicating: one remote id maps to
    /// exactly one local category, as in MO2's nexus-id-keyed map.
    pub fn learn_nexus_id(&mut self, local: i32, nexus_id: i32, name: &str) -> bool {
        if self.for_nexus_id(nexus_id) == Some(local) || !self.by_id.contains_key(&local) {
            return false;
        }
        for row in &mut self.rows {
            row.nexus.retain(|n| n.id != nexus_id);
        }
        let at = self.by_id[&local];
        let name = if name.trim().is_empty() { "Unknown" } else { name.trim() };
        self.rows[at].nexus.push(NexusCategory { id: nexus_id, name: name.to_string() });
        true
    }

    /// The next id for a new category: one past the highest in the catalog.
    ///
    /// MONOTONIC, never the lowest free one. Mods reference categories by NUMBER
    /// in `meta.ini`, and `remove` deliberately leaves those references alone -
    /// so handing a freed id back would make the next category the user creates
    /// silently adopt every mod that pointed at the deleted one. On the MO2
    /// defaults, ids 1..55 are all taken, so "lowest free" meant that deleting
    /// ANY built-in category made its id the very next one handed out. MO2 avoids
    /// this the same way, with an `m_NextID` seeded from the maximum at load.
    ///
    /// Derived from the persisted rows rather than stored, so an instance shared
    /// with MO2 cannot drift.
    pub fn free_id(&self) -> i32 {
        let max = self.rows.iter().map(|c| c.id).max().unwrap_or(0).max(0);
        // Only if a hand-edited file has already reached i32::MAX.
        max.checked_add(1).unwrap_or_else(|| (1..).find(|id| !self.by_id.contains_key(id)).unwrap_or(1))
    }

    /// Add a category, returning its id. Reuses a free id; the name is trimmed and
    /// pipe characters are stripped, since `|` is the field separator and one
    /// inside a name would split the row on the next load.
    pub fn add(&mut self, name: &str, parent: i32) -> i32 {
        let id = self.free_id();
        // A row that is its own parent is neither a root (its parent is known)
        // nor a child of anything (`walk` excludes a row from its own children),
        // so `tree()` never emits it: it cannot be seen, assigned, renamed or
        // deleted, and it survives a save/load round trip invisible.
        // `set_parent` already refuses this; `add` has to as well, and it is
        // reachable whenever the parent dropdown still names a category that was
        // deleted in the same dialog session.
        let parent = if parent == id { 0 } else { parent };
        self.put(Category { id, name: clean_name(name), parent, nexus: Vec::new() });
        id
    }

    /// Rename a category. Returns false if the id is unknown.
    pub fn rename(&mut self, id: i32, name: &str) -> bool {
        match self.by_id.get(&id) {
            Some(&at) => {
                self.rows[at].name = clean_name(name);
                true
            }
            None => false,
        }
    }

    /// Re-parent a category. Refuses to make it its own descendant (which would
    /// orphan the whole branch from `tree()` and loop `is_descendant_of`).
    pub fn set_parent(&mut self, id: i32, parent: i32) -> bool {
        if id == parent || (parent != 0 && self.is_descendant_of(parent, id)) {
            return false;
        }
        match self.by_id.get(&id) {
            Some(&at) => {
                self.rows[at].parent = parent;
                true
            }
            None => false,
        }
    }

    /// Delete a category, re-parenting its children onto its own parent so the
    /// branch stays reachable. Returns false if the id is unknown.
    ///
    /// Mods still referencing the id are NOT touched here: the caller decides
    /// whether to rewrite their `meta.ini`, and doing it silently would rewrite
    /// every mod in the instance behind a single click in a settings dialog.
    pub fn remove(&mut self, id: i32) -> bool {
        let Some(&at) = self.by_id.get(&id) else { return false };
        let parent = self.rows[at].parent;
        self.rows.remove(at);
        for c in &mut self.rows {
            if c.parent == id {
                c.parent = parent;
            }
        }
        self.reindex();
        true
    }

    fn reindex(&mut self) {
        self.by_id = self.rows.iter().enumerate().map(|(at, c)| (c.id, at)).collect();
    }
}

/// MO2's catalog file name, in the instance root.
pub const CATEGORIES_DAT: &str = "categories.dat";
/// MO2's remote-category mapping file, alongside it.
pub const NEXUS_MAP_DAT: &str = "nexuscatmap.dat";

fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("dat.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// Strip the field separator and surrounding space from a category name. A `|` in
/// a name would split the row into extra cells on the next parse, turning one
/// category into a malformed line that is skipped entirely.
fn clean_name(name: &str) -> String {
    name.replace('|', "/").trim().to_string()
}

/// Parse a comma-separated id list, ignoring blanks and junk.
fn parse_id_list(raw: &str) -> Vec<i32> {
    raw.split(',').filter_map(|s| s.trim().parse::<i32>().ok()).collect()
}

/// Every category id in a raw `meta.ini` `category=` value, primary first, in
/// file order. The `-1` placeholder and non-positive ids are dropped.
pub fn parse_all(raw: &str) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for id in parse_id_list(raw) {
        if id > 0 && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Render a category list into MO2's on-disk form: primary first, then the rest,
/// comma-separated with a trailing comma. `None` yields the uncategorised
/// placeholder `-1,`, which is what MO2 writes for a mod with no category.
pub fn format_categories(primary: Option<i32>, others: &[i32]) -> String {
    let mut ids: Vec<i32> = Vec::new();
    if let Some(p) = primary.filter(|&p| p > 0) {
        ids.push(p);
    }
    for &id in others {
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.is_empty() {
        return "-1,".to_string();
    }
    let mut out = String::new();
    for id in ids {
        out.push_str(&id.to_string());
        out.push(',');
    }
    out
}

/// A mod's PRIMARY category id from a raw `meta.ini` `category=` value: the first
/// positive id in the comma-terminated list (MO2 `ModInfoRegular` load). `-1,` (the
/// uncategorised placeholder) and empty yield `None`.
pub fn parse_primary(raw: &str) -> Option<i32> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<i32>().ok())
        .find(|&id| id > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_default_category_names() {
        let f = CategoryFactory::defaults();
        assert_eq!(f.name_for_id(9), Some("Gameplay"));
        assert_eq!(f.name_for_id(17), Some("UI"));
        assert_eq!(f.name_for_id(99999), None);
        // MO2's id-39 collision: the later "Tattoos" wins (parent 10).
        assert_eq!(f.name_for_id(39), Some("Tattoos"));
        assert_eq!(f.parent_id(39), Some(10));
    }

    #[test]
    fn parse_primary_takes_first_positive_id() {
        assert_eq!(parse_primary("-1,"), None); // uncategorised placeholder
        assert_eq!(parse_primary(""), None);
        assert_eq!(parse_primary("9,"), Some(9));
        assert_eq!(parse_primary("27,43,9"), Some(27)); // first wins
        assert_eq!(parse_primary("-1,9"), Some(9)); // skip the invalid -1
    }

    #[test]
    fn descendant_and_top_level() {
        let f = CategoryFactory::defaults();
        assert!(f.is_descendant_of(27, 9)); // Combat is under Gameplay
        assert!(f.is_descendant_of(9, 9)); // a category is its own descendant
        assert!(!f.is_descendant_of(9, 27)); // not the other way
        let top = f.all_top_level();
        assert!(top.iter().any(|&(id, n)| id == 9 && n == "Gameplay"));
        assert!(top.iter().all(|&(id, _)| f.parent_id(id) == Some(0)));
    }

    #[test]
    fn parse_dat_overrides_defaults() {
        let f = CategoryFactory::parse_dat("1|My Anims|0\n900|Custom|0\n");
        assert_eq!(f.name_for_id(1), Some("My Anims"));
        assert_eq!(f.name_for_id(900), Some("Custom"));
    }

    #[test]
    fn parse_all_keeps_order_and_drops_the_placeholder() {
        assert_eq!(parse_all("-1,"), Vec::<i32>::new());
        assert_eq!(parse_all("27,43,9,"), vec![27, 43, 9]);
        assert_eq!(parse_all("9,9,27,"), vec![9, 27]); // deduped
        assert_eq!(parse_all(""), Vec::<i32>::new());
    }

    #[test]
    fn format_categories_matches_what_mo2_writes() {
        assert_eq!(format_categories(None, &[]), "-1,");
        assert_eq!(format_categories(Some(9), &[]), "9,");
        assert_eq!(format_categories(Some(9), &[27, 43]), "9,27,43,");
        // The primary is never repeated in the tail.
        assert_eq!(format_categories(Some(9), &[9, 27]), "9,27,");
        // A primary that is not a real id falls through to the others.
        assert_eq!(format_categories(Some(-1), &[27]), "27,");
    }

    #[test]
    fn legacy_four_cell_dat_is_read_and_written_back_as_three() {
        // MO2 reads the 4-cell form but only ever WRITES 3 cells, moving the
        // mappings into nexuscatmap.dat. Writing 4 cells here would be silently
        // rewritten to 3 by the next MO2 save, taking the mappings with it.
        let f = CategoryFactory::parse_dat("1|Animations|22,51|0\n900|Custom||0\n52|Poses||1\n");
        assert_eq!(f.for_nexus_id(51), Some(1));
        assert_eq!(f.for_nexus_id(999), None);
        // Recovered ids have no remote name, exactly as MO2 records them.
        assert_eq!(f.get(1).unwrap().nexus[0].name, "Unknown");
        assert_eq!(f.to_dat(), "1|Animations|0\n900|Custom|0\n52|Poses|1\n");
        assert_eq!(f.to_nexus_map(), "1|Unknown|22\n1|Unknown|51\n");
    }

    #[test]
    fn three_cell_dat_round_trips_byte_for_byte() {
        let text = "1|Anims|0\n52|Poses|1\n";
        let f = CategoryFactory::parse_dat(text);
        assert!(f.get(1).unwrap().nexus.is_empty());
        assert_eq!(f.to_dat(), text);
        assert_eq!(f.to_nexus_map(), "");
    }

    #[test]
    fn nexus_map_file_round_trips_and_repoints_instead_of_duplicating() {
        let mut f = CategoryFactory::parse_dat("1|Anims|0\n9|Gameplay|0\n");
        f.merge_nexus_map("1|Animation|51\n9|Gameplay|24\n");
        assert_eq!(f.for_nexus_id(51), Some(1));
        assert_eq!(f.for_nexus_id(24), Some(9));
        assert_eq!(f.to_nexus_map(), "1|Animation|51\n9|Gameplay|24\n");

        // Re-pointing 51 onto another category must not leave it on both.
        assert!(f.learn_nexus_id(9, 51, "Animation"));
        assert_eq!(f.for_nexus_id(51), Some(9));
        assert!(f.get(1).unwrap().nexus.is_empty());
        // Idempotent, and a mapping onto an unknown category is refused.
        assert!(!f.learn_nexus_id(9, 51, "Animation"));
        assert!(!f.learn_nexus_id(4242, 77, "Nope"));
        assert_eq!(f.for_nexus_id(77), None);

        // A mapping whose local category is gone is dropped, as MO2 drops it.
        let mut g = CategoryFactory::parse_dat("1|Anims|0\n");
        g.merge_nexus_map("777|Ghost|51\n");
        assert_eq!(g.for_nexus_id(51), None);
        assert_eq!(g.to_nexus_map(), "");
    }

    #[test]
    fn both_catalog_files_survive_a_disk_round_trip() {
        let dir = std::env::temp_dir().join(format!("eidos-cats-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut f = CategoryFactory::parse_dat("1|Anims|0\n9|Gameplay|0\n");
        f.learn_nexus_id(1, 51, "Animation");
        let id = f.add("Mine", 9);
        f.save(&dir).unwrap();

        let back = CategoryFactory::load(&dir);
        assert_eq!(back.name_for_id(id), Some("Mine"));
        assert_eq!(back.parent_id(id), Some(9));
        assert_eq!(back.for_nexus_id(51), Some(1));
        assert_eq!(back.to_dat(), f.to_dat());
        assert_eq!(back.to_nexus_map(), f.to_nexus_map());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_instance_with_no_dat_gets_mo2s_defaults() {
        let empty = std::env::temp_dir().join("eidos-cats-nothing-here");
        let f = CategoryFactory::load(&empty);
        assert_eq!(f.name_for_id(9), Some("Gameplay"));
    }

    #[test]
    fn add_rename_and_reparent() {
        let mut f = CategoryFactory::parse_dat("1|Anims|0\n");
        let id = f.add("Mine", 1);
        assert_eq!(f.name_for_id(id), Some("Mine"));
        assert_eq!(f.parent_id(id), Some(1));
        assert!(f.rename(id, "  Renamed  "));
        assert_eq!(f.name_for_id(id), Some("Renamed"));
        assert!(!f.rename(4242, "nope"));
        // A pipe in a name would split the row on the next parse.
        f.rename(id, "a|b");
        assert_eq!(f.name_for_id(id), Some("a/b"));
        assert_eq!(CategoryFactory::parse_dat(&f.to_dat()).name_for_id(id), Some("a/b"));
    }

    #[test]
    fn reparenting_refuses_to_build_a_cycle() {
        let mut f = CategoryFactory::parse_dat("1|Top|0\n2|Mid|1\n3|Low|2\n");
        assert!(!f.set_parent(1, 1)); // itself
        assert!(!f.set_parent(1, 3)); // under its own grandchild
        assert!(f.set_parent(3, 1)); // legal move
        assert_eq!(f.parent_id(3), Some(1));
    }

    #[test]
    fn removing_a_category_lifts_its_children_instead_of_orphaning_them() {
        let mut f = CategoryFactory::parse_dat("1|Top|0\n2|Mid|1\n3|Low|2\n");
        assert!(f.remove(2));
        assert_eq!(f.name_for_id(2), None);
        assert_eq!(f.parent_id(3), Some(1));
        assert!(!f.remove(2)); // already gone
        // The index kept up with the removal.
        assert_eq!(f.name_for_id(3), Some("Low"));
    }

    #[test]
    fn free_id_never_returns_a_taken_one_or_a_sentinel() {
        let mut f = CategoryFactory::parse_dat("1|A|0\n2|B|0\n");
        assert_eq!(f.free_id(), 3);
        let a = f.add("C", 0);
        let b = f.add("D", 0);
        assert_ne!(a, b);
        assert!(a > 0 && b > 0);
    }

    #[test]
    fn a_deleted_id_is_never_handed_out_again() {
        // The mods still on disk reference the deleted category by NUMBER, and
        // `remove` deliberately leaves them alone - so reusing the id would make
        // the next category the user creates adopt all of them.
        let mut f = CategoryFactory::defaults();
        assert!(f.remove(47), "Miscellaneous");
        let fresh = f.add("Mounts", 0);
        assert_ne!(fresh, 47, "a new category must not inherit the deleted one's mods");
        assert_eq!(f.name_for_id(47), None, "and 47 stays unresolvable, as documented");

        // Holds after a round trip too: the high-water mark is derived from the
        // rows, so it cannot go backwards when the file is re-read.
        let back = CategoryFactory::parse_dat(&f.to_dat());
        assert!(back.free_id() > fresh);
    }

    #[test]
    fn a_category_cannot_be_created_as_its_own_parent() {
        // Reachable from the editor: the parent dropdown still names a category
        // deleted earlier in the same session, and the id allocator could hand
        // that same number back. Such a row is emitted by neither branch of
        // `tree()` and would be invisible forever.
        let mut f = CategoryFactory::parse_dat("1|A|0\n");
        let id = f.add("Self", 2); // 2 is what free_id will return
        assert_eq!(id, 2);
        assert_eq!(f.parent_id(id), Some(0), "silently re-rooted rather than orphaned");
        assert!(f.tree().iter().any(|&(i, _, _)| i == id), "and it is visible");
    }

    #[test]
    fn tree_is_parents_before_children_and_survives_a_missing_parent() {
        let f = CategoryFactory::parse_dat("1|Top|0\n2|Mid|1\n3|Orphan|777\n");
        let t = f.tree();
        let pos = |id: i32| t.iter().position(|&(i, _, _)| i == id).unwrap();
        assert!(pos(1) < pos(2));
        assert_eq!(t[pos(2)].2, 1); // depth
        // The parent 777 does not exist: the row is shown at top level, not lost.
        assert_eq!(t[pos(3)].2, 0);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn tree_terminates_on_a_hand_edited_cycle() {
        let f = CategoryFactory::parse_dat("1|A|2\n2|B|1\n");
        // Both rows have a known parent, so neither is a root - the walk must
        // still stop rather than recurse, and the tree may legitimately be empty.
        let t = f.tree();
        assert!(t.len() <= 2);
    }
}
