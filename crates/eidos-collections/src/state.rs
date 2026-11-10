//! Where each member stands, on disk, updated after every one.
//!
//! A 200-mod collection is hours of downloading. Something will interrupt it -
//! a crash, a closed window, a reboot - and the question the next start has to
//! answer is "where was member 147". Vortex learned this the expensive way: its
//! session lives in a non-persisted store, so it had to build a reconstruction
//! that infers each member's status from the installed mods, the downloads
//! folder and a durable per-rule flag. Eidos writes the answer down instead.
//!
//! The one rule worth stating out loud, because getting it wrong makes a
//! collection permanently unfinishable: **a decision the user made outranks
//! anything the machinery concludes later.** A member the user skipped must not
//! be rehydrated as pending by a later pass, or the collection can never
//! complete and nothing on screen explains why.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where one member is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Nothing has happened to it yet.
    Pending,
    /// Its archive is in `downloads/`, whole.
    Downloaded,
    /// Installed, as the mod folder named here.
    Installed(String),
    /// The user said no. Final, and never overridden automatically.
    Skipped,
    /// Installed, but not the way the collection asked: the author's installer
    /// answers could not all be replayed. Carries what did not match.
    Approximate(String, Vec<String>),
    /// It cannot be had: taken down, gated, or a source Eidos cannot fetch.
    Unavailable(String),
    /// It was tried and it failed. Retryable.
    Failed(String),
    /// A status a newer Eidos wrote, kept verbatim.
    ///
    /// Final here, and written back unchanged: re-running an install a later
    /// version considered finished is the more destructive of the two guesses,
    /// and rewriting its record in this build's vocabulary would destroy what it
    /// knew.
    Foreign(serde_json::Value),
}

impl Status {
    /// Whether this member still needs work.
    ///
    /// `Unavailable` counts: it is what a free account's Nexus file is recorded
    /// as, and the instruction that goes with it is "fetch it yourself and run
    /// this again", so it is the most open a member can be.
    pub fn is_open(&self) -> bool {
        matches!(
            self,
            Status::Pending | Status::Downloaded | Status::Failed(_) | Status::Unavailable(_)
        )
    }

    /// Whether nothing in this build may change it.
    fn is_final(&self) -> bool {
        matches!(self, Status::Skipped | Status::Foreign(_))
    }

    /// A word for a report.
    pub fn word(&self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Downloaded => "downloaded",
            Status::Installed(_) => "installed",
            Status::Skipped => "skipped",
            Status::Approximate(_, _) => "installed with different options",
            Status::Unavailable(_) => "unavailable",
            Status::Failed(_) => "failed",
            Status::Foreign(_) => "recorded by a newer Eidos",
        }
    }
}

/// The install of one collection revision, as it stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallState {
    pub slug: String,
    pub revision: u32,
    pub game_domain: String,
    /// Keyed by the member's stable key - see [`member_key`] - rather than by
    /// its index, so a revision that gains or loses a member does not shift
    /// every later member's recorded status by one.
    pub members: BTreeMap<String, Status>,
    /// Exact folders reserved before extraction. Download state never confers ownership.
    pub folders: BTreeMap<String, String>,
    /// An explicit continuation, valid only for these exact runtime references.
    pub runtime_decision: Option<crate::recipe::RuntimeCheck>,
}

/// What identifies a member across runs.
///
/// The file id when there is one, because it is exact and it is what the
/// download is named after. Otherwise the mod id, otherwise the author's own
/// name for it - which is all a `manual` or `browse` member has.
///
/// Deliberately NOT a hash of the member's contents. Vortex computes one because
/// it has a single global mod store several collections claim shares of; Eidos
/// is per instance, so a collection can own its members outright and the id it
/// was published with is enough.
pub fn member_key(domain: &str, file_id: Option<u64>, mod_id: Option<u64>, name: &str) -> String {
    // Nexus ids are per GAME, and a collection can pull a member off another
    // game's page - a Skyrim SE list taking an asset from the LE page is the
    // ordinary example - so a bare id is ambiguous across domains.
    let d = domain.trim().to_ascii_lowercase();
    match (file_id, mod_id) {
        (Some(f), _) => format!("file:{d}:{f}"),
        (None, Some(m)) => format!("mod:{d}:{m}"),
        _ => format!("name:{}", name.trim().to_lowercase()),
    }
}

