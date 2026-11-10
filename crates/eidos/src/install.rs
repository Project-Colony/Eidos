//! `eidos install` and `eidos import`: archives and existing folders in.

use std::process::exit;

use eidos_instance::InstanceKind;

use crate::*;

#[derive(Debug, Default)]
struct InstallOptions {
    target: String,
    archive: std::path::PathBuf,
    name: Option<String>,
    policy: eidos_install::OverwritePolicy,
    answers: Option<std::path::PathBuf>,
    save_answers: Option<std::path::PathBuf>,
    apply_profile_effects: bool,
    allow_incomplete: bool,
    /// Refuse a FOMOD install that offers a choice and has no recorded
    /// answers to replay.
    no_fomod_defaults: bool,
}
impl InstallOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        if args.len() < 2 || args[..2].iter().any(|s| s.starts_with("--")) {
            return Err("usage: eidos install <game-id-or-instance-path> <archive> [name] [--replace|--merge] [--backup] [--answers FILE] [--save-answers FILE] [--apply-profile-effects] [--allow-incomplete] [--no-fomod-defaults]".into());
        }
        let mut out = Self {
            target: args[0].clone(),
            archive: args[1].clone().into(),
            ..Self::default()
        };
        let (mut replace, mut merge, mut backup) = (false, false, false);
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--replace" if !replace => replace = true,
                "--merge" if !merge => merge = true,
                "--backup" if !backup => backup = true,
                "--apply-profile-effects" if !out.apply_profile_effects => {
                    out.apply_profile_effects = true
                }
                "--allow-incomplete" if !out.allow_incomplete => out.allow_incomplete = true,
                "--no-fomod-defaults" if !out.no_fomod_defaults => out.no_fomod_defaults = true,
                "--answers" | "--save-answers" => {
                    let slot = if args[i] == "--answers" {
                        &mut out.answers
                    } else {
                        &mut out.save_answers
                    };
                    i += 1;
                    let value = args
                        .get(i)
                        .filter(|v| !v.starts_with("--") && !v.is_empty())
                        .ok_or("The answers option needs a filename")?;
                    if slot.replace(value.into()).is_some() {
                        return Err("Duplicate answers option".into());
                    }
                }
                value if value.starts_with("--") => {
                    return Err(format!("Unknown or repeated install option: {value}"))
                }
                value => {
                    if out.name.replace(value.into()).is_some() {
                        return Err("Only one mod name is accepted".into());
                    }
                }
            }
            i += 1;
        }
        if replace && merge {
            return Err("Choose either --replace or --merge".into());
        }
        if backup && !replace && !merge {
            return Err("--backup needs --replace or --merge".into());
        }
        out.policy = match (replace, merge, backup) {
            (true, _, true) => eidos_install::OverwritePolicy::ReplaceWithBackup,
            (true, _, false) => eidos_install::OverwritePolicy::Replace,
            (_, true, true) => eidos_install::OverwritePolicy::MergeWithBackup,
            (_, true, false) => eidos_install::OverwritePolicy::Merge,
            _ => eidos_install::OverwritePolicy::Fail,
        };
        Ok(out)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InstallerAnswers {
    Custom {
        receipt: eidos_install::custom::CustomReceipt,
        #[serde(default)]
        prompt: Option<eidos_install::custom::CustomPrompt>,
    },
    Omod {
        receipt: eidos_install::scripted::OmodReplayReceipt,
        #[serde(default)]
        prompt: Option<eidos_install::obmm::ObmmPrompt>,
    },
}
const MAX_ANSWERS: usize = 16 * 1024 * 1024;
fn read_answers(path: &std::path::Path) -> Result<InstallerAnswers, String> {
    use std::io::Read;
    if !std::fs::metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("Answers must be a regular JSON file".into());
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Answers must be a regular JSON file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_ANSWERS as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_ANSWERS {
        return Err("Answers exceed 16 MiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid recorded installer answers: {e}"))
}
fn save_answers(
    path: Option<&std::path::Path>,
    archive: &std::path::Path,
    answers: &InstallerAnswers,
) -> Result<(), String> {
    let Some(path) = path else { return Ok(()) };
    use std::os::unix::fs::MetadataExt;
    if let (Ok(a), Ok(b)) = (std::fs::metadata(archive), std::fs::metadata(path)) {
        if a.dev() == b.dev() && a.ino() == b.ino() {
            return Err("Answers output must not replace the source archive".into());
        }
    }
    let bytes = serde_json::to_vec_pretty(answers).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_ANSWERS {
        return Err("Recorded answers exceed 16 MiB".into());
    }
    eidos_instance::write_atomic(path, &bytes)
        .map_err(|e| format!("Could not save installer answers: {e}"))
}

