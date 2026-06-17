//! FOMOD scripted-installer engine, porting Mod Organizer 2's `installer_fomod`.
//!
//! A FOMOD ships a `fomod/ModuleConfig.xml` describing an ordered set of install
//! steps. Each step holds groups of plugins (options); the user picks options per
//! the group type (one-of, any, all...). Picking an option can set condition flags
//! and schedules files to install. A final `conditionalFileInstalls` block installs
//! extra files when its flag/file conditions hold.
//!
//! This module parses that XML into a model ([`ModuleConfig`]); the selection ->
//! file-plan engine and the front-end wizard build on it.

mod engine;
mod model;
mod parse;

pub use engine::{
    build_default_plan, build_plan, default_selection, effective_type, eval,
    module_dependencies_met, step_types, unmet_module_dependencies, visible_steps, Context,
    Selection,
};
pub use model::*;
pub use parse::decode_xml;
