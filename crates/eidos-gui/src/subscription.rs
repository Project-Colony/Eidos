//! What the window listens to besides clicks: the keyboard, the pointer, the
//! window itself, dropped files, and the timers that run only while something
//! they watch is actually happening.

use crate::*;

/// How often the Saves tab checks whether the directory changed. Slower than the
/// downloads poll: a save appears when the player makes one, which is not a
/// progress bar anybody is watching tick.
const SAVES_TICK: std::time::Duration = std::time::Duration::from_millis(2500);

/// How often the open log pane re-reads its file. Slower than the downloads
/// tick: a log is read, not watched, and re-parsing half a megabyte twice a
/// second to catch a line that is not there yet is pure waste.
const LOG_TAIL_TICK: std::time::Duration = std::time::Duration::from_millis(1500);

/// How often the downloads directory is re-scanned while something is arriving.
/// Fast enough that a progress bar moves rather than jumps, slow enough that it
/// is a rounding error next to the transfer itself.
const DOWNLOAD_TICK: std::time::Duration = std::time::Duration::from_millis(500);

/// The same, when nothing is in flight: something has to notice that a download
/// STARTED, and that something cannot be the download itself - it runs in
/// another process, launched by the browser, with no way to reach this one.
const DOWNLOAD_IDLE_TICK: std::time::Duration = std::time::Duration::from_secs(2);

