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
    pub(crate) actions: Vec<(String, Message)>,
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
        nxm: app.nxm_handler.clone(),
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
    if let Some((inst, why)) = app.created.as_ref().and_then(|i| {
        i.modlist_checked()
            .1
            .reason()
            .map(|why| (i, why.to_string()))
    }) {
        // The way out when the user deleted those mods on purpose. Never offered
        // while `mods/` cannot be read or holds no mod at all (`missing_mods`
        // errs): that is the unmounted drive, and forgetting would erase the
        // order it is waiting to come back to. Nor when the active list's own
        // mods are all there: then the verdict is about modlist.txt itself
        // (unreadable, emptied), which forgetting other profiles' lines cannot
        // fix, so the card points at Backups instead.
        let mut detail = tf("diag_modlist_mismatch_detail", &[("reason", &why)]);
        let actions = match inst.missing_mods() {
            Ok(missing)
                if inst
                    .active()
                    .listed_mods()
                    .iter()
                    .any(|n| missing.contains(n)) =>
            {
                detail.push(' ');
                detail.push_str(t("diag_modlist_mismatch_forget_hint"));
                vec![(
                    tf("diag_action_forget_missing", &[("n", &missing.len())]),
                    Message::ForgetMissingMods,
                )]
            }
            _ => {
                detail.push(' ');
                detail.push_str(t("diag_modlist_mismatch_backup_hint"));
                Vec::new()
            }
        };
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: t("diag_modlist_mismatch").to_string(),
            detail,
            actions,
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
                if selected_game(app).is_some_and(|g| g.def.id == "starfield") {
                    out.extend(
                        eidos::check::load_order::starfield_findings(list)
                            .iter()
                            .map(crate::health::finding_card),
                    );
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
                    actions: vec![(
                        t("diag_enable_missing_masters").into(),
                        Message::EnableMissingMasters,
                    )],
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

    if let Some(game) = selected_game(app) {
        let names: Vec<String> = plugins
            .map(|l| l.plugins.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();
        let premium = app.nexus_account.as_ref().is_some_and(|a| a.is_premium);
        out.extend(requirement_checks(
            game.def,
            &app.mods,
            &app.meta_cache,
            &names,
            premium,
        ));
    }
    if let (Some(list), Some(map)) = (plugins, app.conflicts.as_ref()) {
        out.extend(patch_order_checks(list, map));
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
        out.extend(
            eidos::check::host_findings(&home(), game, inst, &mods)
                .iter()
                .map(crate::health::finding_card),
        );
        for d in eidos_gamefeatures::preflight::scan_extender_plugins(
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
        for d in eidos_gamefeatures::preflight::scan_fontconfig(
            game.def,
            &game.data_path,
            &mods,
            &inst.overwrite_dir(),
        ) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: t("diag_fontconfig_missing").to_string(),
                detail: tf(
                    "diag_fontconfig_missing_detail",
                    &[("fonts", &d.detail), ("mod", &d.origin_mod)],
                ),
                actions: Vec::new(),
            });
        }
        for d in eidos_gamefeatures::preflight::scan_compressed_audio(
            game.def,
            &mods,
            &inst.overwrite_dir(),
        ) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: t("diag_ba2_audio").to_string(),
                detail: tf(
                    "diag_ba2_audio_detail",
                    &[("archive", &d.detail), ("mod", &d.origin_mod)],
                ),
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
    }

    // A non-empty Overwrite is generated content sitting outside any mod.
    if let Some(inst) = app.created.as_ref() {
        for m in app
            .mods
            .iter()
            .filter(|m| !m.unmanaged && !m.is_separator())
        {
            // The row cache holds it, and every install that writes it drops
            // that row afterwards, a failed one included (a stopped Merge
            // leaves its checkpoint warning), so this is not a second
            // `meta.ini` read per mod on every edit. A row not cached yet is
            // read from disk.
            let warning = match app.meta_cache.get(&m.name) {
                Some(row) => row.install_warning.clone(),
                None => inst.mod_meta(&m.name).install_warning(),
            };
            if let Some(warning) = warning {
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
        // Known plugins: only drifted outputs come back. Unknown: every output
        // in the game's view, each reported as unverified.
        let outputs = match active_plugins.as_ref() {
            Some(plugins) => stale_outputs(app, inst, plugins),
            None => inst.generated_outputs().map(|all| {
                all.into_iter()
                    .filter(|(owner, _)| output_owner_is_active(app, owner.as_deref()))
                    .collect()
            }),
        };
        match outputs {
            Ok(outputs) => {
                for (owner, output) in outputs {
                    if active_plugins.is_none() {
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
                    }
                    {
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
                            // Runs the tool as configured now, not the recorded preset.
                            actions: app
                                .tools
                                .iter()
                                .find(|x| x.title.eq_ignore_ascii_case(&output.receipt.tool))
                                .map(|x| {
                                    (
                                        tf("diag_action_run_tool_again", &[("tool", &x.title)]),
                                        Message::RunTool(x.title.clone()),
                                    )
                                })
                                .into_iter()
                                .collect(),
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
                actions: vec![(
                    t("diag_action_delete_them").into(),
                    Message::CleanInstallDebris,
                )],
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
                        t("diag_action_restore_presession").into(),
                        Message::RestorePreSessionPlugins,
                    ),
                    (t("diag_action_keep_set").into(), Message::AcceptPluginState),
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
        out.extend(
            crate::health::proton_facts(game, &home())
                .map(|f| crate::health::proton_checks(&f))
                .unwrap_or_default(),
        );
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
        if let (Some(se), Some(row), Some(map)) = (
            game.def.script_extender.as_ref(),
            eidos_gamefeatures::preflight::extender(game.def.id),
            app.conflicts.as_ref(),
        ) {
            let label = row.plugins_dir.split('/').next().unwrap_or_default();
            let in_game = game.install_path.join(se.loader).is_file();
            out.extend(extender_script_checks(map, se.loader, label, in_game));
        }
    }

    for (row, plugin) in stale_copies(app) {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: tf("diag_stale_copy", &[("plugin", &plugin)]),
            detail: tf(
                "diag_stale_copy_detail",
                &[("mod", &app.mods[row].name), ("plugin", &plugin)],
            ),
            actions: vec![(
                t("diag_hide_stale_copy").into(),
                Message::ToggleFileHidden(row, plugin),
            )],
        });
    }

    out
}

/// `shown` joined with ", ", then "and n more" when `total` is larger.
pub(crate) fn file_list(shown: &[String], total: usize) -> String {
    let mut s = shown.join(", ");
    if total > shown.len() {
        s.push_str(&tf("diag_and_n_more", &[("n", &(total - shown.len()))]));
    }
    s
}

/// Patches that sit below the mod they patch and so lose its copies of shared
/// loose files. Only loose files: `map.files` does not hold archive members.
pub(crate) fn patch_order_checks(
    list: &PluginList,
    map: &eidos_conflicts::ConflictMap,
) -> Vec<Diagnostic> {
    use eidos_conflicts::BASE_ORIGIN;
    let origin: HashMap<&str, u32> = map
        .names
        .iter()
        .filter(|(&o, _)| o != BASE_ORIGIN && o != u32::MAX)
        .map(|(&o, n)| (n.as_str(), o))
        .collect();
    let by_name: HashMap<String, &eidos_plugins::Plugin> = list
        .plugins
        .iter()
        .map(|p| (p.name.to_lowercase(), p))
        .collect();
    // (patch mod, master mod) -> (patch plugin, master plugin)
    let mut pairs: std::collections::BTreeMap<(String, String), (&str, &str)> =
        std::collections::BTreeMap::new();
    for p in list.plugins.iter().filter(|p| p.enabled) {
        let Some(&b) = origin.get(p.origin_mod.as_str()) else {
            continue;
        };
        for m in &p.masters {
            let Some(mp) = by_name.get(&m.to_lowercase()) else {
                continue;
            };
            let Some(&a) = origin.get(mp.origin_mod.as_str()) else {
                continue;
            };
            if a != b && map.mods.get(&a).is_some_and(|c| c.overwrites.contains(&b)) {
                pairs
                    .entry((p.origin_mod.clone(), mp.origin_mod.clone()))
                    .or_insert((p.name.as_str(), mp.name.as_str()));
            }
        }
    }
    if pairs.is_empty() {
        return Vec::new();
    }
    // Per pair: how many files, and the first 8 paths.
    let mut found: Vec<(usize, Vec<String>)> = vec![(0, Vec::new()); pairs.len()];
    let keys: Vec<(u32, u32)> = pairs
        .keys()
        .map(|(b, a)| (origin[b.as_str()], origin[a.as_str()]))
        .collect();
    for node in map.files.values().filter(|n| n.is_conflicted()) {
        let providers: Vec<u32> = std::iter::once(node.winner)
            .chain(node.alternatives.iter().copied())
            .collect();
        for (i, &(b, a)) in keys.iter().enumerate() {
            let pos = |o: u32| providers.iter().position(|&x| x == o);
            if let (Some(pa), Some(pb)) = (pos(a), pos(b)) {
                if pa < pb {
                    found[i].0 += 1;
                    if found[i].1.len() < 8 {
                        found[i].1.push(node.display_path.clone());
                    }
                }
            }
        }
    }
    pairs
        .into_iter()
        .zip(found)
        .filter(|(_, (n, _))| *n > 0)
        .map(
            |(((patch, master), (plugin, master_plugin)), (n, shown))| Diagnostic {
                level: DiagLevel::Advice,
                title: tf(
                    "diag_patch_loses",
                    &[("patch", &patch), ("master", &master), ("n", &n)],
                ),
                detail: tf(
                    "diag_patch_loses_detail",
                    &[
                        ("plugin", &plugin),
                        ("patch", &patch),
                        ("master_plugin", &master_plugin),
                        ("master", &master),
                        ("files", &file_list(&shown, n)),
                    ],
                ),
                actions: vec![(
                    tf(
                        "diag_action_move_below",
                        &[("mod", &patch), ("target", &master)],
                    ),
                    Message::MoveModBelow {
                        folder: patch,
                        below: master,
                    },
                )],
            },
        )
        .collect()
}

/// Mods whose loose Papyrus scripts beat the script extender's own copies.
/// `extender`: the label from `extender(id).plugins_dir` up to its first '/'.
/// `loader_in_game`: `game.install_path.join(loader).is_file()`, from the caller.
/// Loose `.pex` only: `map.files` does not hold archive members.
pub(crate) fn extender_script_checks(
    map: &eidos_conflicts::ConflictMap,
    loader: &str,
    extender: &str,
    loader_in_game: bool,
) -> Vec<Diagnostic> {
    use eidos_conflicts::BASE_ORIGIN;
    let e = match map.files.get(&format!("root/{}", loader.to_lowercase())) {
        Some(n) => n.winner,
        None if loader_in_game => BASE_ORIGIN,
        None => return Vec::new(),
    };
    // winner name -> (winner origin, count, first 8 paths)
    let mut found: std::collections::BTreeMap<String, (u32, usize, Vec<String>)> =
        std::collections::BTreeMap::new();
    let prefix = "scripts/".to_string();
    for (key, node) in map.files.range(prefix.clone()..) {
        if !key.starts_with(&prefix) {
            break;
        }
        if !key.ends_with(".pex") || node.winner == e || !node.alternatives.contains(&e) {
            continue;
        }
        let w = map.name(node.winner).to_string();
        let f = found.entry(w).or_insert((node.winner, 0, Vec::new()));
        f.1 += 1;
        if f.2.len() < 8 {
            f.2.push(node.display_path.clone());
        }
    }
    let is_mod = |o: u32| o != BASE_ORIGIN && o != u32::MAX;
    let e_name = map.name(e).to_string();
    found
        .into_iter()
        .map(|(w, (w_origin, n, shown))| {
            let files = file_list(&shown, n);
            let detail = if e == BASE_ORIGIN {
                tf(
                    "diag_extender_scripts_game_detail",
                    &[("mod", &w), ("extender", &extender), ("files", &files)],
                )
            } else {
                tf(
                    "diag_extender_scripts_detail",
                    &[
                        ("mod", &w),
                        ("extender_mod", &e_name),
                        ("extender", &extender),
                        ("files", &files),
                    ],
                )
            };
            let actions = if is_mod(e) && is_mod(w_origin) {
                vec![(
                    tf(
                        "diag_action_move_below",
                        &[("mod", &e_name), ("target", &w)],
                    ),
                    Message::MoveModBelow {
                        folder: e_name.clone(),
                        below: w.clone(),
                    },
                )]
            } else {
                Vec::new()
            };
            Diagnostic {
                level: DiagLevel::Advice,
                title: tf(
                    "diag_extender_scripts",
                    &[("mod", &w), ("n", &n), ("extender", &extender)],
                ),
                detail,
                actions,
            }
        })
        .collect()
}

/// Lowercased letters and digits only, so `Far Harbor` and `FarHarbor.esm` meet.
fn plain(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Mods that Nexus says an enabled mod needs and the list lacks, from the
/// requirements cached in each `meta.ini`. Reads only what it is given.
pub(crate) fn requirement_checks(
    game: &eidos_games::GameDef,
    mods: &[ModEntry],
    meta: &HashMap<String, RowMeta>,
    plugins: &[String],
    premium: bool,
) -> Vec<Diagnostic> {
    let stems: Vec<String> = plugins
        .iter()
        .map(|p| plain(p.rsplit_once('.').map_or(p.as_str(), |(stem, _)| stem)))
        .collect();
    // (domain, mod id) of every Nexus mod in the list, by its own id and the
    // ids of the files merged into it -> whether one copy is enabled. Built
    // once, so a requirement is a lookup rather than a scan of the list.
    let mut present: HashMap<(&str, u64), bool> = HashMap::new();
    for o in mods {
        let Some(om) = meta.get(&o.name) else {
            continue;
        };
        let Some(domain) = game.nexus_domain_for(om.game_name.as_deref()) else {
            continue;
        };
        let ids = om.mod_id.into_iter();
        for id in ids.chain(om.installed_files.iter().map(|(id, _)| *id)) {
            *present.entry((domain, id)).or_default() |= o.is_active();
        }
    }
    let mut out = Vec::new();
    for m in mods.iter().filter(|m| m.is_active() && !m.is_unmanaged()) {
        let Some(row) = meta.get(&m.name) else {
            continue;
        };
        let Some(reqs) = row.requirements.as_ref() else {
            continue;
        };
        let who = m.display_name();
        let ignore = |key: String| {
            (
                t("diag_requirement_ignore").to_string(),
                Message::IgnoreRequirement {
                    folder: m.name.clone(),
                    key,
                },
            )
        };
        for r in &reqs.mods {
            if r.mod_id == 0 {
                // Hosted outside Nexus: Eidos cannot see whether it is installed.
                let key = format!("offsite:{}", r.name);
                if row.ignored_requirements.contains(&key) {
                    continue;
                }
                let required = if r.name.is_empty() {
                    t("diag_requirement_offsite_unnamed").to_string()
                } else {
                    r.name.clone()
                };
                let args: [(&str, &dyn std::fmt::Display); 2] =
                    [("mod", &who), ("required", &required)];
                out.push(Diagnostic {
                    level: DiagLevel::Advice,
                    title: tf("diag_requirement_offsite", &args),
                    detail: tf("diag_requirement_offsite_detail", &args),
                    actions: vec![ignore(key)],
                });
                continue;
            }
            let key = format!("{}:{}", r.domain, r.mod_id);
            if row.ignored_requirements.contains(&key) {
                continue;
            }
            let found = present.get(&(r.domain.as_str(), r.mod_id)).copied();
            if found == Some(true) {
                continue;
            }
            let required = if r.name.is_empty() {
                tf("diag_requirement_unnamed", &[("id", &r.mod_id)])
            } else {
                r.name.clone()
            };
            let args: [(&str, &dyn std::fmt::Display); 2] =
                [("mod", &who), ("required", &required)];
            let missing = found.is_none();
            let mut actions = vec![(
                t("diag_requirement_open").to_string(),
                Message::OpenUrl(format!(
                    "https://www.nexusmods.com/{}/mods/{}",
                    r.domain, r.mod_id
                )),
            )];
            // Only a premium account can fetch; a disabled copy needs enabling.
            if premium && missing {
                actions.push((
                    t("diag_requirement_download").to_string(),
                    Message::RequirementDownload {
                        domain: r.domain.clone(),
                        mod_id: r.mod_id,
                    },
                ));
            }
            actions.push(ignore(key));
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: tf(
                    if missing {
                        "diag_requirement_missing"
                    } else {
                        "diag_requirement_disabled"
                    },
                    &args,
                ),
                detail: tf("diag_requirement_detail", &args),
                actions,
            });
        }
        for d in &reqs.dlc {
            // The entry reads `Name` or `Name: notes`.
            let name = d.split(':').next().unwrap_or(d).trim();
            let key = format!("dlc:{name}");
            if row.ignored_requirements.contains(&key) || stems.contains(&plain(name)) {
                continue;
            }
            let args: [(&str, &dyn std::fmt::Display); 2] = [("mod", &who), ("dlc", &name)];
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: tf("diag_requirement_dlc", &args),
                detail: tf("diag_requirement_dlc_detail", &args),
                actions: vec![ignore(key)],
            });
        }
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

/// Generated plugin copies whose source file changed since the copy was made:
/// `(row of the mod holding the copy, plugin name)`. The source is the file of
/// that name in the nearest enabled mod below the copy, else the game's Data
/// folder. Only stats, no reads.
pub(crate) fn stale_copies(app: &App) -> Vec<(usize, String)> {
    let data = selected_game(app).map(|g| &g.data_path);
    let mut out = Vec::new();
    for (row, m) in app.mods.iter().enumerate().filter(|(_, m)| m.enabled) {
        let Some(sources) = app.meta_cache.get(&m.name).map(|r| &r.plugin_sources) else {
            continue;
        };
        for src in sources {
            // Hidden, deleted or moved copies are no longer live: nothing to warn about.
            if !m.path.join(&src.name).exists() {
                continue;
            }
            let now = app.mods[..row]
                .iter()
                .rev()
                .filter(|b| b.enabled)
                .map(|b| b.path.join(&src.name))
                .chain(data.map(|d| d.join(&src.name)))
                .find(|p| p.exists());
            let current = now.and_then(|p| eidos_instance::PluginSource::of(&src.name, &p));
            if current.as_ref() != Some(src) {
                out.push((row, src.name.clone()));
            }
        }
    }
    out
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

/// Plain text for a bug report. Every line goes through `eidos_log::redact`
/// (home path, user name), because titles and Proton paths can carry both.
pub(crate) fn health_report(f: &eidos::check::ReportFacts, checks: &[Diagnostic]) -> String {
    let mut lines = eidos::check::report_header(f);
    lines.push(String::new());
    lines.push("Health:".to_string());
    for d in checks {
        let state = match d.level {
            DiagLevel::Problem => "problem",
            DiagLevel::Advice => "advice",
            DiagLevel::Ok => "ok",
        };
        lines.push(format!("- [{state}] {}", d.title));
    }
    lines
        .iter()
        .map(|l| eidos_log::redact(l).into_owned())
        .collect::<Vec<_>>()
        .join("\n")
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
        .push(text(summary).size(sz(12)))
        .push(
            Row::new()
                .spacing(6)
                .push(tool_btn(t("diag_copy_report"), Message::CopyHealthReport))
                .push(tool_btn(t("diag_bisect"), Message::OpenBisect)),
        );
    // Built from App state at each view, so its slot follows a clean as it runs.
    if let Some(card) = dirty_card(app) {
        col = col.push(dirty_plugins_card(&card));
    }
    for d in checks {
        let foot = (!d.actions.is_empty()).then(|| {
            d.actions
                .into_iter()
                .fold(Row::new().spacing(6), |row, (label, msg)| {
                    row.push(tool_btn(label, msg))
                })
                .into()
        });
        col = col.push(diag_card(d.level, d.title, d.detail, foot));
    }
    scrollable(col).height(Length::Fill).into()
}

/// One Health card: the level tag, the title, the detail, and what goes under it.
fn diag_card<'a>(
    level: DiagLevel,
    title: String,
    detail: String,
    foot: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let (tag, color) = match level {
        DiagLevel::Problem => (t("diag_tag_problem"), pal().error),
        DiagLevel::Advice => (t("diag_tag_advice"), pal().warning),
        DiagLevel::Ok => (t("diag_tag_ok"), pal().success),
    };
    let card = Column::new()
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
                .push(text(title).size(sz(12)).width(Length::Fill)),
        )
        .push(text(detail).size(sz(10)).color(text_muted()))
        .extend(foot);
    container(card)
        .padding([4, 6])
        .width(Length::Fill)
        .style(card_style)
        .into()
}

/// Whether the game has both an xEdit with a QuickAutoClean twin and LOOT
/// dirty data, the games the dirty-plugin card is for.
pub(crate) fn has_dirty_card(def: &eidos_games::GameDef) -> bool {
    def.known_tools
        .iter()
        .any(|(exe, _)| exe.to_ascii_lowercase().ends_with("quickautoclean.exe"))
}

/// What the dirty-plugin card's one slot holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirtySlot {
    /// "Clean all n": Eidos found an xEdit.
    Clean(usize),
    /// No xEdit found: the line saying to add one in Executables.
    AddXedit,
    /// A clean runs or waits: the plugin and how many more are queued.
    Cleaning { plugin: String, left: usize },
}

