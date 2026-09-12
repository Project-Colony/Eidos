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
    target: Option<CollectionTarget>,
    identity: Option<eidos_conflicts::ArchiveIdentity>,
    cancel: Arc<AtomicBool>,
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

pub(crate) fn read(path: &Path) -> Preview {
    use std::os::unix::fs::OpenOptionsExt;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let limit = if IMAGES.contains(&ext.as_str()) || ext == "dds" || ext == "nif" {
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
            return Err("Preview input exceeds 128 MiB".into());
        }
        Ok((bytes, truncated))
    })();
    match read {
        Ok((data, truncated)) => from_bytes(path, data, truncated),
        Err(e) => unsupported(path, e),
    }
}

pub(crate) fn from_bytes(path: &Path, bytes: Vec<u8>, truncated: bool) -> Preview {
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
        return unsupported(path,"NIF model viewing is unavailable for this file. Use a matching preview extension or Reveal.");
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
    static NEXT: AtomicU64 = AtomicU64::new(1);
    app.preview_pending.take();
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));
    app.preview_pending = Some(Pending {
        id,
        path: path.clone(),
        target: target(app),
        identity: eidos_conflicts::archive_identity(&path).ok(),
        cancel: cancel.clone(),
    });
    let old = app.preview.clone();
    if selection.is_none() {
        app.preview = Some(unsupported(&path, "Reading…"));
    }
    let addons = app.addons.clone();
    let context = crate::addon_context(app);
    let game = crate::selected_game(app)
        .map(|g| g.def.id.to_string())
        .unwrap_or_default();
    Task::perform(
        async move {
            if cancel.load(Ordering::Relaxed) {
                return unsupported(&path, "Cancelled");
            }
            if let Some(operation) = extension {
                return extension_preview(&path, id, operation, &game, &addons, &context, &cancel)
                    .unwrap_or_else(|e| unsupported(&path, e));
            }
            if let Some(source) = archive {
                let result = eidos_conflicts::read_archive_member_checked(
                    &source.path,
                    &source.member,
                    dds_preview::MAX_DDS_BYTES as u64,
                    Some(&source.identity),
                );
                return match result {
                    Ok(bytes) => Preview::Archive {
                        content: Box::new(from_bytes(Path::new(&source.member), bytes, false)),
                        source,
                    },
                    Err(e) => unsupported(&path, e.to_string()),
                };
            }
            if let (Some(selection), Some(old)) = (selection, old) {
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
            crate::build_preview(&path)
        },
        move |preview| Message::PreviewReady(id, preview),
    )
}

pub(crate) fn complete(app: &mut App, id: u64, preview: Preview) {
    let Some(pending) = app.preview_pending.as_ref().filter(|p| p.id == id) else {
        return;
    };
    let current = pending.target == target(app)
        && pending.path == preview.path()
        && pending.identity == eidos_conflicts::archive_identity(&pending.path).ok();
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
                let mut preview = read(&file);
                match &mut preview {
                    Preview::Image { path: origin, .. }
                    | Preview::Text { path: origin, .. }
                    | Preview::Dds { path: origin, .. }
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
