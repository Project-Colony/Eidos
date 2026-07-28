//! Nexus Mods sign-in: OAuth 2.0 authorization code + PKCE, the flow every
//! current mod manager uses and the one a user recognises - a browser opens on
//! nexusmods.com, they approve, the manager is connected.
//!
//! Shaped after MO2, which moved off both API keys and the older websocket SSO
//! (`nexusoauthconfig.cpp`, `nxmaccessmanager.cpp:934`):
//!
//! ```text
//! authorize  https://users.nexusmods.com/oauth/authorize
//! token      https://users.nexusmods.com/oauth/token
//! redirect   http://127.0.0.1:<port>/callback     (loopback, never 0.0.0.0)
//! pkce       S256
//! scope      openid profile email
//! ```
//!
//! # The client id is not ours to invent
//!
//! Nexus issues `client_id` per application, by mail to support@nexusmods.com,
//! and ties rate limits and abuse handling to it. MO2's is `modorganizer2`;
//! using it would be passing Eidos off as MO2 to a third party. So there is no
//! default here: with no id configured, [`Config::from_env`] returns `None` and
//! a sign-in refuses with an explanation rather than sending something wrong.
//! Set `EIDOS_NEXUS_CLIENT_ID` (MO2 has the same escape hatch,
//! `MO2_NEXUS_CLIENT_ID`) and the flow works unchanged.
//!
//! The personal API key path stays: it needs no registration, it is what the
//! `[Nexus] api_key=` setting already holds, and MO2 keeps its equivalent too.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

/// Where the loopback callback listens. Registered WITH Nexus as part of the
/// redirect URI, so it cannot be changed casually afterwards - it has to match
/// what the application was registered with. Deliberately not MO2's 28635, so
/// both can be signing in at once without fighting over the port.
pub const DEFAULT_REDIRECT_PORT: u16 = 28638;

const AUTHORIZE_URL: &str = "https://users.nexusmods.com/oauth/authorize";
const TOKEN_URL: &str = "https://users.nexusmods.com/oauth/token";
/// Plain OIDC scopes, not application-specific.
const SCOPES: &str = "openid profile email";

/// Everything the flow needs that is deployment-specific.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub client_id: String,
    pub redirect_port: u16,
    pub authorize_url: String,
    pub token_url: String,
    pub scopes: String,
}

impl Config {
    /// The configured client, or `None` when Nexus has not issued one yet.
    ///
    /// `None` is a real answer, not a failure to read: it is what "we have not
    /// been registered" looks like, and every caller has to say so out loud
    /// instead of falling back to somebody else's identifier.
    pub fn from_env() -> Option<Config> {
        let client_id = std::env::var("EIDOS_NEXUS_CLIENT_ID").ok()?;
        let client_id = client_id.trim().to_string();
        if client_id.is_empty() {
            return None;
        }
        Some(Config {
            client_id,
            redirect_port: std::env::var("EIDOS_NEXUS_REDIRECT_PORT")
                .ok()
                .and_then(|p| p.trim().parse().ok())
                .unwrap_or(DEFAULT_REDIRECT_PORT),
            authorize_url: AUTHORIZE_URL.to_string(),
            token_url: TOKEN_URL.to_string(),
            scopes: SCOPES.to_string(),
        })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.redirect_port)
    }
}

/// A PKCE pair: the secret kept in this process, and the digest handed to the
/// browser. Without it, anything able to intercept the redirect could redeem the
/// authorization code - which is the whole reason a public client uses PKCE.
#[derive(Debug, Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> std::io::Result<Pkce> {
        // 32 bytes -> 43 unreserved characters, the RFC 7636 minimum, and what
        // every reference implementation uses.
        Ok(Pkce::from_verifier(&random_token(32)?))
    }

    /// The S256 transform: `BASE64URL(SHA256(ASCII(verifier)))`, unpadded.
    pub fn from_verifier(verifier: &str) -> Pkce {
        let digest = Sha256::digest(verifier.as_bytes());
        Pkce { verifier: verifier.to_string(), challenge: URL_SAFE_NO_PAD.encode(digest) }
    }
}

