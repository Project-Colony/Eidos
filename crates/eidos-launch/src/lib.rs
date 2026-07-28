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
    /// ROOT-LEVEL mod layers, highest priority first: each enabled mod's `Root/`
    /// directory, projected onto the GAME INSTALL ROOT rather than into `Data/`.
    ///
    /// This is MO2's Root Builder, and it is what makes a script extender, ENB,
    /// ReShade, `.asi` loaders and Engine Fixes manageable as mods instead of
    /// files the user copies into their game by hand. Empty (the default) means
    /// no second mount happens at all and behaviour is exactly as before.
    ///
    /// Other managers deploy these by copying into the real game directory and
    /// restoring afterwards, with a journal so a crash can be cleaned up. Eidos
    /// does not have to: it already owns a private mount namespace, so a second
    /// union over the game root gives the same result with nothing written to
    /// disk and no residue possible - the namespace dies with the process.
    pub root_layers: Vec<PathBuf>,
    /// Writable layer for the ROOT union. `None` falls back to a directory beside
    /// the root stash, which is what the standalone `eidos-launch` binary wants
    /// (it has no instance to put one in); `eidos` passes the instance's single
    /// Overwrite so everything the user can write ends up in one place.
    pub root_overwrite: Option<PathBuf>,
    /// `(game_root, stash)` for the root union, mirroring [`Self::base_bind`]:
    /// the bind captures the pristine game root so the daemon can still read it
    /// once the union covers that same path.
    pub root_base_bind: Option<(PathBuf, PathBuf)>,
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

    // Prefer a plain mount namespace: if this binary carries CAP_SYS_ADMIN (e.g.
    // `setcap cap_sys_admin+ep`), a bare CLONE_NEWNS succeeds. Without the
    // capability that unshare fails, so we fall back to the fully rootless
    // user+mount namespace (the `unshare --map-root-user --mount` idiom). Either
    // way the mount stays invisible to the rest of the system and mods deploy
    // identically - the capability is OPTIONAL, and the rootless path is the one
    // most installs take.
    //
    // The one thing it still gates is FUSE passthrough, which is off by default
    // because it stops the game opening its own archives and plugins (see
    // `passthrough_enabled` in eidos-fuse). So only say something when the user
    // asked for passthrough and cannot have it - warning unconditionally sent
    // people chasing a capability that buys them nothing.
    // SAFETY: unshare with namespace flags has no memory-safety preconditions.
    let privileged = unsafe { libc::unshare(libc::CLONE_NEWNS) } == 0;
    if !privileged {
        if std::env::var("EIDOS_FUSE_PASSTHROUGH").is_ok_and(|v| !v.trim().is_empty() && v != "0") {
            let exe = std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<this eidos binary>".to_string());
            eprintln!(
                "eidos: EIDOS_FUSE_PASSTHROUGH is set but this binary has no CAP_SYS_ADMIN \
                 (setcap is wiped by every rebuild) - running rootless, passthrough stays OFF.\n\
                 eidos: grant it with:  sudo setcap cap_sys_admin+ep {exe}"
            );
        }
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

    // Extra redirects: the active profile's saves and plugin state over the
    // prefix's dirs. FAIL CLOSED: these binds are what makes the session's writes
    // land in the profile, and a run that continues without one silently forks
    // the playthrough - the game writes into the prefix, the bind hides those
    // files on every LATER (successful) run, and the user discovers a hole in
    // their saves weeks after the cause. A refused launch with a reason is
    // recoverable; a forked save history is not. (This warned-and-continued
    // once; the audit found the orphaned sessions it produced.)
    let mut mounted_binds: Vec<&PathBuf> = Vec::new();
    for (src, dst) in &spec.binds {
        let result = std::fs::create_dir_all(src)
            .and_then(|()| std::fs::create_dir_all(dst))
            .and_then(|()| bind_mount(src, dst));
        match result {
            Ok(()) => mounted_binds.push(dst),
            Err(e) => {
                // Refusing the launch must not strand the EARLIER binds: the
                // caller's post-run steps still run after this error, and a
                // leftover bind makes them read the profile through the mount
                // while believing they read the prefix.
                for d in mounted_binds {
                    let _ = unmount_detach(d);
                }
                return Err(std::io::Error::new(
                    e.kind(),
                    format!(
                        "refusing to launch: bind of {} over {} failed ({e}); running without \
                         it would silently split this session's files away from the profile",
                        src.display(),
                        dst.display()
                    ),
                ));
            }
        }
    }

    // ROOT UNION FIRST, if any mod ships a `Root/`. Order matters: this union
    // covers the game install root, and the Data union below then mounts INSIDE
    // it. Mounting Data first would leave it shadowed by the root mount.
    //
    // `_root_session` is bound before `session` so that reverse drop order at the
    // end of this function unmounts Data before the root beneath it.
    let _root_session = if spec.root_layers.is_empty() {
        None
    } else {
        let Some((root_src, root_stash)) = spec.root_base_bind.as_ref() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "root_layers given without root_base_bind",
            ));
        };
        std::fs::create_dir_all(root_stash)?;
        bind_mount(root_src, root_stash)?;
        let mut root_layers = spec.root_layers.clone();
        // Lowest priority: the pristine game files, read through the stash.
        root_layers.push(root_stash.clone());
        // Writes beside the game's own exe (crash logs, ReShade caches, a tool
        // aimed one directory too high) still never touch the install - but they
        // land in the caller's ONE Overwrite, under `Root/`, rather than in a
        // hidden directory of our own invention that no front end ever listed.
        let root_overwrite = match spec.root_overwrite.as_ref() {
            Some(p) => p.clone(),
            None => root_stash.with_extension("root-overwrite"),
        };
        std::fs::create_dir_all(&root_overwrite)?;
        eprintln!(
            "eidos: {} mod(s) provide root-level files; mounting a union over the game root",
            spec.root_layers.len()
        );
        Some(Eidos::new(root_layers, root_overwrite).spawn(root_src)?)
    };

    std::fs::create_dir_all(&spec.mountpoint)?;
    let session = Eidos::new(layers, spec.overwrite).spawn(&spec.mountpoint)?;

    // Run from the game root (the directory that contains Data), exactly like MO2
    // (modorganizer processrunner sets the child's CWD to the game's base dir).
    // This is REQUIRED, not cosmetic: CommonLibSSE-NG opens its address library by
    // the RELATIVE path "Data/SKSE/Plugins/versionlib-<ver>.bin", resolved against
    // the CWD. If the CWD isn't the game root, every CommonLibSSE-NG SKSE plugin
    // fails with "Failed to locate an appropriate address library" and aborts.
    // `mountpoint` is the game's Data dir, so its parent is the game root.
    // Become the subreaper for the launched process tree, so a tool or game that
    // spawns a sibling and exits (Wrye Bash launching the game, a loader-style tool,
    // FNIS spawning a worker) is still waited on before we tear the union down - the
    // Linux equivalent of MO2's job-object wait. Reparented descendants become our
    // children once their direct parent dies.
    // SAFETY: prctl with a constant option/arg has no memory-safety preconditions.
    unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1 as libc::c_ulong) };

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

    // Wait for any reparented descendants to finish before unmounting, so a process
    // that outlived its direct parent doesn't get ENOTCONN on the union mid-run (and
    // the caller's capture_inis runs only once the real workload is gone). The direct
    // child has already been reaped by `Command::status`.
    reap_descendants();

    // Unmount the extra binds NOW, not at process exit: this process stays inside
    // the namespace, and the caller's post-run steps need the REAL prefix back.
    // The Steam Cloud save sync read "the prefix Saves dir" through the still-
    // mounted bind - its own source - and no-op'd on every run while looking like
    // it worked. Detach-style, loudly on failure: a bind that stays up silently
    // turns that sync back into a lie.
    for (_src, dst) in &spec.binds {
        if let Err(e) = unmount_detach(dst) {
            eprintln!(
                "eidos: WARNING - could not unmount {} after the run ({e}); \
                 post-run steps may read the profile through it instead of the prefix",
                dst.display()
            );
        }
    }

    drop(session); // unmount
    status
}

