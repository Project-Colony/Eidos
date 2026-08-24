//! Nexus Collections: reading a published collection's member list.
//!
//! # What this does, and what it deliberately does not
//!
//! It resolves a collection link to its list of mods and says, for each one,
//! whether the instance already has it. It does NOT install a collection, and
//! that is a decision rather than an unfinished edge.
//!
//! Four things make an installer dishonest here rather than merely hard:
//!
//! * **Download gating.** A collection's members are ordinary mod files. Without
//!   a per-file key from the site's own button, only a premium account can mint
//!   a download link - so for everyone else a sixty-mod collection is sixty
//!   browser clicks whatever this code does. Vortex closes that with an embedded
//!   browser; Eidos has none, and a progress bar that stalls on mod one is worse
//!   than an honest list.
//! * **The request budget.** A full install is three v1 calls per member. A
//!   hundred-mod collection is three hundred requests against an hourly budget,
//!   and this client refuses as soon as either counter is spent - so an installer
//!   would stop halfway and leave a part-built mod list.
//! * **The manifest.** Install phases, mod rules, replayed FOMOD answers, binary
//!   patches and LOOT rules all live in the collection archive, and none of their
//!   semantics could be verified against a real published Bethesda collection.
//!   Guessing produces a load order that looks right and is not, which is the
//!   worst failure a mod manager has.
//! * **The adult gate.** This is a SECOND door into mod metadata, and the v1
//!   gate cannot see through it - so it gets its own, below.
//!
//! Reading costs one request and is exact. That is what is built.

use crate::{AdultPolicy, HiddenReason, NxmCollection};

/// One member of a collection revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionMod {
    pub name: String,
    pub mod_id: u64,
    pub file_id: u64,
    /// The member's OWN game domain. A collection can pull a mod from another
    /// game's page (Skyrim LE assets used by an SE collection), so this is not
    /// necessarily the collection's domain.
    pub domain: String,
    pub version: String,
    pub file_name: String,
    pub size_in_bytes: u64,
    /// Optional members are offered by the collection, not required by it.
    pub optional: bool,
}

/// A published collection revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionRevision {
    pub slug: String,
    pub revision_number: u32,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub game_domain: String,
    pub mod_count: u32,
    pub total_size: u64,
    /// The collection author's own installation notes, if any.
    pub instructions: String,
    pub mods: Vec<CollectionMod>,
    /// Set when the revision's metadata is withheld - see [`gate`].
    pub hidden: Option<HiddenReason>,
}

impl CollectionRevision {
    /// Whether the metadata may be shown.
    pub fn visible(&self) -> bool {
        self.hidden.is_none()
    }
}

/// The GraphQL document. Verified field-by-field against live introspection of
/// `https://api.nexusmods.com/v2/graphql`.
///
/// `revision: null` is how the API spells "latest" - not a missing argument and
/// not a sentinel number.
const QUERY: &str = r#"
query collectionRevision($slug: String, $revision: Int, $viewAdultContent: Boolean, $domainName: String) {
  collectionRevision(slug: $slug, revision: $revision, viewAdultContent: $viewAdultContent, domainName: $domainName) {
    revisionNumber
    adultContent
    modCount
    totalSize
    installationInfo
    collection { slug name summary user { name } game { domainName } }
    modFiles {
      fileId
      optional
      version
      file {
        name
        version
        sizeInBytes
        mod { modId name game { domainName } }
      }
    }
  }
}"#;

/// Decide whether a revision's metadata may be shown.
///
/// The v1 gate lives in `RemoteMod::from_payload` and cannot help here: this is a
/// second door into the same kind of data, and the revision carries only ONE
/// rating for the whole collection - there is no per-member flag anywhere in the
/// schema. So a collection marked adult withholds every member's name, not some
/// of them.
///
/// Fail closed at every step, exactly as the v1 gate does: a missing rating is
/// treated as adult, and an unknown account preference hides rather than shows.
/// Every way this can be wrong ends with too little on screen.
pub(crate) fn gate(adult: Option<bool>, policy: AdultPolicy) -> Option<HiddenReason> {
    match (adult, policy) {
        (Some(false), _) => None,
        (Some(true), AdultPolicy::Allowed) => None,
        (Some(true), AdultPolicy::Denied) => Some(HiddenReason::AdultDenied),
        (Some(true), AdultPolicy::Unknown) => Some(HiddenReason::AdultUnknown),
        (None, _) => Some(HiddenReason::RatingUnknown),
    }
}