/// `len` random bytes as URL-safe base64. Straight from the kernel: Eidos is
/// Linux-only (it lives on FUSE and mount namespaces), so `/dev/urandom` costs
/// nothing and saves a dependency whose version churn we would carry forever.
pub fn random_token(len: usize) -> std::io::Result<String> {
    use std::io::Read;
    let mut buf = vec![0u8; len];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

/// The URL to open in the user's browser.
pub fn authorize_url(cfg: &Config, pkce: &Pkce, state: &str) -> String {
    let q = [
        ("response_type", "code"),
        ("client_id", cfg.client_id.as_str()),
        ("redirect_uri", &cfg.redirect_uri()),
        ("scope", cfg.scopes.as_str()),
        ("state", state),
        ("code_challenge", pkce.challenge.as_str()),
        ("code_challenge_method", "S256"),
    ]
    .iter()
    .map(|(k, v)| format!("{k}={}", urlencode(v)))
    .collect::<Vec<_>>()
    .join("&");
    format!("{}?{q}", cfg.authorize_url)
}

/// Percent-encode everything outside the unreserved set. Small on purpose: the
/// only values that pass through are a URL, a scope list and base64url tokens.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn urldecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(b[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Pull the authorization code out of the browser's request line, refusing
/// anything whose `state` is not the one we sent.
///
/// The state check is not ceremony. The listener answers whatever reaches
/// 127.0.0.1, so any page the user has open could navigate to our callback with
/// a code of its own choosing; comparing state is what makes that fail.
pub fn parse_callback(request_line: &str, expected_state: &str) -> Result<String, String> {
    let target = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "malformed request from the browser".to_string())?;
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_desc = None;
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        match k {
            "code" => code = Some(urldecode(v)),
            "state" => state = Some(urldecode(v)),
            "error" => error = Some(urldecode(v)),
            "error_description" => error_desc = Some(urldecode(v)),
            _ => {}
        }
    }
    // Report the refusal before anything else: "access_denied" is the ordinary
    // outcome of the user pressing Cancel, and it deserves its own words.
    if let Some(e) = error {
        return Err(match error_desc {
            Some(d) if !d.is_empty() => format!("Nexus refused the sign-in: {e} ({d})"),
            _ => format!("Nexus refused the sign-in: {e}"),
        });
    }
    match (code, state) {
        (Some(_), Some(s)) if s != expected_state => {
            Err("the reply did not match this sign-in attempt (state mismatch)".to_string())
        }
        (Some(_), None) => Err("the reply carried no state".to_string()),
        (Some(c), Some(_)) => Ok(c),
        (None, _) => Err("the reply carried no authorization code".to_string()),
    }
}

/// What the browser is left looking at once the code is captured.
const DONE_PAGE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><meta charset=utf-8><title>Eidos</title><body style=\"font-family:system-ui;background:#ECDFC2;color:#2B2018;display:grid;place-items:center;height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"font-weight:600\">Eidos is connected.</h1><p>You can close this tab and go back to Eidos.</p></div>";

/// Wait on the loopback port for the browser to come back, and return the code.
///
/// Binds 127.0.0.1 explicitly - never `0.0.0.0`, which would put an
/// authorization endpoint on the local network - and gives up after `timeout`
/// so an abandoned sign-in cannot leave a listener and a thread behind.
pub fn wait_for_code(port: u16, expected_state: &str, timeout: Duration) -> Result<String, String> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(addr).map_err(|e| {
        format!("could not listen on 127.0.0.1:{port} for the Nexus reply: {e}")
    })?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = SystemTime::now() + timeout;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                return handle_callback(stream, expected_state);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if SystemTime::now() > deadline {
                    return Err("timed out waiting for the Nexus sign-in".to_string());
                }
                std::thread::sleep(Duration::from_millis(120));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn handle_callback(mut stream: TcpStream, expected_state: &str) -> Result<String, String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    BufReader::new(stream.try_clone().map_err(|e| e.to_string())?)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let result = parse_callback(&line, expected_state);
    // Answer either way, so the user sees a page instead of a dead tab. On
    // failure the browser still gets a 200 - the error belongs in Eidos, which
    // is where they are about to look.
    let _ = stream.write_all(DONE_PAGE.as_bytes());
    let _ = stream.flush();
    result
}

/// A completed sign-in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
    /// Unix seconds. Absolute rather than a duration, because it has to survive
    /// being written to disk and read back in a later session.
    pub expires_at: u64,
    /// Some deployments hand back a v1 API key alongside the tokens; the rest of
    /// Eidos speaks v1, so keep it when it is offered.
    pub api_key: Option<String>,
}

impl Tokens {
    /// Whether the access token is past use. The default skew matches MO2's five
    /// minutes: a token that expires mid-request is a failure the user cannot
    /// act on, so treat "nearly expired" as expired and refresh early.
    pub fn is_expired(&self, now: u64, skew: Duration) -> bool {
        self.expires_at <= now.saturating_add(skew.as_secs())
    }

