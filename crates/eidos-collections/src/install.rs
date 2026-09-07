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
use crate::state::{member_key, InstallState, Status};

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
    Failed(String),
}

/// Everything the engine cannot do itself.
pub trait Hooks {
    /// Get this member's archive, or say why not.
    fn obtain(&mut self, m: &Mod) -> Obtained;
    /// Install it, replaying `m.choices` where the installer has questions.
    fn install(&mut self, m: &Mod, archive: &std::path::Path) -> Installed;
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
    save: &mut dyn FnMut(&InstallState),
) -> Report {
    let mut report = Report {
        collection: c.info.name.clone(),
        revision: state.revision,
        ..Report::default()
    };
    let order = install_order(c);
    let total = order.len();

    for (n, &i) in order.iter().enumerate() {
        let m = &c.mods[i];
        let key = member_key(m.source.file_id, m.source.mod_id, &m.name);
        let note = |d: &str| Note {
            subject: m.name.clone(),
            detail: d.to_string(),
        };

        // Already settled by a previous run, or by the user.
        match state.status(&key) {
            Status::Installed(_) => {
                report.installed.push(m.name.clone());
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Status::Approximate(_, why) => {
                report.approximate.push(note(&why.join("; ")));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Status::Skipped => {
                report.skipped.push(note("you skipped it"));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Status::Unavailable(why) => {
                report.needs_you.push(note(&why.clone()));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            _ => {}
        }

        // A source this build cannot fetch is not a failure to retry forever.
        if m.source.kind == SourceType::Manual {
            let why = if m.source.instructions.is_empty() {
                "the collection says to install this one by hand".to_string()
            } else {
                m.source.instructions.clone()
            };
            state.set(&key, Status::Unavailable(why.clone()));
            save(state);
            report.needs_you.push(note(&why));
            hooks.progress(n + 1, total, &m.name);
            continue;
        }

        let archive = match hooks.obtain(m) {
            Obtained::Ready(p) => {
                state.set(&key, Status::Downloaded);
                save(state);
                p
            }
            Obtained::NeedsUser(why) => {
                state.set(&key, Status::Unavailable(why.clone()));
                save(state);
                report.needs_you.push(note(&why));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Obtained::Unavailable(why) => {
                state.set(&key, Status::Unavailable(why.clone()));
                save(state);
                report.needs_you.push(note(&why));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
            Obtained::Failed(why) => {
                // Failed, not unavailable: it is worth another go, and the state
                // says so, so a re-run picks it up.
                state.set(&key, Status::Failed(why.clone()));
                save(state);
                report.failed.push(note(&why));
                hooks.progress(n + 1, total, &m.name);
                continue;
            }
        };

        match hooks.install(m, &archive) {
            Installed::Ok(folder) => {
                state.set(&key, Status::Installed(folder));
                report.installed.push(m.name.clone());
            }
            Installed::Approximate(folder, why) => {
                state.set(&key, Status::Approximate(folder, why.clone()));
                report.approximate.push(note(&why.join("; ")));
            }
            Installed::Failed(why) => {
                state.set(&key, Status::Failed(why.clone()));
                report.failed.push(note(&why));
            }
        }
        save(state);
        hooks.progress(n + 1, total, &m.name);
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

/// Which mod folder each member installed as, for the ordering pass.
pub fn installed_folders(c: &Collection, state: &InstallState) -> Vec<(usize, String)> {
    c.mods
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let key = member_key(m.source.file_id, m.source.mod_id, &m.name);
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
        fn install(&mut self, m: &Mod, _a: &std::path::Path) -> Installed {
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

    fn noop(_: &InstallState) {}

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
        let mut save = |_: &InstallState| saves += 1;
        run(&c, &mut state, &mut Fake::default(), &mut save);
        assert!(saves >= 4, "downloaded + installed for each member: {saves}");
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
    fn a_failure_is_retried_next_time_but_an_unavailable_member_is_not() {
        // Different situations, different states. Retrying a mod that has been
        // taken down forever is noise; retrying a timeout is the whole point.
        let c = collection(vec![member("gone", 0, false, 1), member("flaky", 0, false, 2)]);
        let mut f = Fake::default();
        f.obtain.insert(
            "gone".into(),
            Obtained::Unavailable("taken down".into()),
        );
        f.obtain
            .insert("flaky".into(), Obtained::Failed("timeout".into()));
        let mut state = InstallState::default();
        let r = run(&c, &mut state, &mut f, &mut noop);
        assert_eq!(r.needs_you.len(), 1);
        assert_eq!(r.failed.len(), 1);

        let mut again = Fake::default();
        run(&c, &mut state, &mut again, &mut noop);
        assert_eq!(
            again.installed_order,
            vec!["flaky"],
            "the retryable one was retried and the gone one was not"
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
        let key = member_key(Some(1), Some(10), "a");
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
