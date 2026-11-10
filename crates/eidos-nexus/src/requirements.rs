//! What a Nexus mod page says the mod needs: other mods and DLC.
//!
//! One GraphQL request covers up to 50 mods of a domain. The answer is cached in
//! each mod's `meta.ini` so Health can read it offline.

use crate::AdultPolicy;
use serde::{Deserialize, Serialize};

/// A mod the page lists as required. An off-site requirement (a runtime, a
/// tool) has `mod_id` 0 and an empty `domain`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    pub domain: String,
    pub mod_id: u64,
    pub name: String,
    pub notes: String,
}

/// Everything one mod page requires. `dlc` entries read `Name` or `Name: notes`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirements {
    pub mods: Vec<Requirement>,
    pub dlc: Vec<String>,
}

const QUERY: &str = r#"
query modRequirements($ids: [CompositeDomainWithIdInput!]!, $count: Int) {
  legacyModsByDomain(ids: $ids, count: $count) {
    nodes {
      modId
      gameId
      game { domainName }
      modRequirements {
        nexusRequirements(count: 20) { nodes { modId gameId modName notes url externalRequirement } }
        dlcRequirements { gameExpansion { name } notes }
      }
    }
  }
}
"#;

pub(crate) fn query_body(domain: &str, ids: &[u64]) -> serde_json::Value {
    serde_json::json!({
        "query": QUERY,
        "variables": {
            "ids": ids.iter().map(|id| serde_json::json!({"gameDomain": domain, "modId": id})).collect::<Vec<_>>(),
            "count": ids.len(),
        }
    })
}

fn text(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// The domain in a `nexusmods.com/<domain>/mods/<id>` link.
fn domain_of_url(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let rest = rest
        .strip_prefix("www.nexusmods.com/")
        .or_else(|| rest.strip_prefix("nexusmods.com/"))?;
    let seg = rest.split('/').next().unwrap_or_default();
    (!seg.is_empty()).then(|| seg.to_ascii_lowercase())
}

/// Parse the reply, one entry per mod Nexus knows. Requirement nodes carry no
/// rating, so unless adult content is allowed every name is withheld.
pub(crate) fn from_payload(
    v: &serde_json::Value,
    policy: AdultPolicy,
) -> Result<Vec<(u64, Requirements)>, String> {
    let nodes = v
        .get("data")
        .and_then(|d| d.get("legacyModsByDomain"))
        .and_then(|m| m.get("nodes"))
        .and_then(|n| n.as_array())
        .ok_or_else(|| "Nexus returned no requirement data".to_string())?;
    let show = policy.shows_adult();
    let mut out = Vec::new();
    for node in nodes.iter().filter(|n| !n.is_null()) {
        let Some(id) = node.get("modId").and_then(|x| x.as_u64()) else {
            continue;
        };
        let own_domain = node
            .get("game")
            .map(|g| text(g, "domainName"))
            .unwrap_or_default();
        let own_game = node.get("gameId").and_then(|x| x.as_u64());
        let reqs = node.get("modRequirements");
        let mut r = Requirements::default();
        let listed = reqs
            .and_then(|q| q.get("nexusRequirements"))
            .and_then(|q| q.get("nodes"))
            .and_then(|n| n.as_array());
        for n in listed.into_iter().flatten() {
            let mod_id = n.get("modId").and_then(|x| x.as_u64()).unwrap_or(0);
            let external = n.get("externalRequirement").and_then(|x| x.as_bool()) == Some(true);
            let name = if show {
                text(n, "modName")
            } else {
                String::new()
            };
            let notes = text(n, "notes");
            if external || mod_id == 0 {
                r.mods.push(Requirement {
                    domain: String::new(),
                    mod_id: 0,
                    name,
                    notes,
                });
                continue;
            }
            let same_game =
                own_game.is_some() && n.get("gameId").and_then(|x| x.as_u64()) == own_game;
            let domain = domain_of_url(&text(n, "url")).or_else(|| {
                same_game
                    .then(|| own_domain.clone())
                    .filter(|d| !d.is_empty())
            });
            if let Some(domain) = domain {
                r.mods.push(Requirement {
                    domain,
                    mod_id,
                    name,
                    notes,
                });
            }
        }
        let dlc = reqs
            .and_then(|q| q.get("dlcRequirements"))
            .and_then(|d| d.as_array());
        for d in dlc.into_iter().flatten() {
            let name = d
                .get("gameExpansion")
                .map(|e| text(e, "name"))
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let notes = text(d, "notes");
            r.dlc.push(if notes.is_empty() {
                name
            } else {
                format!("{name}: {notes}")
            });
        }
        out.push((id, r));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../tests/data/mod_requirements.json")).unwrap()
    }

    #[test]
    fn requirements_parse_nexus_dlc_and_off_site() {
        let r = from_payload(&fixture(), AdultPolicy::Allowed).unwrap();
        let (id, reqs) = &r[0];
        assert_eq!(*id, 100);
        assert_eq!(
            reqs.mods
                .iter()
                .map(|m| (m.domain.as_str(), m.mod_id))
                .collect::<Vec<_>>(),
            [("skyrimspecialedition", 12604), ("skyrim", 99), ("", 0)]
        );
        assert_eq!(reqs.mods[2].name, "VC++ Redistributable");
        assert_eq!(reqs.dlc, ["Dawnguard: for the patch"]);
        assert_eq!(r[1].1, Requirements::default());
    }

    #[test]
    fn requirement_names_are_withheld_unless_adult_content_is_allowed() {
        for policy in [AdultPolicy::Denied, AdultPolicy::Unknown] {
            let r = from_payload(&fixture(), policy).unwrap();
            assert!(r[0].1.mods.iter().all(|m| m.name.is_empty()));
            assert!(r[0]
                .1
                .mods
                .iter()
                .filter(|m| !m.domain.is_empty())
                .all(|m| m.mod_id != 0));
        }
    }

    #[test]
    fn a_node_without_a_link_takes_the_requiring_mods_domain_or_is_dropped() {
        let mut v = fixture();
        let nodes = &mut v["data"]["legacyModsByDomain"]["nodes"][0]["modRequirements"]
            ["nexusRequirements"]["nodes"];
        *nodes = serde_json::json!([
            {"modId": 5, "gameId": 1704, "modName": "A", "url": ""},
            {"modId": 6, "gameId": 110, "modName": "B", "url": ""}
        ]);
        let r = from_payload(&v, AdultPolicy::Allowed).unwrap();
        assert_eq!(r[0].1.mods.len(), 1);
        assert_eq!(r[0].1.mods[0].domain, "skyrimspecialedition");
    }

    #[test]
    fn query_body_batches_domain_and_ids() {
        let b = query_body("skyrimspecialedition", &[1, 2]);
        assert_eq!(b["variables"]["ids"][1]["modId"], 2);
        assert_eq!(
            b["variables"]["ids"][0]["gameDomain"],
            "skyrimspecialedition"
        );
        assert_eq!(b["variables"]["count"], 2);
    }
}
