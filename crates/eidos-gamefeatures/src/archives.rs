//! Game-specific archive activation, separate from directory parsing and mod layers.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveActivation {
    pub name: String,
    pub plugin: Option<String>,
    pub order_uncertain: bool,
}
#[derive(Debug, Clone, Default)]
pub struct ArchivePlan {
    /// Lowest precedence first. For ambiguous groups consult `diagnostics`.
    pub archives: Vec<ArchiveActivation>,
    pub diagnostics: Vec<String>,
}

/// Test association using libloot's per-game rules. `plugin` must be enabled.
/// Starfield's voice suffix uses the selected INI language (e.g. `en`, `fr`).
pub fn archive_matches_plugin(game_id: &str, archive: &str, plugin: &str, language: &str) -> bool {
    let a = archive.to_ascii_lowercase();
    let p = plugin.to_ascii_lowercase();
    let Some((stem, ext)) = p.rsplit_once('.') else {
        return false;
    };
    if stem.is_empty() || !matches!(ext, "esp" | "esm" | "esl") || a.contains(['/', '\\']) {
        return false;
    }
    match game_id {
        "skyrim" => a == format!("{stem}.bsa"),
        "skyrimse" | "skyrimvr" | "enderalse" => {
            a == format!("{stem}.bsa") || a == format!("{stem} - textures.bsa")
        }
        "oblivion" | "oblivionremastered" => {
            ext == "esp" && a.ends_with(".bsa") && a.starts_with(stem)
        }
        "fallout3" | "falloutnv" => a.ends_with(".bsa") && a.starts_with(stem),
        "fallout4" | "fallout4vr" => a.ends_with(".ba2") && a.starts_with(stem),
        "starfield" => [
            " - main".to_string(),
            " - textures".into(),
            " - localization".into(),
            format!(" - voices_{}", language.to_ascii_lowercase()),
        ]
        .iter()
        .any(|suffix| a == format!("{stem}{suffix}.ba2")),
        _ => false,
    }
}