/// Build the request body for a collection link.
pub(crate) fn query_body(c: &NxmCollection, view_adult: bool) -> serde_json::Value {
    serde_json::json!({
        "query": QUERY,
        "variables": {
            "slug": c.slug,
            // `null` when the link said `latest`.
            "revision": c.revision,
            "viewAdultContent": view_adult,
            "domainName": c.game,
        }
    })
}

/// Parse the reply. Split from the request so it can be tested against a real
/// captured payload without a network.
pub(crate) fn from_payload(
    v: &serde_json::Value,
    fallback_slug: &str,
    policy: AdultPolicy,
) -> Result<CollectionRevision, String> {
    let rev = v
        .get("data")
        .and_then(|d| d.get("collectionRevision"))
        .filter(|r| !r.is_null())
        .ok_or_else(|| "Nexus returned no such collection revision".to_string())?;

    let adult = rev.get("adultContent").and_then(serde_json::Value::as_bool);
    let hidden = gate(adult, policy);
    let redact = hidden.is_some();

    let coll = rev.get("collection");
    let str_at = |v: Option<&serde_json::Value>, k: &str| -> String {
        v.and_then(|v| v.get(k)).and_then(|x| x.as_str()).unwrap_or_default().to_string()
    };

    let mut mods = Vec::new();
    if let Some(list) = rev.get("modFiles").and_then(|m| m.as_array()) {
        for m in list {
            let file = m.get("file");
            let inner = file.and_then(|f| f.get("mod"));
            let Some(mod_id) = inner.and_then(|x| x.get("modId")).and_then(serde_json::Value::as_u64)
            else {
                // A member whose mod was deleted comes back with a null `file`.
                // Skipped rather than shown as a zero: a row that cannot be acted
                // on is worse than a member count that does not add up, and the
                // count is reported separately from the API anyway.
                continue;
            };
            let Some(file_id) = m.get("fileId").and_then(serde_json::Value::as_u64) else { continue };
            mods.push(CollectionMod {
                // Redacted with the rest: a withheld collection must not leak
                // its members' names, which is the whole point of the gate.
                name: if redact { String::new() } else { str_at(inner, "name") },
                mod_id,
                file_id,
                domain: str_at(inner.and_then(|i| i.get("game")), "domainName"),
                // Kept either way: a version identifies nothing on its own and
                // is what tells the user their installed copy is behind.
                version: {
                    let v = str_at(Some(m), "version");
                    if v.is_empty() { str_at(file, "version") } else { v }
                },
                file_name: if redact { String::new() } else { str_at(file, "name") },
                size_in_bytes: file
                    .and_then(|f| f.get("sizeInBytes"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                optional: m.get("optional").and_then(serde_json::Value::as_bool).unwrap_or(false),
            });
        }
    }

    Ok(CollectionRevision {
        slug: {
            let s = str_at(coll, "slug");
            if s.is_empty() { fallback_slug.to_string() } else { s }
        },
        revision_number: rev
            .get("revisionNumber")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32,
        name: if redact { String::new() } else { str_at(coll, "name") },
        summary: if redact { String::new() } else { str_at(coll, "summary") },
        author: if redact {
            String::new()
        } else {
            str_at(coll.and_then(|c| c.get("user")), "name")
        },
        game_domain: str_at(coll.and_then(|c| c.get("game")), "domainName"),
        mod_count: rev.get("modCount").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32,
        total_size: rev.get("totalSize").and_then(serde_json::Value::as_u64).unwrap_or(0),
        instructions: if redact { String::new() } else { str_at(Some(rev), "installationInfo") },
        mods,
        hidden,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A REAL reply, captured from `api.nexusmods.com/v2/graphql` for the public
    /// collection `rqhcxy` (The Great Cities Collection, Skyrim SE), trimmed to
    /// four members plus one synthesised deleted member.
    ///
    /// Captured rather than hand-written on purpose: every field name here was
    /// verified by introspection, and a fixture somebody typed would drift from
    /// the wire the first time Nexus renamed anything.
    const REAL: &str = include_str!("../tests/data/collection_revision.json");

    fn parse(policy: AdultPolicy) -> CollectionRevision {
        let v: serde_json::Value = serde_json::from_str(REAL).unwrap();
        from_payload(&v, "rqhcxy", policy).unwrap()
    }

    #[test]
    fn a_real_reply_parses_into_the_member_list() {
        let r = parse(AdultPolicy::Allowed);
        assert_eq!(r.slug, "rqhcxy");
        assert_eq!(r.revision_number, 1);
        assert_eq!(r.name, "The Great Cities Collection");
        assert_eq!(r.author, "HookerHeels");
        assert_eq!(r.game_domain, "skyrimspecialedition");
        assert_eq!(r.mod_count, 12, "what the API says the collection holds");
        assert!(r.visible());

        // Four real members; the fifth had a null `file` - a mod that has since
        // been deleted - and is dropped rather than shown as a row nothing can
        // act on. `mod_count` still reports what the collection contains, so the
        // two numbers disagreeing is information, not a bug.
        assert_eq!(r.mods.len(), 4, "{:?}", r.mods);
        let first = &r.mods[0];
        assert_eq!(first.mod_id, 37471);
        assert_eq!(first.file_id, 232153);
        assert_eq!(first.name, "The Great Town of Karthwasten Patch Collection");
        assert_eq!(first.version, "2.1");
        assert_eq!(first.domain, "skyrimspecialedition");
        assert!(!first.optional);
    }

    #[test]
    fn an_adult_collection_withholds_its_members_names_not_just_its_own() {
        let v: serde_json::Value = serde_json::from_str(REAL).unwrap();
        let mut adult = v.clone();
        adult["data"]["collectionRevision"]["adultContent"] = serde_json::json!(true);

        let r = from_payload(&adult, "rqhcxy", AdultPolicy::Denied).unwrap();
        assert_eq!(r.hidden, Some(HiddenReason::AdultDenied));
        assert_eq!(r.name, "");
        assert_eq!(r.author, "");
        assert_eq!(r.instructions, "");
        // The whole point: the revision carries ONE rating for the collection,
        // so withholding only its title while listing every mod inside it by
        // name would defeat the gate through the door it was built for.
        assert!(r.mods.iter().all(|m| m.name.is_empty() && m.file_name.is_empty()));
        // Ids and versions survive - they describe nothing on their own, and
        // they are what lets the list still say "you already have this one".
        assert_eq!(r.mods[0].mod_id, 37471);
        assert_eq!(r.mods[0].version, "2.1");

        // Allowed sees it.
        let ok = from_payload(&adult, "rqhcxy", AdultPolicy::Allowed).unwrap();
        assert!(ok.visible());
        assert!(!ok.mods[0].name.is_empty());
    }

    #[test]
    fn a_rating_that_is_absent_is_treated_as_adult() {
        let v: serde_json::Value = serde_json::from_str(REAL).unwrap();
        let mut unknown = v.clone();
        unknown["data"]["collectionRevision"]["adultContent"] = serde_json::Value::Null;
        // Fail closed, exactly as the v1 gate does: every way this can be wrong
        // has to end with too little on screen, never too much.
        for policy in [AdultPolicy::Allowed, AdultPolicy::Denied, AdultPolicy::Unknown] {
            let r = from_payload(&unknown, "rqhcxy", policy).unwrap();
            assert_eq!(r.hidden, Some(HiddenReason::RatingUnknown), "{policy:?}");
        }
        // And an unknown ACCOUNT preference hides an adult collection too.
        assert_eq!(gate(Some(true), AdultPolicy::Unknown), Some(HiddenReason::AdultUnknown));
    }

    #[test]
    fn a_reply_with_no_such_revision_is_an_error_not_an_empty_collection() {
        let v = serde_json::json!({ "data": { "collectionRevision": null } });
        assert!(from_payload(&v, "nope", AdultPolicy::Allowed).is_err());
        let v = serde_json::json!({ "data": {} });
        assert!(from_payload(&v, "nope", AdultPolicy::Allowed).is_err());
    }

    #[test]
    fn latest_is_sent_as_a_null_revision() {
        let c = NxmCollection {
            game: "skyrimspecialedition".into(),
            slug: "rqhcxy".into(),
            revision: None,
        };
        let b = query_body(&c, false);
        // Not a sentinel number and not an omitted key: `revision: null` is how
        // the API itself spells "the latest published revision".
        assert!(b["variables"]["revision"].is_null());
        assert_eq!(b["variables"]["slug"], "rqhcxy");
        assert_eq!(b["variables"]["domainName"], "skyrimspecialedition");
        assert_eq!(b["variables"]["viewAdultContent"], false);

        let c2 = NxmCollection { revision: Some(3), ..c };
        assert_eq!(query_body(&c2, true)["variables"]["revision"], 3);
    }
}
