//! What this process was started to be, decided before anything else runs.
//!
//! `eidos-linux`, the one file a Colony install has, is both programs: the
//! window, and the `eidos` CLI when it is called through a link named `eidos`
//! or with [`eidos::CLI_MARKER`] first. The window also finds its CLI that way
//! when nothing else is installed beside it (see `find_eidos_binary`).

use std::path::Path;

#[derive(Debug, PartialEq)]
pub(crate) enum Start {
    /// The CLI, with its arguments. `--version` lands here too: the CLI answers
    /// it before it migrates, logs or locks anything.
    Cli(Vec<String>),
    /// The window, with Steam's `%command%` (or nothing) as its arguments.
    Window(Vec<String>),
}

/// Only the FIRST argument is looked at: Steam's `%command%` is a path to a
/// Proton script followed by the game's own arguments, so a `--version` further
/// along belongs to the game, never to Eidos.
pub(crate) fn start_as(argv0: &str, args: Vec<String>) -> Start {
    if Path::new(argv0)
        .file_name()
        .is_some_and(|name| name == "eidos")
    {
        return Start::Cli(args);
    }
    match args.first().map(String::as_str) {
        Some(eidos::CLI_MARKER | "--version" | "-V") => Start::Cli(args),
        _ => Start::Window(args),
    }
}

/// Give up every capability before the window starts, or refuse to start it.
///
/// The CLI may carry the opt-in `cap_sys_admin` (FUSE passthrough), and in
/// `eidos-linux` the CLI and the window are one file, so a capability set on it
/// would also reach the window: image decoders, archive previews and a whole
/// GUI toolkit, none of which needs one. The bounding set is left alone, so an
/// `eidos` child still gets its own file's capability at exec.
pub(crate) fn drop_capabilities() -> Result<(), String> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("cannot read /proc/self/status: {e}"))?;
    if !holds_capabilities(&status) {
        return Ok(());
    }
    // capset(2), _LINUX_CAPABILITY_VERSION_3: two 32-bit words per set, all
    // zero, so nothing stays effective, permitted or inheritable.
    #[repr(C)]
    struct Header {
        version: u32,
        pid: libc::c_int,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Data {
        effective: u32,
        permitted: u32,
        inheritable: u32,
    }
    let mut header = Header {
        version: 0x2008_0522,
        pid: 0,
    };
    let data = [Data {
        effective: 0,
        permitted: 0,
        inheritable: 0,
    }; 2];
    // SAFETY: both pointers are to the layouts capset(2) documents for version 3,
    // and both outlive the call.
    let rc = unsafe { libc::syscall(libc::SYS_capset, &mut header as *mut Header, data.as_ptr()) };
    if rc == 0 {
        return Ok(());
    }
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    Err(format!(
        "this binary carries a capability and could not drop it ({}), so the window \
         will not start with it. Remove it with: sudo setcap -r {exe}",
        std::io::Error::last_os_error()
    ))
}

/// Whether `/proc/self/status` shows a permitted capability. Permitted rather
/// than effective, since a permitted one can be made effective at any time. An
/// unreadable line counts as holding one: dropping costs nothing.
fn holds_capabilities(status: &str) -> bool {
    status
        .lines()
        .filter_map(|line| line.strip_prefix("CapPrm:"))
        .any(|v| u64::from_str_radix(v.trim(), 16) != Ok(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn version_and_the_marker_go_to_the_cli() {
        for first in ["--version", "-V", eidos::CLI_MARKER] {
            let args = v(&[first, "games"]);
            assert_eq!(start_as("/opt/eidos-linux", args.clone()), Start::Cli(args));
        }
    }

    #[test]
    fn a_link_named_eidos_is_the_cli() {
        assert_eq!(
            start_as("/home/u/.local/bin/eidos", v(&["games"])),
            Start::Cli(v(&["games"]))
        );
        assert_eq!(start_as("eidos", vec![]), Start::Cli(vec![]));
    }

    #[test]
    fn a_steam_command_is_never_intercepted() {
        // `--version` after the game path is the game's argument, not ours.
        let steam = v(&[
            "/home/u/.steam/steam/steamapps/common/Proton - Experimental/proton",
            "waitforexitandrun",
            "/games/Skyrim/SkyrimSELauncher.exe",
            "--version",
        ]);
        assert_eq!(
            start_as("/opt/eidos-linux", steam.clone()),
            Start::Window(steam)
        );
        assert_eq!(
            start_as("/usr/bin/eidos-gui", vec![]),
            Start::Window(vec![])
        );
        // A name that merely starts with `eidos` is not the CLI.
        assert_eq!(
            start_as("/usr/bin/eidos-gui", v(&["x"])),
            Start::Window(v(&["x"]))
        );
    }

    #[test]
    fn capabilities_are_read_from_the_permitted_set() {
        let none = "Name:\teidos-gui\nCapInh:\t0000000000000000\nCapPrm:\t0000000000000000\nCapEff:\t0000000000000000\n";
        assert!(!holds_capabilities(none));
        // cap_sys_admin is bit 21.
        let admin = "CapPrm:\t0000000000200000\nCapEff:\t0000000000000000\n";
        assert!(holds_capabilities(admin));
        assert!(holds_capabilities("CapPrm:\tnot-hex\n"));
    }
}
