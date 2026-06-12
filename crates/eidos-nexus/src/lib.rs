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

/// The Nexus v1 API client.
pub struct Nexus {
    agent: ureq::Agent,
    api_key: String,
}

fn s(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

impl Nexus {
    pub fn new(api_key: &str) -> Nexus {
        let agent = ureq::AgentBuilder::new()
            .user_agent(&format!("Eidos/{} (Linux)", env!("CARGO_PKG_VERSION")))
            .build();
        Nexus { agent, api_key: api_key.trim().to_string() }
    }

    fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{API_BASE}/{path}");
        let resp = self
            .agent
            .get(&url)
            .set("APIKEY", &self.api_key)
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(401, _) => "invalid API key (401)".to_string(),
                ureq::Error::Status(429, _) => {
                    "rate limited by Nexus (429) - try again later".to_string()
                }
                other => other.to_string(),
            })?;
        resp.into_json().map_err(|e| e.to_string())
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

    /// Stream a (non-API) CDN URL to `dest`. Returns the byte count.
    pub fn download(&self, url: &str, dest: &Path) -> Result<u64, String> {
        let resp = self.agent.get(url).call().map_err(|e| e.to_string())?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = unfinished_path(dest); // MO2's in-progress marker (appended, keeps ext)
        let mut out = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut reader = resp.into_reader();
        let n = copy_stream(&mut reader, &mut out).map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
        Ok(n)
    }
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

/// The file name a CDN URI downloads to (the path's last segment, unescaped).
pub fn file_name_from_uri(uri: &str) -> Option<String> {
    let path = uri.split('?').next()?;
    let name = path.rsplit('/').next()?;
    let name = percent_decode(name);
    (!name.is_empty()).then_some(name)
}

fn percent_decode(sname: &str) -> String {
    let b = sname.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&sname[i + 1..i + 3], 16) {
                out.push(v);
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
}
