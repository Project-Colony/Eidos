//! Namespace + mount orchestration for running a command through the union view.
//!
//! [`launch`] enters a fresh, unprivileged user + mount namespace, mounts the
//! Eidos union, and runs a command inside it so the command (a game, a mod tool)
//! sees the merged mods while the rest of the system sees nothing. On exit the
//! namespace and its mount vanish, having touched neither the game install nor
//! any mod source.
//!
//! This is the engine behind the eventual Steam launch option `eidos %command%`.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use eidos_fuse::Eidos;

/// What to mount and run.
pub struct LaunchSpec {
    /// Mod layers, highest priority first.
    pub layers: Vec<PathBuf>,
    /// Writable Overwrite layer.
    pub overwrite: PathBuf,
    /// Where to mount the union. For a real game this is the game's Data dir.
    pub mountpoint: PathBuf,
    /// Command + args to run inside the namespace.
    pub command: Vec<String>,
    /// Optional `(src, stash)`: bind-mount `src` onto `stash` before mounting,
    /// then append `stash` as the lowest layer. This is how we mount a union
    /// *over the game's own Data dir*: the bind captures the real files at
    /// `stash` so the daemon can still read them once the union covers `src`.
    pub base_bind: Option<(PathBuf, PathBuf)>,
}

fn check(rc: i32) -> std::io::Result<()> {
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn cstring(p: &Path) -> std::io::Result<CString> {
    CString::new(p.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL"))
}

/// Recursively bind-mount `src` onto `dst`.
fn bind_mount(src: &Path, dst: &Path) -> std::io::Result<()> {
    let (s, d) = (cstring(src)?, cstring(dst)?);
    // SAFETY: standard mount(2) bind call with valid C strings.
    check(unsafe {
        libc::mount(
            s.as_ptr(),
            d.as_ptr(),
            std::ptr::null(),
            libc::MS_BIND | libc::MS_REC,
            std::ptr::null(),
        )
    })
}

/// Stop mounts in this namespace from propagating back to the host.
fn make_root_private() -> std::io::Result<()> {
    // SAFETY: standard mount(2) call with static C-string arguments.
    check(unsafe {
        libc::mount(
            c"none".as_ptr(),
            c"/".as_ptr(),
            std::ptr::null(),
            libc::MS_REC | libc::MS_PRIVATE,
            std::ptr::null(),
        )
    })
}

/// Enter a private user + mount namespace, mount the union, run the command
/// through it, then unmount on exit. Returns the command's exit status.
pub fn launch(spec: LaunchSpec) -> std::io::Result<ExitStatus> {
    // SAFETY: getuid/getgid always succeed.
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };

    // Map ourselves to root inside a fresh user+mount namespace (the
    // `unshare --map-root-user --mount` idiom), so the FUSE mount is permitted
    // and stays invisible to the rest of the system.
    // SAFETY: unshare with namespace flags has no memory-safety preconditions.
    check(unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) })?;
    std::fs::write("/proc/self/setgroups", "deny")?;
    std::fs::write("/proc/self/uid_map", format!("0 {uid} 1"))?;
    std::fs::write("/proc/self/gid_map", format!("0 {gid} 1"))?;
    make_root_private()?;

    let mut layers = spec.layers;
    if let Some((src, stash)) = &spec.base_bind {
        std::fs::create_dir_all(stash)?;
        bind_mount(src, stash)?;
        layers.push(stash.clone()); // lowest priority: the pristine game files
    }

    std::fs::create_dir_all(&spec.mountpoint)?;
    let session = Eidos::new(layers, spec.overwrite).spawn(&spec.mountpoint)?;

    let status = Command::new(&spec.command[0]).args(&spec.command[1..]).status();

    drop(session); // unmount
    status
}
