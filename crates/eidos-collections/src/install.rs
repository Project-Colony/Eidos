//! The engine: walk a collection's members in the order it asks for, and record
//! what happened to each one before moving on.
//!
//! Deliberately knows nothing about downloading or extracting. Those arrive
//! through [`Hooks`], which keeps this crate free of the network and the
//! archiver - and, more usefully, makes the whole order-and-recovery behaviour
//! testable without either. Every rule below is exercised by a fake.
//!
//! Two things it does NOT do, both on purpose:
//!
//! * It installs SERIALLY. Vortex needs two separate concurrency limiters and a
//!   phase engine to keep them from deadlocking each other; Eidos has no
//!   concurrent installer, so the correct first design is to not acquire that
//!   problem. Downloads are where the wall-clock goes, and they can be made
//!   concurrent later without touching this.
//! * It refreshes nothing as it goes. Under a union mount, "deployed" is "the
//!   mod list is saved", so there is no per-member barrier to wait on - which is
//!   where Eidos is structurally faster than a manager that hardlinks each mod
//!   into place, and it is worth not giving that away.

use std::path::PathBuf;

use crate::manifest::{Collection, Mod, SourceType};
use crate::report::{Note, Report};
use crate::state::{key_for, InstallState, Status};

/// What became of a member's archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Obtained {
    /// It is on disk, here.
    Ready(PathBuf),
    /// Only the user can get it: a free account's Nexus file, a `browse` page,
    /// a `manual` source. Carries what to tell them.
    NeedsUser(String),
    /// It cannot be had at all - taken down, or a source this build cannot use.
    Unavailable(String),
    Failed(String),
}

/// What became of an install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// Installed as the mod folder named here, exactly as asked.
    Ok(String),
    /// Installed, but the recorded installer answers could not all be replayed,
    /// so the files differ from the author's.
    Approximate(String, Vec<String>),
    /// The owned reservation and exact receipt remain available for a later answer.
    NeedsUser(String),
    Failed(String),
}

/// Everything the engine cannot do itself.
pub trait Hooks {
    /// Validate the entire recipe before any member is downloaded or reserved.
    fn validate_recipe(&mut self, c: &Collection) -> Result<(), String> {
        for m in &c.mods {
            crate::recipe::validate_member(m)?;
        }
        Ok(())
    }
    fn runtime_check(&mut self, c: &Collection) -> crate::recipe::RuntimeCheck {
        crate::recipe::compare_runtime(
            &c.info.game_versions,
            Err("No runtime evidence was provided".into()),
        )
    }
    fn allow_runtime_mismatch(&self) -> bool {
        false
    }
    /// Get this member's archive, or say why not.
    fn obtain(&mut self, m: &Mod) -> Obtained;
    /// Reserve an owned destination before the engine persists it and starts extraction.
    fn reserve(&mut self, m: &Mod, previous: Option<&str>) -> Result<String, String> {
        Ok(previous.unwrap_or(&m.name).to_string())
    }
    /// Check a completed member before trusting a resumed state file.
    fn verify_installed(&mut self, _m: &Mod, _folder: &str) -> Result<bool, String> {
        Ok(true)
    }
    /// Recover a published receipt when the final status checkpoint was interrupted.
    fn recover_installed(&mut self, _m: &Mod, _folder: &str) -> Result<Option<Installed>, String> {
        Ok(None)
    }
    /// Install it, replaying `m.choices` where the installer has questions.
    fn install(&mut self, m: &Mod, archive: &std::path::Path, folder: &str) -> Installed;
    /// One member is done. `done` counts every member reached, `total` is all of
    /// them - so a progress bar can be honest about a collection that is half
    /// skipped.
    fn progress(&mut self, _done: usize, _total: usize, _member: &str) {}
}

/// The order members are installed in.
///
/// `phase` ascending, then the collection's own order within a phase - and
/// optional members last whatever their phase says. Vortex reaches the same
/// place with a sentinel phase number; sorting the flag is the same rule
/// without a magic constant that has to be bigger than every real phase.
pub fn install_order(c: &Collection) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..c.mods.len()).collect();
    idx.sort_by_key(|&i| (c.mods[i].optional, c.mods[i].phase, i));
    idx
}

