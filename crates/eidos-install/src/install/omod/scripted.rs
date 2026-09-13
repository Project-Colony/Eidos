//! Scripted OMOD review, owned staging, and retryable profile effects.
//! Every publication goes through the existing installer transaction. No script
//! or effect writes to an original game, archive, or virtual source provider.

use super::super::fsops::checked_destination;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "scripted_context.rs"]
mod context;
#[path = "scripted_effects.rs"]
mod effects;
#[path = "scripted_stage.rs"]
mod stage;
const MAX_EFFECT_FILE: u64 = 64 * 1024 * 1024;
fn bad(message: impl Into<String>) -> InstallError {
    InstallError::BadSelection(message.into())
}
fn check_cancel(cancel: &AtomicBool) -> Result<(), InstallError> {
    if cancel.load(Ordering::Relaxed) {
        Err(bad("OMOD installation cancelled"))
    } else {
        Ok(())
    }
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn json_error(e: serde_json::Error) -> InstallError {
    bad(format!("OMOD receipt: {e}"))
}

use super::known_handlers::KnownPlan;
use super::obmm::*;
use super::{InstallError, InstallReport, OmodSession, OverwritePolicy};

/// Explicit paths from the selected discovered installation. Prefix=None means
/// no fallback prefix reads; it never selects a default Wine prefix.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScriptedGame {
    pub game_id: String,
    pub install_path: PathBuf,
    pub data_path: PathBuf,
    pub prefix: Option<PathBuf>,
    pub observed_versions: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScriptedOrigin {
    pub instance_root: PathBuf,
    pub profile: String,
    pub game: ScriptedGame,
    pub context_sha256: String,
}
#[derive(Debug, Clone)]
pub struct ScriptedContext {
    pub origin: ScriptedOrigin,
    pub interpreter: ObmmContext,
    sources: BTreeMap<String, context::Source>,
    ini_text: String,
    ini_cp1252: bool,
    plugin_list: eidos_plugins::PluginList,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OmodReplayReceipt {
    pub version: u32,
    pub archive_sha256: String,
    pub script_sha256: String,
    pub handler: String,
    pub origin: ScriptedOrigin,
    pub answers: Vec<ObmmRecordedAnswer>,
}
fn handler(session: &OmodSession) -> Result<String, InstallError> {
    let script = session
        .script
        .as_ref()
        .ok_or_else(|| bad("OMOD has no installer script"))?;
    if script.kind == super::OmodScriptKind::Obmm {
        Ok("native-obmm-1.1.12-v1".into())
    } else if let Some(kind) = super::known_handlers::KnownHandler::recognize(script) {
        Ok(format!("native-{kind:?}-v1"))
    } else {
        Err(bad(
            "Unsupported OMOD script language or unrecognized C# installer; no code was executed",
        ))
    }
}
pub fn new_omod_receipt(
    session: &OmodSession,
    context: &ScriptedContext,
    cancel: &AtomicBool,
) -> Result<OmodReplayReceipt, InstallError> {
    check_cancel(cancel)?;
    let handler = handler(session)?;
    session.verify_archive(cancel)?;
    Ok(OmodReplayReceipt {
        version: 1,
        archive_sha256: session.origin.sha256().to_owned(),
        script_sha256: digest(&session.script.as_ref().unwrap().bytes),
        handler,
        origin: context.origin.clone(),
        answers: Vec::new(),
    })
}

#[derive(Debug, Clone)]
pub struct ScriptedReview {
    pub receipt: OmodReplayReceipt,
    pub plan: ObmmPlan,
    /// Native generated destinations, separate from archive-source selections.
    pub generated_files: Vec<String>,
    pub profile_effects: Vec<ObmmEffect>,
    pub unsupported: Vec<String>,
    known: Option<KnownPlan>,
}
#[derive(Debug, Clone)]
pub enum ScriptedEvaluation {
    NeedPrompt(ObmmPrompt),
    Review(Box<ScriptedReview>),
}
pub fn evaluate_scripted_omod(
    session: &OmodSession,
    context: &ScriptedContext,
    receipt: &OmodReplayReceipt,
    cancel: &AtomicBool,
) -> Result<ScriptedEvaluation, InstallError> {
    let mut expected = new_omod_receipt(session, context, cancel)?;
    expected.answers = receipt.answers.clone();
    if &expected != receipt {
        return Err(bad(
            "OMOD replay receipt no longer matches its script, archive, profile or context",
        ));
    }
    if receipt.answers.len() > 100_000 {
        return Err(bad("OMOD answer count exceeds bound"));
    }
    let script = session.script.as_ref().unwrap();
    let (mut plan, known) =
        if let Some(handler) = super::known_handlers::KnownHandler::recognize(script) {
            match handler
                .evaluate(
                    &session.members,
                    &context.interpreter,
                    &receipt.answers,
                    cancel,
                )
                .map_err(|e| bad(e.to_string()))?
            {
                super::known_handlers::KnownEvaluation::NeedPrompt(prompt) => {
                    return Ok(ScriptedEvaluation::NeedPrompt(prompt))
                }
                super::known_handlers::KnownEvaluation::Complete(known) => {
                    (known.plan.clone(), Some(known))
                }
            }
        } else {
            match ObmmProgram::parse(script.source())
                .and_then(|p| {
                    p.evaluate(
                        &session.members,
                        &context.interpreter,
                        &receipt.answers,
                        cancel,
                    )
                })
                .map_err(|e| bad(e.to_string()))?
            {
                ObmmEvaluation::NeedPrompt(prompt) => {
                    return Ok(ScriptedEvaluation::NeedPrompt(prompt))
                }
                ObmmEvaluation::Complete(plan) => (plan, None),
            }
        };
    let mut profile_effects: Vec<_> = plan
        .files
        .iter()
        .filter(|f| f.kind == super::OmodFileKind::Plugin && !f.destination.contains('/'))
        .map(|f| ObmmEffect {
            line: 0,
            command: "ActivatePlugin".into(),
            arguments: vec![f.destination.clone()],
        })
        .collect();
    let mut unsupported = Vec::new();
    for effect in &plan.effects {
        match effect.command.as_str() {
            "EditINI" | "RegisterBSA" | "UnregisterBSA" | "UncheckESP" => {
                profile_effects.push(effect.clone())
            }
            "EditXMLLine" | "EditXMLReplace" | "SetPluginByte" | "SetPluginShort"
            | "SetPluginInt" | "SetPluginLong" | "SetPluginFloat" | "PatchDataFile"
            | "PatchPlugin" | "EditSDP" | "EditShader" => {}
            _ => unsupported.push(format!(
                "Line {}: {} {:?} remains unapplied",
                effect.line, effect.command, effect.arguments
            )),
        }
    }
    plan.warnings.retain(|warning| {
        !warning.contains("is an unapplied effect proposal; the caller must apply it")
    });
    Ok(ScriptedEvaluation::Review(Box::new(ScriptedReview {
        receipt: receipt.clone(),
        plan,
        generated_files: known
            .as_ref()
            .map(|p| {
                p.generated_destinations()
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect()
            })
            .unwrap_or_default(),
        profile_effects,
        unsupported,
        known,
    })))
}

/// Both choices must be explicit in a UI or recorded unattended request. File
/// effects are already shown in ScriptedReview; these cover profile mutations
/// and the separate acceptance of requested effects that remain unsupported.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScriptedApproval {
    pub apply_profile_effects: bool,
    pub allow_incomplete: bool,
}
#[derive(Debug)]
pub struct ScriptedInstallReport {
    pub install: InstallReport,
    pub receipt_path: PathBuf,
    pub pending_effects: bool,
    pub warnings: Vec<String>,
}

/// Owns the instance lock; callers should invoke it directly from their worker.
/// Merge policies fail before staging, callbacks, metadata, or profile writes.
pub fn install_scripted_omod(
    session: &OmodSession,
    context: &ScriptedContext,
    review: &ScriptedReview,
    name: &str,
    policy: OverwritePolicy,
    approval: ScriptedApproval,
    cancel: &AtomicBool,
) -> Result<ScriptedInstallReport, InstallError> {
    install_scripted_omod_with_finish(
        session,
        context,
        review,
        name,
        policy,
        approval,
        cancel,
        |_| Ok(()),
    )
}
#[allow(clippy::too_many_arguments)]
pub fn install_scripted_omod_with_finish(
    session: &OmodSession,
    context: &ScriptedContext,
    review: &ScriptedReview,
    name: &str,
    policy: OverwritePolicy,
    approval: ScriptedApproval,
    cancel: &AtomicBool,
    finish: impl FnOnce(&Path) -> Result<(), InstallError>,
) -> Result<ScriptedInstallReport, InstallError> {
    if matches!(
        policy,
        OverwritePolicy::Merge | OverwritePolicy::MergeWithBackup
    ) {
        return Err(bad(
            "Scripted OMOD transforms require a staged replacement; Merge is not supported",
        ));
    }
    check_cancel(cancel)?;
    let instance = eidos_instance::Instance::portable(context.origin.instance_root.clone());
    let _lock = instance.try_lock("publishing scripted OMOD")?;
    let fresh = ScriptedContext::capture(&instance, &context.origin.game, cancel)?;
    if fresh.origin != context.origin {
        return Err(bad("OMOD context changed; restart installer review"));
    }
    let ScriptedEvaluation::Review(verified) =
        evaluate_scripted_omod(session, &fresh, &review.receipt, cancel)?
    else {
        return Err(bad("OMOD installer still requires an answer"));
    };
    if verified.plan != review.plan
        || verified.generated_files != review.generated_files
        || verified.profile_effects != review.profile_effects
        || verified.unsupported != review.unsupported
        || verified.known != review.known
    {
        return Err(bad("OMOD reviewed plan changed"));
    }
    let mut warnings = review.plan.warnings.clone();
    warnings.extend(review.unsupported.clone());
    if !approval.apply_profile_effects {
        warnings.extend(review.profile_effects.iter().map(|e| {
            format!(
                "Line {}: {} {:?} was not approved and remains unapplied",
                e.line, e.command, e.arguments
            )
        }));
    }
    if !approval.allow_incomplete
        && (!review.unsupported.is_empty()
            || !approval.apply_profile_effects && !review.profile_effects.is_empty())
    {
        return Err(bad(
            "OMOD requests unapplied effects; explicit incomplete-installation review is required",
        ));
    }
    let mods_dir = instance.mods_dir();
    let tmp = mods_dir.join(format!(
        ".eidos-install-omod-scripted-{}-{}",
        std::process::id(),
        super::COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&tmp)?;
    let prepared = super::ExtractedTree::owned(tmp);
    stage::materialize(session, &fresh, review, prepared.path(), cancel)?;
    if let Some(readme) = &session.readme {
        fs::write(prepared.path().join("omod-readme.txt"), readme)?;
    }
    finish(prepared.path())?;
    stage::validate_tree(prepared.path(), cancel)?;
    effects::prepare(
        &fresh,
        review,
        prepared.path(),
        approval,
        warnings.clone(),
        cancel,
    )?;
    check_cancel(cancel)?;
    // Revalidate after potentially slow generation and caller transforms too.
    if ScriptedContext::capture(&instance, &context.origin.game, cancel)?.origin != context.origin {
        return Err(bad("OMOD context changed while staging; restart review"));
    }
    let final_receipt = new_omod_receipt(session, &fresh, cancel)?;
    if final_receipt.archive_sha256 != review.receipt.archive_sha256
        || final_receipt.script_sha256 != review.receipt.script_sha256
        || final_receipt.handler != review.receipt.handler
    {
        return Err(bad("OMOD archive or script changed while staging"));
    }
    let facts = vec![
        (
            "author".into(),
            serde_json::to_string(&session.metadata.author).map_err(json_error)?,
        ),
        ("version".into(), session.metadata.version()),
        (
            "eidosOmodScript".into(),
            serde_json::to_string(&review.receipt.script_sha256).map_err(json_error)?,
        ),
    ];
    let install = super::super::simple::install_destination_ready(
        &session.archive,
        &mods_dir,
        name,
        "oblivion",
        policy,
        &facts,
        |dest, merging| {
            super::super::simple::place_sources(&[prepared.path().to_path_buf()], dest, merging)?;
            Ok((String::new(), false, warnings.clone()))
        },
    )?;
    let receipt_path = effects::receipt_path(&install.dest);
    let pending_effects = match effects::retry(&instance, &install.name, cancel) {
        Ok(done) => !done,
        Err(error) => {
            warnings.push(format!(
                "Mod payload published; approved profile effects remain pending: {error}"
            ));
            true
        }
    };
    Ok(ScriptedInstallReport {
        install,
        receipt_path,
        pending_effects,
        warnings,
    })
}

/// Retry only the stored approved effects for this mod and its original profile.
/// Changed profile files or a different active profile fail without overwriting
/// them. The pending record remains until all writes and final receipt are durable.
pub fn retry_omod_effects(
    instance: &eidos_instance::Instance,
    mod_name: &str,
    cancel: &AtomicBool,
) -> Result<bool, InstallError> {
    effects::retry(instance, mod_name, cancel)
}

/// Read the bounded replay receipt stored by a completed or pending install.
/// Absence is not an error; malformed, linked or oversized records are errors.
pub fn read_installed_receipt(mod_root: &Path) -> Result<Option<OmodReplayReceipt>, InstallError> {
    effects::read_receipt(mod_root)
}

#[cfg(test)]
#[path = "scripted_tests.rs"]
mod tests;
