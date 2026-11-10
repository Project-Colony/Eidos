//! Nexus: identifying archives, endorsing, tracking, update checks and
//! collections.

use super::*;

/// Installed collections whose latest visible revision is newer than the one
/// installed. Errors are logged and yield nothing.
fn collection_notices(root: &Path) -> Vec<CollectionNotice> {
    let states = eidos_collections::state::installed(root);
    if states.is_empty() {
        return Vec::new();
    }
    let nexus = match eidos_nexus::Nexus::connect() {
        Ok(n) => n,
        Err(e) => {
            eidos_log::warn!("Collection revisions not checked: {e}");
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for s in states {
        let want = eidos_nexus::NxmCollection {
            game: s.game_domain.clone(),
            slug: s.slug.clone(),
            revision: None,
        };
        match nexus.collection_revision(&want) {
            Ok(rev) if rev.visible() && rev.revision_number > s.revision => {
                let diff = eidos_collections::driver::cached_collection(root, &s.slug, s.revision)
                    .map(|old| eidos_collections::state::revision_diff(&old, &s.game_domain, &rev));
                out.push(CollectionNotice {
                    slug: s.slug,
                    domain: s.game_domain,
                    name: rev.name,
                    have: s.revision,
                    latest: rev.revision_number,
                    diff,
                });
            }
            Ok(_) => {}
            Err(e) => eidos_log::warn!("Collection {} not checked: {e}", s.slug),
        }
    }
    out
}

/// What the requirements refresh that follows an update check of the instance
/// at `root` should run on, or `None` to start nothing: the check failed to sign
/// in (`connected` false, so a second sign-in would only repeat its error),
/// another instance is open now, or no mod of this one has a Nexus id to look
/// up. Reads the row cache only, so tests ask it rather than send the message
/// that would sign in to Nexus with the real `nexus.ini`.
pub(super) fn requirements_refresh_target(
    app: &App,
    root: &Path,
    connected: bool,
) -> Option<(Instance, &'static eidos_gamedef::GameDef)> {
    let inst = app.created.as_ref().filter(|i| i.root == root)?;
    let def = selected_game(app)?.def;
    let any_nexus_mod = app.meta_cache.values().any(|m| m.mod_id.is_some());
    (connected && any_nexus_mod).then(|| (inst.clone(), def))
}

/// The Nexus mod at row `i` of the open instance, for Endorse and Track:
/// `(folder, meta, mod id, domain)`, or the status to show instead. The domain
/// is the one the mod's own `gameName` names: a Skyrim SE mod in an Enderal SE
/// instance is endorsed and tracked on Skyrim SE's page. `no_id_key` is the
/// status for a row without a Nexus id.
fn nexus_mod_at(
    app: &App,
    i: usize,
    no_id_key: &'static str,
) -> Result<(String, eidos_instance::ModMeta, u64, String), String> {
    let no_id = || t(no_id_key).to_string();
    let (Some(inst), Some(m), Some(game)) =
        (app.created.as_ref(), app.mods.get(i), selected_game(app))
    else {
        return Err(no_id());
    };
    if m.is_separator() {
        return Err(no_id());
    }
    let meta = inst.mod_meta(&m.name);
    let Some(mod_id) = meta.mod_id() else {
        return Err(no_id());
    };
    let game_name = meta.game_name().filter(|g| !g.is_empty());
    match (game.def.nexus_domain_for(game_name.as_deref()), game_name) {
        (Some(domain), _) => Ok((m.name.clone(), meta, mod_id, domain.to_string())),
        (None, Some(game)) => Err(tf(
            "status_unknown_nexus_game",
            &[("name", &m.display_name()), ("game", &game)],
        )),
        (None, None) => Err(no_id()),
    }
}

/// Store a flag Nexus just confirmed (`key` = `on`) in the mod's `meta.ini`.
/// By folder name: the row index from before the network round trip may point
/// at a different mod by now. `Ok(Some(display name))` once written, `Ok(None)`
/// when the folder is gone or no longer holds `mod_id`. `errors` are the status
/// keys for a busy lock (takes `{error}`), an unreadable file and a failed
/// write (takes `{error}`).
fn store_nexus_flag(
    app: &App,
    folder: &str,
    mod_id: u64,
    key: &str,
    on: bool,
    errors: [&'static str; 3],
) -> Result<Option<String>, String> {
    let (Some(inst), Some(m)) = (
        app.created.as_ref(),
        app.mods.iter().find(|m| m.name == folder),
    ) else {
        return Ok(None);
    };
    let _lock = inst
        .try_lock("saving a flag from Nexus")
        .map_err(|e| tf(errors[0], &[("error", &e)]))?;
    let path = inst.meta_path(&m.name);
    let mut meta =
        eidos_instance::ModMeta::read_checked(&path).map_err(|_| t(errors[1]).to_string())?;
    if meta.mod_id() != Some(mod_id) {
        return Ok(None);
    }
    meta.set(key, if on { "1" } else { "0" });
    meta.write(&path)
        .map_err(|e| tf(errors[2], &[("error", &e)]))?;
    Ok(Some(m.display_name().to_string()))
}

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
            // Hashing hundreds of megabytes and a blocking HTTP round trip do
            // not belong on the async executor iced drives the window with:
            // it is smol's single thread, so work held there freezes every
            // timer, input subscription and other task, including the one that
            // would repaint this row. `unblock` runs it on smol's blocking pool
            // and the future only waits. A thread plus a blocking `recv()` here
            // looked equivalent and was not: the `recv()` itself held the
            // executor for the whole hash and round trip. A panic is caught
            // inside: `unblock` would re-raise it in a detached task that
            // drops it, `IdentifiedDownload` would never arrive, and the guard
            // above would refuse every later identify for the session.
            return Task::perform(
                smol::unblock(move || {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || -> Result<String, String> {
                            let md5 = eidos_nexus::md5_file(&path)
                                .map_err(|e| tf("status_identify_read_failed", &[("error", &e)]))?;
                            let nexus = eidos_nexus::Nexus::connect()?;
                            let found = nexus.md5_search(domain, &md5)?;
                            let label = found.remote.name.clone();
                            eidos_nexus::write_recovered_meta(&path, short, &found).map_err(
                                |e| tf("status_identify_sidecar_failed", &[("error", &e)]),
                            )?;
                            Ok(label)
                        },
                    ))
                    .unwrap_or_else(|p| {
                        eidos_log::error!(
                            "download identify panicked: {}",
                            eidos_log::panic_text(&*p)
                        );
                        Err(t("status_lookup_stopped").into())
                    })
                }),
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
            let (folder, meta, mod_id, domain) =
                match nexus_mod_at(app, i, "status_no_nexus_id_endorse") {
                    Ok(found) => found,
                    Err(status) => {
                        app.status = Some(status);
                        return Task::none();
                    }
                };
            let Some(instance) = app.created.as_ref().map(|i| i.root.clone()) else {
                return Task::none();
            };
            let version = meta.version().unwrap_or_default();
            // Toggle: endorse when not yet endorsed, abstain when already endorsed.
            let endorse = !meta.endorsed();
            app.endorsing = Some(i);
            app.status = Some(if endorse {
                t("status_endorsing").to_string()
            } else {
                t("status_abstaining").to_string()
            });
            return Task::perform(
                smol::unblock(move || {
                    eidos_nexus::Nexus::connect()?.set_endorsed(&domain, mod_id, &version, endorse)
                }),
                move |r| Message::ModEndorsed(instance.clone(), folder.clone(), mod_id, r),
            );
        }
        Message::NexusFilesFetched {
            domain,
            mod_id,
            result,
        } => {
            let key = (domain, mod_id);
            // Only the answer to the request this window is waiting for.
            if app.nexus_files_loading.as_ref() == Some(&key) {
                app.nexus_files_loading = None;
                app.nexus_files.insert(key, result);
            }
        }
        Message::NexusUpdate {
            folder,
            domain,
            mod_id,
            file_id,
        } => {
            // One update waits at a time: a second one would silently drop the
            // install the first promised.
            if let Some(p) = app.pending_update.as_ref().filter(|p| p.folder != folder) {
                app.status = Some(tf("status_update_still_waiting", &[("folder", &p.folder)]));
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            if inst.mod_meta(&folder).installed_files().len() > 1 {
                app.status = Some(tf("nexus_update_merged_refused", &[("folder", &folder)]));
                return Task::none();
            }
            let root = inst.root.clone();
            let Some(premium) = app.nexus_account.as_ref().map(|a| a.is_premium) else {
                app.status = Some(t("status_connect_nexus_first").to_string());
                return Task::none();
            };
            let task = if premium {
                // The keyless link, as Fetch missing sends it: a premium
                // account can mint the download from the ids alone.
                let link = format!("nxm://{domain}/mods/{mod_id}/files/{file_id}");
                if let Err(e) = eidos_command()
                    .arg("nxm")
                    .arg(&link)
                    .env("EIDOS_INSTANCE", &root)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    app.status = Some(tf(
                        "status_start_failed",
                        &[("name", &"eidos nxm"), ("error", &e)],
                    ));
                    return Task::none();
                }
                app.status = Some(tf("status_update_started", &[("folder", &folder)]));
                Task::none()
            } else {
                // A free account downloads through the page's own button.
                let task = update(
                    app,
                    Message::OpenUrl(eidos_nexus::file_page_url(&domain, mod_id, file_id)),
                );
                app.status = Some(t("status_update_waiting_download").to_string());
                task
            };
            app.pending_update = Some(PendingUpdate {
                folder,
                mod_id,
                file_id,
            });
            return task;
        }
        Message::ModEndorsed(instance, folder, mod_id, result) => {
            if app.created.as_ref().is_none_or(|i| i.root != instance) {
                return Task::none();
            }
            app.endorsing = None;
            match result {
                Ok(now_endorsed) => {
                    let stored = store_nexus_flag(
                        app,
                        &folder,
                        mod_id,
                        "endorsed",
                        now_endorsed,
                        [
                            "status_endorse_meta_busy",
                            "status_endorse_meta_unreadable",
                            "status_endorse_meta_save_failed",
                        ],
                    );
                    match stored {
                        Ok(Some(name)) => {
                            app.status = Some(tf(
                                if now_endorsed {
                                    "status_endorsed"
                                } else {
                                    "status_abstained"
                                },
                                &[("name", &name)],
                            ));
                        }
                        Ok(None) => {}
                        Err(status) => {
                            app.status = Some(status);
                            return Task::none();
                        }
                    }
                    // The count reads the row cache, so the row this rewrote
                    // has to be dropped and read again first.
                    invalidate_meta(app, &folder);
                    refresh_meta_cache(app);
                    recompute_counts(app);
                }
                Err(e) => app.status = Some(tf("status_endorse_failed", &[("error", &e)])),
            }
        }
        // ---- Track (MO2's Track: the account's tracked mods on Nexus) --------
        Message::ModTrack(i) => {
            app.menu_mod = None;
            if app.tracking.is_some() {
                return Task::none();
            }
            if !eidos_nexus::Nexus::have_credentials() {
                app.status = Some(t("status_connect_nexus_first").to_string());
                return Task::none();
            }
            let (folder, meta, mod_id, domain) =
                match nexus_mod_at(app, i, "status_no_nexus_id_track") {
                    Ok(found) => found,
                    Err(status) => {
                        app.status = Some(status);
                        return Task::none();
                    }
                };
            let Some(instance) = app.created.as_ref().map(|i| i.root.clone()) else {
                return Task::none();
            };
            let track = !meta.tracked();
            app.tracking = Some(i);
            app.status = Some(t("status_track_requesting").to_string());
            return Task::perform(
                smol::unblock(move || {
                    eidos_nexus::Nexus::connect()?.set_tracked(&domain, mod_id, track)
                }),
                move |r| Message::ModTracked(instance.clone(), folder.clone(), mod_id, r),
            );
        }
        Message::ModTracked(instance, folder, mod_id, result) => {
            if app.created.as_ref().is_none_or(|i| i.root != instance) {
                return Task::none();
            }
            app.tracking = None;
            let now_tracked = match result {
                Ok(now) => now,
                Err(e) => {
                    app.status = Some(tf("status_track_failed", &[("error", &e)]));
                    return Task::none();
                }
            };
            let stored = store_nexus_flag(
                app,
                &folder,
                mod_id,
                "tracked",
                now_tracked,
                [
                    "status_track_meta_busy",
                    "status_track_meta_unreadable",
                    "status_track_meta_save_failed",
                ],
            );
            match stored {
                Ok(Some(name)) => {
                    app.status = Some(tf(
                        if now_tracked {
                            "status_tracking"
                        } else {
                            "status_untracked"
                        },
                        &[("name", &name)],
                    ));
                    invalidate_meta(app, &folder);
                    refresh_meta_cache(app);
                }
                Ok(None) => {}
                Err(status) => app.status = Some(status),
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
            let Some(def) = selected_game(app).map(|g| g.def) else {
                return Task::none();
            };
            let Some(inst) = app.created.clone() else {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            };
            app.update_in_progress = true;
            app.status = Some(t("status_checking_nexus_updates").to_string());
            let root = inst.root.clone();
            let notices_root = inst.root.clone();
            let notices_reply = inst.root.clone();
            return Task::batch([
                Task::perform(
                    // One `mod_info` request per stale mod, one after another:
                    // tens of seconds on a freshly rebuilt list, so on the blocking
                    // pool, never on the executor that runs the timers.
                    smol::unblock(move || match eidos_nexus::Nexus::connect() {
                        Ok(nexus) => (true, eidos_nexus::check_updates(&nexus, &inst, def)),
                        Err(e) => (false, Err(e)),
                    }),
                    move |(connected, result)| Message::UpdatesChecked {
                        root: root.clone(),
                        connected,
                        result,
                    },
                ),
                // One GraphQL request per installed collection.
                Task::perform(
                    smol::unblock(move || collection_notices(&notices_root)),
                    move |n| Message::CollectionRevisionsChecked(notices_reply.clone(), n),
                ),
            ]);
        }
        Message::CollectionRevisionsChecked(root, notices) => {
            if app.created.as_ref().is_none_or(|i| i.root != root) {
                return Task::none();
            }
            app.collection_notices = notices;
        }
        Message::RequirementsRefreshed(root, result) => {
            // The last step of Check for updates (see UpdatesChecked).
            app.update_in_progress = false;
            if app.created.as_ref().is_none_or(|i| i.root != root) {
                return Task::none();
            }
            match result {
                Ok(_) => {
                    app.meta_cache.clear();
                    refresh_meta_cache(app);
                    app.diag_dirty = true;
                }
                Err(e) => {
                    // After the update check's summary, not over it.
                    let failed = tf("health_requirements_failed", &[("error", &e)]);
                    app.status = Some(match app.status.take() {
                        Some(s) => format!("{s} {failed}"),
                        None => failed,
                    });
                }
            }
        }
        Message::IgnoreRequirement { folder, key } => {
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let saved = edit_mod_meta(inst, &folder, |meta| {
                let mut keys = meta.ignored_requirements();
                if !keys.contains(&key) {
                    keys.push(key);
                    meta.set_ignored_requirements(&keys);
                }
            });
            if let Err(e) = saved {
                app.status = Some(tf("status_meta_save_failed", &[("error", &e)]));
                return Task::none();
            }
            invalidate_meta(app, &folder);
            refresh_meta_cache(app);
            app.diag_dirty = true;
        }
        Message::RequirementDownload { domain, mod_id } => {
            let d = domain.clone();
            return Task::perform(
                smol::unblock(move || {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    let info = nexus.mod_info(&d, mod_id)?;
                    nexus
                        .mod_files(&info.gate, &d, mod_id)
                        .map(|f| main_file_id(&f))
                }),
                move |r| Message::RequirementFilePicked {
                    domain: domain.clone(),
                    mod_id,
                    // A failed lookup falls back to the files page like a
                    // mod with no single main file.
                    file_id: r.ok().flatten(),
                },
            );
        }
        Message::RequirementFilePicked {
            domain,
            mod_id,
            file_id,
        } => {
            let Some(file_id) = file_id else {
                return update(
                    app,
                    Message::OpenUrl(format!(
                        "https://www.nexusmods.com/{domain}/mods/{mod_id}?tab=files"
                    )),
                );
            };
            let Some(root) = app.created.as_ref().map(|i| i.root.clone()) else {
                return Task::none();
            };
            // The keyless link, as Fetch missing sends it.
            let link = format!("nxm://{domain}/mods/{mod_id}/files/{file_id}");
            match eidos_command()
                .arg("nxm")
                .arg(&link)
                .env("EIDOS_INSTANCE", &root)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(_) => app.status = Some(t("status_requirement_download_started").to_string()),
                Err(e) => {
                    app.status = Some(tf(
                        "status_start_failed",
                        &[("name", &"eidos nxm"), ("error", &e)],
                    ));
                }
            }
        }
        Message::UpdatesChecked {
            root,
            connected,
            result,
        } => {
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
                    if let Some(e) = &r.account_sync_error {
                        msg.push(' ');
                        msg.push_str(&tf("status_flags_not_synced", &[("error", e)]));
                    }
                    app.status = Some(msg);
                }
                Err(e) => {
                    // The check writes meta.ini mod by mod, so one that stopped
                    // midway already changed the mods before it. Reload the map
                    // as the Ok arm does, or the Updates count stays stale.
                    app.meta_cache.clear();
                    refresh_meta_cache(app);
                    recompute_counts(app);
                    app.status = Some(tf("status_update_check_failed", &[("error", &e)]));
                }
            }
            // Health reads these; GraphQL, so outside the v1 budget. Started
            // only now, and the button stays off until it answers: both write
            // meta.ini under the instance lock, which a second thread of this
            // process gets refused, not queued for.
            let Some((inst, def)) = requirements_refresh_target(app, &root, connected) else {
                app.update_in_progress = false;
                return Task::none();
            };
            return Task::perform(
                smol::unblock(move || {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    eidos_nexus::refresh_requirements(&nexus, &inst, def)
                }),
                move |r| Message::RequirementsRefreshed(root.clone(), r),
            );
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
                walk: None,
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
            c.walk = None;
            let request_link = c.link.clone();
            let Some(instance) = app.created.as_ref().map(|i| i.root.clone()) else {
                return Task::none();
            };
            return Task::perform(
                smol::unblock(move || {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    nexus.collection_revision(&parsed)
                }),
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
            let url = eidos_nexus::file_page_url(&m.domain, m.mod_id, m.file_id);
            return update(app, Message::OpenUrl(url));
        }
        Message::CollectionWalkStart => {
            let (Some(inst), Some(c)) = (app.created.as_ref(), app.collection.as_mut()) else {
                return Task::none();
            };
            let Some(rev) = c.revision.as_ref() else {
                return Task::none();
            };
            // Read from the directory, not the listed rows: a filtered list would
            // let an old archive pass for a new arrival.
            let seen = fs::read_dir(inst.downloads_dir())
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            let mut walk = CollectionWalk {
                queue: walk_queue(rev, &c.states),
                at: 0,
                seen,
            };
            let step = walk_next(&mut walk, &c.states);
            c.walk = Some(walk);
            return walk_apply(app, step);
        }
        Message::CollectionWalkSkip => {
            let Some(c) = app.collection.as_mut() else {
                return Task::none();
            };
            let Some(walk) = c.walk.as_mut() else {
                return Task::none();
            };
            walk.at += 1;
            let step = walk_next(walk, &c.states);
            return walk_apply(app, step);
        }
        Message::CollectionWalkStop => {
            if let Some(c) = app.collection.as_mut() {
                c.walk = None;
            }
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
                // The instance lock it takes is per thread, and `unblock` runs
                // the whole closure on one pool thread, so it is taken and
                // dropped on the same one.
                smol::unblock(move || collection_runtime_on_worker(&inst, &game, &worker_link)),
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

/// One look at `downloads/` for the walk, from `DownloadTick`.
pub(super) fn walk_tick(app: &mut App) -> Task<Message> {
    if app.collection.as_ref().is_none_or(|c| c.walk.is_none()) {
        return Task::none();
    }
    recompute_collection_states(app);
    let Some(c) = app.collection.as_mut() else {
        return Task::none();
    };
    let (Some(walk), Some(rev)) = (c.walk.as_mut(), c.revision.as_ref()) else {
        return Task::none();
    };
    // shortcut: read from the listed rows, so a Downloads filter hiding a wrong
    // file keeps it from being named; scan the directory if that bites.
    let fresh: Vec<&DownloadRow> = app
        .downloads
        .iter()
        .filter(|r| {
            !walk.seen.contains(&r.name)
                && !matches!(
                    r.state,
                    DownloadState::Downloading | DownloadState::Stalled | DownloadState::Paused
                )
        })
        .collect();
    let arrived: Vec<(u64, u64)> = fresh
        .iter()
        .filter_map(|r| Some((r.mod_id?, r.file_id?)))
        .collect();
    // Each arrival is judged once: the next member can be another file of the
    // same mod, and the file that landed for this one must not count against it.
    walk.seen.extend(fresh.iter().map(|r| r.name.clone()));
    let step = walk_step(walk, rev, &c.states, &arrived);
    walk_apply(app, step)
}

/// What a walk step does to the window.
fn walk_apply(app: &mut App, step: WalkStep) -> Task<Message> {
    match step {
        WalkStep::Wait => Task::none(),
        WalkStep::Open(i) => update(app, Message::CollectionOpenMod(i)),
        WalkStep::Wrong { want, got } => {
            app.status = Some(tf(
                "collection_walk_wrong",
                &[("want", &want), ("got", &got)],
            ));
            Task::none()
        }
        WalkStep::Done => {
            let Some(c) = app.collection.as_mut() else {
                return Task::none();
            };
            // Skipped members are not counted: they are not in downloads.
            let n = c.walk.take().map_or(0, |w| {
                w.queue
                    .iter()
                    .filter(|&&i| c.states.get(i) != Some(&MemberState::Missing))
                    .count()
            });
            app.status = Some(tf("collection_walk_done", &[("n", &n)]));
            Task::none()
        }
    }
}

/// The mod's one main file, when Nexus lists exactly one (category 1).
pub(crate) fn main_file_id(files: &eidos_nexus::ModFiles) -> Option<u64> {
    match files
        .files
        .iter()
        .filter(|f| f.category_id == 1)
        .collect::<Vec<_>>()[..]
    {
        [f] => Some(f.file_id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_main_file_is_fetched_without_asking() {
        let f = |id, cat| eidos_nexus::NexusFile {
            file_id: id,
            category_id: cat,
            ..Default::default()
        };
        let files = |v| eidos_nexus::ModFiles {
            files: v,
            updates: vec![],
        };
        assert_eq!(main_file_id(&files(vec![f(5, 1), f(6, 3)])), Some(5));
        assert_eq!(main_file_id(&files(vec![f(5, 1), f(6, 1)])), None);
        assert_eq!(main_file_id(&files(vec![f(6, 3)])), None);
    }
}
