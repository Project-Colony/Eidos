//! Self-contained runtimes a tool needs, fetched once and shared by every
//! instance.
//!
//! The third tier of prerequisite, and it exists because the first two cannot
//! carry this. Tier 1 copies ONE bundled DLL into `system32`; tier 2 runs a
//! winetricks verb. A modern .NET runtime is neither: 193 files that live in
//! their own directory and are found through `DOTNET_ROOT`, never registered,
//! never installed into the prefix at all.
//!
//! # Why this is not bundled
//!
//! DynDOLOD's `LODGenx64Win10.exe` needs .NET 10, and .NET is MIT-licensed, so
//! shipping it would be legal. It is a question of size and of failure mode.
//! Measured on a real generation run, LODGen touched 25 of the 193 files - 25.6
//! MB of 78 - so a trimmed set looks tempting. But those 25 are the ones ONE
//! worldspace happened to need; another code path pulls in `System.Private.Xml`
//! or `System.Text.Json`, neither of which is among them. Ship a trimmed runtime
//! missing one and the tool dies with a `FileNotFoundException` - the same silent
//! nothing that made the original diagnosis cost a night. Trimming a runtime is
//! something an application's own author does, knowing their own code.
//!
//! So: one download, cached, complete.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A runtime Eidos knows how to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Runtime {
    /// The prerequisite verb a tool declares, e.g. `dotnet10`.
    pub verb: &'static str,
    pub version: &'static str,
    pub url: &'static str,
    /// SHA-256 of the archive. Pinned rather than merely fetched alongside the
    /// download: a hash served from the same host by the same request proves
    /// only that the bytes arrived intact, not that they are the bytes this
    /// version of Eidos was tested against.
    pub sha256: &'static str,
    /// A path inside the extracted tree that must exist for the install to count
    /// as complete. Guards against a half-extracted directory left by a crash.
    pub sentinel: &'static str,
    /// The environment variable that points a tool at it.
    pub env_var: &'static str,
}

const RUNTIMES: &[Runtime] = &[Runtime {
    verb: "dotnet10",
    version: "10.0.10",
    url: "https://builds.dotnet.microsoft.com/dotnet/Runtime/10.0.10/dotnet-runtime-10.0.10-win-x64.zip",
    sha256: "79bb04da40ab098f0c2e4ef4652fe5bb98c27c7d5697389003b7be53d2c86f6a",
    sentinel: "dotnet.exe",
    env_var: "DOTNET_ROOT",
}];

/// The runtime a verb names, if Eidos knows it.
pub fn runtime(verb: &str) -> Option<&'static Runtime> {
    RUNTIMES.iter().find(|r| r.verb.eq_ignore_ascii_case(verb))
}

pub fn is_runtime_verb(verb: &str) -> bool {
    runtime(verb).is_some()
}

/// Where runtimes are cached: outside any instance, because a 78 MB download is
/// not per-game and not per-profile. Two instances that both use DynDOLOD share
/// one copy.
pub fn runtimes_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
        .join("eidos")
        .join("runtimes")
}

/// Where one runtime lives, version included so an Eidos that pins a newer one
/// installs beside the old rather than over it.
pub fn runtime_dir(r: &Runtime) -> PathBuf {
    runtimes_dir().join(format!("{}-{}", r.verb, r.version))
}

/// Whether this runtime is present and complete.
pub fn is_installed(r: &Runtime) -> bool {
    let dir = runtime_dir(r);
    dir.join(r.sentinel).is_file()
}

/// The environment a set of prerequisite verbs asks for.
///
/// Windows paths: this is read by a program running under Wine, so `Z:` and
/// backslashes. A Linux path handed to the .NET host resolves to nothing and the
/// tool reports only that it cannot find a runtime.
pub fn env_for(verbs: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for v in verbs {
        let Some(r) = runtime(v) else { continue };
        if !is_installed(r) {
            continue;
        }
        out.push((r.env_var.to_string(), to_windows_dir(&runtime_dir(r))));
    }
    out
}