/// [`member_key`] for a member, with the collection's own domain standing in
/// when the member does not name one.
pub fn key_for(m: &crate::manifest::Mod, collection_domain: &str) -> String {
    let domain = if m.domain_name.trim().is_empty() {
        collection_domain
    } else {
        &m.domain_name
    };
    member_key(domain, m.source.file_id, m.source.mod_id, &m.name)
}

/// Whether an automatic outcome may replace what is recorded.
///
/// The user's own decisions are final. Everything else moves freely, including
/// backwards: a retry legitimately takes a member from `Failed` to `Pending`.
pub fn may_write(current: Option<&Status>, outcome: &Status) -> bool {
    match current {
        Some(c) if c.is_final() => false,
        Some(c) => c != outcome,
        None => true,
    }
}

/// The folder one revision of one collection owns inside an instance.
///
/// The slug arrives from the API, so it is remote data that would otherwise be
/// pasted straight into a path. Anything that is not a plain name is replaced,
/// which keeps a hostile or merely odd slug inside `collections/`.
pub fn revision_dir(root: &Path, slug: &str, revision: u32) -> PathBuf {
    root.join("collections")
        .join(format!("{}-{revision}", safe_slug(slug)))
}

/// A slug reduced to one harmless path segment.
pub fn safe_slug(slug: &str) -> String {
    let cleaned: String = slug
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('_').to_string();
    if cleaned.is_empty() {
        "collection".to_string()
    } else {
        cleaned
    }
}

/// Every installed collection, the highest revision per slug.
///
/// A file that cannot be read is skipped: this feeds a notice, and a notice
/// must not fail because one state file is damaged.
pub fn installed(root: &Path) -> Vec<InstallState> {
    let Ok(dir) = std::fs::read_dir(root.join("collections")) else {
        return Vec::new();
    };
    let mut best: BTreeMap<String, InstallState> = BTreeMap::new();
    for entry in dir.flatten() {
        let path = entry.path();
        let is_state = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with(".state.json"));
        let Some(state) = is_state
            .then(|| InstallState::load(&path).ok().flatten())
            .flatten()
        else {
            continue;
        };
        match best.get(&state.slug) {
            Some(have) if have.revision >= state.revision => {}
            _ => {
                best.insert(state.slug.clone(), state);
            }
        }
    }
    best.into_values().collect()
}

/// What a newer revision changes, by member name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevisionDiff {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub dropped: Vec<String>,
}

/// Compare an installed revision's manifest with a newer revision.
///
/// Only Nexus members count, keyed by (domain, mod id): the same mod with
/// another file id is an update. Members without a mod id have nothing to match on.
pub fn revision_diff(
    old: &crate::manifest::Collection,
    collection_domain: &str,
    new: &eidos_nexus::collections::CollectionRevision,
) -> RevisionDiff {
    let key = |domain: &str, id: u64| {
        let d = if domain.trim().is_empty() {
            collection_domain
        } else {
            domain
        };
        (d.trim().to_ascii_lowercase(), id)
    };
    let old: BTreeMap<_, _> = old
        .mods
        .iter()
        .filter(|m| m.source.kind == crate::manifest::SourceType::Nexus)
        .filter_map(|m| {
            Some((
                key(&m.domain_name, m.source.mod_id?),
                (m.source.file_id, &m.name),
            ))
        })
        .collect();
    let mut diff = RevisionDiff::default();
    let mut seen = std::collections::BTreeSet::new();
    for m in &new.mods {
        let k = key(&m.domain, m.mod_id);
        match old.get(&k) {
            None => diff.added.push(m.name.clone()),
            Some((file, _)) if *file != Some(m.file_id) => diff.updated.push(m.name.clone()),
            Some(_) => {}
        }
        seen.insert(k);
    }
    diff.dropped = old
        .iter()
        .filter(|(k, _)| !seen.contains(*k))
        .map(|(_, (_, name))| (*name).clone())
        .collect();
    diff
}

