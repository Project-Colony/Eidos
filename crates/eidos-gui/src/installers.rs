//! GUI for the shared scripted installer engines. Blocking work owns its session.
use crate::*;
use eidos_install::{custom, obmm::*, scripted, OmodSession, OverwritePolicy};
use iced::widget::{column, row};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone)]
pub(crate) enum Action {
    Ready(u64),
    Toggle(usize),
    Text(String),
    Answer(Option<bool>),
    Back,
    Name(String),
    Replace(bool),
    Backup(bool),
    Profile(bool),
    Incomplete(bool),
    Install,
    Cancel,
    Manual,
    Preview(String),
    Restart,
}

enum Session {
    Omod {
        session: Box<OmodSession>,
        context: Option<Box<scripted::ScriptedContext>>,
        receipt: Option<scripted::OmodReplayReceipt>,
        evaluation: Option<scripted::ScriptedEvaluation>,
        receipt_loaded: bool,
    },
    Custom {
        session: Box<custom::CustomSession>,
        receipt: custom::CustomReceipt,
    },
}
enum Outcome {
    Reviewed,
    Installed(eidos_install::InstallReport, Vec<String>),
    ChoicesSaved,
}
struct Reply {
    session: Session,
    outcome: Result<Outcome, String>,
}

pub(crate) struct Wizard {
    id: u64,
    target: CollectionTarget,
    game: eidos_games::DetectedGame,
    archive: PathBuf,
    session: Option<Session>,
    pending: Option<Arc<Mutex<Option<Reply>>>>,
    cancel: Arc<AtomicBool>,
    name: String,
    selected: Vec<bool>,
    input: String,
    replace: bool,
    backup: bool,
    approval: scripted::ScriptedApproval,
    error: Option<String>,
    review: String,
    collection: Option<(PathBuf, String)>,
}
impl Drop for Wizard {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn invalid(message: impl Into<String>) -> eidos_install::InstallError {
    eidos_install::InstallError::BadSelection(message.into())
}
pub(crate) fn control_dirs(game: &eidos_games::DetectedGame) -> Vec<PathBuf> {
    game.plugin_state_dir()
        .into_iter()
        .chain(
            game.prefix()
                .zip(game.plugin_spec())
                .map(|(prefix, spec)| eidos_plugins::documents_my_games_dir(&prefix, &spec)),
        )
        .collect()
}
fn game_context(game: &eidos_games::DetectedGame) -> scripted::ScriptedGame {
    let mut observed_versions = std::collections::BTreeMap::new();
    if game.def.id == "oblivion" {
        if let Ok(info) =
            eidos_gamefeatures::preflight::inspect_pe(&game.install_path.join("Oblivion.exe"))
        {
            if let Some(version) = info.file_version {
                observed_versions.insert("Oblivion".into(), version.to_string());
            }
        }
    }
    scripted::ScriptedGame {
        game_id: game.def.id.into(),
        install_path: game.install_path.clone(),
        data_path: game.data_path.clone(),
        prefix: game.prefix(),
        observed_versions,
    }
}

pub(crate) fn begin(
    app: &mut App,
    opened: eidos_install::Opened,
    archive: PathBuf,
    name: String,
    fresh: bool,
) -> Task<Message> {
    let (Some(target), Some(game)) = (
        crate::update::collection_target(app),
        selected_game(app).cloned(),
    ) else {
        return Task::none();
    };
    let session = match opened {
        eidos_install::Opened::Omod(session) => Session::Omod {
            session,
            context: None,
            receipt: None,
            evaluation: None,
            receipt_loaded: fresh,
        },
        eidos_install::Opened::Custom(session) => {
            let receipt = session.receipt();
            Session::Custom { session, receipt }
        }
        _ => return Task::none(),
    };
    static IDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    app.installer = Some(Wizard {
        id: IDS.fetch_add(1, Ordering::Relaxed),
        target,
        game,
        archive,
        session: Some(session),
        pending: None,
        cancel: Arc::new(AtomicBool::new(false)),
        name: name.clone(),
        selected: vec![],
        input: String::new(),
        replace: false,
        backup: app.prefs.retain_install_backup,
        approval: Default::default(),
        error: None,
        review: String::new(),
        collection: app.created.as_ref().and_then(|inst| {
            let folder = inst.mods_dir().join(&name);
            inst.mod_meta(&name)
                .collection_owner()
                .filter(|_| eidos_collections::installer_answers::path(&folder).is_file())
                .map(|owner| (folder, owner.to_string()))
        }),
    });
    work(app, false)
}

fn evaluate(
    session: &mut Session,
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    addons: &[eidos_addons::Addon],
    context: &eidos_addons::Context,
    cancel: &AtomicBool,
) -> Result<(), eidos_install::InstallError> {
    match session {
        Session::Omod {
            session,
            context: ctx,
            receipt,
            evaluation,
            ..
        } => {
            if ctx.is_none() {
                *ctx = Some(Box::new(scripted::ScriptedContext::capture(
                    inst,
                    &game_context(game),
                    cancel,
                )?));
            }
            let ctx = ctx.as_ref().unwrap();
            if receipt.is_none() {
                *receipt = Some(scripted::new_omod_receipt(session, ctx, cancel)?);
            }
            *evaluation = Some(scripted::evaluate_scripted_omod(
                session,
                ctx,
                receipt.as_ref().unwrap(),
                cancel,
            )?);
        }
        Session::Custom { session, receipt } => {
            custom::evaluate_custom(session, receipt, addons, context, cancel)?;
        }
    }
    Ok(())
}
fn work(app: &mut App, install: bool) -> Task<Message> {
    let current = crate::update::collection_target(app);
    let Some(w) = app.installer.as_mut() else {
        return Task::none();
    };
    if current.as_ref() != Some(&w.target) {
        w.error =
            Some("The installation or profile changed; close and reopen this installer.".into());
        return Task::none();
    }
    let Some(mut session) = w.session.take() else {
        return Task::none();
    };
    let (id, target, game, name, cancel, addons) = (
        w.id,
        w.target.clone(),
        w.game.clone(),
        w.name.clone(),
        w.cancel.clone(),
        app.addons.clone(),
    );
    let policy = match (w.replace, w.backup) {
        (false, _) => OverwritePolicy::Fail,
        (true, false) => OverwritePolicy::Replace,
        (true, true) => OverwritePolicy::ReplaceWithBackup,
    };
    let approval = w.approval;
    let collection = w.collection.clone();
    let result = Arc::new(Mutex::new(None));
    w.pending = Some(result.clone());
    w.error = None;
    let candidates = app.games.clone();
    Task::perform(
        crate::background_work::run(move || {
            let inst = Instance::portable(target.instance.clone());
            let outcome = (|| {
            if cancel.load(Ordering::Relaxed) { return Err(invalid("Installer cancelled")); }
            let _guard = inst.try_lock("running scripted installer")?;
            if crate::update::collection_target_for(&inst, &game, &candidates).as_ref() != Some(&target) { return Err(invalid("The installation or profile changed")); }
            let context = if matches!(session, Session::Custom { .. }) {
                custom::context_for_instance(&inst, game.def.id, &game.install_path, &game.data_path, game.prefix().as_deref(), &control_dirs(&game), &cancel)?
            } else { eidos_addons::Context::default() };
            if let Session::Omod { receipt_loaded, receipt, .. } = &mut session {
                if !*receipt_loaded {
                    *receipt_loaded = true;
                    *receipt = if let Some((folder, owner)) = &collection {
                        match eidos_collections::installer_answers::read(folder, owner).map_err(invalid)? {
                            Some(eidos_collections::installer_answers::InstallerAnswers::Omod { receipt, .. }) => Some(receipt),
                            _ => return Err(invalid("Collection installer receipt changed or is missing")),
                        }
                    } else { scripted::read_installed_receipt(&inst.mods_dir().join(&name))? };
                }
            }
            evaluate(&mut session, &inst, &game, &addons, &context, &cancel)?;
            if !install { return Ok(Outcome::Reviewed); }
            if let Some((folder, owner)) = &collection {
                use eidos_collections::installer_answers::{self, InstallerAnswers};
                let answers = match &session {
                    Session::Omod { receipt: Some(receipt), evaluation: Some(scripted::ScriptedEvaluation::Review(_)), .. } => InstallerAnswers::Omod {
                        receipt: receipt.clone(), prompt: None, apply_profile_effects: approval.apply_profile_effects, allow_incomplete: approval.allow_incomplete,
                    },
                    Session::Custom { session, .. } if matches!(session.state, custom::CustomEvaluation::Ready(_)) => InstallerAnswers::Custom { receipt: session.receipt(), prompt: None },
                    _ => return Err(invalid("The collection installer still requires an answer")),
                };
                installer_answers::write(folder, owner, &answers).map_err(invalid)?;
                return Ok(Outcome::ChoicesSaved);
            }
            let mut warnings = Vec::new();
            let report = match &session {
                Session::Omod { session, context: Some(ctx), evaluation: Some(scripted::ScriptedEvaluation::Review(review)), .. } => {
                    let result = scripted::install_scripted_omod(session, ctx, review, &name, policy, approval, &cancel)?;
                    warnings.extend(result.warnings);
                    if result.pending_effects { warnings.push(format!("Approved profile effects remain pending. Retry with eidos install \"{}\" --retry-omod-effects \"{}\".", target.instance.display(), result.install.name)); }
                    result.install
                }
                Session::Custom { session, .. } => {
                    if let custom::CustomEvaluation::Ready(plan) = &session.state { warnings.extend(plan.warnings.clone()); }
                    custom::finish_custom(session, &inst.mods_dir(), &name, policy, &addons, &context, &cancel)?
                },
                _ => return Err(invalid("Installer still requires an answer")),
            };
            if let Err(error) = inst.register_installed_mod(&report.name) {
                warnings.push(format!("Payload installed; profile registration failed: {error}"));
            }
            Ok(Outcome::Installed(report, warnings))
        })().map_err(|e: eidos_install::InstallError| e.to_string());
            if let Ok(mut slot) = result.lock() {
                *slot = Some(Reply { session, outcome });
            }
            id
        }),
        |id| Message::Installer(Action::Ready(id)),
    )
}

fn prompt(w: &Wizard) -> Option<ObmmPromptKind> {
    match w.session.as_ref()? {
        Session::Omod {
            evaluation: Some(scripted::ScriptedEvaluation::NeedPrompt(p)),
            ..
        } => Some(p.kind.clone()),
        Session::Custom { session, .. } => match &session.state {
            custom::CustomEvaluation::Prompt(p) => Some(ObmmPromptKind::Select {
                title: p.title.clone(),
                many: p.multiple,
                min_choices: usize::from(!p.multiple),
                options: p
                    .options
                    .iter()
                    .map(|label| ObmmOption {
                        label: label.clone(),
                        description: None,
                        description_file: None,
                        preview: None,
                        default: false,
                    })
                    .collect(),
            }),
            _ => None,
        },
        _ => None,
    }
}
fn set_page(w: &mut Wizard) {
    w.selected.clear();
    w.input.clear();
    w.review.clear();
    match prompt(w) {
        Some(ObmmPromptKind::Select { options, .. }) => {
            w.selected = options.iter().map(|o| o.default).collect()
        }
        Some(ObmmPromptKind::Input { initial, .. }) => w.input = initial,
        _ => {}
    }
    // Approval-critical effects stay complete, regardless of the selected file count.
    let mut lines = Vec::new();
    match w.session.as_ref() {
        Some(Session::Omod {
            evaluation: Some(scripted::ScriptedEvaluation::Review(r)),
            ..
        }) => {
            lines.push(format!(
                "{} files; {} file effects; {} profile effects",
                r.plan.files.len() + r.generated_files.len(),
                r.plan.effects.len(),
                r.profile_effects.len()
            ));
            lines.extend(
                r.generated_files
                    .iter()
                    .map(|path| format!("Generated file: {path}")),
            );
            lines.push("Profile changes:".into());
            lines.extend(
                r.profile_effects
                    .iter()
                    .map(|e| format!("{} {:?}", e.command, e.arguments)),
            );
            lines.push("File transformations:".into());
            lines.extend(
                r.plan
                    .effects
                    .iter()
                    .map(|e| format!("{} {:?}", e.command, e.arguments)),
            );
            lines.extend(r.plan.warnings.iter().chain(&r.unsupported).cloned());
            lines.push("Selected files:".into());
            lines.extend(
                r.plan
                    .files
                    .iter()
                    .take(500)
                    .map(|f| format!("{} → {}", f.source, f.destination)),
            );
            if r.plan.files.len() > 500 {
                lines.push(format!(
                    "… {} more files; the full file plan is retained in the receipt.",
                    r.plan.files.len() - 500
                ));
            }
        }
        Some(Session::Custom { session, .. }) => match &session.state {
            custom::CustomEvaluation::Ready(plan) => {
                lines.push(format!("{} files", plan.files.len()));
                lines.extend(plan.warnings.clone());
                lines.extend(
                    plan.files
                        .iter()
                        .take(500)
                        .map(|f| format!("{} → {}", f.source, f.destination)),
                );
                if plan.files.len() > 500 {
                    lines.push(format!(
                        "… {} more files; the full file plan is retained in the receipt.",
                        plan.files.len() - 500
                    ));
                }
            }
            custom::CustomEvaluation::Manual(reason) => lines.push(format!(
                "{reason}\nThis installer requests manual selection."
            )),
            custom::CustomEvaluation::Cancelled => {
                lines.push("The installer cancelled this operation.".into())
            }
            _ => {}
        },
        _ => {}
    }
    w.review = lines.join("\n");
}

fn answer(w: &mut Wizard, yes: Option<bool>) -> Result<(), String> {
    let p = prompt(w).ok_or("No pending prompt")?;
    let answer = match p {
        ObmmPromptKind::Select {
            many, min_choices, ..
        } => {
            let selected: Vec<_> = w
                .selected
                .iter()
                .enumerate()
                .filter_map(|(i, yes)| yes.then_some(i))
                .collect();
            if selected.len() < min_choices || !many && selected.len() > 1 {
                return Err("Choose the required number of options.".into());
            }
            ObmmAnswer::Select(selected)
        }
        ObmmPromptKind::YesNo { .. } => ObmmAnswer::YesNo(yes.ok_or("Choose Yes or No")?),
        ObmmPromptKind::Input { max_length, .. } => {
            if w.input.chars().count() > max_length.unwrap_or(65_536) {
                return Err("The entered text is too long.".into());
            }
            ObmmAnswer::Text(w.input.clone())
        }
        _ => ObmmAnswer::Acknowledge,
    };
    match w.session.as_mut().unwrap() {
        Session::Omod {
            receipt: Some(receipt),
            evaluation: Some(scripted::ScriptedEvaluation::NeedPrompt(prompt)),
            ..
        } => receipt.answers.push(ObmmRecordedAnswer {
            prompt: prompt.clone(),
            answer,
        }),
        Session::Custom { session, receipt } => {
            let custom::CustomEvaluation::Prompt(prompt) = &session.state else {
                return Err("Prompt changed".into());
            };
            let ObmmAnswer::Select(selected) = answer else {
                return Err("Expected a selection".into());
            };
            receipt.answers.push(custom::CustomRecordedAnswer {
                prompt: prompt.clone(),
                selected,
            });
        }
        _ => return Err("Prompt changed".into()),
    }
    Ok(())
}

pub(crate) fn update(app: &mut App, action: Action) -> Task<Message> {
    if matches!(action, Action::Cancel) {
        app.installer = None;
        app.preview_pending = None;
        app.preview = None;
        app.status = Some(
            "Installer closed. Any published payload and pending effects remain recorded.".into(),
        );
        return Task::none();
    }
    let current = crate::update::collection_target(app);
    let Some(w) = app.installer.as_mut() else {
        return Task::none();
    };
    if let Action::Ready(id) = action {
        if id != w.id {
            return Task::none();
        }
        let reply = w
            .pending
            .take()
            .and_then(|slot| slot.lock().ok().and_then(|mut s| s.take()));
        let Some(reply) = reply else {
            w.error = Some("Installer worker stopped without a result.".into());
            return Task::none();
        };
        w.session = Some(reply.session);
        match reply.outcome {
            Ok(Outcome::Installed(report, warnings)) => {
                let archive = w.archive.clone();
                if current.as_ref() == Some(&w.target) {
                    app.installer = None;
                    after_install_report(app, report, &archive);
                    if !warnings.is_empty() {
                        app.status
                            .get_or_insert_with(String::new)
                            .push_str(&format!("\n{}", warnings.join("\n")));
                    }
                } else {
                    w.error = Some(format!("Installed {} in the original instance. The current view changed and was not refreshed.", report.name));
                }
            }
            Ok(Outcome::Reviewed) => set_page(w),
            Ok(Outcome::ChoicesSaved) => {
                app.installer = None;
                app.status = Some("Installer choices saved. Resume the collection to validate its recipe and publish the mod.".into());
            }
            Err(error) => w.error = Some(error),
        }
        return Task::none();
    }
    if w.pending.is_some() {
        return Task::none();
    }
    if current.as_ref() != Some(&w.target) {
        w.error = Some("The installation or profile changed; reopen the installer.".into());
        return Task::none();
    }
    match action {
        Action::Toggle(index) => {
            if index < w.selected.len() {
                if matches!(prompt(w), Some(ObmmPromptKind::Select { many: false, .. })) {
                    w.selected.fill(false);
                }
                w.selected[index] = !w.selected[index];
            }
        }
        Action::Text(value) => {
            w.input = value.chars().take(65_536).collect();
        }
        Action::Name(value) => w.name = value,
        Action::Replace(value) => w.replace = value,
        Action::Backup(value) => w.backup = value,
        Action::Profile(value) => w.approval.apply_profile_effects = value,
        Action::Incomplete(value) => w.approval.allow_incomplete = value,
        Action::Answer(yes) => {
            w.approval = Default::default();
            match answer(w, yes) {
                Ok(()) => return work(app, false),
                Err(error) => w.error = Some(error),
            }
        }
        Action::Back => {
            w.approval = Default::default();
            match w.session.as_mut() {
                Some(Session::Omod {
                    receipt: Some(receipt),
                    ..
                }) => {
                    receipt.answers.pop();
                }
                Some(Session::Custom { receipt, .. }) => {
                    receipt.answers.pop();
                    receipt.plan = None;
                }
                _ => {}
            }
            return work(app, false);
        }
        Action::Install => return work(app, true),
        Action::Restart => {
            if matches!(w.session, Some(Session::Custom { .. })) {
                let (archive, name) = (w.archive.clone(), w.name.clone());
                app.installer = None;
                return crate::update::open_mod_archive(app, archive, name, true);
            }
            match w.session.as_mut() {
                Some(Session::Omod {
                    context,
                    receipt,
                    evaluation,
                    ..
                }) => {
                    *context = None;
                    *receipt = None;
                    *evaluation = None;
                }
                Some(Session::Custom { receipt, .. }) => {
                    receipt.answers.clear();
                    receipt.plan = None;
                }
                _ => {}
            }
            w.approval = Default::default();
            return work(app, false);
        }
        Action::Manual => {
            if w.collection.is_some() {
                w.error = Some(
                    "Resume the collection so its recipe is applied before publication.".into(),
                );
                return Task::none();
            }
            let target = w.target.clone();
            let _guard = match crate::update::lock_install_target(app, &target) {
                Ok(guard) => guard,
                Err(error) => {
                    app.installer.as_mut().unwrap().error = Some(error);
                    return Task::none();
                }
            };
            let w = app.installer.as_mut().unwrap();
            let Some(Session::Custom { session, .. }) = w.session.take() else {
                return Task::none();
            };
            let (archive, name, target) = (w.archive.clone(), w.name.clone(), w.target.clone());
            let opened = eidos_install::Opened::Manual(session.into_tree());
            app.installer = None;
            return crate::update::finish_open(app, Ok(opened), archive, name, target, false);
        }
        Action::Preview(member) => {
            if let Some(Session::Omod { session, .. }) = w.session.as_ref() {
                if let Some(entry) = session.members.iter().find(|m| {
                    m.path.eq_ignore_ascii_case(&member)
                        && m.kind == eidos_install::OmodFileKind::Data
                }) {
                    let path = session.data_root().join(&entry.path);
                    return crate::file_preview::start(app, path, None, None, None);
                }
                w.error = Some("Preview member is absent from the decoded OMOD payload.".into());
            }
        }
        Action::Cancel | Action::Ready(_) => {}
    }
    Task::none()
}
fn msg(a: Action) -> Message {
    Message::Installer(a)
}

pub(crate) fn view(w: &Wizard) -> Element<'_, Message> {
    let mut body = column![
        text("Mod installer").size(24),
        text(
            w.archive
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        )
    ]
    .spacing(10);
    if w.pending.is_some() {
        body = body.push(text("Preparing and validating the installation…"));
    } else if let Some(p) = prompt(w) {
        match p {
            ObmmPromptKind::Select {
                title,
                many,
                min_choices,
                options,
            } => {
                body = body.push(text(title)).push(text(format!(
                    "Choose {}{} option(s)",
                    if many { "at least " } else { "" },
                    min_choices
                )));
                for (i, option) in options.into_iter().enumerate() {
                    let selected = w.selected.get(i).copied().unwrap_or(false);
                    body = body.push(
                        checkbox(selected)
                            .label(option.label)
                            .on_toggle(move |_| msg(Action::Toggle(i))),
                    );
                    if let Some(description) = option.description {
                        body = body.push(text(description));
                    }
                    for (label, path) in [
                        ("Read description", option.description_file),
                        ("Preview", option.preview),
                    ] {
                        if let Some(path) = path {
                            body =
                                body.push(button(text(label)).on_press(msg(Action::Preview(path))));
                        }
                    }
                }
                body = body.push(button("Continue").on_press(msg(Action::Answer(None))));
            }
            ObmmPromptKind::YesNo { title, message } => {
                body = body.push(text(title)).push(text(message)).push(
                    row![
                        button("Yes").on_press(msg(Action::Answer(Some(true)))),
                        button("No").on_press(msg(Action::Answer(Some(false))))
                    ]
                    .spacing(10),
                )
            }
            ObmmPromptKind::Input { title, .. } => {
                body = body
                    .push(text(title))
                    .push(text_input("Answer", &w.input).on_input(|s| msg(Action::Text(s))))
                    .push(button("Continue").on_press(msg(Action::Answer(None))))
            }
            ObmmPromptKind::Message { title, message } => {
                body = body
                    .push(text(title))
                    .push(text(message))
                    .push(button("Continue").on_press(msg(Action::Answer(None))))
            }
            ObmmPromptKind::Preview { title, path, .. } => {
                body = body
                    .push(text(title))
                    .push(button("Open preview").on_press(msg(Action::Preview(path))))
                    .push(button("Continue").on_press(msg(Action::Answer(None))))
            }
        }
        body = body.push(button("Back").on_press(msg(Action::Back)));
    } else {
        body = body
            .push(button("Back").on_press(msg(Action::Back)))
            .push(text(&w.review));
        let ready = matches!(
            w.session.as_ref(),
            Some(Session::Omod {
                evaluation: Some(scripted::ScriptedEvaluation::Review(_)),
                ..
            })
        ) || matches!(w.session.as_ref(), Some(Session::Custom { session, .. }) if matches!(session.state, custom::CustomEvaluation::Ready(_)));
        if ready {
            if w.collection.is_none() {
                body = body
                    .push(text_input("Mod folder name", &w.name).on_input(|v| msg(Action::Name(v))))
                    .push(
                        checkbox(w.replace)
                            .label("Replace this mod if the folder already exists")
                            .on_toggle(|v| msg(Action::Replace(v))),
                    )
                    .push(
                        checkbox(w.backup)
                            .label("Retain a backup when replacing")
                            .on_toggle(|v| msg(Action::Backup(v))),
                    );
            }
            if matches!(w.session, Some(Session::Omod { .. })) {
                body = body
                    .push(
                        checkbox(w.approval.apply_profile_effects)
                            .label("Apply the listed profile INI and plugin changes")
                            .on_toggle(|v| msg(Action::Profile(v))),
                    )
                    .push(
                        checkbox(w.approval.allow_incomplete)
                            .label("Accept installation with the listed unapplied effects")
                            .on_toggle(|v| msg(Action::Incomplete(v))),
                    );
            }
            body = body.push(
                button(if w.collection.is_some() {
                    "Save choices for the collection"
                } else {
                    "Install reviewed files"
                })
                .on_press(msg(Action::Install)),
            );
        }
        if w.collection.is_none() && matches!(w.session, Some(Session::Custom { .. })) {
            body = body.push(button("Choose files manually").on_press(msg(Action::Manual)));
        }
        body = body.push(button("Start choices again").on_press(msg(Action::Restart)));
    }
    if let Some(error) = &w.error {
        body = body.push(text(error));
    }
    body = body.push(button("Close installer").on_press(msg(Action::Cancel)));
    container(scrollable(body).height(Length::Fill))
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use smol::stream::StreamExt;

