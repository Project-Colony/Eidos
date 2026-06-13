//! The FOMOD `ModuleConfig.xml` data model (mirrors MO2's `fomodinstallerdialog.h`).

/// How many options a group lets the user pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupType {
    SelectAtLeastOne,
    SelectAtMostOne,
    SelectExactlyOne,
    SelectAny,
    SelectAll,
}

/// A plugin (option) selectability/recommendation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Required,
    Recommended,
    Optional,
    NotUsable,
    CouldBeUsable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    And,
    Or,
}

/// A dependency condition (the `<dependencies>` / `*Dependency` elements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// `<flagDependency flag=".." value="..">` - a condition flag has a value.
    Flag { flag: String, value: String },
    /// `<fileDependency file=".." state="Active|Inactive|Missing">`.
    File { file: String, state: String },
    /// `<gameDependency>` / `<fommDependency>` / `<foseDependency>`.
    Version { kind: String, version: String },
    /// `<dependencies operator="And|Or">` - a composite of nested conditions.
    Sub { op: Operator, conditions: Vec<Condition> },
}

/// A `<file>` or `<folder>` install instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    pub source: String,
    pub destination: String,
    pub priority: i32,
    pub is_folder: bool,
    pub always_install: bool,
    pub install_if_usable: bool,
    /// XML order, the tie-breaker at equal priority (MO2 keeps NMM's sequence).
    pub sequence: u32,
}

/// One `<pattern>` of a plugin's `<dependencyType>`: a condition -> a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyPattern {
    pub plugin_type: PluginType,
    pub condition: Condition,
}

/// A plugin's `<typeDescriptor>`: a default type plus optional conditional patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDescriptor {
    pub default_type: PluginType,
    pub patterns: Vec<DependencyPattern>,
}

/// One selectable option in a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub image: Option<String>,
    pub type_descriptor: TypeDescriptor,
    /// Flags this option sets when selected (`<conditionFlags><flag>`).
    pub condition_flags: Vec<(String, String)>,
    pub files: Vec<FileItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub group_type: GroupType,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallStep {
    pub name: String,
    /// `<visible>` - the step is only shown when this condition holds.
    pub visible: Option<Condition>,
    pub groups: Vec<Group>,
}

/// A `conditionalFileInstalls` pattern: install `files` when `condition` holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalInstall {
    pub condition: Condition,
    pub files: Vec<FileItem>,
}

/// A parsed `ModuleConfig.xml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleConfig {
    pub module_name: String,
    pub module_image: Option<String>,
    /// `<moduleDependencies>` - the whole install is refused when this is unmet
    /// (MO2 aborts). Evaluated against the install Context by the caller.
    pub module_dependencies: Option<Condition>,
    pub required_files: Vec<FileItem>,
    pub steps: Vec<InstallStep>,
    pub conditional_installs: Vec<ConditionalInstall>,
}

impl GroupType {
    pub fn parse(s: &str) -> GroupType {
        match s {
            "SelectAtLeastOne" => GroupType::SelectAtLeastOne,
            "SelectAtMostOne" => GroupType::SelectAtMostOne,
            "SelectExactlyOne" => GroupType::SelectExactlyOne,
            "SelectAll" => GroupType::SelectAll,
            _ => GroupType::SelectAny,
        }
    }
}

impl PluginType {
    pub fn parse(s: &str) -> PluginType {
        match s {
            "Required" => PluginType::Required,
            "Recommended" => PluginType::Recommended,
            "NotUsable" => PluginType::NotUsable,
            "CouldBeUsable" => PluginType::CouldBeUsable,
            _ => PluginType::Optional,
        }
    }
}
