//! The Downloads tab and the scan that fills it.

use super::*;

/// A short status label for a download row (MO2's downloads state column).
pub(crate) fn download_state_label(state: DownloadState) -> &'static str {
    match state {
        DownloadState::Untracked => "-",
        DownloadState::Downloading => t("dl_state_downloading"),
        DownloadState::Stalled => t("dl_state_stalled"),
        DownloadState::Paused => t("dl_state_paused"),
        DownloadState::Ready => t("dl_state_ready"),
        DownloadState::Installed => t("dl_state_installed"),
        DownloadState::Uninstalled => t("dl_state_uninstalled"),
    }
}

/// The colour of the status word, following MO2's own scheme
/// (downloadlist.cpp:202): green for the one state that is asking to be acted
/// on, amber for a mod that was installed and then removed, and NOTHING for
/// "Installed" - a finished job should not keep waving.
pub(crate) fn download_state_color(state: DownloadState, theme: &Theme) -> Option<Color> {
    match state {
        DownloadState::Ready => Some(theme.palette().success),
        DownloadState::Uninstalled => Some(theme.palette().warning),
        // Burgundy for the one that is happening right now, amber for one that
        // stopped and is waiting to be resumed.
        DownloadState::Downloading => Some(theme.palette().primary),
        DownloadState::Stalled => Some(theme.palette().warning),
        // Paused is not a warning: nothing went wrong, the user asked for it.
        DownloadState::Paused => None,
        DownloadState::Installed | DownloadState::Untracked => None,
    }
}

// Downloads column widths, declared once so the header and the rows cannot drift
// apart. Each is sized to its widest real content and no more: every pixel they
// do not take goes to the name, which is the only column whose content has no
// bound - Nexus file names run to eighty characters.
//
// They were 80/80/90/150, which is 400px of a pane that is roughly 500 wide once
// the mod list has its half. That left about 68px for the name, so "Dynamic Armor
// Physics" came out three lines tall. The action column was the worst of it: 150
// reserved for two buttons that measure about 100.
pub(crate) const DL_C_VERSION: f32 = 56.0; // "1.0.1"

pub(crate) const DL_C_SIZE: f32 = 66.0; // "10.3 MiB"

pub(crate) const DL_C_STATUS: f32 = 66.0; // "Installed"
                                          // Sized on the WIDEST pair the column can ever hold at once, which is not the
                                          // resting state: "Reinstall" beside Delete armed as "Confirm?". Sizing it on
                                          // "Install" + "Delete" would clip the two labels that only appear when something
                                          // is at stake.

pub(crate) const DL_C_ACTIONS: f32 = 128.0;

/// Fixed height for the action cell, so a row does not change height at the one
/// moment it changes CONTENT: a finishing download swaps its progress bar for
/// two buttons, and if those measured differently the whole list below would
/// jump at exactly the instant the user was watching it.
pub(crate) const DL_ACTION_H: f32 = 24.0;

/// Fixed width for the speed/percentage readout, so the progress bar beside it
/// keeps the same geometry from one tick to the next. Fits "12.3 MiB/s".
pub(crate) const DL_READOUT_W: f32 = 54.0;

