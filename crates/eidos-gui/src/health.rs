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
//!   - the prefix's `S:` drive was pointed at `<lib>/steamapps` for a TOOL
//!     launch, so `S:\steamapps\common\<game>` - the path BodySlide's fallback
//!     tries - stopped existing, and BodySlide created it and wrote 267 MB of
//!     meshes outside the union mount where nothing ever captured them;
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
//! Cost: a few `stat`s and one `readlink` per diagnostics run, which is every
//! mod or plugin edit, not only a visit to the tab. `system.reg` (4.6 MB on a
//! real Skyrim prefix, 1-3 ms to read and scan warm) is read again only when
//! it, the `S:` link or `user.reg` changed since the last run: see
//! [`registry_status_memo`]. No timer, no polling, nothing walking the mod
//! pool - see the test at the bottom that says so.

use std::path::{Path, PathBuf};

use eidos::check::PrefixFacts;
use eidos::nxm_handler::NxmHandler;
use eidos_gamefeatures::RegistryStatus;
use eidos_games::DetectedGame;

use crate::message::Message;
use crate::modinfo::{DiagLevel, Diagnostic};
use crate::{t, tf};

/// [`eidos::check::prefix_facts`], with the registry read through
/// [`registry_status_memo`]. `None` when the game has no prefix path at all.
pub(crate) fn prefix_facts(game: &DetectedGame) -> Option<PrefixFacts> {
    eidos::check::prefix_facts_with(game, registry_status_memo)
}

/// [`eidos_gamefeatures::registry_status`], answered from the last call when
/// none of its inputs moved.
///
/// The diagnostics run on every mod toggle, and the answer depends on a file
/// Wine rewrites only when the prefix runs. So the key is everything the
/// answer reads: the prefix and install paths, the registry name, where `S:`
/// points, whether `user.reg` exists, and `system.reg`'s inode, size and
/// nanosecond mtime. Any rewrite moves the mtime, and Wine's write to a temp
/// file and rename moves the inode as well. The predicate is still the shared one, called
/// unchanged on a miss, so this cannot drift from `ensure_registry`.
fn registry_status_memo(
    compat: &std::path::Path,
    game: &DetectedGame,
    s_drive: Option<&std::path::Path>,
) -> RegistryStatus {
    use std::os::unix::fs::MetadataExt;
    type Key = (
        PathBuf,
        PathBuf,
        &'static str,
        Option<PathBuf>,
        bool,
        Option<(u64, u64, i64, i64)>,
    );
    static LAST: std::sync::Mutex<Option<(Key, RegistryStatus)>> = std::sync::Mutex::new(None);
    let pfx = compat.join("pfx");
    let key: Key = (
        compat.to_path_buf(),
        game.install_path.clone(),
        game.def.registry_name,
        s_drive.map(std::path::Path::to_path_buf),
        pfx.join("user.reg").is_file(),
        std::fs::metadata(pfx.join("system.reg"))
            .ok()
            .map(|m| (m.ino(), m.len(), m.mtime(), m.mtime_nsec())),
    );
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, status)) = last.as_ref() {
        if *k == key {
            return status.clone();
        }
    }
    // A write between the `stat` above and the read below stores a fresh answer
    // under the old key, so the next call misses and reads again: the memo can
    // be one run late, never stuck.
    let status = eidos_gamefeatures::registry_status(
        compat,
        &game.install_path,
        game.def.registry_name,
        s_drive,
    );
    *last = Some((key, status.clone()));
    status
}

/// Turn the facts into cards through [`eidos::check::prefix_findings`]. Pure:
/// no I/O, no `App`, so the interesting cases can be written down as tests
/// instead of reproduced by hand on a real prefix.
pub(crate) fn prefix_checks(f: &PrefixFacts) -> Vec<Diagnostic> {
    eidos::check::prefix_findings(f)
        .iter()
        .map(finding_card)
        .collect()
}

