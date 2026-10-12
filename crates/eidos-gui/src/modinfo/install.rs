//! Installing a mod: the picker, BAIN and collision dialogs, and what runs
//! after an install.

use super::*;

/// Installer decisions use fresh persisted profile state, even before opening Plugins.
pub(crate) fn current_fomod_context(app: &App) -> eidos_fomod::Context {
    let (Some(inst), Some(game)) = (app.created.as_ref(), selected_game(app)) else {
        return eidos_fomod::Context::default();
    };
    let fallback = game.plugin_state_dir();
    eidos_install::fomod_context_for_instance(
        inst,
        &game.data_path,
        game.def.id,
        fallback.as_deref(),
    )
}

/// Retry the pending collision install under `policy`. Reuses the same discovery as
/// a normal install (rebuilds the FOMOD context in case the archive turns out to be
/// a FOMOD). A Rename that collides again re-opens the prompt.
/// The extracted archive parsed once for the picker: the extraction does not
/// change while the dialog is on screen, and re-walking it on every redraw would
/// stutter a large pack - which is also why the picker keeps the TREE and not
/// just its rows: the Manual mode's validity label needs it too.
pub(crate) fn parsed_tree(tree: &eidos_install::ExtractedTree) -> eidos_install::ArchiveTree {
    eidos_install::ArchiveTree::from_dir(tree.path()).unwrap_or_default()
}

/// Install what the manual / BAIN picker currently has selected.
///
/// A name collision hands off to the existing Merge / Replace / Rename prompt,
/// carrying the picks so resolving it does not re-ask them. On any other failure
/// the picker stays open with the reason, so a bad data root can just be
/// re-picked instead of re-extracting the archive.
pub(crate) fn run_picker_install(app: &mut App) {
    let Some(p) = app.picker.as_ref() else { return };
    let _guard = match crate::update::lock_install_target(app, &p.target) {
        Ok(guard) => guard,
        Err(error) => {
            app.status = Some(error);
            return;
        }
    };
    let mods_dir = app
        .created
        .as_ref()
        .expect("validated install target")
        .mods_dir();
    let name = p.name.trim().to_string();
    if name.is_empty() {
        app.status = Some(t("status_mod_needs_name").to_string());
        return;
    }
    let choice = match &p.mode {
        PickerMode::Bain {
            subpackages,
            picked,
            ..
        } => {
            let chosen: Vec<String> = subpackages
                .iter()
                .zip(picked)
                .filter(|(_, &on)| on)
                .map(|(s, _)| s.clone())
                .collect();
            if chosen.is_empty() {
                app.status = Some(t("status_tick_subpackage").to_string());
                return;
            }
            PickerChoice::Bain(chosen)
        }
        PickerMode::Manual { root } => PickerChoice::Manual(root.clone()),
    };
    let result = install_with_choice(
        &p.tree,
        &choice,
        &p.archive,
        &mods_dir,
        &name,
        &p.game_id,
        eidos_install::OverwritePolicy::Fail,
    );
    match result {
        Ok(r) => {
            let archive = p.archive.clone();
            // A successful install may have consumed the tree (a lone source is
            // moved, not copied), so the picker must go before anything else.
            app.picker = None;
            remember_bain_options(app, &r.name, &choice);
            after_install(app, &r.name, r.dest, r.fomod, Some(&archive));
        }
        Err(eidos_install::InstallError::Exists(_)) => {
            let Some(p) = app.picker.take() else { return };
            let rename_to = suggest_free_name(&mods_dir, &name);
            app.collision = Some(CollisionPrompt {
                target: p.target,
                backup: app.prefs.retain_install_backup,
                archive: p.archive,
                name: name.clone(),
                game_id: p.game_id,
                rename_to,
                fomod: false,
                tree: Some(p.tree),
                pick: Some(choice),
            });
            app.status = Some(tf("status_already_exists_choose", &[("name", &name)]));
        }
        Err(e) => app.status = Some(tf("status_install_failed", &[("error", &e)])),
    }
}

