//! Nexus Mods integration, mirroring Mod Organizer 2's client behaviour
//! (`nxmaccessmanager.cpp` + `nexusinterface.cpp` + `downloadmanager.cpp`):
//!
//! - **Auth**: an OAuth access token as `Authorization: Bearer`, and nothing
//!   else. Validated via `users/validate`; like MO2, no `.json` suffix on
//!   requests. Signing in needs a `client_id` registered with Nexus (see
//!   [`oauth`]).
//!
//!   Personal API keys are NOT supported, deliberately and not as a fallback.
//!   Nexus's API team requires their complete removal before issuing a
//!   `client_id`: the personal key is documented on their side as being for
//!   testing and personal use, not for a distributed application. Eidos
//!   therefore has no Nexus access at all until a `client_id` is issued, which
//!   is the intended state rather than an oversight.
//! - **`nxm://` links** (the site's "Mod Manager Download" button):
//!   `nxm://<game>/mods/<id>/files/<file_id>?key=..&expires=..&user_id=..` -
//!   non-premium downloads REQUIRE that key/expires pair forwarded to the
//!   `download_link` endpoint (MO2 `NXMUrl` + `addNXMDownload`).
//! - **`.meta` sidecar**: each download gets `<archive>.meta` in MO2's exact
//!   key set (gameName/modID/fileID/url/version/...), which `eidos-install`
//!   already reads to seed the installed mod's `meta.ini`.
//! - **Adult content** is gated INSIDE this crate, in
//!   [`RemoteMod::from_payload`], and nowhere else. A mod the account may not be
//!   shown has its descriptive fields blanked before the struct is built, so the
//!   text never crosses the crate boundary and no display site can leak it by
//!   forgetting to check. **Any field added to [`RemoteMod`] that comes from the
//!   `games/{game}/mods/{id}` payload must be redacted there too** - there is no
//!   second gate downstream to catch it. Fetching a file or a download link
//!   requires the [`ModGate`] that lookup mints, so the ordering is enforced by
//!   the compiler rather than by convention.
//! - **The request budget** is enforced BEFORE a request is sent, not after
//!   Nexus refuses one: [`Nexus::get`] and `send_with_version` both start with a
//!   pre-flight check, so an exhausted account stops the whole client rather
//!   than each loop that remembered to look. See [`RATE_LIMITED`].

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const API_BASE: &str = "https://api.nexusmods.com/v1";

pub mod oauth;

/// A parsed `nxm://` mod-file link (MO2's `NXMUrl`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxmUrl {
    /// The Nexus game domain from the URL host, e.g. `skyrimspecialedition`.
    pub game: String,
    pub mod_id: u64,
    pub file_id: u64,
    /// Per-user download key (present on non-premium "Mod Manager Download").
    pub key: Option<String>,
    pub expires: Option<u64>,
    pub user_id: Option<u64>,
}

impl NxmUrl {
    /// Parse `nxm://game/mods/<id>/files/<fid>?key=..&expires=..`. Collection
    /// links (`/collections/...`, the v2 GraphQL API) are not supported yet.
    pub fn parse(url: &str) -> Result<NxmUrl, String> {
        let rest = url
            .strip_prefix("nxm://")
            .ok_or_else(|| format!("not an nxm:// link: {url}"))?;
        let (path, query) = match rest.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (rest, None),
        };
        let segs: Vec<&str> = path.trim_end_matches('/').split('/').collect();
        if segs.len() >= 2 && segs[1].eq_ignore_ascii_case("collections") {
            return Err("collection links are not supported yet (Nexus v2 API)".to_string());
        }
        // game / mods / <id> / files / <fid>
        if segs.len() != 5
            || !segs[1].eq_ignore_ascii_case("mods")
            || !segs[3].eq_ignore_ascii_case("files")
        {
            return Err(format!("unrecognized nxm link shape: {url}"));
        }
        let mod_id: u64 = segs[2].parse().map_err(|_| format!("bad mod id in {url}"))?;
        let file_id: u64 = segs[4].parse().map_err(|_| format!("bad file id in {url}"))?;

        let (mut key, mut expires, mut user_id) = (None, None, None);
        if let Some(q) = query {
            for pair in q.split('&') {
                let Some((k, v)) = pair.split_once('=') else { continue };
                match k {
                    "key" if !v.is_empty() => key = Some(v.to_string()),
                    "expires" => expires = v.parse().ok(),
                    "user_id" => user_id = v.parse().ok(),
                    _ => {}
                }
            }
        }
        Ok(NxmUrl { game: segs[0].to_ascii_lowercase(), mod_id, file_id, key, expires, user_id })
    }
}

/// The validated account (subset of `users/validate`).
#[derive(Debug, Clone)]
pub struct Account {
    pub name: String,
    pub user_id: u64,
    pub is_premium: bool,
}

/// Whether this account may be shown adult mod metadata.
///
/// Nexus keeps the answer on the account, not in the v1 API: it is
/// `preferences { adult }` on their GraphQL v2 endpoint, scoped to the bearer
/// token (see [`oauth::adult_preference`]). Vortex, Nexus's own manager, gates on
/// that preference alone and leaves age verification to the website, so Eidos
/// does the same rather than second-guessing who has verified what.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdultPolicy {
    /// The account has adult content switched on.
    Allowed,
    /// The account has it switched off.
    Denied,
    /// Not signed in, the preference could not be read, or the cached answer has
    /// aged out. Hides, exactly like [`AdultPolicy::Denied`], but says something
    /// different to the user: "we could not check" is not "you said no".
    #[default]
    Unknown,
}

impl AdultPolicy {
    /// The single place permission is decided. Only `Allowed` shows.
    pub fn shows_adult(self) -> bool {
        matches!(self, AdultPolicy::Allowed)
    }
}

/// Why a mod's metadata was withheld.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenReason {
    /// Adult mod; the account has adult content switched off.
    AdultDenied,
    /// Adult mod; the account's preference could not be read.
    AdultUnknown,
    /// The payload carried no content rating, so it is treated as adult. Kept
    /// distinct because "we could not confirm this mod's rating" and "your
    /// account hides adult content" have different fixes - and because if the
    /// field ever moves, EVERY mod lands here, which is a signal worth reading.
    RatingUnknown,
    /// Nexus reports the mod as unavailable (hidden or deleted upstream).
    Unavailable,
}

/// The title shown in place of a withheld mod. A constant, never anything
/// derived from the payload.
pub const HIDDEN_TITLE: &str = "Adult content (hidden)";

impl HiddenReason {
    /// What to tell the user, in words that leak nothing about the mod.
    pub fn message(self) -> &'static str {
        match self {
            HiddenReason::AdultDenied => {
                "Hidden because adult content is turned off on your Nexus account. Change it at \
                 https://next.nexusmods.com/settings/content-blocking, then sign in again in Eidos."
            }
            HiddenReason::AdultUnknown => {
                "Hidden: Eidos could not confirm your Nexus content settings. Sign in again to retry."
            }
            HiddenReason::RatingUnknown => "Hidden: Eidos could not confirm this mod's content rating.",
            HiddenReason::Unavailable => "This mod is no longer available on Nexus Mods.",
        }
    }
}

/// Proof that a mod's rating has been resolved and found showable.
///
/// The fields are private and there is no public constructor, so the only way to
/// hold one is to have called [`Nexus::mod_info`] - which is what makes it
/// impossible to reach [`Nexus::file_info`] or [`Nexus::download_link`] without
/// having passed the gate first. The compiler enforces the ordering that a
/// convention would only describe.
#[derive(Debug, Clone, Copy)]
pub struct ModGate {
    adult: bool,
    hidden: Option<HiddenReason>,
}

impl ModGate {
    /// Whether this mod's metadata may be shown.
    pub fn visible(&self) -> bool {
        self.hidden.is_none()
    }
    /// Why it may not be, if it may not.
    pub fn reason(&self) -> Option<HiddenReason> {
        self.hidden
    }
    /// Whether Nexus flags this mod as adult (independent of the account's setting).
    pub fn is_adult(&self) -> bool {
        self.adult
    }
}

/// What an MD5 lookup recovered about an archive with no sidecar.
#[derive(Debug, Clone)]
pub struct Md5Match {
    pub mod_id: u64,
    pub file_id: u64,
    /// The file's display name on Nexus ("Main file"), which is what the
    /// sidecar's `name=` key holds.
    pub file_name: String,
    /// The archive's own name as Nexus knows it, for the `fileName=` key.
    pub archive_name: String,
    pub file_version: String,
    pub remote: RemoteMod,
}

/// Remote mod metadata (subset of `games/{game}/mods/{id}`).
///
/// When [`Self::hidden`] is set, the descriptive fields have already been blanked
/// inside [`Nexus::mod_info`]: a withheld mod's name and summary never leave this
/// crate, so no display site can leak one by forgetting to check. Only `version`
/// survives, because it is what the update check compares and a version string
/// describes nothing.
#[derive(Debug, Clone)]
pub struct RemoteMod {
    pub name: String,
    pub version: String,
    pub summary: String,
    pub category_id: Option<u64>,
    pub available: bool,
    /// Nexus's own rating: `Some(true)` adult, `Some(false)` not, `None` when the
    /// payload said nothing - which is treated as adult.
    pub adult: Option<bool>,
    /// The capability token [`Nexus::file_info`] and [`Nexus::download_link`]
    /// require, and the single answer to "was this withheld, and why"
    /// ([`ModGate::visible`] / [`ModGate::reason`]).
    pub gate: ModGate,
}

