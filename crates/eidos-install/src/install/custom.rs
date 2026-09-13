//! Trusted protocol-1 installers with exact source and recorded-prompt replay.
use super::*;
use eidos_addons::{
    protocol::{self, FileCopy, Operation, Outcome, Payload, Request},
    Addon, Context,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 65_536;
const RECEIPT_FILE: &str = ".eidos-custom-installer.json";
const MAX_RECEIPT: usize = 16 * 1024 * 1024;

fn matching_input(archive: &Path) -> PathBuf {
    // Collection caches use .archive when the remote name is unavailable.
    // Only that opaque suffix may substitute a recognized container signature.
    let mut input = archive.to_path_buf();
    if fs::symlink_metadata(archive).is_ok_and(|m| m.is_file())
        && archive
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("archive"))
    {
        let mut magic = [0; 8];
        if let Ok(mut file) = File::open(archive) {
            if file.read_exact(&mut magic).is_ok() {
                let extension = if magic.starts_with(b"PK\x03\x04")
                    || magic.starts_with(b"PK\x05\x06")
                    || magic.starts_with(b"PK\x07\x08")
                {
                    Some("zip")
                } else if magic.starts_with(b"7z\xbc\xaf\x27\x1c") {
                    Some("7z")
                } else if magic.starts_with(b"Rar!\x1a\x07\x00")
                    || magic.starts_with(b"Rar!\x1a\x07\x01\x00")
                {
                    Some("rar")
                } else {
                    None
                };
                if let Some(extension) = extension {
                    input.set_extension(extension);
                }
            }
        }
    }
    input
}

/// Cheap prefilter before a caller pays for an instance snapshot. Markers are
/// checked after extraction by the normal classifier.
pub fn may_handle(addons: &[Addon], game_id: &str, archive: &Path) -> bool {
    if addons.is_empty() {
        return false;
    }
    let input = matching_input(archive);
    let ext = input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    addons.iter().any(|a| {
        a.kind == eidos_addons::AddonKind::Installer
            && a.applies_to(game_id)
            && a.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    })
}

/// Capture the current instance/profile and visible-layer inputs for exact replay.
/// The caller holds its instance lock across capture and publication. File content
/// outside profile controls is identified by inode/size/mtime/ctime, not read eagerly.
#[allow(clippy::too_many_arguments)]
pub fn context_for_instance(
    instance: &eidos_instance::Instance,
    game_id: &str,
    install: &Path,
    data: &Path,
    prefix: Option<&Path>,
    control_dirs: &[PathBuf],
    cancel: &AtomicBool,
) -> Result<Context, InstallError> {
    use std::os::unix::fs::MetadataExt;
    let profile = instance.active();
    let mods = profile.modlist();
    let mut hash = Sha256::new();
    hash.update(encoded(
        &mods
            .iter()
            .map(|m| (&m.name, m.is_active()))
            .collect::<Vec<_>>(),
    )?);
    let mut controls = vec![instance.root.join("instance.ini")];
    for dir in [profile.dir(), profile.plugins_state_dir()] {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file()
                    && entry.path().extension().is_some_and(|e| {
                        matches!(
                            e.to_string_lossy().to_ascii_lowercase().as_str(),
                            "ini" | "txt" | "json"
                        )
                    })
                {
                    if controls.len() >= 1024 {
                        return Err(invalid("Installer context exceeds 1024 control files"));
                    }
                    controls.push(entry.path());
                }
            }
        }
    }
    if control_dirs.len() > 32 || control_dirs.iter().any(|p| !p.is_absolute()) {
        return Err(invalid(
            "Installer control directories must be at most 32 absolute paths",
        ));
    }
    for dir in control_dirs {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                if entry.path().extension().is_some_and(|e| {
                    matches!(
                        e.to_string_lossy().to_ascii_lowercase().as_str(),
                        "ini" | "txt" | "json"
                    )
                }) {
                    if !entry.file_type()?.is_file() {
                        return Err(invalid("Installer control is not a regular file"));
                    }
                    if controls.len() >= 1024 {
                        return Err(invalid("Installer context exceeds 1024 control files"));
                    }
                    controls.push(entry.path());
                }
            }
        }
    }
    // The effective mod rows above already include unpersisted inactive reservations.
    // Saving that same view is serialization, not a new installer decision.
    controls.retain(|p| {
        !p.file_name()
            .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("modlist.txt"))
    });
    controls.sort();
    controls.dedup();
    let mut remaining = 64 * 1024 * 1024;
    for path in controls {
        check_cancel(cancel)?;
        if path.exists() {
            let stamp = file_stamp(&path, remaining.min(8 * 1024 * 1024), cancel)?;
            remaining -= stamp.size;
            hash.update(encoded(&(path, stamp))?);
        }
    }
    // ponytail: bound a conservative layer snapshot instead of adding another index.
    // Hidden lower changes may require a fresh review, but never silently reuse it.
    let mut layers: Vec<_> = mods
        .iter()
        .filter(|m| m.is_active())
        .map(|m| m.path.clone())
        .collect();
    layers.extend([data.to_path_buf(), instance.overwrite_dir()]);
    layers.extend(instance.root_layers());
    layers.push(instance.root_overwrite_dir());
    layers.push(profile.dir().join("runtime-root"));
    layers.push(install.to_path_buf());
    let mut entries = 0usize;
    for root in layers {
        hash.update(encoded(&root)?);
        if !root.exists() {
            continue;
        }
        let game_root = root == install;
        let mut dirs = vec![(root, 0usize)];
        while let Some((dir, depth)) = dirs.pop() {
            check_cancel(cancel)?;
            if depth > 128 {
                return Err(invalid("Installer context exceeds 128 directory levels"));
            }
            let mut children = Vec::new();
            for entry in fs::read_dir(dir)? {
                entries += 1;
                if entries > 500_000 {
                    return Err(invalid("Installer context exceeds 500000 entries"));
                }
                children.push(entry?.path());
            }
            children.sort();
            for path in children {
                check_cancel(cancel)?;
                if game_root && path == data {
                    continue;
                }
                let meta = fs::symlink_metadata(&path)?;
                if meta.is_dir() {
                    hash.update(encoded(&(&path, "directory"))?);
                    dirs.push((path, depth + 1));
                } else if meta.is_file() {
                    hash.update(encoded(&(
                        &path,
                        [
                            meta.dev(),
                            meta.ino(),
                            meta.len(),
                            meta.mtime() as u64,
                            meta.mtime_nsec() as u64,
                            meta.ctime() as u64,
                            meta.ctime_nsec() as u64,
                        ],
                    ))?);
                } else {
                    return Err(invalid(
                        "Installer context contains a linked or special source",
                    ));
                }
            }
        }
    }
    let values = [
        ("instance", instance.root.display().to_string()),
        ("mods", instance.mods_dir().display().to_string()),
        ("downloads", instance.downloads_dir().display().to_string()),
        ("overwrite", instance.overwrite_dir().display().to_string()),
        ("profile", profile.name.clone()),
        ("profile_dir", profile.dir().display().to_string()),
        ("game", game_id.to_string()),
        ("install", install.display().to_string()),
        ("data", data.display().to_string()),
        (
            "prefix",
            prefix.map(|p| p.display().to_string()).unwrap_or_default(),
        ),
        (
            "instance_control_dirs",
            serde_json::to_string(control_dirs).map_err(|e| invalid(e.to_string()))?,
        ),
        ("instance_state_sha256", hex(&hash.finalize())),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    Ok(Context { values })
}