/// A Linux directory as Wine's `Z:` drive sees it, with no trailing separator.
fn to_windows_dir(p: &Path) -> String {
    format!("Z:{}", p.to_string_lossy().replace('/', "\\"))
}

#[derive(Debug)]
pub enum RuntimeError {
    Unknown(String),
    Network(String),
    Hash { expected: String, got: String },
    No7z,
    Extract(String),
    Io(io::Error),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Unknown(v) => write!(f, "no runtime called '{v}'"),
            RuntimeError::Network(e) => write!(f, "download failed: {e}"),
            RuntimeError::Hash { expected, got } => write!(
                f,
                "the download does not match the expected checksum \
                 (expected {expected}, got {got}) - refusing to install it"
            ),
            RuntimeError::No7z => write!(f, "7z is not installed (needed to unpack the runtime)"),
            RuntimeError::Extract(e) => write!(f, "could not unpack the runtime: {e}"),
            RuntimeError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for RuntimeError {
    fn from(e: io::Error) -> Self {
        RuntimeError::Io(e)
    }
}

/// Download and unpack a runtime, unless it is already there.
///
/// Everything lands in a temporary directory and is renamed into place at the
/// end, so an interrupted install leaves nothing that [`is_installed`] would
/// mistake for a finished one.
pub fn install(verb: &str, mut progress: impl FnMut(&str)) -> Result<bool, RuntimeError> {
    let r = runtime(verb).ok_or_else(|| RuntimeError::Unknown(verb.to_string()))?;
    if is_installed(r) {
        return Ok(false);
    }
    let seven = find_7z().ok_or(RuntimeError::No7z)?;

    let dir = runtime_dir(r);
    fs::create_dir_all(dir.parent().unwrap_or(&dir))?;
    let staging = dir.with_extension("incoming");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;

    progress(&format!("downloading {} {}", r.verb, r.version));
    let archive = staging.join("runtime.zip");
    download(r.url, &archive)?;

    progress("verifying");
    let got = sha256_file(&archive)?;
    if !got.eq_ignore_ascii_case(r.sha256) {
        let _ = fs::remove_dir_all(&staging);
        return Err(RuntimeError::Hash { expected: r.sha256.to_string(), got });
    }

    progress("unpacking");
    let out = staging.join("tree");
    fs::create_dir_all(&out)?;
    let status = Command::new(&seven)
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", out.display()))
        .arg(&archive)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| RuntimeError::Extract(e.to_string()))?;
    if !status.status.success() {
        let _ = fs::remove_dir_all(&staging);
        return Err(RuntimeError::Extract(
            String::from_utf8_lossy(&status.stderr).trim().to_string(),
        ));
    }
    if !out.join(r.sentinel).is_file() {
        let _ = fs::remove_dir_all(&staging);
        return Err(RuntimeError::Extract(format!(
            "the archive did not contain {} - it is not the runtime we expected",
            r.sentinel
        )));
    }

    // Rename last: until this line there is nothing an interrupted run could
    // leave behind that looks installed.
    let _ = fs::remove_dir_all(&dir);
    fs::rename(&out, &dir)?;
    let _ = fs::remove_dir_all(&staging);
    progress("done");
    Ok(true)
}

fn download(url: &str, dest: &Path) -> Result<(), RuntimeError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent(format!("Eidos/{}", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    let mut resp = agent.get(url).call().map_err(|e| RuntimeError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(RuntimeError::Network(format!("HTTP {}", resp.status().as_u16())));
    }
    let mut file = fs::File::create(dest)?;
    io::copy(&mut resp.body_mut().as_reader(), &mut file)?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, RuntimeError> {
    use std::io::Read;
    let mut hasher = Sha256::new();
    let mut f = fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish())
}

/// The 7z binary, under any of the names distributions ship it as. Same set
/// `eidos-install` accepts, because an archive is an archive.
fn find_7z() -> Option<PathBuf> {
    for name in ["7zz", "7z", "7za", "7zr"] {
        if let Ok(out) = Command::new("which").arg(name).output() {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    return Some(PathBuf::from(p));
                }
            }
        }
    }
    None
}

