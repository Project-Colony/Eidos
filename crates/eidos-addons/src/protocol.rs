//! Versioned replies from explicitly installed, trusted helper programs.

use crate::{Addon, AddonKind, Context};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Installer,
    Preview,
    SaveInfo,
}

impl Operation {
    pub fn kind(self) -> AddonKind {
        match self {
            Self::Installer => AddonKind::Installer,
            Self::Preview => AddonKind::Preview,
            Self::SaveInfo => AddonKind::SaveInfo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u32,
    pub request_id: String,
    pub operation: Operation,
    pub game: String,
    pub input: PathBuf,
    /// Extracted source tree for installers; never an installed destination.
    pub source: Option<PathBuf>,
    pub workspace: PathBuf,
    #[serde(default)]
    pub answers: BTreeMap<String, Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub protocol: u32,
    pub request_id: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Declined,
    Manual {
        reason: String,
    },
    Cancelled,
    Failed {
        message: String,
    },
    Prompt {
        id: String,
        title: String,
        options: Vec<String>,
        multiple: bool,
    },
    Handled {
        result: Payload,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Payload {
    Preview {
        path: String,
    },
    SaveInfo {
        fields: Vec<Field>,
        plugins: Vec<String>,
        light_plugins: Vec<String>,
    },
    Install {
        files: Vec<FileCopy>,
        warnings: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCopy {
    pub source: String,
    pub destination: String,
}

/// Stable priority/id ordering. Discovery never executes a candidate.
pub fn matching<'a>(addons: &'a [Addon], request: &Request) -> Vec<&'a Addon> {
    let extension = request
        .input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let mut candidates: Vec<_> = addons
        .iter()
        .filter(|a| {
            a.kind == request.operation.kind()
                && a.applies_to(&request.game)
                && a.extensions
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(extension))
                && a.markers.iter().all(|marker| {
                    request
                        .source
                        .as_ref()
                        .is_some_and(|root| regular_file(root, marker).is_ok())
                })
        })
        .collect();
    candidates.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
    candidates
}

/// One complete JSON reply per invocation. A prompt is replayed by invoking the
/// same helper with recorded answers and the same source; stdout is never a log.
pub fn invoke(
    addon: &Addon,
    context: &Context,
    request: &Request,
    cancel: &AtomicBool,
) -> Result<Reply, String> {
    invoke_with_timeout(addon, context, request, cancel, Duration::from_secs(30))
}

fn invoke_with_timeout(
    addon: &Addon,
    context: &Context,
    request: &Request,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Reply, String> {
    if request.protocol != 1 || addon.protocol != 1 || addon.kind != request.operation.kind() {
        return Err("unsupported extension operation or protocol".into());
    }
    if request.request_id.is_empty() || request.request_id.len() > 256 || request.answers.len() > 64
    {
        return Err("invalid extension request identity or answer count".into());
    }
    if !request.input.is_absolute()
        || !request.workspace.is_absolute()
        || !request.workspace.is_dir()
    {
        return Err("extension input and workspace must be absolute existing paths".into());
    }
    let before = identity(&request.input)?;
    let bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("extension request exceeds 1 MiB".into());
    }
    let mut file =
        tempfile::NamedTempFile::new_in(&request.workspace).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    let mut context = context.clone();
    context
        .values
        .insert("request".into(), file.path().display().to_string());
    context
        .values
        .insert("input".into(), request.input.display().to_string());
    context
        .values
        .insert("workspace".into(), request.workspace.display().to_string());
    let output = crate::capture(addon, &context, timeout, cancel)?;
    if !output.status.success() {
        return Err(format!(
            "extension exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(2048)
                .collect::<String>()
        ));
    }
    if identity(&request.input)? != before {
        return Err("extension input changed during the request".into());
    }
    let reply: Reply = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("invalid extension reply: {e}"))?;
    validate_reply(request, &reply)?;
    Ok(reply)
}

#[derive(PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
fn identity(path: &Path) -> Result<Identity, String> {
    use std::os::unix::fs::MetadataExt;
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() {
        return Err("extension input must be a regular file".into());
    }
    Ok(Identity {
        dev: m.dev(),
        ino: m.ino(),
        len: m.len(),
        mtime: m.mtime(),
        mtime_ns: m.mtime_nsec(),
        ctime: m.ctime(),
        ctime_ns: m.ctime_nsec(),
    })
}

pub fn validate_reply(request: &Request, reply: &Reply) -> Result<(), String> {
    if reply.protocol != 1 || reply.request_id != request.request_id {
        return Err("stale or unsupported extension reply".into());
    }
    match &reply.outcome {
        Outcome::Manual { reason } | Outcome::Failed { message: reason } => text(reason, 4096),
        Outcome::Prompt {
            id, title, options, ..
        } => {
            text(id, 128)?;
            text(title, 4096)?;
            if request.operation != Operation::Installer || options.is_empty() || options.len() > 64
            {
                return Err("invalid extension prompt".into());
            }
            for option in options {
                text(option, 4096)?;
            }
            if request.answers.contains_key(id) {
                return Err("extension repeated an answered prompt".into());
            }
            Ok(())
        }
        Outcome::Handled {
            result: Payload::Preview { path },
        } if request.operation == Operation::Preview => {
            let file = regular_file(&request.workspace, path)?;
            if fs::metadata(&file).map_err(|e| e.to_string())?.len() > 128 * 1024 * 1024 {
                return Err("extension preview exceeds 128 MiB".into());
            }
            Ok(())
        }
        Outcome::Handled {
            result:
                Payload::SaveInfo {
                    fields,
                    plugins,
                    light_plugins,
                },
        } if request.operation == Operation::SaveInfo => {
            if fields.len() > 64 || plugins.len() > 4096 || light_plugins.len() > 4096 {
                return Err("extension save information exceeds limits".into());
            }
            for field in fields {
                text(&field.label, 128)?;
                text(&field.value, 4096)?;
            }
            for plugin in plugins.iter().chain(light_plugins) {
                if relative_path(plugin)?.components().count() != 1 {
                    return Err("invalid save plugin name".into());
                }
            }
            Ok(())
        }
        Outcome::Handled {
            result: Payload::Install { files, warnings },
        } if request.operation == Operation::Installer => {
            if files.is_empty() || files.len() > 65536 || warnings.len() > 64 {
                return Err("extension file plan exceeds limits or is empty".into());
            }
            let root = request
                .source
                .as_ref()
                .ok_or("installer source tree is missing")?;
            let mut destinations = BTreeSet::new();
            let mut size = 0u64;
            for file in files {
                let source = regular_file(root, &file.source)?;
                size = size
                    .checked_add(fs::metadata(source).map_err(|e| e.to_string())?.len())
                    .ok_or("installer size overflow")?;
                if size > 16 * 1024 * 1024 * 1024 {
                    return Err("extension file plan exceeds 16 GiB".into());
                }
                let destination = relative_path(&file.destination)?;
                let key = destination
                    .to_str()
                    .ok_or("invalid destination encoding")?
                    .to_ascii_lowercase();
                if key.split('/').any(|part| {
                    part == "meta.ini" || part.starts_with(".eidos") || part.ends_with(".mohidden")
                }) {
                    return Err("extension destination uses reserved metadata".into());
                }
                if !destinations.insert(key) {
                    return Err("duplicate extension destination".into());
                }
            }
            for destination in &destinations {
                for (index, _) in destination.match_indices('/') {
                    if destinations.contains(&destination[..index]) {
                        return Err("extension file/directory destination collision".into());
                    }
                }
            }
            for warning in warnings {
                text(warning, 4096)?;
            }
            Ok(())
        }
        Outcome::Handled { .. } => Err("extension returned the wrong result kind".into()),
        Outcome::Declined | Outcome::Cancelled => Ok(()),
    }
}

fn text(value: &str, limit: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > limit || value.contains('\0') {
        Err("invalid extension text".into())
    } else {
        Ok(())
    }
}

/// Windows and POSIX paths share this checked, normalized relative spelling.
pub fn relative_path(raw: &str) -> Result<PathBuf, String> {
    if raw.is_empty()
        || raw.len() > 4096
        || raw.chars().any(char::is_control)
        || raw.contains([':', '<', '>', '"', '|', '?', '*'])
    {
        return Err("invalid relative extension path".into());
    }
    let normalized = raw.replace('\\', "/");
    if normalized.split('/').any(|part| {
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .trim_end_matches(' ')
            .to_ascii_uppercase();
        let device = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|n| {
                    matches!(
                        n,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            });
        part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' ']) || device
    }) {
        return Err("extension path must contain ordinary relative components".into());
    }
    Ok(PathBuf::from(normalized))
}

