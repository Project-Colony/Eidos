//! The FOMOD condition engine + install-plan computation.
//!
//! As the user moves through the steps, selecting options sets condition flags;
//! those flags (plus file states and the game version) drive which options are
//! recommended/usable and which `conditionalFileInstalls` apply. This module
//! evaluates conditions, resolves a plugin's effective type, computes the default
//! selection for a non-interactive install, and assembles the ordered file plan.

use std::collections::HashMap;

use crate::model::*;

/// What the engine knows about the world when evaluating conditions.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Flags already set (e.g. carried in from a prior install). Selections add more.
    pub flags: HashMap<String, String>,
    /// Known plugin file states, lowercased name -> "Active"/"Inactive"/"Missing".
    /// Anything absent is treated as "Missing" (nothing is installed yet).
    pub file_states: HashMap<String, String>,
    /// The managed game's version, if known (for `gameDependency`).
    pub game_version: Option<String>,
}

/// Evaluate a condition against the current flags and the context.
pub fn eval(cond: &Condition, flags: &HashMap<String, String>, ctx: &Context) -> bool {
    match cond {
        Condition::Flag { flag, value } => {
            flags.get(flag).map(String::as_str).unwrap_or("") == value.as_str()
        }
        Condition::File { file, state } => {
            let actual =
                ctx.file_states.get(&file.to_lowercase()).map(String::as_str).unwrap_or("Missing");
            actual.eq_ignore_ascii_case(state)
        }
        Condition::Version { kind, version } => {
            if kind == "Game" {
                ctx.game_version.as_deref().map(|v| version_ge(v, version)).unwrap_or(true)
            } else {
                // We don't track FOMM/FOSE versions; don't block on them.
                true
            }
        }
        Condition::Sub { op, conditions } => match op {
            Operator::And => conditions.iter().all(|c| eval(c, flags, ctx)),
            Operator::Or => conditions.iter().any(|c| eval(c, flags, ctx)),
        },
    }
}

/// A plugin's effective type: the first conditional pattern that holds, else the
/// default (MO2's `getPluginDependencyType`).
pub fn effective_type(plugin: &Plugin, flags: &HashMap<String, String>, ctx: &Context) -> PluginType {
    for pat in &plugin.type_descriptor.patterns {
        if eval(&pat.condition, flags, ctx) {
            return pat.plugin_type;
        }
    }
    plugin.type_descriptor.default_type
}

/// The indices selected by default in a group, honouring its type and the plugins'
/// effective types (Required always on, Recommended preselected, NotUsable never).
fn default_group_selection(group: &Group, flags: &HashMap<String, String>, ctx: &Context) -> Vec<usize> {
    let types: Vec<PluginType> = group.plugins.iter().map(|p| effective_type(p, flags, ctx)).collect();
    let pos = |t: PluginType| types.iter().position(|x| *x == t);
    let first_usable = || types.iter().position(|x| *x != PluginType::NotUsable);
    let preselected = || -> Vec<usize> {
        (0..types.len())
            .filter(|&i| matches!(types[i], PluginType::Required | PluginType::Recommended))
            .collect()
    };
    match group.group_type {
        GroupType::SelectAll => {
            (0..types.len()).filter(|&i| types[i] != PluginType::NotUsable).collect()
        }
        GroupType::SelectExactlyOne => pos(PluginType::Required)
            .or_else(|| pos(PluginType::Recommended))
            .or_else(first_usable)
            .into_iter()
            .collect(),
        GroupType::SelectAtMostOne => {
            pos(PluginType::Required).or_else(|| pos(PluginType::Recommended)).into_iter().collect()
        }
        GroupType::SelectAtLeastOne => {
            let rec = preselected();
            if rec.is_empty() {
                first_usable().into_iter().collect()
            } else {
                rec
            }
        }
        GroupType::SelectAny => preselected(),
    }
}

/// A selection of options: `selected[step][group][plugin]`.
pub type Selection = Vec<Vec<Vec<bool>>>;

