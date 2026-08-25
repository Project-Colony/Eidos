//! Whether EIDOS'S OWN machinery is working - as opposed to whether the user's
//! mods are, which is what the rest of the Diagnostics tab already answers.
//!
//! The distinction is the whole point. Every check in `modinfo::diagnostics`
//! watches the setup: a mod list that drifted from the folder, an archive with
//! no mod, a missing script extender. None of them would have lit up for the
//! three defects that cost the most time in August, because none of those were
//! problems with the setup - they were Eidos operations that did nothing while
//! reporting success:
//!
//!   - the prefix registry was never written, so xEdit opened on a path that
//!     did not exist and said "There are no modules in the data folder";
//!   - the prefix's `S:` drive was pointed one directory BELOW where Steam puts
//!     it, so `S:\steamapps\common\<game>` - the way a Windows program looks for
//!     a Steam game - stopped existing, and BodySlide created it and wrote 267 MB
//!     of meshes outside the union mount where nothing ever captured them;
//!   - a Skyrim instance had no Proton prefix at all, and nothing said so.
//!
//! Each check here answers its question by calling the SAME function as the
//! code it watches - `registry_status` shares its predicate with
//! `ensure_registry`, and the expected game drive comes from the very function
//! whose value is handed to Proton. That is deliberate. A health check that
//! re-derived the answer would be a second source of truth, and the two would
//! drift until one reported healthy what the other was busy repairing. Roughly
//! a sixth of this project's fixes have been one mistake made in several
//! places; this module is not allowed to become another.
//!
//! Cost: three `stat`s, one `readlink` and one read of `system.reg` per refresh
//! of a tab the user has to open. No timer, no polling, nothing walking the mod
//! pool - see the test at the bottom that says so.

use std::path::PathBuf;

use eidos_gamefeatures::RegistryStatus;
use eidos_games::DetectedGame;

use crate::modinfo::{DiagLevel, Diagnostic};

/// Everything the prefix checks need, read once so the checks themselves stay
/// pure and testable. Gathering is I/O; deciding is not.
pub(crate) struct PrefixFacts {
    /// The game's display name, for messages a user has to act on.
    pub(crate) game_name: String,
    pub(crate) registry: RegistryStatus,
    /// Where `dosdevices/s:` points, if it exists.
    pub(crate) gamedrive_found: Option<PathBuf>,
    /// Where Steam points it: the library ROOT, the directory that HOLDS
    /// `steamapps`. Taken from `library_path`, the same function whose value is
    /// handed to Proton, so this can never disagree with what Eidos passes.
    pub(crate) gamedrive_want: Option<PathBuf>,
}

/// Read the prefix's health off disk. `None` when the game has no prefix path at
/// all, which is not a fault worth reporting - the game may simply never have
/// been set up to run under Proton.
pub(crate) fn prefix_facts(game: &DetectedGame) -> Option<PrefixFacts> {
    let compat = game.compatdata.as_ref()?;
    Some(PrefixFacts {
        game_name: game.def.name.to_string(),
        registry: eidos_gamefeatures::registry_status(
            compat,
            &game.install_path,
            game.def.registry_name,
        ),
        gamedrive_found: std::fs::read_link(compat.join("pfx/dosdevices/s:")).ok(),
        gamedrive_want: eidos_games::library_path(&game.install_path),
    })
}

