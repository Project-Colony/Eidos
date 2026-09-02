//! Every knob the daemon reads from the environment, and the options handed
//! to the mount: cache TTLs, passthrough/trace/stats switches, open flags,
//! thread count, fd limit.

use std::time::Duration;

use fuser::{Config, FopenFlags, MountOption};

use crate::*;

/// Attribute/entry cache lifetime handed to the kernel for entries that EXIST.
/// A mod's files are immutable for the lifetime of a mount, and every mutation
/// goes through this daemon's own handlers, so the kernel can hold on to them.
/// Set `EIDOS_FUSE_NO_CACHE=1` to zero this (and [`NEG_TTL`]) when a stale-data
/// bug is the suspect.
pub(crate) const TTL_SECS: u64 = 3600;

/// Lifetime of a NEGATIVE dentry - see [`Eidos::reply_negative`]. Much shorter
/// than the positive TTL because the kernel matches negative entries on exact
/// name bytes while Eidos resolves case-insensitively.
pub(crate) const NEG_TTL_SECS: u64 = 60;

pub(crate) static TTL: std::sync::LazyLock<Duration> = std::sync::LazyLock::new(|| {
    Duration::from_secs(if Cache::Attr.is_off() { 0 } else { TTL_SECS })
});

pub(crate) static NEG_TTL: std::sync::LazyLock<Duration> = std::sync::LazyLock::new(|| {
    Duration::from_secs(if Cache::Neg.is_off() { 0 } else { NEG_TTL_SECS })
});

/// Flags for a FILE open reply.
///
/// `FOPEN_KEEP_CACHE` tells the kernel to keep the page cache it already holds
/// for this inode instead of dropping it on every open. That is a large win for
/// the read-only bulk of a load order - BSAs, meshes, textures the game opens
/// over and over - and it is why it is here.
///
/// It is only sound for a file whose BYTES CANNOT CHANGE UNDER THE INODE, which
/// is why `immutable` is a parameter rather than assumed. An earlier version set
/// it unconditionally, reasoning that "the layers are immutable and every write
/// goes through this daemon". Both halves are true and the conclusion is still
/// wrong, twice over:
///
///   * The overwrite layer is not immutable. The game writes its shader cache,
///     its INIs and its saves there and reads them back within the session.
///   * A lower-layer file does not keep its identity. The first write COPIES IT
///     UP, so the same virtual path - and the same FUSE inode - is suddenly
///     backed by a different file on disk, while the kernel still holds pages it
///     read from the old one. With `FUSE_PASSTHROUGH` the kernel serves those
///     reads without ever asking the daemon, so nothing downstream can notice.
///
/// SO IT IS OFF BY DEFAULT, and stays off until someone can explain the crash
/// below rather than argue it away.
///
/// Measured on Skyrim SE 1.6.1170 under proton-cachyos 11.0, with ZERO mods
/// installed: the game reaches its main menu and then dies on a null dereference
/// (`0xc0000005`, address `0x48`) on a worker thread, deterministically, at the
/// same instruction every run. Turning this one flag off fixes it while
/// `FOPEN_CACHE_DIR`, the 1-hour attribute TTL and the negative-dentry cache all
/// stay on - each was bisected out individually and only this one mattered.
///
/// Restricting it to files outside the overwrite layer was tried and did NOT
/// help, which rules out the obvious copy-up explanation: the game crashes while
/// reading files that were never written to. The interaction with
/// `FUSE_PASSTHROUGH` - where the kernel serves reads from the backing file
/// without consulting this daemon at all - was the open question; passthrough is
/// now off by default too (see [`passthrough_enabled`]), so the two are no longer
/// entangled and this one can be re-tested on its own.
///
/// `EIDOS_FUSE_KEEP_CACHE=1` turns it back on for whoever picks that up.
pub(crate) fn file_open_flags() -> FopenFlags {
    if std::env::var("EIDOS_FUSE_KEEP_CACHE").is_ok_and(|v| v != "0") && !Cache::Keep.is_off() {
        FopenFlags::FOPEN_KEEP_CACHE
    } else {
        FopenFlags::empty()
    }
}

