//! The parser against a manifest Nexus actually published.
//!
//! `tests/data/great-cities.json` is the `collection.json` out of the archive
//! for `rqhcxy` revision 1 (The Great Cities Collection, Skyrim SE), fetched
//! from the live API. Not for its contents - those will change - but because a
//! format with no version field and no closed key set can only be checked
//! against something real. Every earlier assumption about this file that was
//! made from the writer code alone turned out to need this to settle it.

use eidos_collections::{read, SourceType};

const REAL: &str = include_str!("data/great-cities.json");

#[test]
fn a_published_manifest_parses() {
    let r = read(REAL).expect("a real collection.json must read");
    let c = &r.collection;
    assert_eq!(c.info.name, "The Great Cities Collection");
    assert_eq!(c.info.domain_name, "skyrimspecialedition");
    assert_eq!(c.mods.len(), 12);
    assert!(c.mods.iter().all(|m| m.source.kind == SourceType::Nexus));
    assert!(c.mods.iter().all(|m| m.source.mod_id.is_some()));
    assert!(c.mods.iter().all(|m| m.source.file_id.is_some()));
}

#[test]
fn nothing_in_a_published_manifest_is_unrecognised() {
    // The point of the not-understood line is that it stays EMPTY on ordinary
    // collections; a report that always says "tools" trains people to ignore it.
    let r = read(REAL).unwrap();
    assert!(
        r.unknown_sections.is_empty(),
        "unexpected sections: {:?}",
        r.unknown_sections
    );
}

#[test]
fn the_fields_the_writer_code_could_not_settle() {
    let c = read(REAL).unwrap().collection;
    // `info` here carries neither `installInstructions` nor `gameVersions`,
    // both of which the TypeScript types declare REQUIRED. Optional it is.
    assert!(c.info.install_instructions.is_empty());
    assert!(c.info.game_versions.is_empty());
    // `fileSize` is BYTES. Nexus's own OpenAPI description says kilobytes; the
    // API reports `sizeInBytes` for this same file as the identical number.
    let k = c
        .mods
        .iter()
        .find(|m| m.source.md5 == "ad453eedf3660139782bb2e8f80e32aa")
        .expect("the member the API was cross-checked against");
    assert_eq!(k.source.file_size, Some(3_609_261));
    assert_eq!(k.source.logical_filename, "The Great Town of Karthwasten Patch Collection");
}

#[test]
fn a_collection_may_carry_no_rules_at_all() {
    // This one does not: no modRules, and empty pluginRules. A reader that
    // required them would refuse a perfectly ordinary published collection.
    let c = read(REAL).unwrap().collection;
    assert!(c.mod_rules.is_empty());
    assert!(c.plugin_rules.plugins.is_empty());
    assert_eq!(c.plugins.len(), 11);
    assert!(c.plugins.iter().all(|p| p.enabled));
    // And every member sits in a phase, so the install order is well defined.
    assert_eq!(c.phases(), vec![0]);
}