/// Turn the facts into cards. Pure: no I/O, no `App`, so the interesting cases
/// can be written down as tests instead of reproduced by hand on a real prefix.
pub(crate) fn prefix_checks(f: &PrefixFacts) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let game = &f.game_name;

    match &f.registry {
        RegistryStatus::NotApplicable | RegistryStatus::Correct => {}
        RegistryStatus::PrefixUninitialised => out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: format!("{game} has no Proton prefix yet"),
            detail: format!(
                "Steam has never created a Wine prefix for {game}, so there is nowhere to register \
                 the game path and nowhere for a tool to run. Launch {game} once through Steam, \
                 then press F5. If Steam refuses to create it, check that the game's folder under \
                 steamapps/compatdata is writable - an empty one owned by root will block it \
                 silently."
            ),
            actions: Vec::new(),
        }),
        RegistryStatus::Stale { found, want } => out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: format!("Windows tools cannot find {game}"),
            detail: match found {
                // Showing the wrong value is the point: it is the exact path the
                // tool is about to complain about, and the only thing that lets
                // someone connect that complaint to Eidos.
                Some(bad) => format!(
                    "The prefix says the game is installed at  {bad}  which is not where it is. \
                     xEdit, DynDOLOD, Wrye Bash and the Creation Kit all read that key, so they \
                     will open on an empty folder. Eidos repairs this the next time you run a \
                     tool; it should become  {want}"
                ),
                None => format!(
                    "The prefix has no record of where {game} is installed, so xEdit, DynDOLOD, \
                     Wrye Bash and the Creation Kit have nothing to read and will open on an empty \
                     folder. Eidos writes it the next time you run a tool: {want}"
                ),
            },
            actions: Vec::new(),
        }),
    }

    // The game drive. Steam points `S:` at the library ROOT - the directory that
    // holds `steamapps` - because Windows programs find a Steam game by trying
    // `<drive>\steamapps\common\<game>`, and the root is what makes that
    // heuristic land. Proton recreates the symlink from whatever it is handed on
    // EVERY run, so a wrong value here is not cosmetic: the path a tool looks for
    // simply stops existing, and a tool that WRITES there creates it somewhere
    // outside the mount instead of failing.
    if let (Some(found), Some(want)) = (&f.gamedrive_found, &f.gamedrive_want) {
        if found != want {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: "The prefix's S: drive is not where Steam puts it".to_string(),
                detail: format!(
                    "S: points at  {}  and Steam points it at  {}. Anything that stored an \
                     S:-relative path in this prefix - the game's own launcher writes one - now \
                     resolves somewhere else. Launching {game} or any tool through Eidos rewrites \
                     it correctly.",
                    found.display(),
                    want.display()
                ),
                actions: Vec::new(),
            });
        }
    }

    out
}

/// Whether Eidos can reach Nexus on the user's behalf. Not a card about the
/// account - the status bar already names it - but about the CAPABILITY, because
/// signed out is the state in which "Mod Manager Download" silently does nothing
/// and no update is ever reported. That combination reads as two separate broken
/// features rather than one missing sign-in.
pub(crate) struct NexusFacts {
    pub(crate) signed_in: bool,
    /// The last error the Nexus client reported, if it is still standing.
    pub(crate) last_error: Option<String>,
}