/// The enabled plugins LOOT flags as dirty, in load order, and the card's slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirtyCard {
    pub(crate) names: Vec<String>,
    pub(crate) slot: DirtySlot,
}

/// The dirty-plugin card for the open setup, from App state only (no IO), so
/// the view can call it. `None` for a game without one, or when no enabled
/// plugin is flagged (or no LOOT sort ran) and no clean runs or waits.
pub(crate) fn dirty_card(app: &App) -> Option<DirtyCard> {
    if !selected_game(app).is_some_and(|g| has_dirty_card(g.def)) {
        return None;
    }
    let mut names: Vec<String> = app
        .loot_meta
        .as_ref()
        .zip(app.plugins.as_ref())
        .map(|(meta, list)| {
            list.plugins
                .iter()
                // Not from the Overwrite: xEdit would save over that file itself.
                .filter(|p| p.enabled && p.origin_mod != "overwrite")
                .filter(|p| {
                    meta.get(&p.name.to_ascii_lowercase())
                        .is_some_and(|b| b.needs_cleaning())
                })
                .map(|p| p.name.clone())
                .collect()
        })
        .unwrap_or_default();
    if names.is_empty() {
        // `finish_run` drops the LOOT verdicts after every run, so a queue that
        // goes on keeps its card, and its slot, from the plugins left to clean.
        names = app
            .cleaning
            .iter()
            .map(|c| c.plugin.clone())
            .chain(app.clean_queue.iter().cloned())
            .collect();
    }
    if names.is_empty() {
        return None;
    }
    let slot = match (&app.cleaning, app.clean_queue.front()) {
        (Some(c), _) => DirtySlot::Cleaning {
            plugin: c.plugin.clone(),
            left: app.clean_queue.len(),
        },
        // Queued behind another run: the next one, and the rest after it.
        (None, Some(next)) => DirtySlot::Cleaning {
            plugin: next.clone(),
            left: app.clean_queue.len() - 1,
        },
        (None, None) if qac_tool(app).is_some() => DirtySlot::Clean(names.len()),
        (None, None) => DirtySlot::AddXedit,
    };
    Some(DirtyCard { names, slot })
}

