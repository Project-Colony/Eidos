//! The account's own lists on Nexus: endorsements and tracked mods, read on
//! each update check and copied into every mod's `meta.ini`, and the Track /
//! Untrack request the mod menu sends.

use std::collections::HashSet;
use std::io;

use crate::{is_rate_limited, Nexus, UpdateCheckResult, API_BASE};

/// `(lowercased Nexus domain, mod id)` of each mod in an account list.
type ModKeys = HashSet<(String, u64)>;

/// The account's endorsed and tracked mods, keyed by `(lowercased Nexus
/// domain, mod id)`. `None` means the list was not read, so it says nothing
/// about any mod; `Some` of an empty set means Nexus sent an empty list.
#[derive(Debug, Default)]
pub struct AccountFlags {
    pub endorsed: Option<ModKeys>,
    pub tracked: Option<ModKeys>,
}

/// The `(domain, mod id)` keys of a `user/endorsements` or `user/tracked_mods`
/// reply. An entry without a string `domain_name` or an unsigned `mod_id` is
/// skipped; with `status`, so is one whose `status` differs (ASCII case
/// ignored).
pub(crate) fn mod_keys(body: &serde_json::Value, status: Option<&str>) -> Result<ModKeys, String> {
    let list = body
        .as_array()
        .ok_or_else(|| "Nexus answered without a list".to_string())?;
    Ok(list
        .iter()
        .filter(|e| {
            status.is_none_or(|s| {
                e.get("status")
                    .and_then(|v| v.as_str())
                    .is_some_and(|v| v.eq_ignore_ascii_case(s))
            })
        })
        .filter_map(|e| {
            let domain = e.get("domain_name")?.as_str()?;
            Some((domain.to_ascii_lowercase(), e.get("mod_id")?.as_u64()?))
        })
        .collect())
}

impl Nexus {
    /// The mods this account has endorsed: `GET user/endorsements`, keeping
    /// only the entries whose status is `Endorsed` (the list also holds
    /// abstentions).
    pub fn endorsements(&self) -> Result<ModKeys, String> {
        mod_keys(&self.get("user/endorsements")?, Some("Endorsed"))
    }

    /// The mods this account tracks: `GET user/tracked_mods`.
    pub fn tracked_mods(&self) -> Result<ModKeys, String> {
        mod_keys(&self.get("user/tracked_mods")?, None)
    }

    /// Track (`POST`) or untrack (`DELETE`) a mod on `user/tracked_mods`, the
    /// request behind MO2's Track. Returns the new tracked state.
    pub fn set_tracked(&self, game: &str, mod_id: u64, track: bool) -> Result<bool, String> {
        let url = format!("{API_BASE}/user/tracked_mods?domain_name={game}");
        let req = if track {
            self.agent.post(&url)
        } else {
            self.agent.delete(&url).force_send_body()
        };
        self.send_form(req, &[("mod_id", &mod_id.to_string())])?;
        Ok(track)
    }
}

/// Read both account lists for an update check, endorsements first. A list is
/// left `None` when the budget would refuse its request or Nexus refused it
/// for the budget (`result.rate_limited` is set), or on any other error, the
/// first of which goes into `result.account_sync_error`.
pub(crate) fn read_account_flags(nexus: &Nexus, result: &mut UpdateCheckResult) -> AccountFlags {
    let mut read = |fetch: fn(&Nexus) -> Result<ModKeys, String>| {
        if nexus.would_block() {
            result.rate_limited = true;
            return None;
        }
        match fetch(nexus) {
            Ok(keys) => Some(keys),
            Err(e) if is_rate_limited(&e) => {
                result.rate_limited = true;
                None
            }
            Err(e) => {
                result.account_sync_error.get_or_insert(e);
                None
            }
        }
    };
    let endorsed = read(Nexus::endorsements);
    let tracked = read(Nexus::tracked_mods);
    AccountFlags { endorsed, tracked }
}

