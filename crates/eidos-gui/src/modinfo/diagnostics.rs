//! The Health tab: every check Eidos runs on the open setup, and its panel.

use super::*;

/// How serious a diagnostic is: `Problem` needs action (it will break or lose
/// something), `Advice` is worth knowing, `Ok` is a passing check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagLevel {
    Problem,
    Advice,
    Ok,
}

/// One health check: what it found, and what to do about it.
#[derive(Clone)]
pub(crate) struct Diagnostic {
    pub(crate) level: DiagLevel,
    pub(crate) title: String,
    pub(crate) detail: String,
    /// One-click remedies, rendered as buttons on the card. Most checks only
    /// inform; the ones that can FIX what they found carry the fix with them, so
    /// recovery is not a file-manager expedition. More than one when the finding
    /// has two honest outcomes (restore vs accept).
    pub(crate) actions: Vec<(&'static str, Message)>,
}

/// Run every health check for the current setup - MO2's problems panel, plus the
/// Linux-specific ones MO2 never needed (the launch capability above all, which
/// silently disables FUSE passthrough after each rebuild).
pub(crate) fn diagnostics(app: &App) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

    // Eidos's own machinery first, before anything about the user's mods. When
    // the prefix is unusable nothing below is actionable: a tool that cannot
    // find the game will not be fixed by tidying an archive, and a user reading
    // top to bottom should meet the cause before the symptoms.
    if let Some(f) = crate::state::selected_game(app).and_then(crate::health::prefix_facts) {
        out.extend(crate::health::prefix_checks(&f));
    }
    out.extend(crate::health::nexus_checks(&crate::health::NexusFacts {
        signed_in: app.nexus_account.is_some(),
        last_error: app.nexus_error.clone(),
    }));

    // What the user's own `diagnose` extensions reported, FIRST and attributed by
    // name. First because someone who wrote a check wants to see it; attributed
    // because a row that reads like one of Eidos's own would put its own
    // authority behind something Eidos did not check.
    for (who, f) in &app.addon_findings {
        out.push(Diagnostic {
            level: match f.level {
                eidos_addons::FindingLevel::Problem => DiagLevel::Problem,
                eidos_addons::FindingLevel::Advice => DiagLevel::Advice,
                eidos_addons::FindingLevel::Ok => DiagLevel::Ok,
            },
            title: format!("{} - {}", who, f.title),
            detail: f.detail.clone(),
            actions: Vec::new(),
        });
    }

