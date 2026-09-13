//! Collection transformations run only inside unpublished installer staging.
use crate::{Collection, Mod, SourceType};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const RECEIPT: &str = ".eidos-collection-recipe.json";
// ponytail: patch/base/output capped at 512 MiB; raise after measuring larger real recipes.
const PATCH_LIMIT: u64 = 512 * 1024 * 1024;
const CONTROL_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeCheck {
    Unrestricted,
    Compatible {
        observed: String,
    },
    Mismatch {
        observed: String,
        expected: Vec<String>,
    },
    Unknown {
        reason: String,
    },
}
impl RuntimeCheck {
    pub fn message(&self) -> String {
        match self {
            Self::Unrestricted => "No runtime versions declared".into(),
            Self::Compatible { observed } => format!("Runtime {observed} matches the collection"),
            Self::Mismatch { observed, expected } => format!(
                "Installed runtime {observed}; collection expects {}. Explicit continuation is required before installing members",
                expected.join(", ")
            ),
            Self::Unknown { reason } => format!("Runtime compatibility is unverified: {reason}"),
        }
    }
}
pub fn compare_runtime(expected: &[String], observed: Result<String, String>) -> RuntimeCheck {
    if expected.is_empty() {
        return RuntimeCheck::Unrestricted;
    }
    match observed {
        Ok(observed) if expected.contains(&observed) => RuntimeCheck::Compatible { observed },
        Ok(observed) => RuntimeCheck::Mismatch {
            observed,
            expected: expected.to_vec(),
        },
        Err(reason) => RuntimeCheck::Unknown { reason },
    }
}
pub fn runtime_check(c: &Collection, game: &eidos_games::DetectedGame) -> RuntimeCheck {
    let observed =
        eidos_gamefeatures::preflight::inspect_pe(&game.install_path.join(game.def.game_binary))
            .and_then(|pe| {
                pe.file_version
                    .map(|v| v.to_string())
                    .ok_or_else(|| "Executable has no readable file-version resource".into())
            });
    compare_runtime(&c.info.game_versions, observed)
}

