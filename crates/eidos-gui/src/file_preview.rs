//! Shared bounded byte previews for loose files, archives and trusted helpers.
use crate::{dds_preview, App, CollectionTarget, Message, Preview, PREVIEW_TEXT_CAP};
use eidos_addons::protocol::{Operation, Outcome, Payload, Request};
use iced::{widget::image::Handle, Task};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

const IMAGES: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp", "ico", "tga"];

#[derive(Debug)]
pub(crate) struct Pending {
    pub id: u64,
    path: PathBuf,
    snapshot: Snapshot,
    cancel: Arc<AtomicBool>,
}

/// Provenance of decoded bytes, retained when a control reuses cached content.
#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    target: Option<CollectionTarget>,
    identity: Option<eidos_conflicts::ArchiveIdentity>,
    epoch: u64,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn target(app: &App) -> Option<CollectionTarget> {
    let inst = app.created.as_ref()?;
    Some(CollectionTarget {
        instance: inst.root.clone(),
        profile: inst.active_profile(),
        installation: crate::selected_game(app)?.selection_id(),
    })
}

pub(crate) fn unsupported(path: &Path, why: impl Into<String>) -> Preview {
    Preview::Unsupported {
        path: path.into(),
        why: why.into(),
    }
}

#[cfg(test)]
pub(crate) fn read(path: &Path) -> Preview {
    read_cancel(path, &AtomicBool::new(false))
}

fn read_cancel(path: &Path, cancel: &AtomicBool) -> Preview {
    use std::os::unix::fs::OpenOptionsExt;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let limit = if ext == "nif" {
        crate::nif_preview::MAX_MODEL_BYTES
    } else if IMAGES.contains(&ext.as_str()) || ext == "dds" {
        dds_preview::MAX_DDS_BYTES
    } else {
        PREVIEW_TEXT_CAP
    };
    let read = (|| -> Result<(Vec<u8>, bool), String> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| e.to_string())?;
        let md = file.metadata().map_err(|e| e.to_string())?;
        if !md.is_file() {
            return Err("This is not a regular file".into());
        }
        let mut bytes = Vec::new();
        file.take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let truncated = bytes.len() > limit;
        bytes.truncate(limit);
        if truncated && limit != PREVIEW_TEXT_CAP {
            return Err(format!("Preview input exceeds {} MiB", limit / 1024 / 1024));
        }
        Ok((bytes, truncated))
    })();
    match read {
        Ok((data, truncated)) => from_bytes_cancel(path, data, truncated, cancel),
        Err(e) => unsupported(path, e),
    }
}

#[cfg(test)]
pub(crate) fn from_bytes(path: &Path, bytes: Vec<u8>, truncated: bool) -> Preview {
    from_bytes_cancel(path, bytes, truncated, &AtomicBool::new(false))
}

fn from_bytes_cancel(path: &Path, bytes: Vec<u8>, truncated: bool, cancel: &AtomicBool) -> Preview {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ext == "dds" {
        return match dds_preview::inspect(&bytes) {
            Ok(info) => dds(
                path,
                Arc::new(bytes),
                info,
                dds_preview::Selection::default(),
            ),
            Err(error) => unsupported(
                path,
                format!("Cannot read DDS: {error}. Reveal opens its source location."),
            ),
        };
    }
    if IMAGES.contains(&ext.as_str()) {
        return match image(&bytes, &ext) {
            Ok(handle) => Preview::Image {
                path: path.into(),
                handle,
            },
            Err(error) => unsupported(path, error),
        };
    }
    if ext == "nif" {
        return crate::nif_preview::from_bytes(path, &bytes, cancel);
    }
    if bytes.contains(&0) {
        return unsupported(path, "This is a binary file; no text preview is available.");
    }
    let was_truncated = truncated || bytes.len() > PREVIEW_TEXT_CAP;
    let bytes = &bytes[..bytes.len().min(PREVIEW_TEXT_CAP)];
    Preview::Text {
        path: path.into(),
        body: String::from_utf8_lossy(bytes).into_owned(),
        truncated: was_truncated,
    }
}

fn image(bytes: &[u8], extension: &str) -> Result<Handle, String> {
    use image_decoder::ImageDecoder;
    let mut reader = image_decoder::ImageReader::new(std::io::Cursor::new(bytes));
    let format =
        image_decoder::ImageFormat::from_extension(extension).ok_or("unsupported image format")?;
    reader.set_format(format);
    let mut limits = image_decoder::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (width, height) = decoder.dimensions();
    if decoder.total_bytes() > dds_preview::MAX_RGBA_BYTES as u64
        || u64::from(width) * u64::from(height) * 4 > dds_preview::MAX_RGBA_BYTES as u64
    {
        return Err("Decoded image exceeds 64 MiB".into());
    }
    let rgba = image_decoder::DynamicImage::from_decoder(decoder)
        .map_err(|e| e.to_string())?
        .into_rgba8();
    Ok(Handle::from_rgba(width, height, rgba.into_raw()))
}

