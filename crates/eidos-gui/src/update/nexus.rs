//! Nexus: identifying archives, endorsing, update checks and collections.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Identify an archive by its MD5 (MO2's Query Metadata) -----------
        Message::IdentifyDownload(name) => {
            // One at a time. The row greys itself, but a keyboard repeat or a
            // second row's button would otherwise start another whole-file hash
            // on the same executor - and the two would race to write one
            // sidecar.
            if app.identifying_download.is_some() {
                return Task::none();
            }
            let Some(row) = app.downloads.iter().find(|d| d.name == name) else {
                return Task::none();
            };
            let Some(game) = selected_game(app) else {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            };
            let (path, domain, short) =
                (row.path.clone(), game.def.nexus_game, game.def.short_name);
            app.identifying_download = Some(name.clone());
            app.status = Some(tf("status_identifying", &[("name", &name)]));
            // Hashing hundreds of megabytes and a round trip to Nexus: off the
            // draw thread, or the window freezes for the duration.
            // Hashing hundreds of megabytes and a blocking HTTP round trip do
            // not belong on the async executor iced drives the window with:
            // held there they freeze every other task, including the one that
            // would repaint this row. A thread does the work and the channel
            // carries the answer back.
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let outcome = (|| {
                    let md5 = eidos_nexus::md5_file(&path)
                        .map_err(|e| tf("status_identify_read_failed", &[("error", &e)]))?;
                    let nexus = eidos_nexus::Nexus::connect()?;
                    let found = nexus.md5_search(domain, &md5)?;
                    let label = found.remote.name.clone();
                    eidos_nexus::write_recovered_meta(&path, short, &found)
                        .map_err(|e| tf("status_identify_sidecar_failed", &[("error", &e)]))?;
                    Ok(label)
                })();
                let _ = tx.send(outcome);
            });
            return Task::perform(
                async move {
                    rx.recv()
                        .unwrap_or_else(|_| Err(t("status_lookup_stopped").into()))
                },
                Message::IdentifiedDownload,
            );
        }
        Message::IdentifiedDownload(result) => {
            app.identifying_download = None;
            match result {
                Ok(name) => {
                    app.status = Some(tf("status_identified", &[("name", &name)]));
                    // The row reads its state from the sidecar that now exists.
                    load_downloads(app);
                }
                Err(e) => app.status = Some(tf("status_identify_failed", &[("error", &e)])),
            }
        }
        // ---- Endorse ---------------------------------------------------------
        Message::ModEndorse(i) => {
            if app.endorsing.is_some() {
                return Task::none();
            }
            // Cheap check here (it reads a file); the actual credential is chosen
            // inside the task, because renewing an OAuth token costs a round trip
            // and must not run on the UI thread.
            if !eidos_nexus::Nexus::have_credentials() {
                app.status = Some(t("status_connect_nexus_first").to_string());
                return Task::none();
            }
            let domain = selected_game(app).map(|g| g.def.nexus_game.to_string());
            let folder = app.mods.get(i).map(|m| m.name.clone()).unwrap_or_default();
            let info = app
                .created
                .as_ref()
                .zip(app.mods.get(i))
                .filter(|(_, m)| !m.is_separator())
                .map(|(inst, m)| {
                    let meta = inst.mod_meta(&m.name);
                    (
                        meta.mod_id(),
                        meta.version().unwrap_or_default(),
                        meta.endorsed(),
                    )
                });
            let (Some(domain), Some((Some(mod_id), version, endorsed))) = (domain, info) else {
                app.status = Some(t("status_no_nexus_id_endorse").to_string());
                return Task::none();
            };
            // Toggle: endorse when not yet endorsed, abstain when already endorsed.
            let Some(instance) = app.created.as_ref().map(|i| i.root.clone()) else {
                return Task::none();
            };
            let endorse = !endorsed;
            app.endorsing = Some(i);
            app.status = Some(if endorse {
                t("status_endorsing").to_string()
            } else {
                t("status_abstaining").to_string()
            });
            return Task::perform(
                async move {
                    eidos_nexus::Nexus::connect()?.set_endorsed(&domain, mod_id, &version, endorse)
                },
                move |r| Message::ModEndorsed(instance.clone(), folder.clone(), mod_id, r),
            );
        }
        Message::ModEndorsed(instance, folder, mod_id, result) => {
            if app.created.as_ref().is_none_or(|i| i.root != instance) {
                return Task::none();
            }
            app.endorsing = None;
            match result {
                Ok(now_endorsed) => {
                    // Persist by folder name: the row index from before the network
                    // round-trip may point at a different mod by now.
                    if let (Some(inst), Some(m)) = (
                        app.created.as_ref(),
                        app.mods.iter().find(|m| m.name == folder),
                    ) {
                        let _lock = match inst.try_lock("saving a Nexus endorsement") {
                            Ok(lock) => lock,
                            Err(e) => {
                                app.status = Some(tf("status_endorse_meta_busy", &[("error", &e)]));
                                return Task::none();
                            }
                        };
                        let Ok(mut meta) =
                            eidos_instance::ModMeta::read_checked(&inst.meta_path(&m.name))
                        else {
                            app.status = Some(t("status_endorse_meta_unreadable").into());
                            return Task::none();
                        };
                        if meta.mod_id() != Some(mod_id) {
                            return Task::none();
                        }
                        meta.set("endorsed", if now_endorsed { "1" } else { "0" });
                        if let Err(e) = meta.write(&inst.meta_path(&m.name)) {
                            app.status =
                                Some(tf("status_endorse_meta_save_failed", &[("error", &e)]));
                            return Task::none();
                        }
                        app.status = Some(tf(
                            if now_endorsed {
                                "status_endorsed"
                            } else {
                                "status_abstained"
                            },
                            &[("name", &m.display_name())],
                        ));
                    }
                    recompute_counts(app);
                }
                Err(e) => app.status = Some(tf("status_endorse_failed", &[("error", &e)])),
            }
        }
        // ---- Mod update check ------------------------------------------------
        Message::CheckUpdates => {
            if app.update_in_progress {
                return Task::none();
            }
            if !eidos_nexus::Nexus::have_credentials() {
                app.status = Some(t("status_connect_nexus_first").to_string());
                return Task::none();
            }
            let Some(domain) = selected_game(app).map(|g| g.def.nexus_game.to_string()) else {
                return Task::none();
            };
            let Some(inst) = app.created.clone() else {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            };
            app.update_in_progress = true;
            app.status = Some(t("status_checking_nexus_updates").to_string());
            return Task::perform(
                async move {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    eidos_nexus::check_updates(&nexus, &inst, &domain)
                },
                Message::UpdatesChecked,
            );
        }
        Message::UpdatesChecked(result) => {
            app.update_in_progress = false;
            match result {
                Ok(r) => {
                    // The check rewrote a version line in an unknown number of
                    // meta.ini files, so this is the one case that really does
                    // need the whole map back.
                    app.meta_cache.clear();
                    refresh_meta_cache(app);
                    recompute_counts(app);
                    // The budget the server reported on the way through. It was
                    // already in the result and thrown away; the status bar shows
                    // it now, so an update check stops being the only way to find
                    // out how much of the hour is left.
                    app.nexus_hourly_left = r.hourly_remaining;
                    app.nexus_daily_left = r.daily_remaining;
                    let mut msg = tf(
                        "status_update_check",
                        &[("checked", &r.checked), ("found", &r.updates_found)],
                    );
                    if !r.unavailable.is_empty() {
                        // Said out loud rather than left to a glyph: an
                        // unavailable mod never shows an update, so nothing else
                        // in this check would ever mention it.
                        let names = r
                            .unavailable
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ");
                        msg.push(' ');
                        msg.push_str(&tf(
                            "status_update_gone",
                            &[("n", &r.unavailable.len()), ("names", &names)],
                        ));
                    }
                    if !r.failures.is_empty() {
                        for (name, error) in &r.failures {
                            eidos_log::warn!("Update not checked for {name}: {error}");
                        }
                        msg.push(' ');
                        msg.push_str(&tf("status_update_failures", &[("n", &r.failures.len())]));
                    }
                    if r.rate_limited {
                        msg.push(' ');
                        msg.push_str(t("status_update_rate_limited"));
                    }
                    app.status = Some(msg);
                }
                Err(e) => app.status = Some(tf("status_update_check_failed", &[("error", &e)])),
            }
        }
        // ---- Nexus collections ------------------------------------------------
        Message::ShowCollection(link) => {
            app.file_menu_open = false;
            app.collection = Some(CollectionState {
                install_check: None,
                runtime: None,
                link,
                revision: None,
                states: Vec::new(),
                loading: false,
                error: None,
                confirm_fetch: false,
                asked: std::collections::HashSet::new(),
            });
            if app
                .collection
                .as_ref()
                .is_some_and(|c| !c.link.trim().is_empty())
            {
                return update(app, Message::CollectionFetch);
            }
        }
        Message::CloseCollection => app.collection = None,
        Message::CollectionLinkChanged(t) => {
            app.typing = true;
            if let Some(c) = &mut app.collection {
                c.link = t;
                c.install_check = None;
                c.runtime = None;
                c.loading = false;
                c.error = None;
            }
        }
        Message::CollectionFetch => {
            let Some(c) = &mut app.collection else {
                return Task::none();
            };
            let parsed = match eidos_nexus::NxmLink::parse(c.link.trim()) {
                Ok(eidos_nexus::NxmLink::Collection(c)) => c,
                Ok(eidos_nexus::NxmLink::Mod(_)) => {
                    c.error = Some(t("collection_single_mod_link").to_string());
                    return Task::none();
                }
                Err(e) => {
                    c.error = Some(e);
                    return Task::none();
                }
            };
            // The collection's game must be the OPEN instance's game. Without
            // this the member list is joined against a different game's mods and
            // downloads, so every "installed" and every "missing" is noise
            // wearing the shape of an answer - and "Try to fetch missing" would
            // send the downloads to a third place again, since the nxm handler
            // routes by domain and the window does not.
            let here = selected_game(app).map(|g| (g.def.nexus_game, g.def.name));
            match here {
                Some((domain, _)) if domain.eq_ignore_ascii_case(&parsed.game) => {}
                Some((_, name)) => {
                    let c = app.collection.as_mut().expect("checked above");
                    c.error = Some(tf(
                        "collection_wrong_game",
                        &[("game", &parsed.game), ("open", &name)],
                    ));
                    return Task::none();
                }
                None => {
                    let c = app.collection.as_mut().expect("checked above");
                    c.error = Some(t("status_open_instance_first").to_string());
                    return Task::none();
                }
            }
            let c = app.collection.as_mut().expect("checked above");
            if !eidos_nexus::Nexus::have_credentials() {
                c.error = Some(t("status_connect_nexus_first_short").to_string());
                return Task::none();
            }
            c.loading = true;
            c.error = None;
            c.revision = None;
            c.states.clear();
            // A different revision is a different member list; what the previous
            // one asked for says nothing about this one.
            c.asked.clear();
            let request_link = c.link.clone();
            let Some(instance) = app.created.as_ref().map(|i| i.root.clone()) else {
                return Task::none();
            };
            return Task::perform(
                async move {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    nexus.collection_revision(&parsed)
                },
                move |r| Message::CollectionFetched(instance.clone(), request_link.clone(), r),
            );
        }
        Message::CollectionFetched(instance, request_link, result) => {
            if app.created.as_ref().is_none_or(|i| i.root != instance) {
                return Task::none();
            }
            let Some(c) = &mut app.collection else {
                return Task::none();
            };
            if c.link != request_link {
                return Task::none();
            }
            c.loading = false;
            match result {
                Ok(rev) => {
                    c.revision = Some(rev);
                    c.error = None;
                }
                Err(e) => {
                    c.error = Some(e);
                    c.revision = None;
                }
            }
            // Joined against the instance HERE rather than in the view: the view
            // runs every frame and this reads every mod's meta.ini once.
            recompute_collection_states(app);
        }
        Message::CollectionOpenMod(i) => {
            let Some(c) = app.collection.as_ref() else {
                return Task::none();
            };
            let Some(m) = c.revision.as_ref().and_then(|r| r.mods.get(i)) else {
                return Task::none();
            };
            // The mod's FILES tab, at the exact file the collection pins - not
            // the mod's front page, which shows whatever is newest. Clicking the
            // Mod Manager Download button there fires the nxm:// link that the
            // registered handler already knows how to service, so the existing
            // download, sidecar and install pipeline runs unchanged.
            let url = format!(
                "https://www.nexusmods.com/{}/mods/{}?tab=files&file_id={}",
                m.domain, m.mod_id, m.file_id
            );
            return update(app, Message::OpenUrl(url));
        }
        Message::CollectionFetchMissing => {
            let Some(c) = app.collection.as_ref() else {
                return Task::none();
            };
            let Some(rev) = c.revision.as_ref() else {
                return Task::none();
            };
            // The pane's instance, not whatever `eidos nxm` would resolve on its
            // own. The handler picks by game domain, so with two instances of
            // one game it can send a collection's downloads somewhere other than
            // the window that asked for them.
            let Some(inst) = app.created.as_ref().map(|i| i.root.clone()) else {
                app.status = Some(t("status_open_an_instance_first").to_string());
                return Task::none();
            };
            let (batch, left) = next_fetch_batch(rev, &c.states, &c.asked, FETCH_BATCH);
            let missing = batch;
            if missing.is_empty() {
                // The escape hatch matters: a transfer that failed leaves its
                // member missing AND already-asked, so without saying this the
                // pane looks stuck.
                app.status = Some(t("collection_nothing_left").to_string());
                return Task::none();
            }
            // Two clicks, like every other action here that does something real.
            if !c.confirm_fetch {
                let c = app.collection.as_mut().expect("checked above");
                c.confirm_fetch = true;
                return Task::none();
            }
            let mut started = 0usize;
            for (mod_id, file_id, domain) in &missing {
                let link = format!("nxm://{domain}/mods/{mod_id}/files/{file_id}");
                match eidos_command()
                    .arg("nxm")
                    .arg(&link)
                    .env("EIDOS_INSTANCE", &inst)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(_) => {
                        started += 1;
                        if let Some(c) = app.collection.as_mut() {
                            c.asked.insert(*file_id);
                        }
                    }
                    Err(e) => {
                        app.status = Some(tf("collection_downloads_failed", &[("error", &e)]));
                        if let Some(c) = app.collection.as_mut() {
                            c.confirm_fetch = false;
                        }
                        return Task::none();
                    }
                }
            }
            if let Some(c) = app.collection.as_mut() {
                c.confirm_fetch = false;
            }
            app.status = Some(if left > 0 {
                tf(
                    "collection_started_batch",
                    &[("n", &started), ("left", &left)],
                )
            } else {
                tf("collection_started_all", &[("n", &started)])
            });
        }
        Message::CollectionInstall => {
            if app.transfer_job.is_some() {
                app.status = Some(t("status_instance_busy").into());
                return Task::none();
            }
            let (Some(inst), Some(game), Some(target)) = (
                app.created.clone(),
                selected_game(app).cloned(),
                collection_target(app),
            ) else {
                app.status = Some(t("status_open_instance_first").into());
                return Task::none();
            };
            let Some(c) = app.collection.as_mut() else {
                return Task::none();
            };
            if c.loading {
                return Task::none();
            }
            let link = c.link.clone();
            if let Some((checked_target, checked)) = &c.runtime {
                if checked_target == &target
                    && matches!(
                        checked,
                        eidos_collections::recipe::RuntimeCheck::Mismatch { .. }
                    )
                {
                    let approved = Some(checked.clone());
                    app.collection = None;
                    let id = game.def.id.to_owned();
                    app.transfer_job = Some(spawn_collection(inst, game, id, link, approved));
                    return Task::none();
                }
            }
            static REQUEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let request = REQUEST.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            c.install_check = Some(request);
            c.runtime = None;
            c.loading = true;
            c.error = None;
            let worker_link = link.clone();
            return Task::perform(
                async move { collection_runtime_on_worker(&inst, &game, &worker_link) },
                move |result| Message::CollectionInstallChecked {
                    request,
                    target: target.clone(),
                    link: link.clone(),
                    result,
                },
            );
        }
        Message::CollectionInstallChecked {
            request,
            target,
            link,
            result,
        } => {
            if collection_target(app).as_ref() != Some(&target) || app.transfer_job.is_some() {
                return Task::none();
            }
            let Some(c) = app.collection.as_mut() else {
                return Task::none();
            };
            if c.install_check != Some(request) || c.link != link {
                return Task::none();
            }
            c.install_check = None;
            c.loading = false;
            match result {
                Err(error) => c.error = Some(error),
                Ok(check @ eidos_collections::recipe::RuntimeCheck::Mismatch { .. }) => {
                    c.error = Some(check.message());
                    c.runtime = Some((target, check));
                }
                Ok(check) => {
                    app.status = Some(check.message());
                    let (Some(inst), Some(game)) =
                        (app.created.clone(), selected_game(app).cloned())
                    else {
                        return Task::none();
                    };
                    app.collection = None;
                    let id = game.def.id.to_owned();
                    app.transfer_job = Some(spawn_collection(inst, game, id, link, None));
                }
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
