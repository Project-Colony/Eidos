//! Bundled NIF parsing and current-provider texture loading for static previews.
use crate::{archive_conflicts::ProviderSource, dds_preview, nif_render, App, Message, Preview};
use eidos_conflicts::ArchiveIdentity;
use iced::{widget::image::Handle, Task};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub(crate) const MAX_MODEL_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEXTURE_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEXTURE_INPUT: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct Model {
    pub scene: Arc<nif_render::Scene>,
    pub textures: Arc<HashMap<String, nif_render::Texture>>,
    pub view: nif_render::View,
    pub image: Option<Result<Handle, String>>,
    pub warnings: Arc<str>,
    dependencies: Vec<(PathBuf, ArchiveIdentity)>,
}

fn warning_text<'a>(warnings: impl IntoIterator<Item = &'a String>) -> Arc<str> {
    // A valid static mesh can carry thousands of unsupported blocks. Keep the
    // full parser diagnostics in Scene, but bound text layout and share it when
    // a slider clones the preview.
    let mut text = String::new();
    let mut omitted = 0;
    for (index, warning) in warnings.into_iter().enumerate() {
        if index >= 64 {
            omitted += 1;
            continue;
        }
        if !text.is_empty() {
            text.push('\n');
        }
        let mut chars = warning.chars();
        text.extend(chars.by_ref().take(1024));
        if chars.next().is_some() {
            text.push('…');
        }
    }
    if omitted > 0 {
        text.push_str(&format!(
            "\n{omitted} additional warnings are not displayed."
        ));
    }
    text.into()
}

fn helper() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let path = exe.parent()?.join("eidos-nif-preview");
            path.is_file().then_some(path)
        })
        .or_else(|| eidos_addons::which(Path::new("eidos-nif-preview")))
}

fn parse_with_helper(
    bytes: &[u8],
    helper: &Path,
    cancel: &AtomicBool,
) -> Result<nif_render::Scene, String> {
    if bytes.len() > MAX_MODEL_BYTES {
        return Err("NIF model exceeds 64 MiB".into());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled".into());
    }
    let workspace = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = workspace.path().join("model.nif");
    std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let output = eidos_addons::capture_command(
        std::process::Command::new(helper).arg(input),
        Duration::from_secs(15),
        cancel,
        MAX_MODEL_BYTES,
    )?;
    if !output.status.success() {
        return Err(format!(
            "NIF helper failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
                .trim()
                .chars()
                .take(2048)
                .collect::<String>()
        ));
    }
    nif_render::parse(&output.stdout)
}

pub(crate) fn from_bytes(path: &Path, bytes: &[u8], cancel: &AtomicBool) -> Preview {
    let scene = helper().ok_or_else(|| "NIF preview helper is unavailable. Install the complete Eidos package or build the native helper.".to_string())
        .and_then(|helper| parse_with_helper(bytes, &helper, cancel));
    match scene {
        Ok(scene) => {
            let warnings = warning_text(&scene.warnings);
            Preview::Nif {
                path: path.into(),
                provenance: None,
                model: Model {
                    scene: Arc::new(scene),
                    textures: Arc::new(HashMap::new()),
                    view: Default::default(),
                    image: None,
                    warnings,
                    dependencies: Vec::new(),
                },
            }
        }
        Err(error) => crate::file_preview::unsupported(path, error),
    }
}

fn model(preview: &Preview) -> Option<&Model> {
    match preview {
        Preview::Nif { model, .. } => Some(model),
        Preview::Archive { content, .. } => model(content),
        _ => None,
    }
}
fn model_mut(preview: &mut Preview) -> Option<&mut Model> {
    match preview {
        Preview::Nif { model, .. } => Some(model),
        Preview::Archive { content, .. } => model_mut(content),
        _ => None,
    }
}

pub(crate) fn requested_view(preview: &mut Preview, view: nif_render::View) {
    if let Some(model) = model_mut(preview) {
        model.view = view;
    }
}