/// Dispatch one picker choice to the matching installer.
pub(crate) fn install_with_choice(
    tree: &eidos_install::ExtractedTree,
    choice: &PickerChoice,
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: eidos_install::OverwritePolicy,
) -> Result<eidos_install::InstallReport, eidos_install::InstallError> {
    match choice {
        PickerChoice::Bain(subs) => {
            eidos_install::install_bain(tree, subs, archive, mods_dir, name, game_id, policy)
        }
        PickerChoice::Manual(root) => {
            eidos_install::install_manual(tree, root, archive, mods_dir, name, game_id, policy)
        }
    }
}

/// Record a BAIN selection in the installed mod's `meta.ini`, so reinstalling it
/// later opens the picker with the same sub-packages already ticked (MO2's
/// `onInstallationEnd`). Best-effort: failing to remember a preference must not
/// look like a failed install.
pub(crate) fn remember_bain_options(app: &App, mod_name: &str, choice: &PickerChoice) {
    let (PickerChoice::Bain(subs), Some(inst)) = (choice, app.created.as_ref()) else {
        return;
    };
    if let Err(error) = edit_mod_meta(inst, mod_name, |meta| meta.set_bain_options(subs)) {
        eidos_log::warn!("Could not remember BAIN choices for {mod_name}: {error}");
    }
}

pub(crate) fn after_install_report(
    app: &mut App,
    report: eidos_install::InstallReport,
    archive: &std::path::Path,
) {
    after_install(app, &report.name, report.dest, report.fomod, Some(archive));
    if let Some(backup) = report.backup {
        let status = app.status.get_or_insert_with(String::new);
        status.push(' ');
        status.push_str(&tf("status_backup_retained", &[("path", &backup.display())]));
    }
}

pub(crate) fn run_collision_install(app: &mut App, policy: eidos_install::OverwritePolicy) {
    let Some(c) = app.collision.take() else {
        return;
    };
    let policy = match (c.backup, policy) {
        (true, eidos_install::OverwritePolicy::Merge) => {
            eidos_install::OverwritePolicy::MergeWithBackup
        }
        (true, eidos_install::OverwritePolicy::Replace) => {
            eidos_install::OverwritePolicy::ReplaceWithBackup
        }
        (_, policy) => policy,
    };
    let _lock = match crate::update::lock_install_target(app, &c.target) {
        Ok(lock) => lock,
        Err(error) => {
            app.status = Some(error);
            app.collision = Some(c);
            return;
        }
    };
    // A FOMOD reinstall: the wizard (with the user's choices) is still open in
    // app.fomod - resolve through finish_fomod, never by re-extracting with
    // default selections.
    if c.fomod {
        let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) else {
            return;
        };
        // A Rename onto another existing mod re-opens the prompt BEFORE the
        // session is consumed (its drop would delete the extracted tree).
        if let eidos_install::OverwritePolicy::Rename(new) = &policy {
            if eidos_install::collision_name(&mods_dir, new).is_some() {
                app.status = Some(t("status_name_taken").to_string());
                app.collision = Some(c);
                return;
            }
        }
        let Some(w) = app.fomod.take() else { return };
        let archive = w.archive.clone();
        match eidos_install::finish_fomod(
            w.session,
            &w.selection,
            &mods_dir,
            &w.game_id,
            &w.ctx,
            policy,
        ) {
            Ok(r) => after_install_report(app, r, &archive),
            Err(e) => app.status = Some(tf("status_install_failed", &[("error", &e)])),
        }
        return;
    }
    let Some(inst) = app.created.as_ref() else {
        app.status = Some(t("status_open_instance_first").to_string());
        return;
    };
    let mods_dir = inst.mods_dir();
    let ctx = current_fomod_context(app);
    let archive = c.archive.clone();
    // A collision raised by the manual / BAIN picker: replay the SAME picks. The
    // tree alone does not say which sub-packages were ticked, so re-running the
    // plain installer here would quietly install something else.
    if let (Some(choice), Some(tree)) = (c.pick.as_ref(), c.tree.as_ref()) {
        match install_with_choice(
            tree, choice, &c.archive, &mods_dir, &c.name, &c.game_id, policy,
        ) {
            Ok(r) => {
                remember_bain_options(app, &r.name, choice);
                after_install_report(app, r, &archive);
            }
            Err(eidos_install::InstallError::Exists(_)) => {
                app.status = Some(t("status_name_taken").to_string());
                app.collision = Some(c);
            }
            Err(e) => app.status = Some(tf("status_install_failed", &[("error", &e)])),
        }
        return;
    }
    // Reuse the tree extracted when the collision was raised; only fall back to a
    // fresh extraction if it is gone.
    let result = match c.tree.as_ref() {
        Some(tree) => eidos_install::install_extracted(
            tree, &c.archive, &mods_dir, &c.name, &c.game_id, policy, &ctx,
        ),
        None => eidos_install::install_archive_with_policy(
            &c.archive, &mods_dir, &c.name, &c.game_id, policy, &ctx,
        ),
    };
    match result {
        Ok(r) => after_install_report(app, r, &archive),
        Err(eidos_install::InstallError::Exists(_)) => {
            // A Rename target that also exists: keep the prompt open for another try.
            app.status = Some(t("status_name_taken").to_string());
            app.collision = Some(c);
        }
        Err(e) => app.status = Some(tf("status_install_failed", &[("error", &e)])),
    }
}

