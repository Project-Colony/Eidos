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
//! # The client id, and why there is no secret next to it
//!
//! Nexus issues `client_id` per application and ties rate limits and abuse
//! handling to it. Eidos is registered as `eidos`; MO2's is `modorganizer2`,
//! and borrowing it would be passing Eidos off as MO2 to a third party.
//! `EIDOS_NEXUS_CLIENT_ID` still overrides, for anyone testing against their
//! own registration (MO2 has the same escape hatch, `MO2_NEXUS_CLIENT_ID`).
//!
//! The id is public by construction - it travels in the authorize URL, in
//! plain sight in the user's own browser - so shipping it in the source is not
//! a leak. A client SECRET would be a different matter, and there is none here
//! on purpose: Eidos is a public client (an installed application), it cannot
//! keep a secret that ships inside its own binary, and PKCE is precisely the
//! mechanism that replaces one. The authorization code is bound to a verifier
//! this process generated and never transmitted, so intercepting the code buys
//! an attacker nothing. Neither `authorize_url` nor `exchange_code` sends a
//! secret, and nothing here should ever start by default.
//!
//! A 401 right after a successful browser approval is NOT a missing secret,
//! however much it looks like one: adding a `client_secret` to the exchange
//! changes nothing, because the exchange was already succeeding. See
//! `claims` - the failure is the API call that used to follow it.
//!
//! There is no personal-API-key fallback either: Nexus requires personal keys
//! absent from a distributed client - absent, not merely unused.

use std::fmt;
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
/// Nexus asks for an EMPTY scope: their OAuth2 guide passes `scope: ''` on both
/// the authorize URL and the token exchange, and everything an application needs
/// about the user (id, username, membership roles) already rides inside the
/// access token's own claims - see [`claims`]. Requesting OIDC scopes we were
/// never granted is a way to be refused at the authorize step for no gain.
const SCOPES: &str = "";

/// Everything the flow needs that is deployment-specific.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub client_id: String,
    pub redirect_port: u16,
    pub authorize_url: String,
    pub token_url: String,
    pub scopes: String,
}

/// The `client_id` Nexus issued for Eidos. Public by design - it is a name, not
/// a credential, and the browser shows it to the user on every sign-in.
const CLIENT_ID: &str = "eidos";

impl Config {
    /// The client configuration: Eidos's own registration, or the override in
    /// `EIDOS_NEXUS_CLIENT_ID` for anyone testing against a different one.
    ///
    /// Still an `Option` rather than an infallible constructor: an override set
    /// to blank means "I meant to configure something and it did not take", and
    /// answering that with the built-in id would sign the user in as Eidos when
    /// they had asked for something else. Callers already handle `None`.
    pub fn from_env() -> Option<Config> {
        let client_id = match std::env::var("EIDOS_NEXUS_CLIENT_ID") {
            Ok(id) if id.trim().is_empty() => return None,
            Ok(id) => id.trim().to_string(),
            Err(_) => CLIENT_ID.to_string(),
        };
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

/// Stands in for a secret in `Debug` output. Reports whether the value is there
/// and how long it is - everything a bug report needs to distinguish "missing"
/// from "wrong" - and never what it is.
struct Redacted<'a>(&'a str);

impl fmt::Debug for Redacted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            f.write_str("\"\"")
        } else {
            write!(f, "<redacted, {} chars>", self.0.len())
        }
    }
}

/// A PKCE pair: the secret kept in this process, and the digest handed to the
/// browser. Without it, anything able to intercept the redirect could redeem the
/// authorization code - which is the whole reason a public client uses PKCE.
#[derive(Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

/// Hand-written: a derived `Debug` would print the verifier, and the verifier is
/// the entire protection PKCE provides. The challenge is the public half - it
/// travels in the authorize URL - so it stays legible.
impl fmt::Debug for Pkce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pkce")
            .field("verifier", &Redacted(&self.verifier))
            .field("challenge", &self.challenge)
            .finish()
    }
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
        Pkce {
            verifier: verifier.to_string(),
            challenge: URL_SAFE_NO_PAD.encode(digest),
        }
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

/// The recognisable query values of a callback request line.
#[derive(Default)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_desc: Option<String>,
}

/// Parse the query out of a request line, or `None` when it has no target.
fn callback_params(request_line: &str) -> Option<CallbackParams> {
    let target = request_line.split_whitespace().nth(1)?;
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut p = CallbackParams::default();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        match k {
            "code" => p.code = Some(urldecode(v)),
            "state" => p.state = Some(urldecode(v)),
            "error" => p.error = Some(urldecode(v)),
            "error_description" => p.error_desc = Some(urldecode(v)),
            _ => {}
        }
    }
    Some(p)
}