impl RemoteMod {
    /// Apply the gate to a `games/{game}/mods/{id}` payload.
    ///
    /// Split out from [`Nexus::mod_info`], which is that one request followed by
    /// this one decision, so the decision can be tested against real payloads
    /// without a network - and so there is exactly one copy of it to audit.
    ///
    /// Fail closed at every step: a payload with no rating is treated as adult,
    /// and an unreadable account preference hides rather than shows. Every way
    /// this can be wrong ends with too little on screen, never too much.
    pub(crate) fn from_payload(v: &serde_json::Value, policy: AdultPolicy) -> RemoteMod {
        let adult = adult_flag(v);
        let available = v.get("available").and_then(|x| x.as_bool()).unwrap_or(true);
        let hidden = match (adult, policy, available) {
            // Unavailable outranks the rest: Nexus has taken the page down, so
            // there is nothing to show whatever the account allows.
            (_, _, false) => Some(HiddenReason::Unavailable),
            (Some(false), _, _) => None,
            (Some(true), AdultPolicy::Allowed, _) => None,
            (Some(true), AdultPolicy::Denied, _) => Some(HiddenReason::AdultDenied),
            (Some(true), AdultPolicy::Unknown, _) => Some(HiddenReason::AdultUnknown),
            (None, _, _) => Some(HiddenReason::RatingUnknown),
        };

        let redact = hidden.is_some();
        RemoteMod {
            // Blanked rather than replaced with the placeholder: a caller that
            // prints this without checking shows nothing, not a wrong name.
            name: if redact { String::new() } else { s(v, "name") },
            // Kept either way. A version string describes nothing, and comparing
            // it is the whole point of the update check - withholding it would
            // break update detection for a mod the user already has installed.
            version: s(v, "version"),
            summary: if redact { String::new() } else { s(v, "summary") },
            category_id: if redact { None } else { v.get("category_id").and_then(|x| x.as_u64()) },
            available,
            adult,
            gate: ModGate { adult: adult.unwrap_or(true), hidden },
        }
    }

    /// Whether this mod's metadata was withheld.
    pub fn hidden(&self) -> Option<HiddenReason> {
        self.gate.reason()
    }
    /// What to show in a list in place of the real name: the mod's name when it
    /// may be shown, the neutral placeholder when it may not.
    pub fn display_name(&self) -> &str {
        if self.gate.visible() {
            &self.name
        } else {
            HIDDEN_TITLE
        }
    }
}

/// Remote file metadata (subset of `.../files/{file_id}`).
#[derive(Debug, Clone)]
pub struct RemoteFile {
    /// Display name of the file entry.
    pub name: String,
    /// The actual archive file name (what lands on disk).
    pub file_name: String,
    pub version: String,
    pub mod_version: String,
    pub category_id: Option<u64>,
    pub description: String,
    /// Total bytes, so a download can be shown as a percentage from the first
    /// instant rather than as a byte count with no end in sight.
    pub size_in_bytes: u64,
}

/// Nexus rate-limit budget from the `X-RL-*` response headers. Fields are `None`
/// until a reply carrying them is seen, and `None` always means "go ahead": a
/// fresh client has no headers yet, and the request that would teach it the
/// budget must be allowed out or the client deadlocks before it ever starts.
#[derive(Debug, Clone, Copy, Default)]
pub struct RateLimits {
    pub hourly_remaining: Option<i64>,
    pub daily_remaining: Option<i64>,
    /// Unix seconds at which a spent hourly budget refills. COMPUTED from the
    /// UTC hour boundary, not parsed from `X-RL-Hourly-Reset`: the two reset
    /// headers are documented in different formats (`2019-02-01T12:00:00+00:00`
    /// against `2019-02-02 00:00:00 +0000`), and Nexus documents the rule itself
    /// ("the hourly quota resets each hour", "daily at 00:00 GMT"), so deriving
    /// the boundary needs no date parser and no new dependency.
    pub hourly_reset: Option<u64>,
    /// Unix seconds at which a spent daily budget refills (next 00:00 UTC).
    pub daily_reset: Option<u64>,
    /// Set by a 429 that arrived while the budget still showed requests left -
    /// i.e. the separate burst guard in front of the API, not the quota. Short
    /// back-off rather than idling until the next hour.
    pub blocked_until: Option<u64>,
}

/// The next UTC hour boundary at or after `now`.
fn next_hour_utc(now: u64) -> u64 {
    (now / 3600 + 1) * 3600
}

/// The next UTC midnight at or after `now`.
fn next_midnight_utc(now: u64) -> u64 {
    (now / 86_400 + 1) * 86_400
}

/// How long to stand down after a 429 that the quota does not explain.
const BURST_BACKOFF: u64 = 60;

/// The one phrase every rate-limit refusal carries, whether it came from a 429
/// or from the pre-flight check that stopped a request being sent at all.
///
/// It exists because the callers used to test for the condition themselves, and
/// disagreed: `check_updates` matched `"rate limited"` while the CLI matched
/// `"429"`, so a pre-flight refusal - which contains no status code - would have
/// stopped one loop and left the other hammering an exhausted account. Both now
/// route through [`is_rate_limited`], which cannot drift from the messages.
pub const RATE_LIMITED: &str = "rate limited by Nexus";

/// Whether an error string is one of ours meaning "the budget is spent".
pub fn is_rate_limited(err: &str) -> bool {
    err.contains(RATE_LIMITED)
}

/// What [`Nexus::connect`] decided to do, separated from doing it so the rule
/// can be tested without a network, a clock or a config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CredentialChoice {
    /// A stored access token that is still fresh.
    Bearer,
    /// A stored session whose access token needs renewing first.
    Refresh,
    /// No usable session. There is no fallback: without an OAuth session there
    /// is no request Eidos is allowed to make.
    None,
}

/// Five minutes of skew, matching MO2: a token that expires mid-request is a
/// failure the user cannot act on, so treat "nearly expired" as expired.
const TOKEN_SKEW: Duration = Duration::from_secs(300);

fn choose_credential(
    creds: &eidos_instance::settings::NexusCreds,
    now: u64,
    can_refresh: bool,
) -> CredentialChoice {
    if creds.has_oauth() {
        let stale = creds.expires_at <= now.saturating_add(TOKEN_SKEW.as_secs());
        if !stale {
            return CredentialChoice::Bearer;
        }
        // Renewing needs BOTH a refresh token and a registered client_id. Without
        // the client_id there is no request we are allowed to make.
        let renewable = can_refresh && creds.refresh_token.as_deref().is_some_and(|r| !r.is_empty());
        if renewable {
            return CredentialChoice::Refresh;
        }
    }
    CredentialChoice::None
}

/// How a request proves who it is.
///
/// One variant, on purpose. It was an enum over `{ApiKey, Bearer}` until Nexus
/// required personal API keys gone from the client entirely; the shape is kept
/// so the header logic stays in one named place rather than inlined at every
/// call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    /// An OAuth access token, as `Authorization: Bearer <token>`.
    Bearer(String),
}

/// The Nexus v1 API client.
pub struct Nexus {
    agent: ureq::Agent,
    credential: Credential,
    limits: std::cell::Cell<RateLimits>,
    /// Whether this account may be shown adult metadata. Defaults to
    /// [`AdultPolicy::Unknown`], so a client built without stating a policy
    /// hides adult content rather than leaking it - forgetting is safe.
    adult: AdultPolicy,
}

fn s(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

/// Nexus's adult rating for a mod payload, or `None` when the payload says
/// nothing - which the caller must read as "assume adult", not as "safe".
///
/// `contains_adult_content` is the v1 spelling (node-nexus-api's `IModInfo`, and
/// what a captured v1 payload carries). The other two are how the newer APIs name
/// the same thing; accepting them costs a comparison and means a payload in a
/// slightly different shape still rates correctly instead of hiding every mod.
fn adult_flag(v: &serde_json::Value) -> Option<bool> {
    ["contains_adult_content", "adult_content", "adultContent"]
        .iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_bool()))
}

/// The `version` an endorsement request must carry: the mod's installed version,
/// trimmed, or `"1.0"` when it is blank (Nexus rejects an empty version, but
/// tolerates a placeholder for mods with no recorded version).
fn endorse_version(version: &str) -> &str {
    let v = version.trim();
    if v.is_empty() {
        "1.0"
    } else {
        v
    }
}

impl Nexus {
    /// A client authenticating with an OAuth access token.
    ///
    /// The caller is responsible for handing over a token that is still valid -
    /// see `oauth::Tokens::is_expired` and `oauth::refresh`. A stale token comes
    /// back as a 401 here, which reads as "signed out" rather than as an error
    /// worth retrying.
    pub fn with_bearer(access_token: &str) -> Nexus {
        Nexus::with_credential(Credential::Bearer(access_token.trim().to_string()))
    }

    /// The same client, told what the account allows. Adult metadata is withheld
    /// unless this says [`AdultPolicy::Allowed`].
    pub fn with_adult_policy(mut self, adult: AdultPolicy) -> Nexus {
        self.adult = adult;
        self
    }

    /// What this client believes the account allows.
    pub fn adult_policy(&self) -> AdultPolicy {
        self.adult
    }

    pub fn with_credential(credential: Credential) -> Nexus {
        // Read/write timeouts detect a stalled connection (which would otherwise
        // hang a task forever and leave GUI buttons greyed) without capping the
        // total duration of a legitimately long download.
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(format!(
                "Eidos/{} (Linux {})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::ARCH
            ))
            .timeout_connect(Some(std::time::Duration::from_secs(10)))
            .timeout_recv_body(Some(std::time::Duration::from_secs(30)))
            .timeout_send_body(Some(std::time::Duration::from_secs(30)))
            // A non-2xx reply comes back as a RESPONSE, not an error. Nexus puts
            // the rate-limit budget in headers on a rejection too - a 429 above
            // all, the one we most need to read - and ureq 2 handed us the
            // response inside Error::Status where ureq 3 does not, so asking for
            // the response directly is how the budget stays visible. (Not on
            // EVERY reply, despite what this comment used to claim: a 401 from
            // the v1 API carries no X-RL-* headers at all, which is why
            // `capture_limits` merges instead of overwriting.)
            .http_status_as_error(false)
            .build()
            .into();
        Nexus {
            agent,
            credential,
            limits: std::cell::Cell::new(RateLimits::default()),
            adult: AdultPolicy::Unknown,
        }
    }