fn verify_context(context: &Context, cancel: &AtomicBool) -> Result<(), InstallError> {
    let Some(expected) = context.values.get("instance_state_sha256") else {
        return Ok(());
    };
    let get = |name: &str| {
        context
            .values
            .get(name)
            .ok_or_else(|| invalid("Incomplete instance installer context"))
    };
    let instance = eidos_instance::Instance::portable(PathBuf::from(get("instance")?));
    let prefix = get("prefix")?;
    let control_dirs: Vec<PathBuf> =
        serde_json::from_str(get("instance_control_dirs")?).map_err(|e| invalid(e.to_string()))?;
    let actual = context_for_instance(
        &instance,
        get("game")?,
        Path::new(get("install")?),
        Path::new(get("data")?),
        (!prefix.is_empty()).then(|| Path::new(prefix)),
        &control_dirs,
        cancel,
    )?;
    if actual.values.get("instance_state_sha256") != Some(expected)
        || actual.values.get("profile") != context.values.get("profile")
    {
        return Err(invalid(
            "Instance/profile inputs changed; reopen the custom installer review",
        ));
    }
    Ok(())
}

/// Read the reserved installed receipt before selecting an installer on reinstall.
pub fn read_installed_receipt(mod_root: &Path) -> Result<Option<CustomReceipt>, InstallError> {
    let path = mod_root.join(RECEIPT_FILE);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(md) if !md.is_file() => {
            return Err(invalid("Custom installer receipt is not a regular file"))
        }
        Ok(md) if md.len() > MAX_RECEIPT as u64 => {
            return Err(invalid("Custom installer receipt exceeds 16 MiB"))
        }
        _ => {}
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_RECEIPT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RECEIPT {
        return Err(invalid("Custom installer receipt exceeds 16 MiB"));
    }
    let receipt: CustomReceipt = serde_json::from_slice(&bytes)
        .map_err(|e| invalid(format!("Invalid custom installer receipt: {e}")))?;
    if receipt.version != 1 || receipt.answers.len() > 64 {
        return Err(invalid("Unsupported custom installer receipt"));
    }
    Ok(Some(receipt))
}