/// Set every mod's Endorsed and Tracked flags in `meta.ini` from the lists in
/// `flags` that were read, each mod on the domain its own `gameName` names.
/// A mod without a Nexus id or a known domain is left alone, and so is a flag
/// that already agrees, whatever value MO2 wrote for it. Returns how many
/// `meta.ini` files changed.
pub fn sync_account_flags(
    inst: &eidos_instance::Instance,
    game: &eidos_gamedef::GameDef,
    flags: &AccountFlags,
) -> io::Result<u32> {
    // The flags in `meta` that differ from the account: (endorsed, tracked).
    let wanted = |meta: &eidos_instance::ModMeta, key: &(String, u64)| {
        let differs = |list: &Option<ModKeys>, now: bool| {
            list.as_ref()
                .map(|l| l.contains(key))
                .filter(|&on| on != now)
        };
        (
            differs(&flags.endorsed, meta.endorsed()),
            differs(&flags.tracked, meta.tracked()),
        )
    };
    let mut changed = 0;
    for m in inst.modlist() {
        let snapshot = inst.mod_meta(&m.name);
        let Some(mod_id) = snapshot.mod_id() else {
            continue;
        };
        let game_name = snapshot.game_name();
        let Some(domain) = game.nexus_domain_for(game_name.as_deref()) else {
            continue;
        };
        let key = (domain.to_ascii_lowercase(), mod_id);
        if wanted(&snapshot, &key) == (None, None) {
            continue;
        }
        // As `apply_mod_update`: write a fresh, locked copy, and only while the
        // folder still names the same Nexus mod.
        let _lock = inst.try_lock("saving Nexus endorsements and tracking")?;
        let path = inst.meta_path(&m.name);
        let mut meta = eidos_instance::ModMeta::read_checked(&path)?;
        if meta.mod_id() != snapshot.mod_id() || meta.game_name() != game_name {
            continue;
        }
        let (endorsed, tracked) = wanted(&meta, &key);
        if (endorsed, tracked) == (None, None) {
            continue;
        }
        if let Some(on) = endorsed {
            meta.set_endorsed(on);
        }
        if let Some(on) = tracked {
            meta.set_tracked(on);
        }
        meta.write(&path)?;
        changed += 1;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn account_lists_keep_each_mods_domain_and_only_endorsed_entries() {
        let tracked = serde_json::json!([
            {"mod_id": 42, "domain_name": "SkyrimSpecialEdition"},
            {"mod_id": 7},                                   // no domain: skipped
            {"domain_name": "skyrim"}                        // no id: skipped
        ]);
        assert_eq!(
            mod_keys(&tracked, None).unwrap(),
            HashSet::from([("skyrimspecialedition".to_string(), 42)])
        );
        let endorsements = serde_json::json!([
            {"mod_id": 1, "domain_name": "skyrim", "status": "Endorsed"},
            {"mod_id": 2, "domain_name": "skyrim", "status": "Abstained"},
            {"mod_id": 3, "domain_name": "skyrim", "status": "Undecided"}
        ]);
        assert_eq!(
            mod_keys(&endorsements, Some("Endorsed")).unwrap(),
            HashSet::from([("skyrim".to_string(), 1)])
        );
        assert!(mod_keys(&serde_json::json!({"message": "x"}), None).is_err());
    }

    /// A portable instance in the temp dir holding `mods` (folder, meta.ini).
    fn instance(test: &str, mods: &[(&str, &str)]) -> eidos_instance::Instance {
        let root =
            std::env::temp_dir().join(format!("eidos-account-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let inst = eidos_instance::Instance::portable(root);
        inst.create().unwrap();
        for (name, meta) in mods {
            fs::create_dir_all(inst.mods_dir().join(name)).unwrap();
            fs::write(inst.meta_path(name), meta).unwrap();
            inst.register_installed_mod(name).unwrap();
        }
        inst
    }

    fn enderal() -> &'static eidos_gamedef::GameDef {
        eidos_gamedef::GameDef::for_id("enderalse").unwrap()
    }

    #[test]
    fn a_list_nexus_did_not_send_changes_no_flag() {
        let inst = instance(
            "unread",
            &[(
                "A",
                "[General]\nmodid=42\ngameName=SkyrimSE\nendorsed=1\ntracked=1\n",
            )],
        );
        let flags = AccountFlags {
            endorsed: None,
            tracked: Some(HashSet::new()),
        };
        let changed = sync_account_flags(&inst, enderal(), &flags).unwrap();
        assert_eq!(changed, 1);
        assert!(
            inst.mod_meta("A").endorsed(),
            "an unread endorsement list cleared the flag"
        );
        assert!(
            !inst.mod_meta("A").tracked(),
            "an empty tracked list Nexus sent is the truth"
        );
        let none = AccountFlags::default();
        assert_eq!(sync_account_flags(&inst, enderal(), &none).unwrap(), 0);
        fs::remove_dir_all(&inst.root).unwrap();
    }

    #[test]
    fn account_lists_win_on_each_mods_own_domain() {
        let inst = instance(
            "domains",
            &[
                ("Sse", "[General]\nmodid=42\ngameName=SkyrimSE\n"),
                ("Enderal", "[General]\nmodid=42\ngameName=EnderalSE\n"),
            ],
        );
        let flags = AccountFlags {
            endorsed: Some(HashSet::from([("skyrimspecialedition".into(), 42)])),
            tracked: Some(HashSet::from([("enderalspecialedition".into(), 42)])),
        };
        assert_eq!(sync_account_flags(&inst, enderal(), &flags).unwrap(), 2);
        assert!(inst.mod_meta("Sse").endorsed() && !inst.mod_meta("Sse").tracked());
        assert!(!inst.mod_meta("Enderal").endorsed() && inst.mod_meta("Enderal").tracked());
        fs::remove_dir_all(&inst.root).unwrap();
    }

    #[test]
    fn a_flag_value_from_another_mo2_state_survives_a_sync_that_agrees() {
        // MO2 writes other numbers for its other endorsement states; the
        // account has not endorsed this one, and neither does `endorsed()`.
        let inst = instance(
            "mo2-state",
            &[("M", "[General]\nmodid=42\ngameName=SkyrimSE\nendorsed=3\n")],
        );
        let flags = AccountFlags {
            endorsed: Some(HashSet::new()),
            tracked: None,
        };
        assert_eq!(sync_account_flags(&inst, enderal(), &flags).unwrap(), 0);
        assert!(fs::read_to_string(inst.meta_path("M"))
            .unwrap()
            .contains("endorsed=3"));
        fs::remove_dir_all(&inst.root).unwrap();
    }
}
