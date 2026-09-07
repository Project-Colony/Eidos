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

use serde::{Deserialize, Deserializer};

/// Anything that is present but the wrong shape becomes the default.
///
/// `#[serde(default)]` only ever covers an ABSENT key. The writer here is
/// TypeScript, where an explicit `null` is the ordinary spelling of "this was
/// never computed", and Nexus quotes its integers about half the time - so
/// strict field types turn one such value anywhere in a 200-member manifest
/// into a refusal of the whole collection, which is precisely the failure the
/// module header says this parser does not have.
fn lenient<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + serde::de::DeserializeOwned,
{
    let v = serde_json::Value::deserialize(d)?;
    Ok(serde_json::from_value(v).unwrap_or_default())
}

/// A string that tolerates `null`, and a number written as one.
fn lenient_str<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match serde_json::Value::deserialize(d)? {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    })
}

/// A whole number however it was written: bare, quoted, or with a `.0` on it.
fn loose_u64(v: &serde_json::Value) -> Option<u64> {
    match v {
        serde_json::Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
        serde_json::Value::String(s) => {
            let t = s.trim();
            t.parse::<u64>()
                .ok()
                .or_else(|| t.parse::<f64>().ok().filter(|f| *f >= 0.0).map(|f| f as u64))
        }
        _ => None,
    }
}

fn lenient_id<'de, D>(d: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(loose_u64(&serde_json::Value::deserialize(d)?))
}

fn lenient_u32<'de, D>(d: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(loose_u64(&serde_json::Value::deserialize(d)?).unwrap_or(0) as u32)
}

fn lenient_usize<'de, D>(d: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(loose_u64(&serde_json::Value::deserialize(d)?).unwrap_or(0) as usize)
}

fn lenient_bool<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(as_bool(&serde_json::Value::deserialize(d)?).unwrap_or(false))
}

/// `enabled` defaults to true when absent, so a wrong shape must too.
fn lenient_bool_true<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(as_bool(&serde_json::Value::deserialize(d)?).unwrap_or(true))
}

fn as_bool(v: &serde_json::Value) -> Option<bool> {
    match v {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::Number(n) => n.as_i64().map(|i| i != 0),
        serde_json::Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// A source type, case-insensitively, with anything else kept as unknown rather
/// than refused.
fn lenient_source_type<'de, D>(d: D) -> Result<SourceType, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    let s = v.as_str().unwrap_or_default().trim().to_ascii_lowercase();
    Ok(serde_json::from_value(serde_json::Value::String(s)).unwrap_or(SourceType::Unknown))
}

fn lenient_rule_type<'de, D>(d: D) -> Result<RuleType, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    let s = v.as_str().unwrap_or_default().trim().to_ascii_lowercase();
    Ok(serde_json::from_value(serde_json::Value::String(s)).unwrap_or(RuleType::Unknown))
}

/// A whole `collection.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Collection {
    #[serde(deserialize_with = "lenient")]
    pub info: Info,
    #[serde(deserialize_with = "lenient")]
    pub mods: Vec<Mod>,
    #[serde(deserialize_with = "lenient")]
    pub mod_rules: Vec<ModRule>,
    /// Bethesda games only: which plugins the collection expects ENABLED.
    ///
    /// Deliberately not a load order. Vortex writes a position here and its own
    /// reader never applies one; ordering is expressed as LOOT rules instead.
    #[serde(deserialize_with = "lenient")]
    pub plugins: Vec<Plugin>,
    /// Bethesda games only: LOOT userlist entries, to be MERGED into the user's.
    #[serde(deserialize_with = "lenient")]
    pub plugin_rules: PluginRules,
    /// Tools the collection expects to exist. NAMED, never created.
    ///
    /// `exe` is a Windows path relative to the game directory, with no field for
    /// a prefix, a runner or a launcher - and Vortex's own authoring UI warns
    /// that the user must already have these installed. Under Eidos this is the
    /// DynDOLOD situation: the honest thing is to tell the user which tools the
    /// collection assumes, not to invent tool entries pointing at nothing.
    #[serde(deserialize_with = "lenient")]
    pub tools: Vec<Tool>,
}

/// A tool a collection expects the user to already have.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Tool {
    #[serde(deserialize_with = "lenient_str")]
    pub name: String,
    #[serde(deserialize_with = "lenient_str")]
    pub exe: String,
}

