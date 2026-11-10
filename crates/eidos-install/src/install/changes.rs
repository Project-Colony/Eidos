//! What a Replace changes in a mod folder: the old tree against the staged one,
//! compared by name and size, plus the plugins' headers. A same-size rewrite of
//! a non-plugin file is not listed.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use eidos_instance::UpdateReport;
use eidos_plugins::{GameSpec, HeaderFacts};

use crate::MAX_TREE_DEPTH;

/// The kind a plugin's header gives it, as the report spells it.
pub fn plugin_kind(h: &HeaderFacts) -> &'static str {
    match (h.is_master, h.is_light, h.is_medium) {
        (true, true, _) => "ESM+ESL",
        (_, true, _) => "ESL",
        (_, _, true) => "ESH",
        (true, _, _) => "ESM",
        _ => "ESP",
    }
}

/// Lowercase relative path to (spelling, size), without the manager's own
/// top-level files.
fn walk(root: &Path) -> BTreeMap<String, (String, u64)> {
    fn rec(dir: &Path, prefix: &str, depth: usize, out: &mut BTreeMap<String, (String, u64)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            // DirEntry::metadata does not follow a symlink, so a linked
            // directory is one entry and is never walked into.
            let Ok(meta) = entry.metadata() else { continue };
            let name = entry.file_name().to_string_lossy().into_owned();
            let lower = name.to_ascii_lowercase();
            if depth == 0
                && (lower.starts_with(".eidos")
                    || lower == "meta.ini" && !meta.is_dir()
                    || lower == "fomod" && meta.is_dir())
            {
                continue;
            }
            let rel = format!("{prefix}{name}");
            if meta.is_dir() {
                if depth < MAX_TREE_DEPTH {
                    rec(&entry.path(), &format!("{rel}/"), depth + 1, out);
                }
            } else {
                out.insert(rel.to_ascii_lowercase(), (rel, meta.len()));
            }
        }
    }
    let mut out = BTreeMap::new();
    rec(root, "", 0, &mut out);
    out
}

/// Top-level plugins: lowercase name to (spelling, header if readable).
type Plugins = BTreeMap<String, (String, Option<HeaderFacts>)>;

fn plugins(root: &Path, files: &BTreeMap<String, (String, u64)>, spec: &GameSpec) -> Plugins {
    files
        .iter()
        .filter(|(key, _)| !key.contains('/') && eidos_plugins::is_plugin(key))
        .map(|(key, (name, _))| {
            let header = eidos_plugins::header_facts(&root.join(name), spec).ok();
            (key.clone(), (name.clone(), header))
        })
        .collect()
}

/// Masters the plugins name that the tree does not ship itself: lowercase to the
/// spelling of the first plugin naming it.
fn outside_masters(plugins: &Plugins) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (_, header) in plugins.values() {
        for master in header.iter().flat_map(|h| &h.masters) {
            out.entry(master.to_ascii_lowercase())
                .or_insert_with(|| master.clone());
        }
    }
    out.retain(|key, _| !plugins.contains_key(key));
    out
}

/// Keys of `a` missing from `b`, spelled as in `a`.
fn only_in<V>(
    a: &BTreeMap<String, V>,
    b: &BTreeMap<String, V>,
    spell: impl Fn(&V) -> &String,
) -> Vec<String> {
    a.iter()
        .filter(|(key, _)| !b.contains_key(*key))
        .map(|(_, v)| spell(v).clone())
        .collect()
}

/// The old mod folder against the new one. `when` and the versions stay empty.
pub fn compare_trees(old: &Path, new: &Path, game_id: &str) -> UpdateReport {
    compare_walked(walk(old), old, new, game_id)
}

/// [`compare_trees`] for a Replace, or `None` when the old folder holds only the
/// manager's own files (a collection's reservation, a new empty mod): that is a
/// first install, not an update.
pub(crate) fn compare_update(old: &Path, new: &Path, game_id: &str) -> Option<UpdateReport> {
    let old_files = walk(old);
    (!old_files.is_empty()).then(|| compare_walked(old_files, old, new, game_id))
}