fn dds(
    path: &Path,
    bytes: Arc<Vec<u8>>,
    info: dds_preview::DdsInfo,
    selection: dds_preview::Selection,
) -> Preview {
    let (info, selection, image) = match dds_preview::decode(&bytes, selection) {
        Ok(decoded) => (
            decoded.info,
            decoded.selection,
            Ok(Handle::from_rgba(
                decoded.width,
                decoded.height,
                decoded.rgba,
            )),
        ),
        Err(error) => (info, selection, Err(error)),
    };
    Preview::Dds {
        path: path.into(),
        provenance: None,
        bytes,
        info,
        selection,
        image,
    }
}

/// Snapshot instance, profile, file identity and a unique request before work.
pub(crate) fn start(
    app: &mut App,
    path: PathBuf,
    extension: Option<Operation>,
    selection: Option<dds_preview::Selection>,
    archive: Option<crate::archive_conflicts::MemberSource>,
) -> Task<Message> {
    start_control(app, path, extension, selection.map(Control::Dds), archive)
}

enum Control {
    Dds(dds_preview::Selection),
    Nif(crate::nif_render::View),
}

pub(crate) fn start_nif(
    app: &mut App,
    path: PathBuf,
    view: crate::nif_render::View,
) -> Task<Message> {
    if let Some(preview) = &mut app.preview {
        crate::nif_preview::requested_view(preview, view);
    }
    start_control(app, path, None, Some(Control::Nif(view)), None)
}

fn start_control(
    app: &mut App,
    path: PathBuf,
    extension: Option<Operation>,
    selection: Option<Control>,
    archive: Option<crate::archive_conflicts::MemberSource>,
) -> Task<Message> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    app.preview_pending.take();
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));
    let old = app.preview.clone();
    // Cached pixels keep the identity of the bytes originally decoded. A new
    // mip/channel request must not relabel old bytes with a replacement's identity.
    let snapshot = if selection.is_some() {
        old.as_ref()
            .and_then(cached_snapshot)
            .cloned()
            .unwrap_or(Snapshot {
                target: target(app),
                epoch: app.archive_epoch.get(),
                identity: None,
            })
    } else {
        Snapshot {
            target: target(app),
            epoch: app.archive_epoch.get(),
            identity: archive
                .as_ref()
                .map(|s| s.identity.clone())
                .or_else(|| eidos_conflicts::archive_identity(&path).ok()),
        }
    };
    let identity = snapshot.identity.clone();
    app.preview_pending = Some(Pending {
        id,
        path: path.clone(),
        snapshot,
        cancel: cancel.clone(),
    });
    if selection.is_none() {
        app.preview = Some(unsupported(&path, "Reading…"));
    }
    let addons = app.addons.clone();
    let context = crate::addon_context(app);
    let game = crate::selected_game(app)
        .map(|g| g.def.id.to_string())
        .unwrap_or_default();
    Task::perform(
        crate::background_work::run(move || {
            if cancel.load(Ordering::Relaxed) {
                return unsupported(&path, "Cancelled");
            }
            if let Some(operation) = extension {
                return extension_preview(&path, id, operation, &game, &addons, &context, &cancel)
                    .unwrap_or_else(|e| unsupported(&path, e));
            }
            if let Some(source) = archive {
                let limit = if Path::new(&source.member)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("nif"))
                {
                    crate::nif_preview::MAX_MODEL_BYTES
                } else {
                    dds_preview::MAX_DDS_BYTES
                };
                let result = eidos_conflicts::read_archive_member_checked(
                    &source.path,
                    &source.member,
                    limit as u64,
                    Some(&source.identity),
                );
                return match result {
                    Ok(bytes) => Preview::Archive {
                        content: Box::new(from_bytes_cancel(
                            Path::new(&source.member),
                            bytes,
                            false,
                            &cancel,
                        )),
                        source,
                    },
                    Err(e) => unsupported(&path, e.to_string()),
                };
            }
            if let (Some(selection), Some(old)) = (selection, old) {
                if identity.is_none() || identity != eidos_conflicts::archive_identity(&path).ok() {
                    return unsupported(&path, "The preview source changed. Open it again.");
                }
                let selection = match selection {
                    Control::Nif(view) => {
                        return crate::nif_preview::change_view(old, view, &cancel)
                    }
                    Control::Dds(selection) => selection,
                };
                return match old {
                    Preview::Dds { bytes, info, .. } => dds(&path, bytes, info, selection),
                    Preview::Archive { source, content } => match *content {
                        Preview::Dds { bytes, info, .. } => Preview::Archive {
                            content: Box::new(dds(
                                Path::new(&source.member),
                                bytes,
                                info,
                                selection,
                            )),
                            source,
                        },
                        _ => unsupported(&path, "The selected archive member is not a DDS texture"),
                    },
                    _ => unsupported(&path, "The selected file is not a DDS texture"),
                };
            }
            read_cancel(&path, &cancel)
        }),
        move |preview| Message::PreviewReady(id, preview),
    )
}