/// Pull the authorization code out of the browser's request line, refusing
/// anything whose `state` is not the one we sent.
///
/// The state check is not ceremony. The listener answers whatever reaches
/// 127.0.0.1, so any page the user has open could navigate to our callback with
/// a code of its own choosing; comparing state is what makes that fail.
pub fn parse_callback(request_line: &str, expected_state: &str) -> Result<String, String> {
    let Some(CallbackParams {
        code,
        state,
        error,
        error_desc,
    }) = callback_params(request_line)
    else {
        return Err("malformed request from the browser".to_string());
    };
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

/// What one connection to the loopback listener turned out to be.
enum Callback {
    /// The genuine redirect: a code whose `state` is this attempt's.
    Code(String),
    /// A definitive refusal from Nexus - the user pressed Cancel - carrying our
    /// `state` as RFC 6749 requires on error redirects. Final: report it.
    Refused(String),
    /// Everything else: a speculative browser preconnect that sends no request
    /// at all (Chrome opens those against an origin it is about to navigate
    /// to), a favicon fetch, a stray local probe, or a redirect whose `state`
    /// is not ours. None of these is the reply this attempt is waiting for, so
    /// none of them may end it - the listener keeps listening.
    Noise,
}

/// Classify a request line for [`wait_for_code`]'s accept loop. Stricter than
/// [`parse_callback`]: even a refusal only counts when it carries our `state`,
/// so a hostile local page cannot end the real sign-in by navigating to the
/// callback with a forged `error`. The worst case of that strictness is a
/// genuine denial with no state, which times out instead of reporting - the
/// safe direction.
fn classify_callback(request_line: &str, expected_state: &str) -> Callback {
    let Some(CallbackParams {
        code,
        state,
        error,
        error_desc,
    }) = callback_params(request_line)
    else {
        return Callback::Noise;
    };
    if state.as_deref() != Some(expected_state) {
        return Callback::Noise;
    }
    if let Some(e) = error {
        return Callback::Refused(match error_desc {
            Some(d) if !d.is_empty() => format!("Nexus refused the sign-in: {e} ({d})"),
            _ => format!("Nexus refused the sign-in: {e}"),
        });
    }
    match code {
        Some(c) => Callback::Code(c),
        None => Callback::Noise,
    }
}

/// What the browser is left looking at once the code is captured.
const DONE_PAGE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><meta charset=utf-8><title>Eidos</title><body style=\"font-family:system-ui;background:#ECDFC2;color:#2B2018;display:grid;place-items:center;height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"font-weight:600\">Eidos is connected.</h1><p>You can close this tab and go back to Eidos.</p></div>";

/// Wait on the loopback port for the browser to come back, and return the code.
///
/// Binds 127.0.0.1 explicitly - never `0.0.0.0`, which would put an
/// authorization endpoint on the local network - and gives up after `timeout`
/// so an abandoned sign-in cannot leave a listener and a thread behind.
///
/// Connections are ACCEPTED IN A LOOP, not once: the first thing to reach the
/// port is routinely not the redirect. Chrome opens a speculative preconnect -
/// a TCP connection that sends nothing - against an origin it is about to
/// navigate to, and returning its emptiness as the flow's verdict failed the
/// whole sign-in while the genuine callback found the listener gone. Noise is
/// answered (or dropped) and the wait continues; only this attempt's own
/// `state` can end it, one way or the other.
pub fn wait_for_code(port: u16, expected_state: &str, timeout: Duration) -> Result<String, String> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(addr)
        .map_err(|e| format!("could not listen on 127.0.0.1:{port} for the Nexus reply: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = SystemTime::now() + timeout;
    loop {
        if SystemTime::now() > deadline {
            return Err("timed out waiting for the Nexus sign-in".to_string());
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if stream.set_nonblocking(false).is_err() {
                    continue; // one broken connection must not end the wait
                }
                match handle_callback(stream, expected_state) {
                    Callback::Code(code) => return Ok(code),
                    Callback::Refused(why) => return Err(why),
                    Callback::Noise => {} // keep listening for the real redirect
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(120));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn handle_callback(mut stream: TcpStream, expected_state: &str) -> Callback {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    let Ok(reader) = stream.try_clone() else {
        return Callback::Noise;
    };
    if BufReader::new(reader).read_line(&mut line).is_err() {
        return Callback::Noise; // a hung or aborted connection is not the reply
    }
    let result = classify_callback(&line, expected_state);
    // Answer the real outcomes so the user sees a page instead of a dead tab -
    // on a refusal too, because the error belongs in Eidos, which is where they
    // are about to look. Noise gets nothing: a preconnect never reads the
    // response, and a favicon fetch survives a dropped connection.
    if !matches!(result, Callback::Noise) {
        let _ = stream.write_all(DONE_PAGE.as_bytes());
        let _ = stream.flush();
    }
    result
}

/// A completed sign-in.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
    /// Unix seconds. Absolute rather than a duration, because it has to survive
    /// being written to disk and read back in a later session.
    pub expires_at: u64,
}

/// Hand-written so the credentials cannot reach a log. `Tokens` is returned
/// through `Result` chains that callers format on failure, and a derived `Debug`
/// would print the refresh token in full - the one secret here that outlives the
/// session and can mint fresh access tokens on its own.
impl fmt::Debug for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tokens")
            .field("access_token", &Redacted(&self.access_token))
            .field("refresh_token", &Redacted(&self.refresh_token))
            .field("scope", &self.scope)
            .field("token_type", &self.token_type)
            .field("expires_at", &self.expires_at)
            .finish()
    }
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
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Where Nexus publishes the keys it signs tokens with.
///
/// Their OpenID discovery document (`/.well-known/openid-configuration`) names
/// this as its `jwks_uri`, so it is theirs to change and ours to follow.
const JWKS_URL: &str = "https://users.nexusmods.com/oauth/discovery/keys";

/// One RSA key out of Nexus's JWKS.
///
/// The components are kept RAW, as the JWK carries them, because `ring` verifies
/// straight from `n` and `e` - so there is no DER to assemble and no encoding to
/// get wrong at three in the morning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningKey {
    /// The key id a token's header names. Empty when the JWK carries none.
    pub kid: String,
    n: Vec<u8>,
    e: Vec<u8>,
}

/// The keys in a JWKS document.
///
/// Split out from the fetch so the shape Nexus actually publishes can be tested
/// without a network - which is the whole reason the old arrangement failed
/// silently: its guard compared two constants in the same file, so it could
/// only ever catch a typo, never a rotation.
fn parse_jwks(text: &str) -> Result<Vec<SigningKey>, String> {
    let doc: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("unreadable JWKS: {e}"))?;
    let Some(keys) = doc.get("keys").and_then(|k| k.as_array()) else {
        return Err("JWKS has no \"keys\" array".to_string());
    };
    let out: Vec<SigningKey> = keys
        .iter()
        .filter(|k| {
            // Signing keys only, and only the algorithm we verify. A JWKS may
            // legitimately carry encryption keys and other algorithms.
            k.get("kty").and_then(|v| v.as_str()) == Some("RSA")
                && k.get("use").and_then(|v| v.as_str()).unwrap_or("sig") == "sig"
                && k.get("alg").and_then(|v| v.as_str()).unwrap_or("RS256") == "RS256"
        })
        .filter_map(|k| {
            Some(SigningKey {
                kid: k
                    .get("kid")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                n: b64(k.get("n")?.as_str()?).ok()?,
                e: b64(k.get("e")?.as_str()?).ok()?,
            })
        })
        .collect();
    if out.is_empty() {
        return Err("JWKS carries no usable RS256 signing key".to_string());
    }
    Ok(out)
}

/// Ask Nexus for the keys it is signing with right now.
fn fetch_jwks() -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .get(JWKS_URL)
        .header("Application-Name", "Eidos")
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .call()
        .map_err(|e| e.to_string())?;
    resp.body_mut().read_to_string().map_err(|e| e.to_string())
}

