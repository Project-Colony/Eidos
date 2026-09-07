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
}

impl Status {
    /// Whether this member still needs work.
    pub fn is_open(&self) -> bool {
        matches!(self, Status::Pending | Status::Downloaded | Status::Failed(_))
    }

    /// Whether the user decided this, rather than the machinery.
    fn is_the_users_word(&self) -> bool {
        matches!(self, Status::Skipped)
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
pub fn member_key(file_id: Option<u64>, mod_id: Option<u64>, name: &str) -> String {
    match (file_id, mod_id) {
        (Some(f), _) => format!("file:{f}"),
        (None, Some(m)) => format!("mod:{m}"),
        _ => format!("name:{}", name.trim().to_lowercase()),
    }
}

/// Whether an automatic outcome may replace what is recorded.
///
/// The user's own decisions are final. Everything else moves freely, including
/// backwards: a retry legitimately takes a member from `Failed` to `Pending`.
pub fn may_write(current: Option<&Status>, outcome: &Status) -> bool {
    match current {
        Some(c) if c.is_the_users_word() => false,
        Some(c) => c != outcome,
        None => true,
    }
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

    /// Whether anything is still to do.
    pub fn is_finished(&self) -> bool {
        !self.members.values().any(Status::is_open)
    }

    /// Where the state file for a collection lives inside an instance.
    pub fn path(root: &Path, slug: &str, revision: u32) -> PathBuf {
        root.join("collections")
            .join(format!("{slug}-{revision}"))
            .join("state.json")
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
        });
        serde_json::to_string_pretty(&doc).unwrap_or_default()
    }

    /// Read a state file back. A file that cannot be understood is `None`, not an
    /// error: the worst it costs is starting the collection over, and refusing
    /// to run because of an unreadable bookkeeping file would be worse.
    pub fn from_json(text: &str) -> Option<InstallState> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let members = v.get("members")?.as_object()?;
        Some(InstallState {
            slug: v.get("slug")?.as_str().unwrap_or_default().to_string(),
            revision: v.get("revision").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            game_domain: v
                .get("gameDomain")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            members: members
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), status_from_json(v)?)))
                .collect(),
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
        // A status a newer Eidos wrote. Treated as done rather than as pending:
        // re-running an install that a later version considered finished is the
        // more destructive of the two guesses.
        _ => Status::Unavailable(format!("recorded by a newer Eidos as \"{s}\"")),
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
        assert_eq!(member_key(Some(9), Some(3), "X"), "file:9");
        assert_eq!(member_key(None, Some(3), "X"), "mod:3");
        assert_eq!(member_key(None, None, " A Mod "), "name:a mod");
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
    fn a_state_file_that_cannot_be_understood_is_a_fresh_start_not_a_refusal() {
        assert!(InstallState::from_json("not json").is_none());
        assert!(InstallState::from_json("{}").is_none());
        // But a member whose status a NEWER Eidos wrote is left alone rather
        // than re-run: re-installing what a later version finished is the more
        // destructive guess.
        let s = InstallState::from_json(
            r#"{"slug":"x","revision":1,"members":{"file:1":{"status":"quantum"}}}"#,
        )
        .unwrap();
        assert!(!s.status("file:1").is_open());
    }

    #[test]
    fn finished_means_nothing_is_open() {
        let mut s = InstallState::default();
        s.set("a", Status::Installed("A".into()));
        s.set_by_user("b", Status::Skipped);
        s.set("c", Status::Unavailable("gone".into()));
        assert!(s.is_finished());
        s.set("d", Status::Failed("try again".into()));
        assert!(!s.is_finished(), "a failure is still open work");
    }

    #[test]
    fn the_state_file_sits_with_the_collection_it_belongs_to() {
        let p = InstallState::path(Path::new("/inst"), "rqhcxy", 12);
        assert_eq!(
            p,
            Path::new("/inst/collections/rqhcxy-12/state.json"),
            "one folder per revision, so two revisions never share a state"
        );
    }
}
