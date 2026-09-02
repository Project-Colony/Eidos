//! Attribution bench for path resolution, against a REAL instance, read-only.
//!
//! Answers one question with numbers instead of theory: when a session spends
//! millions of directory scans in resolution, which phase pays them - the
//! un-indexed Overwrite pre-check, or index fallbacks walking the layers?
//!
//!     resolve_bench <layers.txt> <overwrite-dir> [samples-per-class]
//!
//! `layers.txt` is one directory per line, highest priority first, the game's
//! own data directory last - the same order the mount uses. The workload is
//! three classes, equal sized: existing paths spelled exactly, the same paths
//! case-mangled (Wine's usual spelling), and guaranteed misses (Wine probes far
//! more paths than exist).

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

use eidos_core::LayerStack;

fn collect(dir: &Path, base: &Path, out: &mut Vec<String>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        if out.len() >= cap {
            return;
        }
        let p = e.path();
        if let Ok(rel) = p.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
        if p.is_dir() {
            collect(&p, base, out, cap);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let layers_file = args.next().expect("layers.txt");
    let overwrite = PathBuf::from(args.next().expect("overwrite dir"));
    let cap: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(40_000);

    let layers: Vec<PathBuf> = std::fs::read_to_string(&layers_file)
        .expect("read layers.txt")
        .lines()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect();
    eprintln!(
        "layers: {}   overwrite: {}",
        layers.len(),
        overwrite.display()
    );

    // Sample real paths across layers proportionally to have mod paths AND game
    // paths in the workload, not just whichever layer enumerates first.
    let mut existing: Vec<String> = Vec::new();
    let per_layer = (cap / layers.len().max(1)).max(50);
    for l in &layers {
        let mut chunk = Vec::new();
        collect(l, l, &mut chunk, per_layer);
        existing.extend(chunk);
    }
    existing.sort();
    existing.dedup();
    existing.truncate(cap);

    let mangled: Vec<String> = existing.iter().map(|p| p.to_ascii_uppercase()).collect();
    let missing: Vec<String> = existing.iter().map(|p| format!("{p}-zzq")).collect();

    let stack = LayerStack::new(layers, overwrite);
    let st = stack.resolve_stats();

    let mut report = |label: &str, work: &[String]| {
        let snap = || {
            [
                st.probes.load(Relaxed),
                st.scans.load(Relaxed),
                st.ow_probes.load(Relaxed),
                st.ow_scans.load(Relaxed),
                st.walk_probes.load(Relaxed),
                st.walk_scans.load(Relaxed),
                st.idx_hits.load(Relaxed),
                st.idx_negatives.load(Relaxed),
                st.idx_fallbacks.load(Relaxed),
                st.idx_absent.load(Relaxed),
            ]
        };
        let before = snap();
        let t = std::time::Instant::now();
        let mut found = 0usize;
        for p in work {
            if stack.resolve_read(p).is_some() {
                found += 1;
            }
        }
        let elapsed = t.elapsed();
        let after = snap();
        let d: Vec<u64> = after
            .iter()
            .zip(before.iter())
            .map(|(a, b)| a - b)
            .collect();
        println!(
            "{label:>9}: {n} resolves, {found} found, {ms} ms ({us:.1} µs/resolve)\n\
             {pad}  probes {p} (overwrite {owp}, walk {wp})  scans {s} (overwrite {ows}, walk {ws})\n\
             {pad}  index: {h} hits, {neg} negatives, {fb} ambiguous->walk, {ab} absent->walk",
            n = work.len(),
            ms = elapsed.as_millis(),
            us = elapsed.as_secs_f64() * 1e6 / work.len().max(1) as f64,
            pad = "",
            p = d[0], owp = d[2], wp = d[4],
            s = d[1], ows = d[3], ws = d[5],
            h = d[6], neg = d[7], fb = d[8], ab = d[9],
        );
    };

    report("exact", &existing);
    report("mangled", &mangled);
    report("missing", &missing);
}
