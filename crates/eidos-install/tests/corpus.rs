//! Characterisation test for the archive layout checker.
//!
//! This test asserts nothing about what the checker *should* say. It asserts that
//! what it says today, on a corpus of real mod layouts, is what it says tomorrow.
//! That is the point: the checker is about to stop reading a hardcoded Gamebryo
//! list and start reading per-game rules, and the only thing that makes that
//! refactor safe is a record of every verdict it produces now.
//!
//! `corpus/shapes.txt` is derived from a real Skyrim SE instance (49 distinct mod
//! root shapes, 7 downloaded archives) by `corpus/generate.py`, which keeps the
//! directory names the checker actually reads and replaces every other name with
//! `dN`. No mod name is in the repository.
//!
//! `corpus/expected.txt` is the golden record. To regenerate it after a
//! DELIBERATE behaviour change:
//!
//! ```text
//! EIDOS_BLESS=1 cargo test -p eidos-install --test corpus
//! ```
//!
//! Then read the diff. A line that moved without you meaning it is the bug.

use eidos_install::{ArchiveEntry, ArchiveTree, LayoutRules};
use std::fmt::Write as _;
use std::path::PathBuf;

/// The game a case is judged under when its header does not name one.
///
/// Every case names its game, and recording through `LayoutRules::for_game`
/// rather than a hand-built value is the point: it proves what the installer will
/// actually do for that game, not what it would do for something a test invented.
const GAME: &str = "skyrimse";

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

/// One corpus case: its id, the game whose rules it is judged under, and the
/// archive listing it stands for.
struct Case {
    id: String,
    game: String,
    entries: Vec<ArchiveEntry>,
}

/// Parse `shapes.txt`: `> <kind> <id> game=<id>` opens a case, following lines are
/// paths, a trailing `/` marks a directory. `#` comments and blank lines are
/// skipped. A header with no `game=` falls back to [`GAME`].
fn parse_shapes(text: &str) -> Vec<Case> {
    let mut cases: Vec<Case> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix("> ") {
            let header = header.trim();
            let game = header
                .split_whitespace()
                .find_map(|t| t.strip_prefix("game="))
                .unwrap_or(GAME)
                .to_string();
            cases.push(Case {
                id: header.to_string(),
                game,
                entries: Vec::new(),
            });
            continue;
        }
        let case = cases.last_mut().expect("a path line before any `>` header");
        let is_dir = line.ends_with('/');
        case.entries.push(ArchiveEntry {
            path: line.trim_end_matches('/').to_string(),
            is_dir,
        });
    }
    cases
}

/// Every verdict the checker produces for one tree, as stable text.
///
/// All four public entry points are recorded, not just `data_looks_valid`: the
/// other three call it internally, so a change to the check surfaces differently
/// in each of them (a wrapper that stops being stripped, a BAIN sub-package that
/// stops counting, a Root Builder split that stops resolving).
fn record(tree: &ArchiveTree, rules: LayoutRules) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "  data_looks_valid   = {:?}",
        tree.data_looks_valid(rules)
    );
    let _ = writeln!(
        out,
        "  simple_archive_base= {}",
        match tree.simple_archive_base(rules) {
            Some(base) => format!("{base:?}"),
            None => "none".to_string(),
        }
    );
    let (subs, invalid) = tree.bain_subpackages(rules);
    let _ = writeln!(out, "  bain_subpackages   = {subs:?} + {invalid} invalid");
    let _ = writeln!(
        out,
        "  root_builder_split = {}",
        match tree.root_builder_split(rules) {
            Some(split) => format!(
                "data={:?} root_dir={:?} root_entries={:?}",
                split.data_prefix, split.root_dir, split.root_entries
            ),
            None => "none".to_string(),
        }
    );
    out
}

#[test]
fn real_world_layouts_are_classified_exactly_as_before() {
    let dir = corpus_dir();
    let shapes = std::fs::read_to_string(dir.join("shapes.txt")).expect("corpus/shapes.txt");
    let cases = parse_shapes(&shapes);
    assert!(
        cases.len() > 40,
        "corpus shrank unexpectedly: {} cases",
        cases.len()
    );

    let mut actual = String::from(
        "# Golden record of the layout checker's verdicts on corpus/shapes.txt.\n\
         # Regenerate with: EIDOS_BLESS=1 cargo test -p eidos-install --test corpus\n\n",
    );
    for case in &cases {
        let tree = ArchiveTree::from_entries(&case.entries);
        let _ = writeln!(actual, "> {}", case.id);
        actual.push_str(&record(&tree, LayoutRules::for_game(&case.game)));
        actual.push('\n');
    }

    let golden = dir.join("expected.txt");
    if std::env::var_os("EIDOS_BLESS").is_some() {
        std::fs::write(&golden, &actual).expect("write expected.txt");
        eprintln!("blessed {} cases into {}", cases.len(), golden.display());
        return;
    }

    let expected = std::fs::read_to_string(&golden)
        .unwrap_or_else(|_| panic!("missing {}; create it with EIDOS_BLESS=1", golden.display()));
    if expected != actual {
        // Show the first divergence rather than dumping two 300-line blobs: the
        // failure that matters is "which case changed", not "the file differs".
        let first = expected
            .lines()
            .zip(actual.lines())
            .find(|(e, a)| e != a)
            .map(|(e, a)| format!("expected: {e}\n  actual: {a}"))
            .unwrap_or_else(|| "one record is a prefix of the other".to_string());
        panic!(
            "the checker's verdicts changed.\n\n{first}\n\n\
             If that change was deliberate, re-bless and read the diff:\n  \
             EIDOS_BLESS=1 cargo test -p eidos-install --test corpus"
        );
    }
}

