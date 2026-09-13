//! Checked mod snapshots shared by manual backups and reinstall publication.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Exclusively reserve `<name>_backup`, then `_backup2`, etc. The caller must
/// hold the instance mutation lock and either fill or remove the empty directory.
/// Case variants and dangling symlinks occupy their names too.
pub fn reserve_mod_backup(source: &Path) -> io::Result<PathBuf> {
    if !fs::symlink_metadata(source)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "mod backup source must be a real directory",
        ));
    }
    let parent = source
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "mod backup source needs a parent directory",
            )
        })?;
    let name = source.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "mod backup source needs a directory name",
        )
    })?;
    let occupied: HashSet<_> = fs::read_dir(parent)?
        .map(|entry| entry.map(|e| e.file_name().as_encoded_bytes().to_ascii_lowercase()))
        .collect::<io::Result<_>>()?;
    for number in 1u64.. {
        let mut candidate = name.to_os_string();
        candidate.push(if number == 1 {
            "_backup".to_string()
        } else {
            format!("_backup{number}")
        });
        if occupied.contains(&candidate.as_encoded_bytes().to_ascii_lowercase()) {
            continue;
        }
        let dest = parent.join(&candidate);
        match fs::create_dir(&dest) {
            Ok(()) => return Ok(dest),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free mod backup name",
    ))
}

/// Copy every old entry, including hidden metadata. Links remain links and are
/// never followed; unsupported special files fail instead of producing a partial
/// snapshot. Call while holding the instance mutation lock.
pub fn backup_mod(source: &Path) -> io::Result<PathBuf> {
    fn copy(source: &Path, dest: &Path, depth: usize) -> io::Result<()> {
        if depth > 256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "mod backup exceeds 256 directory levels",
            ));
        }
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let from = entry.path();
            let to = dest.join(entry.file_name());
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                std::os::unix::fs::symlink(fs::read_link(&from)?, &to)?;
            } else if kind.is_dir() {
                fs::create_dir(&to)?;
                copy(&from, &to, depth + 1)?;
            } else if kind.is_file() {
                fs::copy(&from, &to)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "mod backup requires a regular file, directory or symlink: {}",
                        from.display()
                    ),
                ));
            }
        }
        fs::set_permissions(dest, fs::metadata(source)?.permissions())
    }
    let dest = reserve_mod_backup(source)?;
    if let Err(error) = copy(source, &dest, 0) {
        if let Err(cleanup) = fs::remove_dir_all(&dest) {
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "{error}; cannot remove incomplete backup at {}: {cleanup}",
                    dest.display()
                ),
            ));
        }
        return Err(error);
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "eidos-mod-backup-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn backup_keeps_hidden_metadata_and_links_without_following_them() {
        let temp = Temp::new();
        let source = temp.0.join("Mod");
        fs::create_dir_all(source.join(".hidden")).unwrap();
        fs::write(source.join(".hidden/state"), b"hidden").unwrap();
        fs::write(source.join("meta.ini"), b"[General]\nnotes=keep\n").unwrap();
        std::os::unix::fs::symlink("/missing/external", source.join("external")).unwrap();
        std::os::unix::fs::symlink(".", source.join("cycle")).unwrap();
        let backup = backup_mod(&source).unwrap();
        assert_eq!(backup, temp.0.join("Mod_backup"));
        assert_eq!(fs::read(backup.join(".hidden/state")).unwrap(), b"hidden");
        assert_eq!(
            fs::read(backup.join("meta.ini")).unwrap(),
            fs::read(source.join("meta.ini")).unwrap()
        );
        assert_eq!(
            fs::read_link(backup.join("external")).unwrap(),
            PathBuf::from("/missing/external")
        );
        assert_eq!(
            fs::read_link(backup.join("cycle")).unwrap(),
            PathBuf::from(".")
        );
        let entry = crate::ModEntry {
            name: "Mod_backup".into(),
            enabled: true,
            path: backup,
            unmanaged: false,
        };
        assert!(!entry.is_active());
    }

    #[test]
    fn reservations_skip_case_collisions_and_dangling_links() {
        let temp = Temp::new();
        let source = temp.0.join("Mod");
        fs::create_dir(&source).unwrap();
        fs::create_dir(temp.0.join("mod_BACKUP")).unwrap();
        std::os::unix::fs::symlink("missing", temp.0.join("MOD_backup2")).unwrap();
        assert_eq!(
            reserve_mod_backup(&source).unwrap(),
            temp.0.join("Mod_backup3")
        );
        assert_eq!(
            reserve_mod_backup(&source).unwrap(),
            temp.0.join("Mod_backup4")
        );
        assert_eq!(
            fs::read_link(temp.0.join("MOD_backup2")).unwrap(),
            PathBuf::from("missing")
        );
    }

    #[test]
    fn failed_copy_removes_partial_backup_and_leaves_source_intact() {
        let temp = Temp::new();
        let source = temp.0.join("Mod");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("keep"), b"precious").unwrap();
        let _socket =
            std::os::unix::net::UnixListener::bind(source.join("unsupported.socket")).unwrap();
        let error = backup_mod(&source).unwrap_err();
        assert!(error.to_string().contains("regular file"), "{error}");
        assert_eq!(fs::read(source.join("keep")).unwrap(), b"precious");
        assert!(!temp.0.join("Mod_backup").exists());
    }

    #[test]
    fn backup_refuses_a_symlink_as_the_source_directory() {
        let temp = Temp::new();
        fs::create_dir(temp.0.join("real")).unwrap();
        std::os::unix::fs::symlink("real", temp.0.join("linked")).unwrap();
        assert!(backup_mod(&temp.0.join("linked")).is_err());
        assert!(!temp.0.join("linked_backup").exists());
    }
}
