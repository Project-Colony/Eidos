//! Single-member export to a user-chosen filename, anchored to an open directory.
use super::*;
use std::fs::{self, Metadata, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, PathBuf};

/// A short-lived source fingerprint for rejecting changed archive scan results.
/// This is filesystem identity/change metadata, not a content digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveIdentity {
    device: u64,
    inode: u64,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl ArchiveIdentity {
    fn metadata(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            length: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
    pub(super) fn for_file(file: &File) -> Result<Self> {
        let m = file.metadata()?;
        if !m.is_file() {
            return Err(corrupt("archive source is not a regular file"));
        }
        Ok(Self::metadata(&m))
    }
    pub(super) fn verify_file(&self, file: &File) -> Result<()> {
        if *self != Self::for_file(file)? {
            return Err(corrupt("archive changed while reading"));
        }
        Ok(())
    }
    fn same_file(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode
    }
}
fn open_source(path: &Path, expected: Option<&ArchiveIdentity>) -> Result<(File, ArchiveIdentity)> {
    // Nonblocking open prevents a substituted FIFO/device from hanging a worker;
    // regular files are required immediately after opening the descriptor.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
        .open(path)?;
    let identity = ArchiveIdentity::for_file(&file)?;
    if expected.is_some_and(|e| e != &identity) {
        return Err(corrupt("archive changed since selection"));
    }
    Ok((file, identity))
}
pub fn archive_identity(path: &Path) -> Result<ArchiveIdentity> {
    Ok(open_source(path, None)?.1)
}

pub fn read_archive_member(source: &Path, member: &str, max_bytes: u64) -> Result<Vec<u8>> {
    read_archive_member_checked(source, member, max_bytes, None)
}
/// Reads the entire member under the explicit output bound. An expected identity
/// may be captured with the conflict scan; a mismatch requires a fresh selection.
pub fn read_archive_member_checked(
    source: &Path,
    member: &str,
    max_bytes: u64,
    expected: Option<&ArchiveIdentity>,
) -> Result<Vec<u8>> {
    let (mut file, identity) = open_source(source, expected)?;
    let mut bytes = Vec::new();
    write_archive_member(&mut file, member, max_bytes, &mut bytes)?;
    if archive_identity(source)? != identity {
        return Err(corrupt("archive path changed while reading"));
    }
    Ok(bytes)
}

pub fn export_archive_member(
    source: &Path,
    member: &str,
    destination: &Path,
    max_bytes: u64,
) -> Result<ArchiveMemberInfo> {
    export_archive_member_checked(source, member, destination, max_bytes, None)
}
/// Stages beside the chosen destination, verifies/flushes all decoded bytes, then
/// atomically publishes. Existing regular files are replaced only after success.
/// Rejects source aliases, nonregular targets, symlink parents and `..` paths.
/// A sink/decoder/identity failure leaves any original destination untouched.
pub fn export_archive_member_checked(
    source: &Path,
    member: &str,
    destination: &Path,
    max_bytes: u64,
    expected: Option<&ArchiveIdentity>,
) -> Result<ArchiveMemberInfo> {
    let (mut file, identity) = open_source(source, expected)?;
    let (parent, name) = destination_parent(destination)?;
    let parent_identity = ArchiveIdentity::metadata(&parent.metadata()?);
    // /proc/self/fd anchors tempfile's path operations to the descriptor even if
    // a directory above it is renamed. Publication itself uses renameat.
    let anchored = PathBuf::from(format!("/proc/self/fd/{}", parent.as_raw_fd()));
    let target = anchored.join(&name);
    let original = destination_identity(&target)?;
    if original.as_ref().is_some_and(|o| o.same_file(&identity)) {
        return Err(corrupt("export destination aliases the source archive"));
    }
    let mut stage = tempfile::Builder::new()
        .prefix(".eidos-export-")
        .tempfile_in(&anchored)?;
    let info = write_archive_member(&mut file, member, max_bytes, &mut stage)?;
    stage.flush()?;
    stage.as_file().sync_all()?;
    identity.verify_file(&file)?;
    if archive_identity(source)? != identity {
        return Err(corrupt("archive path changed while exporting"));
    }
    let (current_parent, _) = destination_parent(destination)?;
    if !parent_identity.same_file(&ArchiveIdentity::metadata(&current_parent.metadata()?)) {
        return Err(corrupt("export destination parent changed while reading"));
    }
    publish_stage(&stage, &parent, &name, original.as_ref())?;
    Ok(info)
}
fn publish_stage(
    stage: &tempfile::NamedTempFile,
    parent: &File,
    name: &std::ffi::OsStr,
    original: Option<&ArchiveIdentity>,
) -> Result<()> {
    if destination_identity(stage.path())? != Some(ArchiveIdentity::for_file(stage.as_file())?) {
        return Err(corrupt("export stage changed while reading"));
    }
    let target = stage
        .path()
        .parent()
        .ok_or_else(|| corrupt("missing stage parent"))?
        .join(name);
    if destination_identity(&target)?.as_ref() != original {
        return Err(corrupt("export destination changed while reading"));
    }
    let temporary = stage
        .path()
        .file_name()
        .ok_or_else(|| corrupt("missing stage filename"))?;
    let flags = if original.is_none() {
        rustix::fs::RenameFlags::NOREPLACE
    } else {
        rustix::fs::RenameFlags::empty()
    };
    rustix::fs::renameat_with(parent, temporary, parent, name, flags).map_err(io::Error::from)?;
    Ok(())
}
fn destination_identity(path: &Path) -> Result<Option<ArchiveIdentity>> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_file() => Ok(Some(ArchiveIdentity::metadata(&m))),
        Ok(_) => Err(corrupt(
            "export destination is a symlink or nonregular file",
        )),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn destination_parent(destination: &Path) -> Result<(File, std::ffi::OsString)> {
    let raw = destination.as_os_str().as_bytes();
    if matches!(
        raw.rsplit(|&b| b == b'/').next(),
        None | Some(b"" | b"." | b"..")
    ) {
        return Err(corrupt("export destination must name a file"));
    }
    let name = destination
        .file_name()
        .ok_or_else(|| corrupt("missing export filename"))?
        .to_owned();
    let parent = destination
        .parent()
        .ok_or_else(|| corrupt("missing export parent"))?;
    let mut dir = File::open(if destination.is_absolute() { "/" } else { "." })?;
    for component in parent.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(name) => {
                dir = rustix::fs::openat(
                    &dir,
                    name,
                    rustix::fs::OFlags::RDONLY
                        | rustix::fs::OFlags::DIRECTORY
                        | rustix::fs::OFlags::NOFOLLOW
                        | rustix::fs::OFlags::CLOEXEC,
                    rustix::fs::Mode::empty(),
                )
                .map(File::from)
                .map_err(io::Error::from)?;
            }
            _ => return Err(corrupt("unsafe export destination path")),
        }
    }
    Ok((dir, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaced_export_stage_is_never_published() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("chosen.txt");
        fs::write(&destination, b"original").unwrap();
        let (parent, name) = destination_parent(&destination).unwrap();
        let original = destination_identity(&destination).unwrap();
        let anchored = PathBuf::from(format!("/proc/self/fd/{}", parent.as_raw_fd()));
        let mut stage = tempfile::NamedTempFile::new_in(&anchored).unwrap();
        stage.write_all(b"validated").unwrap();
        fs::remove_file(stage.path()).unwrap();
        fs::write(stage.path(), b"replacement").unwrap();
        assert!(publish_stage(&stage, &parent, &name, original.as_ref()).is_err());
        assert_eq!(fs::read(destination).unwrap(), b"original");
    }

    #[test]
    fn changed_export_destination_is_preserved_before_publication() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        for replacement in ["new", "changed", "symlink"] {
            let destination = dir.path().join(replacement);
            if replacement != "new" {
                fs::write(&destination, b"original").unwrap();
            }
            let original = destination_identity(&destination).unwrap();
            let (parent, name) = destination_parent(&destination).unwrap();
            let anchored = PathBuf::from(format!("/proc/self/fd/{}", parent.as_raw_fd()));
            let mut stage = tempfile::NamedTempFile::new_in(&anchored).unwrap();
            stage.write_all(b"validated").unwrap();
            if replacement == "symlink" {
                fs::remove_file(&destination).unwrap();
                symlink("absent", &destination).unwrap();
            } else {
                fs::write(&destination, b"concurrent change").unwrap();
            }
            assert!(publish_stage(&stage, &parent, &name, original.as_ref()).is_err());
            if replacement == "symlink" {
                assert_eq!(fs::read_link(destination).unwrap(), Path::new("absent"));
            } else {
                assert_eq!(fs::read(destination).unwrap(), b"concurrent change");
            }
        }
    }
}