/// Where the last JWKS we fetched is kept, beside the session it verifies.
///
/// Not an optimisation: it is what lets Eidos start OFFLINE and still know
/// whether the token on disk is genuine. Nothing secret is in it - these are
/// public keys - so it is written with ordinary permissions.
fn jwks_cache_path() -> std::path::PathBuf {
    eidos_instance::settings::nexus_key_path().with_file_name("nexus-keys.json")
}

/// Every key we currently believe Nexus signs with, newest source first.
///
/// Memory, then disk, then the network - and the network again, unconditionally,
/// when the token names a `kid` none of them has. That last step is the whole
/// point: a rotation is now something Eidos notices and follows within one
/// sign-in, rather than something that breaks every sign-in until somebody
/// ships a new binary.
fn signing_keys(want: Option<&str>) -> Result<Vec<SigningKey>, String> {
    static CACHED: std::sync::Mutex<Vec<SigningKey>> = std::sync::Mutex::new(Vec::new());

    let has = |keys: &[SigningKey]| match want {
        // A token with no `kid` can only mean "the one key they publish".
        None => !keys.is_empty(),
        Some(kid) => keys.iter().any(|k| k.kid == kid),
    };

    if let Ok(memo) = CACHED.lock() {
        if has(&memo) {
            return Ok(memo.clone());
        }
    }
    if let Some(keys) = std::fs::read_to_string(jwks_cache_path())
        .ok()
        .and_then(|t| parse_jwks(&t).ok())
    {
        if has(&keys) {
            if let Ok(mut memo) = CACHED.lock() {
                memo.clone_from(&keys);
            }
            return Ok(keys);
        }
    }
    let text = fetch_jwks().map_err(|e| {
        format!("could not reach Nexus to check its signing keys ({e}). Try again once you are online.")
    })?;
    let keys = parse_jwks(&text)?;
    // Written only after it parses, so a proxy's error page cannot replace a
    // good cache with something that will fail every start from now on.
    let path = jwks_cache_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, &text);
    if let Ok(mut memo) = CACHED.lock() {
        memo.clone_from(&keys);
    }
    Ok(keys)
}

/// Who the access token says its bearer is.
///
/// These come from the token itself, so reading them costs no API call - which
/// is the point: the UI can say "signed in as X (Premium)" the moment the flow
/// finishes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Claims {
    pub user_id: u64,
    pub username: String,
    pub is_premium: bool,
    /// Unix seconds, the token's own `exp`.
    pub expires_at: u64,
    /// Whether the signature was actually CHECKED, or the payload merely read.
    ///
    /// `false` means Nexus is signing with a key it does not publish, which is
    /// where things stand for access tokens: see [`claims`]. Nothing in Eidos
    /// decides anything on the strength of this - it exists so the log can say
    /// which of the two happened, instead of the difference being invisible.
    pub verified: bool,
}

/// Read an access token's claims, checking the signature when that is possible.
///
/// It was not always conditional. Eidos used to carry Nexus's signing key as a
/// constant and refuse any token it could not verify, so that a hand-edited
/// `nexus.ini` failed closed instead of quietly claiming Premium.
///
/// That stopped working, and the way it stopped is the reason this is written
/// out at length. Nexus rotated the key. The constant went stale, every sign-in
/// began failing with "JWT signature does not match the Nexus signing key", and
/// the guard test that was supposed to catch exactly this compared two
/// constants IN THE SAME FILE - so it could only ever have caught a typo.
///
/// Following the rotation is now automatic: [`signing_keys`] reads their OpenID
/// discovery document's `jwks_uri`. But that endpoint publishes the key for
/// ID tokens, and measurement says it is not the one access tokens are signed
/// with - a token issued minutes earlier carried `kid` `ysoTjItdaj...` against
/// a published `3Y6X5Uv...`, a 2048-bit signature against a 3072-bit key, and
/// it does not verify. The access-token key is published nowhere Eidos can
/// reach it.
///
/// So verification is BEST-EFFORT, and the rule is precise:
///
/// * a token whose `kid` we have a key for is verified, and a mismatch is
///   still fatal - tampering is caught wherever catching it is possible;
/// * a token naming a key nobody publishes is read anyway, with
///   [`Claims::verified`] `false`.
///
/// The alternative was refusing every sign-in, and the thing being defended
/// against is a user editing their own file to make their own window print
/// "Premium" - which gains them nothing, because these claims drive DISPLAY and
/// nothing else. Whether an account may actually download is answered by the
/// API rejecting the request. Two label sites and a status line are the entire
/// blast radius, and that was checked rather than assumed.
pub fn claims(access_token: &str) -> Result<Claims, String> {
    // The header first, because it names the key. Parsed twice - here and in
    // `claims_with_keys` - which costs a base64 decode of forty bytes and keeps
    // the verification a pure function of its inputs.
    let kid = access_token
        .split('.')
        .next()
        .and_then(|h| b64(h).ok())
        .and_then(|h| serde_json::from_slice::<serde_json::Value>(&h).ok())
        .and_then(|h| h.get("kid").and_then(|k| k.as_str()).map(str::to_string));
    // A failure to REACH Nexus is not a reason to refuse a token that is
    // already on disk: an offline start would otherwise report the user signed
    // out. Verification is best-effort by design (see below); no keys is one
    // more way of having no key for this token.
    let keys = signing_keys(kid.as_deref()).unwrap_or_default();
    claims_with_keys(access_token, &keys)
}

