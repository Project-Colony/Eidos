//! What one path resolution costs, on a stack the size of a real setup.
//!
//! Run: `cargo run --release -p eidos-core --example resolve_cost`
//!
//! The figure that matters is not the duration - that moves with the machine and
//! the page cache - but the SYSCALL COUNTS, which are deterministic. A resolve
//! whose cost is proportional to (layers x depth) is the thing being fixed; one
//! that costs a constant is the thing being aimed at.
//!
//! Baseline measured 2026-07-28, before any index: 95 probes and 37 directory
//! scans per resolve, 1.35ms each. A real session made 20,342 resolves from the
//! FUSE handlers alone, which is where a twenty-second load goes.

use std::fs;
use std::sync::atomic::Ordering;

/// Enabled mods in the setup this was measured against.
const LAYERS: usize = 27;
/// Files per layer directory, so a scan costs what it costs in real life rather
/// than what it costs on an empty directory.
const FILES_PER_DIR: usize = 200;
const RESOLVES: u32 = 200;

fn main() {
    let root = std::env::temp_dir().join("eidos-resolve-cost");
    let _ = fs::remove_dir_all(&root);

    // The ordinary case, and the expensive one: 26 mods that do not have the file
    // and one that does. Median real path depth is 6 components.
    let rel = "textures/actors/character/male/body/skin.dds";
    let mut layers = Vec::new();
    for i in 0..LAYERS {
        let l = root.join(format!("layer{i:02}"));
        fs::create_dir_all(l.join("textures/actors")).unwrap();
        for j in 0..FILES_PER_DIR {
            fs::write(l.join(format!("textures/actors/f{j:04}.dds")), b"x").unwrap();
        }
        layers.push(l);
    }
    let last = layers.last().unwrap().clone();
    fs::create_dir_all(last.join("textures/actors/character/male/body")).unwrap();
    fs::write(last.join(rel), b"data").unwrap();
    let overwrite = root.join("overwrite");
    fs::create_dir_all(&overwrite).unwrap();

    let stack = eidos_core::LayerStack::new(layers, overwrite);
    let s = stack.resolve_stats();
    let (p0, c0) = (s.probes.load(Ordering::Relaxed), s.scans.load(Ordering::Relaxed));

    let started = std::time::Instant::now();
    for _ in 0..RESOLVES {
        assert!(stack.resolve_read(rel).is_some(), "the file is in the last layer");
    }
    let elapsed = started.elapsed();

    let probes = s.probes.load(Ordering::Relaxed) - p0;
    let scans = s.scans.load(Ordering::Relaxed) - c0;
    println!("{RESOLVES} resolves of one path, {LAYERS} layers, depth 6:");
    println!("  probes {probes:>7}  ({} per resolve)", probes / u64::from(RESOLVES));
    println!("  scans  {scans:>7}  ({} per resolve)", scans / u64::from(RESOLVES));
    println!("  time   {elapsed:>9.2?}  ({:?} per resolve)", elapsed / RESOLVES);

    let _ = fs::remove_dir_all(&root);
}
