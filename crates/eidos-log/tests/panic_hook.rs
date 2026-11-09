//! Own test binary: the logger and the panic hook are process-wide.

#[test]
fn a_panic_in_a_spawned_thread_is_written_to_the_session_log() {
    let dir = std::env::temp_dir().join(format!("eidos-log-panic-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut cfg = eidos_log::Config::new("panic-test");
    cfg.dir = dir.clone();
    cfg.stderr = false;
    let file = eidos_log::init_with(cfg).expect("session file");
    eidos_log::install_panic_hook();

    let t = std::thread::Builder::new()
        .name("worker-7".into())
        .spawn(|| panic!("boom {}", 42))
        .unwrap();
    assert!(t.join().is_err());

    let text = std::fs::read_to_string(file).unwrap();
    let line = text
        .lines()
        .find(|l| l.contains("panicked"))
        .expect("panic line");
    assert!(line.contains("ERROR"), "{line}");
    assert!(line.contains("worker-7"), "{line}");
    assert!(line.contains("boom 42"), "{line}");
    assert!(line.contains("panic_hook.rs"), "{line}");
    let _ = std::fs::remove_dir_all(dir);
}