/// Recursively copy the CONTENTS of `src` into `dst` (creating `dst`), MO2's
/// "Install from folder": the new mod folder mirrors the chosen directory's root
/// rather than nesting the directory inside itself.
pub(crate) fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Register new mods at highest priority, preserve existing profile decisions,
/// and refresh the affected list, plugin and conflict caches.
pub(crate) fn after_install(
    app: &mut App,
    name: &str,
    dest: PathBuf,
    fomod: bool,
    archive: Option<&Path>,
) {
    let mut registration_warning = None;
    if let Some(inst) = &app.created {
        // Same lock as save_mods: the modlist must not be rewritten under a
        // running session. A refusal is not a lost install - the files are on
        // disk, reconciliation lists the folder on the next reload, and only the
        // auto-enable is skipped.
        match inst.try_lock("the Eidos window") {
            Ok(_lock) => {
                if let Err(error) = inst.register_installed_mod(name) {
                    app.status = Some(tf(
                        "status_installed_unregistered",
                        &[("name", &name), ("error", &error)],
                    ));
                    return;
                }
            }
            Err(e) => {
                registration_warning = Some(tf("status_register_later", &[("error", &e)]));
            }
        }
    }
    reload_mods(app);
    // Flip the source archive's `.meta` status to installed (MO2 marks the
    // download), so the Downloads manager shows it as installed. Best-effort: a
    // manually dropped archive with no sidecar is a no-op.
    if let Some(a) = archive {
        let _ = eidos_nexus::mark_installed(a);
    }
    // The installed mod's tree changed (and a FOMOD may have replaced it wholesale).
    drop_files_cache(app, Some(name));
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    // The reinstalled mod's meta.ini was just rewritten, and
    // `refresh_meta_cache` only computes rows it does not already hold - so
    // without dropping this one the list keeps showing the OLD version, colour
    // and note for a mod that has just changed underneath it.
    invalidate_meta(app, name);
    refresh_meta_cache(app);
    // Refresh the cached downloads only if they were already loaded, so the
    // status column reflects the new install without a full re-scan otherwise.
    if !app.downloads.is_empty() {
        load_downloads(app);
    }
    let mut where_to = String::new();
    let mut move_refused = None;
    // A drop aimed at a gap says WHERE, not just whether. Consumed here, after
    // `reload_mods`, because that is when the new row exists to be moved - and
    // this is MO2's own ordering too (install first, reposition after).
    // Only when this install is the one that was aimed. An aim left behind by an
    // install that failed or was cancelled must never move an unrelated mod.
    let aimed = app
        .install_at
        .as_ref()
        .filter(|(_, want)| archive.is_some_and(|a| a == want.as_path()))
        .map(|(gap, _)| *gap);
    if aimed.is_some() {
        app.install_at = None;
    }
    if let Some(dest) = aimed {
        if let Some(at) = app.mods.iter().position(|m| m.name == name) {
            let dest = dest.min(app.mods.len());
            let hidden = hidden_by_folds(app);
            let landed = move_block(&mut app.mods, &[at], dest);
            settle_folds_after_move(app, landed, 1, &hidden);
            if mods_changed(app) {
                app.selected_mod = Some(landed);
                where_to = format!(" {}", tf("status_at_priority", &[("n", &landed)]));
            } else {
                // The move was reverted; its reason rides along below instead
                // of being replaced by "at priority N".
                move_refused = app.status.take();
            }
        }
    }
    // A note the drop left for the installer to deliver - it could not say it
    // itself, because this line would have overwritten it.
    let note = app
        .pending_note
        .take()
        .map(|n| format!(" ({n})"))
        .unwrap_or_default();
    let warning = eidos_instance::ModMeta::read(&dest.join("meta.ini")).install_warning();
    app.status = Some(if fomod {
        tf(
            "status_installed_fomod",
            &[("name", &name), ("where", &where_to), ("note", &note)],
        )
    } else {
        tf(
            "status_installed",
            &[("name", &name), ("where", &where_to), ("note", &note)],
        )
    });
    for warning in [warning, registration_warning, move_refused].into_iter().flatten() {
        if let Some(status) = &mut app.status {
            status.push(' ');
            status.push_str(&tf("status_warning", &[("warning", &warning)]));
        }
    }
}