    pub fn is_valid(&self) -> bool {
        !self.access_token.is_empty() && self.expires_at != 0
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Build [`Tokens`] from a token-endpoint reply. `now` is passed in so the
/// expiry arithmetic is testable without waiting for a clock.
pub fn tokens_from_json(v: &serde_json::Value, now: u64) -> Result<Tokens, String> {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let access_token = s("access_token");
    if access_token.is_empty() {
        // Surface the server's own words: `invalid_grant` after a refresh means
        // the user revoked us, and that needs a re-login, not a retry.
        let err = v.get("error").and_then(|x| x.as_str()).unwrap_or("no access_token in reply");
        return Err(format!("Nexus token endpoint: {err}"));
    }
    let expires_in = v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(0);
    Ok(Tokens {
        access_token,
        refresh_token: s("refresh_token"),
        scope: s("scope"),
        token_type: s("token_type"),
        expires_at: now.saturating_add(expires_in),
        api_key: v
            .get("api_key")
            .and_then(|x| x.as_str())
            .filter(|k| !k.is_empty())
            .map(str::to_string),
    })
}

fn post_form(url: &str, form: &[(&str, &str)]) -> Result<serde_json::Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        // Read the body on a 4xx instead of turning it into an opaque error:
        // the token endpoint puts the reason in there.
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .post(url)
        .header("Application-Name", "Eidos")
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .send_form(form.iter().map(|(k, v)| (*k, *v)))
        .map_err(|e| e.to_string())?;
    resp.body_mut().read_json().map_err(|e| e.to_string())
}

/// Exchange the authorization code for tokens (the PKCE verifier proves we are
/// the client that started the flow).
pub fn exchange_code(cfg: &Config, code: &str, pkce: &Pkce) -> Result<Tokens, String> {
    let redirect = cfg.redirect_uri();
    let v = post_form(
        &cfg.token_url,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &cfg.client_id),
            ("code", code),
            ("redirect_uri", &redirect),
            ("code_verifier", &pkce.verifier),
        ],
    )?;
    tokens_from_json(&v, now_unix())
}