pub(crate) fn dependencies_current(preview: &Preview) -> bool {
    model(preview).is_none_or(|model| {
        model.dependencies.iter().all(|(path, expected)| {
            eidos_conflicts::archive_identity(path).is_ok_and(|current| current == *expected)
        })
    })
}

/// After parsing reveals the texture names, copy only their selected providers.
/// The complete archive member map stays on the GUI instead of being cloned.
pub(crate) fn prepare_textures(app: &App, id: u64, preview: &Preview) -> Option<Task<Message>> {
    let model = model(preview).filter(|m| m.image.is_none())?;
    if !crate::file_preview::is_current(app, id, preview) {
        return None;
    }
    let cancel = crate::file_preview::pending_cancel(app, id)?;
    let (requests, mut warnings) = texture_requests(app, model);
    let mut preview = preview.clone();
    Some(Task::perform(
        crate::background_work::run(move || {
            let model = model_mut(&mut preview).expect("parsed NIF preview");
            let mut textures = HashMap::new();
            let (mut input_bytes, mut decoded_bytes) = (0usize, 0usize);
            for (key, source) in requests {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let result = read_texture(&source, &mut input_bytes, &mut decoded_bytes);
                match result {
                    Ok((texture, dependency)) => {
                        model.dependencies.push(dependency);
                        textures.insert(key, texture);
                    }
                    Err(error) => warnings.push(format!("{key}: {error}")),
                }
            }
            model.textures = Arc::new(textures);
            model.warnings = warning_text(model.scene.warnings.iter().chain(warnings.iter()));
            render(model, &cancel);
            preview
        }),
        move |preview| Message::PreviewReady(id, preview),
    ))
}

fn texture_requests(app: &App, model: &Model) -> (Vec<(String, ProviderSource)>, Vec<String>) {
    let mut requests = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    let map = app
        .conflicts
        .as_ref()
        .filter(|_| app.archive_completed_epoch == Some(app.archive_epoch.get()));
    for mesh in &model.scene.meshes {
        let Some(name) = mesh.material.textures.first().filter(|s| !s.is_empty()) else {
            continue;
        };
        let key = match nif_render::normalize_texture_path(name) {
            Ok(key) => key,
            Err(error) => {
                warnings.push(format!("{name}: {error}"));
                continue;
            }
        };
        if !seen.insert(key.clone()) {
            continue;
        }
        // ponytail: 64 diffuse textures per model; raise only with a measured
        // memory/latency budget. Missing textures retain neutral shading.
        if seen.len() > 64 {
            warnings.push("Only the first 64 diffuse textures are loaded.".into());
            break;
        }
        let source = map
            .ok_or(
                "Texture providers are unavailable; refresh archive analysis and reopen the model",
            )
            .and_then(|map| {
                let node = map
                    .asset_files
                    .get(&key)
                    .ok_or("Texture is missing from the active providers")?;
                if node.precedence_uncertain {
                    warnings.push(format!(
                        "{key}: provider order is uncertain; using the displayed candidate"
                    ));
                }
                crate::archive_conflicts::resolve_provider(app, &node.display_path, &node.winner)
                    .map_err(|_| "Texture provider changed or is unavailable")
            });
        match source {
            Ok(source) => requests.push((key, source)),
            Err(error) => warnings.push(format!("{key}: {error}")),
        }
    }
    (requests, warnings)
}

