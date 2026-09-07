//! `collection.json`: what a Nexus collection actually says.
//!
//! A collection is not a list of mods, it is a RECIPE. The list is the only part
//! the Nexus API will tell you about; everything that makes the collection play
//! the way its author built it - the order mods are installed in, the answers
//! they gave to each scripted installer, which mod wins a file conflict, which
//! plugins load in which order, the binary patches - exists only inside the
//! collection's own archive, in this file. Vortex's upload path strips all of it
//! before submitting metadata, so no amount of API work substitutes for reading
//! the archive.
//!
//! Parsed TOLERANTLY, on purpose. The format carries no version field of its
//! own, nothing announces a change, and the schema Vortex ships does not close
//! the key set - its own reader treats a validation failure as a warning and
//! carries on. So every field is optional with a sane default, unknown keys are
//! ignored rather than fatal, and the ones that were ignored are RECORDED, so a
//! collection using something Eidos has never seen says so in the report instead
//! of quietly installing differently.

use std::collections::BTreeMap;

use serde::Deserialize;

/// A whole `collection.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Collection {
    pub info: Info,
    pub mods: Vec<Mod>,
    pub mod_rules: Vec<ModRule>,
    /// Bethesda games only: which plugins the collection expects ENABLED.
    ///
    /// Deliberately not a load order. Vortex writes a position here and its own
    /// reader never applies one; ordering is expressed as LOOT rules instead.
    pub plugins: Vec<Plugin>,
    /// Bethesda games only: LOOT userlist entries, to be MERGED into the user's.
    pub plugin_rules: PluginRules,
}

/// What the collection says about itself.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Info {
    pub author: String,
    pub author_url: String,
    pub name: String,
    pub description: String,
    /// The author's own notes, Markdown, shown before anything is installed.
    pub install_instructions: String,
    /// The Nexus domain of the COLLECTION. Each member carries its own, and they
    /// can differ - a Skyrim SE collection may pull an asset from the LE page.
    pub domain_name: String,
    pub game_versions: Vec<String>,
}

/// Where a member's file comes from.
///
/// Five types, and only one of them is a Nexus file. A reader that assumes
/// otherwise drops four fifths of the vocabulary on the floor - which is what
/// Eidos did, and then told the user the member was "no longer on Nexus".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    /// A file on a Nexus mod page: `modId` + `fileId`.
    #[default]
    Nexus,
    /// A direct URL the manager may fetch itself.
    Direct,
    /// A page the USER must visit; the download comes back through `nxm://`.
    Browse,
    /// No automation at all: the author left instructions to follow by hand.
    Manual,
    /// The file travels INSIDE the collection archive, under `bundled/`.
    Bundle,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Source {
    #[serde(rename = "type")]
    pub kind: SourceType,
    pub url: String,
    /// Shown to the user for `browse` and `manual`.
    pub instructions: String,
    pub mod_id: Option<u64>,
    pub file_id: Option<u64>,
    pub update_policy: String,
    pub adult_content: bool,
    pub md5: String,
    /// NOTE the spelling: a SOURCE says `logicalFilename`, a rule REFERENCE says
    /// `logicalFileName`. They mean the same thing and differ by one capital,
    /// which is exactly the kind of difference that silently matches nothing.
    #[serde(alias = "logicalFileName")]
    pub logical_filename: String,
    pub file_expression: String,
    pub tag: String,
    /// Nexus's own OpenAPI calls this kilobytes and every writer in Vortex puts
    /// raw bytes in it. Kept as given; nothing here does arithmetic on it.
    pub file_size: Option<u64>,
}

/// One member of the collection.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Mod {
    /// The author's display name for it. Load-bearing: a mod rule with no other
    /// marker falls back to this exact string.
    pub name: String,
    pub version: String,
    pub optional: bool,
    /// This member's own Nexus domain, not the collection's.
    pub domain_name: String,
    pub source: Source,
    /// Install order. Ascending; every mod in a phase completes before the next
    /// begins. Absent means 0.
    pub phase: u32,
    /// The answers the author gave to this mod's scripted installer.
    pub choices: Option<Choices>,
    /// `path -> CRC32 of the patched file`, with the patch bytes shipped at
    /// `patches/<name>/<path>.diff`.
    pub patches: BTreeMap<String, String>,
    /// Paths this mod provides whatever the deployment order says.
    pub file_overrides: Vec<String>,
    /// A per-file list, used when the author pinned exact contents.
    pub hashes: Vec<FileHash>,
    /// The author's note for this member.
    pub instructions: String,
    pub author: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FileHash {
    pub path: String,
    pub md5: String,
}