fn invalid(message: impl Into<String>) -> InstallError {
    InstallError::BadSelection(message.into())
}
fn check_cancel(cancel: &AtomicBool) -> Result<(), InstallError> {
    if cancel.load(Ordering::Relaxed) {
        Err(invalid("Custom installer cancelled"))
    } else {
        Ok(())
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>, InstallError> {
    serde_json::to_vec(value).map_err(|e| invalid(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomPrompt {
    pub id: String,
    pub title: String,
    pub options: Vec<String>,
    pub multiple: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomRecordedAnswer {
    pub prompt: CustomPrompt,
    pub selected: Vec<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomPlan {
    pub files: Vec<FileCopy>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomReceipt {
    pub version: u32,
    pub installer: String,
    pub installer_version: String,
    pub helper_sha256: String,
    pub archive_sha256: String,
    pub source_sha256: String,
    pub context_sha256: String,
    pub answers: Vec<CustomRecordedAnswer>,
    pub plan: Option<CustomPlan>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomEvaluation {
    Prompt(CustomPrompt),
    Ready(CustomPlan),
    Manual(String),
    Cancelled,
}
pub enum CustomStart {
    Fallback(ExtractedTree),
    Selected(Box<CustomSession>),
}

pub struct CustomSession {
    tree: ExtractedTree,
    workspace: ExtractedTree,
    archive: PathBuf,
    game_id: String,
    addon: Addon,
    sources: BTreeMap<String, FileStamp>,
    receipt: CustomReceipt,
    pub state: CustomEvaluation,
}
impl CustomSession {
    pub fn receipt(&self) -> CustomReceipt {
        self.receipt.clone()
    }
    pub fn into_tree(self) -> ExtractedTree {
        self.tree
    }
    pub fn installer_name(&self) -> &str {
        &self.addon.name
    }
    pub fn archive(&self) -> &Path {
        &self.archive
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FileStamp {
    size: u64,
    sha256: String,
}

fn file_stamp(path: &Path, limit: u64, cancel: &AtomicBool) -> Result<FileStamp, InstallError> {
    use std::os::unix::fs::MetadataExt;
    if !fs::metadata(path)?.is_file() {
        return Err(invalid("Installer input must be a regular file"));
    }
    let mut input = File::open(path)?;
    let before = input.metadata()?;
    if !before.is_file() || before.len() > limit {
        return Err(invalid(
            "Installer file is not regular or exceeds its byte limit",
        ));
    }
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut buffer = [0; 65536];
    loop {
        check_cancel(cancel)?;
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        if size > limit {
            return Err(invalid("Installer file grew past its byte limit"));
        }
        hash.update(&buffer[..n]);
    }
    let after = input.metadata()?;
    let identity = |m: &fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if size != before.len() || identity(&before) != identity(&after) {
        return Err(invalid("Installer file changed while hashing"));
    }
    Ok(FileStamp {
        size,
        sha256: hex(&hash.finalize()),
    })
}

// ponytail: one sorted inventory binds all extracted inputs; no private copy of the tree.
fn inventory(
    root: &Path,
    cancel: &AtomicBool,
) -> Result<BTreeMap<String, FileStamp>, InstallError> {
    fn walk(
        root: &Path,
        dir: &Path,
        depth: usize,
        count: &mut usize,
        total: &mut u64,
        out: &mut BTreeMap<String, FileStamp>,
        cancel: &AtomicBool,
    ) -> Result<(), InstallError> {
        if depth > 128 {
            return Err(invalid("Custom installer source depth exceeds 128"));
        }
        for entry in fs::read_dir(dir)? {
            check_cancel(cancel)?;
            let entry = entry?;
            *count += 1;
            if *count > MAX_FILES {
                return Err(invalid("Custom installer source exceeds 65536 entries"));
            }
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .ok_or_else(|| invalid("Non-UTF-8 installer source path"))?
                .to_string();
            protocol::relative_path(&relative).map_err(invalid)?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                out.insert(
                    relative,
                    FileStamp {
                        size: 0,
                        sha256: "directory".into(),
                    },
                );
                walk(root, &path, depth + 1, count, total, out, cancel)?;
            } else if kind.is_file() {
                let stamp = file_stamp(&path, MAX_BYTES - *total, cancel)?;
                *total += stamp.size;
                out.insert(relative, stamp);
            } else {
                return Err(invalid(
                    "Custom installer source contains a link or special file",
                ));
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(root, root, 0, &mut 0, &mut 0, &mut out, cancel)?;
    Ok(out)
}
fn expanded_context(addon: &Addon, context: &Context) -> Context {
    let mut context = context.clone();
    if let Some(parent) = addon.source.parent() {
        context
            .values
            .insert("addon_dir".into(), parent.display().to_string());
    }
    context
}
fn helper_stamp(
    addon: &Addon,
    context: &Context,
    cancel: &AtomicBool,
) -> Result<String, InstallError> {
    if let Some(reason) = addon.unavailable() {
        return Err(invalid(format!(
            "Required installer {} is unavailable: {reason}",
            addon.id
        )));
    }
    let exe = eidos_addons::which(&addon.exec)
        .or_else(|| addon.exec.is_file().then(|| addon.exec.clone()))
        .ok_or_else(|| invalid("Installer executable is unavailable"))?
        .canonicalize()?;
    let context = expanded_context(addon, context);
    let workdir = if addon.workdir.is_empty() {
        context
            .values
            .get("instance")
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir()?)
    } else {
        PathBuf::from(context.expand(&addon.workdir))
    };
    if addon.args.len() > 1024 {
        return Err(invalid("Installer has more than 1024 arguments"));
    }
    let mut files = BTreeMap::new();
    let mut remaining = 64 * 1024 * 1024;
    for path in std::iter::once(addon.source.clone())
        .chain(std::iter::once(exe))
        .chain(
            addon
                .args
                .iter()
                .filter(|arg| {
                    !arg.contains("{request}")
                        && !arg.contains("{workspace}")
                        && !arg.contains("{input}")
                })
                .filter_map(|arg| {
                    let arg = context.expand(arg);
                    let path = PathBuf::from(arg);
                    let path = if path.is_absolute() {
                        path
                    } else {
                        workdir.join(path)
                    };
                    path.is_file().then_some(path)
                }),
        )
    {
        let path = path.canonicalize()?;
        if files.contains_key(&path) {
            continue;
        }
        let stamp = file_stamp(&path, remaining, cancel)?;
        remaining -= stamp.size;
        files.insert(path, stamp);
    }
    let definition = (
        &addon.id,
        &addon.name,
        &addon.version,
        &addon.exec,
        &addon.args,
        &addon.workdir,
        &addon.games,
        addon.protocol,
        &addon.extensions,
        &addon.markers,
        addon.priority,
        &addon.source,
    );
    Ok(digest(&encoded(&(definition, files))?))
}
fn context_stamp(game: &str, context: &Context) -> Result<String, InstallError> {
    let bytes = encoded(&(game, &context.values))?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("Installer context exceeds 1 MiB"));
    }
    Ok(digest(&bytes))
}
fn base_matches(a: &CustomReceipt, b: &CustomReceipt) -> bool {
    a.version == 1
        && a.version == b.version
        && a.installer == b.installer
        && a.installer_version == b.installer_version
        && a.helper_sha256 == b.helper_sha256
        && a.archive_sha256 == b.archive_sha256
        && a.source_sha256 == b.source_sha256
        && a.context_sha256 == b.context_sha256
}
fn verify_origin<'a>(
    tree: &'a ExtractedTree,
    archive: &Path,
    cancel: &AtomicBool,
) -> Result<&'a super::ArchiveOrigin, InstallError> {
    let origin = tree
        .origin
        .as_ref()
        .ok_or_else(|| invalid("Custom installer source has no archive origin"))?;
    if archive.canonicalize()? != origin.path() || fs::metadata(archive)?.len() > MAX_BYTES {
        return Err(invalid(
            "Custom installer archive does not match the extracted source",
        ));
    }
    origin.verify_current(cancel)?;
    Ok(origin)
}
fn current(
    session: &CustomSession,
    addons: &[Addon],
    context: &Context,
    cancel: &AtomicBool,
) -> Result<(), InstallError> {
    verify_context(context, cancel)?;
    verify_origin(&session.tree, &session.archive, cancel)?;
    let addon = addons
        .iter()
        .find(|addon| addon.id == session.addon.id)
        .ok_or_else(|| invalid("Required custom installer is no longer installed"))?;
    if addon != &session.addon
        || helper_stamp(addon, context, cancel)? != session.receipt.helper_sha256
        || context_stamp(&session.game_id, context)? != session.receipt.context_sha256
        || inventory(session.tree.path(), cancel)? != session.sources
    {
        return Err(invalid(
            "Custom installer, archive, extracted sources or context changed; reopen before replay",
        ));
    }
    Ok(())
}

/// Try explicitly installed helpers before native fallback. FOMOD's priority is 90.
pub fn try_custom_installers(
    tree: ExtractedTree,
    archive: &Path,
    game_id: &str,
    addons: &[Addon],
    context: &Context,
    cancel: &AtomicBool,
) -> Result<CustomStart, InstallError> {
    try_custom_installers_with_receipt(tree, archive, game_id, addons, context, cancel, None)
}

/// Replay validates the required helper before running any candidate.
#[allow(clippy::too_many_arguments)]
pub fn try_custom_installers_with_receipt(
    tree: ExtractedTree,
    archive: &Path,
    game_id: &str,
    addons: &[Addon],
    context: &Context,
    cancel: &AtomicBool,
    expected: Option<&CustomReceipt>,
) -> Result<CustomStart, InstallError> {
    check_cancel(cancel)?;
    let archive = archive.canonicalize()?;
    let request = Request {
        protocol: 1,
        request_id: "discovery".into(),
        operation: Operation::Installer,
        game: game_id.into(),
        input: matching_input(&archive),
        source: Some(tree.path().into()),
        workspace: tree.path().into(),
        answers: BTreeMap::new(),
    };
    let fomod = find_fomod_root(tree.path()).is_some();
    let candidates: Vec<Addon> = protocol::matching(addons, &request)
        .into_iter()
        .filter(|a| !fomod || a.priority > 90)
        .filter(|a| expected.is_none_or(|receipt| receipt.installer == a.id))
        .cloned()
        .collect();
    if candidates.is_empty() {
        return if expected.is_some() {
            Err(invalid(
                "Required custom installer is unavailable or no longer matches",
            ))
        } else {
            Ok(CustomStart::Fallback(tree))
        };
    }
    let sources = inventory(tree.path(), cancel)?;
    let source_sha256 = digest(&encoded(&sources)?);
    let archive_sha256 = verify_origin(&tree, &archive, cancel)?.sha256().to_owned();
    let context_sha256 = context_stamp(game_id, context)?;
    let tmp = tree
        .path()
        .parent()
        .ok_or_else(|| invalid("Installer source has no parent"))?
        .join(format!(
            ".eidos-install-helper-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir(&tmp)?;
    let workspace = ExtractedTree::owned(tmp);
    let mut session = CustomSession {
        tree,
        workspace,
        archive,
        game_id: game_id.into(),
        addon: candidates[0].clone(),
        sources,
        receipt: CustomReceipt {
            version: 1,
            installer: String::new(),
            installer_version: String::new(),
            helper_sha256: String::new(),
            archive_sha256,
            source_sha256,
            context_sha256,
            answers: vec![],
            plan: None,
        },
        state: CustomEvaluation::Cancelled,
    };
    for addon in candidates {
        session.receipt.installer = addon.id.clone();
        session.receipt.installer_version = addon.version.clone();
        session.receipt.helper_sha256 = helper_stamp(&addon, context, cancel)?;
        session.addon = addon;
        if let Some(expected) = expected {
            if !base_matches(expected, &session.receipt) {
                return Err(invalid(
                    "Recorded custom installer identity is stale; helper was not invoked",
                ));
            }
            session.receipt = expected.clone();
        }
        current(&session, addons, context, cancel)?;
        if let Some(state) = run(&session, &session.receipt, context, cancel)? {
            current(&session, addons, context, cancel)?;
            session.receipt.plan = if let CustomEvaluation::Ready(plan) = &state {
                Some(plan.clone())
            } else {
                None
            };
            session.state = state;
            return Ok(CustomStart::Selected(Box::new(session)));
        }
        current(&session, addons, context, cancel)?;
        if expected.is_some() {
            return Err(invalid("Required custom installer declined replay"));
        }
    }
    Ok(CustomStart::Fallback(session.into_tree()))
}

fn run(
    session: &CustomSession,
    receipt: &CustomReceipt,
    context: &Context,
    cancel: &AtomicBool,
) -> Result<Option<CustomEvaluation>, InstallError> {
    if receipt.answers.len() > 64 {
        return Err(invalid("Custom installer exceeds 64 recorded prompts"));
    }
    let mut answers = BTreeMap::new();
    for index in 0..=receipt.answers.len() {
        check_cancel(cancel)?;
        let request = Request {
            protocol: 1,
            request_id: format!("custom-{}-{index}", COUNTER.fetch_add(1, Ordering::Relaxed)),
            operation: Operation::Installer,
            game: session.game_id.clone(),
            input: session.archive.clone(),
            source: Some(session.tree.path().canonicalize()?),
            workspace: session.workspace.path().canonicalize()?,
            answers: answers.clone(),
        };
        let reply = protocol::invoke(&session.addon, context, &request, cancel).map_err(invalid)?;
        let state = match reply.outcome {
            Outcome::Prompt {
                id,
                title,
                options,
                multiple,
            } => {
                let prompt = CustomPrompt {
                    id,
                    title,
                    options,
                    multiple,
                };
                if let Some(recorded) = receipt.answers.get(index) {
                    if prompt != recorded.prompt
                        || (!prompt.multiple && recorded.selected.len() != 1)
                        || recorded.selected.iter().any(|&n| n >= prompt.options.len())
                        || recorded
                            .selected
                            .iter()
                            .collect::<std::collections::BTreeSet<_>>()
                            .len()
                            != recorded.selected.len()
                    {
                        return Err(invalid(
                            "Recorded custom installer prompt/answer is stale or invalid",
                        ));
                    }
                    answers.insert(prompt.id, recorded.selected.clone());
                    continue;
                }
                CustomEvaluation::Prompt(prompt)
            }
            Outcome::Handled {
                result: Payload::Install { files, warnings },
            } => CustomEvaluation::Ready(CustomPlan { files, warnings }),
            Outcome::Declined if index == 0 && receipt.answers.is_empty() => return Ok(None),
            Outcome::Declined => {
                return Err(invalid("Required installer declined recorded replay"))
            }
            Outcome::Manual { reason } => CustomEvaluation::Manual(reason),
            Outcome::Cancelled => CustomEvaluation::Cancelled,
            Outcome::Failed { message } => {
                return Err(invalid(format!("Custom installer failed: {message}")))
            }
            _ => return Err(invalid("Custom installer returned an unexpected payload")),
        };
        if index != receipt.answers.len() {
            return Err(invalid(
                "Recorded custom installer answers were not consumed",
            ));
        }
        if let Some(plan) = &receipt.plan {
            if state != CustomEvaluation::Ready(plan.clone()) {
                return Err(invalid("Custom installer plan changed during replay"));
            }
        }
        return Ok(Some(state));
    }
    Err(invalid("Custom installer prompt limit exceeded"))
}

pub fn evaluate_custom(
    session: &mut CustomSession,
    receipt: &CustomReceipt,
    addons: &[Addon],
    context: &Context,
    cancel: &AtomicBool,
) -> Result<CustomEvaluation, InstallError> {
    if !base_matches(receipt, &session.receipt) {
        return Err(invalid("Recorded custom installer identity is stale"));
    }
    current(session, addons, context, cancel)?;
    let state = run(session, receipt, context, cancel)?
        .ok_or_else(|| invalid("Required custom installer declined replay"))?;
    current(session, addons, context, cancel)?;
    session.receipt = receipt.clone();
    session.receipt.plan = if let CustomEvaluation::Ready(plan) = &state {
        Some(plan.clone())
    } else {
        None
    };
    session.state = state.clone();
    Ok(state)
}

/// Caller holds the instance mutation lock and supplies its current context.
#[allow(clippy::too_many_arguments)]
pub fn finish_custom(
    session: &CustomSession,
    mods_dir: &Path,
    name: &str,
    policy: OverwritePolicy,
    addons: &[Addon],
    context: &Context,
    cancel: &AtomicBool,
) -> Result<InstallReport, InstallError> {
    finish_custom_with_finish(
        session,
        mods_dir,
        name,
        policy,
        addons,
        context,
        cancel,
        |_| Ok(()),
    )
}

/// Apply a collection recipe inside checked staging before publication.
#[allow(clippy::too_many_arguments)]
pub fn finish_custom_with_finish(
    session: &CustomSession,
    mods_dir: &Path,
    name: &str,
    policy: OverwritePolicy,
    addons: &[Addon],
    context: &Context,
    cancel: &AtomicBool,
    finish: impl FnOnce(&Path) -> Result<(), InstallError>,
) -> Result<InstallReport, InstallError> {
    let CustomEvaluation::Ready(plan) = &session.state else {
        return Err(invalid(
            "Custom installer is awaiting a choice or requires manual handling",
        ));
    };
    current(session, addons, context, cancel)?;
    if run(session, &session.receipt, context, cancel)?
        != Some(CustomEvaluation::Ready(plan.clone()))
    {
        return Err(invalid("Custom installer plan changed before publication"));
    }
    current(session, addons, context, cancel)?;
    install_destination(
        &session.archive,
        mods_dir,
        name,
        &session.game_id,
        policy,
        |dest, _| {
            for file in &plan.files {
                check_cancel(cancel)?;
                let source =
                    protocol::regular_file(session.tree.path(), &file.source).map_err(invalid)?;
                let relative = protocol::relative_path(&file.source).map_err(invalid)?;
                let stamp = session
                    .sources
                    .get(relative.to_str().unwrap())
                    .ok_or_else(|| invalid("Selected source is absent from the original tree"))?;
                let destination = protocol::relative_path(&file.destination).map_err(invalid)?;
                let destination = checked_destination(dest, &dest.join(destination))?;
                fs::create_dir_all(destination.parent().unwrap())?;
                let mut input = File::open(source)?;
                if !input.metadata()?.is_file() {
                    return Err(invalid("Selected source is not regular"));
                }
                let mut output = File::options()
                    .write(true)
                    .create_new(true)
                    .open(&destination)?;
                let (mut size, mut hash) = (0u64, Sha256::new());
                let mut buffer = [0; 65536];
                loop {
                    check_cancel(cancel)?;
                    let n = input.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    size += n as u64;
                    if size > stamp.size {
                        return Err(invalid("Selected source grew before copying"));
                    }
                    hash.update(&buffer[..n]);
                    output.write_all(&buffer[..n])?;
                }
                if size != stamp.size || hex(&hash.finalize()) != stamp.sha256 {
                    return Err(invalid("Selected source changed before copying"));
                }
            }
            current(session, addons, context, cancel)?;
            finish(dest)?;
            current(session, addons, context, cancel)?;
            let receipt = encoded(&session.receipt)?;
            if receipt.len() > MAX_RECEIPT {
                return Err(invalid("Custom installer receipt exceeds 16 MiB"));
            }
            let mut output = File::options()
                .write(true)
                .create_new(true)
                .open(dest.join(RECEIPT_FILE))?;
            output.write_all(&receipt)?;
            output.sync_all()?;
            check_cancel(cancel)?;
            Ok((String::new(), false, Vec::new()))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        root: ExtractedTree,
        archive: PathBuf,
        mods: PathBuf,
        addons: Vec<Addon>,
        context: Context,
    }
    impl Fixture {
        fn new(mode: &str) -> Self {
            let tmp = std::env::temp_dir().join(format!(
                "eidos-custom-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&tmp).unwrap();
            let root = ExtractedTree::owned(tmp);
            let archive = root.path().join("archive.zip");
            fs::write(&archive, b"synthetic archive").unwrap();
            let mods = root.path().join("mods");
            fs::create_dir(&mods).unwrap();
            let script = root.path().join("helper.py");
            fs::write(&script,r#"import json,pathlib,sys,time
r=json.loads(pathlib.Path(sys.argv[1]).read_text());m=sys.argv[2]
if m=='timeout': time.sleep(35)
if m=='mutate': pathlib.Path(r['source'],'payload.txt').write_text('changed by helper')
if m=='bad_json': print('not json');sys.exit(0)
if m=='fail': print('failure',file=sys.stderr);sys.exit(7)
if m=='decline': o={'status':'declined'}
elif m=='manual': o={'status':'manual','reason':'Needs manual selection'}
elif m=='cancel': o={'status':'cancelled'}
elif m=='prompt' and not r['answers']: o={'status':'prompt','id':'readme','title':'Choose a file','options':['First','Second'],'multiple':False}
else:
 d='../escape' if m=='escape' else 'textures/copied.txt'
 files=[{'source':'payload.txt','destination':d}]
 if m=='collision': files.append({'source':'payload.txt','destination':'TEXTURES/COPIED.TXT'})
 o={'status':'handled','result':{'kind':'install','files':files,'warnings':['Synthetic warning retained']}}
print(json.dumps({'protocol':1,'request_id':'wrong' if m=='stale' else r['request_id'],'outcome':o}))
"#).unwrap();
            let manifest = root.path().join("installer.toml");
            let text=format!("id='synthetic'\nkind='installer'\nprotocol=1\nexec='/usr/bin/python3'\nargs=['{}','{{request}}','{}']\nextensions=['zip']\npriority=1\nversion='1'\n",script.display(),mode);
            fs::write(&manifest, &text).unwrap();
            let addon = eidos_addons::parse_addon(&text, &manifest).unwrap();
            let mut context = Context::default();
            context.values.insert("profile".into(), "Default".into());
            Self {
                root,
                archive,
                mods,
                addons: vec![addon],
                context,
            }
        }
        fn tree(&self) -> ExtractedTree {
            let tmp = self
                .root
                .path()
                .join(format!("tree-{}", COUNTER.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&tmp).unwrap();
            fs::write(tmp.join("payload.txt"), b"expected source").unwrap();
            let mut tree = ExtractedTree::owned(tmp);
            tree.origin = Some(
                super::super::ArchiveOrigin::capture(&self.archive, &AtomicBool::new(false))
                    .unwrap(),
            );
            tree
        }
        fn start(&self) -> Result<CustomStart, InstallError> {
            try_custom_installers(
                self.tree(),
                &self.archive,
                "skyrimse",
                &self.addons,
                &self.context,
                &AtomicBool::new(false),
            )
        }
        fn selected(&self) -> Box<CustomSession> {
            match self.start().unwrap() {
                CustomStart::Selected(s) => s,
                _ => panic!("helper declined"),
            }
        }
    }
    #[test]
    fn store_control_directories_are_bound_and_rechecked() {
        let f = Fixture::new("success");
        let instance = eidos_instance::Instance::portable(f.root.path().join("instance"));
        instance.create().unwrap();
        let install = f.root.path().join("game");
        let data = install.join("Data");
        fs::create_dir_all(&data).unwrap();
        let cancel = AtomicBool::new(false);
        for store in [
            "Skyrim Special Edition GOG",
            "Skyrim Special Edition EPIC",
            "FalloutNV_Epic",
        ] {
            let controls = f.root.path().join("prefix/LocalAppData").join(store);
            fs::create_dir_all(&controls).unwrap();
            let plugins = controls.join("plugins.txt");
            fs::write(&plugins, b"*Original.esp\n").unwrap();
            let context = context_for_instance(
                &instance,
                "skyrimse",
                &install,
                &data,
                None,
                &[controls],
                &cancel,
            )
            .unwrap();
            verify_context(&context, &cancel).unwrap();
            fs::write(&plugins, b"*Changed.esp\n").unwrap();
            assert!(verify_context(&context, &cancel).is_err(), "{store}");
        }
    }

    #[test]
    fn real_extraction_refuses_replacement_before_first_helper() {
        let f = Fixture::new("success");
        let source = f.root.path().join("zip-source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("payload.txt"), b"original zip payload").unwrap();
        fs::remove_file(&f.archive).unwrap();
        let status = std::process::Command::new(eidos_sevenzip::find_7z().unwrap())
            .current_dir(&source)
            .args(["a", "-tzip"])
            .arg(&f.archive)
            .arg(".")
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        let tree = extract_to_temp(&f.archive, &f.mods).unwrap();
        assert_eq!(
            fs::read(tree.path().join("payload.txt")).unwrap(),
            b"original zip payload"
        );
        let script = f.root.path().join("helper.py");
        let old_script = fs::read_to_string(&script).unwrap();
        fs::write(
            &script,
            old_script.replace(
                "r=json.loads",
                "pathlib.Path(__file__).with_suffix('.called').write_text('called')\nr=json.loads",
            ),
        )
        .unwrap();
        let held = f.root.path().join("held.zip");
        fs::rename(&f.archive, &held).unwrap();
        fs::write(&f.archive, b"replacement archive").unwrap();
        assert!(try_custom_installers(
            tree,
            &f.archive,
            "skyrimse",
            &f.addons,
            &f.context,
            &AtomicBool::new(false)
        )
        .is_err());
        assert!(!script.with_extension("called").exists());
    }

    #[test]
    fn instance_context_binds_effective_rows_and_disk_state_without_reconciliation_noise() {
        let f = Fixture::new("success");
        let instance = eidos_instance::Instance::portable(f.root.path().join("instance"));
        instance.create().unwrap();
        let install = f.root.path().join("game");
        let data = install.join("Data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("base.txt"), b"original").unwrap();
        fs::create_dir(instance.mods_dir().join("Reserved")).unwrap();
        let cancel = AtomicBool::new(false);
        let first =
            context_for_instance(&instance, "skyrimse", &install, &data, None, &[], &cancel)
                .unwrap();
        instance
            .active()
            .save_modlist(&instance.active().modlist())
            .unwrap();
        let reconciled =
            context_for_instance(&instance, "skyrimse", &install, &data, None, &[], &cancel)
                .unwrap();
        assert_eq!(first.values, reconciled.values);
        verify_context(&first, &cancel).unwrap();
        fs::write(data.join("base.txt"), b"changed").unwrap();
        assert!(verify_context(&first, &cancel).is_err());
        let changed =
            context_for_instance(&instance, "skyrimse", &install, &data, None, &[], &cancel)
                .unwrap();
        fs::write(
            instance.active().dir().join("settings.ini"),
            b"[General]\nflag=true\n",
        )
        .unwrap();
        assert!(verify_context(&changed, &cancel).is_err());
        let changed =
            context_for_instance(&instance, "skyrimse", &install, &data, None, &[], &cancel)
                .unwrap();
        fs::write(install.join("Game.exe"), b"synthetic executable identity").unwrap();
        assert!(verify_context(&changed, &cancel).is_err());
        let changed =
            context_for_instance(&instance, "skyrimse", &install, &data, None, &[], &cancel)
                .unwrap();
        let mut rows = instance.active().modlist();
        rows.iter_mut()
            .find(|m| m.name == "Reserved")
            .unwrap()
            .enabled = true;
        instance.active().save_modlist(&rows).unwrap();
        assert!(verify_context(&changed, &cancel).is_err());
        cancel.store(true, Ordering::Relaxed);
        assert!(
            context_for_instance(&instance, "skyrimse", &install, &data, None, &[], &cancel)
                .is_err()
        );
        assert!(may_handle(&f.addons, "skyrimse", &f.archive));
        assert!(!may_handle(&f.addons, "skyrimse", Path::new("other.7z")));
        assert!(!may_handle(&[], "skyrimse", &f.archive));
    }

    #[test]
    fn helper_prompt_receipt_replays_and_publishes_only_the_approved_files() {
        let f = Fixture::new("prompt");
        let mut session = f.selected();
        let CustomEvaluation::Prompt(prompt) = session.state.clone() else {
            panic!("prompt missing")
        };
        let mut receipt = session.receipt();
        receipt.answers.push(CustomRecordedAnswer {
            prompt,
            selected: vec![0],
        });
        let state = evaluate_custom(
            &mut session,
            &receipt,
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(
            matches!(state,CustomEvaluation::Ready(ref p) if p.warnings==["Synthetic warning retained"])
        );
        let saved = serde_json::to_vec(&session.receipt()).unwrap();
        let mut reopened = f.selected();
        evaluate_custom(
            &mut reopened,
            &serde_json::from_slice(&saved).unwrap(),
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
        )
        .unwrap();
        let report = finish_custom(
            &reopened,
            &f.mods,
            "Installed",
            OverwritePolicy::Fail,
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            fs::read(report.dest.join("textures/copied.txt")).unwrap(),
            b"expected source"
        );
        assert!(!report.dest.join("payload.txt").exists());
        assert!(report.dest.join("meta.ini").is_file());
        assert!(finish_custom(
            &reopened,
            &f.mods,
            "Installed",
            OverwritePolicy::Fail,
            &f.addons,
            &f.context,
            &AtomicBool::new(false)
        )
        .is_err());
        fs::write(report.dest.join("keep.txt"), b"old").unwrap();
        assert!(finish_custom(
            &reopened,
            &f.mods,
            "Installed",
            OverwritePolicy::Merge,
            &f.addons,
            &f.context,
            &AtomicBool::new(false)
        )
        .is_err());
        assert_eq!(fs::read(report.dest.join("keep.txt")).unwrap(), b"old");
    }
    #[test]
    fn custom_dispatch_preserves_fomod_priority_and_explicit_fallback_ownership() {
        let mut f = Fixture::new("success");
        let tree = f.tree();
        fs::create_dir(tree.path().join("fomod")).unwrap();
        fs::write(tree.path().join("fomod/ModuleConfig.xml"), b"<config/>").unwrap();
        let CustomStart::Fallback(tree) = try_custom_installers(
            tree,
            &f.archive,
            "skyrimse",
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
        )
        .unwrap() else {
            panic!("FOMOD hijacked")
        };
        f.addons[0].priority = 91;
        assert!(matches!(
            try_custom_installers(
                tree,
                &f.archive,
                "skyrimse",
                &f.addons,
                &f.context,
                &AtomicBool::new(false)
            )
            .unwrap(),
            CustomStart::Selected(_)
        ));
        let f = Fixture::new("decline");
        let CustomStart::Fallback(tree) = f.start().unwrap() else {
            panic!("decline not forwarded")
        };
        assert!(tree.path().join("payload.txt").is_file());
        let f = Fixture::new("manual");
        let session = f.selected();
        assert!(matches!(session.state, CustomEvaluation::Manual(_)));
        assert!(session.into_tree().path().join("payload.txt").is_file());
        let f = Fixture::new("cancel");
        assert_eq!(f.selected().state, CustomEvaluation::Cancelled);
    }
    #[test]
    fn custom_rejects_malformed_stale_escape_collisions_and_cancelled_work() {
        for mode in ["bad_json", "fail", "stale", "escape", "collision", "mutate"] {
            let f = Fixture::new(mode);
            assert!(f.start().is_err(), "{mode}");
            assert_eq!(fs::read_dir(&f.mods).unwrap().count(), 0);
        }
        let f = Fixture::new("success");
        assert!(try_custom_installers(
            f.tree(),
            &f.archive,
            "skyrimse",
            &f.addons,
            &f.context,
            &AtomicBool::new(true)
        )
        .is_err());
    }
    #[test]
    fn custom_replay_rejects_changed_prompt_archive_context_sources_and_helper() {
        let f = Fixture::new("prompt");
        let mut s = f.selected();
        let CustomEvaluation::Prompt(mut p) = s.state.clone() else {
            panic!()
        };
        p.title = "changed".into();
        let mut r = s.receipt();
        r.answers.push(CustomRecordedAnswer {
            prompt: p,
            selected: vec![0],
        });
        assert!(
            evaluate_custom(&mut s, &r, &f.addons, &f.context, &AtomicBool::new(false)).is_err()
        );
        for change in [
            "archive",
            "context",
            "source",
            "script",
            "manifest",
            "unavailable",
        ] {
            let f = Fixture::new("success");
            let s = f.selected();
            let mut context = f.context.clone();
            let mut addons = f.addons.clone();
            match change {
                "archive" => fs::write(&f.archive, b"changed").unwrap(),
                "context" => {
                    context.values.insert("profile".into(), "Another".into());
                }
                "source" => fs::write(s.tree.path().join("payload.txt"), b"changed").unwrap(),
                "script" => fs::write(f.root.path().join("helper.py"), b"changed").unwrap(),
                "manifest" => fs::write(&addons[0].source, b"changed").unwrap(),
                _ => addons.clear(),
            }
            assert!(
                finish_custom(
                    &s,
                    &f.mods,
                    "Target",
                    OverwritePolicy::Fail,
                    &addons,
                    &context,
                    &AtomicBool::new(false)
                )
                .is_err(),
                "{change}"
            );
            assert!(!f.mods.join("Target").exists());
        }
    }
    #[test]
    fn custom_sources_are_revalidated_before_replace_and_reject_links() {
        let f = Fixture::new("success");
        let s = f.selected();
        let dest = f.mods.join("Target");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("keep"), b"original").unwrap();
        fs::remove_file(s.tree.path().join("payload.txt")).unwrap();
        std::os::unix::fs::symlink(&f.archive, s.tree.path().join("payload.txt")).unwrap();
        assert!(finish_custom(
            &s,
            &f.mods,
            "Target",
            OverwritePolicy::Replace,
            &f.addons,
            &f.context,
            &AtomicBool::new(false)
        )
        .is_err());
        assert_eq!(fs::read(dest.join("keep")).unwrap(), b"original");
        assert!(f.start().is_ok());
    }
    #[test]
    fn replay_refuses_changed_helper_before_executing_and_reads_installed_receipt() {
        let f = Fixture::new("success");
        let session = f.selected();
        let receipt = session.receipt();
        let report = finish_custom(
            &session,
            &f.mods,
            "Target",
            OverwritePolicy::Fail,
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            read_installed_receipt(&report.dest).unwrap(),
            Some(receipt.clone())
        );
        let marker = f.root.path().join("must-not-run");
        fs::write(
            f.root.path().join("helper.py"),
            format!(
                "import pathlib\npathlib.Path({:?}).write_text('ran')\n",
                marker.to_str().unwrap()
            ),
        )
        .unwrap();
        assert!(try_custom_installers_with_receipt(
            f.tree(),
            &f.archive,
            "skyrimse",
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
            Some(&receipt)
        )
        .is_err());
        assert!(!marker.exists());
        assert!(try_custom_installers_with_receipt(
            f.tree(),
            &f.archive,
            "skyrimse",
            &[],
            &f.context,
            &AtomicBool::new(false),
            Some(&receipt)
        )
        .is_err());
        fs::write(report.dest.join(RECEIPT_FILE), b"corrupt").unwrap();
        assert!(read_installed_receipt(&report.dest).is_err());
    }
    #[test]
    fn collection_callback_runs_inside_staging_and_failure_preserves_destination() {
        let f = Fixture::new("success");
        let session = f.selected();
        let dest = f.mods.join("Target");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("old"), b"original").unwrap();
        assert!(finish_custom_with_finish(
            &session,
            &f.mods,
            "Target",
            OverwritePolicy::Replace,
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
            |stage| {
                assert!(stage.join("textures/copied.txt").is_file());
                Err(invalid("synthetic transform failure"))
            }
        )
        .is_err());
        assert_eq!(fs::read(dest.join("old")).unwrap(), b"original");
        let report = finish_custom_with_finish(
            &session,
            &f.mods,
            "Target",
            OverwritePolicy::Replace,
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
            |stage| {
                fs::write(stage.join("added.txt"), b"transformed")?;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            fs::read(report.dest.join("added.txt")).unwrap(),
            b"transformed"
        );
        assert!(read_installed_receipt(&report.dest).unwrap().is_some());
    }
    #[test]
    fn callback_input_change_refuses_publication_and_retains_previous_mod() {
        let f = Fixture::new("success");
        let session = f.selected();
        let dest = f.mods.join("Target");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("personal.txt"), b"old bytes").unwrap();
        let result = finish_custom_with_finish(
            &session,
            &f.mods,
            "Target",
            OverwritePolicy::Replace,
            &f.addons,
            &f.context,
            &AtomicBool::new(false),
            |_| {
                fs::write(&f.archive, b"changed during final callback")?;
                Ok(())
            },
        );
        assert!(
            result.is_err(),
            "stale reviewed inputs must not be published"
        );
        assert_eq!(fs::read(dest.join("personal.txt")).unwrap(), b"old bytes");
        assert!(!dest.join("textures/copied.txt").exists());
        assert!(!dest.join(RECEIPT_FILE).exists());
    }

    #[test]
    fn custom_order_is_priority_then_id_and_decline_advances_once() {
        let mut f = Fixture::new("success");
        let mut high = f.addons[0].clone();
        high.priority = 10;
        high.id = "z-last".into();
        let mut first = high.clone();
        first.id = "a-first".into();
        f.addons.extend([high, first]);
        assert_eq!(f.selected().receipt().installer, "a-first");
        f.addons
            .last_mut()
            .unwrap()
            .args
            .last_mut()
            .unwrap()
            .clone_from(&"decline".to_string());
        assert_eq!(f.selected().receipt().installer, "z-last");
    }
}
