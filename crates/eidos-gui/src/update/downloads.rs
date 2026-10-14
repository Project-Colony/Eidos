//! The Downloads tab.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Downloads manager ----
        Message::RefreshDownloads => {
            load_downloads(app);
            app.status = Some(tf("status_found_downloads", &[("n", &app.downloads.len())]));
        }
        Message::CleanInstallDebris => {
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let dir = inst.mods_dir();
            let mut gone = 0usize;
            let mut in_use = 0usize;
            let mut failed: Vec<String> = Vec::new();
            for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(name) = e.file_name().into_string() else {
                    continue;
                };
                // The prefix is the whole guard, and it is checked HERE rather
                // than trusted from the diagnostic: the card the user clicked
                // may have been built before a refresh, and this loop deletes.
                if !name.starts_with(".eidos-install") {
                    continue;
                }
                // The temp's name embeds the pid that made it
                // (`.eidos-install-<pid>-<n>`), and a LIVE pid means a CLI
                // install is extracting into it right now - deleting it out
                // from under 7-Zip failed that install while this button
                // called it safe debris. Alive = not debris; skip it.
                let live = name
                    .strip_prefix(".eidos-install-")
                    .and_then(|r| r.split('-').next())
                    .and_then(|p| p.parse::<u32>().ok())
                    .is_some_and(|pid| std::path::Path::new(&format!("/proc/{pid}")).exists());
                if live {
                    in_use += 1;
                    continue;
                }
                match fs::remove_dir_all(e.path()) {
                    Ok(()) => gone += 1,
                    Err(err) => failed.push(format!("{name}: {err}")),
                }
            }
            let busy = if in_use > 0 {
                format!(" {}", tf("status_extraction_in_use", &[("n", &in_use)]))
            } else {
                String::new()
            };
            app.status = Some(if failed.is_empty() {
                tf("status_extraction_removed", &[("n", &gone), ("busy", &busy)])
            } else {
                tf(
                    "status_extraction_partial",
                    &[("n", &gone), ("failed", &failed.join(", ")), ("busy", &busy)],
                )
            });
            app.diag_dirty = true;
            reload_mods(app);
        }
        Message::DownloadTick => {
            // Cheap and bounded: one read_dir of a directory holding a few dozen
            // files. It is NOT called from view() - that lesson is already paid
            // for - so it costs twice a second, not once per frame.
            load_downloads(app);
            // A resume that FAILED has to say so. The child is also reaped here:
            // left unwaited it becomes a zombie, and a zombie's /proc entry is
            // what made `stop_download` wait out its whole timeout and refuse to
            // clear the recorded pid.
            if let Some((name, child, log)) = &mut app.resuming {
                if let Ok(Some(status)) = child.try_wait() {
                    let (name, log) = (name.clone(), log.clone());
                    app.resuming = None;
                    if !status.success() {
                        let why = std::fs::read_to_string(&log)
                            .unwrap_or_default()
                            .lines()
                            .rev()
                            .find(|l| !l.trim().is_empty())
                            .unwrap_or(t("status_no_reason"))
                            .to_string();
                        app.status = Some(tf("status_resume_failed", &[("name", &name), ("error", &why)]));
                    }
                }
            }
        }
        Message::DeleteDownload(name) => {
            app.confirm_delete_download = Some(name);
        }
        Message::PauseDownload(name) => {
            let Some(row) = app.downloads.iter().find(|r| r.name == name) else {
                return Task::none();
            };
            let path = row.path.clone();
            // The flag goes down FIRST. If the process dies between the two
            // writes, the row still reads as paused rather than reverting to
            // "stalled" - which is the same state on disk but a different thing
            // to tell the user, and the one that says something went wrong.
            let _ = eidos_nexus::set_download_meta_key(&path, "paused", "true");
            if eidos_nexus::stop_download(&path) {
                app.status = Some(tf("status_download_paused", &[("name", &name)]));
            } else {
                // No live process: it had already stopped. The flag still tells
                // the truth about what the user wants.
                app.status = Some(tf("status_download_marked_paused", &[("name", &name)]));
            }
            load_downloads(app);
        }
        Message::ResumeDownload(name) => {
            let Some(row) = app.downloads.iter().find(|r| r.name == name) else {
                return Task::none();
            };
            let path = row.path.clone();
            if eidos_nexus::live_download_pid(&path).is_some() {
                app.status = Some(tf("status_already_downloading", &[("name", &name)]));
                return Task::none();
            }
            let _ = eidos_nexus::set_download_meta_key(&path, "paused", "false");
            // A separate process, exactly like a browser-initiated download: the
            // window must not be blocked for the length of a transfer, and the
            // downloads tick already reports progress from the partial's size.
            // The child's stderr is CAPTURED, not discarded. The one failure
            // this path exists to surface - a non-premium account cannot mint a
            // fresh link, so an unattended resume is impossible - is printed
            // there, and sending it to /dev/null left the window saying
            // "Resuming..." while the row quietly went back to Stalled.
            let log = eidos_log::log_dir().join("resume.log");
            let _ = std::fs::create_dir_all(eidos_log::log_dir());
            let sink = std::fs::File::create(&log).ok();
            let mut cmd = eidos_command();
            cmd.arg("nxm")
                .arg("--resume")
                .arg(&path)
                .stdin(std::process::Stdio::null());
            match sink {
                Some(f) => {
                    let dup = f.try_clone().ok();
                    cmd.stdout(std::process::Stdio::from(f));
                    if let Some(d) = dup {
                        cmd.stderr(std::process::Stdio::from(d));
                    }
                }
                None => {
                    cmd.stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null());
                }
            }
            match cmd.spawn() {
                Ok(child) => {
                    app.resuming = Some((name.clone(), child, log));
                    app.status = Some(tf("status_resuming", &[("name", &name)]));
                }
                Err(e) => app.status = Some(tf("status_resume_failed", &[("name", &name), ("error", &e)])),
            }
            load_downloads(app);
        }
        Message::ConfirmDeleteDownload(name) => {
            // Armed and confirmed on the SAME file. The list re-sorts under a
            // background tick, so an index would have been a way to delete the
            // wrong archive by standing still.
            if app.confirm_delete_download.as_deref() == Some(name.as_str()) {
                if let Some(row) = app.downloads.iter().find(|r| r.name == name) {
                    let name = row.name.clone();
                    // Stop the transfer BEFORE unlinking anything. It writes to
                    // `<archive>.unfinished` and finishes with a rename onto the
                    // real name: delete those out from under a live process and
                    // it keeps filling the orphaned inode, then puts the archive
                    // back on disk seconds after the user removed it.
                    eidos_nexus::stop_download(&row.path);
                    // Remove the archive and its `.meta` sidecar together (MO2 keeps
                    // them paired). A missing sidecar is fine.
                    let meta = PathBuf::from(format!("{}.meta", row.path.display()));
                    // The partial goes too. A stalled download has no archive at
                    // all - only `<name>.unfinished` - so removing the archive
                    // and the sidecar would leave the very file that puts the row
                    // back on the next tick.
                    let partial = eidos_nexus::unfinished_path(&row.path);
                    let had_partial = partial.is_file();
                    let archive_res = std::fs::remove_file(&row.path);
                    let _ = std::fs::remove_file(&partial);
                    let _ = std::fs::remove_file(&meta);
                    match archive_res {
                        Ok(()) => app.status = Some(tf("status_download_deleted", &[("name", &name)])),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound && had_partial => {
                            app.status = Some(tf("status_download_partial_removed", &[("name", &name)]));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.status = Some(tf("status_download_gone", &[("name", &name)]));
                        }
                        Err(e) => app.status = Some(tf("status_delete_named_failed", &[("name", &name), ("error", &e)])),
                    }
                }
                load_downloads(app);
            }
        }
        Message::HideDownload(name) => {
            // MO2's `removed=` in the sidecar, so hiding here hides there too -
            // and, crucially, the ARCHIVE stays. That is the whole point: the
            // list is a library, and putting a book away is not burning it.
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let archive = inst.downloads_dir().join(&name);
            // Toggle, so the same button brings it back - a one-way hide with no
            // visible undo is how a library loses things.
            let hiding = !app.downloads.iter().any(|r| r.name == name && r.hidden);
            // Through `ModMeta`, which CREATES the sidecar when there is none.
            // `set_download_meta_key` edits an existing file, and an archive
            // copied in by hand has no sidecar at all - which is exactly the
            // pile somebody wants to hide.
            let meta_path = eidos_nexus::meta_path_for(&archive);
            let mut meta = eidos_instance::ModMeta::read(&meta_path);
            meta.set("removed", if hiding { "true" } else { "false" });
            match meta.write(&meta_path) {
                Ok(()) => {
                    load_downloads(app);
                    app.status = Some(if hiding {
                        tf("status_download_hidden", &[("name", &name)])
                    } else {
                        tf("status_download_unhidden", &[("name", &name)])
                    });
                }
                Err(e) => app.error = Some(tf("status_update_named_failed", &[("name", &name), ("error", &e)])),
            }
        }
        Message::ToggleShowHiddenDownloads => {
            app.dl_show_hidden = !app.dl_show_hidden;
            load_downloads(app);
        }
        Message::DownloadFilterChanged(t) => {
            app.typing = true;
            app.dl_filter = t;
            load_downloads(app);
        }
        Message::DownloadSortChanged(sort) => {
            app.dl_sort = sort;
            load_downloads(app);
        }
        Message::PurgeInstalledDownloads => {
            // Two clicks, like every other action here that removes something -
            // and this one removes many at once.
            app.confirm_purge_installed = true;
        }
        Message::ConfirmPurgeInstalled => {
            app.confirm_purge_installed = false;
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let dir = inst.downloads_dir();
            // Only what is on screen AND installed. Not every installed archive
            // in the folder: the filter is how the user said which ones they
            // meant, and a bulk delete that ignores it deletes things they were
            // not looking at.
            let doomed: Vec<String> = app
                .downloads
                .iter()
                .filter(|r| r.state == DownloadState::Installed)
                .map(|r| r.name.clone())
                .collect();
            if doomed.is_empty() {
                app.status = Some(t("status_nothing_installed_to_remove").to_string());
                return Task::none();
            }
            let (mut gone, mut failed) = (0usize, 0usize);
            for name in &doomed {
                let archive = dir.join(name);
                // The sidecar goes with it. Leaving one behind would make the
                // archive reappear as a ghost row with no file.
                let sidecar = PathBuf::from(format!("{}.meta", archive.display()));
                match std::fs::remove_file(&archive) {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&sidecar);
                        gone += 1;
                    }
                    Err(_) => failed += 1,
                }
            }
            load_downloads(app);
            app.status = Some(if failed == 0 {
                tf("status_archives_removed", &[("n", &gone)])
            } else {
                tf("status_archives_removed_partial", &[("n", &gone), ("failed", &failed)])
            });
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
