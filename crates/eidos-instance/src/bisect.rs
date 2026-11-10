//! Which mods a variant test run (A36) or a bisect step (A37) needs, and the
//! bisect search itself, kept in `<instance>/bisect.json` between steps.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::PathBuf;

use eidos_plugins::Plugin;
use serde::{Deserialize, Serialize};

use crate::{write_atomic, Instance};

/// `chosen` plus, until nothing changes, the mod providing each master of a
/// plugin from a mod already in the set (plugin names compared without case),
/// all limited to `universe` and returned in `universe` order. A master no mod
/// of `universe` provides (the game's own, the Overwrite's) adds nothing.
pub fn with_masters(chosen: &[String], universe: &[String], plugins: &[Plugin]) -> Vec<String> {
    let in_universe = |m: &str| universe.iter().any(|u| u == m);
    let provider: HashMap<String, &str> = plugins
        .iter()
        .map(|p| (p.name.to_ascii_lowercase(), p.origin_mod.as_str()))
        .collect();
    let mut set: HashSet<&str> = chosen
        .iter()
        .map(String::as_str)
        .filter(|m| in_universe(m))
        .collect();
    loop {
        let added: Vec<&str> = plugins
            .iter()
            .filter(|p| set.contains(p.origin_mod.as_str()))
            .flat_map(|p| &p.masters)
            .filter_map(|m| provider.get(&m.to_ascii_lowercase()).copied())
            .filter(|m| !set.contains(m) && in_universe(m))
            .collect();
        if added.is_empty() {
            break;
        }
        set.extend(added);
    }
    universe
        .iter()
        .filter(|u| set.contains(u.as_str()))
        .cloned()
        .collect()
}

/// The user's answer to one bisect step: did the problem happen?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Answer {
    Good,
    Bad,
    Unsure,
}

/// One step: the candidates its test run turned on, and the answer once given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BisectStep {
    pub tested: Vec<String>,
    pub answer: Option<Answer>,
}

/// A bisect search (A37), as `bisect.json` holds it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bisect {
    pub profile: String,
    /// On in every step and never suspected.
    pub pinned: Vec<String>,
    /// The mods that may still cause the problem.
    pub candidates: Vec<String>,
    /// How many Unsure answers in a row: each one rotates the split.
    pub attempt: u32,
    pub steps: Vec<BisectStep>,
    /// Unix seconds.
    pub started: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BisectResult {
    /// `confirmed`: a run with these mods on showed the problem. Otherwise they
    /// are what was left after every other mod was cleared.
    Found { mods: Vec<String>, confirmed: bool },
    /// Every suspect was cleared: the problem is not in a suspect mod.
    NoneLeft,
}

impl Bisect {
    pub fn new(profile: &str, pinned: Vec<String>, suspects: Vec<String>, now: u64) -> Bisect {
        let candidates = suspects
            .into_iter()
            .filter(|m| !pinned.contains(m))
            .collect();
        Bisect {
            profile: profile.into(),
            pinned,
            candidates,
            started: now,
            ..Default::default()
        }
    }

    /// Split `candidates` in two (rotated left by `attempt * max(1, n / 3)`,
    /// cut at `n / 2` rounded up) and give the closure under masters of the
    /// first half, then the second, that tests a candidate and leaves one out.
    /// The pinned mods go into the closure, so a master one of them needs is on
    /// (and tested) in every step.
    fn usable_closure(&self, universe: &[String], plugins: &[Plugin]) -> Option<Vec<String>> {
        let n = self.candidates.len();
        if n < 2 {
            return None;
        }
        let mut rotated = self.candidates.clone();
        rotated.rotate_left((self.attempt as usize % n) * (n / 3).max(1) % n);
        let (first, second) = rotated.split_at(n.div_ceil(2));
        [first, second].iter().find_map(|half| {
            let chosen: Vec<String> = self.pinned.iter().chain(*half).cloned().collect();
            let closure = with_masters(&chosen, universe, plugins);
            let tested = closure
                .iter()
                .filter(|m| self.candidates.contains(m))
                .count();
            (tested > 0 && tested < n).then_some(closure)
        })
    }