/// What the collection says about itself.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Info {
    #[serde(deserialize_with = "lenient_str")]
    pub author: String,
    #[serde(deserialize_with = "lenient_str")]
    pub author_url: String,
    #[serde(deserialize_with = "lenient_str")]
    pub name: String,
    #[serde(deserialize_with = "lenient_str")]
    pub description: String,
    /// The author's own notes, Markdown, shown before anything is installed.
    #[serde(deserialize_with = "lenient_str")]
    pub install_instructions: String,
    /// The Nexus domain of the COLLECTION. Each member carries its own, and they
    /// can differ - a Skyrim SE collection may pull an asset from the LE page.
    #[serde(deserialize_with = "lenient_str")]
    pub domain_name: String,
    #[serde(deserialize_with = "lenient")]
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
    /// A type this build has never seen.
    ///
    /// The format grows and announces nothing, so an unrecognised type has to be
    /// ONE unusable member, reported as such - not a refusal of the other 199.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Source {
    #[serde(rename = "type", deserialize_with = "lenient_source_type")]
    pub kind: SourceType,
    #[serde(deserialize_with = "lenient_str")]
    pub url: String,
    /// Shown to the user for `browse` and `manual`.
    #[serde(deserialize_with = "lenient_str")]
    pub instructions: String,
    #[serde(deserialize_with = "lenient_id")]
    pub mod_id: Option<u64>,
    #[serde(deserialize_with = "lenient_id")]
    pub file_id: Option<u64>,
    #[serde(deserialize_with = "lenient_str")]
    pub update_policy: String,
    #[serde(deserialize_with = "lenient_bool")]
    pub adult_content: bool,
    #[serde(deserialize_with = "lenient_str")]
    pub md5: String,
    /// NOTE the spelling: a SOURCE says `logicalFilename`, a rule REFERENCE says
    /// `logicalFileName`. They mean the same thing and differ by one capital,
    /// which is exactly the kind of difference that silently matches nothing.
    #[serde(alias = "logicalFileName", deserialize_with = "lenient_str")]
    pub logical_filename: String,
    #[serde(deserialize_with = "lenient_str")]
    pub file_expression: String,
    #[serde(deserialize_with = "lenient_str")]
    pub tag: String,
    /// Nexus's own OpenAPI calls this kilobytes and every writer in Vortex puts
    /// raw bytes in it. Kept as given; nothing here does arithmetic on it.
    #[serde(deserialize_with = "lenient_id")]
    pub file_size: Option<u64>,
}

/// One member of the collection.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Mod {
    /// The author's display name for it. Load-bearing: a mod rule with no other
    /// marker falls back to this exact string.
    #[serde(deserialize_with = "lenient_str")]
    pub name: String,
    #[serde(deserialize_with = "lenient_str")]
    pub version: String,
    #[serde(deserialize_with = "lenient_bool")]
    pub optional: bool,
    /// This member's own Nexus domain, not the collection's.
    #[serde(deserialize_with = "lenient_str")]
    pub domain_name: String,
    #[serde(deserialize_with = "lenient")]
    pub source: Source,
    /// Install order. Ascending; every mod in a phase completes before the next
    /// begins. Absent means 0.
    #[serde(deserialize_with = "lenient_u32")]
    pub phase: u32,
    /// The answers the author gave to this mod's scripted installer.
    #[serde(deserialize_with = "lenient")]
    pub choices: Option<Choices>,
    /// `path -> CRC32 of the patched file`, with the patch bytes shipped at
    /// `patches/<name>/<path>.diff`.
    #[serde(deserialize_with = "lenient")]
    pub patches: BTreeMap<String, String>,
    /// Paths this mod provides whatever the deployment order says.
    #[serde(deserialize_with = "lenient")]
    pub file_overrides: Vec<String>,
    /// A per-file list, used when the author pinned exact contents.
    #[serde(deserialize_with = "lenient")]
    pub hashes: Vec<FileHash>,
    /// The author's note for this member.
    #[serde(deserialize_with = "lenient_str")]
    pub instructions: String,
    #[serde(deserialize_with = "lenient_str")]
    pub author: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FileHash {
    #[serde(deserialize_with = "lenient_str")]
    pub path: String,
    #[serde(deserialize_with = "lenient_str")]
    pub md5: String,
}