pub(crate) fn nexus_checks(f: &NexusFacts) -> Vec<Diagnostic> {
    if f.signed_in && f.last_error.is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if !f.signed_in {
        out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: "Not signed in to Nexus".to_string(),
            detail: "Mod Manager Download links will not reach Eidos and no mod updates will be \
                     reported, both silently. Sign in from the Nexus button in the toolbar."
                .to_string(),
            actions: Vec::new(),
        });
    }
    if let Some(e) = &f.last_error {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: "The last Nexus request failed".to_string(),
            detail: format!("{e}  Downloads and update checks will keep failing until this clears."),
            actions: Vec::new(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(registry: RegistryStatus) -> PrefixFacts {
        PrefixFacts {
            game_name: "Fallout 4".to_string(),
            registry,
            gamedrive_found: Some(PathBuf::from("/mnt/Jeux/SteamLibrary/steamapps")),
            gamedrive_want: Some(PathBuf::from("/mnt/Jeux/SteamLibrary/steamapps")),
        }
    }

    #[test]
    fn a_healthy_prefix_says_nothing() {
        assert!(prefix_checks(&facts(RegistryStatus::Correct)).is_empty());
        // A game that registers no path at all (Enderal, Stellar Blade) is not a
        // fault, and must not produce a card that says it is.
        assert!(prefix_checks(&facts(RegistryStatus::NotApplicable)).is_empty());
    }

    #[test]
    fn a_missing_prefix_is_reported_with_the_thing_to_do() {
        let d = prefix_checks(&facts(RegistryStatus::PrefixUninitialised));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].level, DiagLevel::Problem);
        assert!(d[0].title.contains("Fallout 4"));
        // The remedy has to be IN the card. This exact situation cost a morning:
        // an empty root-owned compatdata directory that Steam could not write to.
        assert!(d[0].detail.contains("through Steam"));
        assert!(d[0].detail.contains("owned by root"));
    }

    #[test]
    fn a_stale_key_shows_the_wrong_value_it_found() {
        // The real one, from 2026-08-25: the value the launcher wrote through a
        // drive letter that later moved.
        let d = prefix_checks(&facts(RegistryStatus::Stale {
            found: Some(r"S:\common\Fallout 4\".to_string()),
            want: r"Z:\mnt\Jeux\SteamLibrary\steamapps\common\Fallout 4\".to_string(),
        }));
        assert_eq!(d.len(), 1);
        // Without the bad value printed, the card cannot be connected to the
        // "There are no modules in the data folder" the user is staring at.
        assert!(d[0].detail.contains(r"S:\common\Fallout 4\"), "the wrong path must be shown");
        assert!(d[0].detail.contains("xEdit"), "name the tools it breaks");
    }

    #[test]
    fn a_missing_key_reads_differently_from_a_wrong_one() {
        let d = prefix_checks(&facts(RegistryStatus::Stale {
            found: None,
            want: r"Z:\games\Fallout 4\".to_string(),
        }));
        assert_eq!(d.len(), 1);
        assert!(d[0].detail.contains("no record"));
    }

    #[test]
    fn a_moved_game_drive_is_reported_with_both_paths() {
        let mut f = facts(RegistryStatus::Correct);
        // One directory too high - exactly what Eidos itself used to do.
        f.gamedrive_found = Some(PathBuf::from("/mnt/Jeux/SteamLibrary"));
        let d = prefix_checks(&f);
        assert_eq!(d.len(), 1);
        assert!(d[0].detail.contains("/mnt/Jeux/SteamLibrary/steamapps"));
        assert!(d[0].title.contains("S: drive"));
    }

    #[test]
    fn an_absent_game_drive_is_not_an_error() {
        // Proton creates it on first run. Before that there is nothing to judge,
        // and a card saying so would fire on every freshly made prefix.
        let mut f = facts(RegistryStatus::Correct);
        f.gamedrive_found = None;
        assert!(prefix_checks(&f).is_empty());
    }

    #[test]
    fn two_faults_produce_two_cards() {
        let mut f = facts(RegistryStatus::Stale {
            found: Some(r"S:\common\Fallout 4\".to_string()),
            want: r"Z:\real\Fallout 4\".to_string(),
        });
        f.gamedrive_found = Some(PathBuf::from("/mnt/Jeux/SteamLibrary"));
        assert_eq!(prefix_checks(&f).len(), 2, "one card per fault, not one card for both");
    }

    #[test]
    fn a_working_nexus_says_nothing() {
        assert!(nexus_checks(&NexusFacts { signed_in: true, last_error: None }).is_empty());
    }

    #[test]
    fn signed_out_names_both_things_it_breaks() {
        let d = nexus_checks(&NexusFacts { signed_in: false, last_error: None });
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].level, DiagLevel::Advice);
        // Both symptoms, because they are reported as two separate bugs when
        // they are one missing sign-in.
        assert!(d[0].detail.contains("Mod Manager Download"));
        assert!(d[0].detail.contains("updates"));
    }

    #[test]
    fn a_standing_error_outranks_being_signed_in() {
        let d = nexus_checks(&NexusFacts {
            signed_in: true,
            last_error: Some("429 Too Many Requests.".to_string()),
        });
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].level, DiagLevel::Problem);
        assert!(d[0].detail.contains("429"));
    }

    #[test]
    fn signed_out_and_failing_is_two_cards() {
        let d = nexus_checks(&NexusFacts {
            signed_in: false,
            last_error: Some("connection refused".to_string()),
        });
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn the_checks_do_no_io() {
        // The property that keeps this cheap enough to run on every refresh of
        // the tab. `prefix_checks` takes facts and returns cards; if someone ever
        // reaches for the filesystem inside it, this file will need `std::fs`
        // imported and this assertion is the note explaining why it must not be.
        let src = include_str!("health.rs");
        let body = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(
            !body.contains("std::fs::") || body.matches("std::fs::").count() == 1,
            "filesystem access belongs in prefix_facts, not in the checks"
        );
        // And nothing here may walk a directory tree: that is the one thing that
        // would make the Diagnostics tab expensive on a large mod pool.
        for forbidden in ["read_dir", "WalkDir", "walkdir", "rglob"] {
            assert!(!body.contains(forbidden), "no directory walking in a per-refresh check");
        }
    }
}
