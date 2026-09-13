//! Pure, bounded OBMM evaluation. No filesystem or process operations.
//!
//! Behavior reference: OMODFramework 05b3d5629124cfa44308b3b78c38959b960b3730,
//! Copyright (C) 2019-2020 erri120, GPL-3.0-only. Its OBMM implementation
//! credits the original Oblivion Mod Manager (GPLv2), Timeslip, Nexus mod 2097.
//! This Rust implementation was written for Eidos in September 2026 and adds
//! checked bounds, explicit prompts/proposals, and deterministic answer replay.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use super::{member_path, OmodFileKind, OmodMember};
#[path = "obmm_expression.rs"]
mod expression;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObmmError {
    pub line: usize,
    pub message: String,
}
impl std::fmt::Display for ObmmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OBMM line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for ObmmError {}

/// A caller-owned snapshot of actual virtual winners and current profile state.
/// Paths/keys are compared without ASCII case sensitivity. No physical scan is
/// performed here. Callers retain and revalidate the snapshot's identity.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ObmmContext {
    pub files: BTreeSet<String>,
    pub plugins: BTreeMap<String, bool>,
    pub active_mods: BTreeSet<String>,
    /// Keys: OBMM, Oblivion, OBSE, OBGE, or an OBSE plugin's filename.
    /// Absence means unknown; a requested comparison then fails explicitly.
    pub versions: BTreeMap<String, String>,
    pub script_extender_present: bool,
    pub graphics_extender_present: bool,
    pub ini: BTreeMap<String, BTreeMap<String, String>>,
    pub renderer: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObmmOption {
    pub label: String,
    pub description: Option<String>,
    pub description_file: Option<String>,
    /// Data-payload-relative member; never a host filesystem path.
    pub preview: Option<String>,
    pub default: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObmmPromptKind {
    Select {
        title: String,
        many: bool,
        min_choices: usize,
        options: Vec<ObmmOption>,
    },
    YesNo {
        message: String,
        title: String,
    },
    Input {
        title: String,
        initial: String,
        max_length: Option<usize>,
    },
    Message {
        message: String,
        title: String,
    },
    Preview {
        path: String,
        title: String,
        image: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObmmPrompt {
    pub line: usize,
    pub ordinal: usize,
    pub kind: ObmmPromptKind,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObmmAnswer {
    Select(Vec<usize>),
    YesNo(bool),
    Text(String),
    Acknowledge,
    Cancel,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObmmRecordedAnswer {
    pub prompt: ObmmPrompt,
    pub answer: ObmmAnswer,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObmmFile {
    pub kind: OmodFileKind,
    pub source: String,
    pub destination: String,
}
/// Validated requests only. An evaluator never marks an effect applied.
/// The caller must apply supported requests in owned staging/profile workflows
/// and explicitly review every unsupported request before publication.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObmmEffect {
    pub line: usize,
    pub command: String,
    pub arguments: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObmmPlan {
    pub files: Vec<ObmmFile>,
    pub effects: Vec<ObmmEffect>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObmmEvaluation {
    NeedPrompt(ObmmPrompt),
    Complete(ObmmPlan),
}

const MAX_SCRIPT: usize = 1024 * 1024;
const MAX_LINES: usize = 20_000;
const MAX_STRING: usize = 65_536;
const MAX_DEPTH: usize = 64;
const MAX_OUTPUT: usize = 100_000;
const MAX_STEPS: usize = 100_000;
const MAX_WORK: usize = 2_000_000;

fn err(line: usize, message: impl Into<String>) -> ObmmError {
    ObmmError {
        line,
        message: message.into(),
    }
}
fn bounded(value: String, line: usize) -> Result<String, ObmmError> {
    if value.len() > MAX_STRING || value.contains('\0') {
        Err(err(line, "string exceeds bounds or contains NUL"))
    } else {
        Ok(value)
    }
}
fn path(value: &str, line: usize) -> Result<String, ObmmError> {
    member_path(value).map_err(|e| err(line, e.to_string()))
}
fn folder(value: &str, line: usize) -> Result<String, ObmmError> {
    if value.is_empty() {
        Ok(String::new())
    } else {
        path(value.trim_end_matches(['/', '\\']), line)
    }
}
fn boolean(value: &str, line: usize) -> Result<bool, ObmmError> {
    if value.eq_ignore_ascii_case("True") {
        Ok(true)
    } else if value.eq_ignore_ascii_case("False") {
        Ok(false)
    } else {
        Err(err(line, "expected True or False"))
    }
}
fn number<T: std::str::FromStr>(value: &str, line: usize) -> Result<T, ObmmError> {
    value
        .parse()
        .map_err(|_| err(line, format!("invalid numeric operand {value:?}")))
}
fn float(value: &str, line: usize) -> Result<f64, ObmmError> {
    let v: f64 = number(value, line)?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(err(line, "non-finite numeric operand"))
    }
}
fn arity(op: &str) -> Option<(usize, usize)> {
    Some(match op {
        "Else"
        | "Default"
        | "EndIf"
        | "EndSelect"
        | "EndFor"
        | "Break"
        | "Continue"
        | "Exit"
        | "Return"
        | "FatalError"
        | "AllowRunOnLines"
        | "DontInstallAnyPlugins"
        | "DontInstallAnyDataFiles"
        | "InstallAllPlugins"
        | "InstallAllDataFiles" => (0, 0),
        "If" | "IfNot" => (1, 4),
        "For" => (4, 6),
        "Select"
        | "SelectMany"
        | "SelectWithPreview"
        | "SelectManyWithPreview"
        | "SelectWithDescriptions"
        | "SelectManyWithDescriptions"
        | "SelectWithDescriptionsAndPreviews"
        | "SelectManyWithDescriptionsAndPreviews" => (2, 4096),
        "SelectVar"
        | "SelectString"
        | "Label"
        | "Goto"
        | "ExecLine"
        | "LoadEarly"
        | "InstallPlugin"
        | "DontInstallPlugin"
        | "InstallDataFile"
        | "DontInstallDataFile"
        | "RegisterBSA"
        | "UnregisterBSA"
        | "UncheckESP" => (1, 1),
        "Case" | "Comment" => (1, 4096),
        "Message" | "DisplayImage" | "DisplayText" => (1, 2),
        "InputString" => (1, 3),
        "InstallDataFolder" | "DontInstallDataFolder" => (1, 2),
        "CopyDataFolder" | "PatchPlugin" | "PatchDataFile" => (2, 3),
        "SetVar"
        | "LoadBefore"
        | "LoadAfter"
        | "SetDeactivationWarning"
        | "CopyDataFile"
        | "CopyPlugin"
        | "GetFolderName"
        | "GetDirectoryName"
        | "GetFileName"
        | "GetFileNameWithoutExtension"
        | "StringLength"
        | "ReadRendererInfo" => (2, 2),
        "Substring" | "RemoveString" => (3, 4),
        "iSet" | "fSet" => (2, 4096),
        "EditINI" | "EditSDP" | "EditShader" | "SetGMST" | "SetGlobal" | "SetPluginByte"
        | "SetPluginShort" | "SetPluginInt" | "SetPluginLong" | "SetPluginFloat"
        | "CombinePaths" | "ReadINI" | "EditXMLLine" | "EditXMLReplace" => (3, 3),
        "ConflictsWith" | "DependsOn" | "ConflictsWithRegex" | "DependsOnRegex" => (1, 7),
        _ => return None,
    })
}

#[derive(Debug)]
struct Instruction {
    line: usize,
    op: String,
    args: Vec<String>,
    end: usize,
    next: usize,
    scope: Vec<usize>,
}
#[derive(Debug)]
pub struct ObmmProgram {
    code: Vec<Instruction>,
    labels: BTreeMap<String, usize>,
}
impl ObmmProgram {
    pub fn parse(source: &str) -> Result<Self, ObmmError> {
        if source.len() > MAX_SCRIPT || source.contains('\0') {
            return Err(err(1, "script exceeds bounds or contains NUL"));
        }
        let run_on = source.lines().any(|l| l.trim() == "AllowRunOnLines");
        let mut code: Vec<Instruction> = Vec::new();
        let mut continued = String::new();
        let mut first_line = 1;
        for (n, raw) in source.lines().enumerate() {
            if n >= MAX_LINES {
                return Err(err(n + 1, "script line budget exceeded"));
            }
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            if continued.is_empty() {
                first_line = n + 1;
            }
            let tail = raw.ends_with('\\');
            if tail && !run_on {
                return Err(err(n + 1, "continuation requires AllowRunOnLines"));
            }
            continued.push_str(if tail {
                raw[..raw.len() - 1].trim_end()
            } else {
                raw
            });
            if continued.len() > MAX_STRING {
                return Err(err(first_line, "logical line exceeds string bound"));
            }
            if tail {
                continued.push(' ');
                continue;
            }
            let words = tokenize(&continued, first_line)?;
            continued.clear();
            if words.is_empty() {
                continue;
            }
            let (op, args) = (words[0].clone(), words[1..].to_vec());
            let (min, max) =
                arity(&op).ok_or_else(|| err(first_line, format!("unsupported command {op}")))?;
            if args.len() < min || args.len() > max {
                return Err(err(first_line, format!("invalid argument count for {op}")));
            }
            code.push(Instruction {
                line: first_line,
                op,
                args,
                end: 0,
                next: 0,
                scope: Vec::new(),
            });
        }
        if !continued.is_empty() {
            return Err(err(first_line, "unterminated continuation"));
        }
        let mut stack: Vec<usize> = Vec::new();
        let mut labels = BTreeMap::new();
        for i in 0..code.len() {
            code[i].scope = stack.clone();
            match code[i].op.as_str() {
                "If" | "IfNot" | "For" => stack.push(i),
                op if op.starts_with("Select") => stack.push(i),
                "Else" => {
                    let s = *stack
                        .last()
                        .ok_or_else(|| err(code[i].line, "Else without If"))?;
                    if !matches!(code[s].op.as_str(), "If" | "IfNot") || code[s].next != 0 {
                        return Err(err(code[i].line, "unexpected or duplicate Else"));
                    }
                    code[s].next = i;
                }
                "Case" | "Default" => {
                    let s = *stack
                        .last()
                        .ok_or_else(|| err(code[i].line, "Case outside Select"))?;
                    if !code[s].op.starts_with("Select") {
                        return Err(err(code[i].line, "Case outside Select"));
                    }
                    let previous = (s + 1..i).rev().find(|&j| {
                        code[j].scope == stack && matches!(code[j].op.as_str(), "Case" | "Default")
                    });
                    if let Some(j) = previous {
                        if code[j].op == "Default" {
                            return Err(err(code[i].line, "Default must be the final case"));
                        }
                        code[j].next = i;
                    } else {
                        code[s].next = i;
                    }
                }
                "EndIf" | "EndFor" | "EndSelect" => {
                    let s = stack
                        .pop()
                        .ok_or_else(|| err(code[i].line, "unmatched block end"))?;
                    let valid = match code[i].op.as_str() {
                        "EndIf" => matches!(code[s].op.as_str(), "If" | "IfNot"),
                        "EndFor" => code[s].op == "For",
                        _ => code[s].op.starts_with("Select"),
                    };
                    if !valid {
                        return Err(err(code[i].line, "mismatched block end"));
                    }
                    code[s].end = i;
                    for child in &mut code[s + 1..i] {
                        if child.scope.last() == Some(&s)
                            && matches!(child.op.as_str(), "Else" | "Case" | "Default")
                        {
                            child.end = i;
                            if child.next == 0 {
                                child.next = i;
                            }
                        }
                    }
                    if code[s].op.starts_with("Select") && code[s].next == 0 {
                        return Err(err(code[s].line, "Select requires a Case or Default"));
                    }
                }
                "Label" => {
                    if code[i].args[0].contains('%')
                        || labels.insert(code[i].args[0].clone(), i).is_some()
                    {
                        return Err(err(code[i].line, "dynamic or duplicate label"));
                    }
                }
                _ => {}
            }
            if stack.len() > MAX_DEPTH {
                return Err(err(code[i].line, "block depth exceeded"));
            }
        }
        if let Some(s) = stack.first() {
            return Err(err(code[*s].line, "unclosed block"));
        }
        for inst in &code {
            if inst.op == "Goto" {
                let target = *labels
                    .get(&inst.args[0])
                    .ok_or_else(|| err(inst.line, "unknown or dynamic Goto label"))?;
                if !inst.scope.starts_with(&code[target].scope) {
                    return Err(err(
                        inst.line,
                        "Goto cannot enter a different control block",
                    ));
                }
            }
            if matches!(inst.op.as_str(), "Break" | "Continue" | "Exit") {
                let found = inst.scope.iter().any(|s| {
                    if inst.op == "Break" {
                        code[*s].op.starts_with("Select")
                    } else {
                        code[*s].op == "For"
                    }
                });
                if !found {
                    return Err(err(
                        inst.line,
                        "flow instruction has no enclosing Select/For",
                    ));
                }
            }
        }
        Ok(Self { code, labels })
    }
    pub fn evaluate(
        &self,
        members: &[OmodMember],
        context: &ObmmContext,
        answers: &[ObmmRecordedAnswer],
        cancel: &AtomicBool,
    ) -> Result<ObmmEvaluation, ObmmError> {
        Machine::new(self, members, context, answers, cancel)?.run()
    }
}

fn tokenize(raw: &str, line: usize) -> Result<Vec<String>, ObmmError> {
    if raw.starts_with(';') || raw == "Comment" || raw.starts_with("Comment ") {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quote = false;
    let mut started = false;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quote = !quote;
                started = true;
            }
            '\\' if quote && matches!(chars.peek(), Some('"' | '\\')) => {
                word.push(chars.next().unwrap());
                started = true;
            }
            ';' if !quote => break,
            ' ' | '\t' | ',' if !quote => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            _ => {
                word.push(c);
                started = true;
            }
        }
    }
    if quote {
        return Err(err(line, "unterminated quote"));
    }
    if started {
        out.push(word);
    }
    Ok(out)
}

struct Selection {
    values: BTreeSet<String>,
    many: bool,
    hit: bool,
}
struct Loop {
    variable: String,
    values: Vec<String>,
    index: usize,
}
struct Machine<'a> {
    program: &'a ObmmProgram,
    context: &'a ObmmContext,
    answers: &'a [ObmmRecordedAnswer],
    cancel: &'a AtomicBool,
    members: BTreeMap<String, &'a OmodMember>,
    files: BTreeMap<String, ObmmFile>,
    variables: BTreeMap<String, String>,
    selections: BTreeMap<usize, Selection>,
    loops: BTreeMap<usize, Loop>,
    effects: Vec<ObmmEffect>,
    pending: Option<ObmmPrompt>,
    answer_index: usize,
    pc: usize,
    current_line: usize,
    work: usize,
}
fn key(kind: OmodFileKind, path: &str) -> String {
    format!(
        "{}:{}",
        if kind == OmodFileKind::Data { 'd' } else { 'p' },
        path.to_ascii_lowercase()
    )
}
impl<'a> Machine<'a> {
    fn new(
        program: &'a ObmmProgram,
        members: &'a [OmodMember],
        context: &'a ObmmContext,
        answers: &'a [ObmmRecordedAnswer],
        cancel: &'a AtomicBool,
    ) -> Result<Self, ObmmError> {
        if members.len() > MAX_OUTPUT || answers.len() > MAX_LINES {
            return Err(err(1, "input count exceeds budget"));
        }
        let mut sources = BTreeMap::new();
        let mut files = BTreeMap::new();
        for m in members {
            let p = path(&m.path, 1)?;
            if sources.insert(key(m.kind, &p), m).is_some()
                || files
                    .insert(
                        p.to_ascii_lowercase(),
                        ObmmFile {
                            kind: m.kind,
                            source: p.clone(),
                            destination: p,
                        },
                    )
                    .is_some()
            {
                return Err(err(1, "duplicate or case-colliding OMOD source"));
            }
        }
        Ok(Self {
            program,
            context,
            answers,
            cancel,
            members: sources,
            files,
            variables: BTreeMap::from([
                ("NewLine".into(), "\n".into()),
                ("Tab".into(), "\t".into()),
            ]),
            selections: BTreeMap::new(),
            loops: BTreeMap::new(),
            effects: vec![],
            pending: None,
            answer_index: 0,
            pc: 0,
            current_line: 1,
            work: 0,
        })
    }
    fn run(mut self) -> Result<ObmmEvaluation, ObmmError> {
        for _ in 0..MAX_STEPS {
            self.charge(1)?;
            if self.pc >= self.program.code.len() {
                if self.answer_index != self.answers.len() {
                    return Err(err(self.line(), "unused replay answers"));
                }
                for destination in self.files.keys() {
                    for (end, _) in destination.match_indices('/') {
                        if self.files.contains_key(&destination[..end]) {
                            return Err(err(
                                self.line(),
                                format!(
                                    "selected file is also an ancestor of output {destination:?}"
                                ),
                            ));
                        }
                    }
                }
                let warnings=self.effects.iter().map(|e|format!("Line {}: {} is an unapplied effect proposal; the caller must apply it or explicitly disclose an incomplete installation",e.line,e.command)).collect();
                return Ok(ObmmEvaluation::Complete(ObmmPlan {
                    files: self.files.into_values().collect(),
                    effects: self.effects,
                    warnings,
                }));
            }
            self.step()?;
            if let Some(p) = self.pending.take() {
                return Ok(ObmmEvaluation::NeedPrompt(p));
            }
        }
        Err(err(self.line(), "executed instruction budget exceeded"))
    }
    fn line(&self) -> usize {
        self.current_line
    }
    fn charge(&mut self, n: usize) -> Result<(), ObmmError> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(err(self.line(), "evaluation canceled"));
        }
        self.work = self.work.saturating_add(n);
        if self.work > MAX_WORK {
            Err(err(self.line(), "evaluation work budget exceeded"))
        } else {
            Ok(())
        }
    }
    fn expand(&self, s: &str) -> Result<String, ObmmError> {
        let parts: Vec<_> = s.split('%').collect();
        let mut out = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i % 2 == 1 && i + 1 < parts.len() {
                out.push_str(
                    self.variables
                        .get(*part)
                        .ok_or_else(|| err(self.line(), format!("unknown variable {part}")))?,
                );
            } else {
                if i % 2 == 1 {
                    out.push('%');
                }
                out.push_str(part);
            }
            if out.len() > MAX_STRING {
                return Err(err(self.line(), "expanded string exceeds bound"));
            }
        }
        Ok(out)
    }
    fn set(&mut self, name: &str, value: String) -> Result<(), ObmmError> {
        if name.is_empty()
            || name.len() > 256
            || name.contains(['%', '\0'])
            || self.variables.len() >= 4096 && !self.variables.contains_key(name)
        {
            return Err(err(
                self.line(),
                "invalid variable name or variable budget exceeded",
            ));
        }
        self.variables
            .insert(name.into(), bounded(value, self.line())?);
        Ok(())
    }
    fn ask(&mut self, kind: ObmmPromptKind) -> Result<Option<ObmmAnswer>, ObmmError> {
        let prompt = ObmmPrompt {
            line: self.line(),
            ordinal: self.answer_index,
            kind,
        };
        let Some(record) = self.answers.get(self.answer_index) else {
            self.pending = Some(prompt);
            return Ok(None);
        };
        if record.prompt != prompt {
            return Err(err(self.line(), "replay prompt mismatch"));
        }
        if record.answer == ObmmAnswer::Cancel {
            return Err(err(self.line(), "prompt canceled"));
        }
        match (&prompt.kind, &record.answer) {
            (
                ObmmPromptKind::Select {
                    many,
                    min_choices,
                    options,
                    ..
                },
                ObmmAnswer::Select(indices),
            ) => {
                let unique: BTreeSet<_> = indices.iter().collect();
                if indices.len() < *min_choices
                    || (!many && indices.len() != 1)
                    || unique.len() != indices.len()
                    || indices.iter().any(|i| *i >= options.len())
                {
                    return Err(err(self.line(), "invalid selected option indices"));
                }
            }
            (ObmmPromptKind::YesNo { .. }, ObmmAnswer::YesNo(_))
            | (
                ObmmPromptKind::Message { .. } | ObmmPromptKind::Preview { .. },
                ObmmAnswer::Acknowledge,
            ) => {}
            (ObmmPromptKind::Input { max_length, .. }, ObmmAnswer::Text(s)) => {
                if max_length.is_some_and(|max| s.chars().count() > max) {
                    return Err(err(self.line(), "input exceeds prompt length bound"));
                }
                bounded(s.clone(), self.line())?;
            }
            _ => return Err(err(self.line(), "answer type does not match prompt")),
        }
        self.answer_index += 1;
        Ok(Some(record.answer.clone()))
    }
    fn source(&self, kind: OmodFileKind, name: &str) -> Result<&'a OmodMember, ObmmError> {
        let p = path(name, self.line())?;
        self.members
            .get(&key(kind, &p))
            .copied()
            .ok_or_else(|| err(self.line(), format!("missing OMOD source {name:?}")))
    }
    fn put(&mut self, kind: OmodFileKind, from: &str, to: &str) -> Result<(), ObmmError> {
        let m = self.source(kind, from)?;
        let destination = path(to, self.line())?;
        if self.files.len() >= MAX_OUTPUT
            && !self.files.contains_key(&destination.to_ascii_lowercase())
        {
            return Err(err(self.line(), "output path budget exceeded"));
        }
        self.files.insert(
            destination.to_ascii_lowercase(),
            ObmmFile {
                kind,
                source: m.path.replace('\\', "/"),
                destination,
            },
        );
        Ok(())
    }
    fn jump(&mut self, target: usize) {
        let scope = self
            .program
            .code
            .get(target)
            .map(|i| i.scope.as_slice())
            .unwrap_or(&[]);
        self.loops.retain(|k, _| scope.contains(k));
        self.selections.retain(|k, _| scope.contains(k));
        self.pc = target;
    }
    fn step(&mut self) -> Result<(), ObmmError> {
        let at = self.pc;
        let inst = &self.program.code[at];
        let line = inst.line;
        self.current_line = line;
        let op = inst.op.as_str();
        let mut a = Vec::with_capacity(inst.args.len());
        for argument in &inst.args {
            let value = self.expand(argument)?;
            // Bound accumulated expanded values as well as instruction count;
            // a small loop must not retain gigabytes of effect arguments.
            self.charge(value.len())?;
            a.push(value);
        }
        self.pc += 1;
        match op {
            "If" | "IfNot" => {
                let Some(value) = self.condition(&a, line)? else {
                    return Ok(());
                };
                if value == (op == "IfNot") {
                    self.pc = if inst.next != 0 {
                        inst.next + 1
                    } else {
                        inst.end + 1
                    };
                }
            }
            "Else" => self.pc = inst.end + 1,
            "EndIf" | "Label" | "AllowRunOnLines" => {}
            "Goto" => self.jump(self.program.labels[&a[0]]),
            "Return" => self.jump(self.program.code.len()),
            "FatalError" => return Err(err(line, "FatalError requested by script")),
            "ExecLine" => {
                return Err(err(
                    line,
                    "ExecLine is unsupported; dynamic code was not executed",
                ))
            }
            op if op.starts_with("Select") => {
                let many = op.starts_with("SelectMany");
                let values = if op == "SelectVar" {
                    BTreeSet::from([self
                        .variables
                        .get(&a[0])
                        .cloned()
                        .ok_or_else(|| err(line, "unknown SelectVar variable"))?])
                } else if op == "SelectString" {
                    BTreeSet::from([a[0].clone()])
                } else {
                    let preview = op.contains("Preview");
                    let description = op.contains("Descriptions");
                    let stride = 1 + usize::from(preview) + usize::from(description);
                    if !(a.len() - 1).is_multiple_of(stride) {
                        return Err(err(line, "incomplete Select option"));
                    }
                    let mut options = a[1..]
                        .chunks(stride)
                        .map(|c| {
                            let image = if preview && c[1] != "None" {
                                Some(
                                    self.source(OmodFileKind::Data, &c[1])?
                                        .path
                                        .replace('\\', "/"),
                                )
                            } else {
                                None
                            };
                            Ok(ObmmOption {
                                label: c[0].replace('|', ""),
                                default: c[0].starts_with('|'),
                                description_file: None,
                                preview: image,
                                description: description.then(|| c[stride - 1].clone()),
                            })
                        })
                        .collect::<Result<Vec<_>, ObmmError>>()?;
                    let labels: BTreeSet<_> = options.iter().map(|o| o.label.as_str()).collect();
                    if labels.len() != options.len() || labels.contains("") {
                        return Err(err(line, "duplicate or empty Select option label"));
                    }
                    if !many && !options.iter().any(|option| option.default) {
                        options[0].default = true;
                    }
                    let Some(ObmmAnswer::Select(indices)) = self.ask_at(
                        line,
                        ObmmPromptKind::Select {
                            title: a[0].clone(),
                            many,
                            min_choices: 1,
                            options: options.clone(),
                        },
                    )?
                    else {
                        return Ok(());
                    };
                    indices
                        .into_iter()
                        .map(|i| options[i].label.clone())
                        .collect()
                };
                self.selections.insert(
                    at,
                    Selection {
                        values,
                        many,
                        hit: false,
                    },
                );
                self.pc = inst.next;
            }
            "Case" | "Default" => {
                let start = *inst.scope.last().unwrap();
                let sel = self
                    .selections
                    .get_mut(&start)
                    .ok_or_else(|| err(line, "Select state is unavailable"))?;
                let active = if op == "Default" {
                    !sel.hit
                } else {
                    (sel.many || !sel.hit) && sel.values.contains(&a.join(" "))
                };
                if active {
                    sel.hit = true;
                } else {
                    self.pc = inst.next;
                }
            }
            "EndSelect" => {
                self.selections.remove(inst.scope.last().unwrap());
            }
            "Break" => {
                let start = *inst
                    .scope
                    .iter()
                    .rev()
                    .find(|s| self.program.code[**s].op.starts_with("Select"))
                    .unwrap();
                let next = (at + 1..=self.program.code[start].end)
                    .find(|i| {
                        self.program.code[*i].scope.last() == Some(&start)
                            && matches!(
                                self.program.code[*i].op.as_str(),
                                "Case" | "Default" | "EndSelect"
                            )
                    })
                    .unwrap();
                self.jump(next);
            }
            "For" => {
                let (variable, values) = self.loop_values(&a, line)?;
                if values.is_empty() {
                    self.pc = inst.end + 1;
                } else {
                    self.set(&variable, values[0].clone())?;
                    self.loops.insert(
                        at,
                        Loop {
                            variable,
                            values,
                            index: 0,
                        },
                    );
                }
            }
            "EndFor" => {
                let start = *inst.scope.last().unwrap();
                let state = self
                    .loops
                    .get_mut(&start)
                    .ok_or_else(|| err(line, "For state is unavailable"))?;
                state.index += 1;
                if state.index < state.values.len() {
                    let (name, value) = (state.variable.clone(), state.values[state.index].clone());
                    self.set(&name, value)?;
                    self.pc = start + 1;
                } else {
                    self.loops.remove(&start);
                }
            }
            "Continue" | "Exit" => {
                let start = *inst
                    .scope
                    .iter()
                    .rev()
                    .find(|s| self.program.code[**s].op == "For")
                    .unwrap();
                self.jump(self.program.code[start].end + usize::from(op == "Exit"));
            }
            "SetVar" => self.set(&a[0], a[1].clone())?,
            "DontInstallAnyDataFiles"
            | "DontInstallAnyPlugins"
            | "InstallAllDataFiles"
            | "InstallAllPlugins" => {
                let kind = if op.ends_with("Plugins") {
                    OmodFileKind::Plugin
                } else {
                    OmodFileKind::Data
                };
                self.files.retain(|_, f| f.kind != kind);
                if op.starts_with("InstallAll") {
                    let sources: Vec<_> = self
                        .members
                        .values()
                        .filter(|m| m.kind == kind)
                        .map(|m| m.path.clone())
                        .collect();
                    self.charge(sources.len())?;
                    for source in sources {
                        self.put(kind, &source, &source)?;
                    }
                }
            }
            "InstallDataFile" | "InstallPlugin" | "CopyDataFile" | "CopyPlugin" => {
                let kind = if op.ends_with("Plugin") {
                    OmodFileKind::Plugin
                } else {
                    OmodFileKind::Data
                };
                self.put(kind, &a[0], a.get(1).unwrap_or(&a[0]))?;
            }
            "DontInstallPlugin" | "DontInstallDataFile" => {
                let kind = if op.ends_with("Plugin") {
                    OmodFileKind::Plugin
                } else {
                    OmodFileKind::Data
                };
                let source = self.source(kind, &a[0])?.path.replace('\\', "/");
                self.files
                    .retain(|_, f| !(f.kind == kind && f.source.eq_ignore_ascii_case(&source)));
            }
            _ => self.command(op, &a, line)?,
        }
        Ok(())
    }
    fn ask_at(
        &mut self,
        line: usize,
        kind: ObmmPromptKind,
    ) -> Result<Option<ObmmAnswer>, ObmmError> {
        debug_assert_eq!(line, self.current_line);
        self.ask(kind)
    }
    fn condition(&mut self, a: &[String], line: usize) -> Result<Option<bool>, ObmmError> {
        let args = &a[1..];
        let need = match a[0].as_str() {
            "ScriptExtenderPresent" | "GraphicsExtenderPresent" => 0,
            "Equal" | "GreaterThan" | "GreaterEqual" | "fGreaterThan" | "fGreaterEqual" => 2,
            "DialogYesNo" => args.len(),
            _ => 1,
        };
        if args.len() != need || (a[0] == "DialogYesNo" && !(1..=2).contains(&need)) {
            return Err(err(line, "invalid condition argument count"));
        }
        Ok(Some(match a[0].as_str() {
            "Equal" => args[0] == args[1],
            "GreaterThan" => number::<i32>(&args[0], line)? > number::<i32>(&args[1], line)?,
            "GreaterEqual" => number::<i32>(&args[0], line)? >= number::<i32>(&args[1], line)?,
            "fGreaterThan" => float(&args[0], line)? > float(&args[1], line)?,
            "fGreaterEqual" => float(&args[0], line)? >= float(&args[1], line)?,
            "DialogYesNo" => {
                let Some(ObmmAnswer::YesNo(answer)) = self.ask_at(
                    line,
                    ObmmPromptKind::YesNo {
                        message: args[0].clone(),
                        title: args.get(1).cloned().unwrap_or_default(),
                    },
                )?
                else {
                    return Ok(None);
                };
                answer
            }
            "ScriptExtenderPresent" => self.context.script_extender_present,
            "GraphicsExtenderPresent" => self.context.graphics_extender_present,
            "DataFileExists" => {
                let p = path(&args[0], line)?;
                self.context
                    .files
                    .iter()
                    .any(|s| s.replace('\\', "/").eq_ignore_ascii_case(&p))
            }
            "PluginExists" | "PluginActive" => self.context.plugins.iter().any(|(p, active)| {
                p.eq_ignore_ascii_case(&args[0]) && (a[0] == "PluginExists" || *active)
            }),
            "ActiveMod" | "ActiveOmod" => self
                .context
                .active_mods
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&args[0])),
            "VersionGreaterThan"
            | "VersionLessThan"
            | "ScriptExtenderNewerThan"
            | "GraphicsExtenderNewerThan"
            | "OblivionNewerThan" => {
                let key = match a[0].as_str() {
                    "ScriptExtenderNewerThan" => "OBSE",
                    "GraphicsExtenderNewerThan" => "OBGE",
                    "OblivionNewerThan" => "Oblivion",
                    _ => "OBMM",
                };
                let observed = self
                    .context
                    .versions
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v)
                    .ok_or_else(|| err(line, format!("missing {key} version evidence")))?;
                let comparison = version(observed, line)?.cmp(&version(&args[0], line)?);
                if a[0] == "VersionLessThan" {
                    comparison.is_lt()
                } else {
                    comparison.is_gt()
                }
            }
            _ => return Err(err(line, format!("unsupported condition {}", a[0]))),
        }))
    }
    fn loop_values(
        &mut self,
        a: &[String],
        line: usize,
    ) -> Result<(String, Vec<String>), ObmmError> {
        if a[0] == "Count" {
            if !(4..=5).contains(&a.len()) {
                return Err(err(line, "invalid For Count arguments"));
            }
            let mut current = number::<i32>(&a[2], line)?;
            let end = number::<i32>(&a[3], line)?;
            let step = if a.len() == 5 {
                number::<i32>(&a[4], line)?
            } else {
                1
            };
            if step == 0 {
                return Err(err(line, "For Count step must not be zero"));
            }
            let mut values = vec![];
            while if step > 0 {
                current <= end
            } else {
                current >= end
            } {
                self.charge(1)?;
                if values.len() >= MAX_OUTPUT {
                    return Err(err(line, "loop value budget exceeded"));
                }
                values.push(current.to_string());
                if current == end {
                    break;
                }
                let Some(next) = current.checked_add(step) else {
                    break;
                };
                current = next;
            }
            Ok((a[1].clone(), values))
        } else {
            if a[0] != "Each" || !(4..=6).contains(&a.len()) {
                return Err(err(line, "invalid For Each arguments"));
            }
            let kind = match a[1].as_str() {
                "DataFile" | "DataFolder" => OmodFileKind::Data,
                "Plugin" | "PluginFolder" => OmodFileKind::Plugin,
                _ => return Err(err(line, "unsupported For enumeration")),
            };
            let recurse = if a.len() > 4 {
                boolean(&a[4], line)?
            } else {
                false
            };
            let pattern = a.get(5).map(String::as_str).unwrap_or("*");
            Ok((
                a[2].clone(),
                self.enumerate(kind, &a[3], recurse, a[1].ends_with("Folder"), pattern)?,
            ))
        }
    }
    fn enumerate(
        &mut self,
        kind: OmodFileKind,
        root: &str,
        recurse: bool,
        folders: bool,
        pattern: &str,
    ) -> Result<Vec<String>, ObmmError> {
        let root = folder(root, self.line())?;
        if pattern.len() > 256 || pattern.contains(['/', '\\', ':']) {
            return Err(err(self.line(), "invalid enumeration pattern"));
        }
        self.charge(self.members.len())?;
        let prefix = if root.is_empty() {
            String::new()
        } else {
            format!("{}/", root.to_ascii_lowercase())
        };
        let mut entries = BTreeMap::new();
        let mut exists = root.is_empty();
        for m in self.members.values().filter(|m| m.kind == kind) {
            let p = m.path.replace('\\', "/");
            if !p.to_ascii_lowercase().starts_with(&prefix) {
                continue;
            }
            exists = true;
            if folders {
                for (i, _) in p.match_indices('/').filter(|(i, _)| *i >= prefix.len()) {
                    let candidate = &p[..i];
                    let relative = &candidate[prefix.len()..];
                    if (recurse || !relative.contains('/'))
                        && wildcard(pattern, relative.rsplit('/').next().unwrap())
                    {
                        entries
                            .entry(candidate.to_ascii_lowercase())
                            .or_insert_with(|| candidate.to_string());
                    }
                }
            } else {
                let relative = &p[prefix.len()..];
                if (recurse || !relative.contains('/'))
                    && wildcard(pattern, relative.rsplit('/').next().unwrap())
                {
                    entries.entry(p.to_ascii_lowercase()).or_insert(p);
                }
            }
        }
        if !exists {
            return Err(err(self.line(), format!("missing OMOD folder {root:?}")));
        }
        Ok(entries.into_values().collect())
    }
    fn edit_source(&mut self, kind: OmodFileKind, name: &str) -> Result<String, ObmmError> {
        let p = path(name, self.line())?;
        if let Some(file) = self.files.get(&p.to_ascii_lowercase()) {
            if file.kind != kind {
                return Err(err(self.line(), "edit target has the wrong payload kind"));
            }
            return Ok(file.destination.clone());
        }
        if self.members.contains_key(&key(kind, &p)) {
            self.put(kind, &p, &p)?;
            return Ok(p);
        }
        if self
            .context
            .files
            .iter()
            .any(|s| s.replace('\\', "/").eq_ignore_ascii_case(&p))
        {
            // The application layer must copy this exact virtual winner to its
            // unpublished stage before applying the requested edit.
            return Ok(p);
        }
        Err(err(
            self.line(),
            format!("missing selected, archive, or virtual edit source {name:?}"),
        ))
    }
    fn command(&mut self, op: &str, a: &[String], line: usize) -> Result<(), ObmmError> {
        match op {
            "InstallDataFolder" | "DontInstallDataFolder" | "CopyDataFolder" => {
                let recurse = if op == "CopyDataFolder" {
                    a.get(2)
                } else {
                    a.get(1)
                }
                .map(|s| boolean(s, line))
                .transpose()?
                .unwrap_or(false);
                let sources = self.enumerate(OmodFileKind::Data, &a[0], recurse, false, "*")?;
                if op == "DontInstallDataFolder" {
                    let sources: BTreeSet<_> =
                        sources.iter().map(|s| s.to_ascii_lowercase()).collect();
                    self.files.retain(|_, f| {
                        f.kind != OmodFileKind::Data
                            || !sources.contains(&f.source.to_ascii_lowercase())
                    });
                } else {
                    let from = folder(&a[0], line)?;
                    let to = if op == "CopyDataFolder" {
                        folder(&a[1], line)?
                    } else {
                        String::new()
                    };
                    for source in sources {
                        let dest = if op == "CopyDataFolder" {
                            let suffix = if from.is_empty() {
                                source.as_str()
                            } else {
                                &source[from.len() + 1..]
                            };
                            if to.is_empty() {
                                suffix.to_string()
                            } else {
                                format!("{to}/{suffix}")
                            }
                        } else {
                            source.clone()
                        };
                        self.put(OmodFileKind::Data, &source, &dest)?;
                    }
                }
            }
            "Message" => {
                self.ask_at(
                    line,
                    ObmmPromptKind::Message {
                        message: a[0].clone(),
                        title: a.get(1).cloned().unwrap_or_default(),
                    },
                )?;
            }
            "DisplayImage" | "DisplayText" => {
                let p = self
                    .source(OmodFileKind::Data, &a[0])?
                    .path
                    .replace('\\', "/");
                self.ask_at(
                    line,
                    ObmmPromptKind::Preview {
                        path: p,
                        title: a.get(1).cloned().unwrap_or_default(),
                        image: op == "DisplayImage",
                    },
                )?;
            }
            "InputString" => {
                if let Some(ObmmAnswer::Text(value)) = self.ask_at(
                    line,
                    ObmmPromptKind::Input {
                        title: a.get(1).cloned().unwrap_or_default(),
                        initial: a.get(2).cloned().unwrap_or_default(),
                        max_length: Some(MAX_STRING),
                    },
                )? {
                    self.set(&a[0], value)?;
                }
            }
            "ReadINI" => {
                let value = self
                    .context
                    .ini
                    .iter()
                    .find(|(section, _)| section.eq_ignore_ascii_case(&a[1]))
                    .and_then(|(_, keys)| {
                        keys.iter().find(|(key, _)| key.eq_ignore_ascii_case(&a[2]))
                    })
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                self.set(&a[0], value)?;
            }
            "ReadRendererInfo" => {
                let value = self
                    .context
                    .renderer
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(&a[1]))
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                self.set(&a[0], value)?;
            }
            "GetDirectoryName"
            | "GetFolderName"
            | "GetFileName"
            | "GetFileNameWithoutExtension" => {
                let p = a[1].replace('\\', "/");
                let parent = p.rsplit_once('/').map_or("", |(parent, _)| parent);
                let name = p.rsplit('/').next().unwrap();
                let value = match op {
                    "GetDirectoryName" => parent,
                    "GetFolderName" => parent.rsplit('/').next().unwrap(),
                    "GetFileName" => name,
                    _ => name.rsplit_once('.').map_or(name, |(stem, _)| stem),
                };
                self.set(&a[0], value.to_string())?;
            }
            "CombinePaths" => {
                let first = folder(&a[1], line)?;
                let second = folder(&a[2], line)?;
                let value = if first.is_empty() {
                    second
                } else if second.is_empty() {
                    first
                } else {
                    format!("{first}/{second}")
                };
                self.set(&a[0], value)?;
            }
            "StringLength" => self.set(&a[0], a[1].encode_utf16().count().to_string())?,
            "Substring" | "RemoveString" => {
                let units: Vec<u16> = a[1].encode_utf16().collect();
                let start = number::<usize>(&a[2], line)?;
                let count = if a.len() == 4 {
                    number::<usize>(&a[3], line)?
                } else {
                    units
                        .len()
                        .checked_sub(start)
                        .ok_or_else(|| err(line, "string index out of bounds"))?
                };
                let end = start
                    .checked_add(count)
                    .filter(|end| *end <= units.len())
                    .ok_or_else(|| err(line, "string range out of bounds"))?;
                let value = if op == "Substring" {
                    String::from_utf16(&units[start..end])
                } else {
                    let mut result = units[..start].to_vec();
                    result.extend_from_slice(&units[end..]);
                    String::from_utf16(&result)
                }
                .map_err(|_| err(line, "string range splits a UTF-16 surrogate pair"))?;
                self.set(&a[0], value)?;
            }
            "iSet" | "fSet" => {
                self.charge(a.len().saturating_mul(a.len()))?;
                let value = expression::evaluate(&a[1..], op == "iSet", line)?;
                self.set(&a[0], value)?;
            }
            _ => {
                let mut args = a.to_vec();
                match op {
                    "EditINI" => {
                        if a[..2]
                            .iter()
                            .any(|s| s.is_empty() || s.contains(['\r', '\n', '[', ']', '=', '\0']))
                            || a[2].contains(['\r', '\n', '\0'])
                        {
                            return Err(err(line, "invalid INI edit field"));
                        }
                    }
                    "EditXMLLine" | "EditXMLReplace" => {
                        args[0] = self.edit_source(OmodFileKind::Data, &a[0])?;
                        if op == "EditXMLLine" {
                            number::<u32>(&a[1], line)?;
                        } else if a[1].is_empty() {
                            return Err(err(line, "empty XML replacement search"));
                        }
                    }
                    "SetGMST" | "SetGlobal" => {
                        args[0] = self.edit_source(OmodFileKind::Plugin, &a[0])?;
                        if a[1].is_empty() {
                            return Err(err(line, "empty plugin record identifier"));
                        }
                    }
                    "SetPluginByte" | "SetPluginShort" | "SetPluginInt" | "SetPluginLong"
                    | "SetPluginFloat" => {
                        args[0] = self.edit_source(OmodFileKind::Plugin, &a[0])?;
                        let offset = number::<u64>(&a[1], line)?;
                        let width = match op {
                            "SetPluginByte" => {
                                number::<u8>(&a[2], line)?;
                                1
                            }
                            "SetPluginShort" => {
                                number::<i16>(&a[2], line)?;
                                2
                            }
                            "SetPluginInt" => {
                                number::<i32>(&a[2], line)?;
                                4
                            }
                            "SetPluginLong" => {
                                number::<i64>(&a[2], line)?;
                                8
                            }
                            _ => {
                                let v = number::<f32>(&a[2], line)?;
                                if !v.is_finite() {
                                    return Err(err(line, "non-finite plugin value"));
                                }
                                4
                            }
                        };
                        if offset
                            .checked_add(width)
                            .is_none_or(|end| end > 2 * 1024 * 1024 * 1024)
                        {
                            return Err(err(line, "plugin edit offset exceeds file bound"));
                        }
                    }
                    "EditSDP" | "EditShader" => {
                        number::<u8>(&a[0], line)?;
                        if a[1].is_empty() || a[1].len() > 256 || a[1].contains(['/', '\\']) {
                            return Err(err(line, "invalid shader name"));
                        }
                        args[2] = self.edit_source(OmodFileKind::Data, &a[2])?;
                    }
                    "PatchDataFile" | "PatchPlugin" => {
                        let kind = if op == "PatchPlugin" {
                            OmodFileKind::Plugin
                        } else {
                            OmodFileKind::Data
                        };
                        args[0] = self.source(kind, &a[0])?.path.replace('\\', "/");
                        args[1] = path(&a[1], line)?;
                        let create = a
                            .get(2)
                            .map(|v| boolean(v, line))
                            .transpose()?
                            .unwrap_or(false);
                        if !create
                            && !self
                                .context
                                .files
                                .iter()
                                .any(|p| p.replace('\\', "/").eq_ignore_ascii_case(&args[1]))
                        {
                            return Err(err(
                                line,
                                "patch target has no virtual winner and create is False",
                            ));
                        }
                        args.truncate(2);
                        args.push(if create { "True" } else { "False" }.into());
                    }
                    "RegisterBSA" | "UnregisterBSA" => {
                        args[0] = path(&a[0], line)?;
                        if !args[0].to_ascii_lowercase().ends_with(".bsa") {
                            return Err(err(line, "archive registration requires a BSA filename"));
                        }
                    }
                    "UncheckESP"
                    | "LoadEarly"
                    | "LoadBefore"
                    | "LoadAfter"
                    | "SetDeactivationWarning" => {
                        args[0] = path(&a[0], line)?;
                        if matches!(op, "LoadBefore" | "LoadAfter") {
                            args[1] = path(&a[1], line)?;
                        }
                        if op == "SetDeactivationWarning"
                            && !matches!(a[1].as_str(), "Allow" | "WarnAgainst" | "Disallow")
                        {
                            return Err(err(line, "unknown deactivation warning"));
                        }
                    }
                    "ConflictsWith" | "ConflictsWithRegex" | "DependsOn" | "DependsOnRegex" => {
                        if !matches!(a.len(), 1 | 2 | 3 | 5 | 6 | 7) || a[0].is_empty() {
                            return Err(err(line, "invalid conflict/dependency declaration"));
                        }
                        if a.len() >= 5 {
                            for v in &a[1..5] {
                                let n = number::<i32>(v, line)?;
                                if n < 0 {
                                    return Err(err(line, "negative dependency version"));
                                }
                            }
                        }
                        if a.len() == 3 || a.len() == 7 {
                            let severity = a.last().unwrap();
                            if !matches!(severity.as_str(), "Minor" | "Major" | "Unusable") {
                                return Err(err(line, "unknown dependency severity"));
                            }
                        }
                        // Regex matching is deliberately not performed. The full
                        // declaration survives as an explicitly unapplied effect.
                    }
                    _ => return Err(err(line, format!("unsupported command {op}"))),
                }
                if self.effects.len() >= MAX_LINES {
                    return Err(err(line, "effect proposal budget exceeded"));
                }
                self.effects.push(ObmmEffect {
                    line,
                    command: op.into(),
                    arguments: args,
                });
            }
        }
        Ok(())
    }
}
fn version(s: &str, line: usize) -> Result<[i32; 4], ObmmError> {
    let parts: Vec<_> = s.split('.').collect();
    if !(2..=4).contains(&parts.len()) {
        return Err(err(line, "version must contain two to four components"));
    }
    let mut result = [-1; 4];
    for (i, p) in parts.iter().enumerate() {
        let n = number::<i32>(p, line)?;
        if n < 0 {
            return Err(err(line, "negative version component"));
        }
        result[i] = n;
    }
    Ok(result)
}
fn wildcard(pattern: &str, value: &str) -> bool {
    let (p, v) = (pattern.to_ascii_lowercase(), value.to_ascii_lowercase());
    let (p, v) = (p.as_bytes(), v.as_bytes());
    let (mut i, mut j, mut star, mut retry) = (0, 0, None, 0);
    while j < v.len() {
        if i < p.len() && (p[i] == b'?' || p[i] == v[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == b'*' {
            star = Some(i);
            i += 1;
            retry = j;
        } else if let Some(s) = star {
            retry += 1;
            j = retry;
            i = s + 1;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == b'*' {
        i += 1;
    }
    i == p.len()
}

#[cfg(test)]
#[path = "obmm_tests.rs"]
mod tests;
