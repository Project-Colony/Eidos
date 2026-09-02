//! End-to-end check of a real session: the public API, the global logger, the
//! file that lands on disk.
//!
//! Everything lives in ONE test function on purpose. The logger is process-wide
//! and initialises once, so a second `#[test]` calling `init_with` would race
//! the first and silently observe someone else's session.

use std::fs;
use std::path::{Path, PathBuf};

use eidos_log::{Config, Level};

/// Session file names the crate would have produced on 2023-11-14, used to
/// pre-fill the rotation bucket with sessions older than the one about to open.
fn stale(dir: &Path, i: usize) -> PathBuf {
    let p = dir.join(format!("sse-test.20231114-2213{i:02}.{i}.log"));
    fs::write(&p, b"old session\n").unwrap();
    p
}

#[test]
fn a_session_writes_a_header_then_redacted_levelled_lines_and_rotates() {
    let dir = std::env::temp_dir().join(format!("eidos-log-session-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let old: Vec<PathBuf> = (0..5).map(|i| stale(&dir, i)).collect();

    let mut cfg = Config::new("sse-test").with_version("eidos 1.2.3");
    cfg.dir = dir.clone();
    cfg.keep = 2;
    cfg.file_level = Level::Debug;
    // Do not pollute the test runner's output; the tee is exercised by having a
    // level below the threshold, not by capturing stderr.
    cfg.stderr = false;

    let path = eidos_log::init_with(cfg).expect("a session log should open under a temp dir");
    assert!(path.starts_with(&dir));
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("sse-test."),
        "{name} should be bucketed by instance"
    );
    assert!(name.ends_with(".log"), "{name} should be a .log");

    // Rotation ran while opening: the current file plus one older survivor.
    let left = fs::read_dir(&dir).unwrap().count();
    assert_eq!(left, 2, "keep = 2 should have pruned the stale sessions");
    assert!(
        old[4].exists(),
        "the newest stale session should be the survivor"
    );
    assert!(old[..4].iter().all(|p| !p.exists()));

    eidos_log::info!("deployed {} mods", 3);
    eidos_log::warn!("no Proton prefix, skipping plugins.txt");
    eidos_log::error!("mount failed: {}", "operation not permitted");
    eidos_log::debug!("resolved 812 layers");
    // Repeat init is a no-op that reports the live session, which is how the
    // GUI asks "where is my log?" without threading state around.
    assert_eq!(eidos_log::path().as_deref(), Some(path.as_path()));

    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.starts_with("==== eidos session log ===="),
        "missing header:\n{text}"
    );
    assert!(text.contains("instance : sse-test"));
    assert!(text.contains("version  : eidos 1.2.3"));
    assert!(text.contains(&format!("pid      : {}", std::process::id())));
    assert!(text.contains("INFO  deployed 3 mods"), "{text}");
    assert!(text.contains("WARN  no Proton prefix"), "{text}");
    assert!(
        text.contains("ERROR mount failed: operation not permitted"),
        "{text}"
    );
    assert!(text.contains("DEBUG resolved 812 layers"), "{text}");

    // Every record carries a full local timestamp, so a single pasted line is
    // still interpretable.
    let record = text
        .lines()
        .find(|l| l.contains("INFO  deployed"))
        .expect("the info record");
    let stamp = &record[..23];
    assert_eq!(
        stamp.len(),
        23,
        "expected `YYYY-MM-DD HH:MM:SS.mmm`, got {record}"
    );
    assert!(
        stamp.chars().filter(|c| *c == '-').count() == 2 && stamp.contains(':'),
        "{record}"
    );

    // The header echoes argv, which under a normal `cargo test` is the test
    // binary inside the user's home: nothing that identifies them may survive.
    if let Ok(home) = std::env::var("HOME") {
        let home = home.trim_end_matches('/').to_string();
        if home.len() >= 2 && home.starts_with('/') {
            assert!(
                !text.contains(&home),
                "the home path leaked into the log:\n{text}"
            );
            eidos_log::info!("archive at {}/Downloads/mod.7z", home);
            let text = fs::read_to_string(&path).unwrap();
            assert!(text.contains("archive at ~/Downloads/mod.7z"), "{text}");
            assert!(!text.contains(&home));
        }
    }

    // The FUSE daemon answers kernel requests on many threads and the GUI logs
    // from its own: every record has to land whole, never spliced into another.
    let writers: Vec<_> = (0..8)
        .map(|t| {
            std::thread::spawn(move || {
                for i in 0..50 {
                    eidos_log::info!("thread {t} record {i:02}");
                }
            })
        })
        .collect();
    for w in writers {
        w.join().unwrap();
    }

    let text = fs::read_to_string(&path).unwrap();
    let records: Vec<&str> = text.lines().filter(|l| l.contains(" record ")).collect();
    assert_eq!(records.len(), 400, "some records were lost");
    // `YYYY-MM-DD HH:MM:SS.mmm ` + `INFO  ` + `thread N record NN`: a torn line
    // would be short, and a spliced pair would be long.
    assert!(
        records.iter().all(|l| l.len() == 24 + 6 + 18),
        "a record was torn or spliced: {records:?}"
    );
    let unique: std::collections::HashSet<&str> = records.iter().map(|l| &l[30..]).collect();
    assert_eq!(unique.len(), 400, "records were duplicated or corrupted");

    let _ = fs::remove_dir_all(&dir);
}