impl InstallState {
    /// Record an outcome, honouring [`may_write`].
    pub fn set(&mut self, key: &str, outcome: Status) -> bool {
        if !may_write(self.members.get(key), &outcome) {
            return false;
        }
        self.members.insert(key.to_string(), outcome);
        true
    }

    /// Record what the USER decided. Always applied.
    pub fn set_by_user(&mut self, key: &str, outcome: Status) {
        self.members.insert(key.to_string(), outcome);
    }

    pub fn status(&self, key: &str) -> &Status {
        self.members.get(key).unwrap_or(&Status::Pending)
    }

    /// Whether anything is still to do, for the members `keys` names.
    ///
    /// The member list has to be passed in: a member the run never reached has
    /// no entry at all, so asking the map alone would call a collection
    /// interrupted at member 147 of 200 finished.
    pub fn is_finished(&self, keys: &[String]) -> bool {
        keys.iter().all(|k| !self.status(k).is_open())
    }

    /// Where the state file for a collection lives inside an instance.
    ///
    /// Beside the revision's directory, never INSIDE it: that directory is where
    /// the collection's own archive is unpacked, and a file Eidos trusts must
    /// not share a path with a file the archive controls - a `state.json` member
    /// would otherwise be read back as this program's own bookkeeping.
    pub fn path(root: &Path, slug: &str, revision: u32) -> PathBuf {
        let dir = revision_dir(root, slug, revision);
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "collection".to_string());
        dir.with_file_name(format!("{name}.state.json"))
    }

    /// Write the state so an interruption cannot destroy it.
    ///
    /// A plain truncating write is rewritten twice per member - four hundred
    /// times on a two hundred mod collection, over several hours - and a crash
    /// inside one of those windows leaves an empty or half-written file. That
    /// file is what stands between a resumed install and re-downloading and
    /// wipe-reinstalling every member, so it is written beside itself and
    /// renamed into place, which yields the old file or the new one and never a
    /// broken one.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = path.with_extension("json.new");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::File::open(&tmp)?.sync_all()?;
        std::fs::rename(&tmp, path)?;
        if let Some(parent) = path.parent() {
            std::fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    }

    /// Refuse a state file belonging to another revision, including sanitized slug collisions.
    pub fn validate_revision(&self, slug: &str, revision: u32, domain: &str) -> Result<(), String> {
        if self.slug != slug
            || self.revision != revision
            || !self.game_domain.eq_ignore_ascii_case(domain)
        {
            return Err("The saved collection state belongs to another revision or game; nothing was installed".into());
        }
        Ok(())
    }

    /// Read the state back.
    ///
    /// `Ok(None)` is "there is no file yet", which is a first run. `Err` is "the
    /// file is there and cannot be understood", which is NOT a first run and
    /// must not be treated as one: starting over means hours of downloads again
    /// and a wipe-reinstall of every member.
    pub fn load(path: &Path) -> Result<Option<InstallState>, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        InstallState::from_json(&text).map(Some).ok_or_else(|| {
            format!(
                "{} records what this collection already installed, and it cannot be read. \
                 Nothing was changed. Move that file aside to start the collection over, \
                 knowing that everything in it will be downloaded and installed again.",
                path.display()
            )
        })
    }

    pub fn to_json(&self) -> String {
        let members: serde_json::Map<String, serde_json::Value> = self
            .members
            .iter()
            .map(|(k, v)| (k.clone(), status_json(v)))
            .collect();
        let doc = serde_json::json!({
            "schema": 1,
            "slug": self.slug,
            "revision": self.revision,
            "gameDomain": self.game_domain,
            "members": members,
            "folders": self.folders,
            "runtimeDecision": self.runtime_decision,
        });
        serde_json::to_string_pretty(&doc).unwrap_or_default()
    }

    /// Decode bookkeeping without treating a malformed reservation as a fresh install.
    /// `load` turns a decoding failure into an error for callers.
    pub fn from_json(text: &str) -> Option<InstallState> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let members = v.get("members")?.as_object()?;
        Some(InstallState {
            slug: v.get("slug")?.as_str()?.to_string(),
            revision: u32::try_from(v.get("revision")?.as_u64()?).ok()?,
            game_domain: v.get("gameDomain")?.as_str()?.to_string(),
            members: members
                .iter()
                .map(|(k, v)| Some((k.clone(), status_from_json(v)?)))
                .collect::<Option<_>>()?,
            runtime_decision: serde_json::from_value(
                v.get("runtimeDecision")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
            .ok()?,
            folders: serde_json::from_value(
                v.get("folders")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            )
            .ok()?,
        })
    }
}