/// Keyboard subscription: surface global shortcuts and keep the live modifier state
/// in sync so a plain mod-row click can branch to Ctrl-toggle / Shift-extend.
///
/// Shortcuts only fire on the main screen, and only when no modal / inline editor is
/// stealing input, so they never clobber typing into a text field. Mirrors MO2's
/// global accelerators: F5 (Refresh) and Ctrl+R (Run).
pub(crate) fn subscription(app: &App) -> iced::Subscription<Message> {
    use iced::keyboard::{self, key::Named, Key};

    // 60 Hz, and ONLY while something is actually moving. An idle window
    // subscribes to nothing here, so the cost of having animations at all is
    // zero rather than small - which is the whole reason this is a condition
    // and not an always-on timer that most frames ignore.
    // One timer, two reasons to want it: something is animating, or an archive
    // is extracting on a worker thread and its dialog has to repaint (and its
    // completion be noticed). iced 0.14 requires a non-capturing closure here,
    // so the tick always carries `AnimationTick` and the handler forwards to
    // `InstallPoll` when there is a job - which also keeps that message
    // separately testable.
    let frames = anim::needs_frames(app).then(|| {
        iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::AnimationTick)
    });

    // Track held modifiers from every key press AND release (a release with no
    // remaining keys still carries the updated modifier set).
    // One stream now: `listen` yields every keyboard event and all three variants
    // carry the modifier set, so press and release no longer need separate
    // subscriptions. ModifiersChanged also reaches us for the first time - no
    // widget captures it - which means the held set no longer goes stale while a
    // text field has the caret.
    let track = keyboard::listen().map(|event| match event {
        keyboard::Event::KeyPressed { modifiers, .. }
        | keyboard::Event::KeyReleased { modifiers, .. }
        | keyboard::Event::ModifiersChanged(modifiers) => Message::ModifiersChanged(modifiers),
    });

    // Where the pointer is, and how big the window is. Needed because iced's
    // right-press carries no coordinates, so a context menu cannot otherwise be
    // placed where it was summoned from. The handlers do nothing but store.
    let pointer = iced::event::listen_with(|event, _status, _window| match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::PointerAt(position))
        }
        // A release ANYWHERE ends a drag. `mouse_area::on_release` only fires
        // while the cursor is over its bounds, so without this a drag let go
        // outside the list stayed armed and the next click moved the mod. It
        // used to be handled by cancelling on pointer EXIT, which cost far more
        // than it bought: dragging upward past the header to scroll dropped the
        // mod every time, because leaving and letting go are indistinguishable
        // to that callback.
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::PointerReleased)
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
        }
        // Files dragged in from a file manager. NOT AVAILABLE ON WAYLAND: winit
        // 0.30 implements XDND for X11 and has no `wl_data_device` at all, so
        // these never fire on a native Wayland session (they do under XWayland).
        // Wired anyway - it costs three arms, it works on X11 today, and it
        // starts working on Wayland the day winit grows the protocol. The
        // Downloads-to-priority drag above is the path that works everywhere,
        // and it is the one the UI points at.
        iced::Event::Window(iced::window::Event::FileHovered(_)) => {
            Some(Message::FilesHovering(true))
        }
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
            Some(Message::FilesHovering(false))
        }
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(path))
        }
        _ => None,
    });

    // App shortcuts. `on_key_press` takes a plain `fn`, so it cannot read `app`;
    // the handlers themselves no-op off the main screen / while a modal is open.
    let shortcuts = keyboard::listen().filter_map(|event| {
        let keyboard::Event::KeyPressed {
            key,
            modifiers: mods,
            ..
        } = event
        else {
            return None;
        };
        match key.as_ref() {
            Key::Named(Named::F5) => Some(Message::Refresh),
            // Ctrl+R launches the current run target (MO2's Run accelerator).
            Key::Character("r") if mods.control() => Some(Message::Run),
            Key::Named(Named::Escape) => Some(Message::ClearSelection),
            Key::Character("a") if mods.control() || mods.command() => {
                Some(Message::SelectAllInFocus)
            }
            // Ctrl+C over the LOOT report copies it whole. `update` no-ops when the
            // report is not open, since this closure cannot see the app.
            Key::Character("c") if mods.control() || mods.command() => {
                Some(Message::CopyLootReport)
            }
            // Navigation. Which list answers is decided in `update` - this closure
            // is a plain `fn` and cannot see the app.
            Key::Named(Named::Tab) => Some(Message::CycleFocus),
            // Ctrl moves the ROW; plain moves the focus. Checked first, or the
            // plain arms below would swallow it.
            Key::Named(Named::ArrowUp) if mods.control() || mods.command() => {
                Some(Message::KeyNav(Nav::ShiftUp))
            }
            Key::Named(Named::ArrowDown) if mods.control() || mods.command() => {
                Some(Message::KeyNav(Nav::ShiftDown))
            }
            Key::Named(Named::ArrowUp) => Some(Message::KeyNav(Nav::Up)),
            Key::Named(Named::ArrowDown) => Some(Message::KeyNav(Nav::Down)),
            Key::Named(Named::PageUp) => Some(Message::KeyNav(Nav::PageUp)),
            Key::Named(Named::PageDown) => Some(Message::KeyNav(Nav::PageDown)),
            Key::Named(Named::Home) => Some(Message::KeyNav(Nav::First)),
            Key::Named(Named::End) => Some(Message::KeyNav(Nav::Last)),
            Key::Named(Named::Space) => Some(Message::KeyNav(Nav::Toggle)),
            Key::Named(Named::Enter) => Some(Message::KeyNav(Nav::Activate)),
            Key::Named(Named::Delete) => Some(Message::KeyNav(Nav::Remove)),
            // Ctrl+F puts the caret in the filter box - the one shortcut everybody
            // tries first in a list this long.
            Key::Character("f") if mods.control() || mods.command() => Some(Message::FocusFilter),
            // A bare letter jumps to the next mod starting with it, the way every
            // desktop list has since before Explorer. Checked LAST so it can never
            // shadow a modified shortcut, and only for a single character with no
            // Ctrl/Alt held - the `typing` gate below keeps it out of text fields.
            Key::Character(c)
                if !mods.control() && !mods.command() && !mods.alt() && c.chars().count() == 1 =>
            {
                c.chars()
                    .next()
                    .filter(|ch| ch.is_alphanumeric())
                    .map(Message::JumpToLetter)
            }
            _ => None,
        }
    });

    // The shortcut stream is gated on the main screen (the wizard/FOMOD views have
    // their own focus); modifier tracking always runs so the set is never stale.
    // Navigation keys are suppressed while a field has the caret; the always-safe
    // ones (F5, Ctrl+R, Escape, Ctrl+A) keep working, and Escape is what gets the
    // keyboard back out of a field.
    let typing = app.typing;
    let shortcuts = shortcuts.with(typing).map(|(typing, m)| match m {
        // JumpToLetter joins them: a bare letter belongs to the list only when
        // no field has the caret, or typing "f" into the filter box would jump
        // the list instead of filtering it.
        Message::KeyNav(_) | Message::CycleFocus | Message::JumpToLetter(_) if typing => {
            Message::Noop
        }
        other => other,
    });

    let mut subs = vec![track, pointer];
    // Archive analysis only reports completion; it does not animate at frame rate.
    if app.archive_job.is_some() {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::ArchivePoll),
        );
    }
    if app.screen == Screen::Main
        && app.installer.is_none()
        && app.fomod.is_none()
        && app.rename.is_none()
        && !app.settings_open
        && app.executables.is_none()
        && app.collision.is_none()
        && app.info_mod.is_none()
        // Don't fire shortcuts (especially Ctrl+R) while the GUI is locked behind a
        // running game or a LOOT report is open. An unlocked tracked run keeps them.
        && app.running.as_ref().is_none_or(|r| !r.lock)
        && app.loot_report.is_none()
        // Every other overlay that owns the screen. A navigation key reaching
        // the mod list from behind one of these moves a selection the user
        // cannot see, and Space would toggle a mod they are not looking at.
        && !app.about_open
        && !app.view_menu_open
        && app.picker.is_none()
        && app.profile_menu.is_none()
        && app.profile_rename.is_none()
        && app.profile_copy.is_none()
        && app.profile_delete_confirm.is_none()
        && app.send_priority.is_none()
        && app.overwrite_to_mod.is_none()
        && app.menu_mod.is_none()
        && app.menu_plugin.is_none()
        // The panes added since. The INI editor above all: it owns the keyboard
        // outright, and an arrow key reaching the mod list behind it moves a
        // selection nobody can see while Space toggles a mod they are not
        // looking at.
        && app.ini_editor.is_none()
        && app.log_pane.is_none()
        && app.categories_dialog.is_none()
        && app.backups.is_none()
        && app.executables.is_none()
        && !app.addons_open
        // The three added since. The export dialog and the instance manager own
        // the screen, and the File dropdown is a menu like the View one - a
        // Delete reaching the mod list from behind any of them arms a removal on
        // a row nobody is looking at.
        && app.export.is_none()
        && !app.instances_open
        && !app.file_menu_open
        // And the two newest overlays. Both own the whole screen behind a scrim,
        // and Delete is the key that matters: two presses through a preview pane
        // arm and then commit a removal, which takes a mod OFF DISK, with the
        // list it happened to hidden behind the pane.
        && app.preview.is_none()
        && app.collection.is_none()
        // Pack and unpack, and the job itself - which runs for twenty minutes
        // behind a modal, long enough that Ctrl+R launching the game and Delete
        // arming a mod removal on a row nobody can see are not hypotheses.
        && app.pack.is_none()
        && app.unpack.is_none()
        && app.transfer_job.is_none()
        // And the extraction, which was never in this list. Tolerable while it
        // was thirty seconds; it is the same defect as the rest, and it is one
        // clause.
        && app.install_job.is_none()
    {
        subs.push(shortcuts);
    }
    // While the LOOT report modal is up, the main shortcut stream above is
    // deliberately off - nothing may walk the lists behind it. But two keys mean
    // something OVER the report: Escape dismisses it and Ctrl+C copies it, and
    // both were promised (the dialog footer says so) yet dead, because their
    // only producer was the stream this modal switches off. A dedicated stream
    // for exactly those two keys, mutually exclusive with the main one.
    if app.screen == Screen::Main && app.loot_report.is_some() {
        let report_keys = keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed {
                key,
                modifiers: mods,
                ..
            } = event
            else {
                return None;
            };
            match key.as_ref() {
                Key::Named(Named::Escape) => Some(Message::CloseLootReport),
                Key::Character("c") if mods.control() || mods.command() => {
                    Some(Message::CopyLootReport)
                }
                _ => None,
            }
        });
        subs.push(report_keys);
    }
    // Watch the downloads directory while its tab is open. Polling is not a
    // shortcut taken for want of something better: the transfer runs in a
    // separate `eidos nxm` process spawned by the BROWSER, so there is no handle
    // to await and no channel to listen on. The filesystem is the interface, and
    // a directory of a few dozen entries is cheap to read twice a second.
    //
    // Faster while something is arriving, so a bar moves instead of jumping;
    // slower otherwise, because the idle case only has to notice that a download
    // has begun.
    if app.tab == Tab::Downloads {
        let arriving = app
            .downloads
            .iter()
            .any(|d| d.state == DownloadState::Downloading);
        let period = if arriving {
            DOWNLOAD_TICK
        } else {
            DOWNLOAD_IDLE_TICK
        };
        subs.push(iced::time::every(period).map(|_| Message::DownloadTick));
    }
    // Watch the saves directory while its tab is open. The game writes there
    // from inside the Proton prefix while Eidos is not looking, so an autosave
    // made mid-session would otherwise never appear until a manual Refresh.
    //
    // The tick compares a FINGERPRINT and only reloads when it moved: rebuilding
    // the list twice a second would drop the selection and close the details
    // pane under the user's hands.
    if app.tab == Tab::Saves {
        subs.push(iced::time::every(SAVES_TICK).map(|_| Message::SavesTick));
    }
    // Tail the session log while its pane is open. Same reasoning as the
    // downloads tick: the records worth reading are written by a SEPARATE
    // `eidos` process, so there is nothing to await - the file is the interface.
    // Subscribed only while the pane is up, so a closed pane costs nothing.
    if app.log_pane.is_some() {
        subs.push(iced::time::every(LOG_TAIL_TICK).map(|_| Message::LogRefresh));
    }
    // Auto-scroll while a drag rests on an edge band. Subscribed only then, so an
    // idle drag - or no drag at all - schedules nothing.
    // Hover-to-expand, subscribed only while a drag actually rests on a
    // collapsed group - so an idle window, and a drag over ordinary rows,
    // schedule nothing.
    if app.drag_state.is_some() && app.drag_hover_group.is_some() {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(300))
                .map(|_| Message::DragHoverTick),
        );
    }
    if app.drag_scroll.is_some() {
        subs.push(
            // 40ms rather than 60: the step is applied whole, so a slower tick
            // buys the same speed only by jumping further each time, and a jump
            // is what the user reads as the list misbehaving.
            iced::time::every(std::time::Duration::from_millis(40))
                .map(|_| Message::DragScrollTick),
        );
    }
    // While waiting on a launched game/tool, poll for its exit so we can unlock.
    if app.running.is_some() {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(600)).map(|_| Message::PollRunning),
        );
    }
    // Pushed last so the condition sits beside every other conditional timer
    // above, all of which follow the same rule: subscribe only while the thing
    // they watch is actually happening.
    subs.extend(frames);
    iced::Subscription::batch(subs)
}
