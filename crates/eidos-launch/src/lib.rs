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
    /// Extra environment variables for the command (e.g. `WINEDLLOVERRIDES` for
    /// forced libraries), applied on top of the inherited environment.
    pub env: Vec<(String, String)>,
    /// Optional `(src, stash)`: bind-mount `src` onto `stash` before mounting,
    /// then append `stash` as the lowest layer. This is how we mount a union
    /// *over the game's own Data dir*: the bind captures the real files at
    /// `stash` so the daemon can still read them once the union covers `src`.
    pub base_bind: Option<(PathBuf, PathBuf)>,
    /// Extra `(src, dst)` bind mounts set up in the namespace before launch, each
    /// making `dst` show `src` for the duration of the run only. Used to redirect
    /// the game's save directory to the active profile's saves (the Linux-native
    /// equivalent of MO2's usvfs save mapping) without ever modifying the prefix.
    pub binds: Vec<(PathBuf, PathBuf)>,
    /// Working directory for the command. `None` = the game root (the
    /// mountpoint's parent) - required for the game itself (CommonLibSSE-NG
    /// resolves its address library CWD-relative). Tools override it (MO2's
    /// default for a tool is the executable's own directory).
    pub cwd: Option<PathBuf>,
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

    // Prefer a privileged mount namespace: if this binary carries CAP_SYS_ADMIN
    // (e.g. `setcap cap_sys_admin+ep`), a bare CLONE_NEWNS succeeds and the FUSE
    // daemon can enable kernel passthrough, so reads/mmap hit the real backing
    // files and Windows DLLs (SKSE plugins) image-map natively. Without the
    // capability that unshare fails, so we fall back to the fully rootless
    // user+mount namespace (the `unshare --map-root-user --mount` idiom), where
    // passthrough is unavailable and executable DLLs may fail to load. Either way
    // the mount stays invisible to the rest of the system.
    // SAFETY: unshare with namespace flags has no memory-safety preconditions.
    let privileged = unsafe { libc::unshare(libc::CLONE_NEWNS) } == 0;
    if !privileged {
        check(unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) })?;
        std::fs::write("/proc/self/setgroups", "deny")?;
        std::fs::write("/proc/self/uid_map", format!("0 {uid} 1"))?;
        std::fs::write("/proc/self/gid_map", format!("0 {gid} 1"))?;
    }
    make_root_private()?;

    let mut layers = spec.layers;
    if let Some((src, stash)) = &spec.base_bind {
        std::fs::create_dir_all(stash)?;
        bind_mount(src, stash)?;
        layers.push(stash.clone()); // lowest priority: the pristine game files
    }

    // Extra redirects (e.g. the active profile's saves over the prefix save dir).
    // Both ends must exist for the bind; best-effort so a missing dir or a failed
    // bind never blocks the game from starting.
    for (src, dst) in &spec.binds {
        if std::fs::create_dir_all(src).is_ok() && std::fs::create_dir_all(dst).is_ok() {
            let _ = bind_mount(src, dst);
        }
    }

    std::fs::create_dir_all(&spec.mountpoint)?;
    let session = Eidos::new(layers, spec.overwrite).spawn(&spec.mountpoint)?;

    // Run from the game root (the directory that contains Data), exactly like MO2
    // (modorganizer processrunner sets the child's CWD to the game's base dir).
    // This is REQUIRED, not cosmetic: CommonLibSSE-NG opens its address library by
    // the RELATIVE path "Data/SKSE/Plugins/versionlib-<ver>.bin", resolved against
    // the CWD. If the CWD isn't the game root, every CommonLibSSE-NG SKSE plugin
    // fails with "Failed to locate an appropriate address library" and aborts.
    // `mountpoint` is the game's Data dir, so its parent is the game root.
    let mut cmd = Command::new(&spec.command[0]);
    cmd.args(&spec.command[1..]);
    cmd.envs(spec.env.iter().map(|(k, v)| (k, v)));
    match &spec.cwd {
        Some(dir) => {
            cmd.current_dir(dir);
        }
        None => {
            if let Some(game_root) = spec.mountpoint.parent() {
                cmd.current_dir(game_root);
            }
        }
    }
    let status = cmd.status();

    drop(session); // unmount
    status
}

/// Compose a `WINEDLLOVERRIDES` value forcing `stems` to load native-then-builtin
/// (`n,b`) - the Linux equivalent of usvfs's forced libraries, what lets a mod's
/// own ENB / ReShade / `.asi` loader DLL actually load under Wine. Any inherited
/// value (Proton sets its own) is preserved and ours appended.
pub fn wine_dll_overrides(stems: &[String], inherited: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(prev) = inherited.filter(|s| !s.is_empty()) {
        parts.push(prev.to_string());
    }
    if !stems.is_empty() {
        parts.push(format!("{}=n,b", stems.join(",")));
    }
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dll_overrides_compose_and_merge() {
        assert_eq!(wine_dll_overrides(&[], None), "");
        assert_eq!(
            wine_dll_overrides(&["d3d11".into(), "dxgi".into()], None),
            "d3d11,dxgi=n,b"
        );
        // An inherited Proton value is kept, ours appended.
        assert_eq!(
            wine_dll_overrides(&["dinput8".into()], Some("mscoree=d;mshtml=d")),
            "mscoree=d;mshtml=d;dinput8=n,b"
        );
        assert_eq!(wine_dll_overrides(&[], Some("a=b")), "a=b");
    }
}