    /// Which credential this client carries. Exposed so a caller can report the
    /// state without holding the secret itself.
    pub fn credential_kind(&self) -> &'static str {
        match self.credential {
            Credential::Bearer(_) => "oauth",
        }
    }

    /// Whether a signed-in session is stored, without touching the network.
    ///
    /// Reads a file, so it is safe to call from a UI update handler - unlike
    /// [`Nexus::connect`], which may spend a round trip renewing a token and
    /// belongs in a background task.
    pub fn have_credentials() -> bool {
        eidos_instance::settings::load_nexus_creds().has_oauth()
    }

    /// A signed-in client that knows what the account allows.
    ///
    /// The adult-content preference is cached in `nexus.ini` with a TTL rather
    /// than fetched per request: it is one extra call a day, and the setting is
    /// the user's own, changed rarely and on the website. A read that fails for
    /// ANY reason leaves the policy `Unknown`, which hides - so being offline
    /// costs the user adult metadata for that session, never the reverse.
    fn signed_in(creds: &mut eidos_instance::settings::NexusCreds, token: &str) -> Nexus {
        let now = oauth::now_unix();
        let known = creds.adult_pref(now).or_else(|| {
            let fetched = oauth::adult_preference(token)?;
            creds.adult_ok = Some(fetched);
            creds.adult_checked_at = now;
            // A failed write only costs the next session another lookup.
            let _ = eidos_instance::settings::save_nexus_creds(creds);
            Some(fetched)
        });
        let policy = match known {
            Some(true) => AdultPolicy::Allowed,
            Some(false) => AdultPolicy::Denied,
            None => AdultPolicy::Unknown,
        };
        Nexus::with_bearer(token).with_adult_policy(policy)
    }

    /// The client to use right now, from whatever is stored on this machine.
    ///
    /// Uses the signed-in OAuth session, renewing the access token first when it
    /// is stale. There is no personal-API-key fallback: a session that cannot be
    /// renewed is an error saying so, because a distributed client is not allowed
    /// to carry a personal key at all.
    pub fn connect() -> Result<Nexus, String> {
        let mut creds = eidos_instance::settings::load_nexus_creds();
        let cfg = oauth::Config::from_env();
        match choose_credential(&creds, oauth::now_unix(), cfg.is_some()) {
            CredentialChoice::Bearer => {
                let token = creds.access_token.clone().unwrap_or_default();
                Ok(Nexus::signed_in(&mut creds, &token))
            }
            CredentialChoice::Refresh => {
                let cfg = cfg.expect("choose_credential only picks Refresh when a config exists");
                let refresh = creds.refresh_token.clone().unwrap_or_default();
                match oauth::refresh(&cfg, &refresh) {
                    Ok(t) => {
                        creds.access_token = Some(t.access_token.clone());
                        creds.refresh_token = Some(t.refresh_token);
                        creds.expires_at = t.expires_at;
                        // A failed write is not fatal: the token in hand still
                        // works, the user just signs in again next session.
                        let _ = eidos_instance::settings::save_nexus_creds(&creds);
                        Ok(Nexus::signed_in(&mut creds, &t.access_token))
                    }
                    // The refresh token is spent or revoked, and there is
                    // nothing to fall back to: signing in again is the only
                    // route, which is what the message has to say.
                    Err(e) => Err(format!("Nexus sign-in expired and could not be renewed: {e}")),
                }
            }
            CredentialChoice::None => {
                Err("not connected to Nexus: sign in from Settings".to_string())
            }
        }
    }

    /// The most recent rate-limit budget seen (MO2's `X-RL-Hourly/Daily-Remaining`).
    pub fn rate_limits(&self) -> RateLimits {
        self.limits.get()
    }

    /// Record the budget a reply carried.
    ///
    /// A field is only overwritten when its header is actually PRESENT. Not every
    /// reply carries them - a 401 from the v1 API carries none at all - and the
    /// earlier version rebuilt the whole struct from each reply, so one such
    /// answer wiped a known-exhausted budget back to "unknown" and the pre-flight
    /// check below would wave the next request straight through.
    fn capture_limits(&self, resp: &ureq::http::Response<ureq::Body>) {
        let parse = |h: &str| {
            resp.headers().get(h).and_then(|v| v.to_str().ok()).and_then(|x| x.trim().parse::<i64>().ok())
        };
        let now = oauth::now_unix();
        let mut lim = self.limits.get();
        if let Some(h) = parse("X-RL-Hourly-Remaining") {
            lim.hourly_remaining = Some(h);
            lim.hourly_reset = (h <= 0).then(|| next_hour_utc(now));
        }
        if let Some(d) = parse("X-RL-Daily-Remaining") {
            lim.daily_remaining = Some(d);
            lim.daily_reset = (d <= 0).then(|| next_midnight_utc(now));
        }
        self.limits.set(lim);
    }

    /// A 429 the quota does not explain is the burst guard sitting in front of
    /// the API (it rejects sustained bursts regardless of budget). Stand down for
    /// a minute rather than idling until the next hour, and never synthesise a
    /// zero into the counters - they are what the server told us.
    fn note_rejection(&self, code: u16) {
        if code != 429 {
            return;
        }
        let mut lim = self.limits.get();
        let spent = |v: Option<i64>| v.is_some_and(|n| n <= 0);
        if !spent(lim.hourly_remaining) && !spent(lim.daily_remaining) {
            lim.blocked_until = Some(oauth::now_unix() + BURST_BACKOFF);
            self.limits.set(lim);
        }
    }

    /// Whether a request may be sent right now, or the reason it may not.
    ///
    /// Nexus's own rule is that an account is refused only once BOTH buckets are
    /// spent, which is why MO2, node-nexus-api and Wabbajack all track the larger
    /// of the two. Their reviewer asked for the stricter reading - stop as soon as
    /// EITHER counter reaches zero - so that is what this implements, at the cost
    /// of idling while the other bucket still has room.
    ///
    /// An exhausted bucket clears itself on the clock: past the boundary the
    /// counter goes back to "unknown" and the next request re-learns the real
    /// budget from its own headers. No probe request is needed, which is why
    /// `users/validate` is not exempted from this check even though it is said
    /// not to count against the hourly quota - issuing any request while we
    /// believe the account is exhausted is the behaviour being corrected.
    fn preflight(&self, now: u64) -> Option<String> {
        let mut lim = self.limits.get();
        let mut expired = false;

        if lim.blocked_until.is_some_and(|t| now >= t) {
            lim.blocked_until = None;
            expired = true;
        }
        if lim.hourly_reset.is_some_and(|t| now >= t) {
            (lim.hourly_remaining, lim.hourly_reset) = (None, None);
            expired = true;
        }
        if lim.daily_reset.is_some_and(|t| now >= t) {
            (lim.daily_remaining, lim.daily_reset) = (None, None);
            expired = true;
        }
        if expired {
            self.limits.set(lim);
        }

        if lim.blocked_until.is_some() {
            return Some(format!("{RATE_LIMITED} - too many requests too quickly; retrying shortly"));
        }
        if lim.hourly_remaining.is_some_and(|n| n <= 0) {
            return Some(format!(
                "{RATE_LIMITED} - the hourly request budget is spent; it refills at the next hour (UTC)"
            ));
        }
        if lim.daily_remaining.is_some_and(|n| n <= 0) {
            return Some(format!(
                "{RATE_LIMITED} - the daily request budget is spent; it refills at 00:00 UTC"
            ));
        }
        None
    }

    /// Whether the next request would be refused by [`Nexus::preflight`]. For
    /// loops that check before building a request, so they stop visibly instead
    /// of relying on the choke point to reject each attempt in turn.
    pub fn would_block(&self) -> bool {
        self.preflight(oauth::now_unix()).is_some()
    }

    /// The MO2 status-code mapping shared by every request (401 = the session is
    /// not accepted, 429 = rate limited, else a generic message with the code).
    fn status_err(code: u16) -> String {
        match code {
            401 => "Nexus rejected the sign-in (401) - sign in again".to_string(),
            429 => format!("{RATE_LIMITED} (429) - try again later"),
            other => format!("Nexus API error (HTTP {other})"),
        }
    }

    /// Attach the identifying headers MO2 sends on every v1 call
    /// (nxmaccessmanager.cpp addAPIHeaders) plus the bearer token.
    ///
    /// `Application-Name`/`Application-Version` are not decoration: the Nexus
    /// API acceptable-use policy requires them so usage can be attributed.
    fn with_headers<B>(&self, req: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        let req = req
            .header("Protocol-Version", "1.0.0")
            .header("Application-Name", "Eidos")
            .header("Application-Version", env!("CARGO_PKG_VERSION"));
        match &self.credential {
            Credential::Bearer(token) => req.header("Authorization", format!("Bearer {token}")),
        }
    }

    fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        // Before anything is sent. Every read endpoint funnels through here, so
        // this one line is what makes "stop as soon as the budget is spent" true
        // of the whole client rather than of the loops that remembered to ask.
        if let Some(stop) = self.preflight(oauth::now_unix()) {
            return Err(stop);
        }
        let url = format!("{API_BASE}/{path}");
        // Every v1 request identifies the client, as MO2 does
        // (nxmaccessmanager.cpp addAPIHeaders): Protocol-Version plus
        // Application-Name/-Version, alongside the bearer token.
        match self.with_headers(self.agent.get(&url)).call() {
            Ok(mut resp) => {
                // Read the budget FIRST: a rejection carries it too, and a 429 is
                // exactly when the caller needs to know how long to wait. Not
                // every reply has it, which is why capture_limits merges.
                self.capture_limits(&resp);
                let code = resp.status().as_u16();
                if !resp.status().is_success() {
                    self.note_rejection(code);
                    return Err(Nexus::status_err(code));
                }
                resp.body_mut().read_json().map_err(|e| e.to_string())
            }
            Err(other) => Err(other.to_string()),
        }
    }

    /// Send a request with a `version` form field in the body, used by the
    /// endorsement endpoints (MO2's endorseMod posts `version=<installed>`). The
    /// reply body is ignored - only success/failure matters. Captures the X-RL-*
    /// budget on success and on a Status error, exactly like [`get`].
    fn send_with_version(&self, req: ureq::RequestBuilder<ureq::typestate::WithBody>, version: &str) -> Result<(), String> {
        if let Some(stop) = self.preflight(oauth::now_unix()) {
            return Err(stop);
        }
        match self.with_headers(req).send_form([("version", version)]) {
            Ok(resp) => {
                self.capture_limits(&resp);
                if resp.status().is_success() {
                    Ok(())
                } else {
                    let code = resp.status().as_u16();
                    self.note_rejection(code);
                    Err(Nexus::status_err(code))
                }
            }
            Err(other) => Err(other.to_string()),
        }
    }

    /// Endorse a mod: `POST games/{game}/mods/{id}/endorse` with the installed
    /// `version` (MO2's endorseMod). Nexus requires the version in the body; pass
    /// the mod's installed version, falling back to `"1.0"` when it is unknown.
    pub fn endorse(&self, game: &str, mod_id: u64, version: &str) -> Result<(), String> {
        let url = format!("{API_BASE}/games/{game}/mods/{mod_id}/endorse");
        self.send_with_version(self.agent.post(&url), endorse_version(version))
    }

    /// Abstain from endorsing a mod: `POST games/{game}/mods/{id}/abstain`
    /// (MO2's "Won't endorse" / un-endorse). Same `version` requirement as
    /// [`endorse`].
    pub fn abstain(&self, game: &str, mod_id: u64, version: &str) -> Result<(), String> {
        let url = format!("{API_BASE}/games/{game}/mods/{mod_id}/abstain");
        self.send_with_version(self.agent.post(&url), endorse_version(version))
    }

    /// Endorse when `endorse` is true, abstain when false - the toggling action the
    /// GUI binds to its Endorse button (it reads the mod's current `endorsed()`
    /// state to pick the direction). Returns the resulting endorsed state on success.
    pub fn set_endorsed(&self, game: &str, mod_id: u64, version: &str, endorse: bool) -> Result<bool, String> {
        if endorse {
            self.endorse(game, mod_id, version)?;
        } else {
            self.abstain(game, mod_id, version)?;
        }
        Ok(endorse)
    }

    /// Validate the key: `users/validate`.
    pub fn validate(&self) -> Result<Account, String> {
        let v = self.get("users/validate")?;
        Ok(Account {
            name: s(&v, "name"),
            user_id: v.get("user_id").and_then(|x| x.as_u64()).unwrap_or(0),
            is_premium: v.get("is_premium").and_then(|x| x.as_bool()).unwrap_or(false),
        })
    }

    /// Mod metadata: `games/{game}/mods/{id}` (the update check reads `version`).
    ///
    /// THE GATE. This is the only place a mod's rating is known, so it is the
    /// only place it can be enforced: the descriptive fields are blanked here,
    /// before the struct exists, and the [`ModGate`] minted alongside them is what
    /// every later call demands. Anything added to [`RemoteMod`] that comes from
    /// this payload must be redacted in [`redact`] too - there is no second gate
    /// downstream to catch it.
    pub fn mod_info(&self, game: &str, mod_id: u64) -> Result<RemoteMod, String> {
        let v = self.get(&format!("games/{game}/mods/{mod_id}"))?;
        Ok(RemoteMod::from_payload(&v, self.adult))
    }

    /// Identify an archive Eidos did not download, by its MD5:
    /// `games/{game}/mods/md5_search/{md5}`.
    ///
    /// Every archive dropped into `downloads/` by hand - moved from another
    /// machine, fetched in a browser, recovered from a backup - arrives with no
    /// `.meta` beside it, and stays "Untracked" forever: no mod id, so no
    /// version, no update check, no Nexus page. This is the recovery path, and
    /// it is the same one MO2 offers as Query Metadata.
    ///
    /// The reply is an ARRAY: one MD5 can match several files (a mod republished
    /// under a new file id keeps the bytes). The first match is taken, as MO2
    /// does - they describe the same file, and the alternative is asking the user
    /// a question they have no way to answer.
    pub fn md5_search(&self, game: &str, md5: &str) -> Result<Md5Match, String> {
        let v = self.get(&format!("games/{game}/mods/md5_search/{md5}"))?;
        let first = v
            .as_array()
            .and_then(|a| a.first())
            .ok_or("Nexus knows no file with that checksum")?;
        let m = first.get("mod").ok_or("the reply carried no mod")?;
        let f = first.get("file_details").ok_or("the reply carried no file")?;
        let remote = RemoteMod::from_payload(m, self.adult);
        // A withheld mod is withheld here too: the gate exists so metadata for a
        // hidden or adult-gated mod cannot arrive through a side door.
        if let Some(why) = remote.gate.reason() {
            return Err(why.message().to_string());
        }
        // `mod_id` / `file_id` are what make the row tracked at all: without
        // them there is no update check and no Nexus page, and the sidecar this
        // writes would REMOVE the Identify button that is the only way to try
        // again. A reply missing either is a failed identification, not a
        // partial one.
        let (Some(mod_id), Some(file_id)) = (
            m.get("mod_id").and_then(|x| x.as_u64()),
            f.get("file_id").and_then(|x| x.as_u64()),
        ) else {
            return Err("Nexus answered without a mod or file id".to_string());
        };
        Ok(Md5Match {
            mod_id,
            file_id,
            // `name` is the file's DISPLAY name ("Main file"); `file_name` is
            // the archive on disk. The sidecar's `name=` key is the former -
            // MO2 shows it as the download's title.
            file_name: s(f, "name"),
            archive_name: s(f, "file_name"),
            file_version: s(f, "version"),
            remote,
        })
    }

    /// File metadata: `games/{game}/mods/{id}/files/{file_id}`.
    ///
    /// Takes the [`ModGate`] minted by [`Nexus::mod_info`], so a file's name and
    /// description cannot be fetched for a mod whose metadata is withheld: the
    /// caller has to have looked the mod up first, and this refuses if that lookup
    /// closed the gate.
    pub fn file_info(&self, gate: &ModGate, game: &str, mod_id: u64, file_id: u64) -> Result<RemoteFile, String> {
        if let Some(why) = gate.reason() {
            return Err(why.message().to_string());
        }
        let v = self.get(&format!("games/{game}/mods/{mod_id}/files/{file_id}"))?;
        Ok(RemoteFile {
            name: s(&v, "name"),
            file_name: s(&v, "file_name"),
            version: s(&v, "version"),
            mod_version: s(&v, "mod_version"),
            category_id: v.get("category_id").and_then(|x| x.as_u64()),
            description: s(&v, "description"),
            size_in_bytes: v
                .get("size_in_bytes")
                .and_then(|x| x.as_u64())
                // Older payloads carry only the rounded kilobyte figure. It is
                // approximate, which is fine for a progress bar and wrong for
                // anything that compares sizes - so nothing else uses it.
                .or_else(|| v.get("size_in_kb").and_then(|x| x.as_u64()).map(|kb| kb * 1024))
                .unwrap_or(0),
        })
    }

    /// Mod ids updated in the last period (`1d`/`1w`/`1m`) - MO2 uses this to
    /// avoid querying every installed mod on an update check.
    pub fn updated_mod_ids(&self, game: &str, period: &str) -> Result<Vec<u64>, String> {
        let v = self.get(&format!("games/{game}/mods/updated?period={period}"))?;
        Ok(v.as_array()
            .map(|a| a.iter().filter_map(|m| m.get("mod_id").and_then(|x| x.as_u64())).collect())
            .unwrap_or_default())
    }

    /// Resolve the CDN download URI for an `nxm://` link: `download_link`
    /// (+`?key=..&expires=..` for non-premium, exactly like MO2). Returns the
    /// first mirror's URI.
    pub fn download_link(&self, gate: &ModGate, nxm: &NxmUrl) -> Result<String, String> {
        // Same token as `file_info`: a mod whose page Eidos may not describe is
        // one whose files Eidos does not fetch either. The link itself carries no
        // description, but resolving it is the first step of a flow that goes on
        // to print the file name and write it into a `.meta` sidecar.
        if let Some(why) = gate.reason() {
            return Err(why.message().to_string());
        }
        let mut path = format!(
            "games/{}/mods/{}/files/{}/download_link",
            nxm.game, nxm.mod_id, nxm.file_id
        );
        if let (Some(key), Some(expires)) = (&nxm.key, nxm.expires) {
            path.push_str(&format!("?key={key}&expires={expires}"));
        }
        let v = self.get(&path)?;
        v.as_array()
            .and_then(|a| a.first())
            .map(|m| s(m, "URI"))
            .filter(|u| !u.is_empty())
            .ok_or_else(|| {
                "no download mirror returned (free accounts need a fresh nxm:// link \
                 from the site's Mod Manager Download button)"
                    .to_string()
            })
    }

    /// Stream a (non-API) CDN URL to `dest`. Returns the total byte count.
    ///
    /// Resumes an interrupted download: if a `<dest>.unfinished` partial is present
    /// it sends `Range: bytes=<len>-` and appends (MO2 Range-resumes the same
    /// marker). A server that ignores the range answers `200` instead of `206`, so
    /// the partial is truncated and the download restarts cleanly.
    pub fn download(&self, url: &str, dest: &Path) -> Result<u64, String> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = unfinished_path(dest); // MO2's in-progress marker (appended, keeps ext)
        let have = fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);

        let mut req = self.agent.get(url);
        if have > 0 {
            req = req.header("Range", format!("bytes={have}-"));
        }
        let resp = req.call().map_err(|e| e.to_string())?;
        // With http_status_as_error off, a rejection arrives here as a response;
        // downloading an HTML error page into the .unfinished file would look
        // like a resumable partial on the next attempt.
        if !resp.status().is_success() {
            return Err(Nexus::status_err(resp.status().as_u16()));
        }

        // 206 Partial Content = the server honoured the range, so append; any other
        // status (200) means it sent the whole file - restart from byte 0.
        let resuming = have > 0 && resp.status() == 206;
        let mut out = if resuming {
            fs::OpenOptions::new().append(true).open(&tmp)
        } else {
            fs::File::create(&tmp)
        }
        .map_err(|e| e.to_string())?;

        let mut reader = resp.into_body().into_reader();
        let n = copy_stream(&mut reader, &mut out).map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
        Ok(if resuming { have + n } else { n })
    }
}

