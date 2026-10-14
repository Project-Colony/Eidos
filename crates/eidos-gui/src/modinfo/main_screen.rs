//! The main window around the tabs: the tab strip, the right pane, the status
//! bar, the send-to and profile menus.

use super::*;

/// How selected a main-strip tab should look right now.
///
/// Split out so the eight call sites below say which tab they are and nothing
/// else - threading the phase, the previous tab and the motion preference
/// through each of them is how one of the eight ends up different.
fn main_mix(app: &App, this: Tab) -> f32 {
    crate::anim::tab_mix(
        crate::anim::at(app, &app.tab_anim),
        &app.tab,
        app.tab_prev.as_ref(),
        &this,
    )
}

/// One tab of the main strip.
///
/// `mix` is how selected it should LOOK, 0.0 to 1.0, rather than whether it is:
/// mid-transition the arriving tab and the one being left behind are both
/// somewhere in between. A window with motion off only ever passes 0.0 or 1.0,
/// and then this draws exactly what it drew before animation existed.
pub(crate) fn tab_btn<'a>(label: String, t: Tab, mix: f32) -> Element<'a, Message> {
    button(text(label).size(sz(12)))
        .padding(6)
        .on_press(Message::SelectTab(t))
        // Both ends are iced's own styles, resolved against the live theme, so
        // no colour is named here and the blend follows the palette.
        .style(move |theme, status| {
            crate::anim::mix_button(
                button::secondary(theme, status),
                button::primary(theme, status),
                mix,
            )
        })
        .into()
}

pub(crate) fn right_pane<'a>(app: &App) -> Element<'a, Message> {
    // Run-target picker (MO2's executables combo): the game, or any tool run
    // through the same merged view. The game's launcher/binary + script extender are
    // auto-detected as tools, so they show up here alongside the user's tools.
    // Pinned first, hidden not at all. A game's defaults include tools somebody
    // may never use, and a picker listing eight entries to reach the second is a
    // picker nobody reads. The order within each half is left alone - that is
    // the order tools.ini has, which the user controls.
    let mut listed: Vec<&Tool> = app.tools.iter().filter(|t| !t.hidden).collect();
    listed.sort_by_key(|t| !t.pinned);
    let run_options: Vec<String> = std::iter::once(run_game_label().to_string())
        .chain(listed.iter().map(|t| t.title.clone()))
        .collect();
    let run_choice = app
        .tool_choice
        .clone()
        .unwrap_or_else(|| run_game_label().to_string());

    let top = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text(t("main_run_label")).size(sz(13)))
        .push(
            pick_list(run_options, Some(run_choice), Message::ToolPicked)
                .text_size(sz(13))
                .padding(8),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            button(
                Row::new()
                    .spacing(6)
                    .push(icon(IC_RUN, 18.0))
                    .push(text(t("main_run")).size(sz(15))),
            )
            .padding(10)
            .on_press(Message::Run)
            .style(button::primary),
        );

    // The tab in force, which is not always `app.tab`: see `effective_tab`.
    let tab = effective_tab(app);
    let mut tabs = Row::new().spacing(4).push(tab_btn(
        t("tab_data").to_string(),
        Tab::Data,
        main_mix(app, Tab::Data),
    ));
    // Only for a game whose plugins Eidos actually manages. Stellar Blade is the
    // first game with no plugin system at all, and every other pane keys off the
    // same `GameSpec::for_id` - so without this the tab is there, opens, and
    // shows an empty list for a game that will never have one.
    if game_manages_plugins(app) {
        tabs = tabs.push(tab_btn(
            t("tab_plugins").to_string(),
            Tab::Plugins,
            main_mix(app, Tab::Plugins),
        ));
    }
    let tabs = tabs
        .push(tab_btn(
            t("tab_conflicts").to_string(),
            Tab::Conflicts,
            main_mix(app, Tab::Conflicts),
        ))
        .push(tab_btn(
            t("tab_overwrite").to_string(),
            Tab::Overwrite,
            main_mix(app, Tab::Overwrite),
        ))
        .push(tab_btn(
            t("tab_archives").to_string(),
            Tab::Archives,
            main_mix(app, Tab::Archives),
        ))
        .push(tab_btn(
            t("tab_saves").to_string(),
            Tab::Saves,
            main_mix(app, Tab::Saves),
        ))
        .push(tab_btn(
            t("tab_downloads").to_string(),
            Tab::Downloads,
            main_mix(app, Tab::Downloads),
        ))
        .push(tab_btn(
            diagnostics_tab_label(app),
            Tab::Diagnostics,
            main_mix(app, Tab::Diagnostics),
        ));

    let content = match tab {
        Tab::Data => data_panel(app),
        Tab::Plugins => plugins_panel(app),
        Tab::Conflicts => conflicts_panel(app),
        Tab::Overwrite => overwrite_panel(app),
        Tab::Archives => archives_panel(app),
        Tab::Saves => saves_panel(app),
        Tab::Downloads => downloads_panel(app),
        Tab::Diagnostics => diagnostics_panel(app),
    };

    let inner = Column::new().spacing(8).push(top).push(tabs).push(content);
    let portion = 1000_u16.saturating_sub((app.split * 1000.0) as u16);
    container(inner)
        .width(Length::FillPortion(portion))
        .height(Length::Fill)
        .padding(8)
        .style(panel_style)
        .into()
}