/// Whether to hand the kernel a backing file so it can serve reads without us.
///
/// OFF BY DEFAULT, because with it on Skyrim SE cannot open its own content.
///
/// Measured A/B on Skyrim SE 1.6.1170 under proton-cachyos 11.0, same 82-plugin
/// load order, `WINEDEBUG=+file` both runs, the only variable being whether the
/// binary carried `cap_sys_admin` (without it `open_backing` returns EPERM and
/// the daemon serves reads itself):
///
/// | passthrough | `NtCreateFile` failures with `STATUS_ACCESS_VIOLATION` |
/// |-------------|--------------------------------------------------------|
/// | on          | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
/// | off         | 0                                                      |
///
/// With it on, the game loads no mod content at all: every plugin and archive it
/// wants for the whole session fails to open, which surfaces in-game as mods that
/// are simply not there. With it off the same load order reaches gameplay with
/// its plugins, archives and Papyrus scripts live.
///
/// The failure is INVISIBLE from here, which is why it took so long to find: our
/// own `open` succeeds every time and `open_backing` is never refused (verified
/// with `EIDOS_FUSE_TRACE=open` - zero `open FAILED`, zero `passthrough refused`
/// across a full failing session). The kernel produces the error after we reply
/// `opened_passthrough`, so no amount of daemon-side logging shows it. Whatever
/// the mechanism, it is not extension-specific: it hits archives and plugins
/// alike, i.e. exactly the files the game holds open for its whole run.
///
/// The cost of leaving it off is throughput, not correctness: reads take a
/// userspace round-trip instead of going straight to the backing file. That is
/// already what every rootless install does, since registering a backing fd needs
/// CAP_SYS_ADMIN in the initial user namespace.
///
/// `EIDOS_FUSE_PASSTHROUGH=1` turns it back on, for measuring the throughput it
/// buys or for re-testing once someone can explain the failure above.
pub(crate) fn passthrough_enabled() -> bool {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("EIDOS_FUSE_PASSTHROUGH").is_ok_and(|v| {
            let v = v.trim();
            !v.is_empty() && v != "0"
        })
    });
    *ON
}

/// Whether `EIDOS_FUSE_TRACE` names this trace channel (comma-separated, or `1`
/// for all of them). Diagnostic only, and off unless asked for: the op counters
/// say HOW MANY, this says WHICH, which is the difference between knowing a
/// directory is enumerated 50000 times and knowing which directory it is.
pub(crate) fn trace_enabled(channel: &str) -> bool {
    let Ok(v) = std::env::var("EIDOS_FUSE_TRACE") else {
        return false;
    };
    let v = v.trim();
    !v.is_empty() && (v == "1" || v.split(',').any(|c| c.trim().eq_ignore_ascii_case(channel)))
}

/// Raise this process's open-file soft limit to its hard limit. Best-effort:
/// every failure path leaves us exactly where we started.
pub(crate) fn raise_fd_limit() {
    // SAFETY: getrlimit/setrlimit with a valid, fully-initialised rlimit struct.
    unsafe {
        let mut lim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) != 0 || lim.rlim_cur >= lim.rlim_max {
            return;
        }
        lim.rlim_cur = lim.rlim_max;
        let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &lim);
    }
}

pub(crate) fn mount_config() -> Config {
    // Config is #[non_exhaustive]; build via Default and set the public field.
    let mut config = Config::default();
    config.mount_options = vec![MountOption::FSName("eidos".to_string())];

    // Serve requests from several event loops. A game's loading screen issues
    // metadata requests far faster than one thread can answer them while each
    // answer costs real directory I/O, so a single loop serialises the whole
    // startup behind the slowest resolution. `clone_fd` gives each worker its own
    // /dev/fuse fd (Linux 4.5+), which is what makes the extra threads actually
    // parallel rather than contending on one channel.
    //
    // The handlers are already safe for this: every piece of shared state is
    // behind a mutex, and `Filesystem` takes `&self`. Bounded at 4 because the
    // work is I/O-bound; more threads buy nothing and cost context switches.
    config.n_threads = Some(fuse_threads());
    config.clone_fd = true;
    config
}

/// Event-loop threads to run. Defaults to 4, capped by the machine's parallelism,
/// and overridable with `EIDOS_FUSE_THREADS` (1 makes the daemon single-threaded
/// again, which is the first thing to try when diagnosing a concurrency bug).
pub(crate) fn fuse_threads() -> usize {
    if let Some(n) = std::env::var("EIDOS_FUSE_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        return n.max(1);
    }
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    cpus.clamp(1, 4)
}