/// The install-collision chooser card (MO2's QueryOverwriteDialog): Merge / Replace
/// / Rename / Cancel for an already-existing `mods/<name>/`.
/// The extraction dialog: what is being unpacked, and how far along.
///
/// MO2 shows one for every archive it opens, and its absence here was not a
/// missing widget but a missing thread - nothing could report progress while
/// the extraction held the window's own event loop. It reports 7-Zip's own
/// percentage, and says so plainly at 100%, when the remaining work (finding
/// the data root, healing NTFS case collisions) no longer moves the bar.
pub(crate) fn install_progress_dialog<'a>(job: &crate::InstallJob) -> Element<'a, Message> {
    let pct = job
        .percent
        .load(std::sync::atomic::Ordering::SeqCst)
        .min(100);
    let card = Column::new()
        .spacing(10)
        .push(text(tf("install_extracting", &[("name", &job.name)])).size(sz(15)))
        .push(
            iced::widget::progress_bar(0.0..=100.0, pct as f32)
                .length(Length::Fill)
                .girth(Length::Fixed(10.0)),
        )
        .push(
            text(if pct >= 100 {
                t("transfer_finishing").to_string()
            } else {
                format!("{pct}%")
            })
            .size(sz(12)),
        )
        .push(
            text(t("install_large_archives"))
                .size(sz(10))
                .color(text_muted()),
        );
    container(card)
        .max_width(420.0)
        .padding(16)
        .style(card_style)
        .into()
}