pub(crate) fn status_bar<'a>(app: &App) -> Element<'a, Message> {
    let kind = match app.kind {
        InstanceKind::Global => t("instance_kind_global"),
        InstanceKind::Portable => t("instance_kind_portable"),
    };
    let game = selected_game(app).map(|g| g.def.name).unwrap_or(t("status_bar_instance"));
    // A live multi-selection count takes the left slot (MO2's "N selected"), unless a
    // transient status message is showing; otherwise the instance summary.
    let showing_status = app.status.is_some();
    let left = if let Some(s) = app.status.clone() {
        s
    } else if app.selected_mods.len() > 1 {
        tf("mod_menu_n_selected", &[("n", &app.selected_mods.len())])
    } else {
        let profile = app
            .created
            .as_ref()
            .map(|i| i.active().name)
            .unwrap_or_else(|| t("profile_default").to_string());
        format!("{game} - {kind} - {profile}")
    };
    // The Nexus account, if connected this session (MO2's status-bar login state).
    // The tier is always spelled out: showing "(Premium)" only when premium made
    // a free account look like an account whose tier had not been checked yet -
    // and the difference is not cosmetic, since a free account cannot fetch a
    // download link without the key/expires pair from a fresh nxm:// link.
    let account = match &app.nexus_account {
        Some(a) => tf(
            "status_bar_nexus",
            &[
                ("name", &a.name),
                (
                    "tier",
                    &if a.is_premium {
                        t("settings_tier_premium")
                    } else {
                        t("settings_tier_free")
                    },
                ),
            ],
        ),
        None => t("status_bar_not_logged_in").to_string(),
    };
    // The left slot fades in when its message changes, so a status that
    // replaces another is visibly a NEW one rather than a word that quietly
    // became a different word. Colour only - the row keeps its height whatever
    // the fade is doing, so the status bar never nudges the panes above it.
    let fg = crate::theme::palette().text;
    let ink = crate::anim::mix(
        Color { a: 0.0, ..fg },
        fg,
        crate::anim::at(app, &app.status_anim),
    );
    let mut row = Row::new()
        .align_y(iced::Alignment::Center)
        .push(text(left).size(sz(11)).color(ink).width(Length::Fill));
    if showing_status {
        // A tiny dismiss so a stale message stops masking the selection count and
        // instance summary.
        row = row.push(
            button(text("x").size(sz(10)))
                .padding([0, 6])
                .on_press(Message::ClearStatus)
                .style(button::text),
        );
    }
    row = row.push(text(account).size(sz(11)));
    container(row)
        .width(Length::Fill)
        .padding(4)
        .style(bar_style)
        .into()
}