/// A minimal SHA-256. In-crate rather than a dependency: this crate has no hash
/// crate today, the algorithm is fixed and published, and the test below pins it
/// against the NIST vectors so an error here cannot pass unnoticed.
struct Sha256 {
    state: [u32; 8],
    buf: [u8; 64],
    buffered: usize,
    len: u64,
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Sha256 {
    fn new() -> Self {
        Sha256 {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: [0; 64],
            buffered: 0,
            len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        while !data.is_empty() {
            let take = (64 - self.buffered).min(data.len());
            self.buf[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
            self.buffered += take;
            data = &data[take..];
            if self.buffered == 64 {
                let block = self.buf;
                self.compress(&block);
                self.buffered = 0;
            }
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }

    fn finish(mut self) -> String {
        let bits = self.len.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0]);
        }
        // `update` counted the padding into `len`; the length field must carry
        // the message length, so it is captured before padding starts.
        let block_tail = bits.to_be_bytes();
        self.buf[56..64].copy_from_slice(&block_tail);
        let block = self.buf;
        self.compress(&block);
        self.state.iter().map(|w| format!("{w:08x}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_published_vectors() {
        // NIST FIPS 180-4 examples, plus the empty string and a message long
        // enough to span several blocks (where the padding arithmetic goes wrong
        // if it is going to).
        let cases: &[(&[u8], &str)] = &[
            (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ];
        for (input, want) in cases {
            let mut h = Sha256::new();
            h.update(input);
            assert_eq!(h.finish(), *want, "input {:?}", String::from_utf8_lossy(input));
        }
        // A million 'a', the classic long vector: catches a length counter that
        // overflows or a block boundary handled one byte out.
        let mut h = Sha256::new();
        for _ in 0..1000 {
            h.update(&[b'a'; 1000]);
        }
        assert_eq!(h.finish(), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    }

    #[test]
    fn feeding_in_odd_chunks_gives_the_same_answer() {
        // The download hashes in 64 KB reads; a buffering bug would only show on
        // inputs that do not align to the 64-byte block.
        let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let mut whole = Sha256::new();
        whole.update(&data);
        let whole = whole.finish();
        for chunk in [1usize, 7, 63, 64, 65, 999] {
            let mut h = Sha256::new();
            for part in data.chunks(chunk) {
                h.update(part);
            }
            assert_eq!(h.finish(), whole, "chunked by {chunk}");
        }
    }

    #[test]
    fn the_pinned_runtime_is_coherent() {
        let r = runtime("dotnet10").expect("dotnet10 is known");
        assert!(r.url.contains(r.version), "url and version disagree: {}", r.url);
        assert_eq!(r.sha256.len(), 64, "a sha256 is 64 hex characters");
        assert!(r.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(runtime("dotnet9").is_none(), "only what we actually pin");
        assert!(is_runtime_verb("DOTNET10"), "verbs match case-insensitively");
    }

    #[test]
    fn the_env_var_is_a_windows_path() {
        // Handed to a program running under Wine. A Linux path resolves to
        // nothing there, and the tool reports only that it found no runtime.
        let win = to_windows_dir(Path::new("/mnt/Jeux/Tools/dotnet"));
        assert_eq!(win, r"Z:\mnt\Jeux\Tools\dotnet");
        assert!(!win.contains('/'));
    }

    #[test]
    fn an_uninstalled_runtime_contributes_no_environment() {
        // Pointing DOTNET_ROOT at a directory that does not exist is worse than
        // leaving it unset: the host stops looking anywhere else.
        let verbs = vec!["dotnet10".to_string()];
        let dir = runtime_dir(runtime("dotnet10").unwrap());
        if !dir.join("dotnet.exe").is_file() {
            assert!(env_for(&verbs).is_empty());
        }
        assert!(env_for(&["vcrun2022".to_string()]).is_empty(), "not ours to answer for");
    }
}