/// Inputs are winning Data-root archive names, enabled plugins in load order,
/// and INI texts in increasing override priority. A later INI replaces a key;
/// lists are never unioned with superseded values. Missing named archives and
/// uncertain engine ordering are explicit diagnostics, not silent omissions.
pub fn archive_plan(
    game_id: &str,
    available: &[String],
    active_plugins: &[String],
    ini_texts: &[String],
    language: &str,
) -> ArchivePlan {
    let mut plan = ArchivePlan::default();
    if !matches!(
        game_id,
        "morrowind"
            | "oblivion"
            | "oblivionremastered"
            | "fallout3"
            | "falloutnv"
            | "skyrim"
            | "skyrimse"
            | "enderalse"
            | "skyrimvr"
            | "fallout4"
            | "fallout4vr"
            | "starfield"
    ) {
        plan.diagnostics
            .push(format!("Archive activation is unsupported for {game_id}."));
        return plan;
    }
    let available: BTreeMap<_, _> = available
        .iter()
        .filter(|n| !n.contains(['/', '\\']))
        .map(|n| (n.to_ascii_lowercase(), n.clone()))
        .collect();
    let mut ini = BTreeMap::new();
    for text in ini_texts {
        let mut document = BTreeMap::new();
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with([';', '#']) {
                continue;
            }
            if let Some(s) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                section = s.trim().to_ascii_lowercase();
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                document
                    .entry((section.clone(), k.trim().to_ascii_lowercase()))
                    .or_insert_with(|| v.trim().to_string());
            }
        }
        ini.extend(document);
    }
    let language = ini
        .get(&("general".into(), "slanguage".into()))
        .map(String::as_str)
        .unwrap_or(language);
    let language = if language.is_empty() { "en" } else { language };
    let keys: &[&str] = match game_id {
        "morrowind" => &[],
        "oblivion" | "oblivionremastered" | "fallout3" | "falloutnv" => &["sarchivelist"],
        "skyrim" | "skyrimse" | "skyrimvr" | "enderalse" => {
            &["sresourcearchivelist", "sresourcearchivelist2"]
        }
        _ => &[
            "sresourcearchivelist",
            "sresourcearchivelist2",
            "sresourcearchivelistbeta",
            "sresourceindexfilelist",
            "sresourcearchive2list",
            "sresourcestartuparchivelist",
            "sresourcearchivememorycachelist",
            "sresourceenglishvoicelist",
        ],
    };
    let mut registered = Vec::new();
    if game_id == "morrowind" {
        let mut numbered: Vec<_> = ini
            .iter()
            .filter_map(|((section, key), value)| {
                (section == "archives")
                    .then(|| {
                        key.strip_prefix("archive ")
                            .and_then(|i| i.parse::<u32>().ok())
                            .map(|i| (i, value))
                    })
                    .flatten()
            })
            .collect();
        numbered.sort_by_key(|(i, _)| *i);
        if available.contains_key("morrowind.bsa") {
            registered.push("Morrowind.bsa".into());
        }
        for (expected, (actual, value)) in numbered.into_iter().enumerate() {
            if actual != expected as u32 {
                plan.diagnostics.push(format!("Morrowind archive numbering stops at missing Archive {expected}; later entries are not active."));
                break;
            }
            registered.push(value.clone());
        }
    } else {
        for key in keys {
            if let Some(value) = ini.get(&("archive".into(), (*key).into())) {
                registered.extend(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                );
            }
        }
    }
    let mut seen = BTreeSet::new();
    for name in registered {
        if !seen.insert(name.to_ascii_lowercase()) {
            continue;
        }
        let resolved = available.get(&name.to_ascii_lowercase());
        if resolved.is_none() {
            plan.diagnostics.push(format!(
                "INI-registered archive {name} is missing from the active layers."
            ));
        }
        plan.archives.push(ArchiveActivation {
            name: resolved.cloned().unwrap_or(name),
            plugin: None,
            order_uncertain: false,
        });
    }
    for plugin in active_plugins {
        let mut matched: Vec<_> = available
            .values()
            .filter(|name| archive_matches_plugin(game_id, name, plugin, language))
            .cloned()
            .collect();
        // Skyrim SE and Starfield have explicit suffix sequences. Arbitrary-suffix
        // engines have no proven same-plugin tie order in the upstream readers.
        if matches!(game_id, "skyrimse" | "skyrimvr" | "enderalse" | "starfield") {
            matched.sort_by_key(|name| {
                let n = name.to_ascii_lowercase();
                if n.ends_with(" - textures.bsa") {
                    1
                } else if n.ends_with(" - main.ba2") {
                    0
                } else if n.ends_with(" - textures.ba2") {
                    1
                } else if n.ends_with(" - localization.ba2") {
                    2
                } else if n.contains(" - voices_") {
                    3
                } else {
                    0
                }
            });
        }
        if matched.len() > 1
            && matches!(
                game_id,
                "oblivion"
                    | "oblivionremastered"
                    | "fallout3"
                    | "falloutnv"
                    | "fallout4"
                    | "fallout4vr"
            )
        {
            plan.diagnostics.push(format!("Archive order within {plugin} is unverified; alphabetical display order is provisional: {}.",matched.join(", ")));
        }
        for name in matched {
            if seen.insert(name.to_ascii_lowercase()) {
                plan.archives.push(ArchiveActivation {
                    name,
                    plugin: Some(plugin.clone()),
                    order_uncertain: matches!(
                        game_id,
                        "oblivion"
                            | "oblivionremastered"
                            | "fallout3"
                            | "falloutnv"
                            | "fallout4"
                            | "fallout4vr"
                    ),
                });
            }
        }
    }
    if matches!(
        game_id,
        "morrowind" | "oblivion" | "oblivionremastered" | "fallout3" | "falloutnv"
    ) && !plan.archives.is_empty()
    {
        plan.diagnostics.push("Legacy archive/loose precedence also depends on timestamps and archive invalidation; displayed load-order precedence assumes invalidation is enabled.".into());
    }
    plan
}

