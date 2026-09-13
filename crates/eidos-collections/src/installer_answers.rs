//! Exact, local installer decisions retained inside a collection's owned reservation.
use eidos_install::{
    custom::{CustomPrompt, CustomReceipt},
    obmm::ObmmPrompt,
    scripted::OmodReplayReceipt,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const NAME: &str = ".eidos-collection-installer.json";
const LIMIT: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum InstallerAnswers {
    Custom {
        receipt: CustomReceipt,
        prompt: Option<CustomPrompt>,
    },
    Omod {
        receipt: OmodReplayReceipt,
        prompt: Option<ObmmPrompt>,
        #[serde(default)]
        apply_profile_effects: bool,
        #[serde(default)]
        allow_incomplete: bool,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    owner: String,
    archive: PathBuf,
    answers: InstallerAnswers,
}

pub fn path(folder: &Path) -> PathBuf {
    folder.join(NAME)
}
fn owned(folder: &Path, owner: &str) -> Result<(), String> {
    if !fs::symlink_metadata(folder).is_ok_and(|m| m.is_dir())
        || eidos_instance::ModMeta::read(&folder.join("meta.ini")).collection_owner() != Some(owner)
    {
        return Err("Installer answers require the collection's owned reservation".into());
    }
    Ok(())
}
/// Read only after checking the current collection/member ownership token.
fn read_record(folder: &Path, owner: &str) -> Result<Option<Record>, String> {
    owned(folder, owner)?;
    let path = path(folder);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(m) if m.is_file() && m.len() <= LIMIT as u64 => {}
        _ => return Err("Installer answers are not a bounded regular file".into()),
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("Installer answers exceed 16 MiB".into());
    }
    let record: Record =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid installer answers: {e}"))?;
    if record.version != 1 || record.owner != owner {
        return Err("Installer answers belong to a different collection member".into());
    }
    Ok(Some(record))
}
pub fn read(folder: &Path, owner: &str) -> Result<Option<InstallerAnswers>, String> {
    Ok(read_record(folder, owner)?.map(|r| r.answers))
}
pub fn archive(folder: &Path, owner: &str) -> Result<Option<PathBuf>, String> {
    Ok(read_record(folder, owner)?.map(|r| r.archive))
}
/// Atomically persist an answered prompt or review before replay; never change its receipt identity.
pub fn write(folder: &Path, owner: &str, answers: &InstallerAnswers) -> Result<(), String> {
    let archive = read_record(folder, owner)?
        .ok_or("No original archive is recorded for these answers")?
        .archive;
    write_for_archive(folder, owner, &archive, answers)
}
pub fn write_for_archive(
    folder: &Path,
    owner: &str,
    archive: &Path,
    answers: &InstallerAnswers,
) -> Result<(), String> {
    owned(folder, owner)?;
    let bytes = serde_json::to_vec_pretty(&Record {
        version: 1,
        owner: owner.into(),
        archive: archive.canonicalize().map_err(|e| e.to_string())?,
        answers: answers.clone(),
    })
    .map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("Installer answers exceed 16 MiB".into());
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp = folder.join(format!(
        "{NAME}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        owned(folder, owner)?;
        fs::rename(&temp, path(folder)).map_err(|e| e.to_string())?;
        fs::File::open(folder)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