/// Resolve a regular file without following symlinks in any returned component.
pub fn regular_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.canonicalize().map_err(|e| e.to_string())?;
    for part in relative_path(relative)?.components() {
        path.push(part);
        if fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("extension file plan contains a symlink".into());
        }
    }
    if !path.is_file() {
        return Err("extension selected a non-regular file".into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_helper_executes_preview_save_information_and_recorded_installer_choices() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/examples/extensions")
            .canonicalize()
            .unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let input = workspace.path().join("save with spaces.json");
        fs::write(&input, br#"{"player":"Example","level":42}"#).unwrap();
        fs::write(
            workspace.path().join("eidos-example.txt"),
            b"example payload",
        )
        .unwrap();
        let mut request = Request {
            protocol: 1,
            request_id: "example".into(),
            operation: Operation::Preview,
            game: "example".into(),
            input: input.clone(),
            source: Some(workspace.path().into()),
            workspace: workspace.path().into(),
            answers: BTreeMap::new(),
        };
        let addons = crate::load_addons_from(&examples);
        let cancel = AtomicBool::new(false);
        for (operation, manifest) in [
            (Operation::Preview, "example-text-preview"),
            (Operation::SaveInfo, "example-json-save"),
            (Operation::Installer, "example-file-installer"),
        ] {
            request.operation = operation;
            let addon = addons.iter().find(|a| a.id == manifest).unwrap();
            let reply = invoke(addon, &Context::default(), &request, &cancel).unwrap();
            match (operation, reply.outcome) {
                (
                    Operation::Preview,
                    Outcome::Handled {
                        result: Payload::Preview { path },
                    },
                ) => assert_eq!(
                    fs::read(regular_file(workspace.path(), &path).unwrap()).unwrap(),
                    fs::read(&input).unwrap()
                ),
                (
                    Operation::SaveInfo,
                    Outcome::Handled {
                        result: Payload::SaveInfo { fields, .. },
                    },
                ) => assert!(fields
                    .iter()
                    .any(|field| field.label == "level" && field.value == "42")),
                (Operation::Installer, Outcome::Prompt { id, .. }) => {
                    request.answers.insert(id, vec![0]);
                    let reply = invoke(addon, &Context::default(), &request, &cancel).unwrap();
                    assert!(
                        matches!(reply.outcome,Outcome::Handled{result:Payload::Install{files,..}} if files.len()==1&&files[0].destination=="docs/example.txt")
                    );
                    request.answers.insert("readme".into(), vec![1]);
                    assert!(matches!(
                        invoke(addon, &Context::default(), &request, &cancel)
                            .unwrap()
                            .outcome,
                        Outcome::Cancelled
                    ));
                }
                (_, other) => panic!("unexpected helper outcome: {other:?}"),
            }
        }
        assert_eq!(
            fs::read(&input).unwrap(),
            br#"{"player":"Example","level":42}"#
        );
    }

    #[test]
    fn installer_timeout_uses_the_real_bounded_host() {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("input.zip");
        fs::write(&input, b"unchanged archive").unwrap();
        let mut addon = crate::parse_addon(
            "id='timeout'\nkind='installer'\nprotocol=1\nexec='/bin/sh'\nextensions=['zip']",
            Path::new("/tmp/timeout.toml"),
        )
        .unwrap();
        addon.args = vec!["-c".into(), "sleep 5".into()];
        let request = Request {
            protocol: 1,
            request_id: "timeout".into(),
            operation: Operation::Installer,
            game: "skyrimse".into(),
            input: input.clone(),
            source: Some(root.path().into()),
            workspace: root.path().into(),
            answers: BTreeMap::new(),
        };
        let started = std::time::Instant::now();
        assert!(invoke_with_timeout(
            &addon,
            &Context::default(),
            &request,
            &AtomicBool::new(false),
            Duration::from_millis(100)
        )
        .unwrap_err()
        .contains("longer"));
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(fs::read(input).unwrap(), b"unchanged archive");
    }

    #[test]
    fn portable_paths_reject_windows_devices_controls_and_wildcards() {
        for path in [
            "CON",
            "con.txt",
            "Aux.dds",
            "PRN",
            "NUL.foo",
            "COM1",
            "LPT9.txt",
            "COM¹.esp",
            "LPT²",
            "COM³",
            "a/aux/b",
            "a?b",
            "a*b",
            "a<b",
            "a>b",
            "a|b",
            "a\"b",
            "a\u{1f}b",
            "a\u{7f}b",
            "x/..",
            "x/",
            "/x",
            "C:x",
            "x.",
            "x ",
        ] {
            assert!(relative_path(path).is_err(), "{path:?}");
        }
        for path in [
            "textures/a.dds",
            "Textures\\名前.dds",
            "COM0",
            "COM10.txt",
            "console.txt",
            "lpt²_extra",
            "normal name.txt",
        ] {
            assert!(relative_path(path).is_ok(), "{path:?}");
        }
    }

    #[test]
    fn replies_reject_stale_identity_escape_wrong_kind_and_overlapping_destinations() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("input.zip"), b"source").unwrap();
        let request = Request {
            protocol: 1,
            request_id: "one".into(),
            operation: Operation::Installer,
            game: "skyrimse".into(),
            input: root.path().join("input.zip"),
            source: Some(root.path().into()),
            workspace: root.path().into(),
            answers: BTreeMap::new(),
        };
        let plan = |destinations: &[&str]| Reply {
            protocol: 1,
            request_id: "one".into(),
            outcome: Outcome::Handled {
                result: Payload::Install {
                    files: destinations
                        .iter()
                        .map(|d| FileCopy {
                            source: "input.zip".into(),
                            destination: (*d).into(),
                        })
                        .collect(),
                    warnings: vec![],
                },
            },
        };
        assert!(validate_reply(&request, &plan(&["textures/a.dds"])).is_ok());
        for destinations in [
            &["../bad"][..],
            &["meta.ini"],
            &["X", "x"],
            &["a", "a/b"],
            &["a", "a-b", "a/c"],
            &["Root\\..\\bad"],
            &["C:\\bad"],
        ] {
            assert!(
                validate_reply(&request, &plan(destinations)).is_err(),
                "{destinations:?}"
            );
        }
        let mut stale = plan(&["ok"]);
        stale.request_id = "old".into();
        assert!(validate_reply(&request, &stale).is_err());
        stale.request_id = "one".into();
        stale.outcome = Outcome::Handled {
            result: Payload::Preview {
                path: "input.zip".into(),
            },
        };
        assert!(validate_reply(&request, &stale).is_err());
        std::os::unix::fs::symlink("input.zip", root.path().join("link")).unwrap();
        assert!(regular_file(root.path(), "link").is_err());
        assert!(serde_json::from_str::<Reply>(
            r#"{"protocol":1,"request_id":"one","outcome":{"status":"declined"},"extra":true}"#
        )
        .is_err());
    }
}