/// Install every member that still needs it, updating `state` after each one.
///
/// Resumes: a member the state already calls finished is not touched, which is
/// what makes an interrupted 200-mod collection continue rather than restart.
pub fn run(
    c: &Collection,
    state: &mut InstallState,
    hooks: &mut dyn Hooks,
    save: &mut dyn FnMut(&InstallState) -> Result<(), String>,
) -> Report {
    let mut report = Report {
        collection: c.info.name.clone(),
        revision: state.revision,
        ..Report::default()
    };
    if let Err(error) = hooks.validate_recipe(c) {
        report.aborted = true;
        report.failed.push(Note {
            subject: "collection recipe".into(),
            detail: error,
        });
        return report;
    }
    let runtime = hooks.runtime_check(c);
    match &runtime {
        crate::recipe::RuntimeCheck::Mismatch { .. } => {
            if !hooks.allow_runtime_mismatch() && state.runtime_decision.as_ref() != Some(&runtime)
            {
                report.aborted = true;
                report.needs_you.push(Note {
                    subject: "game runtime".into(),
                    detail: runtime.message(),
                });
                return report;
            }
            if state.runtime_decision.as_ref() != Some(&runtime) {
                let previous = state.runtime_decision.replace(runtime.clone());
                if !persist(state, save, &mut report, "runtime continuation") {
                    state.runtime_decision = previous;
                    return report;
                }
            }
            report.deferred.push(Note {
                subject: "game runtime".into(),
                detail: format!("{}; explicit continuation recorded", runtime.message()),
            });
        }
        crate::recipe::RuntimeCheck::Unknown { .. } => report.deferred.push(Note {
            subject: "game runtime".into(),
            detail: runtime.message(),
        }),
        _ => {}
    }
    let order = install_order(c);
    let total = order.len();

    for (n, &i) in order.iter().enumerate() {
        let m = &c.mods[i];
        let key = key_for(m, &c.info.domain_name);
        let note = |d: &str| Note {
            subject: m.name.clone(),
            detail: d.to_string(),
        };

        // A durable status is not proof that its folder still exists or is still ours.
        match state.status(&key) {
            Status::Installed(folder) | Status::Approximate(folder, _) => {
                match hooks.verify_installed(m, folder) {
                    Ok(true) => {
                        if let Status::Approximate(_, why) = state.status(&key) {
                            report.approximate.push(note(&why.join("; ")));
                        } else {
                            report.installed.push(m.name.clone());
                        }
                        hooks.progress(n + 1, total, &m.name);
                        continue;
                    }
                    Ok(false) => {
                        state.set(&key, Status::Pending);
                    }
                    Err(error) => {
                        state.set(&key, Status::Failed(error.clone()));
                        if !persist(state, save, &mut report, &m.name) {
                            return report;
                        }
                        report.failed.push(note(&error));
                        hooks.progress(n + 1, total, &m.name);
                        continue;
                    }
                }
            }
            Status::Skipped => {
                report.skipped.push(note("you skipped it"));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Status::Foreign(_) => {
                report.deferred.push(note(
                    "a newer version of Eidos recorded this one, so it was left alone",
                ));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            // NOT settled. Unavailable is what a free account's Nexus file and
            // a `browse` source are recorded as, and the whole instruction the
            // user is given is "fetch it yourself and run this again" - which
            // `obtain` answers by looking in downloads/ first. Treating it as
            // final made that instruction impossible to follow.
            _ => {}
        }

        let recovered = match state
            .folders
            .get(&key)
            .map(|folder| hooks.recover_installed(m, folder))
            .transpose()
        {
            Ok(outcome) => outcome.flatten(),
            Err(error) => Some(Installed::Failed(error)),
        };
        if let Some(outcome) = recovered {
            record_install(outcome, m, &key, state, &mut report);
            if !persist(state, save, &mut report, &m.name) {
                return report;
            }
            hooks.progress(n + 1, total, &m.name);
            if report.aborted {
                return report;
            }
            continue;
        }

        // A source this build cannot fetch is not a failure to retry forever.
        if m.source.kind == SourceType::Manual {
            let why = if m.source.instructions.is_empty() {
                "the collection says to install this one by hand".to_string()
            } else {
                m.source.instructions.clone()
            };
            state.set(&key, Status::Unavailable(why.clone()));
            if !persist(state, save, &mut report, &m.name) {
                return report;
            }
            report.needs_you.push(note(&why));
            hooks.progress(n + 1, total, &m.name);
            continue;
        }

        let archive = match hooks.obtain(m) {
            Obtained::Ready(p) => {
                state.set(&key, Status::Downloaded);
                if !persist(state, save, &mut report, &m.name) {
                    return report;
                }
                p
            }
            Obtained::NeedsUser(why) => {
                state.set(&key, Status::Unavailable(why.clone()));
                if !persist(state, save, &mut report, &m.name) {
                    return report;
                }
                report.needs_you.push(note(&why));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Obtained::Unavailable(why) => {
                state.set(&key, Status::Unavailable(why.clone()));
                if !persist(state, save, &mut report, &m.name) {
                    return report;
                }
                report.needs_you.push(note(&why));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Obtained::Failed(why) => {
                // Failed, not unavailable: it is worth another go, and the state
                // says so, so a re-run picks it up.
                state.set(&key, Status::Failed(why.clone()));
                if !persist(state, save, &mut report, &m.name) {
                    return report;
                }
                report.failed.push(note(&why));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
        };

        let folder = match hooks.reserve(m, state.folders.get(&key).map(String::as_str)) {
            Ok(folder) => folder,
            Err(why) => {
                state.set(&key, Status::Failed(why.clone()));
                report.failed.push(note(&why));
                if !persist(state, save, &mut report, &m.name) {
                    return report;
                }
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
        };
        state.folders.insert(key.clone(), folder.clone());
        if !persist(state, save, &mut report, &m.name) {
            return report;
        }

        record_install(
            hooks.install(m, &archive, &folder),
            m,
            &key,
            state,
            &mut report,
        );
        if !persist(state, save, &mut report, &m.name) {
            return report;
        }
        hooks.progress(n + 1, total, &m.name);
        if report.aborted {
            return report;
        }
    }

    for t in &c.tools {
        report.tools_expected.push(Note {
            subject: t.name.clone(),
            detail: if t.exe.is_empty() {
                "no path recorded".to_string()
            } else {
                t.exe.clone()
            },
        });
    }
    report
}

fn record_install(
    outcome: Installed,
    member: &Mod,
    key: &str,
    state: &mut InstallState,
    report: &mut Report,
) {
    match outcome {
        Installed::Ok(folder) => {
            state.set(key, Status::Installed(folder));
            report.installed.push(member.name.clone());
        }
        Installed::Approximate(folder, why) => {
            state.set(key, Status::Approximate(folder, why.clone()));
            report.approximate.push(Note {
                subject: member.name.clone(),
                detail: why.join("; "),
            });
        }
        Installed::NeedsUser(why) => {
            state.set(key, Status::Unavailable(why.clone()));
            report.aborted = true;
            report.needs_you.push(Note {
                subject: member.name.clone(),
                detail: why,
            });
        }
        Installed::Failed(why) => {
            state.set(key, Status::Failed(why.clone()));
            report.failed.push(Note {
                subject: member.name.clone(),
                detail: why,
            });
        }
    }
}

fn persist(
    state: &InstallState,
    save: &mut dyn FnMut(&InstallState) -> Result<(), String>,
    report: &mut Report,
    member: &str,
) -> bool {
    if let Err(error) = save(state) {
        report.aborted = true;
        report.failed.push(Note {
            subject: member.to_string(),
            detail: format!("Could not save collection progress; installation stopped: {error}"),
        });
        false
    } else {
        true
    }
}

/// Which mod folder each member installed as, for the ordering pass.
pub fn installed_folders(c: &Collection, state: &InstallState) -> Vec<(usize, String)> {
    c.mods
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let key = key_for(m, &c.info.domain_name);
            match state.status(&key) {
                Status::Installed(f) | Status::Approximate(f, _) => Some((i, f.clone())),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Mod, Source};

    #[derive(Default)]
    struct Fake {
        /// member name -> what obtaining it does
        obtain: std::collections::HashMap<String, Obtained>,
        install: std::collections::HashMap<String, Installed>,
        installed_order: Vec<String>,
        progress: Vec<(usize, usize)>,
    }

    impl Hooks for Fake {
        fn obtain(&mut self, m: &Mod) -> Obtained {
            self.obtain
                .get(&m.name)
                .cloned()
                .unwrap_or_else(|| Obtained::Ready(PathBuf::from(format!("/dl/{}.7z", m.name))))
        }
        fn install(&mut self, m: &Mod, _a: &std::path::Path, _folder: &str) -> Installed {
            self.installed_order.push(m.name.clone());
            self.install
                .get(&m.name)
                .cloned()
                .unwrap_or_else(|| Installed::Ok(m.name.clone()))
        }
        fn progress(&mut self, done: usize, total: usize, _m: &str) {
            self.progress.push((done, total));
        }
    }

    fn member(name: &str, phase: u32, optional: bool, file_id: u64) -> Mod {
        Mod {
            name: name.to_string(),
            phase,
            optional,
            source: Source {
                file_id: Some(file_id),
                mod_id: Some(file_id * 10),
                ..Source::default()
            },
            ..Mod::default()
        }
    }

    fn collection(mods: Vec<Mod>) -> Collection {
        Collection {
            mods,
            ..Collection::default()
        }
    }

    fn noop(_: &InstallState) -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn unanswered_installer_stops_then_resumes_same_reserved_member() {
        let c = collection(vec![
            member("First", 0, false, 1),
            member("Later", 0, false, 2),
        ]);
        let mut state = InstallState::default();
        let mut fake = Fake::default();
        fake.install.insert(
            "First".into(),
            Installed::NeedsUser("exact prompt saved".into()),
        );
        let mut saved = None;
        let report = run(&c, &mut state, &mut fake, &mut |s| {
            saved = Some(s.clone());
            Ok(())
        });
        assert!(report.aborted);
        assert_eq!(report.needs_you.len(), 1);
        assert_eq!(fake.installed_order, ["First"]);
        let mut restored = saved.unwrap();
        assert_eq!(
            restored
                .folders
                .get(&key_for(&c.mods[0], ""))
                .map(String::as_str),
            Some("First")
        );
        assert!(restored.status(&key_for(&c.mods[0], "")).is_open());
        fake.install.clear();
        fake.installed_order.clear();
        let report = run(&c, &mut restored, &mut fake, &mut noop);
        assert!(!report.aborted);
        assert_eq!(fake.installed_order, ["First", "Later"]);
        assert_eq!(
            restored
                .folders
                .get(&key_for(&c.mods[0], ""))
                .map(String::as_str),
            Some("First")
        );
    }

    #[test]
    fn members_install_by_phase_then_by_the_order_the_collection_listed_them() {
        let c = collection(vec![
            member("late", 2, false, 1),
            member("early", 0, false, 2),
            member("also early", 0, false, 3),
            member("middle", 1, false, 4),
        ]);
        assert_eq!(install_order(&c), vec![1, 2, 3, 0]);
    }

    #[test]
    fn optional_members_go_last_whatever_phase_they_claim() {
        let c = collection(vec![
            member("optional but early", 0, true, 1),
            member("required and late", 9, false, 2),
        ]);
        assert_eq!(install_order(&c), vec![1, 0]);
    }

    #[test]
    fn the_state_is_written_after_every_member_not_at_the_end() {
        // The whole point of the file: an interrupted 200-mod install has to
        // know where it was, and "at the end" is exactly when it never runs.
        let c = collection(vec![member("a", 0, false, 1), member("b", 0, false, 2)]);
        let mut state = InstallState::default();
        let mut saves = 0;
        let mut save = |_: &InstallState| {
            saves += 1;
            Ok(())
        };
        run(&c, &mut state, &mut Fake::default(), &mut save);
        assert!(
            saves >= 4,
            "downloaded + installed for each member: {saves}"
        );
    }

    #[test]
    fn extraction_cannot_start_before_its_reservation_is_durable() {
        let c = collection(vec![member("a", 0, false, 1)]);
        let mut state = InstallState::default();
        let mut hooks = Fake::default();
        let mut save = |s: &InstallState| {
            if s.folders.is_empty() {
                Ok(())
            } else {
                Err("disk full".to_string())
            }
        };
        let report = run(&c, &mut state, &mut hooks, &mut save);
        assert!(report.aborted);
        assert!(hooks.installed_order.is_empty());
        assert!(report.failed[0].detail.contains("disk full"));
    }

    #[test]
    fn a_finished_member_is_not_installed_again_on_a_re_run() {
        let c = collection(vec![member("a", 0, false, 1), member("b", 0, false, 2)]);
        let mut state = InstallState::default();
        let mut f = Fake::default();
        run(&c, &mut state, &mut f, &mut noop);
        assert_eq!(f.installed_order, vec!["a", "b"]);

        let mut again = Fake::default();
        let r = run(&c, &mut state, &mut again, &mut noop);
        assert!(again.installed_order.is_empty(), "nothing was redone");
        assert_eq!(r.installed.len(), 2, "and both are still reported");
    }

    #[test]
    fn a_member_only_the_user_can_fetch_is_installed_once_they_have() {
        // The whole instruction printed for the free-account wall and for a
        // `browse` source is "fetch it yourself and run this again". A status
        // that skips the member forever makes that instruction impossible to
        // follow, and the collection can never finish.
        let c = collection(vec![
            member("gated", 0, false, 1),
            member("flaky", 0, false, 2),
        ]);
        let mut f = Fake::default();
        f.obtain.insert(
            "gated".into(),
            Obtained::NeedsUser("a free account cannot fetch this".into()),
        );
        f.obtain
            .insert("flaky".into(), Obtained::Failed("timeout".into()));
        let mut state = InstallState::default();
        let r = run(&c, &mut state, &mut f, &mut noop);
        assert_eq!(r.needs_you.len(), 1);
        assert_eq!(r.failed.len(), 1);
        assert!(f.installed_order.is_empty());

        // The user fetched it. The next run finds it in downloads/.
        let mut again = Fake::default();
        let r = run(&c, &mut state, &mut again, &mut noop);
        assert_eq!(again.installed_order, vec!["gated", "flaky"]);
        assert!(r.needs_you.is_empty() && r.failed.is_empty());
        assert_eq!(r.installed.len(), 2);
    }

    #[test]
    fn a_member_that_is_still_unavailable_is_still_reported_as_such() {
        let c = collection(vec![member("bundled", 0, false, 1)]);
        let mut f = Fake::default();
        f.obtain.insert(
            "bundled".into(),
            Obtained::Unavailable("no usable source".into()),
        );
        let mut state = InstallState::default();
        for _ in 0..2 {
            let r = run(&c, &mut state, &mut f, &mut noop);
            assert_eq!(r.needs_you.len(), 1);
            assert!(!r.is_complete());
        }
        assert!(f.installed_order.is_empty());
    }

    #[test]
    fn completed_recipe_hooks_are_not_reported_as_deferred() {
        let mut m = member("A Patch", 0, false, 1);
        m.patches.insert("meshes/x.nif".into(), "DEADBEEF".into());
        m.file_overrides.push("textures/win.dds".into());
        let c = collection(vec![m]);
        let r = run(
            &c,
            &mut InstallState::default(),
            &mut Fake::default(),
            &mut noop,
        );
        assert_eq!(r.installed, vec!["A Patch".to_string()]);
        assert!(
            r.deferred.is_empty(),
            "completed recipe hooks must not be labelled deferred"
        );
    }

    #[test]
    fn a_manual_source_is_never_attempted_and_carries_its_instructions() {
        let mut m = member("by hand", 0, false, 1);
        m.source.kind = SourceType::Manual;
        m.source.instructions = "Get it from the author's Discord.".into();
        let c = collection(vec![m]);
        let mut f = Fake::default();
        let r = run(&c, &mut InstallState::default(), &mut f, &mut noop);
        assert!(f.installed_order.is_empty());
        assert_eq!(r.needs_you[0].detail, "Get it from the author's Discord.");
    }

    #[test]
    fn a_member_whose_answers_could_not_be_replayed_is_reported_as_such() {
        let c = collection(vec![member("patch", 0, false, 1)]);
        let mut f = Fake::default();
        f.install.insert(
            "patch".into(),
            Installed::Approximate("Patch".into(), vec!["\"8K\" is no longer an option".into()]),
        );
        let mut state = InstallState::default();
        let r = run(&c, &mut state, &mut f, &mut noop);
        assert!(r.installed.is_empty(), "it is not a plain success");
        assert_eq!(r.approximate.len(), 1);
        assert!(!r.is_faithful());
        // And it still counts as installed for the ordering pass, because the
        // files ARE there and they still have to be ordered.
        assert_eq!(
            installed_folders(&c, &state),
            vec![(0usize, "Patch".to_string())]
        );
    }

    #[test]
    fn a_member_the_user_skipped_stays_skipped_across_runs() {
        let c = collection(vec![member("a", 0, false, 1)]);
        let mut state = InstallState::default();
        let key = key_for(&c.mods[0], &c.info.domain_name);
        state.set_by_user(&key, Status::Skipped);
        let mut f = Fake::default();
        let r = run(&c, &mut state, &mut f, &mut noop);
        assert!(f.installed_order.is_empty());
        assert_eq!(r.skipped.len(), 1);
        assert_eq!(*state.status(&key), Status::Skipped);
    }

    #[test]
    fn progress_counts_every_member_reached_including_the_ones_that_did_nothing() {
        // A bar that only advances on success stalls on a collection that is
        // half skipped, which reads as a hang.
        let mut skipped = member("skip me", 0, false, 2);
        skipped.source.kind = SourceType::Manual;
        let c = collection(vec![member("a", 0, false, 1), skipped]);
        let mut f = Fake::default();
        run(&c, &mut InstallState::default(), &mut f, &mut noop);
        assert_eq!(f.progress, vec![(1, 2), (2, 2)]);
    }
}
