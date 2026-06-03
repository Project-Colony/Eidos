//! `eidos-launch`: run a command through the Eidos union view, isolated.
//!
//!   eidos-launch --layer <dir> [--layer <dir> ...] [--overwrite <dir>] \
//!                --mount <mountpoint> -- <command> [args...]
//!
//! It enters a fresh, unprivileged user + mount namespace, mounts the union at
//! `<mountpoint>`, and runs `<command>` inside that namespace so the command (a
//! game, a mod tool) sees the merged mods while the rest of the system sees only
//! the pristine directory. When the command exits, the namespace and its mount
//! vanish, having touched neither the game install nor any mod source.
//!
//! This is the engine behind the eventual Steam launch option `eidos %command%`:
//! Steam hands us the Proton command line and we run it through the view.

use std::fs;
use std::path::PathBuf;
use std::process::{exit, Command};

use eidos_fuse::Eidos;

fn usage() -> ! {
    eprintln!(
        "usage: eidos-launch --layer <dir> [--layer <dir> ...] [--overwrite <dir>] \\\n\
         \x20            --mount <mountpoint> -- <command> [args...]\n\
         \n\
         The first --layer has the highest priority; the last is typically the\n\
         pristine game data directory."
    );
    exit(2);
}

fn unshare(flags: i32) -> std::io::Result<()> {
    // SAFETY: unshare with namespace flags has no memory-safety preconditions.
    if unsafe { libc::unshare(flags) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Stop mounts in this namespace from propagating back to the host.
fn make_root_private() -> std::io::Result<()> {
    // SAFETY: standard mount(2) call with static C-string arguments.
    let rc = unsafe {
        libc::mount(
            c"none".as_ptr(),
            c"/".as_ptr(),
            std::ptr::null(),
            libc::MS_REC | libc::MS_PRIVATE,
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut layers: Vec<PathBuf> = Vec::new();
    let mut overwrite: Option<PathBuf> = None;
    let mut mountpoint: Option<PathBuf> = None;
    let mut command: Vec<String> = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--layer" => layers.push(PathBuf::from(args.next().unwrap_or_else(|| usage()))),
            "--overwrite" => overwrite = Some(PathBuf::from(args.next().unwrap_or_else(|| usage()))),
            "--mount" => mountpoint = Some(PathBuf::from(args.next().unwrap_or_else(|| usage()))),
            "--" => {
                command = args.by_ref().collect();
                break;
            }
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }

    let (Some(mountpoint), false, false) = (mountpoint, layers.is_empty(), command.is_empty()) else {
        usage();
    };

    let overwrite = overwrite.unwrap_or_else(|| {
        let p = std::env::temp_dir().join(format!("eidos-overwrite-{}", std::process::id()));
        let _ = fs::create_dir_all(&p);
        p
    });
    let _ = fs::create_dir_all(&mountpoint);

    // SAFETY: getuid/getgid always succeed.
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };

    // Enter a private user + mount namespace, then map ourselves to root inside
    // it (the `unshare --map-root-user --mount` idiom) so the FUSE mount is
    // permitted and stays invisible to the rest of the system.
    unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS)?;
    fs::write("/proc/self/setgroups", "deny")?;
    fs::write("/proc/self/uid_map", format!("0 {uid} 1"))?;
    fs::write("/proc/self/gid_map", format!("0 {gid} 1"))?;
    make_root_private()?;

    // Mount the union inside this namespace.
    let session = Eidos::new(layers, overwrite).spawn(&mountpoint)?;
    eprintln!(
        "eidos-launch: union mounted at {} (private to this launch)",
        mountpoint.display()
    );

    // Run the command inside the namespace; it sees the merged view.
    let status = Command::new(&command[0]).args(&command[1..]).status();

    drop(session); // unmount

    match status {
        Ok(s) => exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("eidos-launch: failed to run {:?}: {e}", command[0]);
            exit(127)
        }
    }
}
