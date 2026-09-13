//! Nexus collections: read a collection's manifest, and install it the way its
//! author built it.
//!
//! The distinction this crate exists for: the Nexus API can tell you which mods
//! a collection contains, and nothing else. The order they install in, the
//! answers their scripted installers were given, which one wins a file conflict,
//! which plugins load where, the binary patches - all of that lives in the
//! collection's own archive. Downloading the same mods is not installing the
//! collection, and the difference is a different game.

pub mod driver;
pub mod install;
pub mod manifest;
pub mod report;
pub mod rules;
pub mod state;

pub use manifest::{
    ChoiceGroup, ChoiceOption, ChoiceStep, Choices, Collection, FileHash, Info, Mod, ModReference,
    ModRule, Plugin, PluginRules, Read, RuleType, Source, SourceType, Tool, read,
};

pub mod recipe;

pub mod installer_answers;
