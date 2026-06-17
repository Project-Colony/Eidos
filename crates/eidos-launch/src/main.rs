//! `eidos-launch` CLI: run a command through the Eidos union view, isolated.
//!
//!   eidos-launch --layer <dir> [--layer <dir> ...] [--overwrite <dir>] \
//!                --mount <mountpoint> -- <command> [args...]
//!
//! The first `--layer` has the highest priority. The union is mounted in a
//! private user+mount namespace and the command runs inside it. See the
//! `eidos` binary for the game-aware front end.

use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::exit;

use eidos_launch::{launch, LaunchSpec};

fn usage() -> ! {
    eprintln!(
        "usage: eidos-launch --layer <dir> [--layer <dir> ...] [--overwrite <dir>] \\\n\
         \x20            --mount <mountpoint> -- <command> [args...]"
    );
    exit(2);
}

fn main() {
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

    let spec = LaunchSpec {
        layers,
        overwrite,
        mountpoint,
        command,
        env: Vec::new(),
        base_bind: None,
        binds: Vec::new(),
        cwd: None,
    };
    match launch(spec) {
        // Propagate the child's real status. `code()` is `None` when the child was
        // killed by a signal, so fall back to the shell convention 128 + signal -
        // otherwise a signal-killed (crashed) command would exit 0 and hide the
        // failure, matching what the `eidos` front end already does.
        Ok(status) => exit(status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1))),
        Err(e) => {
            eprintln!("eidos-launch: {e}");
            exit(1)
        }
    }
}