/// Render one `eidos::check` finding through its `diag_<code>` strings.
pub(crate) fn finding_card(f: &eidos::check::Finding) -> Diagnostic {
    debug_assert!(eidos::check::CODES.contains(&f.code), "{}", f.code);
    // A check leaves out what it could not read; the card says so in the
    // user's language.
    let unread: &[(&str, &str)] = match f.code {
        "game_crashed" | "game_crashed_unattributed" => &[
            ("logger", "diag_crash_logger_unknown"),
            ("exception", "diag_crash_no_exception"),
        ],
        _ => &[],
    };
    let fills: Vec<(&str, &str)> = unread
        .iter()
        .filter(|(k, _)| f.arg(k).is_none())
        .map(|(k, key)| (*k, t(key)))
        .collect();
    let args: Vec<(&str, &dyn std::fmt::Display)> = f
        .args
        .iter()
        .map(|(k, v)| (*k, v as &dyn std::fmt::Display))
        .chain(fills.iter().map(|(k, v)| (*k, v as &dyn std::fmt::Display)))
        .collect();
    let detail = match (f.code, f.arg("runtime")) {
        ("extender_missing", None) => "diag_extender_missing_any_version_detail".to_string(),
        _ => format!("diag_{}_detail", f.code),
    };
    Diagnostic {
        level: match f.level {
            eidos::check::Level::Problem => DiagLevel::Problem,
            eidos::check::Level::Advice => DiagLevel::Advice,
        },
        title: tf(&format!("diag_{}", f.code), &args),
        detail: tf(&detail, &args),
        actions: finding_actions(f),
    }
}

/// The findings Eidos can fix itself get a Fix button; a crash card opens or
/// copies its log.
pub(crate) fn finding_actions(f: &eidos::check::Finding) -> Vec<(String, Message)> {
    match f.code {
        "fullscreen_blocks_framegen" | "enb_linux_version" => {
            vec![(t("diag_action_fix").to_string(), Message::HealthFix(f.code))]
        }
        "game_crashed" | "game_crashed_unattributed" => {
            let Some(log) = f.arg("log").map(PathBuf::from) else {
                return Vec::new();
            };
            vec![
                (
                    t("diag_action_open_log").to_string(),
                    Message::OpenFile(log.clone()),
                ),
                (
                    t("diag_action_copy_log").to_string(),
                    Message::CopyCrashLog(log),
                ),
            ]
        }
        "extender_missing" | "extender_runtime_missing" => {
            let Some(page) = f.arg("page") else {
                return Vec::new();
            };
            vec![
                (
                    t("diag_action_extender_page").to_string(),
                    Message::OpenUrl(page.to_string()),
                ),
                (
                    t("diag_action_install_archive").to_string(),
                    Message::InstallMod,
                ),
            ]
        }
        _ => Vec::new(),
    }
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
    /// Who owns `nxm://` links; None while unread.
    pub(crate) nxm: Option<NxmHandler>,
}

pub(crate) fn nexus_checks(f: &NexusFacts) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if !f.signed_in {
        out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: t("health_nexus_signed_out").to_string(),
            detail: t("health_nexus_signed_out_detail").to_string(),
            actions: Vec::new(),
        });
    }
    if let Some(e) = &f.last_error {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: t("health_nexus_failed").to_string(),
            detail: tf("health_nexus_failed_detail", &[("error", &e)]),
            actions: Vec::new(),
        });
    }
    let register = || vec![(t("settings_nxm_register").to_string(), Message::RegisterNxm)];
    match &f.nxm {
        Some(NxmHandler::Unset) => out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: t("health_nxm_unset").to_string(),
            detail: t("health_nxm_unset_detail").to_string(),
            actions: register(),
        }),
        Some(NxmHandler::EidosMissing { exec }) => out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: t("health_nxm_missing").to_string(),
            detail: tf("health_nxm_missing_detail", &[("exec", exec)]),
            actions: register(),
        }),
        Some(NxmHandler::Other { desktop }) => out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: t("health_nxm_other").to_string(),
            detail: tf("health_nxm_other_detail", &[("desktop", desktop)]),
            actions: register(),
        }),
        _ => {}
    }
    out
}

/// What the window knows about the Proton that runs a game, read once per
/// refresh by [`proton_facts`].
pub(crate) struct ProtonFacts {
    /// Steam's setting for the game (`config.vdf`).
    pub(crate) configured: Option<String>,
    /// The resolved build: the folder holding its `proton` script.
    pub(crate) build: Option<String>,
    pub(crate) flatpak: bool,
    /// The prefix version the build writes.
    pub(crate) build_version: Option<String>,
    /// The prefix version recorded in `compatdata/version`.
    pub(crate) prefix_version: Option<String>,
    /// The build that last ran in the prefix (`config_info`).
    pub(crate) last_run: Option<String>,
    /// The compatdata folder exists, so Rebuild prefix has something to move.
    pub(crate) prefix_present: bool,
}

