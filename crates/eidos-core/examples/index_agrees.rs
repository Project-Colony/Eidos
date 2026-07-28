//! Differential test: the index must answer exactly what the layer walk answers.
//!
//! Run against a real instance:
//! ```text
//! cargo run --release -p eidos-core --example index_agrees -- \
//!     ~/.local/share/eidos/skyrimse/mods ~/.local/share/eidos/skyrimse/overwrite
//! ```
//!
//! Playing the game tests the paths the game happens to ask for. This tests
//! EVERY path in the stack, plus a set of near-misses designed to catch the one
//! failure that matters: an index that confidently says a file is absent when the
//! walk would have found it. A missing mod asset is silent - the game does not
//! crash, it renders a texture that is not there - so it has to be caught here
//! rather than noticed later.
//!
//! Both sides are the real `resolve_read`. One `LayerStack` is built normally,
//! the other with `EIDOS_NO_INDEX=1`, so this compares the two code paths that
//! actually ship, not a model of them.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(mods), Some(overwrite)) = (args.next(), args.next()) else {
        eprintln!("usage: index_agrees <mods-dir> <overwrite-dir>");
        std::process::exit(2);
    };
    let mods = PathBuf::from(mods);

    let mut layers: Vec<PathBuf> =
        fs::read_dir(&mods).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    layers.sort();
    println!("{} layers under {}", layers.len(), mods.display());

    // Every virtual path any layer provides, plus every ancestor directory: the
    // index answers for directories too, and a wrong answer there hides a whole
    // subtree rather than one file.
    let mut vpaths: HashSet<String> = HashSet::new();
    for l in &layers {
        collect(l, l, &mut vpaths);
    }
    println!("{} distinct virtual paths", vpaths.len());

    // Near-misses. The index answers "absent" from memory, so the paths it has
    // never seen are exactly where a build bug would show.
    let mut probes: Vec<String> = vpaths.iter().cloned().collect();
    probes.sort();
    for v in vpaths.iter().take(4000) {
        probes.push(v.to_ascii_uppercase());
        probes.push(v.to_ascii_lowercase());
        probes.push(format!("{v}.nope"));
        probes.push(format!("{v}/child"));
    }
    for v in ["", "/", "does/not/exist.esp", "../escape", "a/../b"] {
        probes.push(v.to_string());
    }

    let indexed = eidos_core::LayerStack::new(layers.clone(), PathBuf::from(&overwrite));
    // SAFETY: single-threaded, and the variable is read once per LayerStack::new.
    unsafe { std::env::set_var("EIDOS_NO_INDEX", "1") };
    let walked = eidos_core::LayerStack::new(layers, PathBuf::from(&overwrite));
    unsafe { std::env::remove_var("EIDOS_NO_INDEX") };

    let mut disagreements = 0usize;
    for p in &probes {
        let a = indexed.resolve_read(p);
        let b = walked.resolve_read(p);
        if a != b {
            disagreements += 1;
            if disagreements <= 20 {
                println!("  DISAGREE {p}\n    index {a:?}\n    walk  {b:?}");
            }
        }
    }

    println!("\n{} paths compared, {disagreements} disagreements", probes.len());
    if disagreements == 0 {
        println!("the index and the walk are indistinguishable on this instance");
    }
    std::process::exit(i32::from(disagreements != 0));
}

fn collect(root: &Path, dir: &Path, out: &mut HashSet<String>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if let Ok(rel) = p.strip_prefix(root) {
            out.insert(rel.to_string_lossy().replace('\\', "/"));
        }
        if p.is_dir() {
            collect(root, &p, out);
        }
    }
}
