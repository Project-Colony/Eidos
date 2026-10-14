//! Fixed native translations; received C# is never executed.
//! Reference: erri120/OMODFramework 05b3d5629124cfa44308b3b78c38959b960b3730,
//! GPL-3.0-only. UI scripts: Copyright 2007–2008 DarN; DarkUI modified by gothic251.
//! Native translation and checked staging implementation: September 2026.

use super::obmm::*;
use super::{member_path, OmodFileKind, OmodMember, OmodScript, OmodScriptKind};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
mod bsa;
mod horse;
#[cfg(test)]
mod tests;
mod ui;

type E<T> = Result<T, ObmmError>;
const HORSE_OUTPUT: &str = "HRMHorseArmor.bsa";
const MAX_GENERATED: usize = 512 * 1024 * 1024;
const MAX_MEMBER: u64 = 64 * 1024 * 1024;
fn error(message: impl Into<String>) -> ObmmError {
    ObmmError {
        line: 0,
        message: format!("Known OMOD handler: {}", message.into()),
    }
}
fn checked_path(path: &str) -> E<String> {
    member_path(path).map_err(|e| error(e.to_string()))
}
fn under(path: &str, folder: &str) -> bool {
    path.eq_ignore_ascii_case(folder)
        || path
            .to_ascii_lowercase()
            .starts_with(&(folder.to_ascii_lowercase() + "/"))
}
fn check_cancel(cancel: &AtomicBool) -> E<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(error("cancelled"))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KnownHandler {
    DarNifiedUi132,
    DarkUidDarn16,
    HorseArmorRevamped18,
}
impl KnownHandler {
    /// CRC is a dispatch key only. A matching body is never interpreted or executed.
    pub fn recognize(script: &OmodScript) -> Option<Self> {
        (script.kind == OmodScriptKind::CSharp)
            .then(|| Self::from_crc(crc32fast::hash(script.source().as_bytes())))
            .flatten()
    }
    fn from_crc(crc: u32) -> Option<Self> {
        match crc {
            0x45738C31 => Some(Self::DarNifiedUi132),
            0xF39B4EF8 => Some(Self::DarkUidDarn16),
            0x9646E015 => Some(Self::HorseArmorRevamped18),
            _ => None,
        }
    }
    pub fn evaluate(
        self,
        members: &[OmodMember],
        context: &ObmmContext,
        answers: &[ObmmRecordedAnswer],
        cancel: &AtomicBool,
    ) -> E<KnownEvaluation> {
        let mut m = Machine::new(self, members, context, answers, cancel)?;
        let result = if self == Self::HorseArmorRevamped18 {
            horse::evaluate(&mut m)
        } else {
            ui::evaluate(&mut m)
        };
        match result {
            Err(Halt::Prompt(p)) => Ok(KnownEvaluation::NeedPrompt(p)),
            Err(Halt::Error(e)) => Err(e),
            Ok(generation) => {
                if m.ordinal != answers.len() {
                    return Err(error("unused recorded answers"));
                }
                validate_files(&m.files)?;
                Ok(KnownEvaluation::Complete(KnownPlan {
                    plan: ObmmPlan {
                        files: m.files.into_values().collect(),
                        effects: m.effects,
                        warnings: m.warnings,
                    },
                    generation,
                }))
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownEvaluation {
    NeedPrompt(ObmmPrompt),
    Complete(KnownPlan),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownPlan {
    pub plan: ObmmPlan,
    generation: Option<HorseArmorRecipe>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct HorseArmorRecipe {
    cloth: String,
    members: BTreeMap<String, OmodMember>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub destination: String,
    pub bytes: Vec<u8>,
}
impl KnownPlan {
    /// Planned generated outputs, available before reading or building any BSA.
    pub fn generated_destinations(&self) -> &'static [&'static str] {
        if self.generation.is_some() {
            &[HORSE_OUTPUT]
        } else {
            &[]
        }
    }

    /// Reads only from caller-owned decoded data and the pinned effective profile.
    /// Readers must enforce `max_bytes` before allocation and revalidate provider identity.
    /// Returned bytes belong in the same owned stage as `plan.files`, before publication.
    /// No intermediate horse assets are published loose. UI plans return no generated files.
    pub fn generate(
        &self,
        mut read_data: impl FnMut(&str, u64) -> E<Vec<u8>>,
        mut read_bsa: impl FnMut(&str, &str, u64) -> E<Vec<u8>>,
        cancel: &AtomicBool,
    ) -> E<Vec<GeneratedFile>> {
        check_cancel(cancel)?;
        match &self.generation {
            None => Ok(vec![]),
            Some(recipe) => horse::generate(recipe, &mut read_data, &mut read_bsa, cancel),
        }
    }
}

enum Halt {
    Prompt(ObmmPrompt),
    Error(ObmmError),
}
impl From<ObmmError> for Halt {
    fn from(e: ObmmError) -> Self {
        Self::Error(e)
    }
}
type Step<T> = Result<T, Halt>;
struct Machine<'a> {
    handler: KnownHandler,
    members: BTreeMap<String, &'a OmodMember>,
    context: &'a ObmmContext,
    answers: &'a [ObmmRecordedAnswer],
    cancel: &'a AtomicBool,
    ordinal: usize,
    files: BTreeMap<String, ObmmFile>,
    effects: Vec<ObmmEffect>,
    warnings: Vec<String>,
}
impl<'a> Machine<'a> {
    fn new(
        handler: KnownHandler,
        members: &'a [OmodMember],
        context: &'a ObmmContext,
        answers: &'a [ObmmRecordedAnswer],
        cancel: &'a AtomicBool,
    ) -> E<Self> {
        check_cancel(cancel)?;
        if members.len() > 100_000 || answers.len() > 64 {
            return Err(error("input count exceeds bound"));
        }
        let mut map = BTreeMap::new();
        let mut files = BTreeMap::new();
        for m in members {
            let p = checked_path(&m.path)?;
            if map.insert(p.to_ascii_lowercase(), m).is_some() {
                return Err(error("case-colliding archive sources"));
            }
            if handler == KnownHandler::HorseArmorRevamped18 {
                files.insert(
                    p.to_ascii_lowercase(),
                    ObmmFile {
                        kind: m.kind,
                        source: p.clone(),
                        destination: p,
                    },
                );
            }
        }
        for p in map.keys() {
            for (i, _) in p.match_indices('/') {
                if map.contains_key(&p[..i]) {
                    return Err(error("archive file is an ancestor of another member"));
                }
            }
        }
        Ok(Self {
            handler,
            members: map,
            context,
            answers,
            cancel,
            ordinal: 0,
            files,
            effects: vec![],
            warnings: vec![],
        })
    }
    fn source(&self, path: &str, kind: OmodFileKind) -> E<&'a OmodMember> {
        let path = checked_path(path)?;
        self.members
            .get(&path.to_ascii_lowercase())
            .copied()
            .filter(|m| m.kind == kind)
            .ok_or_else(|| error(format!("required {kind:?} source is absent: {path}")))
    }
    fn copy(&mut self, source: &str, destination: &str, kind: OmodFileKind) -> E<()> {
        check_cancel(self.cancel)?;
        let member = self.source(source, kind)?;
        let destination = checked_path(destination)?;
        self.files.insert(
            destination.to_ascii_lowercase(),
            ObmmFile {
                kind,
                source: checked_path(&member.path)?,
                destination,
            },
        );
        Ok(())
    }
    fn data(&mut self, path: &str) -> E<()> {
        self.copy(path, path, OmodFileKind::Data)
    }
    fn tree(&mut self, source: &str, destination: &str) -> E<()> {
        let source = checked_path(source.trim_end_matches(['\\', '/']))?;
        let destination = destination.trim_end_matches(['\\', '/']);
        if !destination.is_empty() {
            checked_path(destination)?;
        }
        let paths: Vec<_> = self
            .members
            .values()
            .filter(|m| m.kind == OmodFileKind::Data)
            .filter_map(|m| {
                let path = m.path.replace('\\', "/");
                under(&path, &source).then_some(path)
            })
            .collect();
        if paths.is_empty() {
            return Err(error(format!("required source folder is absent: {source}")));
        }
        for path in paths {
            let tail = path[source.len()..].trim_start_matches('/');
            let output = if destination.is_empty() {
                tail.into()
            } else {
                format!("{destination}/{tail}")
            };
            self.copy(&path, &output, OmodFileKind::Data)?;
        }
        Ok(())
    }
    fn remove(&mut self, path: &str) {
        self.files.remove(&path.to_ascii_lowercase());
    }
    fn effect(&mut self, line: usize, command: &str, args: &[&str]) {
        self.effects.push(ObmmEffect {
            line,
            command: command.into(),
            arguments: args.iter().map(|s| s.to_string()).collect(),
        });
    }
    fn exists(&self, path: &str) -> bool {
        self.context
            .files
            .iter()
            .any(|p| p.replace('\\', "/").eq_ignore_ascii_case(path))
    }
    fn active(&self, names: &[&str]) -> bool {
        names.iter().any(|n| {
            self.context
                .plugins
                .iter()
                .any(|(p, a)| *a && p.eq_ignore_ascii_case(n))
        })
    }
    fn ini(&self, section: &str, key: &str) -> Option<&str> {
        self.context
            .ini
            .iter()
            .find(|(s, _)| s.trim_matches(['[', ']']).eq_ignore_ascii_case(section))
            .and_then(|(_, values)| {
                values
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v.as_str())
            })
    }
    fn version(&self, minimum: &[u32]) -> E<()> {
        let v = self
            .context
            .versions
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("OBMM"))
            .map(|(_, v)| v)
            .ok_or_else(|| error("OBMM compatibility version is unknown"))?;
        let mut actual = [0u32; 4];
        for (i, p) in v.split('.').enumerate() {
            if i >= 4 {
                return Err(error("invalid compatibility version"));
            }
            actual[i] = p
                .parse()
                .map_err(|_| error("invalid compatibility version"))?;
        }
        let mut required = [0; 4];
        required[..minimum.len()].copy_from_slice(minimum);
        if actual < required {
            return Err(error(format!(
                "requires OBMM compatibility version {} or later",
                minimum
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(".")
            )));
        }
        Ok(())
    }
    fn ask(&mut self, line: usize, kind: ObmmPromptKind) -> Step<ObmmAnswer> {
        check_cancel(self.cancel)?;
        let prompt = ObmmPrompt {
            line,
            ordinal: self.ordinal,
            kind,
        };
        let Some(record) = self.answers.get(self.ordinal) else {
            return Err(Halt::Prompt(prompt));
        };
        if record.prompt != prompt {
            return Err(error("recorded prompt changed; review the current choices").into());
        }
        self.ordinal += 1;
        if record.answer == ObmmAnswer::Cancel {
            return Err(error("cancelled").into());
        }
        Ok(record.answer.clone())
    }
    fn select(
        &mut self,
        line: usize,
        title: &str,
        many: bool,
        min_choices: usize,
        options: Vec<ObmmOption>,
    ) -> Step<Vec<String>> {
        let labels: Vec<_> = options.iter().map(|o| o.label.clone()).collect();
        let answer = self.ask(
            line,
            ObmmPromptKind::Select {
                title: title.into(),
                many,
                min_choices,
                options,
            },
        )?;
        let ObmmAnswer::Select(indices) = answer else {
            return Err(error("expected a selection answer").into());
        };
        let chosen: BTreeSet<_> = indices.iter().copied().collect();
        if chosen.len() != indices.len()
            || chosen.len() < min_choices
            || (!many && chosen.len() != 1)
            || chosen.iter().any(|i| *i >= labels.len())
        {
            return Err(error("invalid selection count or index").into());
        }
        Ok(chosen.into_iter().map(|i| labels[i].clone()).collect())
    }
    fn message(&mut self, line: usize, title: &str, message: &str) -> Step<()> {
        if self.ask(
            line,
            ObmmPromptKind::Message {
                title: title.into(),
                message: message.into(),
            },
        )? != ObmmAnswer::Acknowledge
        {
            return Err(error("message acknowledgement required").into());
        }
        Ok(())
    }
    fn option(&self, label: &str, default: bool) -> ObmmOption {
        let prefix = format!("installfiles/previews/{}.", label.to_ascii_lowercase());
        let preview = self
            .members
            .values()
            .find(|m| {
                let p = m.path.replace('\\', "/").to_ascii_lowercase();
                m.kind == OmodFileKind::Data
                    && p.strip_prefix(&prefix)
                        .is_some_and(|tail| !tail.contains('/'))
            })
            .map(|m| m.path.replace('\\', "/"));
        let desc = format!("installfiles/descriptions/{label}.rtf");
        let description_file = self
            .source(&desc, OmodFileKind::Data)
            .ok()
            .map(|m| m.path.replace('\\', "/"));
        ObmmOption {
            label: label.into(),
            description: None,
            description_file,
            preview,
            default,
        }
    }
}

fn validate_files(files: &BTreeMap<String, ObmmFile>) -> E<()> {
    for p in files.keys() {
        for (i, _) in p.match_indices('/') {
            if files.contains_key(&p[..i]) {
                return Err(error("planned file is an ancestor of another output"));
            }
        }
    }
    Ok(())
}