/// [`claims`] against keys given to it, so the verification path can be tested
/// with a key we hold the private half of and with no network. Nothing outside
/// the tests should call this: a caller choosing its own keys is a caller that
/// can be told to trust anything.
fn claims_with_keys(access_token: &str, keys: &[SigningKey]) -> Result<Claims, String> {
    let mut parts = access_token.split('.');
    let (Some(header), Some(payload), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err("not a JWT: expected three dot-separated segments".to_string());
    };

    // Pin the algorithm from the header rather than trusting it: accepting
    // whatever `alg` says is the classic JWT hole ("alg":"none" verifies
    // everything).
    let head: serde_json::Value =
        serde_json::from_slice(&b64(header)?).map_err(|e| format!("unreadable JWT header: {e}"))?;
    if head.get("alg").and_then(|a| a.as_str()) != Some("RS256") {
        return Err("unexpected JWT algorithm (only RS256 is accepted)".to_string());
    }

    // The key the header names, or every key we have when it names none.
    let kid = head.get("kid").and_then(|k| k.as_str());
    let candidates: Vec<&SigningKey> = match kid {
        Some(kid) => keys.iter().filter(|k| k.kid == kid).collect(),
        None => keys.iter().collect(),
    };
    // No key for it: read it anyway, and say so. See this function's docs -
    // Nexus signs access tokens with a key it does not publish, and refusing
    // them means refusing every sign-in.
    if candidates.is_empty() {
        return read_claims(payload, false);
    }

    let signed = format!("{header}.{payload}");
    let sig = b64(signature)?;
    // 2048 and up, not the 1024-and-up legacy exception this used to need.
    // Nexus signed with a 1024-bit key when this was written and now signs with
    // a 3072-bit one, so the weaker verifier is no longer the price of talking
    // to them - and a verifier that accepts 1024-bit RSA is a verifier that
    // accepts a forgery somebody can afford.
    let ok = candidates.iter().any(|k| {
        ring::signature::RsaPublicKeyComponents { n: &k.n, e: &k.e }
            .verify(
                &ring::signature::RSA_PKCS1_2048_8192_SHA256,
                signed.as_bytes(),
                &sig,
            )
            .is_ok()
    });
    // A key we HAVE that does not match is still fatal: this is the tampering
    // case, and it is the half of the old guarantee that survives.
    if !ok {
        return Err("JWT signature does not match the Nexus signing key".to_string());
    }
    read_claims(payload, true)
}

/// The claims themselves, once it has been decided whether they are trustworthy.
fn read_claims(payload: &str, verified: bool) -> Result<Claims, String> {
    let body: serde_json::Value =
        serde_json::from_slice(&b64(payload)?).map_err(|e| format!("unreadable JWT body: {e}"))?;
    let user = body.get("user");
    Ok(Claims {
        user_id: user
            .and_then(|u| u.get("id"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
        username: user
            .and_then(|u| u.get("username"))
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        // "premium" and "lifetimepremium" are separate roles in their example
        // payload, so match the prefix rather than listing the ones we happen
        // to have seen.
        is_premium: user
            .and_then(|u| u.get("membership_roles"))
            .and_then(|x| x.as_array())
            .is_some_and(|roles| {
                roles
                    .iter()
                    .filter_map(|r| r.as_str())
                    .any(|r| r.contains("premium"))
            }),
        expires_at: body.get("exp").and_then(|x| x.as_u64()).unwrap_or(0),
        verified,
    })
}

/// base64url without padding, which is what JWT segments are.
fn b64(segment: &str) -> Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|e| format!("bad base64 in JWT: {e}"))
}

