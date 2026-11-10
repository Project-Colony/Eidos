//! The "Nexus page" tab of the mod information dialog: the mod's description
//! and changelog from Nexus as plain text, under Endorse, Track and Refresh.

use super::*;

pub(crate) const PAGE_ENDORSE_W: f32 = 200.0;
pub(crate) const PAGE_TRACK_W: f32 = 140.0;
pub(crate) const PAGE_REFRESH_W: f32 = 120.0;

/// Starts the fetch unless the page is cached (a failed answer is asked again)
/// or already on its way. Same rule as [`fetch_nexus_files`].
pub(crate) fn fetch_nexus_page(app: &mut App, domain: String, mod_id: u64) -> Task<Message> {
    let key = (domain.clone(), mod_id);
    if matches!(app.nexus_pages.get(&key), Some(Ok(_)))
        || app.nexus_page_loading.as_ref() == Some(&key)
    {
        return Task::none();
    }
    app.nexus_page_loading = Some(key);
    let d = domain.clone();
    Task::perform(
        smol::unblock(move || eidos_nexus::Nexus::connect()?.mod_page(&d, mod_id)),
        move |result| Message::NexusPageFetched {
            domain: domain.clone(),
            mod_id,
            result,
        },
    )
}

/// A fixed-width slot, so a flag that changes a label never moves the next one.
fn slot<'a>(label: &'a str, msg: Message, width: f32) -> Element<'a, Message> {
    container(
        button(
            text(label)
                .size(sz(12))
                .wrapping(iced::widget::text::Wrapping::None),
        )
        .width(Length::Fill)
        .padding(6)
        .style(button::secondary)
        .on_press(msg),
    )
    .width(Length::Fixed(scaled(width)))
    .into()
}

pub(crate) fn info_nexus_page<'a>(app: &App, i: usize, m: &ModEntry) -> Element<'a, Message> {
    let Some(key) = nexus_files_target(app, i) else {
        return Space::new().into();
    };
    let (endorsed, tracked) = app
        .created
        .as_ref()
        .map(|inst| {
            let meta = inst.mod_meta(&m.name);
            (meta.endorsed(), meta.tracked())
        })
        .unwrap_or_default();
    let buttons = Row::new()
        .spacing(8)
        .push(slot(
            t(if endorsed {
                "nexus_page_abstain"
            } else {
                "nexus_page_endorse"
            }),
            Message::ModEndorse(i),
            PAGE_ENDORSE_W,
        ))
        .push(slot(
            t(if tracked {
                "nexus_page_untrack"
            } else {
                "nexus_page_track"
            }),
            Message::ModTrack(i),
            PAGE_TRACK_W,
        ))
        .push(slot(
            t("nexus_page_refresh"),
            Message::NexusPageRefresh,
            PAGE_REFRESH_W,
        ));
    let col = Column::new().spacing(6).push(buttons);
    let line = |s: String| text(s).size(sz(12));
    let page = match app.nexus_pages.get(&key) {
        Some(Ok(page)) => page,
        Some(Err(e)) if app.nexus_page_loading.as_ref() != Some(&key) => {
            return col
                .push(line(tf("nexus_page_failed", &[("error", e)])))
                .into();
        }
        _ => return col.push(line(t("nexus_page_loading").into())).into(),
    };
    if let Some(reason) = page.hidden {
        return col.push(line(reason.message().into())).into();
    }
    let heading = |key: &str| text(t(key)).size(sz(14));
    let mut col = col
        .push(heading("nexus_page_description"))
        .push(line(if page.description.is_empty() {
            t("nexus_page_no_description").into()
        } else {
            page.description.clone()
        }))
        .push(heading("nexus_page_changelog"));
    if page.changelog.is_empty() {
        col = col.push(line(t("nexus_page_no_changelog").into()));
    }
    for (version, lines) in &page.changelog {
        col = col.push(line(version.clone()));
        for l in lines {
            col = col.push(line(format!("• {l}")));
        }
    }
    col.push(text(t("nexus_page_note")).size(sz(11)).color(text_muted()))
        .into()
}