    /// `(mods to enable, tested candidates)` for the next step, in `universe`
    /// order; `None` once [`Bisect::result`] has one.
    pub fn next_run(
        &self,
        universe: &[String],
        plugins: &[Plugin],
    ) -> Option<(Vec<String>, Vec<String>)> {
        let closure = self.usable_closure(universe, plugins)?;
        let tested = closure
            .iter()
            .filter(|m| self.candidates.contains(m))
            .cloned()
            .collect();
        Some((closure, tested))
    }

    pub fn begin_step(&mut self, tested: Vec<String>) {
        self.steps.push(BisectStep {
            tested,
            answer: None,
        });
    }

    /// Answer the last step; a step already answered keeps its first answer.
    pub fn answer(&mut self, a: Answer) {
        let Some(step) = self.steps.last_mut().filter(|s| s.answer.is_none()) else {
            return;
        };
        step.answer = Some(a);
        match a {
            Answer::Good => self.candidates.retain(|c| !step.tested.contains(c)),
            Answer::Bad => self.candidates.retain(|c| step.tested.contains(c)),
            Answer::Unsure => {
                self.attempt = self.attempt.saturating_add(1);
                return;
            }
        }
        self.attempt = 0;
    }

    pub fn result(&self, universe: &[String], plugins: &[Plugin]) -> Option<BisectResult> {
        if self.candidates.is_empty() {
            return Some(BisectResult::NoneLeft);
        }
        if self.usable_closure(universe, plugins).is_some() {
            return None;
        }
        let confirmed = self.steps.iter().any(|s| {
            s.answer == Some(Answer::Bad) && self.candidates.iter().all(|c| s.tested.contains(c))
        });
        Some(BisectResult::Found {
            mods: self.candidates.clone(),
            confirmed,
        })
    }

    /// About how many steps are left: `ceil(log2(candidates))`.
    pub fn runs_left(&self) -> u32 {
        self.candidates.len().next_power_of_two().trailing_zeros()
    }
}

impl Instance {
    pub fn bisect_path(&self) -> PathBuf {
        self.root.join("bisect.json")
    }

    /// The search on disk; `None` when there is none or it cannot be read.
    pub fn read_bisect(&self) -> Option<Bisect> {
        serde_json::from_slice(&fs::read(self.bisect_path()).ok()?).ok()
    }

    pub fn write_bisect(&self, b: &Bisect) -> io::Result<()> {
        let body = serde_json::to_vec_pretty(b).map_err(io::Error::other)?;
        write_atomic(&self.bisect_path(), &body)
    }

