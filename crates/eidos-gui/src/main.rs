//! Eidos GUI (iced) - MO2-style wizard + two-pane main window.
//!
//!   Welcome -> Instance type (portable/global) -> Game -> Name/location
//!           -> Summary -> [create] -> Main (MO2-style mod manager)
//!
//! The main window mirrors Mod Organizer 2: menu bar + toolbar + profile row,
//! left = the mod list (enable, priority, reorder) with an Overwrite entry,
//! right = Run + Data/Saves/Downloads tabs, plus a status bar. Colony parchment
//! / burgundy palette. Run with: `cargo run -p eidos-gui`

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{
    button, checkbox, container, image, mouse_area, operation, pick_list, scrollable, text,
    text_input, tooltip, Column, Row, Space, Stack,
};
// For `widget::Id`, the shared handle every operation addresses a widget by.
use iced::widget;
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use eidos_conflicts::{ConflictMap, ConflictState, Layer};
use eidos_games::{detect, home, DetectedGame};
use eidos_instance::settings::Settings;
use eidos_instance::{ExportScope, Instance, InstanceKind, ModEntry, SaveEntry, Tool};
use eidos_plugins::{GameSpec, MovableRange, PluginList};

// The GUI, split by what each part does rather than by what it is about.
//
// `update` decides, everything else draws. `app` and `message` are the data they
// share; `theme` and `widgets` are leaves that the drawing modules share. This
// file only starts the program: every module reaches the shared names through
// the imports below.
mod anim;
mod app;
mod archive_conflicts;
mod background_work;
mod dds_preview;
mod dialogs;
mod file_preview;
mod fomod;
#[cfg(test)]
mod gui_verification;
mod health;
mod installers;
mod message;
mod modinfo;
mod nif_preview;
mod nif_render;
mod preferences;
mod state;
mod subscription;
#[cfg(test)]
mod test_support;
mod theme;
mod update;
mod view;
mod widgets;
mod wizard;

use app::*;
use archive_conflicts::*;
use dialogs::*;
use file_preview::{Preview, PREVIEW_TEXT_CAP};
use fomod::{fomod_ink_faint, fomod_ink_soft, fomod_wizard_view, FomodWizard};
use message::Message;
use modinfo::*;
use preferences::*;
use state::*;
use subscription::subscription;
use update::*;
use view::*;
use wizard::*;

fn main() -> iced::Result {
    // Steam passes the Proton command as our arguments via `eidos-gui %command%`.
    let launch_command: Vec<String> = std::env::args().skip(1).collect();
    // Onto the ecosystem's layout - `~/.config/Colony/Eidos` - before anything
    // reads a setting, and before the log opens, so the log already lands in
    // the folder it will be read from. Copies rather than moves, runs once, and
    // cannot fail a launch: see `eidos_paths::migrate_legacy_layout`.
    let moved = eidos_paths::migrate_legacy_layout();
    // Its own rotation bucket, distinct from the CLI's: the window and an
    // `eidos` child are separate processes writing the same directory, and
    // sharing a bucket would let one rotate the other's session away.
    let _ =
        eidos_log::init_with(eidos_log::Config::new("gui").with_version(env!("CARGO_PKG_VERSION")));
    // Held until exit: no other Eidos moves an instance this one has resolved.
    let (instances, _instances_held) = eidos_transfer::migrate_global_instances();
    // Logged rather than silent, because a user who goes looking for their
    // settings or their mods deserves to find out from the log where they went.
    for note in moved.into_iter().chain(instances) {
        eidos_log::info!("{note}");
    }
    eidos_log::info!("eidos-gui {} starting", env!("CARGO_PKG_VERSION"));
    // The title moved out of `application` and onto a builder; the first argument
    // is now the boot function that `run_with` used to take. It must be `Fn`, not
    // `FnOnce` - which is why the `.clone()` stays: without it the closure would
    // consume the Vec and only be callable once.
    iced::application(move || new(launch_command.clone()), update, view)
        .title("Eidos")
        .theme(theme::theme)
        .subscription(subscription)
        .window(window_settings())
        .run()
}

/// The desktop identity of the window. MUST equal the basename of the installed
/// `eidos.desktop`, because that pairing is the only thing tying the two
/// together.
pub const APP_ID: &str = "eidos";

/// How the window introduces itself to the desktop.
///
/// Without `application_id` a Wayland surface announces an EMPTY app id, so the
/// compositor has nothing to match against a desktop entry and a taskbar shows a
/// placeholder tile no matter how many icons are installed. That was the actual
/// symptom; the icon files were never the missing part.
///
/// The embedded icon covers X11 and XWayland, where the icon travels with the
/// window instead of being looked up from a desktop file - so the binary is
/// self-sufficient even with nothing installed. It is the dark-ground tile
/// rather than the transparent mark: the mark is pale ink, which disappears
/// against a light panel. A decode failure costs the icon, never the launch.
fn window_settings() -> iced::window::Settings {
    // The size the user left it at, when they asked for that. Read here rather
    // than passed in, because the window is built before the App exists.
    let prefs = Settings::load();
    let size = prefs
        .remember_window
        .then_some(prefs.window_size)
        .flatten()
        // A stored size larger than any plausible screen, or smaller than the
        // toolbar, is a size the window cannot be used at - fall back rather
        // than open something the user has to fix before they can fix it.
        .filter(|&(w, h)| (480..=16384).contains(&w) && (360..=16384).contains(&h))
        .map(|(w, h)| iced::Size::new(w as f32, h as f32));
    iced::window::Settings {
        size: size.unwrap_or(iced::window::Settings::default().size),
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: APP_ID.to_string(),
            ..Default::default()
        },
        icon: iced::window::icon::from_file_data(
            include_bytes!("../../../assets/brand/png/eidos-icon-256-on-dark.png"),
            None,
        )
        .ok(),
        ..Default::default()
    }
}