/// The corpus is only worth having if it covers both answers. A corpus where
/// every case is Valid would pass a checker that always returns Valid.
#[test]
fn the_corpus_exercises_both_verdicts() {
    let shapes =
        std::fs::read_to_string(corpus_dir().join("shapes.txt")).expect("corpus/shapes.txt");
    let (mut valid, mut invalid, mut stripped) = (0, 0, 0);
    for case in parse_shapes(&shapes) {
        let tree = ArchiveTree::from_entries(&case.entries);
        match tree.data_looks_valid(LayoutRules::for_game(&case.game)) {
            eidos_install::CheckReturn::Valid => valid += 1,
            _ => invalid += 1,
        }
        // A non-empty base means a wrapper folder was stripped, which is the part
        // of the checker most likely to break silently.
        if tree
            .simple_archive_base(LayoutRules::for_game(&case.game))
            .is_some_and(|b| !b.is_empty())
        {
            stripped += 1;
        }
    }
    assert!(valid > 0, "no valid layout in the corpus");
    assert!(
        invalid > 0,
        "no invalid layout in the corpus: it cannot catch a checker that always says Valid"
    );
    assert!(stripped > 0, "no wrapper-stripping case in the corpus");
}

/// The corpus is only a proof about Skyrim if Skyrim's lookup lands on the
/// vocabulary the corpus was frozen under. If this ever fails, every record in
/// `expected.txt` is describing a game other than the one it came from.
#[test]
fn the_corpus_game_resolves_to_the_frozen_vocabulary() {
    let (skyrim, default) = (LayoutRules::for_game(GAME), LayoutRules::default());
    assert_eq!(skyrim.folders, default.folders);
    assert_eq!(skyrim.suffixes, default.suffixes);
    // And its mod root is the install root's child, so no archive of its can be
    // read as install-root relative.
    assert!(skyrim.game_dir().is_empty());
}

/// What the per-game vocabulary actually buys, measured on real archives.
///
/// Every Stellar Blade case in the corpus is an archive that was really
/// downloaded for that game. Under the Gamebryo vocabulary - the only one that
/// existed before this - not one of them resolves, because `.pak`, `.ucas` and
/// `.utoc` mean nothing to it: every single archive lands in the manual picker.
/// Under the game's own rules, most install themselves.
///
/// The ones that still do not are not failures, and the count is asserted so that
/// a change in either direction has to be looked at:
///   - two variant bundles whose options are nested archives (the user must pick)
///   - one archive that is a lone `.json`
///   - one UE4SS script mod, whose tree is install-root-relative and belongs on
///     the `Root/` surface rather than this one
#[test]
fn the_game_vocabulary_is_what_makes_unreal_archives_install() {
    let shapes =
        std::fs::read_to_string(corpus_dir().join("shapes.txt")).expect("corpus/shapes.txt");
    let cases: Vec<Case> = parse_shapes(&shapes)
        .into_iter()
        .filter(|c| c.game == "stellarblade" && c.id.starts_with("archive"))
        .collect();
    assert_eq!(cases.len(), 13, "the Stellar Blade archive corpus changed");

    // What `open_archive` calls Simple: a wrapper chain it can strip, OR a split
    // into a Data half and an install-root half. Either way the user is asked
    // nothing, which is the thing being measured.
    let installs_unattended = |rules: LayoutRules| {
        cases
            .iter()
            .filter(|c| {
                let t = ArchiveTree::from_entries(&c.entries);
                t.simple_archive_base(rules).is_some() || t.root_builder_split(rules).is_some()
            })
            .count()
    };

    assert_eq!(
        installs_unattended(LayoutRules::default()),
        0,
        "the Gamebryo vocabulary must not resolve Unreal archives; if it does, \
         the default list has grown something it should not have"
    );
    assert_eq!(
        installs_unattended(LayoutRules::for_game("stellarblade")),
        10,
        "10 of the 13 real archives install without asking the user anything: 9 by \
         stripping a wrapper, and the UE4SS one by splitting off its pak half"
    );
}