pub(crate) fn collision_dialog<'a>(c: &CollisionPrompt) -> Element<'a, Message> {
    let buttons = Row::new()
        .spacing(8)
        .push(
            button(text(t("install_merge")).size(sz(12)))
                .padding([4, 10])
                .on_press(Message::CollisionMerge)
                .style(button::secondary),
        )
        .push(
            button(text(t("install_replace")).size(sz(12)))
                .padding([4, 10])
                .on_press(Message::CollisionReplace)
                .style(button::danger),
        );
    let rename = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text(t("install_rename_label")).size(sz(12)))
        .push(
            text_input(t("info_new_name"), &c.rename_to)
                .on_input(Message::CollisionRenameChanged)
                .on_submit(Message::CollisionRenameCommit)
                .padding(5)
                .size(sz(12))
                .width(Length::Fill),
        )
        .push(
            button(text(t("install_install")).size(sz(12)))
                .padding([4, 10])
                .on_press(Message::CollisionRenameCommit)
                .style(button::primary),
        );
    let card = Column::new()
        .spacing(10)
        .push(text(tf("install_exists_title", &[("name", &c.name)])).size(sz(15)))
        .push(text(t("install_exists_body")).size(sz(12)))
        .push(checkbox(c.backup).label(t("install_keep_backup"))
            .on_toggle(Message::CollisionBackupChanged).text_size(sz(12)))
        .push(buttons)
        .push(
            text(t("install_merge_replace_note"))
                .size(sz(10)),
        )
        .push(rename)
        .push(
            button(text(t("common_cancel")).size(sz(12)))
                .padding([4, 10])
                .on_press(Message::CollisionCancel)
                .style(button::text),
        );
    container(card)
        .max_width(460.0)
        .padding(16)
        .style(card_style)
        .into()
}

/// How many tree rows the manual picker draws. An archive with more entries than
/// this is one whose data root is a top-level folder anyway.
pub(crate) const PICKER_TREE_ROWS: usize = 1500;

/// The manual / BAIN install picker: MO2's `InstallDialog` (point at the data
/// root) and `BainComplexInstallerDialog` (tick sub-packages), which share an
/// When nothing in an archive looks like a mod, whether the archive is really a
/// bundle of OTHER archives - the variant packs that ship two `.zip` options and
/// a folder of screenshots.
///
/// Neither level of such an archive can ever look valid, so the dialog otherwise
/// just repeats "does NOT look valid" wherever the user clicks, with no hint that
/// the answer is to open one of the inner files instead.
pub(crate) fn nested_archive_hint(rows: &[eidos_install::TreeRow]) -> Option<String> {
    let n = rows
        .iter()
        .filter(|r| !r.is_dir)
        .filter(|r| {
            r.name.rsplit_once('.').is_some_and(|(_, e)| {
                matches!(e.to_ascii_lowercase().as_str(), "7z" | "zip" | "rar")
            })
        })
        .count();
    match n {
        0 => None,
        1 => Some(
            t("install_nested_one").to_string(),
        ),
        n => Some(tf("install_nested_many", &[("n", &n)])),
    }
}

