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

use std::collections::HashMap;

use crate::manifest::{Collection, ModReference, Resolved, RuleType};

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
        &r.tag,
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
        let name = |k: &str| {
            format!(
                "{} {k} {}",
                describe(&rule.source),
                describe(&rule.reference)
            )
        };
        let word = if ordering == RuleType::Before {
            "before"
        } else {
            "after"
        };
        let ends = (c.resolve_ref(&rule.source), c.resolve_ref(&rule.reference));
        if matches!(ends.0, Resolved::Ambiguous) || matches!(ends.1, Resolved::Ambiguous) {
            lost.push(Unresolved {
                rule: name(word),
                why: "more than one member carries the marker it names, so there is no \
                      telling which one it means",
            });
            continue;
        }
        let (Resolved::One(a), Resolved::One(b)) = ends else {
            lost.push(Unresolved {
                rule: name(word),
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
    order_ranked(
        count,
        edges,
        &(0..count).map(|i| (i, i)).collect::<Vec<_>>(),
    )
}

/// [`order`] with the tiebreak given: among the members that could come next,
/// the one with the lowest `rank` goes first, and cycles are listed by rank.
fn order_ranked(count: usize, edges: &[Edge], rank: &[(usize, usize)]) -> Order {
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
        // Lowest rank first: the stable tiebreak.
        ready.sort_unstable_by_key(|n| rank[*n]);
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
    let mut cycles: Vec<usize> = (0..count).filter(|i| !sequence.contains(i)).collect();
    cycles.sort_unstable_by_key(|n| rank[*n]);
    // They still have to go somewhere, and their own order is the only
    // defensible one left.
    sequence.extend(cycles.iter().copied());
    Order { sequence, cycles }
}

/// A collection's before/after rules over the folders its members installed
/// as, ready to check a mod list against or restore one.
///
/// Two members can install into one folder (see [`merge`]), and a folder holds
/// one place in the list, so the rules are ordered folder by folder: a rule
/// between two members of one folder orders nothing, and rules that put one
/// folder on both sides of another form a loop. A member that has no folder
/// keeps a node of its own, so rules still chain through it.
#[derive(Debug, Clone, Default)]
pub struct FolderRules {
    /// Node -> the folder it stands for, `None` for a member with no folder.
    folders: Vec<Option<String>>,
    edges: Vec<Edge>,
    /// Nodes no order can place: in a loop of rules, or after one.
    stuck: Vec<bool>,
}

impl FolderRules {
    /// `folders` maps member index -> installed folder.
    pub fn new(c: &Collection, folders: &[(usize, String)]) -> Self {
        let by_member: HashMap<usize, &String> = folders.iter().map(|(m, f)| (*m, f)).collect();
        let mut nodes: Vec<Option<String>> = Vec::new();
        let mut by_folder: HashMap<&String, usize> = HashMap::new();
        let node: Vec<usize> = (0..c.mods.len())
            .map(|i| match by_member.get(&i) {
                Some(f) => *by_folder.entry(f).or_insert_with(|| {
                    nodes.push(Some((*f).clone()));
                    nodes.len() - 1
                }),
                None => {
                    nodes.push(None);
                    nodes.len() - 1
                }
            })
            .collect();
        let mut edges: Vec<Edge> = edges(c)
            .0
            .iter()
            .map(|e| Edge {
                earlier: node[e.earlier],
                later: node[e.later],
            })
            .filter(|e| e.earlier != e.later)
            .collect();
        edges.sort_unstable();
        edges.dedup();
        let mut stuck = vec![false; nodes.len()];
        for n in order(nodes.len(), &edges).cycles {
            stuck[n] = true;
        }
        FolderRules {
            folders: nodes,
            edges,
            stuck,
        }
    }

    /// The rules `enabled` breaks, as (earlier folder, later folder), in edge
    /// order. `enabled` is the enabled folder names, lowest priority first. A
    /// rule is left out when no order can keep it (a loop), or when an end has
    /// no folder or is off in this profile. [`FolderRules::restore`] fixes
    /// every pair this returns.
    pub fn broken_pairs(&self, enabled: &[String]) -> Vec<(String, String)> {
        let at: HashMap<&str, usize> = enabled
            .iter()
            .enumerate()
            .rev()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let mut out = Vec::new();
        for e in &self.edges {
            if self.stuck[e.earlier] || self.stuck[e.later] {
                continue;
            }
            let (Some(a), Some(b)) = (&self.folders[e.earlier], &self.folders[e.later]) else {
                continue;
            };
            let (Some(ia), Some(ib)) = (at.get(a.as_str()), at.get(b.as_str())) else {
                continue;
            };
            if ia > ib {
                out.push((a.clone(), b.clone()));
            }
        }
        out
    }

    /// `list` with the collection's folders re-sorted so every rule that can
    /// hold does. They keep the order `list` gives them wherever the rules
    /// allow, and every other entry stays where it is.
    pub fn restore(&self, list: &[String]) -> Vec<String> {
        let at: HashMap<&str, usize> = list
            .iter()
            .enumerate()
            .rev()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let rank: Vec<(usize, usize)> = self
            .folders
            .iter()
            .enumerate()
            .map(|(k, f)| {
                let pos = f.as_ref().and_then(|f| at.get(f.as_str()).copied());
                (pos.unwrap_or(list.len() + k), k)
            })
            .collect();
        let members: Vec<(usize, String)> = self
            .folders
            .iter()
            .enumerate()
            .filter_map(|(k, f)| Some((k, f.clone()?)))
            .collect();
        merge(
            list,
            &members,
            &order_ranked(self.folders.len(), &self.edges, &rank).sequence,
        )
    }
}

/// [`FolderRules::broken_pairs`] of `c` installed as `folders`.
pub fn broken_pairs(
    c: &Collection,
    folders: &[(usize, String)],
    enabled: &[String],
) -> Vec<(String, String)> {
    FolderRules::new(c, folders).broken_pairs(enabled)
}

/// [`FolderRules::restore`] of `c` installed as `folders`.
pub fn restore_order(c: &Collection, folders: &[(usize, String)], list: &[String]) -> Vec<String> {
    FolderRules::new(c, folders).restore(list)
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
    // Two members can install into the SAME folder - a collection that ships
    // two files of one mod does exactly that - and a folder can only occupy one
    // slot, so the second mention has to be dropped. Keeping it would write the
    // same name into two slots and silently lose whatever was in the other one.
    let mut seen: Vec<&String> = Vec::new();
    let ordered: Vec<&String> = sequence
        .iter()
        .filter_map(|i| members.iter().find(|(mi, _)| mi == i))
        .map(|(_, n)| n)
        .filter(|n| list.contains(n))
        .filter(|n| {
            if seen.contains(n) {
                false
            } else {
                seen.push(n);
                true
            }
        })
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
        let members = [
            (0usize, "A".to_string()),
            (1usize, "Never Installed".into()),
        ];
        assert_eq!(merge(&list, &members, &[1, 0]), list);
    }

    #[test]
    fn two_members_that_installed_into_one_folder_do_not_eat_a_third_mod() {
        // A collection with two files of the same mod resolves both to the same
        // folder. Placed twice, the same name lands in two slots and whatever
        // was in the second one is gone - and the list is still the right
        // LENGTH, so a count check would wave it through.
        let list: Vec<String> = ["Shared Folder", "Other Mod"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let members = [
            (0usize, "Shared Folder".to_string()),
            (1usize, "Shared Folder".to_string()),
            (2usize, "Other Mod".to_string()),
        ];
        assert_eq!(merge(&list, &members, &[0, 1, 2]), list);
    }

    fn ruled(names: &[&str], rules: &[(&str, &str)]) -> (Collection, Vec<(usize, String)>) {
        let mut c = collection(names);
        c.mod_rules = rules
            .iter()
            .map(|(a, b)| rule(a, RuleType::Before, b))
            .collect();
        (
            c,
            names
                .iter()
                .enumerate()
                .map(|(i, n)| (i, n.to_string()))
                .collect(),
        )
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn broken_pairs_name_the_rules_a_later_drag_broke() {
        let (c, folders) = ruled(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
        assert!(broken_pairs(&c, &folders, &s(&["A", "B", "C"])).is_empty());
        assert_eq!(
            broken_pairs(&c, &folders, &s(&["mine", "B", "A", "C"])),
            [("A".to_string(), "B".to_string())]
        );
    }

    #[test]
    fn broken_pairs_skip_cycles_and_members_that_are_off_or_missing() {
        let (c, folders) = ruled(&["A", "B", "C", "D"], &[("A", "B"), ("B", "A"), ("C", "D")]);
        assert!(
            broken_pairs(&c, &folders, &s(&["B", "A"])).is_empty(),
            "a loop of rules"
        );
        assert!(
            broken_pairs(&c, &folders, &s(&["B", "A", "C"])).is_empty(),
            "D is off in this profile"
        );
        let only_d = vec![(3usize, "D".to_string())];
        assert!(
            broken_pairs(&c, &only_d, &s(&["D", "C"])).is_empty(),
            "C was never installed"
        );
    }

    #[test]
    fn restore_order_moves_only_what_the_rules_need() {
        let (c, folders) = ruled(&["A", "B", "C"], &[("A", "B")]);
        // C moved above A on purpose, B above A by mistake; mine-* are not members.
        let list = s(&["mine-1", "C", "B", "mine-2", "A"]);
        assert_eq!(
            restore_order(&c, &folders, &list),
            s(&["mine-1", "C", "A", "mine-2", "B"])
        );
        let good = s(&["C", "A", "B"]);
        assert_eq!(restore_order(&c, &folders, &good), good);
    }

    /// X1 and X2 are two files of one mod, installed into the one folder X.
    fn shared(names: &[&str], rules: &[(&str, &str)]) -> (Collection, Vec<(usize, String)>) {
        let (c, mut folders) = ruled(names, rules);
        for (_, f) in &mut folders {
            if f.starts_with('X') {
                *f = "X".to_string();
            }
        }
        (c, folders)
    }

    #[test]
    fn rules_that_put_a_shared_folder_on_both_sides_of_a_mod_are_a_loop() {
        // X has to sit both above and below A, so no list order keeps both
        // rules: no card that Restore could never clear, and no move.
        let (c, folders) = shared(&["X1", "A", "X2"], &[("X1", "A"), ("A", "X2")]);
        for list in [s(&["X", "A"]), s(&["A", "X"])] {
            assert!(broken_pairs(&c, &folders, &list).is_empty(), "{list:?}");
            assert_eq!(restore_order(&c, &folders, &list), list);
        }
    }

    #[test]
    fn a_rule_on_the_second_file_of_a_shared_folder_is_restored() {
        // X1 has no rule and comes first by rank; the folder still has to move
        // below B for the rule on X2.
        let (c, folders) = shared(&["X1", "X2", "B"], &[("B", "X2")]);
        let list = s(&["X", "mine", "B"]);
        assert_eq!(
            broken_pairs(&c, &folders, &list),
            [("B".to_string(), "X".to_string())]
        );
        let restored = restore_order(&c, &folders, &list);
        assert_eq!(restored, s(&["B", "mine", "X"]));
        assert!(broken_pairs(&c, &folders, &restored).is_empty());
    }
}
