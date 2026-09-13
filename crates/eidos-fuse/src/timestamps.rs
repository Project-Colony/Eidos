//! Profile-owned plugin timestamp projection. Payload files remain in their providers.
use crate::{Eidos, LockExt};
use fuser::TimeOrNow;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct PluginTimestamps {
    /// Case-insensitive plugin filenames in the Data root, mapped to visible mtimes.
    pub times: BTreeMap<String, SystemTime>,
    /// A profile-owned file outside the mounted tree; capture reads it after exit.
    pub state_path: PathBuf,
}

pub(crate) struct ProjectionState {
    projection: PluginTimestamps,
    dirty: bool,
}

const MAX_RECEIPT_BYTES: usize = 4 * 1024 * 1024;

fn recovery_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".pending");
    PathBuf::from(name)
}

fn has_recovery(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(recovery_path(path)) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}
fn key(name: &str) -> io::Result<String> {
    let lower = name.to_ascii_lowercase();
    if name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
        || ![".esp", ".esm", ".esl"]
            .iter()
            .any(|ext| lower.ends_with(ext))
        || lower.starts_with('.')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid projected plugin filename",
        ));
    }
    Ok(lower)
}
fn number(time: SystemTime) -> i128 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as i128,
        Err(e) => -(e.duration().as_nanos() as i128),
    }
}
fn time(value: i128) -> io::Result<SystemTime> {
    let nanos = value.unsigned_abs();
    let secs =
        u64::try_from(nanos / 1_000_000_000).map_err(|_| io::Error::other("Timestamp overflow"))?;
    let d = Duration::new(secs, (nanos % 1_000_000_000) as u32);
    (if value >= 0 {
        UNIX_EPOCH.checked_add(d)
    } else {
        UNIX_EPOCH.checked_sub(d)
    })
    .ok_or_else(|| io::Error::other("Timestamp overflow"))
}
/// Read a durable capture. A sibling `.pending` receipt blocks the primary so
/// capture cannot silently import an older order. Read that pending path
/// explicitly or call `recover_plugin_mtimes` after unmount to recover it.
pub fn read_plugin_mtimes(path: &Path) -> io::Result<BTreeMap<String, SystemTime>> {
    if has_recovery(path)? {
        return Err(io::Error::other(format!(
            "Pending timestamp recovery at {}; refusing stale primary receipt",
            recovery_path(path).display()
        )));
    }
    let mut body = String::new();
    fs::File::open(path)?
        .take(MAX_RECEIPT_BYTES as u64 + 1)
        .read_to_string(&mut body)?;
    if body.len() > MAX_RECEIPT_BYTES {
        return Err(io::Error::other("Timestamp receipt exceeds limit"));
    }
    let mut lines = body.lines();
    if lines.next() != Some("Eidos plugin timestamps v1") {
        return Err(io::Error::other("Invalid timestamp receipt"));
    }
    let mut times = BTreeMap::new();
    for line in lines {
        let (n, name) = line
            .split_once('\t')
            .ok_or_else(|| io::Error::other("Invalid timestamp entry"))?;
        let n = n
            .parse::<i128>()
            .map_err(|_| io::Error::other("Invalid timestamp value"))?;
        if times.insert(key(name)?, time(n)?).is_some() {
            return Err(io::Error::other("Duplicate timestamp filename"));
        }
    }
    Ok(times)
}

/// After unmount, promote a validated recovery sibling before reading capture.
/// The caller must exclude concurrent mounts/capture for this receipt. Failed
/// publication leaves the sibling intact; ordinary reads remain read-only.
pub fn recover_plugin_mtimes(path: &Path) -> io::Result<BTreeMap<String, SystemTime>> {
    if !has_recovery(path)? {
        return read_plugin_mtimes(path);
    }
    let pending = recovery_path(path);
    let times = read_plugin_mtimes(&pending).map_err(|e| {
        // NotFound here is an invalid recovery (for example a dangling link),
        // not the ordinary absence of a receipt that capture can skip.
        io::Error::other(format!(
            "Could not read pending timestamp recovery at {}: {e}",
            pending.display()
        ))
    })?;
    let projection = PluginTimestamps {
        times,
        state_path: path.to_owned(),
    };
    projection.save(&projection.times)?;
    projection.clear_recovery()?;
    Ok(projection.times)
}