/// `None` when the game has no compatdata folder.
pub(crate) fn proton_facts(game: &DetectedGame, home: &Path) -> Option<ProtonFacts> {
    let cd = game.compatdata.as_ref()?;
    let app = game.def.steam_app_id;
    let run = eidos_games::proton_command(home, app, cd, &game.install_path);
    let build = run
        .as_ref()
        .and_then(|r| r.proton.parent()?.file_name()?.to_str().map(String::from));
    Some(ProtonFacts {
        configured: eidos_games::configured_tool_name(home, app),
        build,
        flatpak: run.as_ref().is_some_and(|r| r.flatpak),
        build_version: run
            .as_ref()
            .and_then(|r| eidos_games::build_prefix_version(&r.proton)),
        prefix_version: eidos_games::prefix_version(cd),
        last_run: eidos_games::config_info_name(cd),
        prefix_present: cd.is_dir(),
    })
}

pub(crate) fn proton_checks(f: &ProtonFacts) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    // A Flatpak-Steam Proton runs from the host here, which can fail to resolve
    // its sandbox libraries. Eidos will not re-launch through `flatpak run`:
    // that would put the game in Flatpak's sandbox, blind to the FUSE union in
    // our private namespace, and it would silently play vanilla.
    if f.flatpak {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: t("diag_flatpak_proton").to_string(),
            detail: t("diag_flatpak_proton_detail").to_string(),
            actions: Vec::new(),
        });
    }
    let none = t("diag_proton_none");
    let mut card = match &f.build {
        None => Diagnostic {
            level: DiagLevel::Advice,
            title: t("diag_proton_unresolved").to_string(),
            detail: t("diag_proton_unresolved_detail").to_string(),
            actions: Vec::new(),
        },
        Some(build) => match (&f.prefix_version, &f.build_version) {
            (Some(pv), Some(bv)) if eidos_games::is_prefix_downgrade(pv, bv) => Diagnostic {
                level: DiagLevel::Advice,
                title: tf("diag_proton_downgrade", &[("build", build)]),
                detail: tf(
                    "diag_proton_downgrade_detail",
                    &[
                        ("prefix_version", pv),
                        ("build", build),
                        ("build_version", bv),
                    ],
                ),
                actions: Vec::new(),
            },
            _ => Diagnostic {
                level: DiagLevel::Ok,
                title: tf("diag_proton", &[("build", build)]),
                detail: tf(
                    "diag_proton_detail",
                    &[
                        ("configured", &f.configured.as_deref().unwrap_or(none)),
                        ("last_run", &f.last_run.as_deref().unwrap_or(none)),
                    ],
                ),
                actions: Vec::new(),
            },
        },
    };
    if f.prefix_present {
        card.actions = vec![(
            t("diag_action_rebuild_prefix").to_string(),
            Message::RebuildPrefix,
        )];
    }
    out.push(card);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(registry: RegistryStatus) -> PrefixFacts {
        PrefixFacts {
            game_name: "Fallout 4".to_string(),
            registry,
            // What a real instance looks like between two launches: Steam left
            // `S:` on `steamapps` when the game last ran, and Eidos wants the
            // root for the next tool. Both sound, so the default facts are quiet.
            gamedrive_found: Some(PathBuf::from("/mnt/Jeux/SteamLibrary/steamapps")),
            gamedrive_want: Some(PathBuf::from("/mnt/Jeux/SteamLibrary")),
        }
    }

    fn proton(
        build: Option<&str>,
        prefix_version: Option<&str>,
        build_version: Option<&str>,
    ) -> ProtonFacts {
        ProtonFacts {
            configured: build.map(Into::into),
            build: build.map(Into::into),
            flatpak: false,
            build_version: build_version.map(Into::into),
            prefix_version: prefix_version.map(Into::into),
            last_run: build.map(Into::into),
            prefix_present: false,
        }
    }

    #[test]
    fn the_proton_card_offers_rebuild_only_when_a_prefix_exists() {
        let with = proton_checks(&ProtonFacts {
            prefix_present: true,
            ..proton(Some("GE-Proton10-34"), None, None)
        });
        assert!(matches!(
            with[0].actions.as_slice(),
            [(_, Message::RebuildPrefix)]
        ));
        assert!(
            proton_checks(&proton(Some("GE-Proton10-34"), None, None))[0]
                .actions
                .is_empty()
        );
    }

    #[test]
    fn the_proton_card_names_the_build_and_who_last_ran_the_prefix() {
        let d = proton_checks(&proton(
            Some("GE-Proton10-34"),
            Some("GE-Proton10-34"),
            Some("GE-Proton10-34"),
        ));
        assert_eq!((d.len(), d[0].level), (1, DiagLevel::Ok));
        assert_eq!(d[0].title, "Proton: GE-Proton10-34");
        assert_eq!(
            d[0].detail.matches("GE-Proton10-34").count(),
            2,
            "{}",
            d[0].detail
        );
    }

    #[test]
    fn an_older_build_than_the_prefix_is_advice() {
        let d = proton_checks(&proton(Some("proton_9"), Some("10.0-101"), Some("9.0-203")));
        assert_eq!((d.len(), d[0].level), (1, DiagLevel::Advice));
        assert!(
            d[0].detail.contains("10.0-101") && d[0].detail.contains("9.0-203"),
            "{}",
            d[0].detail
        );
    }

    #[test]
    fn no_resolvable_proton_is_advice_and_flatpak_is_still_a_problem() {
        let none = proton_checks(&proton(None, None, None));
        assert_eq!((none.len(), none[0].level), (1, DiagLevel::Advice));
        assert_eq!(none[0].title, t("diag_proton_unresolved"));
        let flat = proton_checks(&ProtonFacts {
            flatpak: true,
            ..proton(Some("GE-Proton10-34"), None, None)
        });
        assert_eq!(
            flat.iter().map(|d| d.level).collect::<Vec<_>>(),
            [DiagLevel::Problem, DiagLevel::Ok]
        );
    }

    #[test]
    fn fixable_cards_carry_a_fix_button() {
        use eidos::check::{Finding, Level};
        let a = finding_actions(&Finding {
            code: "enb_linux_version",
            level: Level::Advice,
            args: vec![],
        });
        assert!(matches!(
            a.as_slice(),
            [(_, Message::HealthFix("enb_linux_version"))]
        ));
        let none = finding_actions(&Finding {
            code: "nvapi_missing",
            level: Level::Advice,
            args: vec![],
        });
        assert!(none.is_empty());
    }

    #[test]
    fn every_finding_code_has_english_strings() {
        for code in eidos::check::CODES {
            for key in [format!("diag_{code}"), format!("diag_{code}_detail")] {
                assert_ne!(t(&key), key, "missing {key} in en.rs");
            }
        }
    }

    #[test]
    fn crash_cards_offer_open_and_copy() {
        use eidos::check::{Finding, Level};
        use std::path::Path;
        for code in ["game_crashed", "game_crashed_unattributed"] {
            let f = Finding {
                code,
                level: Level::Problem,
                args: vec![("log", "/tmp/crash-x.log".into())],
            };
            let a = finding_actions(&f);
            assert!(matches!(&a[0].1, Message::OpenFile(p) if p == Path::new("/tmp/crash-x.log")));
            assert!(
                matches!(&a[1].1, Message::CopyCrashLog(p) if p == Path::new("/tmp/crash-x.log"))
            );
        }
    }

    #[test]
    fn extender_cards_offer_the_download_page_and_install() {
        use eidos::check::{Finding, Level};
        let f = Finding {
            code: "extender_runtime_missing",
            level: Level::Problem,
            args: vec![("page", "https://skse.silverlock.org/".into())],
        };
        let a = finding_actions(&f);
        assert!(matches!(&a[0].1, Message::OpenUrl(u) if u == "https://skse.silverlock.org/"));
        assert!(matches!(&a[1].1, Message::InstallMod));
    }

    #[test]
    fn a_finding_renders_its_args() {
        let f = eidos::check::Finding {
            code: "extender_runtime_missing",
            level: eidos::check::Level::Problem,
            args: vec![
                ("loader", "skse64_loader.exe".into()),
                ("dll", "skse64_1_6_1179.dll".into()),
                ("runtime", "1.6.1179.0".into()),
            ],
        };
        let d = finding_card(&f);
        assert_eq!(d.level, DiagLevel::Problem);
        assert!(d.detail.contains("skse64_1_6_1179.dll"), "{}", d.detail);
        assert_eq!(d.detail.matches("1.6.1179.0").count(), 2, "{}", d.detail);
    }

    #[test]
    fn a_crash_log_without_a_logger_or_exception_line_reads_as_a_sentence() {
        for code in ["game_crashed", "game_crashed_unattributed"] {
            let f = eidos::check::Finding {
                code,
                level: eidos::check::Level::Problem,
                args: vec![
                    ("log", "/tmp/crash-x.log".into()),
                    ("culprits", "x.dll (Mod)".into()),
                ],
            };
            let d = finding_card(&f);
            assert!(
                d.detail.starts_with(&format!(
                    "{} wrote /tmp/crash-x.log. {}.",
                    t("diag_crash_logger_unknown"),
                    t("diag_crash_no_exception")
                )),
                "{}",
                d.detail
            );
            assert!(!d.detail.contains('{'), "{}", d.detail);
        }
    }

    #[test]
    fn an_extender_card_names_a_game_version_only_when_it_was_read() {
        let mut f = eidos::check::Finding {
            code: "extender_missing",
            level: eidos::check::Level::Problem,
            args: vec![
                ("loader", "nvse_loader.exe".into()),
                ("dir", "NVSE/Plugins".into()),
                ("n", "3".into()),
                ("page", "https://xnvse.example/".into()),
            ],
        };
        let d = finding_card(&f);
        assert!(!d.detail.contains("{runtime}") && !d.detail.contains("unknown"));
        assert!(d.detail.contains("https://xnvse.example/"), "{}", d.detail);
        f.args.push(("runtime", "1.6.1170.0".into()));
        assert!(finding_card(&f).detail.contains("game version 1.6.1170.0"));
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
        assert!(
            d[0].detail.contains(r"S:\common\Fallout 4\"),
            "the wrong path must be shown"
        );
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
    fn steams_own_game_drive_is_not_a_fault() {
        // The false positive that stood on a real instance for weeks: Steam
        // leaves `S:` on `<lib>/steamapps` after every game launch, Eidos wants
        // the root for tools, and the old check reported Steam's value as "not
        // where Steam puts it" after every single game launch.
        let f = facts(RegistryStatus::Correct);
        assert_eq!(
            f.gamedrive_found.as_ref(),
            Some(&PathBuf::from("/mnt/Jeux/SteamLibrary/steamapps"))
        );
        assert!(prefix_checks(&f).is_empty());
        // And Eidos's own value, the state right after a tool ran.
        let mut f = facts(RegistryStatus::Correct);
        f.gamedrive_found = Some(PathBuf::from("/mnt/Jeux/SteamLibrary"));
        assert!(prefix_checks(&f).is_empty());
    }

    #[test]
    fn a_game_drive_outside_the_library_is_reported_with_every_path() {
        let mut f = facts(RegistryStatus::Correct);
        // A library that moved, or a prefix copied from another machine.
        f.gamedrive_found = Some(PathBuf::from("/mnt/OldDisk/SteamLibrary"));
        let d = prefix_checks(&f);
        assert_eq!(d.len(), 1);
        assert!(d[0].title.contains("S: drive"));
        // Where it is, and BOTH places it may legitimately be.
        assert!(d[0].detail.contains("/mnt/OldDisk/SteamLibrary"));
        assert!(d[0].detail.contains("/mnt/Jeux/SteamLibrary  for a tool"));
        assert!(d[0]
            .detail
            .contains("/mnt/Jeux/SteamLibrary/steamapps  for"));
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
        f.gamedrive_found = Some(PathBuf::from("/mnt/OldDisk/SteamLibrary"));
        assert_eq!(
            prefix_checks(&f).len(),
            2,
            "one card per fault, not one card for both"
        );
    }

    #[test]
    fn a_working_nexus_says_nothing() {
        assert!(nexus_checks(&NexusFacts {
            signed_in: true,
            last_error: None,
            nxm: None,
        })
        .is_empty());
    }

    #[test]
    fn signed_out_names_both_things_it_breaks() {
        let d = nexus_checks(&NexusFacts {
            signed_in: false,
            last_error: None,
            nxm: None,
        });
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
            nxm: None,
        });
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].level, DiagLevel::Problem);
        assert!(d[0].detail.contains("429"));
    }

    #[test]
    fn nxm_handler_cards_offer_register() {
        let card = |h| {
            nexus_checks(&NexusFacts {
                signed_in: true,
                last_error: None,
                nxm: Some(h),
            })
        };
        assert!(card(NxmHandler::Eidos { exec: "/x".into() }).is_empty());
        assert!(card(NxmHandler::Unknown).is_empty());
        let missing = card(NxmHandler::EidosMissing { exec: "/x".into() });
        assert_eq!(missing[0].level, DiagLevel::Problem);
        assert!(matches!(missing[0].actions[0].1, Message::RegisterNxm));
        assert_eq!(card(NxmHandler::Unset)[0].level, DiagLevel::Advice);
        assert_eq!(
            card(NxmHandler::Other {
                desktop: "a.desktop".into()
            })[0]
                .level,
            DiagLevel::Advice
        );
    }

    #[test]
    fn signed_out_and_failing_is_two_cards() {
        let d = nexus_checks(&NexusFacts {
            signed_in: false,
            last_error: Some("connection refused".to_string()),
            nxm: None,
        });
        assert_eq!(d.len(), 2);
    }

    /// The memo must never outlive the file it read: Wine rewrites
    /// `system.reg` when the prefix runs, and the card has to follow.
    #[test]
    fn the_registry_answer_follows_a_rewritten_system_reg() {
        let root = crate::test_support::temp_portable("fallout4");
        let compat = root.join("compatdata");
        let pfx = compat.join("pfx");
        std::fs::create_dir_all(&pfx).unwrap();
        std::fs::write(pfx.join("user.reg"), "WINE REGISTRY Version 2\n").unwrap();
        std::fs::write(pfx.join("system.reg"), "WINE REGISTRY Version 2\n").unwrap();
        let install = root.join("Fallout 4");
        let game = DetectedGame {
            def: eidos_games::GameDef::for_id("fallout4").expect("in the catalog"),
            install_path: install.clone(),
            data_path: install.join("Data"),
            compatdata: Some(compat.clone()),
            source: Default::default(),
            steam_name: "Fallout 4".to_string(),
        };
        let first = prefix_facts(&game).unwrap().registry;
        assert!(
            matches!(first, RegistryStatus::Stale { found: None, .. }),
            "{first:?}"
        );
        // Asked again with nothing changed: the same answer.
        assert_eq!(prefix_facts(&game).unwrap().registry, first);

        let value = format!("Z:{}\\", install.display())
            .replace('/', "\\")
            .replace('\\', "\\\\");
        let mut reg = String::from("WINE REGISTRY Version 2\n");
        for view in ["Software", "Software\\\\Wow6432Node"] {
            reg.push_str(&format!(
                "\n[{view}\\\\Bethesda Softworks\\\\Fallout4] 1\n\"installed path\"=\"{value}\"\n"
            ));
        }
        std::fs::write(pfx.join("system.reg"), reg).unwrap();
        assert_eq!(
            prefix_facts(&game).unwrap().registry,
            RegistryStatus::Correct,
            "a rewritten system.reg is read again"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_checks_do_no_io() {
        // The property that keeps this cheap enough to run on every refresh of
        // the tab. `prefix_checks` takes facts and returns cards; if someone ever
        // reaches for the filesystem inside it, this file will need `std::fs`
        // imported and this assertion is the note explaining why it must not be.
        let src = include_str!("health.rs");
        let body = src.split("#[cfg(test)]").next().unwrap_or("");
        for check in [
            "pub(crate) fn prefix_checks",
            "pub(crate) fn nexus_checks",
            "pub(crate) fn proton_checks",
        ] {
            let fn_body = body
                .split(check)
                .nth(1)
                .and_then(|rest| rest.split("\n}\n").next())
                .unwrap_or("");
            assert!(!fn_body.is_empty(), "{check} is still in this file");
            assert!(
                !fn_body.contains("fs::"),
                "filesystem access belongs in prefix_facts, not in {check}"
            );
        }
        // And nothing here may walk a directory tree: that is the one thing that
        // would make the Diagnostics tab expensive on a large mod pool.
        for forbidden in ["read_dir", "WalkDir", "walkdir", "rglob"] {
            assert!(
                !body.contains(forbidden),
                "no directory walking in a per-refresh check"
            );
        }
    }
}