/// The laid-out height of one `tool_btn` at scale 1, the dirty card's slot.
pub(crate) const DIRTY_SLOT_H: f32 = 28.0;

/// The dirty-plugin card. Its slot always exists at one fixed height and holds
/// the button, the add-xEdit line or the cleaning status, so nothing below it
/// moves under the pointer when a clean starts or ends (the #120 pattern).
pub(crate) fn dirty_plugins_card<'a>(card: &DirtyCard) -> Element<'a, Message> {
    let mut list = card
        .names
        .iter()
        .take(8)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if card.names.len() > 8 {
        list.push_str(&tf("diag_and_n_more", &[("n", &(card.names.len() - 8))]));
    }
    let line = |s: String| -> Element<'a, Message> {
        text(s)
            .size(sz(10))
            .wrapping(iced::widget::text::Wrapping::None)
            .into()
    };
    let slot = match &card.slot {
        DirtySlot::Clean(n) => tool_btn(
            tf("diag_clean_all", &[("n", n)]),
            Message::CleanPlugins(card.names.clone()),
        ),
        DirtySlot::AddXedit => line(t("diag_dirty_plugins_add_xedit").into()),
        DirtySlot::Cleaning { plugin, left } => line(tf(
            "diag_dirty_plugins_cleaning",
            &[("plugin", plugin), ("left", left)],
        )),
    };
    diag_card(
        DiagLevel::Advice,
        tf("diag_dirty_plugins", &[("n", &card.names.len())]),
        tf("diag_dirty_plugins_detail", &[("list", &list)]),
        Some(
            container(slot)
                .height(Length::Fixed(scaled(DIRTY_SLOT_H)))
                .clip(true)
                .into(),
        ),
    )
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