pub(crate) fn downloads_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text(t("panel_no_instance")).into();
    };
    let dir = inst.downloads_dir();

    let header = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text(t("tab_downloads")).size(sz(13)))
        .push(Space::new().width(Length::Fill))
        .push(
            button(text(t("common_open_folder")).size(sz(11)))
                .padding(4)
                .on_press(Message::OpenFolder(dir.clone())),
        )
        .push(
            button(text(t("toolbar_refresh")).size(sz(11)))
                .padding(4)
                .on_press(Message::RefreshDownloads),
        );

    // MO2 treats this tab as an archive library rather than a transfer queue,
    // and a library needs to be searchable and orderable to be worth keeping.
    let installed_here = app
        .downloads
        .iter()
        .filter(|r| r.state == DownloadState::Installed)
        .count();
    let mut tools = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text_input(t("dl_filter"), &app.dl_filter)
                .on_input(Message::DownloadFilterChanged)
                .padding(4)
                .size(sz(11))
                .width(Length::Fill),
        )
        .push(
            pick_list(
                DownloadSort::ALL.to_vec(),
                Some(app.dl_sort),
                Message::DownloadSortChanged,
            )
            .text_size(sz(11))
            .padding(4),
        )
        .push(
            button(
                text(if app.dl_show_hidden {
                    t("dl_hide_hidden")
                } else {
                    t("dl_show_hidden")
                })
                .size(sz(11)),
            )
            .padding(4)
            .style(button::secondary)
            .on_press(Message::ToggleShowHiddenDownloads),
        );
    // Offered only when it would do something, and it names the count rather
    // than saying "installed" - a bulk delete has to say how many.
    if installed_here > 0 {
        let armed = app.confirm_purge_installed;
        tools = tools.push(
            button(
                text(if armed {
                    tf("dl_confirm_purge", &[("n", &installed_here)])
                } else {
                    tf("dl_purge", &[("n", &installed_here)])
                })
                .size(sz(11)),
            )
            .padding(4)
            .style(if armed {
                button::danger
            } else {
                button::secondary
            })
            .on_press(if armed {
                Message::ConfirmPurgeInstalled
            } else {
                Message::PurgeInstalledDownloads
            }),
        );
    }

    let col_header = Row::new()
        .spacing(8)
        .push(text(t("info_name")).size(sz(11)).width(Length::Fill))
        .push(
            text(t("info_version"))
                .size(sz(11))
                .width(Length::Fixed(scaled(DL_C_VERSION))),
        )
        .push(
            text(t("col_size"))
                .size(sz(11))
                .width(Length::Fixed(scaled(DL_C_SIZE))),
        )
        .push(
            text(t("prereq_status"))
                .size(sz(11))
                .width(Length::Fixed(scaled(DL_C_STATUS))),
        )
        .push(Space::new().width(Length::Fixed(scaled(DL_C_ACTIONS))));

    let mut rows = Column::new().spacing(2);
    if app.downloads.is_empty() {
        rows = rows.push(text(t("dl_none_yet")).size(sz(11)));
    }
    for (i, row) in app.downloads.iter().enumerate() {
        let armed = app.confirm_delete_download.as_deref() == Some(row.name.as_str());
        let identifying = app.identifying_download.as_deref() == Some(row.name.as_str());
        // Two action buttons: Install (re-run the installer) and Delete.
        // MO2 keeps Install available on an already-installed archive
        // (downloadlistview.cpp:230, `state >= STATE_READY`) because re-running a
        // FOMOD with different answers is a real thing to want. What it does NOT
        // do is present it as the next step: its Install lives in a context menu,
        // and it colours the STATUS rather than the action.
        //
        // So keep the action, drop the shouting. Burgundy means "this is what to
        // do here"; on a row that is already installed, that was a lie, and the
        // label said "Install" for something that would install it a second time.
        let arriving = matches!(
            row.state,
            DownloadState::Downloading | DownloadState::Stalled | DownloadState::Paused
        );
        let installed = row.state == DownloadState::Installed;
        // Nothing can be installed out of a partial file, so while one is
        // arriving the action column carries the progress instead of two buttons
        // that would either lie or refuse. It is also the widest column, which is
        // what a bar wants.
        let actions: Element<'a, Message> = if arriving {
            // Stopped either way - the difference is only WHY, which the status
            // column already says. Both offer the same two things to do.
            let stalled = matches!(row.state, DownloadState::Stalled | DownloadState::Paused);
            let frac = if row.total > 0 {
                (row.downloaded as f32 / row.total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let label = if stalled {
                match row.total {
                    0 => format_size(row.downloaded),
                    _ => format!("{:.0}%", frac * 100.0),
                }
            } else {
                match (row.total, row.speed) {
                    // No total: an older sidecar, from before the size was recorded.
                    // Say how much has arrived rather than invent a percentage.
                    (0, _) => format_size(row.downloaded),
                    (_, Some(bps)) => format!("{}/s", format_size(bps as u64)),
                    // Between the first sighting and the next tick there is no rate
                    // yet. A "0 B/s" here would read as stopped, which it is not.
                    (_, None) => format!("{:.0}%", frac * 100.0),
                }
            };
            // FIXED width for the readout. The bar takes what is left, so a
            // label that measures differently every tick - "9.8 MiB/s" then
            // "12.3 MiB/s" then "985 KiB/s" - would resize the bar under a
            // monotonic value, and the fill would visibly step BACKWARDS while
            // the download went forwards.
            let readout = text(label)
                .size(sz(10))
                .color(fomod_ink_soft())
                .width(Length::Fixed(scaled(DL_READOUT_W)))
                .align_x(iced::alignment::Horizontal::Right);
            let cell = Row::new()
                .spacing(5)
                .align_y(iced::Alignment::Center)
                .width(Length::Fixed(scaled(DL_C_ACTIONS)))
                .height(Length::Fixed(scaled(DL_ACTION_H)));
            // A live transfer offers nothing: there is nothing to do but wait,
            // and Install on a partial file would be a lie. A STALLED one has to
            // be removable, or an abandoned download becomes a row that can never
            // be got rid of - and its partial is invisible in a file manager too,
            // having no archive extension.
            //
            // The button and the bar do not share the cell: squeezed beside it a
            // bar would be twenty pixels wide, which says less than the number
            // next to it already does. Stalled gets the number and the button.
            if stalled {
                cell.push(readout)
                    .push(
                        button(text(t("dl_resume")).size(sz(10)))
                            .padding(3)
                            .on_press(Message::ResumeDownload(row.name.clone()))
                            .style(button::primary),
                    )
                    .push(
                        button(
                            text(if armed {
                                t("common_confirm")
                            } else {
                                t("common_delete")
                            })
                            .size(sz(10)),
                        )
                        .padding(3)
                        .on_press(if armed {
                            Message::ConfirmDeleteDownload(row.name.clone())
                        } else {
                            Message::DeleteDownload(row.name.clone())
                        })
                        .style(if armed {
                            button::danger
                        } else {
                            button::secondary
                        }),
                    )
                    .into()
            } else {
                cell.push(
                    // iced 0.14 names a bar's axes `length` (along) and `girth`
                    // (across), not width/height - it can be vertical.
                    iced::widget::progress_bar(0.0..=1.0, frac)
                        .length(Length::Fill)
                        .girth(Length::Fixed(7.0)),
                )
                .push(readout)
                // A live transfer gets exactly one control: stop it. Delete is
                // deliberately absent - a partial cannot be installed, and the
                // safe order is always pause first, then decide.
                .push(
                    button(text(t("dl_pause")).size(sz(10)))
                        .padding(3)
                        .on_press(Message::PauseDownload(row.name.clone()))
                        .style(button::secondary),
                )
                .into()
            }
        } else {
            let install = button(
                text(if installed {
                    t("dl_reinstall")
                } else {
                    t("install_install")
                })
                .size(sz(11)),
            )
            .padding(4)
            .on_press(Message::ModPicked(Some(row.path.clone())))
            .style(if installed {
                button::secondary
            } else {
                button::primary
            });
            let del = button(
                text(if armed {
                    t("common_confirm")
                } else {
                    t("common_delete")
                })
                .size(sz(11)),
            )
            .padding(4)
            .on_press(if armed {
                Message::ConfirmDeleteDownload(row.name.clone())
            } else {
                Message::DeleteDownload(row.name.clone())
            })
            .style(if armed {
                button::danger
            } else {
                button::secondary
            });
            let mut actions = Row::new()
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .width(Length::Fixed(scaled(DL_C_ACTIONS)))
                .height(Length::Fixed(scaled(DL_ACTION_H)));
            // An archive with no sidecar has no mod id, so no version, no update
            // check and no Nexus page - and every archive copied in by hand
            // arrives that way. Nexus can identify it from its MD5, which is the
            // only route back (MO2 calls this Query Metadata).
            // On the ID, not on the STATE: hiding a hand-copied archive writes a
            // sidecar, which stops it being Untracked - and took the only route
            // back to a mod id with it. What Identify is for is an archive
            // nobody has identified, which is exactly "no mod id".
            if row.mod_id.is_none() {
                actions = actions.push(
                    button(text(if identifying { "..." } else { t("dl_identify") }).size(sz(11)))
                        .padding(4)
                        .style(button::secondary)
                        .on_press_maybe(
                            (!identifying).then(|| Message::IdentifyDownload(row.name.clone())),
                        ),
                );
            }
            // The mod's page, straight from the row. The sidecar has known the id
            // since the download started; nothing offered it.
            if let (Some(id), Some(domain)) = (
                row.mod_id,
                selected_game(app)
                    .map(|g| g.def.nexus_game)
                    .filter(|d| !d.is_empty()),
            ) {
                actions = actions.push(
                    button(text("Nexus").size(sz(11)))
                        .padding(4)
                        .style(button::text)
                        .on_press(Message::OpenUrl(format!(
                            "https://www.nexusmods.com/{domain}/mods/{id}"
                        ))),
                );
            }
            // Hiding is not deleting, which is why both are offered: an archive
            // somebody is done with but does not want to lose is the common case
            // this list had no answer for.
            let hide = button(
                text(if row.hidden {
                    t("info_unhide")
                } else {
                    t("info_hide")
                })
                .size(sz(11)),
            )
            .padding(4)
            .style(button::text)
            .on_press(Message::HideDownload(row.name.clone()));
            actions.push(hide).push(install).push(del).into()
        };
        // Prefer the friendly Nexus mod name when present, else the file name.
        let display = row.mod_name.clone().unwrap_or_else(|| row.name.clone());
        let r = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(text(display).size(sz(12)).width(Length::Fill))
            .push(
                text(row.version.clone())
                    .size(sz(11))
                    .width(Length::Fixed(scaled(DL_C_VERSION))),
            )
            .push(
                text(format_size(row.size))
                    .size(sz(11))
                    .width(Length::Fixed(scaled(DL_C_SIZE))),
            )
            .push({
                let st = row.state;
                let label = match (st, row.total) {
                    (DownloadState::Downloading, t) if t > 0 => {
                        format!("{:.0}%", (row.downloaded as f64 / t as f64) * 100.0)
                    }
                    _ => download_state_label(st).to_string(),
                };
                text(label)
                    .size(sz(11))
                    .width(Length::Fixed(scaled(DL_C_STATUS)))
                    .style(move |t: &Theme| iced::widget::text::Style {
                        color: download_state_color(st, t),
                    })
            })
            .push(actions);
        // Pressing the row arms a drag onto the mod list, so a download can be
        // installed AT a priority instead of always landing at the end. The
        // Install / Delete buttons swallow their own presses, exactly as the mod
        // rows do, so this never steals a click meant for them. A partial has
        // nothing to install, and the handler refuses it.
        let r: Element<'a, Message> = if arriving {
            container(r).padding(3).into()
        } else {
            mouse_area(container(r).padding(3))
                .on_press(Message::DownloadDragStart(i))
                .into()
        };
        rows = rows.push(striped(r, i % 2 == 0));
    }

    Column::new()
        .spacing(6)
        .push(header)
        .push(text(dir.display().to_string()).size(sz(10)))
        .push(tools)
        .push(col_header)
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

