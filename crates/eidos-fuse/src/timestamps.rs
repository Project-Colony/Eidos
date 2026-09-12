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
/// Read the durable capture. Invalid or missing receipts are errors, not empty orders.
pub fn read_plugin_mtimes(path: &Path) -> io::Result<BTreeMap<String, SystemTime>> {
    let mut body = String::new();
    fs::File::open(path)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut body)?;
    if body.len() > 4 * 1024 * 1024 {
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
            file.write_all(b"Eidos plugin timestamps v1\n")?;
            for (name, t) in times {
                key(name)?;
                writeln!(file, "{}\t{name}", number(*t))?;
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
        projection.save(&projection.times)?;
        self.plugin_timestamps = Some(std::sync::Mutex::new(projection));
        Ok(self)
    }
    pub(crate) fn projected_times(&self, vpath: &str) -> (Option<SystemTime>, Option<SystemTime>) {
        let mtime = self.plugin_timestamps.as_ref().and_then(|p| {
            p.lock_recover()
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
            let mut next = projection.times.clone();
            next.insert(name.clone(), value(t));
            projection.save(&next)?;
            projection.times = next;
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
        let mut next = projection.times.clone();
        let value = next.remove(&from.to_ascii_lowercase());
        next.remove(&to.to_ascii_lowercase());
        if let (Some(value), Ok(to)) = (value, key(to)) {
            next.insert(to, value);
        }
        if next != projection.times {
            projection.save(&next)?;
            projection.times = next;
        }
        let mut atimes = self.plugin_atimes.lock_recover();
        let atime = atimes.remove(&from.to_ascii_lowercase());
        atimes.remove(&to.to_ascii_lowercase());
        if let (Some(t), Ok(to)) = (atime, key(to)) {
            atimes.insert(to, t);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