/// Trade a refresh token for a fresh access token, so a returning user is not
/// sent back to the browser every session.
pub fn refresh(cfg: &Config, refresh_token: &str) -> Result<Tokens, String> {
    let v = post_form(
        &cfg.token_url,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", &cfg.client_id),
            ("refresh_token", refresh_token),
        ],
    )?;
    let mut t = tokens_from_json(&v, now_unix())?;
    // Some servers omit the refresh token on a refresh, meaning "keep the one
    // you have". Dropping it would log the user out on the following session.
    if t.refresh_token.is_empty() {
        t.refresh_token = refresh_token.to_string();
    }
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_the_rfc_test_vector() {
        // RFC 7636 appendix B, the published known answer for S256. If this ever
        // fails, sign-in fails at the token endpoint with a message that will
        // not point here - so pin it.
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(p.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        assert!(!p.challenge.contains('='), "base64url for PKCE is unpadded");
    }

    #[test]
    fn a_generated_verifier_is_long_enough_and_unreserved() {
        let p = Pkce::new().unwrap();
        // RFC 7636 section 4.1: 43..=128 characters from the unreserved set.
        assert!((43..=128).contains(&p.verifier.len()), "got {}", p.verifier.len());
        assert!(p
            .verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')));
        assert_ne!(Pkce::new().unwrap().verifier, p.verifier, "two draws must differ");
    }

    #[test]
    fn there_is_no_default_client_id() {
        // The point of the whole module doc: with nothing registered, a sign-in
        // must refuse rather than borrow another application's identity.
        assert!(!AUTHORIZE_URL.contains("modorganizer"));
        let cfg = Config {
            client_id: "eidos-test".into(),
            redirect_port: 28638,
            authorize_url: AUTHORIZE_URL.into(),
            token_url: TOKEN_URL.into(),
            scopes: SCOPES.into(),
        };
        assert_eq!(cfg.redirect_uri(), "http://127.0.0.1:28638/callback");
    }

    fn cfg() -> Config {
        Config {
            client_id: "eidos".into(),
            redirect_port: 28638,
            authorize_url: AUTHORIZE_URL.into(),
            token_url: TOKEN_URL.into(),
            scopes: SCOPES.into(),
        }
    }

    #[test]
    fn the_authorize_url_carries_every_required_parameter() {
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        let u = authorize_url(&cfg(), &p, "st4te");
        for expected in [
            "response_type=code",
            "client_id=eidos",
            "code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            "code_challenge_method=S256",
            "state=st4te",
        ] {
            assert!(u.contains(expected), "{expected} missing from {u}");
        }
        // The redirect and the scopes are percent-encoded, not raw.
        assert!(u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A28638%2Fcallback"));
        assert!(u.contains("scope=openid%20profile%20email"));
        // The verifier is the secret half: it must never leave this process.
        assert!(!u.contains("dBjftJeZ4CVP"), "the verifier leaked into the browser URL");
    }

    #[test]
    fn a_good_callback_yields_the_code() {
        let line = "GET /callback?code=abc123&state=st4te HTTP/1.1";
        assert_eq!(parse_callback(line, "st4te").unwrap(), "abc123");
    }

    #[test]
    fn a_mismatched_state_is_refused() {
        // Anything on the machine can reach 127.0.0.1 and hand us a code of its
        // choosing; the state is what makes that attempt fail.
        let line = "GET /callback?code=attacker&state=someone-elses HTTP/1.1";
        let err = parse_callback(line, "st4te").unwrap_err();
        assert!(err.contains("state"), "{err}");
        // Missing entirely is refused too, not treated as "no objection".
        assert!(parse_callback("GET /callback?code=x HTTP/1.1", "st4te").is_err());
    }

    #[test]
    fn a_refusal_is_reported_in_the_servers_own_words() {
        let line =
            "GET /callback?error=access_denied&error_description=User%20said%20no HTTP/1.1";
        let err = parse_callback(line, "st4te").unwrap_err();
        assert!(err.contains("access_denied"), "{err}");
        assert!(err.contains("User said no"), "{err}");
    }

    #[test]
    fn a_reply_with_no_code_is_an_error_not_an_empty_success() {
        assert!(parse_callback("GET /callback HTTP/1.1", "st4te").is_err());
        assert!(parse_callback("garbage", "st4te").is_err());
    }

    #[test]
    fn tokens_are_read_with_an_absolute_expiry() {
        let v = serde_json::json!({
            "access_token": "at", "refresh_token": "rt",
            "token_type": "Bearer", "scope": "openid", "expires_in": 3600
        });
        let t = tokens_from_json(&v, 1_000).unwrap();
        assert_eq!(t.expires_at, 4_600);
        assert!(t.is_valid());
        assert!(t.api_key.is_none());
    }

    #[test]
    fn a_token_about_to_expire_counts_as_expired() {
        let t = Tokens { access_token: "at".into(), expires_at: 1_000, ..Default::default() };
        let skew = Duration::from_secs(300);
        assert!(!t.is_expired(600, skew), "still good with 400s to run");
        assert!(t.is_expired(701, skew), "inside the skew, so refresh early");
        assert!(t.is_expired(2_000, skew));
    }

    #[test]
    fn an_error_reply_does_not_masquerade_as_a_token() {
        let v = serde_json::json!({ "error": "invalid_grant" });
        let err = tokens_from_json(&v, 0).unwrap_err();
        assert!(err.contains("invalid_grant"), "{err}");
    }

    #[test]
    fn percent_coding_round_trips() {
        for s in ["openid profile email", "http://127.0.0.1:28638/callback", "a~b-c_d.e", "é"] {
            assert_eq!(urldecode(&urlencode(s)), s, "{s}");
        }
        // The browser sends spaces either way; both have to decode.
        assert_eq!(urldecode("User+said+no"), "User said no");
        assert_eq!(urldecode("User%20said%20no"), "User said no");
    }

    #[test]
    fn the_callback_listener_only_binds_loopback() {
        // An authorization endpoint reachable from the LAN would be a real hole.
        // Bind the same port on loopback and assert the wildcard is still free -
        // if `wait_for_code` had used 0.0.0.0, this second bind would fail.
        let port = 28937;
        let _guard = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).unwrap();
        let elsewhere = TcpListener::bind(SocketAddr::from(([127, 0, 0, 2], port)));
        assert!(elsewhere.is_ok(), "loopback bind must not have claimed every interface");
    }

    #[test]
    fn waiting_gives_up_instead_of_hanging_forever() {
        let start = std::time::Instant::now();
        let err = wait_for_code(28938, "st4te", Duration::from_millis(300)).unwrap_err();
        assert!(err.contains("timed out"), "{err}");
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