/// One mod whose installed version is behind the latest seen on Nexus, surfaced by
/// [`check_updates`] so the GUI can list exactly what changed without re-reading
/// every `meta.ini`.
#[derive(Debug, Clone)]
pub struct ModUpdate {
    /// The mods/ folder name (the GUI maps this back to its row).
    pub name: String,
    /// The installed version recorded in `meta.ini` (may be empty).
    pub installed: String,
    /// The newest version reported by Nexus.
    pub latest: String,
}

/// The outcome of a GUI-triggered update check across an instance's mods. Mirrors
/// the CLI summary so the GUI status bar can report the same numbers, plus the
/// per-mod list of mods now behind.
#[derive(Debug, Clone, Default)]
pub struct UpdateCheckResult {
    /// Mods with a Nexus id that were considered.
    pub checked: u32,
    /// Of those, how many a remote query was actually issued for.
    pub queried: u32,
    /// Mods now behind the latest Nexus version (`updates.len()`).
    pub updates_found: u32,
    /// The mods now behind, with their versions.
    pub updates: Vec<ModUpdate>,
    /// True if the hourly budget was exhausted and the loop stopped early, leaving
    /// some mods unchecked (MO2 stops dispatching the moment the account is spent).
    pub rate_limited: bool,
    /// The Nexus hourly request budget remaining after the check, if a reply set it.
    pub hourly_remaining: Option<i64>,
    /// The Nexus daily request budget remaining after the check, if a reply set it.
    pub daily_remaining: Option<i64>,
}