fn compare_walked(
    old_files: BTreeMap<String, (String, u64)>,
    old: &Path,
    new: &Path,
    game_id: &str,
) -> UpdateReport {
    let new_files = walk(new);
    let mut report = UpdateReport {
        added: only_in(&new_files, &old_files, |(n, _)| n),
        removed: only_in(&old_files, &new_files, |(n, _)| n),
        resized: new_files
            .iter()
            .filter(|(key, (_, size))| old_files.get(*key).is_some_and(|(_, s)| s != size))
            .map(|(_, (name, _))| name.clone())
            .collect(),
        ..UpdateReport::default()
    };
    let Some(spec) = GameSpec::for_id(game_id) else {
        return report;
    };
    let (old_plugins, new_plugins) = (
        plugins(old, &old_files, &spec),
        plugins(new, &new_files, &spec),
    );
    report.plugins_added = only_in(&new_plugins, &old_plugins, |(n, _)| n);
    report.plugins_removed = only_in(&old_plugins, &new_plugins, |(n, _)| n);
    report.kinds_changed = new_plugins
        .iter()
        .filter_map(|(key, (name, after))| {
            let before = plugin_kind(old_plugins.get(key)?.1.as_ref()?);
            let after = plugin_kind(after.as_ref()?);
            (before != after).then(|| (name.clone(), before.to_string(), after.to_string()))
        })
        .collect();
    let (old_masters, new_masters) = (outside_masters(&old_plugins), outside_masters(&new_plugins));
    report.masters_added = only_in(&new_masters, &old_masters, |n| n);
    report.masters_removed = only_in(&old_masters, &new_masters, |n| n);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("eidos-changes-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A 24-byte TES4 header (form version 44) followed by one MAST per master.
    fn tes4(flags: u32, masters: &[&str]) -> Vec<u8> {
        let mut bytes = vec![0u8; 24];
        bytes[..4].copy_from_slice(b"TES4");
        bytes[8..12].copy_from_slice(&flags.to_le_bytes());
        bytes[20..22].copy_from_slice(&44u16.to_le_bytes());
        for master in masters {
            bytes.extend_from_slice(b"MAST");
            bytes.extend_from_slice(&(master.len() as u16 + 1).to_le_bytes());
            bytes.extend_from_slice(master.as_bytes());
            bytes.push(0);
        }
        let len = (bytes.len() - 24) as u32;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        bytes
    }

    #[test]
    fn compare_trees_lists_files_plugins_masters_and_kinds() {
        let dir = temp_dir("compare");
        let (old, new) = (dir.join("old"), dir.join("new"));
        let put = |root: &Path, rel: &str, bytes: &[u8]| {
            let p = root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, bytes).unwrap();
        };
        put(&old, "meta.ini", b"[General]\nversion=1.0\n");
        put(&new, "meta.ini", b"[General]\nversion=2.0\nmore=1\n");
        put(&old, "textures/a.dds", b"aaaa");
        put(&new, "Textures/A.dds", b"aaaa");
        put(&old, "meshes/b.nif", b"bb");
        put(&new, "meshes/b.nif", b"bbbbbb");
        put(&old, "scripts/gone.pex", b"x");
        put(&new, "sound/new.wav", b"x");
        put(&old, "Patch.esp", &tes4(0, &["Skyrim.esm"]));
        put(
            &new,
            "Patch.esp",
            &tes4(0x200, &["Skyrim.esm", "Dawnguard.esm"]),
        );
        put(&old, "Old.esp", &tes4(0, &["Update.esm"]));
        put(&new, "Extra.esm", &tes4(0x1, &[]));
        put(&new, "fomod/ModuleConfig.xml", b"x");
        put(&new, ".eidos-collection-recipe.json", b"{}");
        let r = compare_trees(&old, &new, "skyrimse");
        assert_eq!(r.added, ["Extra.esm", "sound/new.wav"]);
        assert_eq!(r.removed, ["Old.esp", "scripts/gone.pex"]);
        assert_eq!(r.resized, ["meshes/b.nif", "Patch.esp"]);
        assert_eq!(
            (r.plugins_added.as_slice(), r.plugins_removed.as_slice()),
            (&["Extra.esm".to_string()][..], &["Old.esp".to_string()][..])
        );
        assert_eq!(
            (r.masters_added.as_slice(), r.masters_removed.as_slice()),
            (
                &["Dawnguard.esm".to_string()][..],
                &["Update.esm".to_string()][..]
            )
        );
        assert_eq!(
            r.kinds_changed,
            [(
                "Patch.esp".to_string(),
                "ESP".to_string(),
                "ESL".to_string()
            )]
        );
        let files_only = compare_trees(&old, &new, "stellarblade");
        assert_eq!(files_only.added, r.added);
        assert!(files_only.plugins_added.is_empty() && files_only.masters_added.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_plugin_header_is_listed_but_adds_no_master() {
        let dir = temp_dir("broken");
        fs::create_dir_all(dir.join("old")).unwrap();
        fs::create_dir_all(dir.join("new")).unwrap();
        fs::write(dir.join("new/Broken.esp"), b"junk").unwrap();
        let r = compare_trees(&dir.join("old"), &dir.join("new"), "skyrimse");
        assert_eq!(r.plugins_added, ["Broken.esp"]);
        assert!(r.masters_added.is_empty() && r.kinds_changed.is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
