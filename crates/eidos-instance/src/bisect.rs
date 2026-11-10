//! Which mods a variant test run (A36) or a bisect step (A37) needs.

use std::collections::{HashMap, HashSet};

/// `chosen` plus, until nothing changes, the mod providing each master of a
/// plugin from a mod already in the set (plugin names compared without case),
/// all limited to `universe` and returned in `universe` order. A master no mod
/// of `universe` provides (the game's own, the Overwrite's) adds nothing.
pub fn with_masters(
    chosen: &[String],
    universe: &[String],
    plugins: &[eidos_plugins::Plugin],
) -> Vec<String> {
    let in_universe = |m: &str| universe.iter().any(|u| u == m);
    let provider: HashMap<String, &str> = plugins
        .iter()
        .map(|p| (p.name.to_ascii_lowercase(), p.origin_mod.as_str()))
        .collect();
    let mut set: HashSet<&str> = chosen
        .iter()
        .map(String::as_str)
        .filter(|m| in_universe(m))
        .collect();
    loop {
        let added: Vec<&str> = plugins
            .iter()
            .filter(|p| set.contains(p.origin_mod.as_str()))
            .flat_map(|p| &p.masters)
            .filter_map(|m| provider.get(&m.to_ascii_lowercase()).copied())
            .filter(|m| !set.contains(m) && in_universe(m))
            .collect();
        if added.is_empty() {
            break;
        }
        set.extend(added);
    }
    universe
        .iter()
        .filter(|u| set.contains(u.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The fields `with_masters` does not read, filled as eidos-gui's
    /// `test_support::plugin_row` fills them.
    fn plugin(name: &str, origin: &str, masters: &[&str]) -> eidos_plugins::Plugin {
        eidos_plugins::Plugin {
            name: name.into(),
            origin_mod: origin.into(),
            masters: masters.iter().map(|m| m.to_string()).collect(),
            path: PathBuf::new(),
            enabled: true,
            force_disabled: false,
            header_error: None,
            form_version: Some(44),
            author: None,
            description: None,
            header_version: None,
            is_blueprint: false,
            is_update: false,
            is_master: false,
            is_light: false,
            is_medium: false,
            is_dummy: false,
            priority: 0,
            index: None,
            ini: None,
        }
    }

    #[test]
    fn with_masters_adds_providers_in_profile_order() {
        let plugins = [
            plugin("Base.esm", "Base", &[]),
            plugin("Mid.esp", "Mid", &["base.ESM"]),
            plugin("Patch.esp", "Patch", &["Mid.esp", "Skyrim.esm"]),
            plugin("Other.esp", "Other", &[]),
        ];
        let universe: Vec<String> = ["Base", "Other", "Mid", "Patch"].map(String::from).to_vec();
        assert_eq!(
            with_masters(&["Patch".into()], &universe, &plugins),
            ["Base", "Mid", "Patch"]
        );
        assert_eq!(with_masters(&[], &universe, &plugins), Vec::<String>::new());
    }
}