fn installer_context(
    inst: &eidos_instance::Instance,
    game: &eidos_games::DetectedGame,
) -> eidos_addons::Context {
    let mut controls: Vec<_> = game.plugin_state_dir().into_iter().collect();
    if let (Some(prefix), Some(spec)) = (game.prefix(), game.plugin_spec()) {
        controls.push(eidos_plugins::documents_my_games_dir(&prefix, &spec));
    }
    let files = crate::stock_game::game_files(game, inst);
    let context = eidos_install::custom::context_for_instance(
        inst,
        game.def.id,
        &files.root,
        &files.data,
        game.prefix().as_deref(),
        &controls,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap_or_else(|e| {
        eidos_log::warn!("Cannot capture installer context: {e}");
        exit(1)
    });
    context
}

fn retry_request(args: &[String]) -> Result<Option<(&str, &str)>, String> {
    if !args.iter().any(|a| a == "--retry-omod-effects") {
        return Ok(None);
    }
    if args.len() != 3
        || args[1] != "--retry-omod-effects"
        || args[0].starts_with("--")
        || eidos_install::fix_directory_name(&args[2]).as_deref() != Some(args[2].as_str())
    {
        return Err(
            "usage: eidos install <game-id-or-instance-path> --retry-omod-effects <mod-name>"
                .into(),
        );
    }
    Ok(Some((&args[0], &args[2])))
}

/// The FOMOD answers a CLI install uses: the stored ones when they still fit
/// the installer, the installer's defaults when there are none and either
/// `defaults_ok` or the installer asks nothing. A replay that no longer fits is
/// refused rather than finished with a guess.
fn fomod_selection(
    config: &eidos_fomod::ModuleConfig,
    ctx: &eidos_fomod::Context,
    stored: Option<&str>,
    defaults_ok: bool,
) -> Result<eidos_fomod::Selection, String> {
    let Some(replay) = stored.and_then(|json| eidos_install::replay_stored(config, ctx, json))
    else {
        if !defaults_ok && fomod_has_choice(config) {
            return Err("No FOMOD answers are recorded for this mod, so its installer's defaults would replace the options it was installed with. Update it once from the window, which records them.".into());
        }
        return Ok(eidos_fomod::default_selection(config, ctx));
    };
    if !replay.is_faithful() {
        return Err(format!(
            "The installer's options changed since the last install: {}. Reinstall from the window to choose again.",
            replay.unmatched.join("; ")
        ));
    }
    Ok(replay.selection)
}

/// Whether the installer lets the user change anything. When it does not, its
/// defaults are the only install it can make, so no recorded answers are
/// needed. A group asks nothing when it is empty, takes all its options, has a
/// single option it must take, or holds only options fixed as Required or Not
/// Usable with no condition that could change that.
fn fomod_has_choice(config: &eidos_fomod::ModuleConfig) -> bool {
    use eidos_fomod::{GroupType, PluginType};
    let fixed = |p: &eidos_fomod::Plugin| {
        p.type_descriptor.patterns.is_empty()
            && matches!(
                p.type_descriptor.default_type,
                PluginType::Required | PluginType::NotUsable
            )
    };
    config
        .steps
        .iter()
        .flat_map(|s| &s.groups)
        .any(|g| match g.group_type {
            GroupType::SelectAll => false,
            GroupType::SelectExactlyOne | GroupType::SelectAtLeastOne if g.plugins.len() == 1 => {
                false
            }
            _ => !g.plugins.iter().all(fixed),
        })
}

pub(crate) fn cmd_install(args: &[String]) {
    match retry_request(args) {
        Ok(Some((target, name))) => {
            let target = resolve(target);
            match eidos_install::scripted::retry_omod_effects(
                &target.inst,
                name,
                &std::sync::atomic::AtomicBool::new(false),
            ) {
                Ok(true) => println!("Approved OMOD effects completed for {name}."),
                Ok(false) => {
                    eidos_log::warn!("OMOD effects remain pending for {name}.");
                    exit(1);
                }
                Err(e) => {
                    eidos_log::warn!("OMOD effect retry failed: {e}");
                    exit(1);
                }
            }
            return;
        }
        Err(e) => {
            eidos_log::warn!("{e}");
            exit(2);
        }
        Ok(None) => {}
    }

    let options = InstallOptions::parse(args).unwrap_or_else(|e| {
        eidos_log::info!("{e}");
        exit(2)
    });
    let answers = options
        .answers
        .as_deref()
        .map(read_answers)
        .transpose()
        .unwrap_or_else(|e| {
            eidos_log::warn!("{e}");
            exit(2)
        });
    let target = resolve(&options.target);
    let Some(game) = find_instance_game(&target) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(1)
    };
    let inst = target.inst;
    let lock = inst.try_lock("eidos install").unwrap_or_else(|e| {
        eidos_log::warn!("Cannot install now: {e}");
        exit(1)
    });
    if let Err(e) = inst
        .create()
        .and_then(|_| inst.ensure_manifest(&target.game_id, InstanceKind::Global))
    {
        eidos_log::warn!("Cannot prepare this instance: {e}");
        exit(1)
    }
    let original_profile = inst.active_profile();
    let name = options
        .name
        .clone()
        .unwrap_or_else(|| eidos_install::mod_name_for(&options.archive));
    // Same fallback as the GUI: the install root for Morrowind and root-mode
    // Oblivion, where AppData has no state and every plugin would read active.
    // The FOMOD context itself is built in the Fomod arm only, as the GUI does:
    // it reads the plugin state of every enabled mod, and no other archive kind
    // ever looks at it.
    let fallback = game.plugin_state_dir();
    let addons = eidos_addons::load_addons();

    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = (|| -> Result<eidos_install::InstallReport, String> {
        use eidos_install::{custom, scripted, Opened};
        if options.policy == eidos_install::OverwritePolicy::Fail {
            if let Some(existing) = eidos_install::collision_name(&inst.mods_dir(), &name) {
                return Err(format!(
                    "Target already exists: {}. Use --replace to reinstall it.",
                    inst.mods_dir().join(existing).display()
                ));
            }
        }
        let native_omod = eidos_install::try_open_omod(&options.archive, &inst.mods_dir(), |_| {})
            .map_err(|e| e.to_string())?;
        let context = if native_omod.is_none()
            && (eidos_install::custom::may_handle(&addons, &target.game_id, &options.archive)
                || matches!(answers, Some(InstallerAnswers::Custom { .. })))
        {
            installer_context(&inst, &game)
        } else {
            eidos_addons::Context::default()
        };
        let opened = if let Some(session) = native_omod {
            if matches!(answers, Some(InstallerAnswers::Custom { .. })) {
                return Err("Recorded custom installer cannot change into an OMOD".into());
            }
            Opened::Omod(Box::new(session))
        } else if matches!(answers, Some(InstallerAnswers::Omod { .. })) {
            return Err("Recorded OMOD answers require the same OMOD container".into());
        } else {
            eidos_install::open_archive_with_installers(
                &options.archive,
                &inst.mods_dir(),
                &name,
                &target.game_id,
                &addons,
                &context,
                &cancel,
                match &answers {
                    Some(InstallerAnswers::Custom { receipt, .. }) => Some(receipt),
                    _ => None,
                },
                |_| {},
            )
            .map_err(|e| e.to_string())?
        };
        if (options.apply_profile_effects || options.allow_incomplete)
            && !matches!(&opened, Opened::Omod(session) if session.script.is_some())
        {
            return Err("Profile-effect and incomplete-effect approvals apply only to scripted OMOD installers".into());
        }
        match opened {
            Opened::Custom(session) => {
                match answers {
                    Some(InstallerAnswers::Custom { .. }) => {}
                    Some(_) => return Err(
                        "Recorded answers require a different installer; refusing native fallback"
                            .into(),
                    ),
                    None => {}
                }
                let prompt = if let custom::CustomEvaluation::Prompt(prompt) = &session.state {
                    Some(prompt.clone())
                } else {
                    None
                };
                save_answers(
                    options.save_answers.as_deref(),
                    &options.archive,
                    &InstallerAnswers::Custom {
                        receipt: session.receipt(),
                        prompt,
                    },
                )?;
                match &session.state {
                    custom::CustomEvaluation::Prompt(prompt)=>return Err(format!("Installer choice required: {}. Use --save-answers FILE to record the exact prompt, add its selected option indexes to receipt.answers, then replay with --answers FILE.",serde_json::to_string(prompt).unwrap())),
                    custom::CustomEvaluation::Manual(reason)=>return Err(format!("Custom installer requires manual handling: {reason}. Open this archive in the GUI to select native fallback.")),
                    custom::CustomEvaluation::Cancelled=>return Err("Custom installer cancelled; no mod installed".into()),
                    custom::CustomEvaluation::Ready(plan)=>for warning in &plan.warnings{eidos_log::warn!("Installer warning: {warning}");},
                }
                custom::finish_custom(
                    &session,
                    &inst.mods_dir(),
                    &name,
                    options.policy.clone(),
                    &eidos_addons::load_addons(),
                    &installer_context(&inst, &game),
                    &cancel,
                )
                .map_err(|e| e.to_string())
            }
            Opened::Omod(session) if session.script.is_some() => {
                drop(lock);
                let files = crate::stock_game::game_files(&game, &inst);
                let game = scripted::ScriptedGame {
                    game_id: target.game_id.clone(),
                    install_path: files.root.clone(),
                    data_path: files.data,
                    prefix: game.prefix(),
                    observed_versions: eidos_gamefeatures::preflight::inspect_pe(
                        &files.root.join("Oblivion.exe"),
                    )
                    .ok()
                    .and_then(|pe| pe.file_version)
                    .map(|version| {
                        [("Oblivion".into(), version.to_string())]
                            .into_iter()
                            .collect()
                    })
                    .unwrap_or_default(),
                };
                let context = scripted::ScriptedContext::capture(&inst, &game, &cancel)
                    .map_err(|e| e.to_string())?;
                let receipt = match answers {
                    Some(InstallerAnswers::Omod { receipt, .. }) => receipt,
                    Some(_) => return Err(
                        "Recorded answers require a different installer; refusing native fallback"
                            .into(),
                    ),
                    None => scripted::new_omod_receipt(&session, &context, &cancel)
                        .map_err(|e| e.to_string())?,
                };
                match scripted::evaluate_scripted_omod(&session, &context, &receipt, &cancel)
                    .map_err(|e| e.to_string())?
                {
                    scripted::ScriptedEvaluation::NeedPrompt(prompt) => {
                        save_answers(
                            options.save_answers.as_deref(),
                            &options.archive,
                            &InstallerAnswers::Omod {
                                receipt,
                                prompt: Some(prompt.clone()),
                            },
                        )?;
                        Err(format!("OMOD choice required: {}. Record the exact prompt and answer with --save-answers FILE, then replay with --answers FILE.",serde_json::to_string(&prompt).unwrap()))
                    }
                    scripted::ScriptedEvaluation::Review(review) => {
                        save_answers(
                            options.save_answers.as_deref(),
                            &options.archive,
                            &InstallerAnswers::Omod {
                                receipt: review.receipt.clone(),
                                prompt: None,
                            },
                        )?;
                        for warning in review.plan.warnings.iter().chain(&review.unsupported) {
                            eidos_log::warn!("OMOD warning: {warning}");
                        }
                        for effect in &review.profile_effects {
                            eidos_log::info!(
                                "Requested profile effect at line {}: {} {:?}",
                                effect.line,
                                effect.command,
                                effect.arguments
                            );
                        }
                        let result = scripted::install_scripted_omod(
                            &session,
                            &context,
                            &review,
                            &name,
                            options.policy.clone(),
                            scripted::ScriptedApproval {
                                apply_profile_effects: options.apply_profile_effects,
                                allow_incomplete: options.allow_incomplete,
                            },
                            &cancel,
                        )
                        .map_err(|e| e.to_string())?;
                        for warning in result.warnings {
                            eidos_log::warn!("OMOD warning: {warning}");
                        }
                        if result.pending_effects {
                            eidos_log::warn!(
                                "OMOD installed with pending effects; receipt retained at {}",
                                result.receipt_path.display()
                            );
                        }
                        Ok(result.install)
                    }
                }
            }
            native => {
                if answers.is_some() {
                    return Err("Recorded installer is unavailable or no longer matches this archive; refusing native fallback".into());
                }
                if options.save_answers.is_some() {
                    return Err("This native installer has no custom/OMOD answer receipt".into());
                }
                match native {
                    Opened::Omod(session) => eidos_install::install_omod(
                        &session,
                        &inst.mods_dir(),
                        &name,
                        &target.game_id,
                        options.policy.clone(),
                    ),
                    Opened::Fomod(session) => {
                        let fomod = eidos_install::fomod_context_for_instance(
                            &inst,
                            &crate::stock_game::game_files(&game, &inst).data,
                            &target.game_id,
                            fallback.as_deref(),
                        );
                        // The answers the folder being replaced was installed
                        // with. Fail never replaces: the collision check above
                        // already refused an existing folder.
                        let stored = (options.policy != eidos_install::OverwritePolicy::Fail)
                            .then(|| eidos_install::collision_name(&inst.mods_dir(), &name))
                            .flatten()
                            .and_then(|folder| inst.mod_meta(&folder).fomod_choices());
                        let selection = fomod_selection(
                            &session.config,
                            &fomod,
                            stored.as_deref(),
                            !options.no_fomod_defaults,
                        )?;
                        eidos_install::finish_fomod(
                            *session,
                            &selection,
                            &inst.mods_dir(),
                            &target.game_id,
                            &fomod,
                            options.policy.clone(),
                        )
                    }
                    // `classify_extracted` returns these only when the tree has no
                    // FOMOD wizard, and the context is read only for one.
                    Opened::Simple(tree) | Opened::Manual(tree) | Opened::Bain { tree, .. } => {
                        eidos_install::install_extracted(
                            &tree,
                            &options.archive,
                            &inst.mods_dir(),
                            &name,
                            &target.game_id,
                            options.policy.clone(),
                            &eidos_fomod::Context::default(),
                        )
                    }
                    Opened::Custom(_) => unreachable!(),
                }
                .map_err(|e| e.to_string())
            }
        }
    })();
    match result {
        Ok(report) => {
            // Keep registration attached to the profile selected before extraction.
            let _registration = inst
                .try_lock("registering CLI install")
                .unwrap_or_else(|e| {
                    eidos_log::warn!(
                        "Installed '{}', but registration lock failed: {e}",
                        report.name
                    );
                    exit(1)
                });
            if inst.active_profile() != original_profile {
                eidos_log::warn!(
                    "Installed '{}', but the active profile changed; registration was left pending",
                    report.name
                );
                exit(1)
            }
            if let Err(e) = inst.register_installed_mod(&report.name) {
                eidos_log::warn!(
                    "Installed '{}', but could not register it: {e}",
                    report.name
                );
                exit(1)
            }
            let _ = eidos_nexus::mark_installed(&options.archive);
            println!(
                "Installed '{}' for {}\n  -> {}",
                report.name,
                game.def.name,
                report.dest.display()
            );
            if let Some(backup) = report.backup {
                println!("  backup: {}", backup.display());
            }
            for missing in report.missing {
                eidos_log::warn!("Installer expected missing archive file: {missing}");
            }
            println!("Registered in the active profile; an existing mod keeps its priority and enabled state.");
        }
        Err(e) => {
            eidos_log::warn!("install failed: {e}");
            exit(1)
        }
    }
}