    pub(crate) fn drive(app: &mut App, task: Task<Message>) {
        let mut tasks = vec![task];
        while let Some(task) = tasks.pop() {
            if let Some(mut stream) = iced_runtime::task::into_stream(task) {
                while let Some(action) = smol::block_on(smol::future::or(stream.next(), async {
                    smol::Timer::after(std::time::Duration::from_secs(30)).await;
                    panic!("installer task exceeded 30 seconds")
                })) {
                    if let iced_runtime::Action::Output(message) = action {
                        tasks.push(crate::update::update_inner(app, message));
                    }
                }
            }
        }
    }
    pub(crate) fn fixture() -> (tempfile::TempDir, App, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let inst = Instance::portable(dir.path().join("instance"));
        inst.create().unwrap();
        inst.ensure_manifest("oblivion", InstanceKind::Portable)
            .unwrap();
        let game = DetectedGame {
            def: eidos_games::GameDef::for_id("oblivion").unwrap(),
            install_path: dir.path().join("game"),
            data_path: dir.path().join("game/Data"),
            compatdata: None,
            source: Default::default(),
            steam_name: "Synthetic Oblivion".into(),
        };
        fs::create_dir_all(&game.data_path).unwrap();
        fs::write(
            inst.active().ini_path("Oblivion.ini"),
            "[General]\nName=Original\n",
        )
        .unwrap();
        let archive = dir.path().join("fixture.omod");
        let status = std::process::Command::new("python3").arg("-c").arg(r#"
import io,struct,sys,zipfile,zlib
def string(s):
    b=s.encode(); n=len(b); out=bytearray()
    while n>=128: out.append((n&127)|128); n>>=7
    return bytes(out)+bytes([n])+b
def archive(entries):
    out=io.BytesIO()
    with zipfile.ZipFile(out,'w') as z:
        for name,data in entries: z.writestr(name,data)
    return out.getvalue()
config=bytes([4])+string('GUI fixture')+struct.pack('<ii',1,0)
config+=b''.join(map(string,['Fixture','','','Synthetic GUI test']))+struct.pack('<qBi',638000000000000000,1,0)
script='DontInstallAnyDataFiles\nDontInstallAnyPlugins\nSelectWithDescriptionsAndPreviews "Variant" One preview.txt "First variant" Two preview.txt "Second variant"\nCase One\nCopyDataFile a.txt selected.txt\nBreak\nCase Two\nCopyDataFile b.txt selected.txt\nBreak\nEndSelect'
files=[('a.txt',b'first'),('b.txt',b'second'),('preview.txt',b'Preview fixture')]
crc=b''.join(string(n)+struct.pack('<IQ',zlib.crc32(b),len(b)) for n,b in files)
open(sys.argv[1],'wb').write(archive([('config',config),('script',string(script)),('data.crc',crc),('data',archive([('a',b''.join(b for n,b in files))]))]))
"#).arg(&archive).status().unwrap();
        assert!(status.success());
        let mut app = crate::state::new(vec![]).0;
        assert!(app.created.is_none());
        app.created = Some(inst);
        app.games = vec![game];
        app.selected = Some(0);
        app.screen = Screen::Main;
        (dir, app, archive)
    }

    fn changed_profile_install(route: &str) {
        let (dir, mut app, _) = fixture();
        let inst = app.created.as_ref().unwrap().clone();
        let archive = dir.path().join("Ordinary.zip");
        let status = std::process::Command::new("python3").arg("-c").arg(r#"
import sys,zipfile
with zipfile.ZipFile(sys.argv[1],'w') as z:
    z.writestr('textures/fixture.txt',b'new payload')
    if sys.argv[2]=='fomod':
        z.writestr('fomod/ModuleConfig.xml','<config><moduleName>Ordinary</moduleName><requiredInstallFiles><file source="textures/fixture.txt" destination="textures/fixture.txt" /></requiredInstallFiles></config>')
"#).arg(&archive).arg(route).status().unwrap();
        assert!(status.success());
        let installed = inst.mods_dir().join("Ordinary");
        if route == "collision" {
            fs::create_dir_all(installed.join("textures")).unwrap();
            fs::write(installed.join("textures/fixture.txt"), b"existing payload").unwrap();
        }
        if route == "completion" {
            let task = crate::update::open_mod_archive(
                &mut app,
                archive.clone(),
                "Ordinary".into(),
                false,
            );
            drive(&mut app, task);
        } else {
            let mut opened =
                eidos_install::open_archive(&archive, &inst.mods_dir(), "Ordinary", "oblivion")
                    .unwrap();
            if route == "manual" {
                let eidos_install::Opened::Simple(tree) = opened else {
                    panic!("simple fixture")
                };
                opened = eidos_install::Opened::Manual(tree);
            }
            let target = crate::update::collection_target(&app).unwrap();
            let task = crate::update::finish_open(
                &mut app,
                Ok(opened),
                archive.clone(),
                "Ordinary".into(),
                target,
                false,
            );
            drive(&mut app, task);
        }
        let original = inst.active();
        inst.profile("Other").create_from(&original).unwrap();
        let before = fs::read(inst.profile("Other").dir().join("modlist.txt")).ok();
        {
            let _guard = inst.try_lock("another window switching profiles").unwrap();
            inst.set_active_profile("Other").unwrap();
        }
        match route {
            "completion" => poll_open(&mut app),
            "fomod" => {
                let task = crate::update::update_inner(&mut app, Message::FomodInstall);
                drive(&mut app, task);
                assert!(
                    app.fomod.is_some(),
                    "retain FOMOD choices on target refusal"
                );
            }
            "manual" => {
                crate::modinfo::run_picker_install(&mut app);
                assert!(
                    app.picker.is_some(),
                    "retain manual choices on target refusal"
                );
            }
            "collision" => {
                crate::modinfo::run_collision_install(&mut app, OverwritePolicy::Replace);
                assert!(
                    app.collision.is_some(),
                    "retain collision choices on target refusal"
                );
            }
            _ => panic!("unknown route"),
        }
        if route == "collision" {
            assert_eq!(
                fs::read(installed.join("textures/fixture.txt")).unwrap(),
                b"existing payload"
            );
        } else {
            assert!(
                !installed.exists(),
                "{route} published into the changed profile"
            );
        }
        assert_eq!(
            fs::read(inst.profile("Other").dir().join("modlist.txt")).ok(),
            before
        );
        assert!(
            app.status.as_deref().is_some_and(|s| s.contains("changed")),
            "{:?}",
            app.status
        );
        if route != "completion" {
            {
                let _guard = inst.try_lock("restoring original profile").unwrap();
                inst.set_active_profile(&original.name).unwrap();
            }
            match route {
                "fomod" => {
                    let task = crate::update::update_inner(&mut app, Message::FomodInstall);
                    drive(&mut app, task);
                }
                "manual" => crate::modinfo::run_picker_install(&mut app),
                "collision" => {
                    crate::modinfo::run_collision_install(&mut app, OverwritePolicy::Replace)
                }
                _ => unreachable!(),
            }
            assert_eq!(
                fs::read(installed.join("textures/fixture.txt")).unwrap(),
                b"new payload"
            );
            assert_eq!(
                fs::read(inst.profile("Other").dir().join("modlist.txt")).ok(),
                before
            );
        }
    }
    #[test]
    fn changed_profile_refuses_simple_job_completion() {
        changed_profile_install("completion");
    }
    #[test]
    fn changed_profile_refuses_fomod_publication() {
        changed_profile_install("fomod");
    }
    #[test]
    fn changed_profile_refuses_manual_publication() {
        changed_profile_install("manual");
    }
    #[test]
    fn changed_profile_refuses_collision_publication() {
        changed_profile_install("collision");
    }

    #[test]
    fn missing_original_target_refuses_opened_archive() {
        let (_dir, mut app, archive) = fixture();
        let task = crate::update::open_mod_archive(&mut app, archive, "Ordinary".into(), false);
        drive(&mut app, task);
        app.install_job.as_mut().unwrap().target = None;
        poll_open(&mut app);
        assert!(app.installer.is_none());
        assert!(!app
            .created
            .as_ref()
            .unwrap()
            .mods_dir()
            .join("Ordinary")
            .exists());
        assert!(app
            .status
            .as_deref()
            .is_some_and(|s| s.contains("no original target")));
    }

    #[test]
    fn ordinary_install_routes_refuse_an_externally_locked_instance() {
        use std::os::fd::AsRawFd;
        for route in ["simple", "manual", "fomod"] {
            let (dir, mut app, _) = fixture();
            let inst = app.created.as_ref().unwrap().clone();
            let archive = dir.path().join("Ordinary.zip");
            let status = std::process::Command::new("python3").arg("-c").arg(r#"
import sys,zipfile
with zipfile.ZipFile(sys.argv[1],'w') as z:
    z.writestr('textures/fixture.txt',b'original payload')
    if sys.argv[2]=='fomod':
        z.writestr('fomod/ModuleConfig.xml','<config><moduleName>Ordinary</moduleName><requiredInstallFiles><file source="textures/fixture.txt" destination="textures/fixture.txt" /></requiredInstallFiles></config>')
"#).arg(&archive).arg(route).status().unwrap();
            assert!(status.success());
            let opened =
                eidos_install::open_archive(&archive, &inst.mods_dir(), "Ordinary", "oblivion")
                    .unwrap();
            let held = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(inst.root.join(".eidos.lock"))
                .unwrap();
            let lock_externally = || {
                assert_eq!(
                    unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
                    0
                )
            };
            match (route, opened) {
                ("manual", eidos_install::Opened::Simple(tree)) => {
                    lock_externally();
                    let archive_tree = parsed_tree(&tree);
                    app.picker = Some(InstallPicker {
                        target: crate::update::collection_target(&app).unwrap(),
                        rows: archive_tree.flatten(),
                        archive_tree,
                        tree,
                        archive: archive.clone(),
                        name: "Ordinary".into(),
                        game_id: "oblivion".into(),
                        mode: PickerMode::Manual {
                            root: String::new(),
                        },
                    });
                    crate::modinfo::run_picker_install(&mut app);
                    assert!(app.picker.is_some(), "Keep the choices on lock refusal");
                }
                (_, opened) => {
                    if route != "fomod" {
                        lock_externally();
                    }
                    let target = crate::update::collection_target(&app).unwrap();
                    let task = crate::update::finish_open(
                        &mut app,
                        Ok(opened),
                        archive,
                        "Ordinary".into(),
                        target,
                        false,
                    );
                    drive(&mut app, task);
                    if route == "fomod" {
                        // The wizard opens first; another process then owns publication.
                        lock_externally();
                        let task = crate::update::update_inner(&mut app, Message::FomodInstall);
                        drive(&mut app, task);
                        assert!(app.fomod.is_some(), "Keep FOMOD choices on lock refusal");
                    }
                }
            }
            assert!(
                !inst.mods_dir().join("Ordinary").exists(),
                "{route} published under a running session"
            );
            assert!(
                app.status
                    .as_deref()
                    .is_some_and(|s| s.contains("Cannot install")),
                "{:?}",
                app.status
            );
        }
    }
    pub(crate) fn open(app: &mut App, archive: &Path) {
        let opened = eidos_install::open_archive_with(
            archive,
            &app.created.as_ref().unwrap().mods_dir(),
            "Choice",
            "oblivion",
            |_| {},
        )
        .unwrap();
        let task = begin(app, opened, archive.to_owned(), "Choice".into(), false);
        drive(app, task);
        assert!(
            prompt(app.installer.as_ref().unwrap()).is_some(),
            "{:?}",
            app.installer.as_ref().unwrap().error
        );
    }
    fn poll_open(app: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !app
            .install_job
            .as_ref()
            .unwrap()
            .done
            .load(Ordering::SeqCst)
        {
            assert!(
                std::time::Instant::now() < deadline,
                "archive open timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let task = crate::update::update_inner(app, Message::InstallPoll);
        drive(app, task);
    }
    pub(crate) fn act(app: &mut App, action: Action) {
        let task = update(app, action);
        drive(app, task);
    }

    #[test]
    fn omod_gui_drives_real_tasks_back_preview_and_exact_file_publication() {
        let (_dir, mut app, archive) = fixture();
        open(&mut app, &archive);
        act(&mut app, Action::Preview("preview.txt".into()));
        assert!(app.preview.is_some());
        let _ = crate::update::update_inner(&mut app, Message::ClosePreview);
        act(&mut app, Action::Toggle(1));
        act(&mut app, Action::Answer(None));
        assert!(prompt(app.installer.as_ref().unwrap()).is_none());
        act(&mut app, Action::Back);
        assert!(prompt(app.installer.as_ref().unwrap()).is_some());
        act(&mut app, Action::Toggle(0));
        act(&mut app, Action::Answer(None));
        act(&mut app, Action::Install);
        assert!(
            app.installer.is_none(),
            "{:?}",
            app.installer.as_ref().and_then(|w| w.error.as_ref())
        );
        let dest = app.created.as_ref().unwrap().mods_dir().join("Choice");
        assert_eq!(fs::read(dest.join("selected.txt")).unwrap(), b"first");
        assert!(!dest.join("b.txt").exists());
        assert_eq!(
            scripted::read_installed_receipt(&dest)
                .unwrap()
                .unwrap()
                .answers
                .len(),
            1
        );
    }
    #[test]
    fn review_refuses_context_change_and_restart_rebuilds_the_prompt() {
        let (_dir, mut app, archive) = fixture();
        open(&mut app, &archive);
        act(&mut app, Action::Toggle(0));
        act(&mut app, Action::Answer(None));
        fs::write(
            app.created
                .as_ref()
                .unwrap()
                .active()
                .ini_path("Oblivion.ini"),
            "[General]\nName=Changed\n",
        )
        .unwrap();
        act(&mut app, Action::Install);
        assert!(app
            .installer
            .as_ref()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("context"));
        assert!(!app
            .created
            .as_ref()
            .unwrap()
            .mods_dir()
            .join("Choice")
            .exists());
        act(&mut app, Action::Restart);
        assert!(prompt(app.installer.as_ref().unwrap()).is_some());
        assert!(app.installer.as_ref().unwrap().error.is_none());
    }
    #[test]
    fn closing_a_queued_installer_cancels_work_and_late_completion() {
        let (_dir, mut app, archive) = fixture();
        let opened = eidos_install::open_archive_with(
            &archive,
            &app.created.as_ref().unwrap().mods_dir(),
            "Choice",
            "oblivion",
            |_| {},
        )
        .unwrap();
        let task = begin(&mut app, opened, archive, "Choice".into(), false);
        let cancel = app.installer.as_ref().unwrap().cancel.clone();
        let _ = update(&mut app, Action::Cancel);
        assert!(cancel.load(Ordering::Relaxed));
        drive(&mut app, task);
        assert!(app.installer.is_none());
        assert!(!app
            .created
            .as_ref()
            .unwrap()
            .mods_dir()
            .join("Choice")
            .exists());
    }
    #[test]
    fn modal_blocks_background_mutations_and_stale_ready_ids() {
        let (_dir, mut app, archive) = fixture();
        open(&mut app, &archive);
        let profile = app.created.as_ref().unwrap().active_profile();
        let _ = crate::update::update_inner(&mut app, Message::SwitchProfile("Other".into()));
        assert_eq!(app.created.as_ref().unwrap().active_profile(), profile);
        let id = app.installer.as_ref().unwrap().id;
        let _ = update(&mut app, Action::Ready(id + 1));
        assert!(prompt(app.installer.as_ref().unwrap()).is_some());
    }
    #[test]
    fn collection_review_saves_choices_without_publishing_or_losing_ownership() {
        use eidos_collections::installer_answers::{self, InstallerAnswers};
        let (_dir, mut app, archive) = fixture();
        let inst = app.created.as_ref().unwrap().clone();
        let folder = inst.mods_dir().join("Choice");
        fs::create_dir(&folder).unwrap();
        let owner = r#"["fixture/1","member"]"#;
        let mut meta = eidos_instance::ModMeta::default();
        meta.set("eidosCollectionOwner", owner);
        meta.write(&folder.join("meta.ini")).unwrap();
        inst.save_modlist(&inst.modlist()).unwrap();
        let session = eidos_install::open_omod_with(
            &archive,
            &inst.mods_dir(),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
        let ctx = scripted::ScriptedContext::capture(
            &inst,
            &game_context(&app.games[0]),
            &AtomicBool::new(false),
        )
        .unwrap();
        let receipt = scripted::new_omod_receipt(&session, &ctx, &AtomicBool::new(false)).unwrap();
        installer_answers::write_for_archive(
            &folder,
            owner,
            &archive,
            &InstallerAnswers::Omod {
                receipt,
                prompt: None,
                apply_profile_effects: false,
                allow_incomplete: false,
            },
        )
        .unwrap();
        drop(session);
        open(&mut app, &archive);
        assert!(app.installer.as_ref().unwrap().collection.is_some());
        act(&mut app, Action::Toggle(1));
        act(&mut app, Action::Answer(None));
        act(&mut app, Action::Install);
        assert!(app.installer.is_none());
        assert!(!folder.join("selected.txt").exists());
        let InstallerAnswers::Omod {
            receipt, prompt, ..
        } = installer_answers::read(&folder, owner).unwrap().unwrap()
        else {
            panic!("wrong handler")
        };
        assert_eq!(receipt.answers.len(), 1);
        assert!(prompt.is_none());
        assert_eq!(
            installer_answers::archive(&folder, owner).unwrap().unwrap(),
            archive
        );
        assert_eq!(
            eidos_instance::ModMeta::read(&folder.join("meta.ini")).collection_owner(),
            Some(owner)
        );
    }

    #[test]
    fn custom_gui_replays_a_real_helper_and_publishes_only_selected_files() {
        let (dir, mut app, _) = fixture();
        let archive = dir.path().join("custom.zip");
        assert!(std::process::Command::new("python3").arg("-c").arg("import zipfile,sys; z=zipfile.ZipFile(sys.argv[1],'w'); z.writestr('first.txt','first'); z.writestr('second.txt','second'); z.close()").arg(&archive).status().unwrap().success());
        let helper = dir.path().join("installer.py");
        fs::write(&helper, r#"import json,sys
r=json.load(open(sys.argv[1]))
if not r['answers']:
    o={'status':'prompt','id':'variant','title':'Variant','options':['First','Second'],'multiple':False}
else:
    name=['first.txt','second.txt'][r['answers']['variant'][0]]
    o={'status':'handled','result':{'kind':'install','files':[{'source':name,'destination':'selected.txt'}],'warnings':[]}}
print(json.dumps({'protocol':1,'request_id':r['request_id'],'outcome':o}))
"#).unwrap();
        let manifest = dir.path().join("installer.toml");
        let text = format!("id='gui-fixture'\nkind='installer'\nprotocol=1\nexec='/usr/bin/python3'\nargs=['{}','{{request}}']\nextensions=['zip']\nversion='1'\n", helper.display());
        fs::write(&manifest, &text).unwrap();
        app.addons = vec![eidos_addons::parse_addon(&text, &manifest).unwrap()];
        let inst = app.created.as_ref().unwrap();
        let game = &app.games[0];
        let ctx = custom::context_for_instance(
            inst,
            game.def.id,
            &game.install_path,
            &game.data_path,
            None,
            &control_dirs(game),
            &AtomicBool::new(false),
        )
        .unwrap();
        let opened = eidos_install::open_archive_with_installers(
            &archive,
            &inst.mods_dir(),
            "Choice",
            game.def.id,
            &app.addons,
            &ctx,
            &AtomicBool::new(false),
            None,
            |_| {},
        )
        .unwrap();
        let task = begin(&mut app, opened, archive.clone(), "Choice".into(), false);
        drive(&mut app, task);
        assert!(prompt(app.installer.as_ref().unwrap()).is_some());
        act(&mut app, Action::Toggle(1));
        act(&mut app, Action::Answer(None));
        act(&mut app, Action::Back);
        act(&mut app, Action::Toggle(0));
        act(&mut app, Action::Answer(None));
        act(&mut app, Action::Install);
        assert!(
            app.installer.is_none(),
            "{:?}",
            app.installer.as_ref().and_then(|w| w.error.as_ref())
        );
        let folder = app.created.as_ref().unwrap().mods_dir().join("Choice");
        assert_eq!(fs::read(folder.join("selected.txt")).unwrap(), b"first");
        assert!(!folder.join("second.txt").exists());
        assert_eq!(
            custom::read_installed_receipt(&folder)
                .unwrap()
                .unwrap()
                .answers[0]
                .selected,
            [0]
        );
        fs::write(
            app.created
                .as_ref()
                .unwrap()
                .active()
                .ini_path("Oblivion.ini"),
            "[General]\nName=Changed after install\n",
        )
        .unwrap();
        let _ = crate::update::open_mod_archive(&mut app, archive.clone(), "Choice".into(), false);
        poll_open(&mut app);
        assert!(
            app.installer.is_none(),
            "saved stale identity must fail before a new wizard"
        );
        assert!(
            app.status.as_ref().unwrap().contains("stale"),
            "{:?}",
            app.status
        );
        let _ = crate::update::open_mod_archive(&mut app, archive, "Choice".into(), true);
        poll_open(&mut app);
        assert!(prompt(app.installer.as_ref().unwrap()).is_some());
        assert_eq!(
            fs::read(folder.join("selected.txt")).unwrap(),
            b"first",
            "fresh choices are reviewed before replacing files"
        );
        act(&mut app, Action::Toggle(1));
        act(&mut app, Action::Answer(None));
        act(&mut app, Action::Replace(true));
        act(&mut app, Action::Install);
        assert!(
            app.installer.is_none(),
            "{:?}",
            app.installer.as_ref().and_then(|w| w.error.as_ref())
        );
        assert_eq!(fs::read(folder.join("selected.txt")).unwrap(), b"second");
    }
    #[test]
    fn large_file_review_keeps_all_approval_effects_and_resets_approval_on_back() {
        let (_dir, mut app, archive) = fixture();
        open(&mut app, &archive);
        act(&mut app, Action::Toggle(0));
        act(&mut app, Action::Answer(None));
        let w = app.installer.as_mut().unwrap();
        let Some(Session::Omod {
            evaluation: Some(scripted::ScriptedEvaluation::Review(r)),
            ..
        }) = w.session.as_mut()
        else {
            panic!("review")
        };
        r.plan.files = vec![r.plan.files[0].clone(); 1000];
        r.generated_files.push("HRMHorseArmor.bsa".into());
        r.profile_effects.push(ObmmEffect {
            line: 10,
            command: "EditINI".into(),
            arguments: vec!["Fonts".into(), "SFontFile_1".into(), "new_font.fnt".into()],
        });
        r.unsupported = (0..600).map(|i| format!("Unapplied effect {i}")).collect();
        set_page(w);
        assert!(w.review.contains("Generated file: HRMHorseArmor.bsa"));
        assert!(w.review.contains("new_font.fnt") && w.review.contains("Unapplied effect 599"));
        w.approval = scripted::ScriptedApproval {
            apply_profile_effects: true,
            allow_incomplete: true,
        };
        act(&mut app, Action::Back);
        let approval = app.installer.as_ref().unwrap().approval;
        assert!(!approval.apply_profile_effects && !approval.allow_incomplete);
    }

    #[test]
    fn late_archive_pickers_keep_their_original_request_and_installation() {
        let (_dir, mut app, archive) = fixture();
        let target = crate::update::collection_target(&app).unwrap();
        app.mod_picker = 12;
        for (request, selected) in [
            (11, target.clone()),
            (
                12,
                CollectionTarget {
                    profile: "Other".into(),
                    ..target.clone()
                },
            ),
        ] {
            let _ = crate::update::update_inner(
                &mut app,
                Message::ModPickedFor {
                    fresh: false,
                    request,
                    target: Some(selected),
                    name: Some("Chosen name".into()),
                    path: Some(archive.clone()),
                },
            );
            assert!(app.install_job.is_none());
        }
        let _ = crate::update::update_inner(
            &mut app,
            Message::ModPickedFor {
                fresh: true,
                request: 12,
                target: Some(target),
                name: Some("Chosen name".into()),
                path: Some(archive),
            },
        );
        assert_eq!(app.install_job.as_ref().unwrap().name, "Chosen name");
        assert!(app.install_job.as_ref().unwrap().fresh);
        poll_open(&mut app);
        assert_eq!(app.installer.as_ref().unwrap().name, "Chosen name");
    }

    #[test]
    fn ambiguous_legacy_targets_refuse_ordinary_and_queued_scripted_installers() {
        for legacy_key in [true, false] {
            let (dir, mut app, archive) = fixture();
            let inst = app.created.as_ref().unwrap().clone();
            let first_prefix = dir.path().join("prefix-a");
            let second_prefix = dir.path().join("prefix-b");
            fs::create_dir_all(&first_prefix).unwrap();
            fs::create_dir_all(&second_prefix).unwrap();
            app.games[0].source = eidos_games::GameSource::External {
                store: eidos_games::Store::Gog,
                app_id: "synthetic-oblivion".into(),
                prefix: Some(first_prefix),
                heroic: true,
            };
            let mut second = app.games[0].clone();
            if let eidos_games::GameSource::External { prefix, .. } = &mut second.source {
                *prefix = Some(second_prefix);
            }
            app.games.push(second);
            let mut manifest = inst.read_manifest().unwrap();
            manifest.installation = Some(app.games[0].selection_id());
            manifest.write(&inst.manifest_path()).unwrap();
            let target = crate::update::collection_target(&app).unwrap();
            open(&mut app, &archive);
            act(&mut app, Action::Toggle(0));
            act(&mut app, Action::Answer(None));
            let task = update(&mut app, Action::Install);

            manifest.installation = legacy_key.then(|| {
                serde_json::json!([
                    "GOG:synthetic-oblivion",
                    app.games[0].install_path.to_string_lossy()
                ])
                .to_string()
            });
            manifest.write(&inst.manifest_path()).unwrap();
            let ordinary_refused = crate::update::lock_install_target(&app, &target).is_err();
            drive(&mut app, task);
            assert!(ordinary_refused, "accepted an ambiguous saved identity");
            assert!(app
                .installer
                .as_ref()
                .unwrap()
                .error
                .as_deref()
                .is_some_and(|error| error.contains("installation or profile changed")));
            assert!(!inst.mods_dir().join("Choice").exists());

            // Legacy and missing keys remain supported when exactly one copy matches.
            app.games.pop();
            assert_eq!(crate::update::collection_target(&app), Some(target.clone()));
            assert!(crate::update::lock_install_target(&app, &target).is_ok());
        }
    }

    #[test]
    fn manifest_retargeting_refuses_ordinary_and_queued_scripted_installers() {
        let (_dir, mut app, archive) = fixture();
        let inst = app.created.as_ref().unwrap().clone();
        let target = crate::update::collection_target(&app).unwrap();
        let original = eidos_instance::Manifest::read_checked(&inst.manifest_path())
            .unwrap()
            .unwrap();
        open(&mut app, &archive);
        let task = update(&mut app, Action::Answer(None));
        let mut changed = original.clone();
        changed.installation = Some("another installation".into());
        changed.write(&inst.manifest_path()).unwrap();
        assert!(crate::update::lock_install_target(&app, &target).is_err());
        drive(&mut app, task);
        assert!(app
            .installer
            .as_ref()
            .unwrap()
            .error
            .as_deref()
            .is_some_and(|s| s.contains("installation or profile changed")));
        changed = original.clone();
        changed.game_id = "fallout4".into();
        changed.write(&inst.manifest_path()).unwrap();
        assert!(crate::update::lock_install_target(&app, &target).is_err());
        fs::write(inst.manifest_path(), "invalid manifest").unwrap();
        assert!(crate::update::lock_install_target(&app, &target).is_err());
        original.write(&inst.manifest_path()).unwrap();
        assert!(crate::update::lock_install_target(&app, &target).is_ok());
        assert_eq!(crate::update::collection_target(&app), Some(target));
    }
}