/// Run a Nexus update check across every mod in `inst` (the GUI-callable port of
/// `eidos nexus update`). `nexus_game` is the game's Nexus domain
/// (`GameDef::nexus_game`, e.g. `skyrimspecialedition`).
///
/// MO2's strategy: one `updated?period=1m` bulk query, then an individual
/// `mod_info` only for mods in that window's intersection OR never/long-ago
/// checked (so an update published over a month ago is not missed on a fresh
/// instance). Each queried mod has `newestVersion` and `lastNexusUpdate` written
/// back to its `meta.ini`; the GUI re-reads `update_available()` afterward to
/// refresh its row markers. The loop stops on the first 429, returning what it had.
///
/// This is blocking I/O (`ureq`); the GUI runs it inside a `Task::perform` closure
/// on iced's executor, exactly like the plugin-sort task. The `Instance` is
/// `Clone`, so the closure can own a clone and complete harmlessly even if the
/// user switches instances mid-check.
pub fn check_updates(nexus: &Nexus, inst: &eidos_instance::Instance, nexus_game: &str) -> Result<UpdateCheckResult, String> {
    // One "updated this month" query, then only fetch the intersection - stays
    // inside the API rate limits (MO2's approach).
    let updated = match nexus.updated_mod_ids(nexus_game, "1m") {
        Ok(v) => v,
        // An exhausted budget is a state to report, not a failure: the caller
        // shows "the budget is spent" rather than an error toast, and nothing
        // about the mod list is wrong - it just could not be refreshed.
        Err(e) if is_rate_limited(&e) => {
            return Ok(UpdateCheckResult { rate_limited: true, ..Default::default() })
        }
        Err(e) => return Err(e),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    const MONTH: u64 = 30 * 24 * 3600;

    let mut result = UpdateCheckResult::default();
    for m in inst.modlist() {
        let mut meta = inst.mod_meta(&m.name);
        let Some(mod_id) = meta.mod_id() else { continue };
        result.checked += 1;

        // The `updated?period=1m` list is only trustworthy for mods checked within
        // that window. A mod never checked - or checked over a month ago - gets an
        // individual query regardless of the intersection, else an update published
        // more than a month ago is missed forever (the common first-run case).
        let stale = meta.last_nexus_update().map(|t| now.saturating_sub(t) > MONTH).unwrap_or(true);
        if !stale && !updated.contains(&mod_id) {
            continue;
        }
        // Stop before building the request, not after it is refused. The choke
        // point in `get` would reject it anyway; checking here is what makes the
        // loop visibly stop issuing requests once the budget is spent.
        if nexus.would_block() {
            result.rate_limited = true;
            break;
        }
        result.queried += 1;

        match nexus.mod_info(nexus_game, mod_id) {
            Ok(remote) => {
                meta.set_newest_version(&remote.version);
                meta.set_last_nexus_update(now);
                // A failed write is not fatal: the in-memory result is still correct,
                // the GUI just won't see the `^` marker persist across a restart.
                let _ = meta.write(&inst.meta_path(&m.name));
                if meta.update_available() {
                    result.updates.push(ModUpdate {
                        name: m.name.clone(),
                        installed: meta.version().unwrap_or_default(),
                        latest: remote.version,
                    });
                }
            }
            Err(e) => {
                // Stop dispatching the moment the account is exhausted. The test
                // is the shared predicate, not a bare "429": a pre-flight refusal
                // carries no status code, and a mod id or file size that happens
                // to contain "429" must not trip it.
                if is_rate_limited(&e) {
                    result.rate_limited = true;
                    break;
                }
                // A single mod's failure (deleted page, transient error) must not
                // abort the whole check - skip it and keep going, like the CLI.
            }
        }
    }

    result.updates_found = result.updates.len() as u32;
    let rl = nexus.rate_limits();
    result.hourly_remaining = rl.hourly_remaining;
    result.daily_remaining = rl.daily_remaining;
    Ok(result)
}

/// MO2 appends `.unfinished` to the FULL archive name (`Mod-1.0.7z.unfinished`),
/// keeping the real extension so a leftover partial maps back to its target and
/// two files differing only by extension (`Foo.7z` vs `Foo.zip`) don't collide.
/// (`Path::with_extension` would instead REPLACE `.7z`, destroying it.)
/// Where a download lives while it is still running. Public because the window
/// identifies an in-flight download by this suffix - the transfer happens in
/// another process, so the file name is the only channel between them.
pub fn unfinished_path(dest: &Path) -> PathBuf {
    PathBuf::from(format!("{}.unfinished", dest.display()))
}

fn copy_stream(r: &mut dyn Read, w: &mut dyn Write) -> io::Result<u64> {
    let mut buf = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            return Ok(total);
        }
        w.write_all(&buf[..n])?;
        total += n as u64;
    }
}

/// The file name a CDN URI downloads to: the path's last segment, percent-decoded
/// and sanitized (an encoded `../` or `/` must not survive into a path join).
pub fn file_name_from_uri(uri: &str) -> Option<String> {
    let path = uri.split('?').next()?;
    let name = path.rsplit('/').next()?;
    sanitize_file_name(&percent_decode(name))
}

/// MO2's `sanitizeFileName` (uibase `filesystemutilities.cpp`): replace control
/// chars and the Windows-illegal set `\ / : * ? " < > |` with `_`, strip trailing
/// dots/spaces, and reject the empty / `.` / `..` results (caller falls back to a
/// synthetic name). Keeps an untrusted CDN/API name from escaping the downloads dir.
pub fn sanitize_file_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim_end_matches(['.', ' ']).to_string();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        None
    } else {
        Some(cleaned)
    }
}

/// MO2's `getDownloadFileName` uniquification: if `dir/name` already exists, return
/// the first free `<i>_<name>` (i from 1), else `name` - so re-downloading a file
/// already on disk never silently clobbers it.
pub fn unique_download_name(dir: &Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }
    (1..)
        .map(|i| format!("{i}_{name}"))
        .find(|c| !dir.join(c).exists())
        .unwrap_or_else(|| name.to_string())
}

