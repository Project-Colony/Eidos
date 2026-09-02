//! `eidos-fuse` CLI: mount a read-write union of mod layers over game data.
//!
//!   eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
//!
//! The first `--layer` has the highest priority; the last is typically the
//! pristine game data directory. Unmount with `fusermount3 -u <mountpoint>`.

use std::fs;
use std::path::PathBuf;
use std::process::exit;

use eidos_fuse::Eidos;

fn usage() -> ! {
    eprintln!(
        "usage: eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>\n\
         \n\
         The first --layer has the highest priority; the last is typically the\n\
         pristine game data directory. Unmount with: fusermount3 -u <mountpoint>"
    );
    exit(2);
}

fn main() -> std::io::Result<()> {
    let mut layers: Vec<PathBuf> = Vec::new();
    let mut overwrite: Option<PathBuf> = None;
    let mut mountpoint: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--layer" => layers.push(PathBuf::from(args.next().unwrap_or_else(|| usage()))),
            "--overwrite" => {
                overwrite = Some(PathBuf::from(args.next().unwrap_or_else(|| usage())))
            }
            "-h" | "--help" => usage(),
            _ => mountpoint = Some(PathBuf::from(arg)),
        }
    }

    let (Some(mountpoint), false) = (mountpoint, layers.is_empty()) else {
        usage();
    };

    let overwrite = overwrite.unwrap_or_else(|| {
        let p = std::env::temp_dir().join(format!("eidos-overwrite-{}", std::process::id()));
        let _ = fs::create_dir_all(&p);
        p
    });

    eprintln!("eidos-fuse: mounting at {}", mountpoint.display());
    Eidos::new(layers, overwrite).mount(&mountpoint)
}
