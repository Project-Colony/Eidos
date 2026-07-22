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
        let crlf = eidos_ini::newline_style(&text) == "\r\n";
        let mut general = Vec::new();
        let mut tail = String::new();
        let mut in_general = false;
        let mut pos = 0usize; // byte offset of the current segment's start

        for seg in text.split_inclusive('\n') {
            let body = seg.trim_end_matches('\n').trim_end_matches('\r');
            let section = eidos_ini::section_header(body);

            if !in_general {
                if section.is_some_and(|s| s.eq_ignore_ascii_case("General")) {
                    in_general = true;
                }
                pos += seg.len();
                continue;
            }

            // Inside [General]: consecutive `key=value` lines belong to it. The
            // first line that is not one (blank, comment, or a new section) ends
            // the section; from there to EOF is preserved verbatim. Values stay
            // RAW (everything after `=`) so MO2's quoted/escaped values survive.
            if section.is_none() {
                if let Some((k, v)) = eidos_ini::key_value(body) {
                    general.push((k.to_string(), v.to_string()));
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

    /// The download sidecar's `modName` (the mod's display name on Nexus).
    pub fn mod_name(&self) -> Option<String> {
        self.string("modName")
    }

    /// The sidecar's `name` (the file entry's name), HTML-stripped like MO2.
    pub fn name(&self) -> Option<String> {
        self.string("name").map(|s| strip_html(&s)).filter(|s| !s.is_empty())
    }

    /// The sidecar's `fileCategory` (the Nexus file category id).
    pub fn file_category(&self) -> Option<String> {
        self.string("fileCategory")
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

    /// MO2's "Ignore update": when set, the mod is excluded from the update
    /// markers + count even if Nexus reports a newer version.
    pub fn ignore_update(&self) -> bool {
        matches!(self.raw("ignoreUpdate").map(str::trim), Some("1") | Some("true"))
    }

    /// Endorse / abstain the mod locally (the network side is handled by the
    /// caller); mirrors the `1`/`0` form `endorsed()` reads.
    pub fn set_endorsed(&mut self, b: bool) {
        self.set("endorsed", if b { "1" } else { "0" });
    }

    /// Track / untrack the mod (local flag only, MO2's "Track").
    pub fn set_tracked(&mut self, b: bool) {
        self.set("tracked", if b { "1" } else { "0" });
    }

    /// Set / clear MO2's "Ignore update" flag for the mod.
    pub fn set_ignore_update(&mut self, b: bool) {
        self.set("ignoreUpdate", if b { "1" } else { "0" });
    }

    /// A Nexus update is available when `newestVersion` is present and differs
    /// from the installed `version` - compared by numeric segments, so cosmetic
    /// differences ("1.5" vs "1.5.0", "v1.2" vs "1.2") don't flag phantom updates.
    pub fn update_available(&self) -> bool {
        match (self.version(), self.newest_version()) {
            (Some(v), Some(n)) => !same_version(&v, &n),
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

    // ---- download lifecycle (the Downloads-manager status column) ----
    // These mirror the flags `write_download_meta` writes in the `.meta` sidecar,
    // using the same `true`/`false` form MO2's download manager records.

    /// The archive has been installed into a mod (`installed=true`).
    pub fn installed(&self) -> bool {
        matches!(self.raw("installed").map(str::trim), Some("true") | Some("1"))
    }

    /// Mark the archive installed / not installed.
    pub fn set_installed(&mut self, b: bool) {
        self.set("installed", if b { "true" } else { "false" });
    }

    /// The archive's mod was later removed (`uninstalled=true`).
    pub fn uninstalled(&self) -> bool {
        matches!(self.raw("uninstalled").map(str::trim), Some("true") | Some("1"))
    }

    pub fn set_uninstalled(&mut self, b: bool) {
        self.set("uninstalled", if b { "true" } else { "false" });
    }

    /// The download was paused mid-transfer (`paused=true`).
    pub fn paused(&self) -> bool {
        matches!(self.raw("paused").map(str::trim), Some("true") | Some("1"))
    }

    /// The download was hidden/removed in the manager (`removed=true`).
    pub fn removed(&self) -> bool {
        matches!(self.raw("removed").map(str::trim), Some("true") | Some("1"))
    }

    pub fn set_removed(&mut self, b: bool) {
        self.set("removed", if b { "true" } else { "false" });
    }

    /// The user's free-text note for this mod (MO2's modlist Notes column).
    pub fn notes(&self) -> Option<String> {
        self.string("notes")
    }

    /// Set the note text, quoting it the way MO2's QSettings dialect does so a
    /// value with spaces or commas round-trips. Empty clears it.
    pub fn set_notes(&mut self, notes: &str) {
        let trimmed = notes.trim();
        if trimmed.is_empty() {
            self.set("notes", "");
        } else {
            let escaped = trimmed.replace('\\', "\\\\").replace('"', "\\\"");
            self.set("notes", &format!("\"{escaped}\""));
        }
    }

    /// A separator's display colour, decoded from MO2's `color=@Variant(...)` (a
    /// Qt `QColor` serialised by `QSettings`). `None` if absent or unparseable.
    pub fn color(&self) -> Option<[u8; 3]> {
        self.raw("color").and_then(variant_qcolor_decode)
    }

    /// Set (or, with `None`, clear) the separator colour, written in MO2's
    /// `@Variant(...)` form so an existing MO2 instance reads it back.
    pub fn set_color(&mut self, rgb: Option<[u8; 3]>) {
        match rgb {
            Some(rgb) => self.set("color", &variant_qcolor_encode(rgb)),
            None => self.set("color", ""),
        }
    }

    /// When this mod was last checked against Nexus (unix seconds; `0`/absent ->
    /// `None`). MO2 tracks this so it can trust the `updated?period=1m` bulk list
    /// only for mods checked within the window, and query the rest individually.
    pub fn last_nexus_update(&self) -> Option<u64> {
        self.raw("lastNexusUpdate").and_then(|v| v.trim().parse().ok()).filter(|&n| n != 0)
    }

    pub fn set_last_nexus_update(&mut self, ts: u64) {
        self.set("lastNexusUpdate", &ts.to_string());
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

/// Whether two version strings denote the same version, compared by numeric
/// segments: a leading `v`/`V` is ignored, `.`/`-`/`_` split segments, and
/// trailing zero segments are insignificant ("1.5" == "1.5.0" == "v1.5"). If
/// either side has a non-numeric segment, falls back to a trimmed
/// case-insensitive string compare (e.g. "1.5SE" style tags).
fn same_version(a: &str, b: &str) -> bool {
    fn segments(s: &str) -> Option<Vec<u64>> {
        let t = s.trim().trim_start_matches(['v', 'V']);
        let mut out = Vec::new();
        for part in t.split(['.', '-', '_']) {
            if part.is_empty() {
                continue;
            }
            out.push(part.parse::<u64>().ok()?);
        }
        (!out.is_empty()).then_some(out)
    }
    match (segments(a), segments(b)) {
        (Some(mut x), Some(mut y)) => {
            while x.last() == Some(&0) {
                x.pop();
            }
            while y.last() == Some(&0) {
                y.pop();
            }
            x == y
        }
        _ => a.trim().eq_ignore_ascii_case(b.trim()),
    }
}

/// Decode a `@Variant(...)` value (Qt `QSettings`-serialised `QColor`) to RGB.
///
/// The inner bytes are a `QDataStream`: `[u32 type-id = 67 (QColor)]`, then `[u8
/// colour-spec]`, then five big-endian `u16` channels `[alpha, red, green, blue,
/// pad]`, each an 8-bit value scaled by 257 (`0xVV -> 0xVVVV`). Defensive: any
/// shape we don't recognise yields `None` (the caller falls back to a default).
fn variant_qcolor_decode(raw: &str) -> Option<[u8; 3]> {
    let inner = raw.trim().strip_prefix("@Variant(")?.strip_suffix(')')?;
    let bytes = unescape_variant(inner)?;
    // type-id(4) + spec(1) + alpha(2) + red(2) + green(2) + blue(2) = 13 minimum.
    if bytes.len() < 13 {
        return None;
    }
    if u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != 67 {
        return None; // not a QColor
    }
    let red = u16::from_be_bytes([bytes[7], bytes[8]]);
    let green = u16::from_be_bytes([bytes[9], bytes[10]]);
    let blue = u16::from_be_bytes([bytes[11], bytes[12]]);
    Some([(red >> 8) as u8, (green >> 8) as u8, (blue >> 8) as u8])
}

/// Encode RGB into MO2's opaque `@Variant(...)` `QColor` form (the inverse of
/// [`variant_qcolor_decode`]).
fn variant_qcolor_encode([r, g, b]: [u8; 3]) -> String {
    let chan = |v: u8| ((v as u16) * 257).to_be_bytes(); // 0xVV -> 0xVVVV
    let mut bytes: Vec<u8> = Vec::with_capacity(15);
    bytes.extend_from_slice(&67u32.to_be_bytes()); // QColor type id
    bytes.push(1); // colour-spec = RGB
    bytes.extend_from_slice(&0xFFFFu16.to_be_bytes()); // alpha = opaque
    bytes.extend_from_slice(&chan(r));
    bytes.extend_from_slice(&chan(g));
    bytes.extend_from_slice(&chan(b));
    bytes.extend_from_slice(&0u16.to_be_bytes()); // pad
    let mut s = String::from("@Variant(");
    for byte in bytes {
        if byte == 0 {
            s.push_str("\\0");
        } else {
            s.push_str(&format!("\\x{byte:02x}"));
        }
    }
    s.push(')');
    s
}

/// Un-escape the byte stream inside `@Variant(...)` (Qt's `QSettings` escaping:
/// `\0`, `\xNN`, `\\`, the usual control escapes, else literal).
fn unescape_variant(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        i += 1;
        let c = *b.get(i)?;
        match c {
            b'0' => {
                out.push(0);
                i += 1;
            }
            b'x' | b'X' => {
                i += 1;
                let start = i;
                while i < b.len() && b[i].is_ascii_hexdigit() {
                    i += 1;
                }
                if i == start {
                    return None;
                }
                let hex = std::str::from_utf8(&b[start..i]).ok()?;
                out.push((u32::from_str_radix(hex, 16).ok()? & 0xff) as u8);
            }
            b'\\' => {
                out.push(b'\\');
                i += 1;
            }
            b'n' => {
                out.push(b'\n');
                i += 1;
            }
            b'r' => {
                out.push(b'\r');
                i += 1;
            }
            b't' => {
                out.push(b'\t');
                i += 1;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    Some(out)
}

/// Strip one layer of surrounding double quotes. MO2 quotes values that contain
/// commas or special characters (e.g. `category="-1,"`); bare values pass through.
fn unquote(v: &str) -> String {
    let v = v.trim();
    v.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(v).to_string()
}

/// Remove `<...>` tags from a value (MO2's `name` field can carry HTML).
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
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
    fn last_nexus_update_round_trips() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        assert_eq!(m.last_nexus_update(), None);
        m.set_last_nexus_update(1_700_000_000);
        assert_eq!(m.last_nexus_update(), Some(1_700_000_000));
        assert!(m.is_dirty());
        m.write(&p).unwrap();
        assert_eq!(ModMeta::read(&p).last_nexus_update(), Some(1_700_000_000));
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

    #[test]
    fn notes_round_trip_through_quoting() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        assert_eq!(m.notes(), None);
        m.set_notes("merge with USSEP, keep my patch on top");
        assert!(m.is_dirty());
        assert_eq!(m.notes().as_deref(), Some("merge with USSEP, keep my patch on top"));
        m.write(&p).unwrap();
        assert_eq!(
            ModMeta::read(&p).notes().as_deref(),
            Some("merge with USSEP, keep my patch on top")
        );
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn decodes_a_canonical_mo2_separator_color() {
        // A canonical opaque MO2 QColor: type 67, spec 1, alpha FFFF, then
        // r=3333 g=6666 b=9999 (each 8-bit*257), pad 0000 -> RGB #336699.
        let raw = "@Variant(\\0\\0\\0\\x43\\x1\\xff\\xff\\x33\\x33\\x66\\x66\\x99\\x99\\0\\0)";
        assert_eq!(variant_qcolor_decode(raw), Some([0x33, 0x66, 0x99]));
        // A short/garbage blob is rejected (caller falls back to the default).
        assert_eq!(variant_qcolor_decode("@Variant(\\0\\0)"), None);
        assert_eq!(variant_qcolor_decode("not a variant"), None);
    }

    #[test]
    fn color_encode_decode_round_trips_through_modmeta() {
        for rgb in [[0u8, 0, 0], [0xFF, 0xFF, 0xFF], [0x33, 0x66, 0x99], [0x12, 0xAB, 0x7E]] {
            assert_eq!(variant_qcolor_decode(&variant_qcolor_encode(rgb)), Some(rgb));
        }
        // Through the full ModMeta set/read path (the value survives the INI round-trip).
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        m.set_color(Some([0x33, 0x66, 0x99]));
        assert!(m.is_dirty());
        m.write(&p).unwrap();
        assert_eq!(ModMeta::read(&p).color(), Some([0x33, 0x66, 0x99]));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn download_lifecycle_flags_read_the_sidecar_form() {
        // The `true`/`false` form `write_download_meta` records is read back.
        let p = tmp_ini(concat!(
            "[General]\n",
            "version=1.2\n",
            "installed=true\nuninstalled=false\npaused=false\nremoved=false\n",
        ));
        let m = ModMeta::read(&p);
        assert!(m.installed());
        assert!(!m.uninstalled());
        assert!(!m.paused());
        assert!(!m.removed());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn download_lifecycle_setters_round_trip() {
        let p = tmp_ini("[General]\ninstalled=false\nuninstalled=false\n");
        let mut m = ModMeta::read(&p);
        m.set_installed(true);
        m.set_uninstalled(true);
        m.set_removed(true);
        assert!(m.is_dirty());
        m.write(&p).unwrap();
        let r = ModMeta::read(&p);
        assert!(r.installed());
        assert!(r.uninstalled());
        assert!(r.removed());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn endorse_track_ignore_setters_round_trip() {
        let p = tmp_ini("[General]\nendorsed=0\n");
        let mut m = ModMeta::read(&p);
        assert!(!m.endorsed());
        assert!(!m.tracked());
        assert!(!m.ignore_update());
        m.set_endorsed(true);
        m.set_tracked(true);
        m.set_ignore_update(true);
        assert!(m.is_dirty());
        m.write(&p).unwrap();
        let r = ModMeta::read(&p);
        assert!(r.endorsed());
        assert!(r.tracked());
        assert!(r.ignore_update());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn version_compare_ignores_cosmetic_differences() {
        // Same version, different spellings: no phantom update.
        assert!(same_version("1.5", "1.5.0"));
        assert!(same_version("v1.2", "1.2"));
        assert!(same_version("1.5.0.0", "1.5"));
        assert!(same_version("2-1", "2.1"));
        // Genuinely different.
        assert!(!same_version("1.5", "1.5.1"));
        assert!(!same_version("1.4", "1.5"));
        // Non-numeric tags fall back to string compare.
        assert!(same_version("1.5SE", "1.5se"));
        assert!(!same_version("1.5SE", "1.5AE"));
    }
}