    // First, because while it is showing nothing else in this tab is trustworthy:
    // the mod list Eidos is working from does not match what is on disk, so the
    // conflict map, the load order and the layer stack are all built from a
    // partial picture. Saving is refused for as long as this is true.
    if let Some(why) = app
        .created
        .as_ref()
        .and_then(|i| i.modlist_checked().1.reason().map(str::to_string))
    {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: t("diag_modlist_mismatch").to_string(),
            detail: tf("diag_modlist_mismatch_detail", &[("reason", &why)]),
            actions: Vec::new(),
        });
    }

    // The launch capability is optional: it only gates FUSE passthrough, which is
    // off by default because it stops the game opening its own archives and
    // plugins. So this is only worth a Problem when passthrough was asked for.
    if passthrough_requested() {
        out.push(if app.cap_missing {
            Diagnostic {
                level: DiagLevel::Problem,
                title: t("diag_passthrough_unavailable").to_string(),
                detail: tf(
                    "diag_passthrough_unavailable_detail",
                    &[("command", &setcap_hint())],
                ),
                actions: Vec::new(),
            }
        } else {
            Diagnostic {
                level: DiagLevel::Advice,
                title: t("diag_passthrough_on").to_string(),
                detail: t("diag_passthrough_on_detail").to_string(),
                actions: Vec::new(),
            }
        });
    } else {
        out.push(Diagnostic {
            level: DiagLevel::Ok,
            title: t("diag_passthrough_off").to_string(),
            detail: t("diag_passthrough_off_detail").to_string(),
            actions: Vec::new(),
        });
    }

    // Dropping a file from the file manager is inert on a native Wayland
    // session, and there is no moment of failure to hang a message on: the event
    // never fires, so the window cannot know a drop was attempted. It has to be
    // said somewhere findable instead, and this tab is where "why did that do
    // nothing" is meant to be answered.
    if on_wayland() {
        out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: t("diag_wayland_drops").to_string(),
            detail: t("diag_wayland_drops_detail").to_string(),
            actions: Vec::new(),
        });
    }

    // Missing masters: the single most reliable crash predictor.
    //
    // `app.plugins` is a CACHE - dropped on every mod-list change and only
    // rebuilt while the Plugins tab is open - so most of the time it is `None`.
    // This check used to skip itself and print "load order not computed yet",
    // which reads as reassurance and is not: it says nothing was looked at, on
    // the one check most likely to predict a crash. If the cache is cold,
    // compute the answer. "Something changed" includes every checkbox click and
    // every Ctrl+Arrow move, so `compute_plugins` must stay cheap: it reads the
    // layers' root listings, never a full index of every mod's files.
    let computed;
    let plugins = match app.plugins.as_ref() {
        Some(list) => Some(list),
        None => {
            computed = compute_plugins(app);
            computed.as_ref()
        }
    };
    match plugins {
        Some(list) => {
            if let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) {
                let mut unverified = Vec::new();
                for d in list.diagnostics(&spec) {
                    if d.code == "plugin_records_unverified" {
                        let checked = app
                            .loot_meta
                            .as_ref()
                            .and_then(|m| m.get(&d.plugin.to_ascii_lowercase()))
                            .is_some_and(|m| m.record_validity.is_some());
                        if !checked {
                            unverified.push(d.plugin);
                        }
                        continue;
                    }
                    let origin = if d.origin_mod.is_empty() {
                        t("diag_origin_game")
                    } else {
                        &d.origin_mod
                    };
                    out.push(Diagnostic {
                        level: match d.severity {
                            eidos_plugins::DiagnosticSeverity::Error => DiagLevel::Problem,
                            _ => DiagLevel::Advice,
                        },
                        title: format!("{:?}: {} [{}]", d.severity, d.plugin, d.code),
                        detail: format!("{} ({origin}): {}", d.plugin, d.detail),
                        actions: Vec::new(),
                    });
                }
                if !unverified.is_empty() {
                    out.push(Diagnostic {
                        level: DiagLevel::Advice,
                        title: tf("diag_unverified_limits", &[("n", &unverified.len())]),
                        detail: tf(
                            if unverified.len() > 8 {
                                "diag_unverified_limits_detail_more"
                            } else {
                                "diag_unverified_limits_detail"
                            },
                            &[(
                                "plugins",
                                &unverified
                                    .iter()
                                    .take(8)
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            )],
                        ),
                        actions: Vec::new(),
                    });
                }
                for p in list.plugins.iter().filter(|p| p.enabled) {
                    if let Some(meta) = app
                        .loot_meta
                        .as_ref()
                        .and_then(|m| m.get(&p.name.to_ascii_lowercase()))
                    {
                        for message in &meta.messages {
                            if message.kind == eidos_loot::MessageType::Say {
                                continue;
                            }
                            out.push(Diagnostic {
                                level: if message.kind == eidos_loot::MessageType::Error {
                                    DiagLevel::Problem
                                } else {
                                    DiagLevel::Advice
                                },
                                title: format!("LOOT: {}", p.name),
                                detail: format!(
                                    "{} ({}): {}",
                                    p.name,
                                    if p.origin_mod.is_empty() {
                                        t("diag_origin_game")
                                    } else {
                                        &p.origin_mod
                                    },
                                    message.text
                                ),
                                actions: Vec::new(),
                            });
                        }
                    }
                }
            }
            let missing = list.missing_masters();
            let unreadable = list
                .plugins
                .iter()
                .filter(|p| p.enabled && p.header_error.is_some())
                .count();
            if missing.is_empty() && unreadable == 0 {
                out.push(Diagnostic {
                    level: DiagLevel::Ok,
                    title: t("diag_no_missing_masters").to_string(),
                    detail: tf(
                        "diag_no_missing_masters_detail",
                        &[("n", &list.plugins.iter().filter(|p| p.enabled).count())],
                    ),
                    actions: Vec::new(),
                });
            } else if !missing.is_empty() {
                let mut detail = missing
                    .iter()
                    .take(8)
                    .map(|(p, m)| tf("diag_needs_master", &[("plugin", &p), ("master", &m)]))
                    .collect::<Vec<_>>()
                    .join("; ");
                if missing.len() > 8 {
                    detail.push_str(&tf("diag_and_n_more", &[("n", &(missing.len() - 8))]));
                }
                out.push(Diagnostic {
                    level: DiagLevel::Problem,
                    title: tf("diag_missing_masters", &[("n", &missing.len())]),
                    detail: tf("diag_missing_masters_detail", &[("list", &detail)]),
                    actions: Vec::new(),
                });
            } else {
                out.push(Diagnostic {
                    level: DiagLevel::Advice,
                    title: t("diag_unverified_masters").into(),
                    detail: tf("diag_unverified_masters_detail", &[("n", &unreadable)]),
                    actions: Vec::new(),
                });
            }
        }
        // Reachable when there is no game yet, not when nobody has opened a tab.
        // Say which, rather than asking the user to go and do the program's job.
        //
        // NOT reachable for a game that simply has no plugins: telling a Stellar
        // Blade user that their load order is unavailable describes a thing that
        // does not exist for their game, which reads as something being broken.
        None if game_has_plugins(app) => out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: t("diag_load_order_unavailable").to_string(),
            detail: t("diag_load_order_unavailable_detail").to_string(),
            actions: Vec::new(),
        }),
        None => {}
    }

    // ENB + Community Shaders both injecting into D3D11.
    if let (Some(game), Some(inst)) = (selected_game(app), app.created.as_ref()) {
        let mods = app
            .mods
            .iter()
            .rev()
            .filter(|m| m.is_active() && !m.is_unmanaged())
            .map(|m| (m.name.clone(), m.path.clone()))
            .collect::<Vec<_>>();
        for d in eidos_gamefeatures::preflight::scan_skse(
            game.def,
            &game.data_path,
            &game.install_path,
            &mods,
            &inst.overwrite_dir(),
        ) {
            out.push(Diagnostic {
                level: if d.severity == eidos_gamefeatures::preflight::DiagnosticSeverity::Error {
                    DiagLevel::Problem
                } else {
                    DiagLevel::Advice
                },
                title: tf(
                    "diag_native_check",
                    &[
                        ("severity", &format!("{:?}", d.severity)),
                        ("code", &d.code),
                    ],
                ),
                detail: format!("{} ({}): {}", d.path.display(), d.origin_mod, d.detail),
                actions: Vec::new(),
            });
        }
        let cs_roots: Vec<PathBuf> = inst
            .modlist()
            .into_iter()
            .filter(|m| m.is_active())
            .map(|m| m.path)
            .collect();
        if eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: t("diag_enb_cs").to_string(),
                detail: t("diag_enb_cs_detail").to_string(),
                actions: Vec::new(),
            });
        }
        // Eidos forces d3dcompiler_47 native at launch but ships no Microsoft
        // DLL; without the real one the mod meets Wine's builtin and fails to load.
        // The prefix check is cheap, so it runs before the DLL import scan.
        let windows = game.prefix().map(|p| p.join("drive_c").join("windows"));
        if windows
            .as_deref()
            .is_some_and(|w| !eidos_gamefeatures::native_dll_present(w, "d3dcompiler_47"))
            && eidos_gamefeatures::scan_imports_provisionable(&cs_roots)
        {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: t("diag_d3dcompiler").to_string(),
                detail: t("diag_d3dcompiler_detail").to_string(),
                actions: vec![(t("diag_action_install"), Message::SetupPrereqs)],
            });
        }
    }

    // A non-empty Overwrite is generated content sitting outside any mod.
    if let Some(inst) = app.created.as_ref() {
        for m in app
            .mods
            .iter()
            .filter(|m| !m.unmanaged && !m.is_separator())
        {
            if let Some(warning) = inst.mod_meta(&m.name).install_warning() {
                out.push(Diagnostic {
                    level: if m.is_active() {
                        DiagLevel::Problem
                    } else {
                        DiagLevel::Advice
                    },
                    title: tf("diag_incomplete_install", &[("name", &m.name)]),
                    detail: format!("{}: {warning}", m.name),
                    actions: Vec::new(),
                });
            }
        }
        match inst.pending_tool_runs() {
            Ok(runs) => {
                for run in runs {
                    out.push(Diagnostic {
                        level: DiagLevel::Advice,
                        title: tf("diag_unfinished_tool", &[("tool", &run.tool)]),
                        detail: tf(
                            "diag_unfinished_tool_detail",
                            &[("started", &run.started_at), ("profile", &run.profile)],
                        ),
                        actions: Vec::new(),
                    });
                }
            }
            Err(e) => out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: t("diag_tool_history_unverified").into(),
                detail: e.to_string(),
                actions: Vec::new(),
            }),
        }
        let active_plugins = plugins
            .map(|list| {
                list.plugins
                    .iter()
                    .map(|p| (p.name.clone(), p.enabled))
                    .collect::<Vec<_>>()
            })
            .or_else(|| (!game_has_plugins(app)).then(Vec::new));
        match inst.generated_outputs() {
            Ok(outputs) => {
                for (owner, output) in outputs {
                    if owner.as_ref().is_some_and(|owner| {
                        !app.mods
                            .iter()
                            .any(|m| m.is_active() && m.name.eq_ignore_ascii_case(owner))
                    }) {
                        continue;
                    }
                    let Some(active_plugins) = active_plugins.as_ref() else {
                        out.push(Diagnostic {
                            level: DiagLevel::Advice,
                            title: t("diag_generated_inputs_unverified").into(),
                            detail: tf(
                                "diag_generated_inputs_unverified_detail",
                                &[
                                    ("tool", &output.receipt.tool),
                                    ("owner", &owner.as_deref().unwrap_or("Overwrite")),
                                ],
                            ),
                            actions: Vec::new(),
                        });
                        continue;
                    };
                    if inst.generated_input_drift(&output.receipt, active_plugins) {
                        out.push(Diagnostic {
                            level: DiagLevel::Advice,
                            title: t("diag_generated_inputs_changed").into(),
                            detail: tf(
                                "diag_generated_inputs_changed_detail",
                                &[
                                    ("tool", &output.receipt.tool),
                                    ("owner", &owner.as_deref().unwrap_or("Overwrite")),
                                    (
                                        "files",
                                        &format!(
                                            "{}{}",
                                            output
                                                .paths
                                                .iter()
                                                .take(8)
                                                .map(|p| p.display().to_string())
                                                .collect::<Vec<_>>()
                                                .join(", "),
                                            if output.paths.len() > 8 { ", …" } else { "" }
                                        ),
                                    ),
                                ],
                            ),
                            actions: Vec::new(),
                        });
                    }
                }
            }
            Err(e) => out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: t("diag_generated_provenance_unverified").into(),
                detail: e.to_string(),
                actions: Vec::new(),
            }),
        }
        if !inst.overwrite_is_empty() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: t("diag_overwrite_generated").to_string(),
                detail: t("diag_overwrite_generated_detail").to_string(),
                actions: Vec::new(),
            });
        }
        // Debris from an interrupted install.
        let debris: Vec<String> = fs::read_dir(inst.mods_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with(".eidos-install"))
            .collect();
        if !debris.is_empty() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: tf("diag_debris", &[("n", &debris.len())]),
                detail: tf(
                    "diag_debris_detail",
                    &[("path", &inst.mods_dir().display())],
                ),
                // A check that knows the answer should not end by naming a
                // directory and wishing the user luck. This one can only ever
                // remove `mods/.eidos-install*`, which Eidos created itself and
                // which nothing else reads.
                actions: vec![(t("diag_action_delete_them"), Message::CleanInstallDebris)],
            });
        }
    }

    for warning in &app.archive_warnings {
        out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: t("diag_archive_contents_unverified").into(),
            detail: warning.clone(),
            actions: Vec::new(),
        });
    }

    // The last session wrecked the active set (a crash artifact written straight
    // into the bound profile dir): the pre-session snapshot noticed, and the fix
    // is one click. Also fires when the user deliberately disabled most plugins
    // in-game - they dismiss it by playing on; restoring is never automatic.
    if let (Some(inst), Some(spec)) = (
        app.created.as_ref(),
        selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)),
    ) {
        let prof = inst.active();
        if let Some(reason) = prof.plugin_loss_since_snapshot(&spec) {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: t("diag_plugin_loss").to_string(),
                detail: tf("diag_plugin_loss_detail", &[("reason", &reason)]),
                actions: vec![
                    (
                        t("diag_action_restore_presession"),
                        Message::RestorePreSessionPlugins,
                    ),
                    (t("diag_action_keep_set"), Message::AcceptPluginState),
                ],
            });
        }
    }

    // The game rewrites its own load order; a profile that never captured one is
    // still riding on the prefix's copy.
    if let Some(inst) = app.created.as_ref().filter(|_| game_manages_plugins(app)) {
        let prof = inst.active();
        if !prof.has_plugin_state() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: tf("diag_profile_no_load_order", &[("profile", &prof.name)]),
                detail: t("diag_profile_no_load_order_detail").to_string(),
                actions: Vec::new(),
            });
        }
    }

    // LOOT coverage for this game.
    if let Some(game) = selected_game(app) {
        // Only worth saying for a game that HAS plugins. LOOT sorts Bethesda
        // plugin files and nothing else, so "LOOT cannot sort Stellar Blade" is
        // true of every game ever made that is not Bethesda's - and it pointed at
        // a Plugins tab that this game does not even show.
        if game_has_plugins(app) && !eidos_loot::is_supported(game.def.id) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: tf("diag_loot_cannot_sort", &[("game", &game.def.name)]),
                detail: t("diag_loot_cannot_sort_detail").to_string(),
                actions: Vec::new(),
            });
        }
        // A Flatpak-Steam Proton runs from the host here, which can fail to resolve
        // its sandbox libraries. Eidos will not re-launch through `flatpak run`:
        // that would put the game in Flatpak's sandbox, blind to the FUSE union in
        // our private namespace, and it would silently play vanilla.
        if let Some(cd) = game.compatdata.as_ref() {
            let flatpak =
                eidos_games::proton_command(&home(), game.def.steam_app_id, cd, &game.install_path)
                    .is_some_and(|r| r.flatpak);
            if flatpak {
                out.push(Diagnostic {
                    level: DiagLevel::Problem,
                    title: t("diag_flatpak_proton").to_string(),
                    detail: t("diag_flatpak_proton_detail").to_string(),
                    actions: Vec::new(),
                });
            }
        }
        if !game.is_steam() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: tf("diag_manual_launch", &[("source", &game.source_name())]),
                detail: t("diag_manual_launch_detail").into(),
                actions: Vec::new(),
            });
        }
        if game.prefix().is_none() && game.is_steam() {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: t("diag_no_prefix").to_string(),
                detail: t("diag_no_prefix_detail").to_string(),
                actions: Vec::new(),
            });
        }
        out.extend(orphan_archive_diagnostics(app, game.def.id));
        out.extend(script_extender_diagnostic(game));
    }

    out
}

