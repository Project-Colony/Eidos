//! Turning a collection's `modRules` into a deployment order.
//!
//! This is the rule that decides which mod wins a shared file, and it is a
//! different question from both of the other two orderings a collection carries.
//! `phase` says what installs first. `pluginRules` says what LOADS first, and is
//! LOOT's business. These say what DEPLOYS over what - in Eidos, the order of
//! the union mount's layers, which is the mod list's own order.
//!
//! Eidos's mod list is MO2's display order: index 0 is the top and the LOWEST
//! priority, the last entry wins. So a rule "A before B" puts A EARLIER in the
//! list than B, and B wins the files they share.

use crate::manifest::{Collection, ModReference, RuleType};

/// One constraint over member indices: `earlier` sits before `later`, so `later`
/// wins the files they share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub earlier: usize,
    pub later: usize,
}

/// A rule that could not be turned into a constraint, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    /// The rule's own words, as close to what the author wrote as this crate
    /// can render: enough for a person to find it in the collection.
    pub rule: String,
    pub why: &'static str,
}

/// Name a reference the way a report can print it.
fn describe(r: &ModReference) -> String {
    for candidate in [
        &r.description,
        &r.logical_file_name,
        &r.file_expression,
        &r.file_md5,
    ] {
        if !candidate.is_empty() {
            return candidate.clone();
        }
    }
    "an unnamed mod".to_string()
}

/// The ordering constraints a collection asks for, and the rules that produced
/// none.
///
/// Only `before` and `after` order anything. `requires`, `conflicts`,
/// `recommends` and `provides` are real parts of the vocabulary and say nothing
/// about deployment order, so they are skipped SILENTLY rather than reported -
/// a report full of "ignored: recommends" would bury the rules that genuinely
/// could not be applied.
pub fn edges(c: &Collection) -> (Vec<Edge>, Vec<Unresolved>) {
    let mut edges = Vec::new();
    let mut lost = Vec::new();
    for rule in &c.mod_rules {
        let ordering = match rule.kind {
            RuleType::Before | RuleType::After => rule.kind,
            _ => continue,
        };
        let name = |k: &str| format!("{} {k} {}", describe(&rule.source), describe(&rule.reference));
        let (Some(a), Some(b)) = (c.resolve(&rule.source), c.resolve(&rule.reference)) else {
            lost.push(Unresolved {
                rule: name(if ordering == RuleType::Before {
                    "before"
                } else {
                    "after"
                }),
                why: "neither end of it names a mod this collection contains",
            });
            continue;
        };
        if a == b {
            lost.push(Unresolved {
                rule: name("before/after"),
                why: "both ends of it resolve to the same mod",
            });
            continue;
        }
        edges.push(match ordering {
            // "source before reference": source deploys first, reference wins.
            RuleType::Before => Edge {
                earlier: a,
                later: b,
            },
            _ => Edge {
                earlier: b,
                later: a,
            },
        });
    }
    edges.sort_unstable();
    edges.dedup();
    (edges, lost)
}

/// A resolved deployment order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Order {
    /// Member indices, first to last, i.e. lowest priority to highest.
    pub sequence: Vec<usize>,
    /// Members caught in a cycle of rules, in their original order.
    ///
    /// Reported rather than resolved. A cycle means the collection asks for
    /// something impossible, and any order this code invented would be a guess
    /// presented as the author's intent. They keep their original relative
    /// order and the user is told which ones.
    pub cycles: Vec<usize>,
}

/// Order `count` members so every edge is satisfied, keeping the collection's
/// own order wherever the rules do not care.
///
/// Kahn's algorithm with a stable tiebreak: among the members that could come
/// next, the one the collection listed first goes first. Without that tiebreak
/// the order would depend on hash iteration and a re-run could reshuffle mods
/// the rules said nothing about.
pub fn order(count: usize, edges: &[Edge]) -> Order {
    let mut incoming = vec![0usize; count];
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); count];
    for e in edges {
        if e.earlier >= count || e.later >= count {
            continue;
        }
        out[e.earlier].push(e.later);
        incoming[e.later] += 1;
    }
    let mut ready: Vec<usize> = (0..count).filter(|i| incoming[*i] == 0).collect();
    let mut sequence = Vec::with_capacity(count);
    while !ready.is_empty() {
        // Lowest original index first: the stable tiebreak.
        ready.sort_unstable();
        let n = ready.remove(0);
        sequence.push(n);
        for &m in &out[n] {
            incoming[m] -= 1;
            if incoming[m] == 0 {
                ready.push(m);
            }
        }
    }
    // Whatever still has an incoming edge is in a cycle, or downstream of one.
    let cycles: Vec<usize> = (0..count).filter(|i| !sequence.contains(i)).collect();
    // They still have to go somewhere, and their own order is the only
    // defensible one left.
    sequence.extend(cycles.iter().copied());
    Order { sequence, cycles }
}