/// Build [`Tokens`] from a token-endpoint reply. `now` is passed in so the
/// expiry arithmetic is testable without waiting for a clock.
pub fn tokens_from_json(v: &serde_json::Value, now: u64) -> Result<Tokens, String> {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let access_token = s("access_token");
    if access_token.is_empty() {
        // Surface the server's own words: `invalid_grant` after a refresh means
        // the user revoked us, and that needs a re-login, not a retry.
        let err = v
            .get("error")
            .and_then(|x| x.as_str())
            .unwrap_or("no access_token in reply");
        return Err(format!("Nexus token endpoint: {err}"));
    }
    let expires_in = v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(0);
    Ok(Tokens {
        access_token,
        refresh_token: s("refresh_token"),
        scope: s("scope"),
        token_type: s("token_type"),
        expires_at: now.saturating_add(expires_in),
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

/// Nexus's GraphQL endpoint. The account's content preferences live here and
/// nowhere in the v1 REST API - `users/validate` does not carry them.
pub const GRAPHQL_URL: &str = "https://api.nexusmods.com/v2/graphql";

/// The account's "show adult content" setting, or `None` if it could not be read.
///
/// `None` is not a failure to report: it is the answer "we do not know", which
/// the caller turns into [`crate::AdultPolicy::Unknown`] and therefore into
/// hiding adult metadata. Every way this can go wrong - offline, the token
/// rejected, the schema moved, a malformed body - lands there rather than
/// granting permission by accident.
///
/// The query takes no arguments: the endpoint scopes `preferences` to whoever the
/// bearer token belongs to. Unauthenticated it answers with an `errors` array and
/// a null `preferences`, which parses to `None` here without special-casing.
pub fn adult_preference(access_token: &str) -> Option<bool> {
    let body = serde_json::json!({
        "query": "query preferences { preferences { adult adultBlurImages } }"
    });
    let v = graphql(Some(access_token), &body).ok()?;
    v.get("data")?.get("preferences")?.get("adult")?.as_bool()
}

/// POST one GraphQL query and return its `data`-bearing body.
///
/// Two rules are baked in because getting either wrong is silent.
///
/// EXACTLY ONE credential. The endpoint accepts a bearer token or a personal
/// `apikey` header, and a stale one of either rejects the whole request with 401
/// even when the query itself needs no authentication at all - so `None` sends
/// nothing rather than sending an empty header.
///
/// AN ERRORS ARRAY IS A FAILURE. GraphQL reports errors inside a 200, so the
/// status alone says nothing: `{"errors": [...], "data": null}` is what an
/// unauthorised field looks like, and reading `data` out of it would take
/// "you may not see this" for "there is nothing here".
pub fn graphql(
    access_token: Option<&str>,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut req = agent
        .post(GRAPHQL_URL)
        // Attribution, as Nexus's own client sends on every call.
        .header("Application-Name", "Eidos")
        .header("Application-Version", env!("CARGO_PKG_VERSION"));
    if let Some(t) = access_token {
        req = req.header("Authorization", format!("Bearer {t}"));
    }
    let mut resp = req.send_json(body).map_err(|e| e.to_string())?;
    let status = resp.status();
    let v: serde_json::Value = resp.body_mut().read_json().map_err(|_| {
        format!(
            "Nexus answered {} with something that is not JSON",
            status.as_u16()
        )
    })?;
    if let Some(errs) = v.get("errors").filter(|e| !e.is_null()) {
        // The first message is the useful one; the rest are usually locations.
        let first = errs
            .as_array()
            .and_then(|a| a.first())
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("the query was refused");
        return Err(first.to_string());
    }
    if !status.is_success() {
        return Err(format!("Nexus answered {}", status.as_u16()));
    }
    Ok(v)
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
    fn the_flow_never_carries_a_client_secret() {
        // Eidos is a public client: whatever secret it shipped would ship inside
        // the binary, where any user can read it - which is why PKCE exists and
        // why the authorization server must not be handed one. This pins that no
        // future edit quietly adds a `client_secret` parameter to either leg of
        // the flow, and that the id itself is what the registration says.
        let cfg = Config {
            client_id: CLIENT_ID.to_string(),
            redirect_port: DEFAULT_REDIRECT_PORT,
            authorize_url: AUTHORIZE_URL.to_string(),
            token_url: TOKEN_URL.to_string(),
            scopes: SCOPES.to_string(),
        };
        let url = authorize_url(&cfg, &Pkce::from_verifier("v".repeat(43).as_str()), "state");
        assert!(url.contains("client_id=eidos"), "{url}");
        assert!(
            url.contains("code_challenge"),
            "PKCE must be on the wire: {url}"
        );
        assert!(!url.to_ascii_lowercase().contains("secret"), "{url}");
        // The redirect Nexus has on file for this registration. Changing it
        // breaks sign-in for everyone until Nexus is told, so it is pinned here.
        assert_eq!(cfg.redirect_uri(), "http://127.0.0.1:28638/callback");
    }

    #[test]
    fn an_empty_client_id_override_is_refused_rather_than_falling_back() {
        // "I set the variable and it did not take" must not silently sign the
        // user in as Eidos - they asked for a different identity.
        temp_env_var("EIDOS_NEXUS_CLIENT_ID", Some("   "), || {
            assert!(Config::from_env().is_none());
        });
        temp_env_var("EIDOS_NEXUS_CLIENT_ID", Some("someone-else"), || {
            assert_eq!(Config::from_env().unwrap().client_id, "someone-else");
        });
        temp_env_var("EIDOS_NEXUS_CLIENT_ID", None, || {
            assert_eq!(Config::from_env().unwrap().client_id, CLIENT_ID);
        });
    }

    /// Run `f` with an environment variable set (or removed), then restore it.
    /// Serialised because the environment is process-wide and Rust runs tests on
    /// threads - two of these racing would read each other's value.
    fn temp_env_var(key: &str, value: Option<&str>, f: impl FnOnce()) {
        use std::sync::Mutex;
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        f();
        match previous {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

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
        assert!(
            (43..=128).contains(&p.verifier.len()),
            "got {}",
            p.verifier.len()
        );
        assert!(p
            .verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')));
        assert_ne!(
            Pkce::new().unwrap().verifier,
            p.verifier,
            "two draws must differ"
        );
    }

    #[test]
    fn the_registration_is_eidos_own_and_carries_no_secret_by_default() {
        // Eidos signs in as itself, never as another application, and the
        // built-in configuration authenticates with PKCE alone.
        assert!(!AUTHORIZE_URL.contains("modorganizer"));
        assert_eq!(CLIENT_ID, "eidos");
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
        // The redirect is percent-encoded, not raw.
        assert!(u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A28638%2Fcallback"));
        // `scope` is PRESENT and EMPTY - Nexus's guide passes `scope: ''`, and
        // dropping the parameter entirely is not the same request as sending it
        // blank. Asserting both halves is what stops a future edit from
        // silently reintroducing a scope we were never granted.
        assert!(
            u.contains("scope=&") || u.ends_with("scope="),
            "scope not sent empty: {u}"
        );
        assert!(!u.contains("scope=openid"), "an OIDC scope came back: {u}");
        // The verifier is the secret half: it must never leave this process.
        assert!(
            !u.contains("dBjftJeZ4CVP"),
            "the verifier leaked into the browser URL"
        );
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
        let line = "GET /callback?error=access_denied&error_description=User%20said%20no HTTP/1.1";
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
        // The token endpoint may hand back a v1 API key. Eidos does not parse it:
        // Nexus requires personal-key usage gone from the client entirely, and a
        // key that is read "just in case" is exactly what that rules out.
        assert!(!format!("{t:?}").contains("api_key"));
    }

    /// The refresh token outlives the session and can mint fresh access tokens on
    /// its own, so a `{:?}` that reaches a log or a formatted error hands over
    /// durable access. Locked here rather than left to whoever next adds a log
    /// line near a `Tokens`.
    #[test]
    fn a_debug_of_tokens_never_prints_the_tokens() {
        let t = Tokens {
            access_token: "sup3r-secret-access".into(),
            refresh_token: "sup3r-secret-refresh".into(),
            scope: "openid".into(),
            token_type: "Bearer".into(),
            expires_at: 4_600,
        };
        let shown = format!("{t:?}");
        assert!(!shown.contains(&t.access_token), "{shown}");
        assert!(!shown.contains(&t.refresh_token), "{shown}");
        // Still worth printing: the non-secret fields stay readable, and a secret
        // that is set reads differently from one that is missing - which is the
        // whole reason anyone reaches for `{:?}` on this type.
        assert!(shown.contains("openid"), "{shown}");
        assert!(shown.contains("4600"), "{shown}");
        assert!(
            shown.contains(&format!("{} chars", t.access_token.len())),
            "the length stands in for the value: {shown}"
        );
        assert!(
            format!("{:?}", Tokens::default()).contains("\"\""),
            "and an absent secret reads as absent, not as redacted"
        );
    }

    /// Same rule one type over: the verifier is the entire protection PKCE gives,
    /// and it sits next to a challenge that is public by design.
    #[test]
    fn a_debug_of_pkce_never_prints_the_verifier() {
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        let shown = format!("{p:?}");
        assert!(!shown.contains(&p.verifier), "{shown}");
        assert!(
            shown.contains(&p.challenge),
            "the challenge travels in the authorize URL, it stays legible: {shown}"
        );
    }

    #[test]
    fn a_token_about_to_expire_counts_as_expired() {
        let t = Tokens {
            access_token: "at".into(),
            expires_at: 1_000,
            ..Default::default()
        };
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
        for s in [
            "openid profile email",
            "http://127.0.0.1:28638/callback",
            "a~b-c_d.e",
            "é",
        ] {
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
        assert!(
            elsewhere.is_ok(),
            "loopback bind must not have claimed every interface"
        );
    }

    #[test]
    fn waiting_gives_up_instead_of_hanging_forever() {
        let start = std::time::Instant::now();
        let err = wait_for_code(28938, "st4te", Duration::from_millis(300)).unwrap_err();
        assert!(err.contains("timed out"), "{err}");
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn the_wait_survives_preconnects_and_noise_before_the_real_redirect() {
        // What actually reaches the port during a Chrome sign-in, in order: a
        // speculative preconnect that sends NOTHING, a favicon fetch, and - if a
        // hostile local page is trying - a callback with somebody else's state.
        // The single-accept version returned the preconnect's emptiness as the
        // flow's verdict and the genuine redirect found the listener gone.
        let port = 28939;
        let waiter =
            std::thread::spawn(move || wait_for_code(port, "st4te", Duration::from_secs(10)));
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let connect = || loop {
            match TcpStream::connect(addr) {
                Ok(s) => break s,
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        };
        // 1. The empty preconnect: open, send nothing, close.
        drop(connect());
        // 2. A favicon fetch: a real request that is not the callback.
        let mut s = connect();
        s.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
        drop(s);
        // 3. A forged refusal WITHOUT our state: must not end the real flow.
        let mut s = connect();
        s.write_all(b"GET /callback?error=access_denied&state=forged HTTP/1.1\r\n\r\n")
            .unwrap();
        drop(s);
        // 4. The genuine redirect.
        let mut s = connect();
        s.write_all(b"GET /callback?code=the-real-code&state=st4te HTTP/1.1\r\n\r\n")
            .unwrap();
        drop(s);

        let got = waiter.join().unwrap();
        assert_eq!(got.as_deref(), Ok("the-real-code"), "{got:?}");
    }

    #[test]
    fn a_refusal_carrying_our_state_still_ends_the_wait() {
        // Pressing Cancel is an ordinary outcome and must be reported, not
        // waited past: Nexus echoes our state on the error redirect (RFC 6749).
        let port = 28941;
        let waiter =
            std::thread::spawn(move || wait_for_code(port, "st4te", Duration::from_secs(10)));
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let mut s = loop {
            match TcpStream::connect(addr) {
                Ok(s) => break s,
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        };
        s.write_all(b"GET /callback?error=access_denied&state=st4te HTTP/1.1\r\n\r\n")
            .unwrap();
        drop(s);
        let err = waiter.join().unwrap().unwrap_err();
        assert!(err.contains("access_denied"), "{err}");
    }

    // ---- access-token claims -------------------------------------------------
    //
    // Signed with a throwaway 3072-bit key generated for these tests, NOT with
    // anyone's real token: the point is to exercise the verification path
    // deterministically. Payload copied from the shape in the Nexus OAuth2
    // guide, `exp` in 2100 so the vector does not rot, and the same key SIZE
    // Nexus signs with today - so the tests exercise the verifier production
    // actually uses rather than a weaker one.

    const TEST_JWT: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImVpZG9zLXRlc3Qta2V5In0.eyJhcHBsaWNhdGlvbl9pZCI6MTAwLCJleHAiOjQxMDI0NDQ4MDAsImlhdCI6MTc1NDM4OTU5OCwianRpIjoidGVzdCIsInN1YiI6IjEyMzQ1IiwiaXNzIjoibmV4dXMtdXNlci1zZXJ2aWNlIiwidXNlciI6eyJpZCI6MTIzNDUsInVzZXJuYW1lIjoiVGVzdEFjY291bnQiLCJncm91cF9pZCI6MSwibWVtYmVyc2hpcF9yb2xlcyI6WyJtZW1iZXIiLCJzdXBwb3J0ZXIiLCJwcmVtaXVtIl0sInByZW1pdW1fZXhwaXJ5IjowLCJqb2luZWQiOjE0MTk1MzExMzR9fQ.F3aUtaK3Fv0ZTulzklXOSIMycPmnAZj9DmBLSBduKi4SlLGjiVfo9hi_zxh0eeQ23WqCz55LnjOHh-r3nG655T5msWNlP0LTk_l8KO81TUqtmVtnDmjQh9dxEX58WT70HFmk7BoFAIN4fecGcrOMwnb7Rx_ctsEnN_2inKEtiOaA9DVZdV_QLM5AmOpkwMbbbJ97QWCQjtTTKvsIv6XjGCFRbeBeKrivQdvFdE0fjkuRGzbO7IToYLEhHBeSwy__uuQjxo1nutz9TsCHzW3XkwEX8oJtNcTNkT46SuepgfKm67fkBT1edV2zgbO6J6i4DYMIqjWdEH9WUBDjnK_lJn3PzhUjce2gc9G3lbDv9QYjZx7ehKI3mA3Vg_RrwcjVg07oYpS94Tu-IrTnZl818DtnCHjlktaML9K0qgmHH57tqR-4ht73GA2P0H3N6ht45seOIrjtTNaDj3HQ5-ITHxTPoU1hlZSCtlmrUaXp8myfhPSG3lFnPJIfJN_UgLeM";

    /// The public half, as a JWKS carries it.
    const TEST_N: &str = "4-OtZFAhG9tkax3yxsylxyJWR8Lp4B8rykcwm_KGjuwQ6YVU-PLQOxb3A0i6-A-K41FPcw5PR2hnTlRlTQWV30dKXHJ-qEIxqSmwshSj4Bx_it-XrmcfXGkjahprNv-Jyf5iQeOY6uLGYO5TB2PhAeUFQLMeq-8VgNUVeBSEp3RvL5_HosFGqAiJZ6hIXU_fJkngSb_mEQVJUH6Xi8O3V66OJpN_BNnPPhOruO5sSeHVx-Ac3XjyqZkl465sAxfGoxg8FzGqjkImkfvbbJZPJwsOFq_B5cgivDp9az2AHkJoPhSABNHoSj0x5794z087Z-XlJrfXhpFzza2JCDI7liLiYpPHb4JrYXSlEdSr4PpUd2ZsqJzyp4-fTDxKsgiwRNG5_McI5z2otn5vS_y5edpCYpzTEV4ItTlRm7j98dTuXrh7amW6QrYBtYJt2B5-Ci-Bb6Qq_foPFZcBt0EtCSIqgSA92DldvKenrO_EK14i76a_dgyAq0ux8oFng6GV";
    const TEST_E: &str = "AQAB";

    fn test_keys() -> Vec<SigningKey> {
        parse_jwks(&format!(
            r#"{{"keys":[{{"kty":"RSA","use":"sig","alg":"RS256","kid":"eidos-test-key","n":"{TEST_N}","e":"{TEST_E}"}}]}}"#
        ))
        .expect("the fixture is a well-formed JWKS")
    }

    #[test]
    fn a_valid_token_yields_its_claims() {
        let c = claims_with_keys(TEST_JWT, &test_keys()).expect("the vector is correctly signed");
        assert!(c.verified, "a key that matches means a signature that was checked");
        assert_eq!(c.user_id, 12345);
        assert_eq!(c.username, "TestAccount");
        assert!(c.is_premium, "membership_roles carries \"premium\"");
        assert_eq!(c.expires_at, 4_102_444_800);
    }

    #[test]
    fn a_tampered_payload_is_refused() {
        // Re-encode the body with premium granted to a free account, leaving the
        // signature untouched: the exact edit someone would make by hand in
        // nexus.ini, and the reason the signature is checked at all.
        let mut parts: Vec<&str> = TEST_JWT.split('.').collect();
        let forged = URL_SAFE_NO_PAD.encode(
            br#"{"exp":4102444800,"user":{"id":1,"username":"Nobody","membership_roles":["premium"]}}"#,
        );
        parts[1] = &forged;
        let err = claims_with_keys(&parts.join("."), &test_keys()).unwrap_err();
        assert!(err.contains("signature"), "{err}");
    }

    #[test]
    fn the_algorithm_is_pinned_so_alg_none_cannot_pass() {
        let head = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let body = URL_SAFE_NO_PAD.encode(br#"{"user":{"username":"Nobody"}}"#);
        let err = claims_with_keys(&format!("{head}.{body}."), &test_keys()).unwrap_err();
        assert!(err.contains("algorithm"), "{err}");
    }

    #[test]
    fn junk_is_rejected_without_panicking() {
        for bad in ["", "abc", "a.b", "a.b.c.d", "....", "!!.??.$$"] {
            assert!(claims_with_keys(bad, &test_keys()).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn a_free_account_is_not_premium() {
        // The role list decides it, and only a role CONTAINING "premium" counts -
        // "supporter" and "member" must not be mistaken for it.
        let v = serde_json::json!({"user": {"membership_roles": ["member", "supporter"]}});
        let roles = v["user"]["membership_roles"].as_array().unwrap().clone();
        assert!(!roles
            .iter()
            .filter_map(|r| r.as_str())
            .any(|r| r.contains("premium")));
    }

    #[test]
    fn a_token_naming_a_key_nobody_publishes_is_read_but_marked_unverified() {
        // The failure that started this. Nexus signs access tokens with a key
        // it does not publish, so refusing them refuses every sign-in - while
        // the claims drive nothing but two labels. They are read, and the fact
        // that nothing vouched for them is recorded rather than hidden.
        let keys = test_keys();
        let mut other = keys[0].clone();
        other.kid = "some-other-key".to_string();
        let c = claims_with_keys(TEST_JWT, &[other]).expect("read, not refused");
        assert_eq!(c.username, "TestAccount");
        assert!(!c.verified, "and it must not claim to have been checked");
        // With no keys at all - offline, empty cache - the same.
        let c = claims_with_keys(TEST_JWT, &[]).expect("an offline start still signs in");
        assert!(!c.verified);
    }

    #[test]
    fn a_key_we_do_have_still_refuses_a_forgery() {
        // The half of the old guarantee that survives, and the reason the rule
        // is "no key means read it" rather than "verification is optional":
        // where a matching key EXISTS, a mismatch is still fatal.
        let mut parts: Vec<&str> = TEST_JWT.split('.').collect();
        let forged = URL_SAFE_NO_PAD.encode(br#"{"user":{"username":"Nobody"}}"#);
        parts[1] = &forged;
        let err = claims_with_keys(&parts.join("."), &test_keys()).unwrap_err();
        assert!(err.contains("signature"), "{err}");
    }

    #[test]
    fn a_token_with_no_kid_is_tried_against_every_key() {
        // Nexus published no `kid` at all when this code was written. A token
        // from that era, or from a provider that omits it, must still verify
        // against the one key they publish.
        let head = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let mut parts: Vec<&str> = TEST_JWT.split('.').collect();
        parts[0] = &head;
        // The signature no longer covers this header, so it must FAIL on the
        // signature - not on the key lookup, which is what is being tested.
        let err = claims_with_keys(&parts.join("."), &test_keys()).unwrap_err();
        assert!(err.contains("signature"), "{err}");
    }

    /// The document Nexus actually serves at their `jwks_uri`, captured
    /// 2026-09-07. Not for its key - that will rotate again, which is the whole
    /// point - but for its SHAPE, so a parser that stops understanding what they
    /// publish fails here rather than at somebody's sign-in.
    #[test]
    fn the_real_nexus_jwks_shape_parses() {
        let doc = r#"{"keys":[{"kty":"RSA","n":"9J0ftAKHorF8SoB0qztUQM8JfLjVi3GssO0owIfwDAhKzt5p4fG6osmuq5-G4OpR8MW9ZDwG8KXTz12FrlWyKdzbjDxM4h03VtoQjGSLuvEob0rfwRvneY8SHA1ogABD_igH7nq8otuW4gA8-KV15HRuGrd4KTKzt4kVXJc9F5q4wAuBi_kmqyhVtk4RRRaONsqxKCUTUdQghbeTjiuBF_5lXiFuGWip7AuWt-ohXyKFAZw9EkuBR-S6lZ4WfRkxApKtHHgG0xmPMYRJjXlz55ARDQClkV6jFjIex3Jo9QOvpOqlQnU3cX8bM-Jb8DNoVaBBTm8iYxNph2jqi9HafSTGnqmToGTJlq3BDUwnZUmptz1n2MreHYZPdAUtks7HzLJJioLg4tr1C0xTTdTce0qGWTksOA5Q5CLYdPsFImapZ37rpegXdf9mebjfl4-qqSH4WA7peAQkgauuO_kBqVBGJ3r5a3l4p6G2zSPLPc9BoXe77efAsRjd_rmWC9VX","e":"AQAB","kid":"3Y6X5UvJYLH_G759QiLBWDy8ozRAVWzEua0VTEJvj58","use":"sig","alg":"RS256"}]}"#;
        let keys = parse_jwks(doc).expect("their own document must parse");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].kid, "3Y6X5UvJYLH_G759QiLBWDy8ozRAVWzEua0VTEJvj58");
        assert_eq!(keys[0].e, vec![0x01, 0x00, 0x01]);
    }

    #[test]
    fn a_jwks_with_nothing_usable_in_it_is_an_error_not_an_empty_list() {
        // An empty key list that verified nothing would be indistinguishable
        // from a forgery being accepted.
        for doc in [
            r#"{"keys":[]}"#,
            r#"{"keys":[{"kty":"EC","crv":"P-256","x":"a","y":"b"}]}"#,
            r#"{"keys":[{"kty":"RSA","use":"enc","n":"AQAB","e":"AQAB"}]}"#,
            r#"{"keys":[{"kty":"RSA","alg":"RS512","n":"AQAB","e":"AQAB"}]}"#,
            r#"{"nope":1}"#,
            "not json at all",
        ] {
            assert!(parse_jwks(doc).is_err(), "accepted {doc}");
        }
    }

    #[test]
    fn scope_is_empty_as_the_guide_requires() {
        // Their own example passes scope: ''. A non-empty scope here would be
        // sent on both the authorize URL and the token exchange.
        assert_eq!(SCOPES, "");
    }
}