/// Replayed installer answers.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Choices {
    /// `"fomod"` in practice. Kept so a future installer kind is visible rather
    /// than silently replayed as if it were a FOMOD.
    #[serde(rename = "type")]
    pub kind: String,
    /// `null` is a legal value here and means "no answers recorded".
    pub options: Option<Vec<ChoiceStep>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ChoiceStep {
    pub name: String,
    pub groups: Vec<ChoiceGroup>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ChoiceGroup {
    pub name: String,
    /// The options the author SELECTED in this group. An option absent from the
    /// list was not selected.
    pub choices: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ChoiceOption {
    pub name: String,
    /// The option's position in its group when the author recorded it. A
    /// tiebreak, never the primary key: positions move between mod versions and
    /// names usually do not.
    pub idx: usize,
}

/// What one endpoint of a mod rule points at.
///
/// There is NO mod id here. Vortex builds these from whatever markers a local
/// mod happened to carry, and its fallback is the mod's display name in
/// `file_expression`. So resolving a rule means matching these against the
/// collection's own members - see [`Collection::resolve`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModReference {
    pub file_expression: String,
    #[serde(rename = "fileMD5")]
    pub file_md5: String,
    pub logical_file_name: String,
    pub version_match: String,
    /// Only ever a display aid.
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleType {
    /// The source must deploy BEFORE the reference, so the reference wins their
    /// shared files.
    #[default]
    Before,
    After,
    Requires,
    Conflicts,
    Recommends,
    Provides,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ModRule {
    pub source: ModReference,
    #[serde(rename = "type")]
    pub kind: RuleType,
    pub reference: ModReference,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Plugin {
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

/// LOOT userlist entries a collection contributes.
///
/// Kept as raw JSON values rather than modelled: they are LOOT's schema, not
/// this crate's, and they are handed to the LOOT layer verbatim. Modelling them
/// here would be a second, drifting copy of somebody else's format.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PluginRules {
    pub plugins: Vec<serde_json::Value>,
    pub groups: Vec<serde_json::Value>,
}

/// The keys this crate understands at the top level of `collection.json`.
const KNOWN_SECTIONS: &[&str] = &[
    "info",
    "mods",
    "modRules",
    "plugins",
    "pluginRules",
    "collectionConfig",
];

/// What reading a manifest produced, INCLUDING what it did not understand.
#[derive(Debug, Clone, Default)]
pub struct Read {
    pub collection: Collection,
    /// Top-level keys Eidos ignored. Reported rather than dropped: a collection
    /// carrying a section this build has never heard of will install differently
    /// from the way its author built it, and the user is entitled to know which
    /// one it was.
    pub unknown_sections: Vec<String>,
}

/// Parse a `collection.json`.
pub fn read(text: &str) -> Result<Read, String> {
    let raw: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("collection.json is not valid JSON: {e}"))?;
    let Some(obj) = raw.as_object() else {
        return Err("collection.json is not a JSON object".to_string());
    };
    let mut unknown_sections: Vec<String> = obj
        .keys()
        .filter(|k| !KNOWN_SECTIONS.contains(&k.as_str()))
        .cloned()
        .collect();
    unknown_sections.sort();
    let collection: Collection = serde_json::from_value(raw)
        .map_err(|e| format!("collection.json is not shaped like a collection: {e}"))?;
    if collection.mods.is_empty() {
        return Err("collection.json lists no mods".to_string());
    }
    Ok(Read {
        collection,
        unknown_sections,
    })
}

impl Collection {
    /// Which member a rule endpoint points at, by index.
    ///
    /// A reference carries no mod id, so this matches on the markers Vortex
    /// actually writes, most specific first: the archive's MD5, then the logical
    /// file name, then the file expression - which for a mod that had none of
    /// those is simply the member's display NAME, because that is the fallback
    /// `makeModReference` falls back to.
    ///
    /// `None` is a real answer and must stay one. A collection can carry a rule
    /// about a mod that is not one of its members, and inventing a match for it
    /// would reorder somebody's mod list on the strength of a guess.
    pub fn resolve(&self, r: &ModReference) -> Option<usize> {
        let eq = |a: &str, b: &str| !a.is_empty() && a.eq_ignore_ascii_case(b);
        self.mods
            .iter()
            .position(|m| eq(&r.file_md5, &m.source.md5))
            .or_else(|| {
                self.mods
                    .iter()
                    .position(|m| eq(&r.logical_file_name, &m.source.logical_filename))
            })
            .or_else(|| {
                self.mods
                    .iter()
                    .position(|m| eq(&r.file_expression, &m.source.file_expression))
            })
            .or_else(|| {
                self.mods
                    .iter()
                    .position(|m| eq(&r.file_expression, &m.name))
            })
            .or_else(|| self.mods.iter().position(|m| eq(&r.description, &m.name)))
    }

    /// Every distinct install phase, ascending.
    ///
    /// Enumerated from what the members actually carry rather than counted from
    /// zero: a collection whose phases are 0 and 20 has two phases, not
    /// twenty-one, and walking the gap is how a phase engine ends up waiting
    /// forever for a phase nobody is in.
    pub fn phases(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.mods.iter().map(|m| m.phase).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape Vortex writes, trimmed to what matters here. Built from its
    /// exporter and its type definitions rather than typed from imagination.
    const SAMPLE: &str = r#"{
      "info": {
        "author": "Someone", "authorUrl": "", "name": "A Collection",
        "description": "", "installInstructions": "Read me first.",
        "domainName": "skyrimspecialedition", "gameVersions": ["1.6.1170"]
      },
      "mods": [
        { "name": "Base Textures", "version": "1.2", "optional": false,
          "domainName": "skyrimspecialedition", "phase": 0,
          "source": { "type": "nexus", "modId": 100, "fileId": 200,
                      "md5": "aabb", "logicalFilename": "BaseTextures",
                      "fileSize": 1234 } },
        { "name": "A Patch", "version": "3.0", "optional": false,
          "domainName": "skyrimspecialedition", "phase": 1,
          "source": { "type": "nexus", "modId": 101, "fileId": 201 },
          "choices": { "type": "fomod", "options": [
             { "name": "Main", "groups": [
                { "name": "Resolution", "choices": [ { "name": "2K", "idx": 1 } ] } ] } ] },
          "patches": { "meshes/x.nif": "DEADBEEF" },
          "fileOverrides": ["textures/win.dds"] },
        { "name": "From The Web", "version": "1.0", "optional": true,
          "domainName": "skyrimspecialedition",
          "source": { "type": "browse", "url": "https://example.invalid/x",
                      "instructions": "Grab the 2K one." } }
      ],
      "modRules": [
        { "type": "after",
          "source": { "fileExpression": "A Patch" },
          "reference": { "logicalFileName": "BaseTextures" } }
      ],
      "plugins": [ { "name": "Base.esp", "enabled": true }, { "name": "Off.esp", "enabled": false } ],
      "pluginRules": { "plugins": [ { "name": "A.esp", "after": ["B.esp"] } ], "groups": [] },
      "somethingNexusAddedLater": { "whatever": 1 }
    }"#;

    #[test]
    fn a_manifest_parses_into_its_recipe() {
        let r = read(SAMPLE).unwrap();
        let c = &r.collection;
        assert_eq!(c.info.name, "A Collection");
        assert_eq!(c.info.install_instructions, "Read me first.");
        assert_eq!(c.mods.len(), 3);
        assert_eq!(c.mods[0].source.kind, SourceType::Nexus);
        assert_eq!(c.mods[0].source.mod_id, Some(100));
        assert_eq!(c.mods[0].phase, 0);
        assert_eq!(c.mods[1].phase, 1);
        assert!(c.mods[2].optional);
        assert_eq!(c.mods[2].source.kind, SourceType::Browse);
        assert_eq!(c.mods[2].source.url, "https://example.invalid/x");
        assert_eq!(c.mods[1].patches.get("meshes/x.nif").unwrap(), "DEADBEEF");
        assert_eq!(c.mods[1].file_overrides, vec!["textures/win.dds"]);
        assert_eq!(c.plugins.len(), 2);
        assert!(c.plugins[0].enabled && !c.plugins[1].enabled);
        assert_eq!(c.plugin_rules.plugins.len(), 1);
    }

    #[test]
    fn a_section_nobody_here_understands_is_reported_not_dropped() {
        // The format has no version field and nothing announces a change, so the
        // only defence is to say what was ignored.
        let r = read(SAMPLE).unwrap();
        assert_eq!(r.unknown_sections, vec!["somethingNexusAddedLater"]);
    }

    #[test]
    fn the_recorded_answers_survive_the_trip() {
        let r = read(SAMPLE).unwrap();
        let ch = r.collection.mods[1].choices.as_ref().unwrap();
        assert_eq!(ch.kind, "fomod");
        let steps = ch.options.as_ref().unwrap();
        assert_eq!(steps[0].name, "Main");
        assert_eq!(steps[0].groups[0].name, "Resolution");
        assert_eq!(steps[0].groups[0].choices[0].name, "2K");
        assert_eq!(steps[0].groups[0].choices[0].idx, 1);
    }

    #[test]
    fn a_rule_resolves_to_a_member_by_whichever_marker_it_carries() {
        let r = read(SAMPLE).unwrap();
        let c = &r.collection;
        let rule = &c.mod_rules[0];
        // `logicalFileName` on the reference vs `logicalFilename` on the source:
        // one capital apart, and matching them is the whole job.
        assert_eq!(c.resolve(&rule.reference), Some(0));
        // The source here carries only a file expression, which for a mod with
        // no markers is the member's display NAME.
        assert_eq!(c.resolve(&rule.source), Some(1));
        assert_eq!(rule.kind, RuleType::After);
    }

    #[test]
    fn a_rule_about_a_mod_that_is_not_a_member_resolves_to_nothing() {
        // Real answer, not a failure. Inventing a match would reorder a mod list
        // on the strength of a guess.
        let r = read(SAMPLE).unwrap();
        let stranger = ModReference {
            file_expression: "Some Other Mod".into(),
            ..ModReference::default()
        };
        assert_eq!(r.collection.resolve(&stranger), None);
        // And an empty reference matches nothing rather than the first member.
        assert_eq!(r.collection.resolve(&ModReference::default()), None);
    }

    #[test]
    fn phases_are_the_ones_that_exist_not_a_range() {
        let mut c = read(SAMPLE).unwrap().collection;
        assert_eq!(c.phases(), vec![0, 1]);
        c.mods[2].phase = 20;
        assert_eq!(c.phases(), vec![0, 1, 20], "not 0..=20");
    }

    #[test]
    fn a_manifest_that_is_not_one_is_refused_by_name() {
        assert!(read("not json").unwrap_err().contains("valid JSON"));
        assert!(read("[]").unwrap_err().contains("JSON object"));
        assert!(read("{}").unwrap_err().contains("no mods"));
        // A plausible-looking file with an empty list is still not installable.
        assert!(read(r#"{"info":{"name":"x"},"mods":[]}"#).is_err());
    }

    #[test]
    fn everything_is_optional_because_the_format_promises_nothing() {
        // No version field, no closed key set, and Vortex's own reader treats a
        // schema failure as a warning. A minimal manifest must still read.
        let r = read(r#"{"mods":[{"name":"Only This"}]}"#).unwrap();
        assert_eq!(r.collection.mods[0].name, "Only This");
        assert_eq!(r.collection.mods[0].source.kind, SourceType::Nexus);
        assert!(r.collection.mods[0].choices.is_none());
        assert_eq!(r.collection.phases(), vec![0]);
    }

    #[test]
    fn an_enabled_flag_that_is_absent_means_enabled() {
        // Vortex writes `{ name, enabled? }` and omits the flag for the common
        // case; reading a missing flag as false would silently disable plugins.
        let r = read(r#"{"mods":[{"name":"m"}],"plugins":[{"name":"A.esp"}]}"#).unwrap();
        assert!(r.collection.plugins[0].enabled);
    }
}