/// Rewrite an existing mod list so the collection's members sit in `sequence`,
/// leaving every other mod exactly where it was.
///
/// A MERGE, not an overwrite, and that is the whole point: the user already has
/// mods this collection has never heard of, and their positions are the user's
/// decision. Only the slots the members already occupy are refilled.
///
/// `members` maps member index -> the mod-list entry it installed as. A member
/// with no entry (not installed, skipped, failed) simply takes part in nothing.
pub fn merge(list: &[String], members: &[(usize, String)], sequence: &[usize]) -> Vec<String> {
    let mut slots: Vec<usize> = Vec::new();
    for (pos, name) in list.iter().enumerate() {
        if members.iter().any(|(_, n)| n == name) {
            slots.push(pos);
        }
    }
    // The members that are actually in the list, in the order the rules asked
    // for. A member the rules ordered but that is not installed is skipped here
    // rather than inserted: this function reorders, it never adds.
    let ordered: Vec<&String> = sequence
        .iter()
        .filter_map(|i| members.iter().find(|(mi, _)| mi == i))
        .map(|(_, n)| n)
        .filter(|n| list.contains(n))
        .collect();
    let mut out = list.to_vec();
    for (slot, name) in slots.iter().zip(ordered) {
        out[*slot] = name.clone();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Mod, ModRule, Source};

    fn collection(names: &[&str]) -> Collection {
        Collection {
            mods: names
                .iter()
                .map(|n| Mod {
                    name: (*n).to_string(),
                    source: Source::default(),
                    ..Mod::default()
                })
                .collect(),
            ..Collection::default()
        }
    }

    fn by_name(n: &str) -> ModReference {
        ModReference {
            file_expression: n.to_string(),
            ..ModReference::default()
        }
    }

    fn rule(a: &str, kind: RuleType, b: &str) -> ModRule {
        ModRule {
            source: by_name(a),
            kind,
            reference: by_name(b),
        }
    }

    #[test]
    fn before_puts_the_source_earlier_so_the_reference_wins() {
        // Eidos's list is display order: later in the list wins a file conflict.
        // "A before B" therefore means A sits earlier and B wins.
        let mut c = collection(&["A", "B"]);
        c.mod_rules = vec![rule("A", RuleType::Before, "B")];
        let (e, lost) = edges(&c);
        assert!(lost.is_empty());
        assert_eq!(
            e,
            vec![Edge {
                earlier: 0,
                later: 1
            }]
        );
        assert_eq!(order(2, &e).sequence, vec![0, 1]);
    }

    #[test]
    fn after_is_the_same_edge_the_other_way_round() {
        let mut c = collection(&["A", "B"]);
        c.mod_rules = vec![rule("A", RuleType::After, "B")];
        let (e, _) = edges(&c);
        assert_eq!(
            e,
            vec![Edge {
                earlier: 1,
                later: 0
            }],
            "A after B means B deploys first and A wins"
        );
        assert_eq!(order(2, &e).sequence, vec![1, 0]);
    }

    #[test]
    fn rules_that_are_not_about_order_are_skipped_without_noise() {
        // They are a real part of the vocabulary and say nothing about
        // deployment. Reporting them would bury the rules that genuinely failed.
        let mut c = collection(&["A", "B"]);
        c.mod_rules = vec![
            rule("A", RuleType::Requires, "B"),
            rule("A", RuleType::Conflicts, "B"),
            rule("A", RuleType::Recommends, "B"),
            rule("A", RuleType::Provides, "B"),
        ];
        let (e, lost) = edges(&c);
        assert!(e.is_empty());
        assert!(lost.is_empty(), "{lost:?}");
    }

    #[test]
    fn a_rule_naming_a_mod_the_collection_does_not_contain_is_reported() {
        let mut c = collection(&["A"]);
        c.mod_rules = vec![rule("A", RuleType::Before, "Somebody Else's Mod")];
        let (e, lost) = edges(&c);
        assert!(e.is_empty());
        assert_eq!(lost.len(), 1);
        assert!(lost[0].rule.contains("Somebody Else's Mod"), "{lost:?}");
    }

    #[test]
    fn the_order_the_collection_listed_is_kept_where_the_rules_do_not_care() {
        // Without a stable tiebreak the answer would depend on iteration order
        // and a re-run could reshuffle mods nothing had an opinion about.
        let e = [Edge {
            earlier: 3,
            later: 0,
        }];
        assert_eq!(order(5, &e).sequence, vec![1, 2, 3, 0, 4]);
        assert!(order(5, &e).cycles.is_empty());
    }

    #[test]
    fn a_cycle_is_reported_rather_than_broken_by_guesswork() {
        // The collection is asking for something impossible. Inventing an order
        // would present a guess as the author's intent.
        let e = [
            Edge {
                earlier: 0,
                later: 1,
            },
            Edge {
                earlier: 1,
                later: 2,
            },
            Edge {
                earlier: 2,
                later: 0,
            },
        ];
        let o = order(3, &e);
        assert_eq!(o.cycles, vec![0, 1, 2]);
        // They still all appear exactly once, in their original relative order.
        let mut seen = o.sequence.clone();
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 1, 2]);
        assert_eq!(o.sequence, vec![0, 1, 2]);
    }

    #[test]
    fn merging_moves_only_the_members_and_leaves_the_users_own_mods_alone() {
        // The user has mods the collection has never heard of, and where they
        // sit is the user's decision.
        let list: Vec<String> = ["mine-a", "B", "mine-b", "A", "mine-c"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let members = [(0usize, "A".to_string()), (1usize, "B".to_string())];
        // The rules want A before B; today the list has B first.
        let out = merge(&list, &members, &[0, 1]);
        assert_eq!(out, vec!["mine-a", "A", "mine-b", "B", "mine-c"]);
        // Every one of the user's mods kept its index.
        assert_eq!(out[0], list[0]);
        assert_eq!(out[2], list[2]);
        assert_eq!(out[4], list[4]);
    }

    #[test]
    fn merging_never_adds_a_member_that_is_not_installed() {
        // A member that failed, was skipped, or was optional and declined takes
        // part in nothing. This function reorders; it does not install.
        let list: Vec<String> = ["A", "mine"].iter().map(|s| s.to_string()).collect();
        let members = [(0usize, "A".to_string()), (1usize, "Never Installed".into())];
        assert_eq!(merge(&list, &members, &[1, 0]), list);
    }
}