/// What the script extender itself recorded about each of its plugin DLLs on the
/// last run.
///
/// The passthrough check above says whether DLL loading is *likely* to work. This
/// says what happened. The distinction matters because the two failure modes look
/// identical from outside: a plugin refused for an incompatible runtime version
/// and one the manager failed to expose both end with the feature simply absent
/// in game.
/// How one refused plugin reads in the report.
///
/// By the name its author declared, not by its file. A plugin may spell its DLL
/// however it likes to control WHERE it loads - the extender goes alphabetically,
/// and one real plugin prefixes itself `!!!!!!!##$` to hook cosaves before
/// anything else. That is a load-order device and it works; it is not a name,
/// and a report that shows it asks the reader to recognise a mod by its sorting
/// trick. The declared name already falls back to the file name when a plugin
/// declared none, so nothing is ever unidentified.
pub(crate) fn failed_plugin_line(p: &eidos_gamefeatures::SePluginLoad) -> String {
    format!("{}: {}", p.name, p.status)
}

pub(crate) fn script_extender_diagnostic(game: &DetectedGame) -> Option<Diagnostic> {
    let spec = game.plugin_spec()?;
    let prefix = game.prefix()?;
    let docs = eidos_plugins::documents_my_games_dir(&prefix, &spec);
    let path = eidos_gamefeatures::se_log_path(game.def.id, &docs, &game.install_path)?;

    let Ok(raw) = fs::read(&path) else {
        return Some(Diagnostic {
            level: DiagLevel::Advice,
            title: t("diag_no_se_log").to_string(),
            detail: tf("diag_no_se_log_detail", &[("path", &path.display())]),
            actions: Vec::new(),
        });
    };
    // The extender writes cp1252, so a plugin name with an accent is not valid
    // UTF-8; lossy keeps the rest of the line readable rather than dropping it.
    let plugins = eidos_gamefeatures::parse_se_log(&String::from_utf8_lossy(&raw));
    if plugins.is_empty() {
        return None;
    }
    // The log is from the LAST run, which may predate the current load order, so
    // stamp it - an old log claiming success is the confusing case.
    let when = fs::metadata(&path)
        .and_then(|m| m.modified())
        .map(format_mtime)
        .unwrap_or_default();
    let failed: Vec<&eidos_gamefeatures::SePluginLoad> =
        plugins.iter().filter(|p| !p.loaded).collect();
    if failed.is_empty() {
        return Some(Diagnostic {
            level: DiagLevel::Ok,
            title: tf("diag_se_all_loaded", &[("n", &plugins.len())]),
            detail: tf("diag_se_log_written", &[("when", &when)]),
            actions: Vec::new(),
        });
    }
    let lines: Vec<String> = failed
        .iter()
        .take(10)
        .map(|p| failed_plugin_line(p))
        .collect();
    let more = failed.len().saturating_sub(lines.len());
    let tail = if more > 0 {
        format!("  {}", tf("diag_se_and_more", &[("n", &more)]))
    } else {
        String::new()
    };
    Some(Diagnostic {
        level: DiagLevel::Problem,
        title: tf(
            "diag_se_failed",
            &[("n", &failed.len()), ("total", &plugins.len())],
        ),
        detail: tf(
            "diag_se_failed_detail",
            &[
                ("lines", &lines.join("   ")),
                ("tail", &tail),
                ("when", &when),
            ],
        ),
        actions: Vec::new(),
    })
}

