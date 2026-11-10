//! A57: an installed collection's before/after rules that the mod list no longer keeps.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use eidos_collections::{driver, install, rules, state};

use super::{Finding, Level};

/// One installed collection's before/after rules, read from its cached recipe.
#[derive(Debug, Clone)]
pub struct CollectionOrder {
    pub slug: String,
    /// The recipe's `info.name`, the slug when empty.
    pub name: String,
    pub rules: rules::FolderRules,
}

/// The rules of every installed collection whose cached recipe is still there.
///
/// This parses and resolves each recipe, which for a collection of 2000 mods
/// takes far longer than a frame: a window keeps the answer until
/// [`orders_stamp`] changes.
pub fn collection_orders(root: &Path) -> Vec<CollectionOrder> {
    state::installed(root)
        .into_iter()
        .filter_map(|st| {
            let c = driver::cached_collection(root, &st.slug, st.revision)?;
            let rules = rules::FolderRules::new(&c, &install::installed_folders(&c, &st));
            let name = if c.info.name.is_empty() {
                st.slug.clone()
            } else {
                c.info.name
            };
            Some(CollectionOrder {
                slug: st.slug,
                name,
                rules,
            })
        })
        .collect()
}

/// The modification time of every file [`collection_orders`] reads: each
/// revision's state file, its recipe and the recipe's completion marker.
pub fn orders_stamp(root: &Path) -> Vec<(PathBuf, Option<SystemTime>)> {
    let dir = root.join("collections");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".state.json") else {
            continue;
        };
        for path in [
            entry.path(),
            dir.join(stem).join("collection.json"),
            dir.join(format!("{stem}.payload-complete")),
        ] {
            let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            out.push((path, modified));
        }
    }
    out.sort();
    out
}

/// One `collection_order_broken` finding per collection in `orders` whose
/// rules the enabled mods break. `mods` is highest priority first, as
/// [`super::active_mods`] gives it.
pub fn order_findings(orders: &[CollectionOrder], mods: &[(String, PathBuf)]) -> Vec<Finding> {
    let enabled: Vec<String> = mods.iter().rev().map(|(n, _)| n.clone()).collect();
    let mut out = Vec::new();
    for o in orders {
        let pairs = o.rules.broken_pairs(&enabled);
        let Some((earlier, later)) = pairs.first().cloned() else {
            continue;
        };
        out.push(Finding {
            code: "collection_order_broken",
            level: Level::Advice,
            args: vec![
                ("collection", o.name.clone()),
                ("slug", o.slug.clone()),
                ("n", pairs.len().to_string()),
                ("earlier", earlier),
                ("later", later),
            ],
        });
    }
    out
}

/// [`order_findings`] over [`collection_orders`]. A collection without its
/// cached recipe is not checked.
pub fn collection_order_findings(root: &Path, mods: &[(String, PathBuf)]) -> Vec<Finding> {
    order_findings(&collection_orders(root), mods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eidos_collections::state::{self, InstallState, Status};

    fn write_ruled(root: &Path, slug: &str, members: &[(&str, u64, u64)], before: &[(&str, &str)]) {
        let mods: Vec<serde_json::Value> = members
            .iter()
            .map(|(n, m, f)| {
                serde_json::json!({"name": n, "source": {"type": "nexus", "modId": m, "fileId": f}})
            })
            .collect();
        let rules: Vec<serde_json::Value> = before
            .iter()
            .map(|(a, b)| {
                serde_json::json!({"source": {"fileExpression": a}, "type": "before", "reference": {"fileExpression": b}})
            })
            .collect();
        let text = serde_json::json!({
            "info": {"name": "Cities", "domainName": "skyrimspecialedition"},
            "mods": mods,
            "modRules": rules,
        })
        .to_string();
        let dir = state::revision_dir(root, slug, 1);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("collection.json"), &text).unwrap();
        let md5 = eidos_nexus::md5_file(&dir.join("collection.json")).unwrap();
        std::fs::write(dir.with_extension("payload-complete"), md5).unwrap();
        let mut st = InstallState {
            slug: slug.to_string(),
            revision: 1,
            game_domain: "skyrimspecialedition".to_string(),
            ..InstallState::default()
        };
        for m in &eidos_collections::read(&text).unwrap().collection.mods {
            st.set(
                &state::key_for(m, "skyrimspecialedition"),
                Status::Installed(m.name.clone()),
            );
        }
        st.save(&InstallState::path(root, slug, 1)).unwrap();
    }

    #[test]
    fn a_collection_whose_order_broke_gives_one_advice_finding() {
        let root =
            std::env::temp_dir().join(format!("eidos-collection-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_ruled(
            &root,
            "abc123",
            &[("A", 1, 10), ("B", 2, 20)],
            &[("A", "B")],
        );
        let mods = |names: &[&str]| {
            names
                .iter()
                .map(|n| (n.to_string(), root.join("mods").join(n)))
                .collect::<Vec<_>>()
        };
        assert!(
            collection_order_findings(&root, &mods(&["B", "A"])).is_empty(),
            "B wins, as the rule asks"
        );
        let f = collection_order_findings(&root, &mods(&["A", "B"]));
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].code, f[0].level),
            ("collection_order_broken", Level::Advice)
        );
        for (k, v) in [
            ("collection", "Cities"),
            ("slug", "abc123"),
            ("n", "1"),
            ("earlier", "A"),
            ("later", "B"),
        ] {
            assert_eq!(f[0].arg(k), Some(v), "{k}");
        }
        let stamp = orders_stamp(&root);
        assert_eq!(stamp.len(), 3, "{stamp:?}");
        assert!(stamp.iter().all(|(_, t)| t.is_some()), "{stamp:?}");
        std::fs::remove_file(
            eidos_collections::state::revision_dir(&root, "abc123", 1).join("collection.json"),
        )
        .unwrap();
        assert!(
            collection_order_findings(&root, &mods(&["A", "B"])).is_empty(),
            "no recipe, no check"
        );
        assert_ne!(orders_stamp(&root), stamp, "a window reads the rules again");
        let _ = std::fs::remove_dir_all(root);
    }
}
