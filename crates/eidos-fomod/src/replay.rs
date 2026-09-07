//! Replaying the answers somebody else gave to this installer.
//!
//! A collection records, per mod, which option its author picked in each group
//! of each step. Replaying them is what makes a collection install the mod the
//! way the collection was built, instead of the way the mod's defaults happen to
//! fall - and half of a real Skyrim list is FOMOD-driven, so without this an
//! install that reports success has quietly laid down different files.
//!
//! Why it cannot be a table lookup. A step is only SHOWN when its `<visible>`
//! condition holds, and those conditions read flags that earlier options set. So
//! which questions exist at all depends on the answers to the questions before
//! them, and the replay has to walk forward through the same engine that
//! computes those flags, exactly as [`crate::default_selection`] does. Writing
//! the recorded indices straight into a `Selection` would answer questions that
//! were never asked and miss ones that were.
//!
//! Matching is by NAME, with the recorded position only as a tiebreak between
//! two options of the same name. Positions move when a mod is updated and names
//! usually do not, so a positional match is the one most likely to be silently
//! wrong - and silently wrong is the failure mode this whole module exists to
//! avoid. Anything that cannot be matched is REPORTED, so the caller can show
//! the dialog with the author's answers pre-ticked rather than install a guess.

use std::collections::HashMap;

use crate::engine::{effective_type, eval, Context, Selection};
use crate::model::{GroupType, ModuleConfig, PluginType};

/// One option the author selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedOption {
    pub name: String,
    /// Its position in the group when the author recorded it.
    pub idx: usize,
}

/// One group's recorded answers: the options that were SELECTED. An option not
/// listed here was not selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedGroup {
    pub name: String,
    pub selected: Vec<RecordedOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedStep {
    pub name: String,
    pub groups: Vec<RecordedGroup>,
}

/// What a replay produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Replay {
    /// The selection to install with.
    pub selection: Selection,
    /// Everything that could not be replayed, in the author's own words.
    ///
    /// A non-empty list means the install would NOT be the one the collection
    /// author built. The honest response is to show the dialog with this
    /// selection already applied and let the user finish it - which is what
    /// Vortex does for the same situation - not to install and call it done.
    pub unmatched: Vec<String>,
}

impl Replay {
    /// Whether every recorded answer found its question.
    pub fn is_faithful(&self) -> bool {
        self.unmatched.is_empty()
    }
}

/// Which option in `names` a recorded answer means.
///
/// Name first, case-insensitively, because that is what survives a mod update.
/// The recorded index breaks a tie between two options sharing a name, and is
/// the last resort when the name matches nothing - and even then only when it is
/// in range, because an out-of-range index is evidence the group changed shape.
fn find_option(names: &[String], want: &RecordedOption) -> Option<usize> {
    let same: Vec<usize> = names
        .iter()
        .enumerate()
        .filter(|(_, n)| n.trim().eq_ignore_ascii_case(want.name.trim()))
        .map(|(i, _)| i)
        .collect();
    match same.len() {
        0 => None,
        1 => Some(same[0]),
        // Two options with the same label in one group: the position decides.
        _ => same
            .iter()
            .copied()
            .find(|i| *i == want.idx)
            .or_else(|| same.first().copied()),
    }
}