fn read_texture(
    source: &ProviderSource,
    input_bytes: &mut usize,
    decoded_bytes: &mut usize,
) -> Result<(nif_render::Texture, (PathBuf, ArchiveIdentity)), String> {
    let remaining = MAX_TEXTURE_INPUT
        .saturating_sub(*input_bytes)
        .min(dds_preview::MAX_DDS_BYTES);
    if remaining == 0 {
        return Err("Model texture input exceeds 256 MiB".into());
    }
    let previous = *input_bytes;
    // Failed archive reads do not report partial bytes, so reserve their whole
    // attempted budget. Successful reads give the unused allowance back.
    *input_bytes += remaining;
    let (path, identity, bytes) = match source {
        ProviderSource::Archive(source) => {
            let bytes = eidos_conflicts::read_archive_member_checked(
                &source.path,
                &source.member,
                remaining as u64,
                Some(&source.identity),
            )
            .map_err(|e| e.to_string())?;
            (source.path.clone(), source.identity.clone(), bytes)
        }
        ProviderSource::Loose(path) => {
            use std::os::unix::fs::OpenOptionsExt;
            let identity = eidos_conflicts::archive_identity(path).map_err(|e| e.to_string())?;
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path)
                .map_err(|e| e.to_string())?;
            if !file.metadata().map_err(|e| e.to_string())?.is_file() {
                return Err("Texture is not a regular file".into());
            }
            let mut bytes = Vec::new();
            file.take(remaining as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > remaining {
                return Err("Model texture input exceeds its limit".into());
            }
            (path.clone(), identity, bytes)
        }
    };
    *input_bytes = previous + bytes.len();
    if eidos_conflicts::archive_identity(&path).ok().as_ref() != Some(&identity) {
        return Err("Texture source changed while reading".into());
    }
    let info = dds_preview::inspect(&bytes)?;
    if info.layers != 1 || info.faces != 1 {
        return Err("Diffuse cube/array textures are unsupported".into());
    }
    let mut selection = dds_preview::Selection::default();
    while (info.width >> selection.mip).max(info.height >> selection.mip) > 4096 {
        if selection.mip + 1 >= info.mips {
            return Err("Diffuse texture exceeds 4096 pixels and has no smaller mip".into());
        }
        selection.mip += 1;
    }
    let required = (info.width >> selection.mip).max(1) as usize
        * (info.height >> selection.mip).max(1) as usize
        * 4;
    if required > MAX_TEXTURE_BYTES.saturating_sub(*decoded_bytes) {
        return Err("Decoded model textures exceed 64 MiB".into());
    }
    let decoded = dds_preview::decode(&bytes, selection)?;
    *decoded_bytes += decoded.rgba.len();
    Ok((
        nif_render::Texture {
            width: decoded.width,
            height: decoded.height,
            rgba: decoded.rgba,
        },
        (path, identity),
    ))
}

fn render(model: &mut Model, cancel: &AtomicBool) {
    model.image = Some(
        nif_render::render(&model.scene, model.view, &model.textures, cancel)
            .map(|frame| Handle::from_rgba(frame.width, frame.height, frame.rgba)),
    );
}