/// Flip a download's `.meta` sidecar to `installed=true` / `uninstalled=false`
/// after the archive is installed (MO2's `markInstalled`), so the Downloads view
/// and a shared MO2 downloads folder both see it installed. No-op if no sidecar.
pub fn mark_installed(archive: &Path) -> io::Result<()> {
    let meta_path = PathBuf::from(format!("{}.meta", archive.display()));
    let Ok(text) = fs::read_to_string(&meta_path) else {
        return Ok(());
    };
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let key = line.trim_start();
        if key.starts_with("installed=") {
            out.push_str("installed=true\n");
        } else if key.starts_with("uninstalled=") {
            out.push_str("uninstalled=false\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    fs::write(&meta_path, out)
}

fn percent_decode(sname: &str) -> String {
    // Pure byte-level decoding: slicing the &str (`&sname[i+1..i+3]`) would panic
    // when a multi-byte UTF-8 character follows a stray '%' in a CDN URI.
    fn hex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
    let b = sname.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(hi), Some(lo)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Write the MO2-format `.meta` sidecar next to a downloaded archive, with the
/// exact key set MO2's download manager writes (minus the Qt binary blobs).
/// `game_short` is MO2's gameShortName (e.g. `SkyrimSE`), not the Nexus domain.
#[allow(clippy::too_many_arguments)]
pub fn write_download_meta(
    archive: &Path,
    game_short: &str,
    nxm: &NxmUrl,
    url: &str,
    file: &RemoteFile,
    remote_mod: &RemoteMod,
) -> io::Result<PathBuf> {
    // Defence in depth, and the reason the gate had to sit upstream of this
    // function rather than at the display sites: what goes in here is written to
    // disk, and `modName` goes on to name the directory under `mods/`. A gate
    // that only guarded the screen could not un-write either. Callers refuse a
    // hidden mod long before reaching this point; if one ever does not, refuse
    // rather than persisting a description that may not be shown.
    if let Some(why) = remote_mod.hidden() {
        return Err(io::Error::other(why.message()));
    }
    let meta_path = PathBuf::from(format!("{}.meta", archive.display()));
    let mut out = String::from("[General]\n");
    out.push_str(&format!("gameName={game_short}\n"));
    out.push_str(&format!("modID={}\n", nxm.mod_id));
    out.push_str(&format!("fileID={}\n", nxm.file_id));
    out.push_str(&format!("url=\"{url}\"\n"));
    out.push_str(&format!("name={}\n", file.name));
    out.push_str(&format!("description={}\n", file.description.replace('\n', " ")));
    out.push_str(&format!("modName={}\n", remote_mod.name));
    out.push_str(&format!("version={}\n", file.version));
    out.push_str("newestVersion=\n");
    if let Some(c) = file.category_id {
        out.push_str(&format!("fileCategory={c}\n"));
    }
    if let Some(c) = remote_mod.category_id {
        out.push_str(&format!("category={c}\n"));
    }
    if file.size_in_bytes != 0 {
        out.push_str(&format!("totalSize={}\n", file.size_in_bytes));
    }
    out.push_str("repository=Nexus\n");
    out.push_str("installed=false\nuninstalled=false\npaused=false\nremoved=false\n");
    fs::write(&meta_path, out)?;
    Ok(meta_path)
}

/// The MD5 of a file, lowercase hex - the fingerprint Nexus indexes archives by.
///
/// Streamed in chunks rather than read whole: a mod archive is routinely
/// hundreds of megabytes and this runs on the GUI's behalf.
pub fn md5_file(path: &Path) -> io::Result<String> {
    use md5::{Digest, Md5};
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Write a `.meta` for an archive identified by [`Nexus::md5_search`].
///
/// The same MO2 key set [`write_download_meta`] writes, minus what only a live
/// download knows (the CDN url): what matters is that the row stops being
/// "Untracked" and gains its mod id, name and version, so update checks and the
/// Nexus page work from here on.
pub fn write_recovered_meta(
    archive: &Path,
    game_short: &str,
    found: &Md5Match,
) -> io::Result<PathBuf> {
    // Same defence as the download path: a withheld mod must not have its
    // metadata land on disk through this door either.
    if let Some(why) = found.remote.hidden() {
        return Err(io::Error::other(why.message()));
    }
    let meta_path = PathBuf::from(format!("{}.meta", archive.display()));
    let name = archive.file_name().unwrap_or_default().to_string_lossy();
    let mut out = String::from("[General]\n");
    out.push_str(&format!("gameName={game_short}\n"));
    out.push_str(&format!("modID={}\n", found.mod_id));
    out.push_str(&format!("fileID={}\n", found.file_id));
    out.push_str(&format!("name={}\n", found.file_name));
    out.push_str(&format!("modName={}\n", found.remote.name));
    out.push_str(&format!(
        "version={}\n",
        if found.file_version.is_empty() { &found.remote.version } else { &found.file_version }
    ));
    out.push_str(&format!("newestVersion={}\n", found.remote.version));
    out.push_str(&format!("fileName={name}\n"));
    if let Some(c) = found.remote.category_id {
        out.push_str(&format!("category={c}\n"));
    }
    if let Ok(md) = fs::metadata(archive) {
        out.push_str(&format!("totalSize={}\n", md.len()));
    }
    out.push_str("repository=Nexus\n");
    out.push_str("installed=false\nuninstalled=false\npaused=false\nremoved=false\n");
    fs::write(&meta_path, out)?;
    Ok(meta_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_md5_matches_the_rfc_1321_vectors() {
        // Published known answers. If this ever fails, every lookup returns "no
        // file with that checksum" and the reason will not point here.
        let dir = std::env::temp_dir().join(format!("eidos-md5-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        for (input, want) in [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ] {
            let f = dir.join("probe.bin");
            fs::write(&f, input).unwrap();
            assert_eq!(md5_file(&f).unwrap(), want, "MD5 of {input:?}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_withheld_mod_cannot_land_on_disk_through_the_md5_path_either() {
        // The gate exists so metadata for a hidden or adult-gated mod cannot
        // arrive through a side door. The download path asserts this; the
        // recovery path is the same door.
        let dir = std::env::temp_dir().join(format!("eidos-gate-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let archive = dir.join("x.zip");
        fs::write(&archive, b"x").unwrap();
        let hidden = gated(&mod_payload(Some(true)), AdultPolicy::Denied);
        assert!(hidden.hidden().is_some(), "the fixture really is withheld");
        let found = Md5Match {
            mod_id: 1,
            file_id: 2,
            file_name: "Main file".into(),
            archive_name: "x.zip".into(),
            file_version: "1.0".into(),
            remote: hidden,
        };
        assert!(write_recovered_meta(&archive, "SkyrimSE", &found).is_err());
        assert!(!dir.join("x.zip.meta").exists(), "nothing was written");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_recovered_meta_carries_what_makes_a_download_tracked() {
        // The point of the recovery: a row that was "Untracked" gains the mod
        // id and version that update checks and the Nexus page need.
        let dir = std::env::temp_dir().join(format!("eidos-recov-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let archive = dir.join("Cool Mod-123-1-0.zip");
        fs::write(&archive, b"payload").unwrap();
        let found = Md5Match {
            mod_id: 123,
            file_id: 456,
            file_name: "Main file".into(),
            archive_name: "Cool Mod-123-1-0.zip".into(),
            file_version: "1.0".into(),
            remote: RemoteMod {
                name: "Cool Mod".into(),
                version: "1.1".into(),
                summary: String::new(),
                category_id: Some(7),
                available: true,
                adult: Some(false),
                gate: shown_gate(),
            },
        };
        let meta = write_recovered_meta(&archive, "SkyrimSE", &found).unwrap();
        let body = fs::read_to_string(&meta).unwrap();
        assert!(body.contains("modID=123"), "{body}");
        assert!(body.contains("fileID=456"), "{body}");
        assert!(body.contains("version=1.0"), "the FILE version, not the mod's: {body}");
        assert!(body.contains("newestVersion=1.1"), "so the update check has a target: {body}");
        assert!(body.contains("name=Main file"), "the display name, not the archive: {body}");
        assert!(body.contains("fileName=Cool Mod-123-1-0.zip"), "{body}");
        assert!(body.contains("totalSize=7"), "{body}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// A gate for a mod that may be shown, for fixtures whose subject is not the
    /// gate itself.
    fn shown_gate() -> ModGate {
        ModGate { adult: false, hidden: None }
    }

    /// A mod payload as the v1 API returns it, with the adult flag under our
    /// control. `adult: None` omits the field entirely, which is the case that
    /// must be read as "assume adult".
    fn mod_payload(adult: Option<bool>) -> serde_json::Value {
        let mut v = serde_json::json!({
            "name": "Ivy the Companion",
            "version": "3.2",
            "summary": "A summary that must not leak.",
            "category_id": 42,
            "available": true,
        });
        if let Some(a) = adult {
            v["contains_adult_content"] = serde_json::Value::Bool(a);
        }
        v
    }

    /// The gate as the client applies it: the real function, on a real payload.
    fn gated(payload: &serde_json::Value, policy: AdultPolicy) -> RemoteMod {
        RemoteMod::from_payload(payload, policy)
    }

    // ---- The age gate ------------------------------------------------------

    #[test]
    fn an_adult_mod_is_redacted_when_the_account_has_not_opted_in() {
        let m = gated(&mod_payload(Some(true)), AdultPolicy::Denied);
        assert_eq!(m.hidden(), Some(HiddenReason::AdultDenied));
        assert!(m.name.is_empty() && m.summary.is_empty() && m.category_id.is_none());
        assert!(!m.gate.visible());
    }

    #[test]
    fn an_adult_mod_is_returned_in_full_when_the_account_has_opted_in() {
        let m = gated(&mod_payload(Some(true)), AdultPolicy::Allowed);
        assert_eq!(m.hidden(), None);
        assert_eq!(m.name, "Ivy the Companion");
        assert_eq!(m.category_id, Some(42));
        assert!(m.gate.visible() && m.gate.is_adult());
    }

    #[test]
    fn a_non_adult_mod_is_untouched_by_the_gate() {
        for policy in [AdultPolicy::Allowed, AdultPolicy::Denied, AdultPolicy::Unknown] {
            let m = gated(&mod_payload(Some(false)), policy);
            assert_eq!(m.hidden(), None, "{policy:?}");
            assert_eq!(m.name, "Ivy the Companion");
            assert!(!m.gate.is_adult());
        }
    }

    #[test]
    fn a_payload_with_no_rating_is_treated_as_adult() {
        // The field absent must not read as "safe". Its own reason, because if
        // the API ever moves the field EVERY mod lands here, and that has to be
        // distinguishable from the user's own setting.
        let m = gated(&mod_payload(None), AdultPolicy::Allowed);
        assert_eq!(m.hidden(), Some(HiddenReason::RatingUnknown));
        assert!(m.name.is_empty());
    }

    #[test]
    fn an_unknown_account_preference_hides_rather_than_shows() {
        let m = gated(&mod_payload(Some(true)), AdultPolicy::Unknown);
        assert_eq!(m.hidden(), Some(HiddenReason::AdultUnknown));
        assert!(m.name.is_empty());
    }

    #[test]
    fn a_client_built_without_a_policy_hides_adult_content() {
        // Forgetting to state a policy must fail closed, so the default is the
        // one that hides - not the one that shows.
        assert_eq!(Nexus::with_bearer("t").adult_policy(), AdultPolicy::Unknown);
        assert!(!AdultPolicy::default().shows_adult());
    }

    #[test]
    fn a_redacted_mod_keeps_its_version_so_the_update_check_still_works() {
        // The one field that survives redaction. A user with an adult mod already
        // installed must keep seeing that an update exists.
        let m = gated(&mod_payload(Some(true)), AdultPolicy::Denied);
        assert_eq!(m.version, "3.2");
    }

    #[test]
    fn the_placeholder_and_the_explanations_contain_nothing_from_the_payload() {
        let payload = mod_payload(Some(true));
        let m = gated(&payload, AdultPolicy::Denied);
        let shown = format!("{} {}", m.display_name(), m.hidden().unwrap().message());
        for leak in ["Ivy", "Companion", "summary that must not leak", "42"] {
            assert!(!shown.contains(leak), "{shown} leaked {leak}");
        }
        assert_eq!(m.display_name(), HIDDEN_TITLE);
    }

    #[test]
    fn the_denied_explanation_points_at_the_setting_that_fixes_it() {
        assert!(HiddenReason::AdultDenied
            .message()
            .contains("next.nexusmods.com/settings/content-blocking"));
    }

    #[test]
    fn an_unavailable_mod_is_withheld_whatever_the_account_allows() {
        let mut payload = mod_payload(Some(false));
        payload["available"] = serde_json::Value::Bool(false);
        let m = gated(&payload, AdultPolicy::Allowed);
        assert_eq!(m.hidden(), Some(HiddenReason::Unavailable));
        assert!(m.name.is_empty());
    }

    #[test]
    fn a_hidden_mod_yields_no_download_sidecar() {
        // The gate has to sit upstream of persistence: `modName` from this file
        // becomes a directory name under mods/, which no display-side check
        // could ever take back.
        let dir = std::env::temp_dir().join(format!("eidos-gate-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("x.7z");
        let nxm = NxmUrl {
            game: "skyrimspecialedition".into(),
            mod_id: 1,
            file_id: 2,
            key: None,
            expires: None,
            user_id: None,
        };
        let file = RemoteFile {
            name: "Main file".into(),
            file_name: "x.7z".into(),
            version: "1.0".into(),
            mod_version: "1.0".into(),
            category_id: None,
            description: String::new(),
            size_in_bytes: 1,
        };
        let hidden = gated(&mod_payload(Some(true)), AdultPolicy::Denied);
        assert!(write_download_meta(&archive, "SkyrimSE", &nxm, "https://x/y", &file, &hidden).is_err());
        assert!(!archive.with_extension("7z.meta").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_adult_preference_is_read_from_whichever_spelling_the_payload_uses() {
        // v1 spells it contains_adult_content; the newer APIs use other names for
        // the same thing. Accepting all three means a payload in a slightly
        // different shape rates correctly instead of hiding every mod.
        for key in ["contains_adult_content", "adult_content", "adultContent"] {
            let v = serde_json::json!({ key: true });
            assert_eq!(adult_flag(&v), Some(true), "{key}");
        }
        assert_eq!(adult_flag(&serde_json::json!({ "name": "x" })), None);
    }

    // ---- The request budget ------------------------------------------------

    /// A client whose last-seen budget is whatever the test says it is.
    fn client_with(limits: RateLimits) -> Nexus {
        let n = Nexus::with_bearer("t");
        n.limits.set(limits);
        n
    }

    const NOON: u64 = 1_700_000_000; // a fixed instant; the maths is absolute

    #[test]
    fn a_spent_hourly_budget_blocks_the_next_request_before_it_is_sent() {
        let n = client_with(RateLimits {
            hourly_remaining: Some(0),
            hourly_reset: Some(next_hour_utc(NOON)),
            ..Default::default()
        });
        let stop = n.preflight(NOON).expect("must refuse");
        assert!(is_rate_limited(&stop), "{stop}");
        assert!(stop.contains("hourly"), "{stop}");
    }

    #[test]
    fn a_spent_daily_budget_blocks_the_next_request_before_it_is_sent() {
        let n = client_with(RateLimits {
            daily_remaining: Some(0),
            daily_reset: Some(next_midnight_utc(NOON)),
            ..Default::default()
        });
        let stop = n.preflight(NOON).expect("must refuse");
        assert!(is_rate_limited(&stop) && stop.contains("daily"), "{stop}");
    }

    #[test]
    fn either_counter_reaching_zero_is_enough_to_stop() {
        // Nexus's own rule refuses only once BOTH buckets are spent, which is why
        // the reference clients track the larger of the two. Their reviewer asked
        // for the stricter reading, so one empty bucket stops us even while the
        // other still has room.
        let n = client_with(RateLimits {
            hourly_remaining: Some(0),
            hourly_reset: Some(next_hour_utc(NOON)),
            daily_remaining: Some(4_000),
            ..Default::default()
        });
        assert!(n.preflight(NOON).is_some());
    }

    #[test]
    fn an_unknown_budget_never_blocks_so_a_fresh_client_can_learn_it() {
        // The deadlock this avoids: no headers seen yet, so if "unknown" blocked,
        // the very request that would teach us the budget could never go out.
        assert!(Nexus::with_bearer("t").preflight(NOON).is_none());
    }

    #[test]
    fn an_account_with_no_budget_headers_at_all_is_never_blocked() {
        // Whatever the account tier, absent headers mean "no budget stated".
        let n = client_with(RateLimits { hourly_remaining: None, daily_remaining: None, ..Default::default() });
        assert!(n.preflight(NOON).is_none());
        assert!(!n.would_block());
    }

    #[test]
    fn the_hourly_block_lifts_at_the_next_utc_hour_and_the_budget_goes_unknown() {
        let reset = next_hour_utc(NOON);
        let n = client_with(RateLimits {
            hourly_remaining: Some(0),
            hourly_reset: Some(reset),
            ..Default::default()
        });
        assert!(n.preflight(reset - 1).is_some(), "still inside the hour");
        assert!(n.preflight(reset).is_none(), "the boundary releases it");
        // And it forgets the spent count, so the next reply re-teaches the truth
        // instead of the client carrying a stale zero forever.
        assert_eq!(n.rate_limits().hourly_remaining, None);
    }

    #[test]
    fn the_daily_block_lifts_at_the_next_utc_midnight() {
        let reset = next_midnight_utc(NOON);
        let n = client_with(RateLimits {
            daily_remaining: Some(0),
            daily_reset: Some(reset),
            ..Default::default()
        });
        assert!(n.preflight(reset - 1).is_some());
        assert!(n.preflight(reset).is_none());
    }

    #[test]
    fn the_reset_boundaries_are_the_next_ones_and_roll_over_cleanly() {
        // 23:30 rolls to the next midnight, not to hour 24 of the same day - the
        // off-by-one that a `(hour + 1) % 24` formulation invites.
        let almost_midnight = next_midnight_utc(NOON) - 1_800;
        assert_eq!(next_hour_utc(almost_midnight), next_midnight_utc(NOON));
        assert_eq!(next_midnight_utc(almost_midnight), next_midnight_utc(NOON));
        // Exactly on a boundary means the NEXT one, never "now".
        let midnight = next_midnight_utc(NOON);
        assert_eq!(next_midnight_utc(midnight), midnight + 86_400);
        assert_eq!(next_hour_utc(midnight), midnight + 3_600);
    }

    #[test]
    fn a_burst_guard_429_backs_off_briefly_rather_than_for_an_hour() {
        // A 429 while the counters still show budget is the burst guard in front
        // of the API, not the quota. Standing down until the next hour for that
        // would idle the client for up to an hour over a one-second burst.
        let n = client_with(RateLimits { hourly_remaining: Some(90), ..Default::default() });
        n.note_rejection(429);
        let until = n.rate_limits().blocked_until.expect("burst back-off recorded");
        assert!(until <= oauth::now_unix() + BURST_BACKOFF);
        assert!(n.would_block());
        // And it is a back-off, not an invented zero: the counters still say what
        // the server said.
        assert_eq!(n.rate_limits().hourly_remaining, Some(90));
    }

    #[test]
    fn a_429_that_the_spent_budget_explains_adds_no_extra_back_off() {
        let n = client_with(RateLimits {
            hourly_remaining: Some(0),
            hourly_reset: Some(next_hour_utc(NOON)),
            ..Default::default()
        });
        n.note_rejection(429);
        assert_eq!(n.rate_limits().blocked_until, None);
    }

    #[test]
    fn every_rate_limit_refusal_is_recognised_by_the_one_predicate() {
        // The bug this locks out: the library tested for "rate limited" while the
        // CLI tested for "429", so a pre-flight refusal - which carries no status
        // code - stopped one loop and left the other hammering a spent account.
        let n = client_with(RateLimits {
            hourly_remaining: Some(0),
            hourly_reset: Some(next_hour_utc(NOON)),
            ..Default::default()
        });
        assert!(is_rate_limited(&n.preflight(NOON).unwrap()));
        assert!(is_rate_limited(&Nexus::status_err(429)));
        assert!(!is_rate_limited(&Nexus::status_err(401)));
        // A mod id or byte count that happens to contain "429" is not a budget
        // refusal, which is exactly what the old substring test got wrong.
        assert!(!is_rate_limited("Nexus API error (HTTP 500) for mod 42942"));
    }

    #[test]
    fn parses_a_full_mod_manager_link() {
        // The shape the site's "Mod Manager Download" button produces.
        let u = NxmUrl::parse(
            "nxm://SkyrimSpecialEdition/mods/107676/files/697660?key=AbC-_12&expires=1776793111&user_id=86878448",
        )
        .unwrap();
        assert_eq!(u.game, "skyrimspecialedition"); // host lowercased
        assert_eq!(u.mod_id, 107676);
        assert_eq!(u.file_id, 697660);
        assert_eq!(u.key.as_deref(), Some("AbC-_12"));
        assert_eq!(u.expires, Some(1776793111));
        assert_eq!(u.user_id, Some(86878448));
    }

    #[test]
    fn unfinished_marker_appends_keeping_the_extension() {
        // MO2's marker is "<full name>.unfinished" - the .7z must survive so the
        // partial maps back, unlike Path::with_extension which would drop it.
        assert_eq!(
            unfinished_path(Path::new("/dl/Mod-1.0.7z")),
            PathBuf::from("/dl/Mod-1.0.7z.unfinished")
        );
        // Files differing only by extension get distinct temp names.
        assert_ne!(
            unfinished_path(Path::new("/dl/Foo.7z")),
            unfinished_path(Path::new("/dl/Foo.zip"))
        );
    }

    #[test]
    fn parses_a_bare_premium_link_and_rejects_junk() {
        let u = NxmUrl::parse("nxm://fallout4/mods/123/files/456").unwrap();
        assert_eq!((u.mod_id, u.file_id), (123, 456));
        assert!(u.key.is_none());

        assert!(NxmUrl::parse("https://nexusmods.com/whatever").is_err());
        assert!(NxmUrl::parse("nxm://skyrim/mods/notanumber/files/1").is_err());
        // Collections are explicitly unsupported for now (v2 API).
        let err = NxmUrl::parse("nxm://skyrim/collections/abcd/revisions/3").unwrap_err();
        assert!(err.contains("collection"));
    }

    #[test]
    fn meta_sidecar_matches_mo2_keys_and_roundtrips() {
        let dir = std::env::temp_dir().join(format!("eidos-nexus-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("Dynamic String Distributor-107676-1-3-1-1765658342.7z");
        let nxm = NxmUrl::parse("nxm://skyrimspecialedition/mods/107676/files/697660?key=k&expires=1&user_id=2").unwrap();
        let file = RemoteFile {
            name: "Dynamic String Distributor".into(),
            file_name: "Dynamic String Distributor-107676-1-3-1-1765658342.7z".into(),
            version: "1.3.1.0".into(),
            mod_version: "1.3.1".into(),
            category_id: Some(1),
            description: "Please read the changelog!".into(),
            size_in_bytes: 4_194_304,
        };
        let rmod = RemoteMod {
            name: "Dynamic String Distributor (DSD)".into(),
            version: "1.3.1".into(),
            summary: "".into(),
            category_id: Some(42),
            available: true,
            adult: Some(false),
            gate: shown_gate(),
        };
        let meta = write_download_meta(&archive, "SkyrimSE", &nxm, "https://cdn/x.7z", &file, &rmod).unwrap();
        let text = fs::read_to_string(&meta).unwrap();
        // The exact MO2 downloadmanager key set (sans the Qt blobs).
        for needle in [
            "[General]",
            "gameName=SkyrimSE",
            "modID=107676",
            "fileID=697660",
            "url=\"https://cdn/x.7z\"",
            "name=Dynamic String Distributor",
            "modName=Dynamic String Distributor (DSD)",
            "version=1.3.1.0",
            "fileCategory=1",
            "category=42",
            "repository=Nexus",
            "installed=false",
        ] {
            assert!(text.contains(needle), "missing {needle} in:\n{text}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cdn_uri_file_name() {
        assert_eq!(
            file_name_from_uri("https://cf-files.nexus-cdn.com/1704/107676/Dynamic%20String%20Distributor-107676.7z?md5=x&expires=1"),
            Some("Dynamic String Distributor-107676.7z".to_string())
        );
        assert_eq!(file_name_from_uri("https://host/"), None);
    }

    #[test]
    fn sanitizes_encoded_traversal_and_illegal_chars() {
        // %2e%2e%2f decodes to ../ - the separators must be neutralized so the
        // name can never escape the downloads dir when joined onto a path.
        let n = file_name_from_uri("https://cdn/files/%2e%2e%2fevil.7z?md5=x").unwrap();
        assert!(!n.contains('/') && !n.contains('\\'), "got {n}");
        assert_ne!(n, "..");
        assert_eq!(sanitize_file_name("a:b*c?.7z").as_deref(), Some("a_b_c_.7z"));
        assert_eq!(sanitize_file_name(".."), None);
        assert_eq!(sanitize_file_name("trailing.  ").as_deref(), Some("trailing"));
        assert_eq!(sanitize_file_name("   "), None);
    }

    #[test]
    fn unique_name_avoids_clobbering_existing_downloads() {
        let dir = std::env::temp_dir().join(format!("eidos-uniq-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_download_name(&dir, "Mod.7z"), "Mod.7z");
        fs::write(dir.join("Mod.7z"), b"x").unwrap();
        assert_eq!(unique_download_name(&dir, "Mod.7z"), "1_Mod.7z");
        fs::write(dir.join("1_Mod.7z"), b"x").unwrap();
        assert_eq!(unique_download_name(&dir, "Mod.7z"), "2_Mod.7z");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn endorse_version_falls_back_to_a_placeholder() {
        // Nexus requires a non-empty version in the endorsement body; a blank or
        // whitespace-only installed version becomes the placeholder "1.0".
        assert_eq!(endorse_version("1.3.1"), "1.3.1");
        assert_eq!(endorse_version("  2.0.4  "), "2.0.4"); // trimmed
        assert_eq!(endorse_version(""), "1.0");
        assert_eq!(endorse_version("   "), "1.0");
    }

    #[test]
    fn status_err_maps_the_mo2_codes() {
        assert_eq!(Nexus::status_err(401), "Nexus rejected the sign-in (401) - sign in again");
        assert!(Nexus::status_err(429).contains("429"));
        assert!(Nexus::status_err(429).contains("rate limited"));
        assert_eq!(Nexus::status_err(503), "Nexus API error (HTTP 503)");
    }

    #[test]
    fn update_check_result_defaults_are_empty() {
        // A fresh result reports nothing checked and no updates - the GUI relies on
        // these zero defaults before a check runs.
        let r = UpdateCheckResult::default();
        assert_eq!((r.checked, r.queried, r.updates_found), (0, 0, 0));
        assert!(r.updates.is_empty());
        assert!(!r.rate_limited);
        assert!(r.hourly_remaining.is_none() && r.daily_remaining.is_none());
    }

    #[test]
    fn mark_installed_flips_sidecar_and_is_a_noop_without_one() {
        let dir = std::env::temp_dir().join(format!("eidos-mark-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("M.7z");
        let meta = PathBuf::from(format!("{}.meta", archive.display()));
        fs::write(&meta, "[General]\nmodID=1\ninstalled=false\nuninstalled=false\nremoved=false\n").unwrap();
        mark_installed(&archive).unwrap();
        let t = fs::read_to_string(&meta).unwrap();
        assert!(t.contains("installed=true"));
        assert!(t.contains("uninstalled=false"));
        assert!(t.contains("modID=1")); // other keys preserved
        // No sidecar = silent no-op, not an error.
        mark_installed(&dir.join("absent.7z")).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_supporter_cdn_uri_yields_no_usable_name() {
        // The real URL from Lin's first download: Nexus serves supporter files
        // from a path whose last segment is a bare UUID. It parses fine as a
        // name, which is exactly the trap - the caller must prefer the API's
        // `file_name` and only fall back to the URI when it has an extension.
        let uri = "https://supporter-files.nexus-cdn.com/66/2a/35/\
662a3503-f985-4c27-a638-c811070e103a?expires=1785275952&h=4161fdf&md5=dQlw&user_id=86878448";
        let from_uri = file_name_from_uri(uri).unwrap();
        assert_eq!(from_uri, "662a3503-f985-4c27-a638-c811070e103a");
        assert!(!from_uri.contains('.'), "no extension, so not a file name");

        // A free-user CDN link does carry the real name, and must still work.
        let free = "https://cf-files.nexus-cdn.com/1704/186346/Dynamic%20Armor%20Physics-186346-1-0-1.zip?md5=x&expires=1";
        assert_eq!(
            file_name_from_uri(free).unwrap(),
            "Dynamic Armor Physics-186346-1-0-1.zip"
        );
    }


    #[test]
    fn the_sidecar_records_the_total_so_progress_can_be_a_percentage() {
        let dir = std::env::temp_dir().join(format!("eidos-meta-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let archive = dir.join("Thing.zip");
        let nxm = NxmUrl::parse("nxm://skyrimspecialedition/mods/1/files/2").unwrap();
        let file = RemoteFile {
            name: "Thing".into(),
            file_name: "Thing.zip".into(),
            version: "1.0".into(),
            mod_version: "1.0".into(),
            category_id: None,
            description: String::new(),
            size_in_bytes: 12_345,
        };
        let m = RemoteMod {
            name: "Thing".into(),
            version: "1.0".into(),
            summary: String::new(),
            category_id: None,
            available: true,
            adult: Some(false),
            gate: shown_gate(),
        };
        let path = write_download_meta(&archive, "SkyrimSE", &nxm, "https://x/y", &file, &m).unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("totalSize=12345"), "{body}");

        // A size we do not know must be ABSENT, not written as zero: the reader
        // treats 0 as "unknown" either way, but a zero on disk looks like a fact.
        let file0 = RemoteFile { size_in_bytes: 0, ..file };
        let p0 = write_download_meta(&archive, "SkyrimSE", &nxm, "https://x/y", &file0, &m).unwrap();
        assert!(!fs::read_to_string(&p0).unwrap().contains("totalSize"));
        let _ = fs::remove_dir_all(&dir);
    }


    // ---- which credential a session actually uses -------------------------
    //
    // `Nexus::connect` reads the disk, the clock and the environment; the RULE
    // it applies does not, so it is tested here directly. Since personal API keys
    // were removed at Nexus's request, these cases are about one question: when
    // there is no usable session, does the client say so rather than reach for
    // something it is not allowed to use.

    use eidos_instance::settings::NexusCreds;

    fn signed_in(expires_at: u64) -> NexusCreds {
        NexusCreds {
            access_token: Some("at".into()),
            refresh_token: Some("rt".into()),
            expires_at,
            adult_ok: None,
            adult_checked_at: 0,
        }
    }

    #[test]
    fn a_fresh_session_is_used_as_is() {
        assert_eq!(choose_credential(&signed_in(10_000), 1_000, true), CredentialChoice::Bearer);
    }

    #[test]
    fn a_session_expiring_within_the_skew_is_renewed_early() {
        // Expires in 60s, inside the 300s skew: renew now rather than fail
        // halfway through a download the user cannot retry cleanly.
        assert_eq!(choose_credential(&signed_in(1_060), 1_000, true), CredentialChoice::Refresh);
        // Comfortably outside it, so leave it alone.
        assert_eq!(choose_credential(&signed_in(1_400), 1_000, true), CredentialChoice::Bearer);
    }

    #[test]
    fn a_lapsed_session_that_cannot_be_renewed_is_reported_not_worked_around() {
        // No client_id registered, so no refresh is possible. There is nothing
        // else to try: this used to fall back to a personal API key, which is
        // exactly what Nexus requires a distributed client not to do.
        assert_eq!(choose_credential(&signed_in(500), 1_000, false), CredentialChoice::None);

        // Same when the refresh token itself is gone.
        let mut c = signed_in(500);
        c.refresh_token = None;
        assert_eq!(choose_credential(&c, 1_000, true), CredentialChoice::None);
    }

    #[test]
    fn nothing_stored_is_reported_not_guessed() {
        assert_eq!(choose_credential(&NexusCreds::default(), 1_000, true), CredentialChoice::None);
    }

    #[test]
    fn the_only_credential_is_a_bearer_token() {
        // There is one way to authenticate and one header it rides in. If a
        // second variant ever reappears here, it has to be a deliberate decision
        // rather than a fallback that crept back in.
        assert_eq!(Nexus::with_bearer("t").credential_kind(), "oauth");
        assert_eq!(
            Nexus::with_credential(Credential::Bearer("t".into())).credential_kind(),
            "oauth"
        );
    }
}