/// archive tree and a name field.
pub(crate) fn install_picker_dialog<'a>(p: &InstallPicker) -> Element<'a, Message> {
    let name_row = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text(t("install_as")).size(sz(12)))
        .push(
            text_input(t("install_mod_name"), &p.name)
                .on_input(Message::PickerNameChanged)
                .on_submit(Message::PickerInstall)
                .padding(5)
                .size(sz(12))
                .width(Length::Fill),
        );

    let (title, body): (String, Element<'a, Message>) = match &p.mode {
        // MO2 asks before assuming: an archive whose top level mixes sub-packages
        // with other folders is as likely to be a plain mod with extras.
        PickerMode::Bain {
            asking: true,
            subpackages,
            ..
        } => (
            t("install_maybe_bain").to_string(),
            Column::new()
                .spacing(10)
                .push(
                    text(tf("install_maybe_bain_body", &[("n", &subpackages.len())]))
                    .size(sz(12)),
                )
                .push(
                    Row::new()
                        .spacing(8)
                        .push(
                            button(text(t("install_bain_yes")).size(sz(12)))
                                .padding([4, 10])
                                .on_press(Message::PickerBainConfirm(true))
                                .style(button::primary),
                        )
                        .push(
                            button(text(t("install_bain_no")).size(sz(12)))
                                .padding([4, 10])
                                .on_press(Message::PickerBainConfirm(false))
                                .style(button::secondary),
                        ),
                )
                .into(),
        ),
        PickerMode::Bain {
            subpackages,
            picked,
            ..
        } => {
            // No spacing: the insertion strips below provide the separation, and they
            // must be part of the flow so the layout is identical with and without a drag.
            let mut list = Column::new();
            for (i, (name, &on)) in subpackages.iter().zip(picked).enumerate() {
                list = list.push(
                    checkbox(on)
                        .label(name.clone())
                        .on_toggle(move |_| Message::PickerBainToggle(i))
                        .size(sz(13))
                        .text_size(sz(12)),
                );
            }
            (
                t("install_choose_subpackages").to_string(),
                Column::new()
                    .spacing(8)
                    .push(
                        text(t("install_subpackages_note"))
                            .size(sz(11)),
                    )
                    .push(scrollable(list).height(Length::Fixed(240.0)))
                    .into(),
            )
        }
        PickerMode::Manual { root } => {
            // Like MO2's live green/red label - but against the tree parsed at
            // construction, never a re-walk of the extraction inside view().
            let rules = eidos_install::LayoutRules::for_game(&p.game_id);
            let valid = p.archive_tree.root_looks_valid(root, rules);
            let chosen = if root.is_empty() {
                t("install_archive_root")
            } else {
                root.as_str()
            };

            let mut list = Column::new().spacing(1).push(
                button(text(t("install_archive_root")).size(sz(12)))
                    .padding([1, 4])
                    .on_press(Message::PickerSetRoot(String::new()))
                    .style(if root.is_empty() {
                        button::primary
                    } else {
                        button::text
                    }),
            );
            for r in p.rows.iter().filter(|r| r.is_dir).take(PICKER_TREE_ROWS) {
                let selected = *root == r.path;
                let label = format!("{}{}", "    ".repeat(r.depth + 1), r.name);
                list = list.push(
                    button(text(label).size(sz(12)))
                        .padding([1, 4])
                        .on_press(Message::PickerSetRoot(r.path.clone()))
                        .style(if selected {
                            button::primary
                        } else {
                            button::text
                        }),
                );
            }
            (
                t("install_choose_data_folder").to_string(),
                Column::new()
                    .spacing(6)
                    .push(scrollable(list).height(Length::Fixed(220.0)))
                    .push(
                        text(if valid {
                            tf("install_root_valid", &[("folder", &chosen)])
                        } else {
                            tf("install_root_invalid", &[("folder", &chosen)])
                        })
                        .size(sz(11))
                        .color(if valid {
                            pal().success
                        } else {
                            pal().error
                        }),
                    )
                    // MO2 warns but still lets you through: the checker only knows
                    // what the game itself calls mod content, and plenty of valid
                    // mods (SKSE plugins, tool configs) match none of it.
                    //
                    // "folder names" alone was already half the story - extensions
                    // have always counted too - and became actively misleading once
                    // the vocabulary went per-game: a Stellar Blade mod is
                    // recognised ONLY by its `.pak`, never by a folder.
                    .push(
                        text(
                            t("install_root_hint"),
                        )
                        .size(sz(10)),
                    )
                    // The case that sends people round in circles: an archive whose
                    // real content is another archive. Nothing here can install it,
                    // and without saying so the dialog just repeats that nothing
                    // looks valid at every level the user tries.
                    .push(match nested_archive_hint(&p.rows) {
                        Some(h) => Element::from(text(h).size(sz(10))),
                        None => Space::new().into(),
                    })
                    .into(),
            )
        }
    };

    let mut card = Column::new()
        .spacing(10)
        .push(text(title).size(sz(15)))
        .push(name_row)
        .push(body);

    // No Install button while the BAIN question is open: the answer decides which
    // installer would even run.
    if !matches!(p.mode, PickerMode::Bain { asking: true, .. }) {
        card = card.push(
            Row::new()
                .spacing(8)
                .push(
                    button(text(t("install_install")).size(sz(12)))
                        .padding([4, 10])
                        .on_press(Message::PickerInstall)
                        .style(button::primary),
                )
                .push(
                    button(text(t("common_cancel")).size(sz(12)))
                        .padding([4, 10])
                        .on_press(Message::PickerCancel)
                        .style(button::text),
                ),
        );
    }
    container(card)
        .max_width(520.0)
        .padding(16)
        .style(card_style)
        .into()
}