/// Overwrite, or a mod that is enabled: an output of a disabled mod is not in the
/// game's view, so it is neither checked nor asked about.
///
/// A cleaned copy is skipped too: its inputs are its one source plugin, not the
/// whole list this receipt records, and rerunning the tool without its `-qac`
/// arguments would clean nothing.
fn output_owner_is_active(app: &App, owner: Option<&str>) -> bool {
    owner.is_none_or(|owner| {
        !owner.eq_ignore_ascii_case(CLEANED_MOD)
            && app
                .mods
                .iter()
                .any(|m| m.is_active() && m.name.eq_ignore_ascii_case(owner))
    })
}

/// Generated outputs in the game's view whose recorded inputs no longer match,
/// as the Diagnostics card and the play prompt both need them.
pub(crate) fn stale_outputs(
    app: &App,
    inst: &Instance,
    plugins: &[(String, bool)],
) -> std::io::Result<Vec<(Option<String>, eidos_instance::GeneratedOutput)>> {
    Ok(inst
        .generated_outputs()?
        .into_iter()
        .filter(|(owner, _)| output_owner_is_active(app, owner.as_deref()))
        .filter(|(_, output)| inst.generated_input_drift(&output.receipt, plugins))
        .collect())
}

/// The first flagged generator whose output is out of date and not yet waved
/// through for this exact input state. Unknown plugins give `None`: the Health
/// card already says the inputs are unverified.
pub(crate) fn play_prompt_for(app: &App) -> Option<PlayPrompt> {
    let inst = app.created.as_ref()?;
    if !app.tools.iter().any(|t| t.check_before_play) {
        return None;
    }
    let computed;
    let list = match app.plugins.as_ref() {
        Some(list) => Some(list),
        None => {
            computed = compute_plugins(app);
            computed.as_ref()
        }
    };
    let plugins = match list {
        Some(list) => list
            .plugins
            .iter()
            .map(|p| (p.name.clone(), p.enabled))
            .collect::<Vec<_>>(),
        None if !game_has_plugins(app) => Vec::new(),
        None => return None,
    };
    let state = inst.generated_input_state(&plugins);
    stale_outputs(app, inst, &plugins)
        .unwrap_or_default()
        .into_iter()
        .find_map(|(_, output)| {
            let tool = app.tools.iter().find(|t| {
                t.check_before_play && t.title.eq_ignore_ascii_case(&output.receipt.tool)
            })?;
            let key = (output.receipt.id, state);
            (!app.play_anyway.contains(&key)).then(|| PlayPrompt {
                tool: tool.title.clone(),
                key,
            })
        })
}
