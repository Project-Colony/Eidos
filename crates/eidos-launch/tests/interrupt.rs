//! Its own process, so the global stop flag touches no other test.

#[test]
fn ctrl_c_is_a_stop_request() {
    eidos_launch::install_stop_handler();
    assert!(!eidos_launch::stop_requested());
    // SAFETY: raise has no memory-safety preconditions; the handler only stores an atomic.
    unsafe { libc::raise(libc::SIGINT) };
    assert!(eidos_launch::stop_requested());
}
