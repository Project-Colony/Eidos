//! Per-mod `meta.ini`, MO2-compatible, so existing Mod Organizer 2 instances are
//! read - and written back - without losing anything.
//!
//! MO2 writes `mods/<name>/meta.ini` in Qt's `QSettings` INI dialect, whose
//! values are nastier than a normal INI: `nexusDescription` is a single line of
//! quote-wrapped HTML containing `=`, escaped quotes and literal `\n`; `color` is
//! a Qt `@Variant(...)` binary blob; there is a trailing `[installedFiles]`
//! section; and real files use CRLF line endings. A general INI library would
//! re-escape the values on write and corrupt the file.
//!
//! So we keep `[General]` values *raw* (everything after the first `=`, verbatim),
//! keep everything from the first non-key line onward as a verbatim byte slice
//! (the trailing sections and blank lines), remember whether the file used CRLF,
//! and only rewrite when something actually changed (`dirty`). An instance Eidos
//! merely reads is never rewritten; and when it does write a single field, every
//! other byte - line endings included - is reproduced exactly.

use std::fs;
use std::io;
use std::path::Path;

/// MO2-compatible per-mod metadata. Read non-destructively; write preserves every
/// untouched key, later section, and the original line endings.
#[derive(Debug, Clone, Default)]
pub struct ModMeta {
    /// `[General]` entries as `(key, raw-value-after-'=')`, original order kept.
    general: Vec<(String, String)>,
    /// Everything from the first non-key line in `[General]` to EOF, verbatim
    /// (blank line(s) + later sections like `[installedFiles]`).
    tail: String,
    /// The source used CRLF line endings (MO2's real files do).
    crlf: bool,
    dirty: bool,
}

impl ModMeta {
    /// Read a `meta.ini`. A missing or unreadable file yields an empty,
    /// non-dirty `ModMeta` (so callers can treat "no metadata" uniformly).
    pub fn read(path: &Path) -> ModMeta {
        let Ok(text) = fs::read_to_string(path) else {
            return ModMeta::default();
        };
        let crlf = text.contains("\r\n");
        let mut general = Vec::new();
        let mut tail = String::new();
        let mut in_general = false;
        let mut pos = 0usize; // byte offset of the current segment's start

        for seg in text.split_inclusive('\n') {
            let body = seg.trim_end_matches('\n').trim_end_matches('\r');
            let t = body.trim();
            let is_section = t.starts_with('[') && t.ends_with(']');

            if !in_general {
                if is_section && t.eq_ignore_ascii_case("[General]") {
                    in_general = true;
                }
                pos += seg.len();
                continue;
            }

            // Inside [General]: consecutive `key=value` lines belong to it. The
            // first line that is not one (blank, comment, or a new section) ends
            // the section; from there to EOF is preserved verbatim.
            if !is_section {
                if let Some((k, v)) = body.split_once('=') {
                    general.push((k.trim().to_string(), v.to_string()));
                    pos += seg.len();
                    continue;
                }
            }
            tail = text[pos..].to_string();
            break;
        }
        ModMeta { general, tail, crlf, dirty: false }
    }

    /// Raw value (everything after `=`) for a `[General]` key, case-insensitive.
    fn raw(&self, key: &str) -> Option<&str> {
        self.general.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }

    /// A `[General]` string value, unquoted and with empty treated as absent.
    fn string(&self, key: &str) -> Option<String> {
        self.raw(key).map(unquote).filter(|s| !s.is_empty())
    }

    pub fn game_name(&self) -> Option<String> {
        self.string("gameName")
    }

    /// The Nexus mod id (`0`/absent -> `None`).
    pub fn mod_id(&self) -> Option<u64> {
        self.raw("modid").and_then(|v| v.trim().parse().ok()).filter(|&n| n != 0)
    }

    pub fn version(&self) -> Option<String> {
        self.string("version")
    }

    pub fn newest_version(&self) -> Option<String> {
        self.string("newestVersion")
    }

    /// The MO2 category field, raw (a comma-terminated id list, e.g. `-1,`).
    pub fn category(&self) -> Option<String> {
        self.string("category")
    }

    pub fn installation_file(&self) -> Option<String> {
        self.string("installationFile")
    }

    pub fn repository(&self) -> Option<String> {
        self.string("repository")
    }

    pub fn endorsed(&self) -> bool {
        matches!(self.raw("endorsed").map(str::trim), Some("1") | Some("true"))
    }

    pub fn tracked(&self) -> bool {
        matches!(self.raw("tracked").map(str::trim), Some("1") | Some("true"))
    }

