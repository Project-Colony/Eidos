//! Mod categories, MO2-compatible. A mod's `meta.ini` stores `category=<primaryId>,
//! <otherIds>` - a comma-terminated list of integer ids, the first being the
//! "primary" category (MO2 `ModInfoRegular`). The id -> name mapping is NOT stored
//! per mod; it is resolved at display time through a catalog (MO2's
//! `CategoryFactory`), seeded from MO2's ~60 built-in defaults or an optional
//! per-instance `categories.dat`.
//!
//! Eidos reads this catalog READ-ONLY: it never rewrites a mod's `category=` value,
//! so an MO2 instance's `meta.ini` round-trips byte-for-byte.

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

/// The category catalog: maps a category id to its name + parent.
#[derive(Debug, Clone)]
pub struct CategoryFactory {
    by_id: HashMap<i32, (String, i32)>,
}

impl Default for CategoryFactory {
    fn default() -> Self {
        Self::defaults()
    }
}

impl CategoryFactory {
    /// MO2's built-in default catalog.
    pub fn defaults() -> Self {
        let mut by_id = HashMap::new();
        // Insertion order matters: MO2's id collisions resolve last-write-wins.
        for &(id, name, parent) in DEFAULTS {
            by_id.insert(id, (name.to_string(), parent));
        }
        CategoryFactory { by_id }
    }

    /// Load the catalog: a per-instance `categories.dat` if present (MO2 format),
    /// else the built-in defaults.
    pub fn load(categories_dat: &Path) -> Self {
        match std::fs::read_to_string(categories_dat) {
            Ok(text) => Self::parse_dat(&text),
            Err(_) => Self::defaults(),
        }
    }

    /// Parse MO2's `categories.dat`: pipe-separated, either `id|name|parentID` (3
    /// cells) or `id|name|nexusIDs|parentID` (4 cells, the nexus column ignored).
    fn parse_dat(text: &str) -> Self {
        let mut by_id = HashMap::new();
        for line in text.lines() {
            let cells: Vec<&str> = line.split('|').collect();
            let (id, name, parent) = match cells.as_slice() {
                [id, name, parent] => (id, name, parent),
                [id, name, _nexus, parent] => (id, name, parent),
                _ => continue,
            };
            if let (Ok(id), Ok(parent)) = (id.trim().parse::<i32>(), parent.trim().parse::<i32>()) {
                by_id.insert(id, (name.trim().to_string(), parent));
            }
        }
        if by_id.is_empty() {
            Self::defaults()
        } else {
            CategoryFactory { by_id }
        }
    }

    /// The display name for a category id, if known.
    pub fn name_for_id(&self, id: i32) -> Option<&str> {
        self.by_id.get(&id).map(|(n, _)| n.as_str())
    }

    /// The parent id of a category (0 = top-level), if known.
    pub fn parent_id(&self, id: i32) -> Option<i32> {
        self.by_id.get(&id).map(|(_, p)| *p)
    }

    /// Top-level categories (parent 0), sorted by name - for the filter dropdown.
    pub fn all_top_level(&self) -> Vec<(i32, &str)> {
        let mut v: Vec<(i32, &str)> = self
            .by_id
            .iter()
            .filter(|(_, (_, parent))| *parent == 0)
            .map(|(id, (name, _))| (*id, name.as_str()))
            .collect();
        v.sort_by(|a, b| a.1.cmp(b.1));
        v
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
}