pub(crate) fn change_view(
    mut preview: Preview,
    view: nif_render::View,
    cancel: &AtomicBool,
) -> Preview {
    if !dependencies_current(&preview) {
        return crate::file_preview::unsupported(
            preview.path(),
            "Model textures changed. Open the preview again.",
        );
    }
    if let Some(model) = model_mut(&mut preview) {
        model.view = view;
        render(model, cancel);
    }
    preview
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn fixture_model() -> Model {
        Model {
            scene: Arc::new(
                nif_render::parse(include_bytes!("nif_render_fixtures/sse.json")).unwrap(),
            ),
            textures: Arc::new(HashMap::new()),
            view: Default::default(),
            image: None,
            warnings: Arc::from(""),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn warning_heavy_models_bound_display_text_and_share_it_between_views() {
        let warnings: Vec<_> = (0..19_000)
            .map(|i| format!("{i}: {}", "é".repeat(1100)))
            .collect();
        let mut model = fixture_model();
        model.warnings = warning_text(&warnings);
        assert_eq!(model.warnings.lines().count(), 65);
        assert!(model
            .warnings
            .ends_with("18936 additional warnings are not displayed."));
        assert!(model.warnings.chars().count() < 66 * 1025);
        assert!(Arc::ptr_eq(&model.warnings, &model.clone().warnings));
    }

    fn dds(color: [u8; 4]) -> Vec<u8> {
        let mut bytes = vec![0; 152];
        bytes[..4].copy_from_slice(b"DDS ");
        for (at, value) in [
            (4, 124u32),
            (8, 0x21007),
            (12, 1),
            (16, 1),
            (28, 1),
            (76, 32),
            (80, 4),
            (84, u32::from_le_bytes(*b"DX10")),
            (108, 0x401008),
            (128, 28),
            (132, 3),
            (140, 1),
        ] {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[148..].copy_from_slice(&color);
        bytes
    }

    fn archive(member: &str, bytes: &[u8]) -> Vec<u8> {
        let mut names = member.as_bytes().to_vec();
        names.push(0);
        let mut out = Vec::new();
        for word in [
            0x100u32,
            12 + names.len() as u32,
            1,
            bytes.len() as u32,
            0,
            0,
        ] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend(names);
        out.extend([0; 8]);
        out.extend(bytes);
        out
    }

    #[test]
    fn real_nif_helper_parses_both_native_fixtures_and_rejects_invalid_input() {
        let helper = helper().expect(
            "Build native/eidos-nif-preview and put its output directory on PATH before GUI tests",
        );
        let cancel = AtomicBool::new(false);
        for bytes in [
            include_bytes!("../../../native/eidos-nif-preview/tests/fixtures/le-valid.nif")
                .as_slice(),
            include_bytes!("../../../native/eidos-nif-preview/tests/fixtures/sse-valid.nif")
                .as_slice(),
        ] {
            let scene = parse_with_helper(bytes, &helper, &cancel).unwrap();
            assert_eq!(scene.meshes.len(), 1);
            let image =
                nif_render::render(&scene, Default::default(), &HashMap::new(), &cancel).unwrap();
            assert!(image.rgba.chunks_exact(4).any(|p| p != [24, 27, 34, 255]));
        }
        assert!(parse_with_helper(b"not a model", &helper, &cancel).is_err());
        assert!(
            parse_with_helper(b"unused", &helper, &AtomicBool::new(true))
                .unwrap_err()
                .contains("Cancelled")
        );
    }

    #[test]
    fn model_textures_use_current_loose_winners_then_checked_archive_members() {
        let member = "Textures/Synthetic/Diffuse.dds";
        let (mut app, root) =
            crate::tests::data_app(&[(member, "placeholder")], &[(member, "placeholder")]);
        let loose = root.join("overwrite").join(member);
        let packed = root.join("mods/AAA/textures.bsa");
        std::fs::write(&loose, dds([0, 255, 0, 255])).unwrap();
        std::fs::remove_file(root.join("mods/AAA").join(member)).unwrap();
        std::fs::write(&packed, archive(member, &dds([255, 0, 0, 255]))).unwrap();
        let update_map = |app: &mut App| {
            let parts: Vec<_> = crate::conflict_layers(app)
                .unwrap()
                .into_iter()
                .map(|l| {
                    let files = eidos_conflicts::collect_files(&l.root);
                    (l, files)
                })
                .collect();
            app.conflicts = Some(eidos_conflicts::ConflictMap::build_with_archives_from(
                &parts,
                &[eidos_conflicts::ActiveArchive {
                    name: "textures.bsa".into(),
                    plugin: None,
                    order_uncertain: false,
                }],
            ));
            app.archive_sources.insert(
                packed.clone(),
                eidos_conflicts::archive_identity(&packed).unwrap(),
            );
            app.archive_completed_epoch = Some(app.archive_epoch.get());
        };
        update_map(&mut app);
        let mut model = fixture_model();
        let (requests, warnings) = texture_requests(&app, &model);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(requests.len(), 1);
        assert!(matches!(&requests[0].1, ProviderSource::Loose(path) if *path==loose));
        let (texture, dependency) = read_texture(&requests[0].1, &mut 0, &mut 0).unwrap();
        assert_eq!(texture.rgba, [0, 255, 0, 255]);
        model.dependencies.push(dependency);
        let preview = Preview::Nif {
            path: root.join("mesh.nif"),
            provenance: None,
            model,
        };
        assert!(dependencies_current(&preview));
        std::fs::remove_file(&loose).unwrap();
        assert!(!dependencies_current(&preview));
        assert!(matches!(
            change_view(preview, Default::default(), &AtomicBool::new(false)),
            Preview::Unsupported { .. }
        ));
        update_map(&mut app);
        let (requests, warnings) = texture_requests(&app, &fixture_model());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(matches!(requests[0].1, ProviderSource::Archive(_)));
        let (texture, _) = read_texture(&requests[0].1, &mut 0, &mut 0).unwrap();
        assert_eq!(texture.rgba, [255, 0, 0, 255]);
        std::fs::write(&packed, b"changed archive").unwrap();
        assert!(read_texture(&requests[0].1, &mut 0, &mut 0).is_err());
        assert!(texture_requests(&app, &fixture_model()).0.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nif_controls_keep_original_source_and_view_provenance() {
        for archived in [false, true] {
            for replace_source in [false, true] {
                let (mut app, root) = crate::tests::data_app(&[], &[]);
                let path = root.join(if archived { "models.bsa" } else { "mesh.nif" });
                std::fs::write(&path, b"source").unwrap();
                let _ = crate::file_preview::start(&mut app, path.clone(), None, None, None);
                let id = app.preview_pending.as_ref().unwrap().id;
                let content = Preview::Nif {
                    path: path.clone(),
                    provenance: None,
                    model: fixture_model(),
                };
                let preview = if archived {
                    Preview::Archive {
                        source: crate::archive_conflicts::MemberSource {
                            path: path.clone(),
                            member: "mesh.nif".into(),
                            identity: eidos_conflicts::archive_identity(&path).unwrap(),
                        },
                        content: Box::new(content),
                    }
                } else {
                    content
                };
                crate::file_preview::complete(&mut app, id, preview);
                let cached = app.preview.clone().unwrap();
                if replace_source {
                    std::fs::write(&path, b"replacement source").unwrap();
                } else {
                    crate::bump_views(&app);
                }
                let _ = crate::file_preview::start_nif(&mut app, path.clone(), Default::default());
                let id = app.preview_pending.as_ref().unwrap().id;
                crate::file_preview::complete(&mut app, id, cached);
                assert!(
                    app.preview.is_none(),
                    "stale NIF context; archived={archived}, replaced={replace_source}"
                );
                std::fs::remove_dir_all(root).unwrap();
            }
        }
    }

    #[test]
    fn nif_texture_phase_and_late_replies_keep_the_original_request() {
        let (mut app, root) = crate::tests::data_app(&[], &[]);
        let path = root.join("mesh.nif");
        std::fs::write(&path, b"source").unwrap();
        let _ = crate::file_preview::start(&mut app, path.clone(), None, None, None);
        let id = app.preview_pending.as_ref().unwrap().id;
        let cancel = crate::file_preview::pending_cancel(&app, id).unwrap();
        let preview = Preview::Nif {
            path: path.clone(),
            provenance: None,
            model: fixture_model(),
        };
        assert!(prepare_textures(&app, id, &preview).is_some());
        std::fs::write(&path, b"changed source").unwrap();
        assert!(prepare_textures(&app, id, &preview).is_none());

        let _ = crate::file_preview::start(&mut app, path.clone(), None, None, None);
        let next = app.preview_pending.as_ref().unwrap().id;
        assert!(cancel.load(Ordering::Relaxed));
        crate::file_preview::complete(&mut app, id, preview.clone());
        assert_eq!(app.preview_pending.as_ref().unwrap().id, next);
        app.preview_pending.take();
        app.preview = None;
        assert!(prepare_textures(&app, next, &preview).is_none());
        crate::file_preview::complete(&mut app, next, preview);
        assert!(app.preview.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_texture_reads_consume_their_attempted_budget() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("too-large.dds");
        std::fs::write(&path, [0; 32]).unwrap();
        let mut input = MAX_TEXTURE_INPUT - 16;
        let mut pixels = 0;
        assert!(read_texture(&ProviderSource::Loose(path), &mut input, &mut pixels).is_err());
        assert_eq!(
            input, MAX_TEXTURE_INPUT,
            "failed reads must not bypass the aggregate limit"
        );
        assert_eq!(pixels, 0);
    }
}