/// `eidos import <game-id> <mo2-profile-dir>`: adopt an existing Mod Organizer 2
/// profile's mod order, enabled states and load order.
pub(crate) fn cmd_import(args: &[String]) -> ! {
    let (Some(id), Some(dir)) = (args.first(), args.get(1)) else {
        usage()
    };
    let target = resolve(id);
    let inst = target.inst;
    if !inst.exists() {
        eidos_log::info!("eidos import: no instance for '{id}' - run `eidos init {id}` first.");
        exit(1);
    }
    match inst.import_mo2_profile(std::path::Path::new(dir)) {
        Ok(r) => {
            println!(
                "Imported {} mod(s) from {dir} into profile '{}'.",
                r.matched,
                inst.active_profile()
            );
            if r.kept_local > 0 {
                println!(
                    "{} local mod(s) MO2 did not list were kept at the bottom.",
                    r.kept_local
                );
            }
            if r.plugin_files > 0 {
                println!("Load order imported ({} file(s)).", r.plugin_files);
            }
            if !r.missing.is_empty() {
                println!(
                    "\n{} mod(s) MO2 listed are not installed here:",
                    r.missing.len()
                );
                for m in r.missing.iter().take(40) {
                    println!("  - {m}");
                }
                if r.missing.len() > 40 {
                    println!("  ... and {} more", r.missing.len() - 40);
                }
                println!("Install them, then run this again to place them in order.");
            }
            exit(0)
        }
        Err(e) => {
            eidos_log::info!("eidos import: {e}");
            exit(1)
        }
    }
}