/// Replayed installer answers.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Choices {
    /// `"fomod"` in practice. Kept so a future installer kind is visible rather
    /// than silently replayed as if it were a FOMOD.
    #[serde(rename = "type", deserialize_with = "lenient_str")]
    pub kind: String,
    /// `null` is a legal value here and means "no answers recorded".
    #[serde(deserialize_with = "lenient")]
    pub options: Option<Vec<ChoiceStep>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ChoiceStep {
    #[serde(deserialize_with = "lenient_str")]
    pub name: String,
    #[serde(deserialize_with = "lenient")]
    pub groups: Vec<ChoiceGroup>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ChoiceGroup {
    #[serde(deserialize_with = "lenient_str")]
    pub name: String,
    /// The options the author SELECTED in this group. An option absent from the
    /// list was not selected.
    #[serde(deserialize_with = "lenient")]
    pub choices: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ChoiceOption {
    #[serde(deserialize_with = "lenient_str")]
    pub name: String,
    /// The option's position in its group when the author recorded it. A
    /// tiebreak, never the primary key: positions move between mod versions and
    /// names usually do not.
    #[serde(deserialize_with = "lenient_usize")]
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
    #[serde(deserialize_with = "lenient_str")]
    pub file_expression: String,
    #[serde(rename = "fileMD5", deserialize_with = "lenient_str")]
    pub file_md5: String,
    #[serde(deserialize_with = "lenient_str")]
    pub logical_file_name: String,
    /// The author's own identity marker for a member, and the only one that does
    /// not move when the mod is updated. A `browse`, `manual` or `direct` member
    /// has no md5 and no logical file name, so this is often the only exact
    /// marker a rule about it can carry.
    #[serde(deserialize_with = "lenient_str")]
    pub tag: String,
    #[serde(deserialize_with = "lenient_str")]
    pub version_match: String,
    /// Only ever a display aid.
    #[serde(deserialize_with = "lenient_str")]
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
    /// A kind this build has never seen. Ordered by nothing, fatal to nothing.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ModRule {
    #[serde(deserialize_with = "lenient")]
    pub source: ModReference,
    #[serde(rename = "type", deserialize_with = "lenient_rule_type")]
    pub kind: RuleType,
    #[serde(deserialize_with = "lenient")]
    pub reference: ModReference,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Plugin {
    #[serde(deserialize_with = "lenient_str")]
    pub name: String,
    #[serde(default = "yes", deserialize_with = "lenient_bool_true")]
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
    #[serde(deserialize_with = "lenient")]
    pub plugins: Vec<serde_json::Value>,
    #[serde(deserialize_with = "lenient")]
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
    // Read and reported, though nothing is created from it - see `Tool`. It
    // appears in real published collections, so leaving it out of this list
    // would put "tools" in the not-understood line of every single report.
    "tools",
];

/// The keys this crate understands inside one member.
const KNOWN_MOD_KEYS: &[&str] = &[
    "name",
    "version",
    "optional",
    "domainName",
    "source",
    "phase",
    "choices",
    "patches",
    "fileOverrides",
    "hashes",
    "instructions",
    "author",
    // Display metadata every real manifest carries. Nothing is installed from
    // it, and listing it here keeps it out of every single report.
    "details",
];

/// The keys this crate understands inside one member's `source`.
const KNOWN_SOURCE_KEYS: &[&str] = &[
    "type",
    "url",
    "instructions",
    "modId",
    "fileId",
    "updatePolicy",
    "adultContent",
    "md5",
    "logicalFilename",
    "logicalFileName",
    "fileExpression",
    "tag",
    "fileSize",
];

/// The keys this crate understands inside `info`.
const KNOWN_INFO_KEYS: &[&str] = &[
    "author",
    "authorUrl",
    "name",
    "description",
    "installInstructions",
    "domainName",
    "gameVersions",
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
    // The format grows BELOW the top level too - phase, choices, patches and
    // fileOverrides are all member keys it grew - so stopping at the six
    // sections would let exactly the additions that change behaviour pass in
    // silence.
    let member_keys = |v: &serde_json::Value, known: &[&str], prefix: &str, out: &mut Vec<String>| {
        if let Some(o) = v.as_object() {
            for k in o.keys() {
                if !known.contains(&k.as_str()) {
                    out.push(format!("{prefix}{k}"));
                }
            }
        }
    };
    if let Some(mods) = obj.get("mods").and_then(|m| m.as_array()) {
        for m in mods {
            member_keys(m, KNOWN_MOD_KEYS, "mods[].", &mut unknown_sections);
            if let Some(src) = m.get("source") {
                member_keys(src, KNOWN_SOURCE_KEYS, "mods[].source.", &mut unknown_sections);
            }
        }
    }
    if let Some(info) = obj.get("info") {
        member_keys(info, KNOWN_INFO_KEYS, "info.", &mut unknown_sections);
    }
    unknown_sections.sort();
    unknown_sections.dedup();
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

/// What matching a rule endpoint against the members produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    One(usize),
    /// Nothing carried that marker.
    None,
    /// More than one member did, so there is no answer to give.
    Ambiguous,
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
        match self.resolve_ref(r) {
            Resolved::One(i) => Some(i),
            _ => None,
        }
    }

    /// [`Collection::resolve`], with the reason a miss was a miss.
    pub fn resolve_ref(&self, r: &ModReference) -> Resolved {
        let eq = |a: &str, b: &str| {
            let a = a.trim();
            !a.is_empty() && a.eq_ignore_ascii_case(b.trim())
        };
        let only = |f: &dyn Fn(&Mod) -> bool| -> Option<Resolved> {
            let hits: Vec<usize> = self
                .mods
                .iter()
                .enumerate()
                .filter(|(_, m)| f(m))
                .map(|(i, _)| i)
                .collect();
            match hits.len() {
                0 => None,
                1 => Some(Resolved::One(hits[0])),
                // Two members can legitimately share a marker - the same archive
                // listed twice so its installer can be answered two ways - and
                // picking the first would reorder somebody's mod list on a coin
                // toss, silently. Say so instead.
                _ => Some(Resolved::Ambiguous),
            }
        };
        // Most specific first. `tag` is LAST of the identity markers, not first:
        // Vortex writes it only alongside the others, and its own authoring code
        // warns that two curators' shortids can collide - so it is the widest
        // net here, useful only for the endpoint that carries nothing else.
        for m in [
            &|m: &Mod| eq(&r.file_md5, &m.source.md5),
            &|m: &Mod| eq(&r.logical_file_name, &m.source.logical_filename),
            &|m: &Mod| eq(&r.file_expression, &m.source.file_expression),
            &|m: &Mod| eq(&r.file_expression, &m.name),
            &|m: &Mod| eq(&r.tag, &m.source.tag),
        ] as [&dyn Fn(&Mod) -> bool; 5]
        {
            if let Some(v) = only(m) {
                return v;
            }
        }
        // `description` is a display aid, not an identity. It may stand in only
        // when the reference carries no identity marker at all: a marker that
        // matched nothing is evidence AGAINST a match, not silence.
        if r.tag.trim().is_empty()
            && r.file_md5.trim().is_empty()
            && r.logical_file_name.trim().is_empty()
            && r.file_expression.trim().is_empty()
        {
            if let Some(v) = only(&|m: &Mod| eq(&r.description, &m.name)) {
                return v;
            }
        }
        Resolved::None
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

    #[test]
    fn a_source_type_this_build_has_never_seen_costs_one_member_not_the_collection() {
        let text = r#"{"mods":[
          {"name":"Fine","source":{"type":"nexus","modId":1,"fileId":2}},
          {"name":"New","source":{"type":"archive"}},
          {"name":"Shouty","source":{"type":"Nexus","modId":3,"fileId":4}}
        ]}"#;
        let r = read(text).unwrap();
        assert_eq!(r.collection.mods[1].source.kind, SourceType::Unknown);
        // And case is not vocabulary: every other matcher here is
        // case-insensitive, so this one is too.
        assert_eq!(r.collection.mods[2].source.kind, SourceType::Nexus);
    }

    #[test]
    fn one_null_or_one_quoted_number_does_not_refuse_the_manifest() {
        // The writer is TypeScript, where `null` is how "never computed" is
        // spelled, and Nexus quotes its integers about half the time.
        let text = r#"{
          "info": {"name":"C","description":null,"gameVersions":null},
          "mods":[{"name":"A","version":null,"optional":null,"phase":null,
                   "instructions":null,"hashes":null,"fileOverrides":null,
                   "patches":null,"choices":null,
                   "source":{"type":"nexus","modId":"37471","fileId":3609261.0,
                             "md5":null,"fileSize":"1234","adultContent":null}}],
          "modRules":null, "plugins":null, "pluginRules":null
        }"#;
        let r = read(text).expect("a manifest Vortex would install");
        let m = &r.collection.mods[0];
        assert_eq!(m.source.mod_id, Some(37471));
        assert_eq!(m.source.file_id, Some(3609261));
        assert_eq!(m.source.file_size, Some(1234));
        assert_eq!(m.phase, 0);
        assert!(!m.optional);
    }

    #[test]
    fn a_rule_kind_this_build_has_never_seen_is_not_fatal_either() {
        let text = r#"{"mods":[{"name":"A"}],
          "modRules":[{"type":"supersedes","source":{},"reference":{}}]}"#;
        let r = read(text).unwrap();
        assert_eq!(r.collection.mod_rules[0].kind, RuleType::Unknown);
    }

    #[test]
    fn a_rule_can_name_its_ends_by_tag() {
        let text = r#"{"mods":[
          {"name":"One","source":{"type":"browse","tag":"aaa"}},
          {"name":"Two","source":{"type":"browse","tag":"bbb"}}
        ],"modRules":[{"type":"before",
          "source":{"tag":"aaa"},"reference":{"tag":"bbb"}}]}"#;
        let c = read(text).unwrap().collection;
        assert_eq!(c.resolve(&c.mod_rules[0].source), Some(0));
        assert_eq!(c.resolve(&c.mod_rules[0].reference), Some(1));
    }

    #[test]
    fn a_marker_two_members_share_resolves_to_neither() {
        let text = r#"{"mods":[
          {"name":"Twice A","source":{"type":"nexus","modId":1,"fileId":2,"md5":"same"}},
          {"name":"Twice B","source":{"type":"nexus","modId":1,"fileId":2,"md5":"same"}}
        ]}"#;
        let c = read(text).unwrap().collection;
        let r = ModReference {
            file_md5: "same".into(),
            ..ModReference::default()
        };
        assert_eq!(c.resolve_ref(&r), Resolved::Ambiguous);
        assert_eq!(c.resolve(&r), None);
    }

    #[test]
    fn a_reference_with_a_real_marker_that_missed_does_not_fall_back_to_free_text() {
        let text = r#"{"mods":[{"name":"Some Mod","source":{"type":"nexus","md5":"aaaa"}}]}"#;
        let c = read(text).unwrap().collection;
        let r = ModReference {
            file_md5: "bbbb".into(),
            description: "Some Mod".into(),
            ..ModReference::default()
        };
        assert_eq!(c.resolve(&r), None, "an md5 that missed is evidence");
        // With no identity marker at all, the display name may still stand in.
        let only_text = ModReference {
            description: "Some Mod".into(),
            ..ModReference::default()
        };
        assert_eq!(c.resolve(&only_text), Some(0));
    }

    #[test]
    fn a_padded_name_still_finds_its_member() {
        let text = r#"{"mods":[{"name":"Some Mod ","source":{"type":"nexus"}}]}"#;
        let c = read(text).unwrap().collection;
        let r = ModReference {
            file_expression: " some mod".into(),
            ..ModReference::default()
        };
        assert_eq!(c.resolve(&r), Some(0));
    }

    #[test]
    fn an_unknown_key_inside_a_member_is_reported_like_a_top_level_one() {
        let text = r#"{"mods":[
          {"name":"A","source":{"type":"nexus","somethingNew":1},"anotherThing":2},
          {"name":"B","source":{"type":"nexus"},"anotherThing":3}
        ],"info":{"name":"C","aNewInfoKey":1}}"#;
        let r = read(text).unwrap();
        assert_eq!(
            r.unknown_sections,
            vec![
                "info.aNewInfoKey".to_string(),
                "mods[].anotherThing".to_string(),
                "mods[].source.somethingNew".to_string(),
            ],
            "deduplicated, so one new key is one line however many members have it"
        );
    }
}