/// The default selection for every step/group/plugin, via a forward pass so later
/// steps see the flags set by earlier default choices. Invisible steps select nothing.
pub fn default_selection(config: &ModuleConfig, ctx: &Context) -> Selection {
    let mut flags = ctx.flags.clone();
    let mut sel = Vec::with_capacity(config.steps.len());
    for step in &config.steps {
        let visible = step.visible.as_ref().map(|v| eval(v, &flags, ctx)).unwrap_or(true);
        let mut step_sel = Vec::with_capacity(step.groups.len());
        for group in &step.groups {
            let mut g = vec![false; group.plugins.len()];
            if visible {
                for i in default_group_selection(group, &flags, ctx) {
                    g[i] = true;
                    for (n, v) in &group.plugins[i].condition_flags {
                        flags.insert(n.clone(), v.clone());
                    }
                }
            }
            step_sel.push(g);
        }
        sel.push(step_sel);
    }
    sel
}

/// Compute the ordered file plan for a given selection: walk the visible steps,
/// take each selected option's flags + files, then apply `conditionalFileInstalls`.
/// Sorted by priority then XML order, so applying in order lets higher-priority
/// sources overwrite lower ones.
pub fn build_plan(config: &ModuleConfig, selection: &Selection, ctx: &Context) -> Vec<FileItem> {
    let mut flags = ctx.flags.clone();
    let mut files = config.required_files.clone();

    for (si, step) in config.steps.iter().enumerate() {
        let visible = step.visible.as_ref().map(|v| eval(v, &flags, ctx)).unwrap_or(true);
        if !visible {
            continue;
        }
        for (gi, group) in step.groups.iter().enumerate() {
            for (pi, plugin) in group.plugins.iter().enumerate() {
                let on = selection
                    .get(si)
                    .and_then(|s| s.get(gi))
                    .and_then(|g| g.get(pi))
                    .copied()
                    .unwrap_or(false);
                if on {
                    for (n, v) in &plugin.condition_flags {
                        flags.insert(n.clone(), v.clone());
                    }
                    files.extend(plugin.files.iter().cloned());
                }
            }
        }
    }

    for ci in &config.conditional_installs {
        if eval(&ci.condition, &flags, ctx) {
            files.extend(ci.files.iter().cloned());
        }
    }

    files.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.sequence.cmp(&b.sequence)));
    files
}

/// The plan for a non-interactive install, using the default selection.
pub fn build_default_plan(config: &ModuleConfig, ctx: &Context) -> Vec<FileItem> {
    build_plan(config, &default_selection(config, ctx), ctx)
}

/// The effective type of every plugin in `step_idx`'s groups, given the condition
/// flags accumulated by the selections in the prior visible steps. Lets a front end
/// disable `NotUsable` options and highlight `Required`/`Recommended` ones, with the
/// types re-evaluated as earlier choices set flags.
pub fn step_types(
    config: &ModuleConfig,
    selection: &Selection,
    ctx: &Context,
    step_idx: usize,
) -> Vec<Vec<PluginType>> {
    let mut flags = ctx.flags.clone();
    for (si, step) in config.steps.iter().enumerate() {
        if si == step_idx {
            return step
                .groups
                .iter()
                .map(|g| g.plugins.iter().map(|p| effective_type(p, &flags, ctx)).collect())
                .collect();
        }
        let visible = step.visible.as_ref().map(|v| eval(v, &flags, ctx)).unwrap_or(true);
        if visible {
            for (gi, group) in step.groups.iter().enumerate() {
                for (pi, plugin) in group.plugins.iter().enumerate() {
                    let on = selection
                        .get(si)
                        .and_then(|s| s.get(gi))
                        .and_then(|g| g.get(pi))
                        .copied()
                        .unwrap_or(false);
                    if on {
                        for (n, v) in &plugin.condition_flags {
                            flags.insert(n.clone(), v.clone());
                        }
                    }
                }
            }
        }
    }
    Vec::new()
}

