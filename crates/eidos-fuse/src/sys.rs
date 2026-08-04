//! Thin syscall wrappers - positional I/O, xattrs, utimensat, statvfs - kept
//! apart so the dispatcher reads as logic rather than as libc plumbing.

use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io::ErrorKind;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::time::UNIX_EPOCH;

use fuser::{
    Errno, TimeOrNow,
};


/// Read up to `size` bytes at `offset` via `pread`, looping over short reads and
/// stopping at EOF. `pread` does not disturb a shared file offset, so concurrent
/// reads on one handle are safe.
pub(crate) fn read_full_at(file: &File, mut offset: u64, size: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; size];
    let mut filled = 0;
    while filled < size {
        match file.read_at(&mut buf[filled..], offset) {
            Ok(0) => break, // EOF
            Ok(n) => {
                filled += n;
                offset += n as u64;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    buf.truncate(filled);
    Ok(buf)
}

/// Write all of `data` at `offset` via `pwrite`, looping over short writes.
pub(crate) fn write_all_at(file: &File, mut data: &[u8], mut offset: u64) -> std::io::Result<()> {
    while !data.is_empty() {
        match file.write_at(data, offset) {
            Ok(0) => return Err(std::io::Error::new(ErrorKind::WriteZero, "write returned 0")),
            Ok(n) => {
                data = &data[n..];
                offset += n as u64;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Map a `TimeOrNow` (or its absence) onto a `timespec` for `utimensat`.
pub(crate) fn to_timespec(t: Option<TimeOrNow>) -> libc::timespec {
    match t {
        None => libc::timespec { tv_sec: 0, tv_nsec: libc::UTIME_OMIT },
        Some(TimeOrNow::Now) => libc::timespec { tv_sec: 0, tv_nsec: libc::UTIME_NOW },
        Some(TimeOrNow::SpecificTime(st)) => {
            let d = st.duration_since(UNIX_EPOCH).unwrap_or_default();
            libc::timespec {
                tv_sec: d.as_secs() as libc::time_t,
                tv_nsec: d.subsec_nanos() as _,
            }
        }
    }
}

/// Set a file's access/modification times (each optional) via `utimensat`.
pub(crate) fn set_times(path: &Path, atime: Option<TimeOrNow>, mtime: Option<TimeOrNow>) -> std::io::Result<()> {
    let times = [to_timespec(atime), to_timespec(mtime)];
    let c = cpath(path)?;
    // SAFETY: valid C path and a 2-element timespec array, per utimensat(2).
    let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), times.as_ptr(), 0) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// A path as a C string, rejecting embedded NULs.
pub(crate) fn cpath(path: &Path) -> std::io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "path contains NUL"))
}

/// Map an `io::Error` to a FUSE errno, defaulting to EIO when there is no raw
/// OS code (so xattr ENODATA/ENOTSUP/ERANGE round-trip to the game unchanged).
pub(crate) fn to_errno(e: &std::io::Error) -> Errno {
    Errno::from_i32(e.raw_os_error().unwrap_or(libc::EIO))
}

/// Read extended attribute `name` from `path`.
pub(crate) fn xattr_get(path: &Path, name: &OsStr) -> std::io::Result<Vec<u8>> {
    let p = cpath(path)?;
    let n = CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "xattr name contains NUL"))?;
    // SAFETY: valid C strings; first call sizes the buffer, second fills it.
    let len = unsafe { libc::getxattr(p.as_ptr(), n.as_ptr(), std::ptr::null_mut(), 0) };
    if len < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut buf = vec![0u8; len as usize];
    let got = unsafe {
        libc::getxattr(p.as_ptr(), n.as_ptr(), buf.as_mut_ptr() as *mut libc::c_void, buf.len())
    };
    if got < 0 {
        return Err(std::io::Error::last_os_error());
    }
    buf.truncate(got as usize);
    Ok(buf)
}

/// Set extended attribute `name` on `path`.
pub(crate) fn xattr_set(path: &Path, name: &OsStr, value: &[u8], flags: i32) -> std::io::Result<()> {
    let p = cpath(path)?;
    let n = CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "xattr name contains NUL"))?;
    // SAFETY: valid C strings and a sized value buffer, per setxattr(2).
    let rc = unsafe {
        libc::setxattr(
            p.as_ptr(),
            n.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            flags,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// List extended attribute names of `path` (NUL-separated, as the kernel returns).
pub(crate) fn xattr_list(path: &Path) -> std::io::Result<Vec<u8>> {
    let p = cpath(path)?;
    // SAFETY: valid C path; first call sizes the buffer, second fills it.
    let len = unsafe { libc::listxattr(p.as_ptr(), std::ptr::null_mut(), 0) };
    if len < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut buf = vec![0u8; len as usize];
    let got =
        unsafe { libc::listxattr(p.as_ptr(), buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if got < 0 {
        return Err(std::io::Error::last_os_error());
    }
    buf.truncate(got as usize);
    Ok(buf)
}

/// Remove extended attribute `name` from `path`.
pub(crate) fn xattr_remove(path: &Path, name: &OsStr) -> std::io::Result<()> {
    let p = cpath(path)?;
    let n = CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "xattr name contains NUL"))?;
    // SAFETY: valid C strings, per removexattr(2).
    let rc = unsafe { libc::removexattr(p.as_ptr(), n.as_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// `statvfs(2)` of a path, for reporting real free space to the game.
pub(crate) fn statvfs_of(path: &Path) -> std::io::Result<libc::statvfs> {
    let c = cpath(path)?;
    // SAFETY: valid C path and a zeroed statvfs out-param; we check the return.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(st)
}