pub fn orphan_archives_for_game(
    game_id: &str,
    archives: &[(String, String)],
    active_plugins: &[String],
    ini_archives: &[String],
    language: &str,
) -> Vec<(String, String)> {
    archives
        .iter()
        .filter(|(_, a)| {
            !ini_archives.iter().any(|i| i.eq_ignore_ascii_case(a))
                && !active_plugins
                    .iter()
                    .any(|p| archive_matches_plugin(game_id, a, p, language))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn names(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn game_rules_do_not_treat_sse_as_arbitrary_prefix() {
        for (game, a, p, want) in [
            ("skyrimse", "Foo - Scripts.bsa", "Foo.esp", false),
            ("skyrimse", "FOO - TEXTURES.BSA", "foo.esm", true),
            ("skyrim", "Foo - Textures.bsa", "Foo.esp", false),
            ("oblivion", "FooMore.bsa", "Foo.esm", false),
            ("oblivion", "FooMore.bsa", "Foo.esp", true),
            ("falloutnv", "FooMore.bsa", "Foo.esm", true),
            ("fallout4", "FooMore.ba2", "Foo.esl", true),
            ("morrowind", "Foo.bsa", "Foo.esp", false),
            ("starfield", "Foo - Voices_fr.ba2", "Foo.esm", true),
        ] {
            assert_eq!(
                archive_matches_plugin(game, a, p, "fr"),
                want,
                "{game}: {a}"
            )
        }
    }
    #[test]
    fn enderal_uses_sse_archive_rules_and_its_effective_ini() {
        let plan = archive_plan(
            "enderalse",
            &names(&[
                "Base.bsa",
                "Patch.bsa",
                "Patch - Textures.bsa",
                "Patch - Scripts.bsa",
            ]),
            &names(&["Patch.esp"]),
            &names(&["[Archive]\nsResourceArchiveList=Base.bsa\n"]),
            "en",
        );
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(
            plan.archives
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            ["Base.bsa", "Patch.bsa", "Patch - Textures.bsa"]
        );
        assert_eq!(plan.archives[1].plugin.as_deref(), Some("Patch.esp"));
    }

    #[test]
    fn disabled_archives_ini_overrides_language_and_order() {
        let files = names(&[
            "Base.bsa",
            "Removed.bsa",
            "A.bsa",
            "A - Textures.bsa",
            "B.bsa",
            "Disabled.bsa",
            "A - Scripts.bsa",
        ]);
        let plan = archive_plan(
            "skyrimse",
            &files,
            &names(&["A.esp", "B.esm"]),
            &names(&[
                "[Archive]\nsResourceArchiveList=Removed.bsa\n",
                "[Archive]\nsResourceArchiveList=Base.bsa\n",
            ]),
            "en",
        );
        assert_eq!(
            plan.archives
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            ["Base.bsa", "A.bsa", "A - Textures.bsa", "B.bsa"]
        );
        assert!(plan.diagnostics.is_empty());
        assert_eq!(plan.archives[1].plugin.as_deref(), Some("A.esp"));
        let plan = archive_plan(
            "starfield",
            &names(&["A - Voices_en.ba2", "A - Voices_fr.ba2"]),
            &names(&["A.esm"]),
            &names(&["[General]\nsLanguage=fr\n"]),
            "en",
        );
        assert_eq!(plan.archives[0].name, "A - Voices_fr.ba2");
        assert_eq!(plan.archives.len(), 1);
    }
    #[test]
    fn morrowind_ini_numbering_missing_and_unsupported_are_explicit() {
        let plan = archive_plan(
            "morrowind",
            &names(&["A.bsa", "B.bsa"]),
            &[],
            &names(&["[Archives]\nArchive 1=B.bsa\nArchive 0=A.bsa\nArchive 2=Missing.bsa"]),
            "en",
        );
        assert_eq!(
            plan.archives
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            ["A.bsa", "B.bsa", "Missing.bsa"]
        );
        assert_eq!(plan.diagnostics.len(), 2);
        assert!(
            !archive_plan("unknown", &[], &[], &[], "en")
                .diagnostics
                .is_empty()
        );
    }
}