    /// Keep `log` as `logs/bisect-<now>.txt`, then drop the search.
    pub fn end_bisect(&self, log: &str, now: u64) -> io::Result<()> {
        let logs = self.root.join("logs");
        fs::create_dir_all(&logs)?;
        write_atomic(&logs.join(format!("bisect-{now}.txt")), log.as_bytes())?;
        match fs::remove_file(self.bisect_path()) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eidos_plugins::Plugin;
    use std::fs;
    use std::path::PathBuf;

    /// The fields `with_masters` does not read, filled as eidos-gui's
    /// `test_support::plugin_row` fills them.
    fn plugin(name: &str, origin: &str, masters: &[&str]) -> eidos_plugins::Plugin {
        eidos_plugins::Plugin {
            name: name.into(),
            origin_mod: origin.into(),
            masters: masters.iter().map(|m| m.to_string()).collect(),
            path: PathBuf::new(),
            enabled: true,
            force_disabled: false,
            header_error: None,
            form_version: Some(44),
            author: None,
            description: None,
            header_version: None,
            is_blueprint: false,
            is_update: false,
            is_master: false,
            is_light: false,
            is_medium: false,
            is_dummy: false,
            priority: 0,
            index: None,
            ini: None,
        }
    }

    #[test]
    fn with_masters_adds_providers_in_profile_order() {
        let plugins = [
            plugin("Base.esm", "Base", &[]),
            plugin("Mid.esp", "Mid", &["base.ESM"]),
            plugin("Patch.esp", "Patch", &["Mid.esp", "Skyrim.esm"]),
            plugin("Other.esp", "Other", &[]),
        ];
        let universe: Vec<String> = ["Base", "Other", "Mid", "Patch"].map(String::from).to_vec();
        assert_eq!(
            with_masters(&["Patch".into()], &universe, &plugins),
            ["Base", "Mid", "Patch"]
        );
        assert_eq!(with_masters(&[], &universe, &plugins), Vec::<String>::new());
    }

    fn chain(n: usize) -> (Vec<String>, Vec<Plugin>) {
        let mods: Vec<String> = (0..n).map(|i| format!("M{i:03}")).collect();
        let plugins = mods
            .iter()
            .map(|m| plugin(&format!("{m}.esp"), m, &[]))
            .collect();
        (mods, plugins)
    }

    fn inst(name: &str) -> Instance {
        let root = std::env::temp_dir().join(format!("eidos-bisect-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Instance::portable(root)
    }

    #[test]
    fn two_hundred_suspects_take_eight_runs() {
        let (mods, plugins) = chain(200);
        // M199 sits in the untested half at every step: every answer is Good, no run ever
        // showed the problem, so the Rules leave it unconfirmed (found by elimination only).
        for (culprit, confirmed) in [("M000", true), ("M117", true), ("M199", false)] {
            let mut b = Bisect::new("Default", vec![], mods.clone(), 0);
            assert_eq!(b.runs_left(), 8);
            let mut runs = 0;
            while let Some((_, tested)) = b.next_run(&mods, &plugins) {
                runs += 1;
                let bad = tested.iter().any(|m| m == culprit);
                b.begin_step(tested);
                b.answer(if bad { Answer::Bad } else { Answer::Good });
            }
            assert!(runs <= 8, "{culprit}: {runs} runs");
            assert_eq!(
                b.result(&mods, &plugins),
                Some(BisectResult::Found {
                    mods: vec![culprit.into()],
                    confirmed
                }),
                "{culprit}"
            );
            assert_eq!(b.runs_left(), 0);
        }
    }

    #[test]
    fn unsure_resplits_and_pinned_mods_are_always_on_but_never_tested() {
        let (mods, plugins) = chain(6);
        let mut b = Bisect::new("Default", vec!["M005".into()], mods.clone(), 0);
        let (on, first) = b.next_run(&mods, &plugins).unwrap();
        assert!(on.contains(&"M005".to_string()) && !first.contains(&"M005".to_string()));
        b.begin_step(first.clone());
        b.answer(Answer::Unsure);
        let (_, second) = b.next_run(&mods, &plugins).unwrap();
        assert_ne!(second, first);
        assert_eq!(b.candidates.len(), 5);
    }

    #[test]
    fn a_split_whose_masters_cover_every_candidate_takes_the_other_half_or_ends() {
        let universe: Vec<String> = ["Patch", "Base"].map(String::from).to_vec();
        let plugins = [
            plugin("Patch.esp", "Patch", &["Base.esm"]),
            plugin("Base.esm", "Base", &[]),
        ];
        let mut b = Bisect::new("Default", vec![], universe.clone(), 0);
        let (_, tested) = b.next_run(&universe, &plugins).unwrap();
        assert_eq!(tested, ["Base"], "Patch alone would pull Base in");
        b.begin_step(tested);
        b.answer(Answer::Good);
        assert_eq!(
            b.result(&universe, &plugins),
            Some(BisectResult::Found {
                mods: vec!["Patch".into()],
                confirmed: false
            })
        );
        // broken data with a master cycle: neither half can run without the other, so the search ends
        let pair: Vec<String> = ["P", "Q"].map(String::from).to_vec();
        let cycle = [
            plugin("P.esp", "P", &["Q.esp"]),
            plugin("Q.esp", "Q", &["P.esp"]),
        ];
        let stuck = Bisect::new("Default", vec![], pair.clone(), 0);
        assert_eq!(stuck.next_run(&pair, &cycle), None);
        assert_eq!(
            stuck.result(&pair, &cycle),
            Some(BisectResult::Found {
                mods: pair.clone(),
                confirmed: false
            })
        );
    }

    #[test]
    fn a_pinned_mods_masters_stay_on_in_every_step() {
        // Pinned "Patch" needs "Base", a suspect: a step without Base would fail on
        // a missing master and read as Bad, so Base is on (and tested) in each step.
        let universe: Vec<String> = ["Base", "A", "B", "Patch"].map(String::from).to_vec();
        let plugins = [
            plugin("Base.esm", "Base", &[]),
            plugin("A.esp", "A", &[]),
            plugin("B.esp", "B", &[]),
            plugin("Patch.esp", "Patch", &["Base.esm"]),
        ];
        let mut b = Bisect::new("Default", vec!["Patch".into()], universe.clone(), 0);
        assert_eq!(b.candidates, ["Base", "A", "B"]);
        let (on, tested) = b.next_run(&universe, &plugins).unwrap();
        assert_eq!(on, ["Base", "A", "Patch"]);
        assert_eq!(tested, ["Base", "A"]);
        b.begin_step(tested);
        b.answer(Answer::Bad);
        let (on, tested) = b.next_run(&universe, &plugins).unwrap();
        assert!(on.contains(&"Base".to_string()), "{on:?}");
        assert_eq!(tested, ["Base"]);
    }

    #[test]
    fn an_answer_counts_once_and_needs_a_step() {
        let mut b = Bisect::new(
            "Default",
            vec![],
            ["A", "B", "C"].map(String::from).to_vec(),
            0,
        );
        b.answer(Answer::Bad);
        assert_eq!(b.candidates.len(), 3, "no step to answer");
        b.begin_step(vec!["A".into()]);
        b.answer(Answer::Good);
        b.answer(Answer::Bad);
        assert_eq!(b.candidates, ["B", "C"]);
        assert_eq!(b.steps.last().unwrap().answer, Some(Answer::Good));
    }

    #[test]
    fn bisect_state_round_trips_and_ends_in_a_log() {
        let inst = inst("state");
        let b = Bisect::new(
            "Default",
            vec!["SKSE".into()],
            vec!["A".into(), "B".into()],
            7,
        );
        inst.write_bisect(&b).unwrap();
        assert_eq!(inst.read_bisect(), Some(b));
        inst.end_bisect("Found: A", 9).unwrap();
        assert!(inst.read_bisect().is_none());
        assert_eq!(
            fs::read_to_string(inst.root.join("logs/bisect-9.txt")).unwrap(),
            "Found: A"
        );
        let _ = fs::remove_dir_all(&inst.root);
    }

    #[test]
    fn the_extender_mod_is_found_by_its_root_file() {
        let inst = inst("rootfile");
        let profile = inst.root.join("profiles/Default");
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("modlist.txt"), "+High\n+Low\n").unwrap();
        for (m, root, file) in [
            ("Low", "root", "SKSE64_Loader.exe"),
            ("High", "Root", "skse64_loader.exe"),
        ] {
            let dir = inst.mods_dir().join(m).join(root);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(file), b"x").unwrap();
        }
        assert_eq!(
            inst.active()
                .mod_with_root_file("skse64_loader.exe")
                .as_deref(),
            Some("High")
        );
        assert_eq!(
            inst.active()
                .mod_with_root_file("SKSE64_LOADER.EXE")
                .as_deref(),
            Some("High")
        );
        assert_eq!(inst.active().mod_with_root_file("f4se_loader.exe"), None);
        let _ = fs::remove_dir_all(&inst.root);
    }
}
