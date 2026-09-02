//! Is the layer index actually active on a real instance?
//!
//! ```text
//! cargo run --release -p eidos-core --example index_health -- \
//!     ~/.local/share/eidos/skyrimse/mods ~/.local/share/eidos/skyrimse/overwrite
//! ```
//!
//! The index is all-or-nothing and built in silence: `LayerStack::new` either
//! gets a complete map of the read-only layers or `None`, and every caller then
//! walks the layers exactly as it did before the index existed. Nothing in a
//! session log distinguishes the two, so a stack that quietly falls back looks
//! identical to one that is working - while paying the old cost.
//!
//! This asks the question the only way that cannot be fooled: resolve real paths
//! with the index, then again with `EIDOS_NO_INDEX=1`, and compare the DIRECTORY
//! SCANS each performs. A working index answers from memory and scans nothing;
//! if both sides scan the same, there is no index.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(mods), Some(overwrite)) = (args.next(), args.next()) else {
        eprintln!("usage: index_health <mods-dir> <overwrite-dir>");
        std::process::exit(2);
    };

    let mut layers: Vec<PathBuf> = fs::read_dir(&mods)
        .unwrap_or_else(|e| panic!("cannot read {mods}: {e}"))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    layers.sort();
    println!("{} layers under {mods}", layers.len());

    // Real paths, taken from the layers themselves: a path nothing provides
    // would be answered "absent" by both sides for different reasons.
    //
    // Sampled from EVERY layer, a few each, rather than filling the quota from
    // the first ones. A file in the first layer is found before any layer has to
    // miss, so a sample drawn only from there measures the cheapest case and
    // makes the walk look far better than it is - which is exactly the mistake
    // this comment exists to stop the next person repeating.
    let mut probes: Vec<String> = Vec::new();
    let per_layer = (400 / layers.len().max(1)).max(4);
    for layer in &layers {
        let mut here: Vec<String> = Vec::new();
        collect(layer, layer, &mut here, 0, per_layer);
        probes.extend(here);
    }
    if probes.is_empty() {
        println!("no files found under the layers - nothing to measure");
        return;
    }
    println!("{} real paths sampled\n", probes.len());

    let built = std::time::Instant::now();
    let indexed = eidos_core::LayerStack::new(layers.clone(), PathBuf::from(&overwrite));
    let build = built.elapsed();
    let with = measure(&indexed, &probes);

    // SAFETY: single-threaded, and the variable is read once per LayerStack::new.
    unsafe { std::env::set_var("EIDOS_NO_INDEX", "1") };
    let walked = eidos_core::LayerStack::new(layers, PathBuf::from(&overwrite));
    unsafe { std::env::remove_var("EIDOS_NO_INDEX") };
    let without = measure(&walked, &probes);

    println!("                     with index   forced walk");
    println!("  probes             {:>10}   {:>11}", with.0, without.0);
    println!("  directory scans    {:>10}   {:>11}", with.1, without.1);
    println!("  build time         {build:>10.2?}\n");

    // A working index resolves from memory. Allow a little slack rather than
    // demanding zero: the overwrite is deliberately not indexed, so its own
    // lookups still scan.
    if with.1 * 4 < without.1 {
        println!(
            "VERDICT: the index is ACTIVE ({}x fewer scans)",
            without.1 / with.1.max(1)
        );
    } else {
        println!("VERDICT: the index is NOT in use - every resolve is walking the layers.");
        println!("  `LayerStack::new` discarded it. It is abandoned on ANY doubt:");
        println!("  an unreadable directory, a non-UTF-8 name, a tree deeper than 64,");
        println!("  or more than 4,000,000 entries.");
    }
}

fn measure(stack: &eidos_core::LayerStack, probes: &[String]) -> (u64, u64) {
    let s = stack.resolve_stats();
    let (p0, c0) = (
        s.probes.load(Ordering::Relaxed),
        s.scans.load(Ordering::Relaxed),
    );
    for p in probes {
        let _ = stack.resolve_read(p);
    }
    (
        s.probes.load(Ordering::Relaxed) - p0,
        s.scans.load(Ordering::Relaxed) - c0,
    )
}

fn collect(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<String>,
    depth: usize,
    want: usize,
) {
    if depth > 6 || out.len() >= want {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(root, &p, out, depth + 1, want);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
        if out.len() >= want {
            return;
        }
    }
}