#[cfg(test)]
mod installer_option_tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<InstallOptions, String> {
        InstallOptions::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    #[test]
    fn retry_mode_is_explicit_and_has_no_archive_or_install_flags() {
        let strings = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            retry_request(&strings(&["oblivion", "--retry-omod-effects", "Mod"])).unwrap(),
            Some(("oblivion", "Mod"))
        );
        for args in [
            vec!["--retry-omod-effects", "Mod"],
            vec!["oblivion", "--retry-omod-effects", "../other"],
            vec!["oblivion", "--retry-omod-effects", "Mod", "--replace"],
        ] {
            assert!(retry_request(&strings(&args)).is_err());
        }
        assert_eq!(
            retry_request(&strings(&["oblivion", "archive.omod"])).unwrap(),
            None
        );
    }

    #[test]
    fn options_keep_answer_filenames_out_of_the_mod_name() {
        let got = parse(&[
            "skyrimse",
            "archive.zip",
            "--answers",
            "saved choices.json",
            "My Mod",
            "--replace",
            "--backup",
            "--save-answers",
            "new choices.json",
            "--apply-profile-effects",
            "--allow-incomplete",
            "--no-fomod-defaults",
        ])
        .unwrap();
        assert_eq!(got.name.as_deref(), Some("My Mod"));
        assert_eq!(
            got.answers.as_deref(),
            Some(std::path::Path::new("saved choices.json"))
        );
        assert_eq!(
            got.save_answers.as_deref(),
            Some(std::path::Path::new("new choices.json"))
        );
        assert_eq!(
            got.policy,
            eidos_install::OverwritePolicy::ReplaceWithBackup
        );
        assert!(got.apply_profile_effects && got.allow_incomplete && got.no_fomod_defaults);
        assert!(
            parse(&["oblivion", "mod.omod", "--answers", "choices.json"])
                .unwrap()
                .name
                .is_none()
        );
    }
    #[test]
    fn options_reject_ambiguous_missing_or_unknown_values() {
        for tail in [
            &["--answers"][..],
            &["--answers", "--replace"],
            &["--replace", "--merge"],
            &["--backup"],
            &["one", "two"],
            &["--unknown"],
            &["--answers", "a", "--answers", "b"],
            &["--replace", "--replace"],
            &["--apply-profile-effects", "--apply-profile-effects"],
            &["--no-fomod-defaults", "--no-fomod-defaults"],
        ] {
            let args: [Vec<&str>; 2] = [vec!["oblivion", "mod.omod"], tail.to_vec()];
            assert!(parse(&args.concat()).is_err(), "{tail:?}");
        }
        assert!(parse(&[]).is_err());
        assert!(parse(&["oblivion"]).is_err());
    }
    #[test]
    fn answer_files_reject_wrong_schemas_and_cannot_overwrite_the_archive() {
        static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "eidos-cli-answers-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let archive = root.join("archive.zip");
        std::fs::write(&archive, b"source").unwrap();
        let answers = InstallerAnswers::Custom {
            receipt: eidos_install::custom::CustomReceipt {
                version: 1,
                installer: "fixture".into(),
                installer_version: "1".into(),
                helper_sha256: "h".into(),
                archive_sha256: "a".into(),
                source_sha256: "s".into(),
                context_sha256: "c".into(),
                answers: vec![],
                plan: None,
            },
            prompt: Some(eidos_install::custom::CustomPrompt {
                id: "pick".into(),
                title: "Choose".into(),
                options: vec!["One".into()],
                multiple: false,
            }),
        };
        assert!(save_answers(Some(&archive), &archive, &answers).is_err());
        let alias = root.join("alias");
        std::fs::hard_link(&archive, &alias).unwrap();
        assert!(save_answers(Some(&alias), &archive, &answers).is_err());
        let path = root.join("answers.json");
        save_answers(Some(&path), &archive, &answers).unwrap();
        assert!(matches!(
            read_answers(&path).unwrap(),
            InstallerAnswers::Custom {
                prompt: Some(_),
                ..
            }
        ));
        std::fs::write(&path, b"{\"kind\":\"unknown\"}").unwrap();
        assert!(read_answers(&path).is_err());
        std::fs::write(
            &path,
            b"{\"kind\":\"custom\",\"receipt\":{},\"unexpected\":true}",
        )
        .unwrap();
        assert!(read_answers(&path).is_err());
        assert_eq!(std::fs::read(&archive).unwrap(), b"source");
        std::fs::remove_dir_all(root).unwrap();
    }

    fn replay_config(snow: &str) -> eidos_fomod::ModuleConfig {
        eidos_fomod::ModuleConfig::parse(&format!(
            r#"<config><installSteps order="Explicit">
<installStep name="Main"><optionalFileGroups order="Explicit"><group name="Textures" type="SelectExactlyOne"><plugins order="Explicit">
<plugin name="SD"><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
<plugin name="HD"><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
</plugins></group></optionalFileGroups></installStep>
<installStep name="Extras"><optionalFileGroups><group name="Patches" type="SelectAny"><plugins>
<plugin name="{snow}"><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
</plugins></group></optionalFileGroups></installStep>
</installSteps></config>"#
        ))
        .unwrap()
    }

    #[test]
    fn cli_fomod_selection_refuses_an_unfaithful_replay() {
        let ctx = eidos_fomod::Context::default();
        let config = replay_config("Snow");
        let picked: eidos_fomod::Selection = vec![vec![vec![false, true]], vec![vec![true]]];
        let stored_same =
            eidos_install::fomod_choices_json(&eidos_fomod::to_recorded(&config, &picked, &ctx));
        assert_eq!(
            fomod_selection(&config, &ctx, Some(&stored_same), true).unwrap(),
            picked
        );
        let err = fomod_selection(
            &replay_config("Snow (Cathedral)"),
            &ctx,
            Some(&stored_same),
            true,
        )
        .unwrap_err();
        assert!(err.contains("Reinstall from the window"), "{err}");
        assert!(err.contains("\"Snow\""), "{err}");
        assert_eq!(
            fomod_selection(&config, &ctx, None, true).unwrap(),
            eidos_fomod::default_selection(&config, &ctx)
        );
        // `nexus update --apply` installs unattended: no recorded answers, or
        // unreadable ones, is a refusal there, never the defaults.
        for stored in [None, Some("{")] {
            let err = fomod_selection(&config, &ctx, stored, false).unwrap_err();
            assert!(err.contains("Update it once from the window"), "{err}");
        }
        assert_eq!(
            fomod_selection(&config, &ctx, Some(&stored_same), false).unwrap(),
            picked
        );
    }

    #[test]
    fn unattended_fomod_defaults_are_kept_when_the_installer_asks_nothing() {
        let ctx = eidos_fomod::Context::default();
        let parse = |groups: &str| {
            eidos_fomod::ModuleConfig::parse(&format!(
                r#"<config><requiredInstallFiles><file source="a.esp"/></requiredInstallFiles><installSteps order="Explicit"><installStep name="Main"><optionalFileGroups order="Explicit">{groups}</optionalFileGroups></installStep></installSteps></config>"#
            ))
            .unwrap()
        };
        let group = |kind: &str, plugins: &[&str]| {
            let plugins: String = plugins
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    format!(r#"<plugin name="P{i}"><typeDescriptor><type name="{t}"/></typeDescriptor></plugin>"#)
                })
                .collect();
            format!(
                r#"<group name="G" type="{kind}"><plugins order="Explicit">{plugins}</plugins></group>"#
            )
        };
        let nothing_to_choose = [
            eidos_fomod::ModuleConfig::parse(
                r#"<config><requiredInstallFiles><file source="a.esp"/></requiredInstallFiles></config>"#,
            )
            .unwrap(),
            parse(""),
            parse(&group("SelectAny", &["Required", "NotUsable"])),
            parse(&group("SelectAll", &["Optional", "Optional"])),
            parse(&group("SelectExactlyOne", &["Optional"])),
        ];
        for config in &nothing_to_choose {
            assert_eq!(
                fomod_selection(config, &ctx, None, false).unwrap(),
                eidos_fomod::default_selection(config, &ctx),
                "{config:?}"
            );
        }
        for kind in ["SelectAny", "SelectAtMostOne"] {
            let config = parse(&group(kind, &["Optional"]));
            let err = fomod_selection(&config, &ctx, None, false).unwrap_err();
            assert!(err.contains("Update it once from the window"), "{err}");
        }
        let config = parse(&group("SelectExactlyOne", &["Recommended", "Optional"]));
        assert!(fomod_selection(&config, &ctx, None, false).is_err());
    }
}
