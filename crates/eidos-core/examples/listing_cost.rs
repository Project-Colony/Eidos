//! What a directory listing costs, with and without the children index.
//!
//! ```text
//! cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
//! ```
//!
//! `readdir` was the one handler the path index did nothing for. It needs every
//! layer's copy of one directory, merged, while the index only knew which layer
//! wins - so a listing on a 50-layer stack cost 50 case-folding walks whatever
//! the index held. This measures the difference that closing that gap makes.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(mods), Some(overwrite)) = (args.next(), args.next()) else {
        eprintln!("usage: listing_cost <mods-dir> <overwrite-dir>");
        std::process::exit(2);
    };

    let mut layers: Vec<PathBuf> = fs::read_dir(&mods)
        .unwrap_or_else(|e| panic!("cannot read {mods}: {e}"))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    layers.sort();

    // The directories a game actually enumerates are the deep, heavily-shared
    // ones - meshes, textures, scripts - so sample virtual directories rather
    // than layer roots.
    let mut dirs: Vec<String> = Vec::new();
    for layer in &layers {
        collect(layer, layer, &mut dirs, 0);
    }
    dirs.sort();
    dirs.dedup();
    dirs.truncate(500);
    println!("{} layers, {} distinct virtual directories sampled\n", layers.len(), dirs.len());

    let indexed = eidos_core::LayerStack::new(layers.clone(), PathBuf::from(&overwrite));
    let with = run(&indexed, &dirs);
    // SAFETY: single-threaded, read once per LayerStack::new.
    unsafe { std::env::set_var("EIDOS_NO_INDEX", "1") };
    let walked = eidos_core::LayerStack::new(layers, PathBuf::from(&overwrite));
    unsafe { std::env::remove_var("EIDOS_NO_INDEX") };
    let without = run(&walked, &dirs);

    println!("                     with index   forced walk        ratio");
    row("probes", with.0, without.0);
    row("directory scans", with.1, without.1);
    println!("  {:<18} {:>10.2?}   {:>11.2?}", "wall clock", with.2, without.2);
}

fn row(label: &str, a: u64, b: u64) {
    let ratio =
        if a == 0 { "-".to_string() } else { format!("{:.1}x", b as f64 / a.max(1) as f64) };
    println!("  {label:<18} {a:>10}   {b:>11}   {ratio:>10}");
}

fn run(stack: &eidos_core::LayerStack, dirs: &[String]) -> (u64, u64, std::time::Duration) {
    let s = stack.resolve_stats();
    let (p0, c0) = (s.probes.load(Ordering::Relaxed), s.scans.load(Ordering::Relaxed));
    let t = std::time::Instant::now();
    for d in dirs {
        let _ = stack.list_dir_typed(d);
    }
    let elapsed = t.elapsed();
    (
        s.probes.load(Ordering::Relaxed) - p0,
        s.scans.load(Ordering::Relaxed) - c0,
        elapsed,
    )
}

fn collect(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>, depth: usize) {
    if depth > 5 || out.len() > 3000 {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/").to_ascii_lowercase());
            }
            collect(root, &p, out, depth + 1);
        }
    }
}