/// Apply `recorded` to `config`, walking forward so each step is answered in the
/// world its predecessors created.
pub fn replay(config: &ModuleConfig, ctx: &Context, recorded: &[RecordedStep]) -> Replay {
    let mut flags: HashMap<String, String> = ctx.flags.clone();
    let mut selection: Selection = Vec::with_capacity(config.steps.len());
    let mut unmatched: Vec<String> = Vec::new();
    // Recorded steps are consumed in order, but matched by name: an installer
    // that gained or lost a step between the author's version and this one must
    // not shift every later answer by one.
    let mut used = vec![false; recorded.len()];

    for step in &config.steps {
        let visible = step
            .visible
            .as_ref()
            .map(|v| eval(v, &flags, ctx))
            .unwrap_or(true);
        let mut step_sel: Vec<Vec<bool>> = Vec::with_capacity(step.groups.len());
        if !visible {
            // A step the flags hid asks nothing, so it answers nothing. The
            // engine's own default pass does the same.
            for group in &step.groups {
                step_sel.push(vec![false; group.plugins.len()]);
            }
            selection.push(step_sel);
            continue;
        }

        let rec_step = recorded
            .iter()
            .enumerate()
            .find(|(i, r)| !used[*i] && r.name.trim().eq_ignore_ascii_case(step.name.trim()))
            .map(|(i, r)| {
                used[i] = true;
                r
            });

        for group in &step.groups {
            let names: Vec<String> = group.plugins.iter().map(|p| p.name.clone()).collect();
            let types: Vec<PluginType> = group
                .plugins
                .iter()
                .map(|p| effective_type(p, &flags, ctx))
                .collect();
            let mut on = vec![false; group.plugins.len()];

            let rec_group = rec_step.and_then(|s| {
                s.groups
                    .iter()
                    .find(|g| g.name.trim().eq_ignore_ascii_case(group.name.trim()))
            });

            match rec_group {
                Some(g) => {
                    for want in &g.selected {
                        match find_option(&names, want) {
                            Some(i) if types[i] != PluginType::NotUsable => on[i] = true,
                            // Recorded, found, and this installer will not allow
                            // it here - a condition the author's run did not hit.
                            Some(_) => unmatched.push(format!(
                                "\"{}\" in \"{}\" cannot be selected under these conditions",
                                want.name, group.name
                            )),
                            None => unmatched.push(format!(
                                "\"{}\" is no longer an option in \"{}\"",
                                want.name, group.name
                            )),
                        }
                    }
                }
                None if !group.plugins.is_empty() => unmatched.push(format!(
                    "no recorded answer for \"{}\" in \"{}\"",
                    group.name, step.name
                )),
                None => {}
            }

            // A group whose rules the replay cannot satisfy is not quietly
            // bent into shape: the mismatch is what the caller needs to hear.
            // Required options are forced on regardless, because the installer
            // itself does not offer them as a choice.
            for (i, t) in types.iter().enumerate() {
                if *t == PluginType::Required {
                    on[i] = true;
                }
            }
            let picked = on.iter().filter(|x| **x).count();
            let complaint = match group.group_type {
                GroupType::SelectExactlyOne if picked != 1 => {
                    Some(format!("\"{}\" needs exactly one answer", group.name))
                }
                GroupType::SelectAtMostOne if picked > 1 => {
                    Some(format!("\"{}\" accepts at most one answer", group.name))
                }
                GroupType::SelectAtLeastOne if picked == 0 => {
                    Some(format!("\"{}\" needs at least one answer", group.name))
                }
                GroupType::SelectAll if picked != names.len() => {
                    Some(format!("\"{}\" takes all of its options", group.name))
                }
                _ => None,
            };
            if let Some(c) = complaint {
                unmatched.push(c);
            }

            for (i, chosen) in on.iter().enumerate() {
                if *chosen {
                    for (n, v) in &group.plugins[i].condition_flags {
                        flags.insert(n.clone(), v.clone());
                    }
                }
            }
            step_sel.push(on);
        }
        selection.push(step_sel);
    }

    // Answers for questions this installer never asked. Worth saying: it means
    // the mod has changed since the collection was built.
    for (i, r) in recorded.iter().enumerate() {
        if !used[i] {
            unmatched.push(format!("this installer has no step called \"{}\"", r.name));
        }
    }

    Replay {
        selection,
        unmatched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn plugin(name: &str, t: PluginType, flags: &[(&str, &str)]) -> Plugin {
        Plugin {
            name: name.to_string(),
            description: String::new(),
            image: None,
            type_descriptor: TypeDescriptor {
                default_type: t,
                patterns: Vec::new(),
            },
            condition_flags: flags
                .iter()
                .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
                .collect(),
            files: Vec::new(),
        }
    }

    fn group(name: &str, t: GroupType, plugins: Vec<Plugin>) -> Group {
        Group {
            name: name.to_string(),
            group_type: t,
            plugins,
        }
    }

    fn step(name: &str, visible: Option<Condition>, groups: Vec<Group>) -> InstallStep {
        InstallStep {
            name: name.to_string(),
            visible,
            groups,
        }
    }

    fn rec(step: &str, group: &str, options: &[(&str, usize)]) -> RecordedStep {
        RecordedStep {
            name: step.to_string(),
            groups: vec![RecordedGroup {
                name: group.to_string(),
                selected: options
                    .iter()
                    .map(|(n, i)| RecordedOption {
                        name: (*n).to_string(),
                        idx: *i,
                    })
                    .collect(),
            }],
        }
    }

    fn ctx() -> Context {
        Context::default()
    }

    #[test]
    fn a_recorded_answer_is_applied_by_name() {
        let config = ModuleConfig {
            steps: vec![step(
                "Main",
                None,
                vec![group(
                    "Resolution",
                    GroupType::SelectExactlyOne,
                    vec![
                        plugin("1K", PluginType::Optional, &[]),
                        plugin("2K", PluginType::Optional, &[]),
                        plugin("4K", PluginType::Optional, &[]),
                    ],
                )],
            )],
            ..ModuleConfig::default()
        };
        let r = replay(&config, &ctx(), &[rec("Main", "Resolution", &[("2K", 1)])]);
        assert!(r.is_faithful(), "{:?}", r.unmatched);
        assert_eq!(r.selection[0][0], vec![false, true, false]);
    }

    #[test]
    fn the_name_wins_over_the_recorded_position() {
        // The whole reason to match on names: a mod update inserted an option,
        // so every index after it moved. The author's answer is still "4K".
        let config = ModuleConfig {
            steps: vec![step(
                "Main",
                None,
                vec![group(
                    "Resolution",
                    GroupType::SelectExactlyOne,
                    vec![
                        plugin("512", PluginType::Optional, &[]),
                        plugin("1K", PluginType::Optional, &[]),
                        plugin("2K", PluginType::Optional, &[]),
                        plugin("4K", PluginType::Optional, &[]),
                    ],
                )],
            )],
            ..ModuleConfig::default()
        };
        // Recorded at index 2, which is now "2K".
        let r = replay(&config, &ctx(), &[rec("Main", "Resolution", &[("4K", 2)])]);
        assert!(r.is_faithful(), "{:?}", r.unmatched);
        assert_eq!(r.selection[0][0], vec![false, false, false, true]);
    }

    #[test]
    fn a_step_hidden_by_an_earlier_answer_is_not_answered() {
        // The forward pass is the point: whether the second step is asked at all
        // depends on the flag the first step's answer sets.
        let config = ModuleConfig {
            steps: vec![
                step(
                    "Choose",
                    None,
                    vec![group(
                        "Path",
                        GroupType::SelectExactlyOne,
                        vec![
                            plugin("Simple", PluginType::Optional, &[("mode", "simple")]),
                            plugin("Advanced", PluginType::Optional, &[("mode", "advanced")]),
                        ],
                    )],
                ),
                step(
                    "Extras",
                    Some(Condition::Flag {
                        flag: "mode".into(),
                        value: "advanced".into(),
                    }),
                    vec![group(
                        "Options",
                        GroupType::SelectAny,
                        vec![plugin("Cloaks", PluginType::Optional, &[])],
                    )],
                ),
            ],
            ..ModuleConfig::default()
        };
        // The author took the simple path, so "Extras" was never shown to them.
        let r = replay(&config, &ctx(), &[rec("Choose", "Path", &[("Simple", 0)])]);
        assert!(r.is_faithful(), "{:?}", r.unmatched);
        assert_eq!(r.selection[0][0], vec![true, false]);
        assert_eq!(r.selection[1][0], vec![false], "the step was never asked");

        // And taking the other path DOES ask it - and then not having an answer
        // for it is a mismatch, not a silent default.
        let r = replay(&config, &ctx(), &[rec("Choose", "Path", &[("Advanced", 1)])]);
        assert!(!r.is_faithful());
        assert!(
            r.unmatched.iter().any(|u| u.contains("Options")),
            "{:?}",
            r.unmatched
        );
    }

    #[test]
    fn an_option_that_no_longer_exists_is_reported_not_ignored() {
        let config = ModuleConfig {
            steps: vec![step(
                "Main",
                None,
                vec![group(
                    "Resolution",
                    GroupType::SelectExactlyOne,
                    vec![plugin("1K", PluginType::Optional, &[])],
                )],
            )],
            ..ModuleConfig::default()
        };
        let r = replay(&config, &ctx(), &[rec("Main", "Resolution", &[("8K", 3)])]);
        assert!(!r.is_faithful());
        assert!(r.unmatched[0].contains("8K"), "{:?}", r.unmatched);
        // And the group's own rule is reported too: nothing was selected.
        assert!(
            r.unmatched.iter().any(|u| u.contains("exactly one")),
            "{:?}",
            r.unmatched
        );
    }

    #[test]
    fn a_required_option_is_on_whether_it_was_recorded_or_not() {
        // The installer does not offer it as a choice, so its absence from the
        // recorded answers is not a disagreement.
        let config = ModuleConfig {
            steps: vec![step(
                "Main",
                None,
                vec![group(
                    "Core",
                    GroupType::SelectAll,
                    vec![plugin("Base", PluginType::Required, &[])],
                )],
            )],
            ..ModuleConfig::default()
        };
        let r = replay(&config, &ctx(), &[rec("Main", "Core", &[])]);
        assert_eq!(r.selection[0][0], vec![true]);
        assert!(r.is_faithful(), "{:?}", r.unmatched);
    }

    #[test]
    fn an_answer_for_a_step_this_installer_does_not_have_is_reported() {
        // It means the mod changed since the collection was built, which is
        // exactly what somebody wants to know before playing.
        let config = ModuleConfig {
            steps: vec![step(
                "Main",
                None,
                vec![group(
                    "G",
                    GroupType::SelectAny,
                    vec![plugin("A", PluginType::Optional, &[])],
                )],
            )],
            ..ModuleConfig::default()
        };
        let r = replay(
            &config,
            &ctx(),
            &[
                rec("Main", "G", &[("A", 0)]),
                rec("A Step That Went Away", "G", &[("A", 0)]),
            ],
        );
        assert!(!r.is_faithful());
        assert!(
            r.unmatched.iter().any(|u| u.contains("A Step That Went Away")),
            "{:?}",
            r.unmatched
        );
    }

    #[test]
    fn a_selection_replayed_faithfully_has_the_shape_the_engine_expects() {
        // It is handed straight to `build_plan`, which indexes it by step, group
        // and option - so a short vec is an out-of-bounds waiting to happen.
        let config = ModuleConfig {
            steps: vec![
                step(
                    "One",
                    None,
                    vec![
                        group(
                            "A",
                            GroupType::SelectAny,
                            vec![plugin("x", PluginType::Optional, &[])],
                        ),
                        group(
                            "B",
                            GroupType::SelectAny,
                            vec![
                                plugin("y", PluginType::Optional, &[]),
                                plugin("z", PluginType::Optional, &[]),
                            ],
                        ),
                    ],
                ),
                step(
                    "Two",
                    None,
                    vec![group(
                        "C",
                        GroupType::SelectAny,
                        vec![plugin("w", PluginType::Optional, &[])],
                    )],
                ),
            ],
            ..ModuleConfig::default()
        };
        let r = replay(&config, &ctx(), &[]);
        assert_eq!(r.selection.len(), 2);
        assert_eq!(r.selection[0].len(), 2);
        assert_eq!(r.selection[0][0].len(), 1);
        assert_eq!(r.selection[0][1].len(), 2);
        assert_eq!(r.selection[1][0].len(), 1);
        // And with no answers at all, every group says so.
        assert_eq!(r.unmatched.len(), 3, "{:?}", r.unmatched);
    }
}