pub(crate) fn orphan_archive_diagnostics(app: &App, game_id: &str) -> Vec<Diagnostic> {
    let Some(rows) = cached_archive_rows(app, game_id) else {
        return Vec::new();
    };
    let orphan: Vec<_> = rows
        .iter()
        .filter(|r| !r.loaded() && r.overridden_by.is_none())
        .collect();
    if orphan.is_empty() {
        return Vec::new();
    }
    vec![Diagnostic {
        level: DiagLevel::Advice,
        title: tf("diag_orphan_archives", &[("n", &orphan.len())]),
        detail: tf(
            "diag_orphan_archives_detail",
            &[(
                "list",
                &orphan
                    .iter()
                    .take(8)
                    .map(|r| format!("{} ({})", r.archive, r.mod_name))
                    .collect::<Vec<_>>()
                    .join(", "),
            )],
        ),
        actions: Vec::new(),
    }]
}

/// The Diagnostics tab label, carrying the count of things needing attention.
pub(crate) fn diagnostics_tab_label(app: &App) -> String {
    let n = app
        .diag
        .iter()
        .filter(|d| d.level == DiagLevel::Problem)
        .count();
    if n > 0 {
        tf("diag_tab_n", &[("n", &n)])
    } else {
        t("diag_tab").to_string()
    }
}