/// Re-scan the downloads directory into `app.downloads`, reading each archive's
/// `.meta` sidecar for its version + install status. Newest first.
pub(crate) fn load_downloads(app: &mut App) {
    let Some(inst) = &app.created else {
        app.downloads = Vec::new();
        app.confirm_delete_download = None;
        app.identifying_download = None;
        return;
    };
    let dir = inst.downloads_dir();
    let entries: Vec<(DownloadRow, std::time::SystemTime)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let raw = p.file_name()?.to_string_lossy().into_owned();
            let lower = raw.to_ascii_lowercase();

            // Two kinds of entry are a download. A finished archive, and a
            // `<archive>.unfinished` partial, which is one ARRIVING - it is
            // written by the separate `eidos nxm` process, and noticing it is
            // the whole reason a running download can be shown at all.
            let partial = lower.ends_with(".unfinished");
            let name = if partial {
                raw.get(..raw.len() - ".unfinished".len())?.to_string()
            } else {
                let is_archive =
                    lower.ends_with(".7z") || lower.ends_with(".zip") || lower.ends_with(".rar");
                if !is_archive {
                    return None;
                }
                raw
            };
            // The row always points at where the archive WILL be, so Install
            // works the instant it lands without the row being rebuilt around it.
            let dest = p.with_file_name(&name);
            let md = e.metadata().ok()?;
            let modified = md.modified().ok()?;

            // Version + install status from the MO2-format `.meta` sidecar.
            let meta_path = PathBuf::from(format!("{}.meta", dest.display()));
            let meta = eidos_instance::ModMeta::read(&meta_path);
            let has_meta = std::fs::metadata(&meta_path).is_ok();
            let total = meta.total_size().unwrap_or(0);
            let state = if partial {
                // Growing or abandoned? The writer is another process, so there
                // is nobody to ask - but a file being appended to has a fresh
                // mtime. A generous window, because a slow mirror can go quiet
                // for a few seconds without being dead.
                //
                // The partial's mtime ALONE is not enough, and getting that wrong
                // loses a download. `eidos nxm` writes the sidecar before the
                // first byte, then makes three API calls and waits on the CDN
                // before it opens the partial - and a RESUMED download appends,
                // which does not touch the mtime until the first byte lands. So
                // for the whole API-plus-latency window a live retry carries the
                // DEAD attempt's mtime, read as "Stalled", which offered a Delete
                // that unlinked the file the running process was writing to: it
                // kept filling an unlinked inode, its rename failed, and the
                // transfer vanished with its sidecar and no message anywhere.
                //
                // The sidecar is the missing signal. It is rewritten on every
                // attempt, so the NEWER of the two mtimes is when something last
                // happened. A genuinely dead download has an equally old sidecar,
                // so this does not weaken the true case.
                let touched = std::fs::metadata(&meta_path)
                    .and_then(|m| m.modified())
                    .map_or(modified, |t| t.max(modified));
                let quiet = touched
                    .elapsed()
                    .map(|d| d > STALLED_AFTER)
                    .unwrap_or(false);
                // Paused BEATS the mtime heuristic, in both directions. A user
                // who paused a second ago has a fresh mtime and would otherwise
                // still read as "Downloading" for the whole quiet window; and
                // one paused an hour ago would read as "Stalled", which says
                // something went wrong when nothing did.
                if eidos_nexus::download_meta_key(&dest, "paused").as_deref() == Some("true") {
                    DownloadState::Paused
                } else if quiet {
                    DownloadState::Stalled
                } else {
                    DownloadState::Downloading
                }
            } else if !has_meta {
                DownloadState::Untracked
            } else if meta.uninstalled() {
                DownloadState::Uninstalled
            } else if meta.installed() {
                DownloadState::Installed
            } else {
                DownloadState::Ready
            };
            let row = DownloadRow {
                name,
                path: dest,
                // Show the eventual size while it is arriving, so the number does
                // not creep upward in the Size column while the bar already says
                // how far along it is.
                size: if partial && total != 0 {
                    total
                } else {
                    md.len()
                },
                version: meta.version().unwrap_or_default(),
                mod_name: meta.mod_name(),
                mod_id: meta.mod_id(),
                state,
                downloaded: md.len(),
                total,
                speed: None,
                hidden: meta.removed(),
                modified,
            };
            Some((row, modified))
        })
        .collect();
    let mut rows: Vec<DownloadRow> = entries.into_iter().map(|(r, _)| r).collect();
    // Hidden rows are dropped BEFORE the cap, so hiding a hundred stale archives
    // actually reveals the ones underneath rather than shortening the list.
    if !app.dl_show_hidden {
        rows.retain(|r| !r.hidden);
    }
    let q = app.dl_filter.trim().to_lowercase();
    if !q.is_empty() {
        // The friendly mod name too, not just the file name: an archive called
        // `SkyUI_5_2_SE-12604-5-2SE.7z` is found by typing "skyui" only if the
        // sidecar's modName is searched as well.
        rows.retain(|r| {
            r.name.to_lowercase().contains(&q)
                || r.mod_name
                    .as_deref()
                    .is_some_and(|m| m.to_lowercase().contains(&q))
        });
    }
    match app.dl_sort {
        DownloadSort::Newest => rows.sort_by_key(|r| std::cmp::Reverse(r.modified)),
        // By what the ROW SHOWS, which is the friendly mod name when there is
        // one: sorting by the archive file name puts `SkyUI_5_2_SE-12604.7z`
        // under S while the row reads "SkyUI", and a list ordered by something
        // invisible reads as unordered.
        DownloadSort::Name => rows.sort_by_key(|r| {
            r.mod_name
                .as_deref()
                .unwrap_or(r.name.as_str())
                .to_lowercase()
        }),
        DownloadSort::Size => rows.sort_by_key(|r| std::cmp::Reverse(r.size)),
        // Within a state, newest first - so the secondary order is the one the
        // list has always had rather than whatever read_dir happened to return.
        DownloadSort::State => rows.sort_by_key(|r| (r.state as u8, std::cmp::Reverse(r.modified))),
    }
    rows.truncate(SAVES_LIST_CAP);

    // Speed is a derivative: compare each in-flight row against the previous
    // sample. A first sighting has no rate yet and says so rather than showing a
    // zero, which would read as "stopped".
    let now = std::time::Instant::now();
    let mut samples = HashMap::new();
    for r in rows
        .iter_mut()
        .filter(|r| r.state == DownloadState::Downloading)
    {
        if let Some((then, bytes)) = app.download_samples.get(&r.name) {
            let secs = now.duration_since(*then).as_secs_f64();
            // Guard both ends: a tick that arrives too close carries no signal,
            // and a partial that SHRANK means the transfer restarted from zero
            // (a server that ignored our Range), which is not a negative speed.
            if secs > 0.05 && r.downloaded >= *bytes {
                r.speed = Some((r.downloaded - *bytes) as f64 / secs);
            }
        }
        samples.insert(r.name.clone(), (now, r.downloaded));
    }
    // Only in-flight rows are kept, so the map cannot grow without bound.
    app.download_samples = samples;
    app.downloads = rows;
    // A collection member turns from "missing" to "downloaded" when its archive
    // lands, and "Try to fetch missing" produces exactly that - without this the
    // pane keeps saying "missing" for files that are already on disk, and the
    // asked-set means clicking again does nothing.
    if app.collection.is_some() {
        crate::update::recompute_collection_states(app);
    }
}
