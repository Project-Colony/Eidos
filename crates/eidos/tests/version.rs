//! `--version` is the release workflow's smoke test, so it must answer before
//! the CLI migrates, logs or locks anything.

use std::process::Command;

#[test]
fn version_exits_zero_and_touches_nothing() {
    let home = std::env::temp_dir().join(format!("eidos-version-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    for args in [&["--version"][..], &["-V"], &["--eidos-cli", "--version"]] {
        let out = Command::new(env!("CARGO_BIN_EXE_eidos"))
            .args(args)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("XDG_STATE_HOME", home.join("state"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{args:?}: {out:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            format!("eidos {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
    let left: Vec<_> = std::fs::read_dir(&home).unwrap().collect();
    assert!(left.is_empty(), "--version wrote {left:?}");
    std::fs::remove_dir_all(&home).unwrap();
}