    /// A Nexus update is available when `newestVersion` is present and differs
    /// from the installed `version`. (String compare, matching MO2's own check;
    /// a future Nexus crate fills `newestVersion`.)
    pub fn update_available(&self) -> bool {
        match (self.version(), self.newest_version()) {
            (Some(v), Some(n)) => v != n,
            _ => false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Set a `[General]` value (raw - the caller quotes it the way MO2 would, if
    /// needed). Updates in place or appends; a no-op write does not mark dirty.
    pub fn set(&mut self, key: &str, raw_value: &str) {
        if let Some(slot) = self.general.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
            if slot.1 == raw_value {
                return;
            }
            slot.1 = raw_value.to_string();
        } else {
            self.general.push((key.to_string(), raw_value.to_string()));
        }
        self.dirty = true;
    }

    /// Record the latest version seen on Nexus (for update checks).
    pub fn set_newest_version(&mut self, v: &str) {
        self.set("newestVersion", v);
    }

    /// Write back to `meta.ini`, but only if something changed. Reproduces the
    /// original line endings and preserves every later section verbatim, so MO2's
    /// `[installedFiles]` etc. - and an unchanged file - survive byte-for-byte.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let eol = if self.crlf { "\r\n" } else { "\n" };
        let mut out = String::new();
        out.push_str("[General]");
        out.push_str(eol);
        for (k, v) in &self.general {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push_str(eol);
        }
        out.push_str(&self.tail);
        fs::write(path, out)
    }
}

/// Strip one layer of surrounding double quotes. MO2 quotes values that contain
/// commas or special characters (e.g. `category="-1,"`); bare values pass through.
fn unquote(v: &str) -> String {
    let v = v.trim();
    v.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(v).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp_ini(contents: &str) -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("eidos-meta-{}-{}.ini", std::process::id(), n));
        fs::write(&p, contents).unwrap();
        p
    }

    // A real-shaped MO2 meta.ini in CRLF: quoted category, a nexusDescription
    // containing `=` and an escaped quote, a Qt @Variant color, a blank line, and
    // a trailing section.
    const SAMPLE: &str = concat!(
        "[General]\r\n",
        "gameName=SkyrimSE\r\n",
        "modid=32117\r\n",
        "version=d2026.4.3.0\r\n",
        "newestVersion=0.139.2.0\r\n",
        "category=\"-1,\"\r\n",
        "installationFile=Z:/home/me/Downloads/Assorted Mesh Fixes-32117-0-139-2.7z\r\n",
        "repository=Nexus\r\n",
        "nexusDescription=\"line1\\n<br />line2 with = and \\\" quote\"\r\n",
        "color=@Variant(\\0\\0\\0\\x43\\0\\xff\\xff\\0\\0\\0\\0\\0\\0\\0\\0)\r\n",
        "endorsed=1\r\n",
        "tracked=0\r\n",
        "\r\n",
        "[installedFiles]\r\n",
        "size=0\r\n",
    );

    #[test]
    fn reads_known_fields() {
        let p = tmp_ini(SAMPLE);
        let m = ModMeta::read(&p);
        assert_eq!(m.game_name().as_deref(), Some("SkyrimSE"));
        assert_eq!(m.mod_id(), Some(32117));
        assert_eq!(m.version().as_deref(), Some("d2026.4.3.0"));
        assert_eq!(m.newest_version().as_deref(), Some("0.139.2.0"));
        assert_eq!(m.category().as_deref(), Some("-1,")); // unquoted
        assert!(m.installation_file().unwrap().ends_with(".7z"));
        assert_eq!(m.repository().as_deref(), Some("Nexus"));
        assert!(m.endorsed());
        assert!(!m.tracked());
        assert!(m.update_available()); // d2026.4.3.0 != 0.139.2.0
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn missing_file_is_empty_and_clean() {
        let m = ModMeta::read(Path::new("/no/such/eidos/meta.ini"));
        assert_eq!(m.mod_id(), None);
        assert!(!m.update_available());
        assert!(!m.is_dirty());
    }

    #[test]
    fn write_changes_one_key_and_keeps_everything_else_byte_for_byte() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);

        // A read-only round-trip must not mark dirty, and write() must no-op.
        assert!(!m.is_dirty());
        m.write(&p).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), SAMPLE); // untouched, byte-identical

        // Change exactly one key. The result must equal SAMPLE with only that
        // substring swapped - CRLF, the gnarly values, the blank line and the
        // [installedFiles] section all intact.
        m.set_newest_version("d2026.4.3.0");
        assert!(m.is_dirty());
        assert!(!m.update_available());
        m.write(&p).unwrap();

        let expected = SAMPLE.replace("newestVersion=0.139.2.0", "newestVersion=d2026.4.3.0");
        assert_eq!(fs::read_to_string(&p).unwrap(), expected);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn set_to_same_value_is_not_dirty() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        m.set("repository", "Nexus"); // unchanged
        assert!(!m.is_dirty());
        let _ = fs::remove_file(&p);
    }
}
