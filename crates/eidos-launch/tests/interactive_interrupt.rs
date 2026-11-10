//! Its own process, so the SIGINT disposition it sets touches no other test.

/// An interactive shell run must not hand an ignored SIGINT down: bash keeps an
/// inherited ignore for every command it starts, so Ctrl+C would do nothing there.
#[test]
fn an_interactive_run_leaves_ctrl_c_to_the_programs_it_starts() {
    eidos_launch::install_stop_handler();
    eidos_launch::ignore_interrupt();

    let out = std::process::Command::new("sh")
        .args(["-c", "grep SigIgn /proc/self/status"])
        .output()
        .unwrap();
    let line = String::from_utf8(out.stdout).unwrap();
    let mask = line
        .trim()
        .strip_prefix("SigIgn:")
        .map(|m| u64::from_str_radix(m.trim(), 16).unwrap())
        .unwrap_or_else(|| panic!("no SigIgn line: {line:?}"));
    assert_eq!(mask & 0x2, 0, "SIGINT is ignored in the child: {line}");

    // And a stray SIGINT still does not end the run.
    // SAFETY: raise has no memory-safety preconditions; the handler does nothing.
    unsafe { libc::raise(libc::SIGINT) };
    assert!(!eidos_launch::stop_requested());
}
