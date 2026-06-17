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

/// Whether the FOMOD's `<moduleDependencies>` are satisfied by the context. MO2
/// refuses the whole install when they are not (e.g. a required master/SKSE is
/// absent). No `<moduleDependencies>` element means always installable.
pub fn module_dependencies_met(config: &ModuleConfig, ctx: &Context) -> bool {
    match &config.module_dependencies {
        Some(cond) => eval(cond, &ctx.flags, ctx),
        None => true,
    }
}

/// A human-readable description of the FOMOD's module-level dependencies when they
/// are NOT met (so the caller can tell the user what the mod requires), else `None`.
pub fn unmet_module_dependencies(config: &ModuleConfig, ctx: &Context) -> Option<String> {
    let cond = config.module_dependencies.as_ref()?;
    if eval(cond, &ctx.flags, ctx) {
        return None;
    }
    Some(describe_condition(cond))
}

/// Render a condition into a short human phrase (for the unmet-dependency message).
fn describe_condition(c: &Condition) -> String {
    match c {
        Condition::File { file, state } => format!("'{file}' must be {}", state.to_lowercase()),
        Condition::Flag { flag, value } => format!("flag '{flag}' must be '{value}'"),
        Condition::Version { kind, version } => format!("{kind} version must be >= {version}"),
        Condition::Sub { op, conditions } => {
            let joiner = match op {
                Operator::And => " and ",
                Operator::Or => " or ",
            };
            let parts: Vec<String> = conditions.iter().map(describe_condition).collect();
            parts.join(joiner)
        }
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
        // MO2's forced-selection fallback order: Required, then Recommended, then
        // the first Optional, then the first CouldBeUsable, and only then any
        // remaining usable option - so a CouldBeUsable never wins ahead of an
        // Optional listed after it.
        GroupType::SelectExactlyOne => pos(PluginType::Required)
            .or_else(|| pos(PluginType::Recommended))
            .or_else(|| pos(PluginType::Optional))
            .or_else(|| pos(PluginType::CouldBeUsable))
            .or_else(first_usable)
            .into_iter()
            .collect(),
        GroupType::SelectAtMostOne => {
            pos(PluginType::Required).or_else(|| pos(PluginType::Recommended)).into_iter().collect()
        }
        GroupType::SelectAtLeastOne => {
            let rec = preselected();
            if rec.is_empty() {
                pos(PluginType::Optional)
                    .or_else(|| pos(PluginType::CouldBeUsable))
                    .or_else(first_usable)
                    .into_iter()
                    .collect()
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
                }
                // A selected option installs all its files. An UNSELECTED option still
                // installs files flagged `alwaysInstall` (unconditional) or
                // `installIfUsable` (when the option is usable - effective type is not
                // NotUsable). MO2's FOMOD installer honours both flags; Eidos used to
                // drop them, silently skipping files the author meant to ship.
                let etype = (!on).then(|| effective_type(plugin, &flags, ctx));
                let usable = etype != Some(PluginType::NotUsable);
                for f in &plugin.files {
                    if on || f.always_install || (f.install_if_usable && usable) {
                        files.push(f.clone());
                    }
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

/// Per-step visibility, given the flags accumulated by the selections in the prior
/// visible steps. A step whose `<visible>` condition is false should be skipped in
/// navigation (and its options contribute nothing to the plan).
pub fn visible_steps(config: &ModuleConfig, selection: &Selection, ctx: &Context) -> Vec<bool> {
    let mut flags = ctx.flags.clone();
    let mut out = Vec::with_capacity(config.steps.len());
    for (si, step) in config.steps.iter().enumerate() {
        let visible = step.visible.as_ref().map(|v| eval(v, &flags, ctx)).unwrap_or(true);
        out.push(visible);
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
    out
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
          <plugins order="Explicit">
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
    fn exactly_one_prefers_optional_over_couldbeusable() {
        // A CouldBeUsable option listed BEFORE an Optional one: MO2's forced
        // selection picks the Optional, not merely the first usable (CouldBeUsable).
        const ORD: &str = r#"<config>
  <moduleName>T</moduleName>
  <installSteps>
    <installStep name="S">
      <optionalFileGroups>
        <group name="G" type="SelectExactlyOne">
          <plugins>
            <plugin name="Maybe">
              <files><file source="maybe.esp" destination="maybe.esp"/></files>
              <typeDescriptor><type name="CouldBeUsable"/></typeDescriptor>
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
</config>"#;
        let mc = ModuleConfig::parse(ORD).unwrap();
        let plan = build_default_plan(&mc, &Context::default());
        let dests: Vec<&str> = plan.iter().map(|f| f.destination.as_str()).collect();
        assert!(dests.contains(&"opt.esp"), "the Optional must be the default pick");
        assert!(!dests.contains(&"maybe.esp"), "the CouldBeUsable must not win ahead of it");
    }

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
    fn step_types_reflects_effective_types() {
        let mc = ModuleConfig::parse(XML).unwrap();
        let sel = default_selection(&mc, &Context::default());
        let types = step_types(&mc, &sel, &Context::default(), 0);
        assert_eq!(types, vec![vec![PluginType::Recommended, PluginType::Optional]]);
    }

    #[test]
    fn visible_steps_all_visible_without_conditions() {
        let mc = ModuleConfig::parse(XML).unwrap();
        let sel = default_selection(&mc, &Context::default());
        assert_eq!(visible_steps(&mc, &sel, &Context::default()), vec![true]);
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
    fn always_install_and_if_usable_honour_unselected_options() {
        // SelectAny preselects only Required/Recommended, so both options are OFF by
        // default. Their files install only per the alwaysInstall / installIfUsable flags.
        const X: &str = r#"<config>
  <moduleName>T</moduleName>
  <installSteps>
    <installStep name="S">
      <optionalFileGroups>
        <group name="G" type="SelectAny">
          <plugins>
            <plugin name="UsableOpt">
              <files>
                <file source="normal.esp" destination="normal.esp"/>
                <file source="u_if.esp" destination="u_if.esp" installIfUsable="true"/>
              </files>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
            <plugin name="Bad">
              <files>
                <file source="b_if.esp" destination="b_if.esp" installIfUsable="true"/>
                <file source="b_always.esp" destination="b_always.esp" alwaysInstall="true"/>
              </files>
              <typeDescriptor><type name="NotUsable"/></typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
</config>"#;
        let mc = ModuleConfig::parse(X).unwrap();
        let plan = build_default_plan(&mc, &Context::default());
        let d: Vec<&str> = plan.iter().map(|f| f.destination.as_str()).collect();
        assert!(!d.contains(&"normal.esp"), "a plain file of an unselected option is skipped");
        assert!(d.contains(&"u_if.esp"), "installIfUsable on a usable (Optional) option installs");
        assert!(!d.contains(&"b_if.esp"), "installIfUsable on a NotUsable option does NOT install");
        assert!(d.contains(&"b_always.esp"), "alwaysInstall installs even on a NotUsable option");
    }

    #[test]
    fn module_dependencies_gate_the_install() {
        const X: &str = r#"<config>
  <moduleName>T</moduleName>
  <moduleDependencies operator="And">
    <fileDependency file="Required.esm" state="Active"/>
  </moduleDependencies>
  <installSteps><installStep name="S"><optionalFileGroups/></installStep></installSteps>
</config>"#;
        let mc = ModuleConfig::parse(X).unwrap();
        // Unmet: the required master is absent -> Missing.
        let empty = Context::default();
        assert!(!module_dependencies_met(&mc, &empty));
        let msg = unmet_module_dependencies(&mc, &empty).expect("should report unmet");
        assert!(msg.contains("Required.esm"), "message names the requirement: {msg}");
        // Met: mark the master Active.
        let mut ctx = Context::default();
        ctx.file_states.insert("required.esm".into(), "Active".into());
        assert!(module_dependencies_met(&mc, &ctx));
        assert!(unmet_module_dependencies(&mc, &ctx).is_none());
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
