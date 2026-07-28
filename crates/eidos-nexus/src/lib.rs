//! Nexus Mods integration, mirroring Mod Organizer 2's client behaviour
//! (`nxmaccessmanager.cpp` + `nexusinterface.cpp` + `downloadmanager.cpp`):
//!
//! - **Auth**: the personal API key sent as a raw `APIKEY` header against
//!   `https://api.nexusmods.com/v1/` (MO2's legacy-but-supported path; OAuth
//!   needs a registered client and can come later), validated via
//!   `users/validate`. Like MO2, no `.json` suffix on requests.
//! - **`nxm://` links** (the site's "Mod Manager Download" button):
//!   `nxm://<game>/mods/<id>/files/<file_id>?key=..&expires=..&user_id=..` -
//!   non-premium downloads REQUIRE that key/expires pair forwarded to the
//!   `download_link` endpoint (MO2 `NXMUrl` + `addNXMDownload`).
//! - **`.meta` sidecar**: each download gets `<archive>.meta` in MO2's exact
//!   key set (gameName/modID/fileID/url/version/...), which `eidos-install`
//!   already reads to seed the installed mod's `meta.ini`.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

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

/// Remote mod metadata (subset of `games/{game}/mods/{id}`).
#[derive(Debug, Clone)]
pub struct RemoteMod {
    pub name: String,
    pub version: String,
    pub summary: String,
    pub category_id: Option<u64>,
    pub available: bool,
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
}

/// Nexus rate-limit budget from the `X-RL-*` response headers (MO2 surfaces these
/// and stops dispatching when exhausted). Fields are `None` until a reply is seen.
#[derive(Debug, Clone, Copy, Default)]
pub struct RateLimits {
    pub hourly_remaining: Option<i64>,
    pub daily_remaining: Option<i64>,
}

/// The Nexus v1 API client.
pub struct Nexus {
    agent: ureq::Agent,
    api_key: String,
    limits: std::cell::Cell<RateLimits>,
}

fn s(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
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
    pub fn new(api_key: &str) -> Nexus {
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
            // the rate-limit budget in headers on EVERY reply including a 429,
            // and that is the one we most need to read: ureq 2 handed us the
            // response inside Error::Status, and 3 does not, so asking for the
            // response directly is how the budget stays visible.
            .http_status_as_error(false)
            .build()
            .into();
        Nexus {
            agent,
            api_key: api_key.trim().to_string(),
            limits: std::cell::Cell::new(RateLimits::default()),
        }
    }

    /// The most recent rate-limit budget seen (MO2's `X-RL-Hourly/Daily-Remaining`).
    pub fn rate_limits(&self) -> RateLimits {
        self.limits.get()
    }

    fn capture_limits(&self, resp: &ureq::http::Response<ureq::Body>) {
        let parse = |h: &str| {
            resp.headers().get(h).and_then(|v| v.to_str().ok()).and_then(|x| x.trim().parse::<i64>().ok())
        };
        self.limits.set(RateLimits {
            hourly_remaining: parse("X-RL-Hourly-Remaining"),
            daily_remaining: parse("X-RL-Daily-Remaining"),
        });
    }

    /// The MO2 status-code mapping shared by every request (401 = bad key,
    /// 429 = rate limited, else a generic message with the code).
    fn status_err(code: u16) -> String {
        match code {
            401 => "invalid API key (401)".to_string(),
            429 => "rate limited by Nexus (429) - try again later".to_string(),
            other => format!("Nexus API error (HTTP {other})"),
        }
    }

    /// Attach MO2's four identifying headers + APIKEY to a request (every v1 call
    /// carries these, per nxmaccessmanager.cpp addAPIHeaders).
    fn with_headers<B>(&self, req: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        req.header("APIKEY", &self.api_key)
            .header("Protocol-Version", "1.0.0")
            .header("Application-Name", "Eidos")
            .header("Application-Version", env!("CARGO_PKG_VERSION"))
    }

    fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{API_BASE}/{path}");
        // MO2 identifies the client on every v1 request (nxmaccessmanager.cpp
        // addAPIHeaders): Protocol-Version + Application-Name/-Version with APIKEY.
        // It also reads the X-RL-* budget from every reply (incl. a 429).
        match self.with_headers(self.agent.get(&url)).call() {
            Ok(mut resp) => {
                // Read the budget FIRST: it is present on a rejection too, and a
                // 429 is exactly when the caller needs to know how long to wait.
                self.capture_limits(&resp);
                let code = resp.status().as_u16();
                if !resp.status().is_success() {
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
        match self.with_headers(req).send_form([("version", version)]) {
            Ok(resp) => {
                self.capture_limits(&resp);
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(Nexus::status_err(resp.status().as_u16()))
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
    pub fn mod_info(&self, game: &str, mod_id: u64) -> Result<RemoteMod, String> {
        let v = self.get(&format!("games/{game}/mods/{mod_id}"))?;
        Ok(RemoteMod {
            name: s(&v, "name"),
            version: s(&v, "version"),
            summary: s(&v, "summary"),
            category_id: v.get("category_id").and_then(|x| x.as_u64()),
            available: v.get("available").and_then(|x| x.as_bool()).unwrap_or(true),
        })
    }

    /// File metadata: `games/{game}/mods/{id}/files/{file_id}`.
    pub fn file_info(&self, game: &str, mod_id: u64, file_id: u64) -> Result<RemoteFile, String> {
        let v = self.get(&format!("games/{game}/mods/{mod_id}/files/{file_id}"))?;
        Ok(RemoteFile {
            name: s(&v, "name"),
            file_name: s(&v, "file_name"),
            version: s(&v, "version"),
            mod_version: s(&v, "mod_version"),
            category_id: v.get("category_id").and_then(|x| x.as_u64()),
            description: s(&v, "description"),
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
    pub fn download_link(&self, nxm: &NxmUrl) -> Result<String, String> {
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
    let updated = nexus.updated_mod_ids(nexus_game, "1m")?;

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
                // MO2 stops dispatching the moment the account is exhausted. Match
                // our own status_err wording, not a bare "429" - a mod id or file
                // size containing 429 in some other error text must not trip this.
                if e.contains("rate limited") {
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
fn unfinished_path(dest: &Path) -> PathBuf {
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
    out.push_str("repository=Nexus\n");
    out.push_str("installed=false\nuninstalled=false\npaused=false\nremoved=false\n");
    fs::write(&meta_path, out)?;
    Ok(meta_path)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        };
        let rmod = RemoteMod {
            name: "Dynamic String Distributor (DSD)".into(),
            version: "1.3.1".into(),
            summary: "".into(),
            category_id: Some(42),
            available: true,
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
        assert_eq!(Nexus::status_err(401), "invalid API key (401)");
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
}