fn digest(value: &str, digits: usize) -> Result<(), String> {
    if value.len() != digits || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("Malformed {digits}-digit digest: {value}"));
    }
    Ok(())
}
/// Literal portable path; reject aliases Windows would collapse or interpret as streams.
pub fn relative(raw: &str) -> Result<PathBuf, String> {
    let p = raw.replace('\\', "/");
    if p.is_empty()
        || p.starts_with('/')
        || p.split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.ends_with([' ', '.'])
                || s.chars().any(|c| c.is_control() || ":*?\"<>|".contains(c))
        })
    {
        return Err(format!("Unsafe or unmappable recipe path: {raw}"));
    }
    Ok(PathBuf::from(p))
}
fn installed_path(raw: &str) -> Result<PathBuf, String> {
    let path = relative(raw)?;
    if path.components().next().is_some_and(|c| {
        let name = c.as_os_str().to_string_lossy();
        [
            "meta.ini",
            RECEIPT,
            ".eidos-custom-installer.json",
            ".eidos-omod.mohidden",
            ".eidos-collection-installer.json",
        ]
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
    }) {
        return Err(format!(
            "Recipe path conflicts with installer metadata: {raw}"
        ));
    }
    Ok(path)
}
fn unique_paths<'a>(paths: impl Iterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for raw in paths {
        let path = installed_path(raw)?.to_string_lossy().to_lowercase();
        if seen.iter().any(|old: &String| {
            old == &path
                || old.starts_with(&format!("{path}/"))
                || path.starts_with(&format!("{old}/"))
        }) {
            return Err(format!("Colliding recipe destination: {raw}"));
        }
        seen.insert(path);
    }
    Ok(())
}
pub fn validate_member(m: &Mod) -> Result<(), String> {
    if !m.source.md5.is_empty() {
        digest(&m.source.md5, 32)?;
    }
    if m.source.kind == SourceType::Bundle {
        relative(&m.source.file_expression)?;
    }
    unique_paths(m.hashes.iter().map(|h| h.path.as_str()))?;
    for h in &m.hashes {
        digest(&h.md5, 32)?;
    }
    unique_paths(m.patches.keys().map(String::as_str))?;
    if !m.patches.is_empty() {
        relative(&m.name)?;
    }
    for crc in m.patches.values() {
        digest(crc, 8)?;
    }
    unique_paths(m.file_overrides.iter().map(String::as_str))
}
/// Only exact known roots can rebase foreign paths. Wine's Z: maps the Unix root.
/// Arbitrary Windows installation prefixes remain an actionable error.
pub fn map_exclusions(m: &Mod, game_root: &Path, data_root: &Path) -> Result<Mod, String> {
    let mut mapped = m.clone();
    for raw in &mut mapped.file_overrides {
        if relative(raw).is_ok() {
            continue;
        }
        let normalized = raw.replace('\\', "/");
        let lower = normalized.to_lowercase();
        let mut matches = BTreeSet::new();
        for (root, prefix) in [(data_root, ""), (game_root, "Root/")] {
            let root = root.to_string_lossy().replace('\\', "/");
            for known in [root.clone(), format!("Z:{root}")] {
                let known = format!("{}/", known.trim_end_matches('/'));
                if lower.starts_with(&known.to_lowercase()) {
                    let tail = &normalized[known.len()..];
                    // Game/Data paths use Data-relative placement, never Root/Data.
                    if prefix == "Root/"
                        && data_root.starts_with(game_root)
                        && lower.starts_with(
                            &format!("{}/", data_root.to_string_lossy()).to_lowercase(),
                        )
                    {
                        continue;
                    }
                    matches.insert(format!("{prefix}{tail}"));
                }
            }
        }
        // The more specific data-root mapping owns game-root descendants.
        if matches.len() > 1 {
            matches.retain(|p| !p.starts_with("Root/"));
        }
        if matches.len() != 1 {
            return Err(format!(
                "Absolute file exclusion has no unique known game/data-root mapping: {raw}"
            ));
        }
        *raw = matches.into_iter().next().unwrap();
    }
    validate_member(&mapped)?;
    Ok(mapped)
}
pub fn validate_archive(m: &Mod, archive: &Path) -> Result<(), String> {
    validate_member(m)?;
    let meta = fs::symlink_metadata(archive).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() {
        return Err("The selected archive is not a regular file".into());
    }
    if m.source.file_size.is_some_and(|n| n != meta.len()) {
        return Err(format!(
            "Archive size mismatch for {}: expected {:?}, got {}; archive retained",
            m.name,
            m.source.file_size,
            meta.len()
        ));
    }
    if !m.source.md5.is_empty()
        && !eidos_nexus::md5_file(archive)
            .map_err(|e| e.to_string())?
            .eq_ignore_ascii_case(&m.source.md5)
    {
        return Err(format!(
            "Archive MD5 mismatch for {}; archive retained",
            m.name
        ));
    }
    Ok(())
}
fn child(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let mut current = root.to_path_buf();
    if !fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_dir()
    {
        return Err(format!("Not a real directory: {}", root.display()));
    }
    for part in relative.components() {
        let wanted = part.as_os_str().to_string_lossy();
        let matches = fs::read_dir(&current)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&wanted)
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!(
                "Missing or ambiguous recipe path: {}",
                root.join(relative).display()
            ));
        }
        current = matches[0].path();
        if fs::symlink_metadata(&current)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err(format!("Symlink in recipe source: {}", current.display()));
        }
    }
    Ok(current)
}
fn files(root: &Path, directories: bool) -> Result<BTreeMap<String, PathBuf>, String> {
    fn visit(
        root: &Path,
        dir: &Path,
        depth: usize,
        directories: bool,
        out: &mut BTreeMap<String, PathBuf>,
        seen: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if depth > 128 || out.len() > 1_000_000 {
            return Err("Collection tree exceeds traversal limits".into());
        }
        if !fs::symlink_metadata(dir)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_dir()
        {
            return Err(format!(
                "Not a real collection directory: {}",
                dir.display()
            ));
        }
        for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            let path = e.path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .ok_or("Recipe tree contains a non-UTF-8 filename")?
                .to_string();
            relative(&rel)?;
            if !seen.insert(rel.to_lowercase()) {
                return Err(format!("Ambiguous case-colliding collection path: {rel}"));
            }
            let ty = e.file_type().map_err(|e| e.to_string())?;
            if ty.is_dir() {
                if directories {
                    out.insert(format!("{rel}/"), path.clone());
                }
                visit(root, &path, depth + 1, directories, out, seen)?;
            } else if ty.is_file() {
                out.insert(rel, path);
            } else {
                return Err(format!("Symlink or special file in collection tree: {rel}"));
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(root, root, 0, directories, &mut out, &mut BTreeSet::new())?;
    Ok(out)
}
pub fn bundle_source(m: &Mod, payload: &Path) -> Result<PathBuf, String> {
    validate_member(m)?;
    let path = child(
        payload,
        &Path::new("bundled").join(relative(&m.source.file_expression)?),
    )?;
    let mut size = 0u64;
    for file in files(&path, false)?.values() {
        size = size
            .checked_add(fs::metadata(file).map_err(|e| e.to_string())?.len())
            .ok_or("Bundle size overflow")?;
    }
    if m.source.file_size.is_some_and(|expected| expected != size) {
        return Err(format!(
            "Bundle uncompressed size mismatch: expected {:?}, got {size}",
            m.source.file_size
        ));
    }
    Ok(path)
}
fn output_dir(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    match fs::create_dir(root) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    if !fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_dir()
    {
        return Err("Recipe destination is not a real staging directory".into());
    }
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let matches = fs::read_dir(&current)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&component.as_os_str().to_string_lossy())
            })
            .collect::<Vec<_>>();
        current = match matches.as_slice() {
            [] => {
                let path = current.join(component.as_os_str());
                fs::create_dir(&path).map_err(|e| e.to_string())?;
                path
            }
            [entry] => entry.path(),
            _ => return Err("Ambiguous case-colliding staging directory".into()),
        };
        if !fs::symlink_metadata(&current)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_dir()
        {
            return Err("Symlink or file in recipe destination parents".into());
        }
    }
    Ok(current)
}
fn copy_directories(source: &Path, dest: &Path, relative: &Path) -> Result<(), String> {
    output_dir(dest, relative)?;
    for e in fs::read_dir(source.join(relative)).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        if e.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_directories(source, dest, &relative.join(e.file_name()))?;
        }
    }
    Ok(())
}