/// `actual >= required` on dotted numeric versions (missing parts count as 0).
fn version_ge(actual: &str, required: &str) -> bool {
    let pa: Vec<u64> = actual.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let pr: Vec<u64> = required.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    for i in 0..pa.len().max(pr.len()) {
        let a = pa.get(i).copied().unwrap_or(0);
        let r = pr.get(i).copied().unwrap_or(0);
        if a != r {
            return a > r;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<config>
  <moduleName>T</moduleName>
  <requiredInstallFiles><file source="a.esp" destination="a.esp"/></requiredInstallFiles>
  <installSteps>
    <installStep name="S">
      <optionalFileGroups>
        <group name="G" type="SelectExactlyOne">
          <plugins>
            <plugin name="Rec">
              <conditionFlags><flag name="v">on</flag></conditionFlags>
              <files><file source="rec.esp" destination="rec.esp"/></files>
              <typeDescriptor><type name="Recommended"/></typeDescriptor>
            </plugin>
            <plugin name="Opt">
              <files><file source="opt.esp" destination="opt.esp"/></files>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
  <conditionalFileInstalls><patterns><pattern>
    <dependencies operator="And"><flagDependency flag="v" value="on"/></dependencies>
    <files><file source="cond.esp" destination="cond.esp"/></files>
  </pattern></patterns></conditionalFileInstalls>
</config>"#;

    #[test]
    fn default_plan_picks_recommended_then_applies_conditionals() {
        let mc = ModuleConfig::parse(XML).unwrap();
        let plan = build_default_plan(&mc, &Context::default());
        let dests: Vec<&str> = plan.iter().map(|f| f.destination.as_str()).collect();
        assert!(dests.contains(&"a.esp")); // required
        assert!(dests.contains(&"rec.esp")); // Recommended selected in the exactly-one group
        assert!(dests.contains(&"cond.esp")); // conditional fires: Rec set flag v=on
        assert!(!dests.contains(&"opt.esp")); // the Optional sibling was not chosen
    }

    #[test]
    fn explicit_selection_overrides_default() {
        let mc = ModuleConfig::parse(XML).unwrap();
        // Pick "Opt" instead of the default "Rec".
        let sel = vec![vec![vec![false, true]]];
        let plan = build_plan(&mc, &sel, &Context::default());
        let dests: Vec<&str> = plan.iter().map(|f| f.destination.as_str()).collect();
        assert!(dests.contains(&"opt.esp"));
        assert!(!dests.contains(&"rec.esp"));
        assert!(!dests.contains(&"cond.esp")); // flag v not set -> conditional skipped
    }

    #[test]
    fn eval_flags_and_or() {
        let mut flags = HashMap::new();
        flags.insert("a".to_string(), "on".to_string());
        let ctx = Context::default();
        let on = Condition::Flag { flag: "a".into(), value: "on".into() };
        let off = Condition::Flag { flag: "b".into(), value: "on".into() };
        assert!(eval(&on, &flags, &ctx));
        assert!(!eval(&off, &flags, &ctx));
        assert!(eval(&Condition::Sub { op: Operator::Or, conditions: vec![on.clone(), off.clone()] }, &flags, &ctx));
        assert!(!eval(&Condition::Sub { op: Operator::And, conditions: vec![on, off] }, &flags, &ctx));
    }

    #[test]
    fn effective_type_follows_patterns() {
        // A plugin Optional by default, NotUsable when flag v=on.
        let p = Plugin {
            name: "P".into(),
            description: String::new(),
            image: None,
            type_descriptor: TypeDescriptor {
                default_type: PluginType::Optional,
                patterns: vec![DependencyPattern {
                    plugin_type: PluginType::NotUsable,
                    condition: Condition::Flag { flag: "v".into(), value: "on".into() },
                }],
            },
            condition_flags: Vec::new(),
            files: Vec::new(),
        };
        let ctx = Context::default();
        assert_eq!(effective_type(&p, &HashMap::new(), &ctx), PluginType::Optional);
        let mut flags = HashMap::new();
        flags.insert("v".to_string(), "on".to_string());
        assert_eq!(effective_type(&p, &flags, &ctx), PluginType::NotUsable);
    }
}