fn status_json(s: &Status) -> serde_json::Value {
    match s {
        Status::Pending => serde_json::json!({ "status": "pending" }),
        Status::Downloaded => serde_json::json!({ "status": "downloaded" }),
        Status::Installed(m) => serde_json::json!({ "status": "installed", "mod": m }),
        Status::Skipped => serde_json::json!({ "status": "skipped" }),
        Status::Approximate(m, why) => {
            serde_json::json!({ "status": "approximate", "mod": m, "notes": why })
        }
        Status::Unavailable(w) => serde_json::json!({ "status": "unavailable", "why": w }),
        Status::Failed(w) => serde_json::json!({ "status": "failed", "why": w }),
        // Verbatim: a newer Eidos has to be able to read its own record back.
        Status::Foreign(v) => v.clone(),
    }
}

fn status_from_json(v: &serde_json::Value) -> Option<Status> {
    let s = v.get("status")?.as_str()?;
    let text = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string()
    };
    Some(match s {
        "pending" => Status::Pending,
        "downloaded" => Status::Downloaded,
        "installed" => Status::Installed(text("mod")),
        "skipped" => Status::Skipped,
        "approximate" => Status::Approximate(
            text("mod"),
            v.get("notes")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|n| n.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        "unavailable" => Status::Unavailable(text("why")),
        "failed" => Status::Failed(text("why")),
        // A status a newer Eidos wrote.
        _ => Status::Foreign(v.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_users_skip_outranks_anything_the_machinery_concludes() {
        // Get this wrong and a skipped REQUIRED member rehydrates as pending,
        // the collection can never report itself finished, and nothing on
        // screen explains why.
        let mut s = InstallState::default();
        s.set_by_user("file:1", Status::Skipped);
        assert!(!s.set("file:1", Status::Pending));
        assert!(!s.set("file:1", Status::Failed("boom".into())));
        assert!(!s.set("file:1", Status::Installed("Mod".into())));
        assert_eq!(*s.status("file:1"), Status::Skipped);
        // The user can still change their own mind.
        s.set_by_user("file:1", Status::Pending);
        assert_eq!(*s.status("file:1"), Status::Pending);
    }

    #[test]
    fn a_retry_may_move_a_member_backwards() {
        let mut s = InstallState::default();
        assert!(s.set("file:1", Status::Failed("timeout".into())));
        assert!(s.set("file:1", Status::Pending), "retrying is legitimate");
    }

    #[test]
    fn an_unchanged_outcome_is_not_a_write() {
        // The engine reports after every member; rewriting the same file
        // hundreds of times for no change is work nobody asked for.
        let mut s = InstallState::default();
        assert!(s.set("file:1", Status::Downloaded));
        assert!(!s.set("file:1", Status::Downloaded));
    }

    #[test]
    fn the_key_survives_a_revision_that_gains_or_loses_a_member() {
        // Keyed by index, member 147's recorded status would land on member 146
        // the moment a revision dropped a mod.
        assert_eq!(
            member_key("skyrimspecialedition", Some(9), Some(3), "X"),
            "file:skyrimspecialedition:9"
        );
        assert_eq!(member_key("skyrim", None, Some(3), "X"), "mod:skyrim:3");
        assert_eq!(member_key("", None, None, " A Mod "), "name:a mod");
        // The same numeric id on two Nexus pages is two members.
        assert_ne!(
            member_key("skyrim", Some(9), None, "X"),
            member_key("skyrimspecialedition", Some(9), None, "X")
        );
    }

    #[test]
    fn the_state_round_trips_through_its_file() {
        let mut s = InstallState {
            slug: "rqhcxy".into(),
            revision: 12,
            game_domain: "skyrimspecialedition".into(),
            ..InstallState::default()
        };
        s.set("file:1", Status::Installed("Great Cities".into()));
        s.set("file:2", Status::Failed("404".into()));
        s.set_by_user("file:3", Status::Skipped);
        s.set(
            "file:4",
            Status::Approximate("Patch".into(), vec!["\"8K\" is no longer an option".into()]),
        );
        s.set("file:5", Status::Unavailable("taken down".into()));
        let back = InstallState::from_json(&s.to_json()).expect("its own file must read");
        assert_eq!(back, s);
    }

    #[test]
    fn corrupt_state_is_rejected_and_unknown_member_status_is_preserved() {
        assert!(InstallState::from_json("not json").is_none());
        assert!(InstallState::from_json("{}").is_none());
        // But a member whose status a NEWER Eidos wrote is left alone rather
        // than re-run: re-installing what a later version finished is the more
        // destructive guess.
        let s = InstallState::from_json(
            r#"{"slug":"x","revision":1,"gameDomain":"skyrimspecialedition","members":{"file:1":{"status":"quantum"}}}"#,
        )
        .unwrap();
        assert!(!s.status("file:1").is_open());
        // And it is written back exactly as it was found.
        let round = InstallState::from_json(&s.to_json()).unwrap();
        assert_eq!(round.status("file:1"), s.status("file:1"));
        assert!(s.to_json().contains("quantum"));
    }

    #[test]
    fn invalid_bookkeeping_cannot_drop_members_or_change_collection_identity() {
        let state = InstallState {
            slug: "one".into(),
            revision: 1,
            game_domain: "skyrimspecialedition".into(),
            ..Default::default()
        };
        assert!(state
            .validate_revision("one", 1, "SkyrimSpecialEdition")
            .is_ok());
        assert!(state
            .validate_revision("two", 1, "skyrimspecialedition")
            .is_err());
        assert!(state
            .validate_revision("one", 2, "skyrimspecialedition")
            .is_err());
        for (key, invalid) in [
            ("members", serde_json::json!({"file:1": {}})),
            ("folders", serde_json::json!({"file:1": 9})),
            ("revision", serde_json::json!(4294967296u64)),
            ("slug", serde_json::json!([])),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(&state.to_json()).unwrap();
            value[key] = invalid;
            assert!(
                InstallState::from_json(&value.to_string()).is_none(),
                "{key}"
            );
        }
    }

    #[test]
    fn finished_means_nothing_is_open() {
        let keys: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let mut s = InstallState::default();
        s.set("a", Status::Installed("A".into()));
        s.set_by_user("b", Status::Skipped);
        // A member the run never reached has no entry at all, and it is exactly
        // the one an interrupted install has most of.
        assert!(!s.is_finished(&keys), "c and d were never reached");
        assert!(s.is_finished(&keys[..2]));
        s.set("c", Status::Unavailable("only you can fetch this".into()));
        assert!(
            !s.is_finished(&keys[..3]),
            "a member waiting on the user is the most open one there is"
        );
        s.set("d", Status::Failed("try again".into()));
        assert!(!s.is_finished(&keys), "a failure is still open work");
    }

    #[test]
    fn the_state_file_sits_with_the_collection_it_belongs_to() {
        let p = InstallState::path(Path::new("/inst"), "rqhcxy", 12);
        assert_eq!(
            p,
            Path::new("/inst/collections/rqhcxy-12.state.json"),
            "one file per revision, so two revisions never share a state"
        );
        // And NOT inside the directory the collection's own archive unpacks
        // into, or a collection could ship its own bookkeeping.
        assert!(!p.starts_with(revision_dir(Path::new("/inst"), "rqhcxy", 12)));
    }

    #[test]
    fn an_interrupted_write_cannot_destroy_the_record() {
        let dir = std::env::temp_dir().join(format!("eidos-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("state.json");
        let mut s = InstallState {
            slug: "x".into(),
            revision: 1,
            ..InstallState::default()
        };
        s.set("file:1", Status::Installed("A".into()));
        s.save(&p).expect("first save");
        assert_eq!(InstallState::load(&p).unwrap().as_ref(), Some(&s));
        // The temp file never survives a completed save.
        assert!(!p.with_extension("json.new").exists());

        // A file that exists and cannot be read is NOT a first run: saying so is
        // what stands between a resume and re-downloading everything.
        std::fs::write(&p, "half a fi").unwrap();
        let e = InstallState::load(&p).unwrap_err();
        assert!(e.contains("cannot be read"), "{e}");
        assert!(InstallState::load(&dir.join("nope.json"))
            .unwrap()
            .is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_slug_from_the_api_cannot_walk_out_of_the_instance() {
        let root = Path::new("/inst");
        let p = InstallState::path(root, "../../etc/cron.d/evil", 3);
        assert!(p.starts_with("/inst/collections"), "{}", p.display());
        assert_eq!(safe_slug("../../etc"), "etc");
        assert_eq!(safe_slug("rqhcxy"), "rqhcxy");
        assert_eq!(safe_slug("a/b"), "a_b");
        assert_eq!(safe_slug("///"), "collection");
    }

    #[test]
    fn installed_reports_the_newest_revision_per_slug() {
        let root = std::env::temp_dir().join(format!("eidos-installed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (slug, rev) in [("abc", 12), ("abc", 14), ("xyz", 3)] {
            let s = InstallState {
                slug: slug.into(),
                revision: rev,
                game_domain: "skyrimspecialedition".into(),
                ..InstallState::default()
            };
            s.save(&InstallState::path(&root, slug, rev)).unwrap();
        }
        std::fs::write(root.join("collections/junk.state.json"), "nope").unwrap();
        let mut got: Vec<_> = installed(&root)
            .into_iter()
            .map(|s| (s.slug, s.revision))
            .collect();
        got.sort();
        assert_eq!(got, [("abc".to_string(), 14), ("xyz".to_string(), 3)]);
        assert!(installed(&root.join("missing")).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn revision_diff_names_added_updated_and_dropped_members() {
        use crate::manifest::{Collection, Mod, Source};
        use eidos_nexus::collections::{CollectionMod, CollectionRevision};
        let old_mod = |name: &str, domain: &str, mod_id, file_id| Mod {
            name: name.into(),
            domain_name: domain.into(),
            source: Source {
                mod_id: Some(mod_id),
                file_id: Some(file_id),
                ..Source::default()
            },
            ..Mod::default()
        };
        let new_mod = |name: &str, domain: &str, mod_id, file_id| CollectionMod {
            name: name.into(),
            domain: domain.into(),
            mod_id,
            file_id,
            version: String::new(),
            file_title: String::new(),
            size_in_bytes: 0,
            optional: false,
        };
        let old = Collection {
            mods: vec![
                old_mod("A", "", 1, 10),
                old_mod("B", "skyrimspecialedition", 2, 20),
                old_mod("C", "skyrim", 3, 30),
            ],
            ..Collection::default()
        };
        let new = CollectionRevision {
            slug: "abc".into(),
            revision_number: 2,
            name: String::new(),
            summary: String::new(),
            author: String::new(),
            game_domain: "skyrimspecialedition".into(),
            mod_count: 3,
            total_size: 0,
            instructions: String::new(),
            mods: vec![
                new_mod("A", "SkyrimSpecialEdition", 1, 10),
                new_mod("B", "", 2, 21),
                new_mod("D", "skyrim", 4, 40),
            ],
            download_link: String::new(),
            hidden: None,
        };
        let d = revision_diff(&old, "skyrimspecialedition", &new);
        assert_eq!(
            (d.added, d.updated, d.dropped),
            (vec!["D".into()], vec!["B".into()], vec!["C".into()])
        );
    }
}