fn cached_snapshot(preview: &Preview) -> Option<&Snapshot> {
    match preview {
        Preview::Dds { provenance, .. } | Preview::Nif { provenance, .. } => provenance.as_ref(),
        Preview::Archive { content, .. } => cached_snapshot(content),
        _ => None,
    }
}

fn retain_snapshot(preview: &mut Preview, snapshot: &Snapshot) {
    match preview {
        Preview::Dds { provenance, .. } | Preview::Nif { provenance, .. } => {
            *provenance = Some(snapshot.clone())
        }
        Preview::Archive { content, .. } => retain_snapshot(content, snapshot),
        _ => {}
    }
}

pub(crate) fn is_current(app: &App, id: u64, preview: &Preview) -> bool {
    let Some(pending) = app.preview_pending.as_ref().filter(|p| p.id == id) else {
        return false;
    };
    pending.snapshot.target == target(app)
        && pending.snapshot.epoch == app.archive_epoch.get()
        && pending.path == preview.path()
        && pending.snapshot.identity == eidos_conflicts::archive_identity(&pending.path).ok()
        && crate::nif_preview::dependencies_current(preview)
}

pub(crate) fn pending_cancel(app: &App, id: u64) -> Option<Arc<AtomicBool>> {
    app.preview_pending
        .as_ref()
        .filter(|p| p.id == id)
        .map(|p| p.cancel.clone())
}

pub(crate) fn complete(app: &mut App, id: u64, mut preview: Preview) {
    let Some(pending) = app.preview_pending.as_ref().filter(|p| p.id == id) else {
        return;
    };
    let current = is_current(app, id, &preview);
    retain_snapshot(&mut preview, &pending.snapshot);
    app.preview_pending.take();
    if current {
        app.preview = Some(preview);
    } else {
        app.preview = None;
        app.status =
            Some("Preview discarded because its file, profile or installation changed.".into());
    }
}

fn extension_preview(
    path: &Path,
    id: u64,
    operation: Operation,
    game: &str,
    addons: &[eidos_addons::Addon],
    context: &eidos_addons::Context,
    cancel: &AtomicBool,
) -> Result<Preview, String> {
    let workspace = tempfile::tempdir().map_err(|e| e.to_string())?;
    let request = Request {
        protocol: 1,
        request_id: id.to_string(),
        operation,
        game: game.into(),
        input: path.canonicalize().map_err(|e| e.to_string())?,
        source: None,
        workspace: workspace.path().into(),
        answers: Default::default(),
    };
    for addon in eidos_addons::protocol::matching(addons, &request) {
        let reply = eidos_addons::protocol::invoke(addon, context, &request, cancel)?;
        match reply.outcome {
            Outcome::Declined => continue,
            Outcome::Handled {
                result: Payload::Preview { path: output },
            } => {
                let file = eidos_addons::protocol::regular_file(workspace.path(), &output)?;
                // Decode before deleting the helper workspace; Reveal keeps the original.
                let mut preview = read_cancel(&file, cancel);
                match &mut preview {
                    Preview::Image { path: origin, .. }
                    | Preview::Text { path: origin, .. }
                    | Preview::Dds { path: origin, .. }
                    | Preview::Nif { path: origin, .. }
                    | Preview::Unsupported { path: origin, .. } => *origin = path.into(),
                    Preview::Archive { .. } => {
                        return Err("Unexpected archived helper result".into())
                    }
                }
                return Ok(preview);
            }
            Outcome::Handled {
                result:
                    Payload::SaveInfo {
                        fields,
                        plugins,
                        light_plugins,
                    },
            } => {
                let mut body = format!("{}\n\n", addon.name);
                for field in fields {
                    body.push_str(&format!("{}: {}\n", field.label, field.value));
                }
                body.push_str("\nPlugins:\n");
                for name in plugins.iter().chain(&light_plugins) {
                    body.push_str(name);
                    body.push('\n');
                }
                let truncated = body.len() > PREVIEW_TEXT_CAP;
                while body.len() > PREVIEW_TEXT_CAP {
                    body.pop();
                }
                return Ok(Preview::Text {
                    path: path.into(),
                    body,
                    truncated,
                });
            }
            Outcome::Manual { reason } | Outcome::Failed { message: reason } => return Err(reason),
            Outcome::Cancelled => return Err("Extension cancelled".into()),
            _ => return Err("Unexpected extension reply".into()),
        }
    }
    Err(
        "No installed extension handled this file. Open Extensions to inspect matching rules."
            .into(),
    )
}