/// Lazy-detach unmount of `path` (`MNT_DETACH`: the mount leaves the namespace
/// now; the kernel finishes when the last user lets go).
fn unmount_detach(path: &Path) -> std::io::Result<()> {
    let p = cstring(path)?;
    // SAFETY: umount2 with a valid NUL-terminated path and no memory preconditions.
    check(unsafe { libc::umount2(p.as_ptr(), libc::MNT_DETACH) })
}

/// Reap every remaining reparented descendant; returns once none are left
/// (`waitpid` -> `ECHILD`). Blocks like Steam's reaper so the union stays mounted
/// for the whole process tree.
fn reap_descendants() {
    // SAFETY: waitpid(-1, NULL, 0) is a valid blocking wait for any child; it
    // returns <= 0 (ECHILD) once none remain.
    while unsafe { libc::waitpid(-1, std::ptr::null_mut(), 0) } > 0 {}
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

/// Force a UTF-8 locale for the launched process when the inherited one is not
/// already UTF-8, as `[(key, value)]` to add to the child's environment.
///
/// Wine picks its Unix codepage from `nl_langinfo(CODESET)`. Under a C/POSIX
/// locale that collapses to CP1252, and MSVC's `std::filesystem` then throws
/// "Invalid name" on any mod file with a CJK, Cyrillic or accented character in
/// its path - a failure that looks like a broken mod rather than a broken locale.
/// Steam's pressure-vessel can strip the user's locale on the way in, so this
/// cannot be assumed to be inherited correctly.
///
/// An existing UTF-8 locale is left completely alone: `fr_FR.UTF-8` carries the
/// user's collation and date formats and there is no reason to override it.
/// Precedence follows POSIX: `LC_ALL` beats `LC_CTYPE` beats `LANG`.
pub fn utf8_locale_env(
    lc_all: Option<&str>,
    lc_ctype: Option<&str>,
    lang: Option<&str>,
) -> Vec<(String, String)> {
    let effective = [lc_all, lc_ctype, lang].into_iter().flatten().find(|v| !v.is_empty());
    let is_utf8 = effective.is_some_and(|v| {
        let v = v.to_ascii_lowercase();
        v.contains("utf-8") || v.contains("utf8")
    });
    if is_utf8 {
        return Vec::new();
    }
    vec![
        ("LC_ALL".to_string(), "C.UTF-8".to_string()),
        ("LANG".to_string(), "C.UTF-8".to_string()),
    ]
}

/// [`utf8_locale_env`] applied to this process's own environment.
pub fn utf8_locale_env_from_process() -> Vec<(String, String)> {
    let get = |k: &str| std::env::var(k).ok();
    utf8_locale_env(get("LC_ALL").as_deref(), get("LC_CTYPE").as_deref(), get("LANG").as_deref())
}

/// Whether `binary` carries CAP_SYS_ADMIN in its file capabilities (the
/// `setcap cap_sys_admin+ep` state FUSE passthrough needs). Reads the
/// `security.capability` xattr directly, so it works without the libcap tools.
/// Every rebuild of the binary wipes the xattr, which silently degrades launches
/// to rootless mode - front ends use this to warn instead of degrading silently.
pub fn binary_has_cap_sys_admin(binary: &Path) -> bool {
    let Ok(path) = std::ffi::CString::new(binary.as_os_str().as_encoded_bytes()) else {
        return false;
    };
    let name = c"security.capability";
    let mut buf = [0u8; 64];
    // SAFETY: valid NUL-terminated strings and a correctly-sized buffer.
    let n = unsafe {
        libc::getxattr(path.as_ptr(), name.as_ptr(), buf.as_mut_ptr().cast(), buf.len())
    };
    // VFS cap data (v2/v3): u32 magic+flags, then permitted-lo at bytes 4..8.
    // CAP_SYS_ADMIN is capability 21, in the low word.
    if n < 8 {
        return false;
    }
    let permitted_lo = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    permitted_lo & (1 << 21) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_or_xattr_reports_no_cap() {
        // A path that does not exist has no capability.
        assert!(!binary_has_cap_sys_admin(Path::new("/nonexistent/eidos")));
        // A plain file without the xattr has no capability.
        let p = std::env::temp_dir().join(format!("eidos-capcheck-{}", std::process::id()));
        std::fs::write(&p, b"x").unwrap();
        assert!(!binary_has_cap_sys_admin(&p));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn utf8_locale_is_forced_only_when_needed() {
        // Nothing inherited (pressure-vessel can strip it) -> force.
        assert_eq!(
            utf8_locale_env(None, None, None),
            vec![
                ("LC_ALL".to_string(), "C.UTF-8".to_string()),
                ("LANG".to_string(), "C.UTF-8".to_string())
            ]
        );
        // A C/POSIX locale is the actual bug: it collapses Wine to CP1252.
        assert!(!utf8_locale_env(None, None, Some("C")).is_empty());
        assert!(!utf8_locale_env(None, None, Some("POSIX")).is_empty());
        // The user's own UTF-8 locale carries their collation and formats: leave it.
        assert!(utf8_locale_env(None, None, Some("en_US.UTF-8")).is_empty());
        assert!(utf8_locale_env(None, None, Some("fr_FR.utf8")).is_empty());
        assert!(utf8_locale_env(None, Some("ja_JP.UTF-8"), None).is_empty());
        // POSIX precedence: LC_ALL wins, so a C there beats a UTF-8 LC_CTYPE.
        assert!(!utf8_locale_env(Some("C"), Some("en_US.UTF-8"), Some("en_US.UTF-8")).is_empty());
        // ...and a UTF-8 LC_ALL wins over a C LANG.
        assert!(utf8_locale_env(Some("en_US.UTF-8"), None, Some("C")).is_empty());
        // An empty variable is not set: fall through to the next one.
        assert!(utf8_locale_env(Some(""), None, Some("en_US.UTF-8")).is_empty());
    }

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
        // The provisioned d3dcompiler_47 folds into the same n,b list (Community
        // Shaders / ENB), preserving any inherited Proton value.
        assert_eq!(
            wine_dll_overrides(&["d3dcompiler_47".into(), "dxgi".into()], Some("mscoree=d")),
            "mscoree=d;d3dcompiler_47,dxgi=n,b"
        );
    }
}