impl PluginTimestamps {
    fn save(&self, times: &BTreeMap<String, SystemTime>) -> io::Result<()> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = self
            .state_path
            .parent()
            .ok_or_else(|| io::Error::other("Timestamp receipt needs a parent"))?;
        fs::create_dir_all(parent)?;
        let temp = parent.join(format!(
            ".plugin-times-{}-{}.new",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            let header = b"Eidos plugin timestamps v1\n";
            file.write_all(header)?;
            let mut bytes = header.len();
            for (name, t) in times {
                if name.len() > MAX_RECEIPT_BYTES - bytes {
                    return Err(io::Error::other("Timestamp receipt exceeds limit"));
                }
                key(name)?;
                let line = format!("{}\t{name}\n", number(*t));
                if line.len() > MAX_RECEIPT_BYTES - bytes {
                    return Err(io::Error::other("Timestamp receipt exceeds limit"));
                }
                file.write_all(line.as_bytes())?;
                bytes += line.len();
            }
            file.sync_all()?;
            fs::rename(&temp, &self.state_path)?;
            fs::File::open(parent)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }

    fn clear_recovery(&self) -> io::Result<()> {
        match fs::remove_file(recovery_path(&self.state_path)) {
            Ok(()) => fs::File::open(
                self.state_path
                    .parent()
                    .ok_or_else(|| io::Error::other("Timestamp receipt needs a parent"))?,
            )?
            .sync_all(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl ProjectionState {
    fn persist(&mut self) -> io::Result<()> {
        if self.dirty {
            self.projection.save(&self.projection.times)?;
            self.projection.clear_recovery()?;
            self.dirty = false;
        }
        Ok(())
    }
}
impl Eidos {
    pub fn with_plugin_timestamps(mut self, mut projection: PluginTimestamps) -> io::Result<Self> {
        let mut folded = BTreeMap::new();
        for (name, time) in projection.times {
            if folded.insert(key(&name)?, time).is_some() {
                return Err(io::Error::other("Duplicate projected plugin filename"));
            }
        }
        projection.times = folded;
        if has_recovery(&projection.state_path)?
            && read_plugin_mtimes(&recovery_path(&projection.state_path))? != projection.times
        {
            return Err(io::Error::other(
                "Unresolved pending timestamp order differs from this projection",
            ));
        }
        projection.save(&projection.times)?;
        projection.clear_recovery()?;
        self.plugin_timestamps = Some(std::sync::Mutex::new(ProjectionState {
            projection,
            dirty: false,
        }));
        Ok(self)
    }
    pub(crate) fn projected_times(&self, vpath: &str) -> (Option<SystemTime>, Option<SystemTime>) {
        let mtime = self.plugin_timestamps.as_ref().and_then(|p| {
            p.lock_recover()
                .projection
                .times
                .get(&vpath.to_ascii_lowercase())
                .copied()
        });
        let atime = self
            .plugin_atimes
            .lock_recover()
            .get(&vpath.to_ascii_lowercase())
            .copied();
        (mtime, atime)
    }
    pub(crate) fn set_projected_times(
        &self,
        vpath: &str,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
    ) -> io::Result<bool> {
        let Some(projection) = &self.plugin_timestamps else {
            return Ok(false);
        };
        let Ok(name) = key(vpath) else {
            return Ok(false);
        };
        if !self
            .stack
            .resolve_read(vpath)
            .is_some_and(|p| fs::symlink_metadata(p).is_ok_and(|m| m.is_file()))
        {
            return Ok(false);
        }
        let value = |t| match t {
            TimeOrNow::SpecificTime(t) => t,
            TimeOrNow::Now => SystemTime::now(),
        };
        let mut projection = projection.lock_recover();
        if let Some(t) = mtime {
            // A metadata-only failure leaves the visible value unchanged. Keep
            // that value dirty too: a directory-fsync failure can follow rename.
            let previous = projection.projection.times.clone();
            projection.projection.times.insert(name.clone(), value(t));
            projection.dirty = true;
            if let Err(e) = projection.persist() {
                projection.projection.times = previous;
                return Err(e);
            }
        } else {
            projection.persist()?;
        }
        if let Some(t) = atime {
            self.plugin_atimes.lock_recover().insert(name, value(t));
        }
        Ok(true)
    }
    pub(crate) fn remove_projected_time(&self, vpath: &str) -> io::Result<()> {
        self.rename_projected_time(vpath, "")
    }
    pub(crate) fn rename_projected_time(&self, from: &str, to: &str) -> io::Result<()> {
        let Some(projection) = &self.plugin_timestamps else {
            return Ok(());
        };
        let mut projection = projection.lock_recover();
        let mut next = projection.projection.times.clone();
        let value = next.remove(&from.to_ascii_lowercase());
        next.remove(&to.to_ascii_lowercase());
        if let (Some(value), Ok(to)) = (value, key(to)) {
            next.insert(to, value);
        }
        if next != projection.projection.times {
            // The payload operation already happened. Preserve its intended
            // order in memory even if the receipt cannot currently be saved.
            projection.projection.times = next;
            projection.dirty = true;
        }
        let mut atimes = self.plugin_atimes.lock_recover();
        let atime = atimes.remove(&from.to_ascii_lowercase());
        atimes.remove(&to.to_ascii_lowercase());
        if let (Some(t), Ok(to)) = (atime, key(to)) {
            atimes.insert(to, t);
        }
        drop(atimes);
        projection.persist()
    }

    pub(crate) fn flush_projected_times(&self) -> io::Result<()> {
        if let Some(projection) = &self.plugin_timestamps {
            projection.lock_recover().persist()?;
        }
        Ok(())
    }

    pub(crate) fn finish_projected_times(&self) -> io::Result<()> {
        let Some(projection) = &self.plugin_timestamps else {
            return Ok(());
        };
        let mut state = projection.lock_recover();
        if let Err(primary) = state.persist() {
            let pending = PluginTimestamps {
                times: state.projection.times.clone(),
                state_path: recovery_path(&state.projection.state_path),
            };
            return match pending.save(&pending.times) {
                Ok(()) => Err(io::Error::other(format!(
                    "Timestamp receipt failed ({primary}); pending order saved at {}. Capture must recover it before using the primary receipt",
                    pending.state_path.display()
                ))),
                Err(recovery) => Err(io::Error::other(format!(
                    "Timestamp receipt failed ({primary}); recovery also failed ({recovery}). Pending order is not durable"
                ))),
            };
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_recovery_promotes_without_primary_and_rejects_broken_pending() {
        let root = std::env::temp_dir().join(format!(
            "eidos-time-explicit-recovery-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let primary = root.join("times");
        let pending = recovery_path(&primary);
        assert_eq!(
            recover_plugin_mtimes(&primary).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        let times = BTreeMap::from([("a.esp".into(), UNIX_EPOCH)]);
        PluginTimestamps {
            times: times.clone(),
            state_path: pending.clone(),
        }
        .save(&times)
        .unwrap();
        assert!(read_plugin_mtimes(&primary).is_err());
        assert_eq!(recover_plugin_mtimes(&primary).unwrap(), times);
        assert!(!pending.exists());
        assert_eq!(read_plugin_mtimes(&primary).unwrap(), times);
        std::os::unix::fs::symlink(root.join("missing"), &pending).unwrap();
        assert_ne!(
            recover_plugin_mtimes(&primary).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(fs::symlink_metadata(&pending).is_ok());
        fs::remove_file(&pending).unwrap();
        assert_eq!(read_plugin_mtimes(&primary).unwrap(), times);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_receipt_save_retains_the_completed_rename_projection() {
        let root =
            std::env::temp_dir().join(format!("eidos-time-failed-rename-{}", std::process::id()));
        fs::create_dir_all(root.join("lower")).unwrap();
        fs::write(root.join("lower/A.esp"), b"A").unwrap();
        let first = UNIX_EPOCH + Duration::from_secs(1000);
        let receipt = root.join("profile/times");
        let fsys = Eidos::new(vec![root.join("lower")], root.join("overwrite"))
            .with_plugin_timestamps(PluginTimestamps {
                times: BTreeMap::from([("a.esp".into(), first)]),
                state_path: receipt.clone(),
            })
            .unwrap();
        fs::rename(root.join("profile"), root.join("saved-profile")).unwrap();
        fs::write(root.join("profile"), b"receipt parent fault").unwrap();
        fsys.stack.rename("A.esp", "Renamed.esp").unwrap();
        assert!(fsys.rename_projected_time("A.esp", "Renamed.esp").is_err());
        assert_eq!(fsys.projected_times("Renamed.esp").0, Some(first));
        assert_eq!(fsys.projected_times("A.esp").0, None);
        assert_eq!(
            read_plugin_mtimes(&root.join("saved-profile/times"))
                .unwrap()
                .get("a.esp"),
            Some(&first)
        );
        assert!(fsys
            .finish_projected_times()
            .unwrap_err()
            .to_string()
            .contains("not durable"));
        fs::remove_file(root.join("profile")).unwrap();
        fs::rename(root.join("saved-profile"), root.join("profile")).unwrap();
        fsys.flush_projected_times().unwrap();
        assert_eq!(
            read_plugin_mtimes(&receipt).unwrap(),
            BTreeMap::from([("renamed.esp".into(), first)])
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn teardown_pending_receipt_blocks_stale_capture_and_recovers_on_restart() {
        let root = std::env::temp_dir().join(format!("eidos-time-recovery-{}", std::process::id()));
        fs::create_dir_all(root.join("lower")).unwrap();
        fs::write(root.join("lower/A.esp"), b"A").unwrap();
        let first = UNIX_EPOCH + Duration::from_secs(1000);
        let receipt = root.join("profile/times");
        let original = BTreeMap::from([("a.esp".into(), first)]);
        let fsys = Eidos::new(vec![root.join("lower")], root.join("overwrite"))
            .with_plugin_timestamps(PluginTimestamps {
                times: original.clone(),
                state_path: receipt.clone(),
            })
            .unwrap();
        // A directory at the receipt destination blocks primary publication,
        // while its sibling recovery path remains writable.
        fs::rename(&receipt, root.join("old-times")).unwrap();
        fs::create_dir(&receipt).unwrap();
        fsys.stack.rename("A.esp", "B.esp").unwrap();
        assert!(fsys.rename_projected_time("A.esp", "B.esp").is_err());
        assert!(fsys
            .finish_projected_times()
            .unwrap_err()
            .to_string()
            .contains("pending order saved"));
        drop(fsys);
        fs::remove_dir(&receipt).unwrap();
        fs::rename(root.join("old-times"), &receipt).unwrap();
        assert!(read_plugin_mtimes(&receipt)
            .unwrap_err()
            .to_string()
            .contains("refusing stale primary"));
        let recovered = read_plugin_mtimes(&recovery_path(&receipt)).unwrap();
        assert_eq!(recovered, BTreeMap::from([("b.esp".into(), first)]));
        assert!(Eidos::new(vec![root.join("lower")], root.join("overwrite"))
            .with_plugin_timestamps(PluginTimestamps {
                times: original,
                state_path: receipt.clone()
            })
            .is_err());
        let restarted = Eidos::new(vec![root.join("lower")], root.join("overwrite"))
            .with_plugin_timestamps(PluginTimestamps {
                times: recovered.clone(),
                state_path: receipt.clone(),
            })
            .unwrap();
        assert_eq!(restarted.projected_times("B.esp").0, Some(first));
        assert_eq!(read_plugin_mtimes(&receipt).unwrap(), recovered);
        assert!(!recovery_path(&receipt).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_metadata_only_projection_keeps_previous_order_and_retries_it() {
        let root =
            std::env::temp_dir().join(format!("eidos-time-metadata-fault-{}", std::process::id()));
        fs::create_dir_all(root.join("lower")).unwrap();
        fs::write(root.join("lower/A.esp"), b"A").unwrap();
        let first = UNIX_EPOCH + Duration::from_secs(1000);
        let receipt = root.join("profile/times");
        let original = BTreeMap::from([("a.esp".into(), first)]);
        let fsys = Eidos::new(vec![root.join("lower")], root.join("overwrite"))
            .with_plugin_timestamps(PluginTimestamps {
                times: original.clone(),
                state_path: receipt.clone(),
            })
            .unwrap();
        fs::rename(&receipt, root.join("old-times")).unwrap();
        fs::create_dir(&receipt).unwrap();
        assert!(fsys
            .set_projected_times("A.esp", Some(TimeOrNow::Now), Some(TimeOrNow::Now))
            .is_err());
        assert_eq!(fsys.projected_times("A.esp"), (Some(first), None));
        assert!(!root.join("overwrite/A.esp").exists());
        fs::remove_dir(&receipt).unwrap();
        fsys.flush_projected_times().unwrap();
        assert_eq!(read_plugin_mtimes(&receipt).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn receipt_round_trips_nanoseconds_and_rejects_invalid_state() {
        let root = std::env::temp_dir().join(format!("eidos-time-receipt-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("times");
        let times = BTreeMap::from([
            ("a.esp".into(), UNIX_EPOCH - Duration::new(2, 3)),
            ("b.esm".into(), UNIX_EPOCH + Duration::new(4, 5)),
        ]);
        PluginTimestamps {
            times: times.clone(),
            state_path: path.clone(),
        }
        .save(&times)
        .unwrap();
        assert_eq!(read_plugin_mtimes(&path).unwrap(), times);
        let oversized =
            BTreeMap::from([(format!("{}.esp", "a".repeat(MAX_RECEIPT_BYTES)), UNIX_EPOCH)]);
        assert!(PluginTimestamps {
            times: oversized.clone(),
            state_path: path.clone()
        }
        .save(&oversized)
        .is_err());
        assert_eq!(read_plugin_mtimes(&path).unwrap(), times);
        assert_eq!(
            fs::read_dir(&root).unwrap().count(),
            1,
            "failed saves must remove their temporary file"
        );
        for body in [
            "wrong header\n",
            "Eidos plugin timestamps v1\n1\ta.esp\n2\tA.esp\n",
            "Eidos plugin timestamps v1\n1\t../a.esp\n",
            "Eidos plugin timestamps v1\nnot-a-time\ta.esp\n",
        ] {
            fs::write(&path, body).unwrap();
            assert!(read_plugin_mtimes(&path).is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
