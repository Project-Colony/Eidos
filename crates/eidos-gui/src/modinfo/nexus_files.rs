//! The "Nexus files" tab of the mod information dialog: the file list of the
//! mod's Nexus page, with the file installed here and the one that replaces it.

use super::*;
use eidos_nexus::{decide_update, file_page_url, ModFiles, NexusFile};

pub(crate) struct FileRow {
    pub(crate) file: NexusFile,
    pub(crate) installed: bool,
    pub(crate) successor: bool,
}

/// Newest upload first. `installed` holds the `(mod id, file id)` pairs the
/// mod's meta.ini records.
pub(crate) fn file_rows(
    files: &ModFiles,
    mod_id: u64,
    installed: &[(u64, u64)],
    page_version: &str,
) -> Vec<FileRow> {
    let ids: Vec<u64> = installed
        .iter()
        .filter(|(m, _)| *m == mod_id)
        .map(|(_, f)| *f)
        .collect();
    let successor = decide_update(page_version, files, &ids).successor;
    let mut rows: Vec<FileRow> = files
        .files
        .iter()
        .map(|f| FileRow {
            installed: ids.contains(&f.file_id),
            successor: successor == Some(f.file_id),
            file: f.clone(),
        })
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.file.uploaded_timestamp));
    rows
}

/// The Nexus domain and mod id of the mod at row `i`, when it has both.
pub(crate) fn nexus_files_target(app: &App, i: usize) -> Option<(String, u64)> {
    let inst = app.created.as_ref()?;
    let m = app.mods.get(i).filter(|m| !m.is_separator())?;
    let meta = inst.mod_meta(&m.name);
    let game = meta.game_name().filter(|g| !g.is_empty());
    let domain = selected_game(app)?.def.nexus_domain_for(game.as_deref())?;
    Some((domain.to_string(), meta.mod_id()?))
}

/// Starts the fetch unless the list is cached (a failed answer is asked again)
/// or already on its way.
pub(crate) fn fetch_nexus_files(app: &mut App, domain: String, mod_id: u64) -> Task<Message> {
    let key = (domain.clone(), mod_id);
    if matches!(app.nexus_files.get(&key), Some(Ok(_)))
        || app.nexus_files_loading.as_ref() == Some(&key)
    {
        return Task::none();
    }
    app.nexus_files_loading = Some(key);
    let d = domain.clone();
    Task::perform(
        smol::unblock(move || {
            let nexus = eidos_nexus::Nexus::connect()?;
            let info = nexus.mod_info(&d, mod_id)?;
            nexus.mod_files(&info.gate, &d, mod_id)
        }),
        move |result| Message::NexusFilesFetched {
            domain: domain.clone(),
            mod_id,
            result,
        },
    )
}

fn cell<'a>(s: impl iced::widget::text::IntoFragment<'a>, w: f32) -> Element<'a, Message> {
    text(s).size(sz(12)).width(Length::Fixed(w)).into()
}

pub(crate) fn info_nexus_files<'a>(app: &App, i: usize, m: &ModEntry) -> Element<'a, Message> {
    let Some((domain, mod_id)) = nexus_files_target(app, i) else {
        return Space::new().into();
    };
    let key = (domain.clone(), mod_id);
    let files = match app.nexus_files.get(&key) {
        Some(Ok(files)) => files,
        Some(Err(e)) if app.nexus_files_loading.as_ref() != Some(&key) => {
            return text(tf("nexus_files_failed", &[("error", e)]))
                .size(sz(12))
                .into();
        }
        _ => return text(t("nexus_files_loading")).size(sz(12)).into(),
    };
    let meta = app.created.as_ref().map(|inst| inst.mod_meta(&m.name));
    let installed = meta
        .as_ref()
        .map(|m| m.installed_files())
        .unwrap_or_default();
    let version = meta.and_then(|m| m.version()).unwrap_or_default();
    let mut col = Column::new().spacing(6);
    for r in file_rows(files, mod_id, &installed, &version) {
        let mark = if r.installed {
            t("nexus_file_installed")
        } else if r.successor {
            t("nexus_file_replaces_yours")
        } else {
            ""
        };
        let when =
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(r.file.uploaded_timestamp);
        col = col.push(
            Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(cell(r.file.name.clone(), 150.0))
                .push(cell(
                    t(&format!("nexus_file_cat_{}", r.file.category_id)),
                    80.0,
                ))
                .push(cell(r.file.version.clone(), 60.0))
                .push(cell(
                    format!("{:.1} MiB", r.file.size_in_bytes as f64 / 1_048_576.0),
                    70.0,
                ))
                .push(cell(format_mtime(when), 110.0))
                .push(cell(mark, 90.0))
                .push(tool_btn(
                    t("nexus_file_open"),
                    Message::OpenUrl(file_page_url(&domain, mod_id, r.file.file_id)),
                )),
        );
    }
    col.into()
}