/// Hash-list mode owns selection; a bundle without hashes is an exact installed tree.
pub fn validate_installer_selection(m: &Mod, stage: &Path) -> Result<(), String> {
    if m.hashes.is_empty() {
        return Ok(());
    }
    let selected = files(stage, false)?;
    if selected.len() != m.hashes.len() {
        return Err(
            "Collection installer selection conflicts with the exact hash recipe file count".into(),
        );
    }
    for expected in &m.hashes {
        let path = child(stage, &installed_path(&expected.path)?)
            .map_err(|e| format!("Collection installer selection conflicts: {e}"))?;
        if !eidos_nexus::md5_file(&path)
            .map_err(|e| e.to_string())?
            .eq_ignore_ascii_case(&expected.md5)
        {
            return Err(format!(
                "Collection installer selection conflicts with recipe path {}",
                expected.path
            ));
        }
    }
    Ok(())
}

pub fn populate(m: &Mod, source: &Path, dest: &Path) -> Result<(), String> {
    validate_member(m)?;
    let source_files = files(source, false)?;
    let mut plan = Vec::new();
    if m.hashes.is_empty() {
        copy_directories(source, dest, Path::new(""))?;
        plan.extend(source_files.into_iter().map(|(p, s)| (PathBuf::from(p), s)));
    } else {
        let mut index = BTreeMap::new();
        for source in source_files.values() {
            index
                .entry(eidos_nexus::md5_file(source).map_err(|e| e.to_string())?)
                .or_insert(source);
        }
        for h in &m.hashes {
            let source = index
                .get(&h.md5.to_ascii_lowercase())
                .ok_or_else(|| format!("Missing clone content {} for {}", h.md5, h.path))?;
            plan.push((installed_path(&h.path)?, (*source).clone()));
        }
    }
    for (rel, source) in plan {
        let parent = output_dir(dest, rel.parent().unwrap())?;
        let target = parent.join(rel.file_name().unwrap());
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| e.to_string())?;
        io::copy(
            &mut fs::File::open(&source).map_err(|e| e.to_string())?,
            &mut output,
        )
        .map_err(|e| e.to_string())?;
        fs::set_permissions(
            &target,
            fs::metadata(&source)
                .map_err(|e| e.to_string())?
                .permissions(),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
struct Capped<W> {
    writer: W,
    remaining: u64,
}
impl<W: Write> Write for Capped<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other("Patch output exceeds declared limit"));
        }
        let n = self.writer.write(bytes)?;
        self.remaining -= n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("Patch input exceeds {limit} byte limit"));
    }
    Ok(bytes)
}
fn patch_header(bytes: &[u8], limit: u64) -> Result<(usize, usize, u64), String> {
    if bytes.len() < 32 || &bytes[..8] != b"BSDIFF40" || bytes.len() as u64 > PATCH_LIMIT {
        return Err("Invalid or oversized BSDIFF40 patch".into());
    }
    let number = |start: usize| -> Result<u64, String> {
        if bytes[start + 7] & 0x80 != 0 {
            return Err("Negative BSDIFF40 length".into());
        }
        Ok(u64::from_le_bytes(
            bytes[start..start + 8].try_into().unwrap(),
        ))
    };
    let ctrl = number(8)?;
    let diff = number(16)?;
    let target = number(24)?;
    let end = 32u64
        .checked_add(ctrl)
        .and_then(|n| n.checked_add(diff))
        .ok_or("BSDIFF40 length overflow")?;
    if end > bytes.len() as u64 || target > limit {
        return Err("BSDIFF40 stream or target exceeds bounds".into());
    }
    Ok((32 + ctrl as usize, end as usize, target))
}
pub fn patch_file(path: &Path, patch: &[u8], crc: &str, limit: u64) -> Result<(), String> {
    digest(crc, 8)?;
    let limit = limit.min(PATCH_LIMIT);
    let (ctrl_end, diff_end, target) = patch_header(patch, limit)?;
    // Validate bounded streams and complete control records before handing bytes to qbsdiff.
    for (stream, cap, control) in [
        (&patch[32..ctrl_end], CONTROL_LIMIT, true),
        (&patch[ctrl_end..diff_end], limit, false),
        (&patch[diff_end..], limit, false),
    ] {
        let mut reader = bzip2::read::BzDecoder::new(stream);
        let len = io::copy(&mut reader.by_ref().take(cap + 1), &mut io::sink())
            .map_err(|e| e.to_string())?;
        if len > cap || control && len % 24 != 0 || reader.total_in() != stream.len() as u64 {
            return Err("Corrupt or oversized BSDIFF40 stream".into());
        }
    }
    let base = bounded_read(path, limit)?;
    if format!("{:08X}", crc32fast::hash(&base)) != crc.to_ascii_uppercase() {
        return Err(format!("Original CRC32 mismatch for {}", path.display()));
    }
    let temp = path.with_file_name(format!(
        ".eidos-patch-{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let patcher = qbsdiff::Bspatch::new(patch).map_err(|e| e.to_string())?;
        let written = patcher
            .apply(
                &base,
                Capped {
                    writer: &mut output,
                    remaining: target,
                },
            )
            .map_err(|e| e.to_string())?;
        if written != target {
            return Err("BSDIFF40 target length mismatch".into());
        }
        fs::set_permissions(
            &temp,
            fs::metadata(path).map_err(|e| e.to_string())?.permissions(),
        )
        .map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
fn patch_source(m: &Mod, payload: &Path, path: &str) -> Result<PathBuf, String> {
    child(
        payload,
        &Path::new("patches")
            .join(relative(&m.name)?)
            .join(relative(&format!("{path}.diff"))?),
    )
}
fn tree_digests(root: &Path) -> Result<BTreeMap<String, String>, String> {
    files(root, true)?
        .into_iter()
        .filter(|(p, _)| {
            let p = p.to_ascii_lowercase();
            p != "meta.ini"
                && p != RECEIPT
                && p != ".eidos-custom-installer.json"
                && !p.starts_with(".eidos-omod.mohidden/")
                && p != ".eidos-collection-installer.json"
        })
        .map(|(p, file)| {
            Ok((
                p.clone(),
                if p.ends_with('/') {
                    String::new()
                } else {
                    eidos_nexus::md5_file(&file).map_err(|e| e.to_string())?
                },
            ))
        })
        .collect()
}
fn identity(m: &Mod, payload: &Path) -> Result<serde_json::Value, String> {
    let mut patches = BTreeMap::new();
    for path in m.patches.keys() {
        patches.insert(
            path,
            eidos_nexus::md5_file(&patch_source(m, payload, path)?).map_err(|e| e.to_string())?,
        );
    }
    let bundle = if m.source.kind == SourceType::Bundle {
        Some(tree_digests(&bundle_source(m, payload)?)?)
    } else {
        None
    };
    Ok(serde_json::json!({"member":m,"patches":patches,"bundle":bundle}))
}
#[derive(Serialize, Deserialize)]
struct Receipt {
    schema: u32,
    owner: String,
    recipe: serde_json::Value,
    files: BTreeMap<String, String>,
    exclusions: BTreeMap<String, String>,
}
pub fn finish(m: &Mod, payload: &Path, stage: &Path, owner: &str) -> Result<(), String> {
    validate_member(m)?;
    // Host receipts are added only after this callback. Payloads cannot grant
    // themselves replay or pending-effect authority, even under another casing.
    for entry in fs::read_dir(stage).map_err(|e| e.to_string())? {
        let name = entry.map_err(|e| e.to_string())?.file_name();
        if [
            RECEIPT,
            ".eidos-custom-installer.json",
            ".eidos-omod.mohidden",
            ".eidos-collection-installer.json",
        ]
        .iter()
        .any(|reserved| name.to_string_lossy().eq_ignore_ascii_case(reserved))
        {
            return Err("Source payload collides with host installer receipt metadata".into());
        }
    }
    for (path, crc) in &m.patches {
        let target = child(stage, &installed_path(path)?)?;
        let patch = bounded_read(&patch_source(m, payload, path)?, PATCH_LIMIT)?;
        patch_file(&target, &patch, crc, PATCH_LIMIT)?;
    }
    let mut exclusions = BTreeMap::new();
    for path in &m.file_overrides {
        let target = child(stage, &installed_path(path)?)?;
        if !fs::symlink_metadata(&target)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_file()
        {
            return Err(format!("Excluded path is not a file: {path}"));
        }
        let hidden = target.with_file_name(format!(
            "{}.mohidden",
            target.file_name().unwrap().to_string_lossy()
        ));
        if fs::read_dir(hidden.parent().unwrap())
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&hidden.file_name().unwrap().to_string_lossy())
            })
        {
            return Err(format!(
                "Excluded file collides with existing hidden file: {}",
                hidden.display()
            ));
        }
        fs::rename(&target, &hidden).map_err(|e| e.to_string())?;
        exclusions.insert(
            path.clone(),
            hidden
                .strip_prefix(stage)
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );
    }
    let receipt = Receipt {
        schema: 1,
        owner: owner.into(),
        recipe: identity(m, payload)?,
        files: tree_digests(stage)?,
        exclusions,
    };
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(stage.join(RECEIPT))
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut output, &receipt).map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())
}
pub fn verify_receipt(m: &Mod, payload: &Path, folder: &Path, owner: &str) -> Result<bool, String> {
    let path = folder.join(RECEIPT);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Ok(meta) if meta.file_type().is_file() => {}
        _ => return Err("Collection receipt is not a readable regular file".into()),
    }
    let receipt: Receipt = match serde_json::from_slice(&bounded_read(&path, 64 * 1024 * 1024)?) {
        Ok(r) => r,
        Err(_) => return Ok(false),
    };
    Ok(receipt.schema == 1
        && receipt.owner == owner
        && receipt.recipe == identity(m, payload)?
        && receipt.files == tree_digests(folder)?)
}