pub(crate) fn diagnostics_panel<'a>(app: &App) -> Element<'a, Message> {
    // The same cache the tab label reads, so the count on the tab and the cards
    // in the panel can never tell two different stories.
    let checks = app.diag.clone();
    let problems = checks
        .iter()
        .filter(|d| d.level == DiagLevel::Problem)
        .count();
    let summary = if problems == 0 {
        t("diag_no_problems").to_string()
    } else {
        tf("diag_n_problems", &[("n", &problems)])
    };
    let mut col = Column::new()
        .spacing(8)
        .push(text(t("diag_tab")).size(sz(13)))
        .push(text(summary).size(sz(12)));
    for d in checks {
        let (tag, color) = match d.level {
            DiagLevel::Problem => (t("diag_tag_problem"), pal().error),
            DiagLevel::Advice => (t("diag_tag_advice"), pal().warning),
            DiagLevel::Ok => (t("diag_tag_ok"), pal().success),
        };
        let mut card = Column::new()
            .spacing(2)
            .push(
                Row::new()
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .push(
                        text(tag)
                            .size(sz(9))
                            .color(color)
                            .width(Length::Fixed(scaled(58.0))),
                    )
                    .push(text(d.title).size(sz(12)).width(Length::Fill)),
            )
            .push(text(d.detail).size(sz(10)).color(text_muted()));
        if !d.actions.is_empty() {
            let mut row = Row::new().spacing(6);
            for (label, msg) in d.actions {
                row = row.push(tool_btn(label, msg));
            }
            card = card.push(row);
        }
        col = col.push(
            container(card)
                .padding([4, 6])
                .width(Length::Fill)
                .style(card_style),
        );
    }
    scrollable(col).height(Length::Fill).into()
}

/// Whether this process is talking Wayland rather than X11.
///
/// The same test winit itself makes when it picks a backend: it uses Wayland when
/// `WAYLAND_DISPLAY` names a socket it can reach, and X11 otherwise. There is no
/// API to ask it afterwards, and the env var is what decided the answer - so
/// reading it here gives the same verdict rather than a guess about it.
///
/// It matters because winit 0.30 has no `wl_data_device` at all: on Wayland, the
/// file-drop events exist in the type system and never fire.
pub(crate) fn on_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}