pub(crate) fn main_screen(app: &App) -> Element<'_, Message> {
    // The name is the way into Preferences, and the way out: the same button,
    // the same message as the page's Close, showing whether the page is open.
    let header = Row::new()
        .spacing(10)
        .push(colony_ui::widgets::identity_button(
            &colony_ui::Typography::current(),
            "Eidos",
            20,
            app.settings_open,
            Message::TogglePreferences,
        ))
        .push(Space::new().width(Length::Fill))
        .push(tool_btn(t("main_new_instance"), Message::Restart));

    // A grabbable divider rather than a gap. The panes were a fixed 3:2, which is
    // a guess about somebody else's screen: on a narrow window the right pane's
    // tab strip runs out of room and clips "Diagnostics", with nothing to do
    // about it. Six pixels wide because a hairline is hard to hit and a bar is
    // furniture; it lights up while held so the drag is visibly in progress.
    let held = app.split_drag;
    let divider = mouse_area(
        container(Space::new().width(Length::Fixed(6.0)).height(Length::Fill))
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(if held {
                    crate::theme::divider_held()
                } else {
                    crate::theme::divider()
                })),
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .on_press(Message::SplitGrab);

    // Preferences REPLACE the content area - they are not a modal, a separate
    // window or a popover, which the Colony convention names as the three things
    // this must not be. The header, the menu bar, the toolbar and the status bar
    // all stay exactly where they are: the program's chrome does not move
    // because the user went to configure it.
    let body: Element<'_, Message> = if app.settings_open {
        preferences_page(app)
    } else {
        Row::new()
            .spacing(4)
            .height(Length::Fill)
            .push(modlist_pane(app))
            .push(divider)
            .push(right_pane(app))
            .into()
    };

    let mut base = Column::new()
        .spacing(4)
        .padding(4)
        .push(header)
        .push(menu_bar(app));
    if app.ui_toolbar_visible {
        base = base.push(toolbar(app));
    }
    // Persistent warning while the eidos binary lacks CAP_SYS_ADMIN: launches
    // still work but FUSE passthrough is off and SKSE plugin DLLs may fail to
    // load. Every rebuild wipes the capability, so this fires often enough that
    // silence cost real debugging time.
    if app.cap_missing && passthrough_requested() {
        base = base.push(cap_warning_banner());
    }
    base = base.push(body);
    if app.ui_statusbar_visible {
        base = base.push(status_bar(app));
    }

    let mut layers = Stack::new().push(base);

    // The right-click action menu floats over the window (MO2's context menu).
    // A full-window catcher behind it dismisses on a click outside the card.
    if let Some(i) = app.menu_mod {
        if i < app.mods.len() {
            let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Message::CloseMenu);
            let at = app.menu_at.unwrap_or(app.cursor);
            let card = floating_at(mod_menu_card(app, i), at, app.window);
            layers = layers.push(catcher).push(card);
        }
    }

    // The per-mod info dialog is a centered modal (MO2's modinfodialog).
    if let Some(i) = app.info_mod {
        if i < app.mods.len() {
            let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Message::CloseInfo);
            let dialog = container(mod_info_dialog(app, i)).center(Length::Fill);
            layers = layers.push(scrim).push(dialog);
        }
    }

    // The extraction dialog. No scrim catcher: there is nothing to cancel yet -
    // 7-Zip is mid-archive - and a click that dismissed it would leave the job
    // running with nothing watching for its result.
    if let Some(job) = &app.install_job {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill));
        layers = layers
            .push(scrim)
            .push(container(install_progress_dialog(job)).center(Length::Fill));
    }

    // A pack or unpack, and afterwards its report. Above the extraction dialog
    // because it is the longer job of the two and the one that must not be
    // covered; no scrim catcher either, for the same reason as the extraction
    // and one more - while it is FINISHED, a stray click would throw away the
    // report before it had been read.
    if let Some(job) = &app.transfer_job {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill));
        let card = container(mouse_area(transfer_dialog(job)).on_press(Message::Noop))
            .center(Length::Fill);
        layers = layers.push(scrim).push(card);
    }

    // The manual / BAIN picker (MO2's InstallDialog and BainComplexInstallerDialog).
    // Below the collision chooser in the stack: a collision raised BY the picker
    // has to be the thing you can click.
    if let Some(p) = &app.picker {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::PickerCancel);
        layers = layers
            .push(scrim)
            .push(container(install_picker_dialog(p)).center(Length::Fill));
    }

    // The install-collision chooser is a centered modal (MO2's QueryOverwriteDialog).
    if let Some(c) = &app.collision {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CollisionCancel);
        let dialog = container(collision_dialog(c)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Executables editor (MO2's Modify Executables dialog).
    if let Some(state) = &app.executables {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseExecutablesDialog);
        let dialog = container(executables_dialog(app, state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The filter pane, hanging under its button.
    if app.filters_open {
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ToggleFilterPane)
            .on_right_press(Message::ToggleFilterPane);
        // Hangs under the Filters button rather than floating at the cursor:
        // it is a panel belonging to a control, not a context menu. Its old
        // hardcoded corner was wrong twice over - the button sits behind a
        // Length::Fill search box inside a resizable pane, so its x moves with
        // the window width AND with the split divider.
        let card = dropdown_under(
            filter_pane(app),
            app.filters_at.unwrap_or(FALLBACK_MENU_ANCHOR),
            app.window,
        );
        layers = layers.push(catcher).push(card);
    }

    // The plugin context menu, floating where it was summoned from.
    // Bounds-checked: a stale row would otherwise render an invisible
    // full-window click catcher over an empty card.
    let plugin_menu_row = app
        .menu_plugin
        .filter(|&i| app.plugins.as_ref().is_some_and(|l| i < l.plugins.len()));
    if let Some(i) = plugin_menu_row {
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ClosePluginMenu)
            .on_right_press(Message::ClosePluginMenu);
        // From the frozen anchor, not the live cursor.
        let at = app.menu_at.unwrap_or(app.cursor);
        let card = floating_at(plugin_menu_card(app, i), at, app.window);
        layers = layers.push(catcher).push(card);
    }

    // The Backups dialog (MO2's Create Backup / Restore Backup).
    if let Some(state) = &app.backups {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseBackupsDialog);
        let dialog = container(backups_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Categories dialog (MO2's Change Categories).
    if let Some(state) = &app.categories_dialog {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseCategoriesDialog);
        let dialog = container(categories_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Extensions list.
    if app.addons_open {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseAddons);
        let dialog = container(addons_dialog(app)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The log pane (MO2's dockable log view).
    if let Some(state) = &app.log_pane {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseLogPane);
        let dialog = container(log_pane_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The INI editor (MO2's bundled tool plugin).
    if let Some(state) = &app.ini_editor {
        // No scrim dismiss: the card holds an editable buffer, and a stray click
        // outside it must not be how unsaved edits are thrown away. The catcher
        // is still there to swallow the click.
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::Noop)
            .on_right_press(Message::Noop);
        let dialog = container(ini_editor_dialog(app, state)).center(Length::Fill);
        layers = layers.push(catcher).push(dialog);
    }

    // The About box (Help menu).
    if app.about_open {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseAbout);
        let dialog = container(about_dialog()).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // A file is hovering over the window: say the drop will be taken. The flag
    // was already tracked and read by nothing, so on X11 the gesture had no
    // feedback at all - the user could not tell the window from any other.
    if app.files_hovering {
        let banner = container(
            container(text(t("main_drop_to_install")).size(sz(15)))
                .padding([10, 20])
                .style(|t: &Theme| container::Style {
                    background: Some(Background::Color(t.extended_palette().primary.base.color)),
                    text_color: Some(t.extended_palette().primary.base.text),
                    border: Border {
                        width: 0.0,
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill);
        // No catcher: a hover is not a click, and swallowing presses here would
        // freeze the window for as long as the pointer carried a file.
        layers = layers.push(banner);
    }

    // The preview pane.
    if let Some(p) = &app.preview {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ClosePreview);
        let dialog = container(preview_dialog(p)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The collection browser.
    if let Some(state) = &app.collection {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseCollection);
        let dialog = container(collection_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The instance manager (MO2's Manage Instances).
    if app.instances_open {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseInstanceManager);
        let dialog = container(instances_dialog(app)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // Pack and unpack. Their cards swallow their own presses: both carry a
    // block of computed text somebody will click on while reading it, and a
    // `container` does not take mouse events, so without this that click falls
    // through to the scrim and dismisses what they were reading.
    if let Some(state) = &app.pack {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ClosePackDialog);
        let dialog = container(mouse_area(pack_dialog(app, state)).on_press(Message::Noop))
            .center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }
    if let Some(state) = &app.unpack {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseUnpackDialog);
        let dialog = container(mouse_area(unpack_dialog(state)).on_press(Message::Noop))
            .center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Export dialog (MO2's Export to csv).
    if let Some(state) = &app.export {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseExportDialog);
        let dialog = container(export_dialog(app, state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The File dropdown floats just under the menu bar, at the File item.
    if app.file_menu_open {
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseFileMenu);
        let card = dropdown_under(
            file_menu_card(app),
            app.file_menu_at.unwrap_or(FALLBACK_MENU_ANCHOR),
            app.window,
        );
        layers = layers.push(catcher).push(card);
    }

    // The View dropdown floats just under the menu bar, near the View item.
    if app.view_menu_open {
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseViewMenu);
        let card = dropdown_under(
            view_menu_card(app),
            app.view_menu_at.unwrap_or(FALLBACK_MENU_ANCHOR),
            app.window,
        );
        layers = layers.push(catcher).push(card);
    }

    // The per-profile menu (rename / copy / delete), opened by right-clicking a
    // profile chip. A catcher behind it dismisses on an outside click.
    if let Some(name) = app.profile_menu.clone() {
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ProfileCloseMenu);
        let at = app.menu_at.unwrap_or(app.cursor);
        let card = floating_at(profile_menu_card(app, &name), at, app.window);
        layers = layers.push(catcher).push(card);
    }

    // The LOOT report (MO2's post-sort dialog): a centered modal listing general
    // messages + per-plugin missing masters / messages / dirty advice.
    if let Some(report) = &app.loot_report {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseLootReport);
        // The card swallows its own presses. A `container` does not take mouse
        // events, so without this a click anywhere ON the report fell through to
        // the scrim behind it and dismissed the thing the user was reading.
        let dialog = container(mouse_area(loot_report_dialog(report)).on_press(Message::Noop))
            .center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The run lock (MO2's "lock GUI while the application runs"): a full-window
    // overlay that blocks everything beneath it until the game exits or the user
    // clicks Unlock. Added last so it sits on top of every other layer. A tracked
    // run with `lock` off (setting disabled, or force-unlocked) shows no overlay.
    if let Some(run) = app.running.as_ref().filter(|r| r.lock) {
        // A backdrop that swallows EVERY pointer event (press/release/right/scroll)
        // so nothing beneath it is reachable - clicks, context menus and the modlist
        // scroll wheel are all inert while locked. `interaction` also tells the Stack
        // to mark lower layers unavailable for scroll.
        let scrim = mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| {
                container::Style {
                    background: Some(shade(0.55).into()),
                    ..Default::default()
                }
            }),
        )
        .on_press(Message::Noop)
        .on_release(Message::Noop)
        .on_right_press(Message::Noop)
        .on_scroll(|_| Message::Noop)
        .interaction(iced::mouse::Interaction::NotAllowed);
        let dialog = container(running_lock_card(run)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    layers.into()
}

/// MO2's targeted "Send to" actions, below the blunt top/bottom pair.
///
/// The two conflict-relative moves are the ones people actually reach for -
/// "put this just above the mod it is overriding" - and both are gated on the
/// relevant set being non-empty, so the menu never offers a move that would do
/// nothing. Priority and separator open an inline editor rather than a modal,
/// matching how rename already works in this menu.
/// The separators offered as a destination when sending row `i` (and whatever
/// else is selected with it) into a group.
///
/// The moved rows are excluded. MO2 does not bother, because in a flat list
/// sending a separator into its own group is merely nonsensical rather than
/// unsound - it lands at the tail of the group it used to head. Offering the
/// user a choice whose only outcome is confusion is not parity worth having.
pub(crate) fn separator_choices(app: &App, i: usize) -> Vec<usize> {
    let moving = selection_or(app, i);
    (0..app.mods.len())
        .filter(|&idx| app.mods[idx].is_separator())
        .filter(|idx| !moving.contains(idx))
        .collect()
}

pub(crate) fn send_to_targets<'a>(app: &App, i: usize) -> Element<'a, Message> {
    // Same origin convention as the emblems: index + 1, with the game (0) and the
    // Overwrite pseudo-layer (u32::MAX) excluded because they are not rows.
    let real = |set: &std::collections::BTreeSet<u32>| set.iter().any(|&o| o != 0 && o != u32::MAX);
    let mc = app
        .conflicts
        .as_ref()
        .and_then(|m| m.mods.get(&((i + 1) as u32)));
    let mut col = Column::new().spacing(1);

    if let Some((row, text)) = app.send_priority.as_ref().filter(|(r, _)| *r == i) {
        let _ = row;
        col = col.push(
            text_input(t("send_priority"), text)
                .on_input(Message::SendToPriorityChanged)
                .on_submit(Message::SendToPriorityCommit)
                .padding(5)
                .size(sz(12)),
        );
        return col.into();
    }
    if app.send_separator == Some(i) {
        // An inline chooser of the separators, scrollable because a big load
        // order has plenty of them.
        // No spacing: the insertion strips below provide the separation, and they
        // must be part of the flow so the layout is identical with and without a drag.
        let mut list = Column::new();
        for idx in separator_choices(app, i) {
            // Owned, so the Element does not borrow from `app`.
            let label = app.mods[idx].display_name().to_string();
            list = list.push(menu_item_owned(label, Message::SendToSeparatorPick(idx)));
        }
        col = col
            .push(text(t("send_move_into_group")).size(sz(11)))
            .push(scrollable(list).height(Length::Fixed(160.0)))
            .push(menu_item(t("common_cancel"), Message::SendToTargetCancel));
        return col.into();
    }

    if mc.is_some_and(|m| real(&m.overwrites)) {
        col = col.push(menu_item(
            t("send_above_first_conflict"),
            Message::SendToFirstConflict(i),
        ));
    }
    if mc.is_some_and(|m| real(&m.overwritten_by)) {
        col = col.push(menu_item(
            t("send_below_last_conflict"),
            Message::SendToLastConflict(i),
        ));
    }
    col = col
        .push(menu_item(
            t("view_send_to_priority"),
            Message::SendToPriorityStart(i),
        ))
        .push(menu_item(
            t("send_to_separator"),
            Message::SendToSeparatorStart(i),
        ));
    col.into()
}

/// The per-profile context menu (MO2's profile manager actions), opened by
/// right-clicking a profile chip: rename, copy-to-new, delete (two-click confirm).
pub(crate) fn profile_menu_card<'a>(app: &App, name: &str) -> Element<'a, Message> {
    let title = Row::new()
        .spacing(6)
        .push(
            text(tf("profile_menu_title", &[("name", &name)]))
                .size(sz(13))
                .width(Length::Fill),
        )
        .push(
            button(text("x").size(sz(13)))
                .padding([1, 6])
                .on_press(Message::ProfileCloseMenu)
                .style(button::text),
        );
    let mut col = Column::new().spacing(1).push(title).push(menu_sep());

    // Rename: an inline editor when armed, else a menu item that arms it.
    match &app.profile_rename {
        Some((orig, edited)) if orig == name => {
            col = col.push(
                text_input(t("mod_menu_new_name"), edited)
                    .on_input(Message::ProfileRenameChanged)
                    .on_submit(Message::ProfileRenameCommit)
                    .padding(5)
                    .size(sz(12)),
            );
        }
        _ => {
            col = col.push(menu_item(
                t("common_rename"),
                Message::ProfileRenameStart(name.to_string()),
            ))
        }
    }

    // Copy to a new profile: an inline editor when armed, else a menu item.
    match &app.profile_copy {
        Some((src, edited)) if src == name => {
            col = col.push(
                text_input(t("profile_copy_name_field"), edited)
                    .on_input(Message::ProfileCopyChanged)
                    .on_submit(Message::ProfileCopyCommit)
                    .padding(5)
                    .size(sz(12)),
            );
        }
        _ => {
            col = col.push(menu_item(
                t("profile_copy_to_new"),
                Message::ProfileCopyStart(name.to_string()),
            ))
        }
    }

    col = col.push(menu_sep());
    // Take over an existing MO2 profile's mod order + load order, so a migrating
    // user does not re-tick dozens of mods and plugins by hand.
    col = col.push(menu_item(t("profile_import_mo2"), Message::ImportMo2Pick));

    col = col.push(menu_sep());
    // Delete: two-click confirm (backend refuses the active / last profile).
    let delete: Element<'a, Message> = if app.profile_delete_confirm.as_deref() == Some(name) {
        button(text(t("profile_click_again_delete")).size(sz(12)))
            .padding([2, 6])
            .width(Length::Fill)
            .on_press(Message::ProfileDeleteCommit(name.to_string()))
            .style(button::danger)
            .into()
    } else {
        menu_item(t("common_delete"), Message::ProfileDeleteConfirm(name.to_string()))
    };
    col = col.push(delete);

    // The same shared frame as the other five, for the same reason.
    menu_frame(col.into())
}

/// Suggest a free profile name near `base` (`base`, `base 2`, `base 3`, ...) so the
/// copy editor never starts on a name that already collides.
pub(crate) fn suggest_free_profile_name(inst: &Instance, base: &str) -> String {
    if !inst.profile(base).dir().exists() {
        return base.to_string();
    }
    (2..1000)
        .map(|n| format!("{base} {n}"))
        .find(|cand| !inst.profile(cand).dir().exists())
        .unwrap_or_else(|| base.to_string())
}

/// Suggest a free mod-folder name near `name` (`name (2)`, `name (3)`, ...) for the
/// Rename option, so the prefilled value doesn't immediately collide again.
pub(crate) fn suggest_free_name(mods_dir: &std::path::Path, name: &str) -> String {
    if !mods_dir.join(name).exists() {
        return name.to_string();
    }
    (2..1000)
        .map(|n| format!("{name} ({n})"))
        .find(|cand| !mods_dir.join(cand).exists())
        .unwrap_or_else(|| name.to_string())
}
