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
    /// Enabled INI-tweak fragment names, in the order MO2 applies them, parsed
    /// out of the `[INI Tweaks]` array that lives inside `tail`.
    ini_tweaks: Vec<String>,
    /// The BAIN sub-packages the last install ticked, so a reinstall pre-selects
    /// them (MO2's `m_PreviousOptions`). Parsed out of `[Plugins]` in `tail`.
    bain_options: Vec<String>,
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
        let ini_tweaks = parse_ini_tweaks(&tail);
        let bain_options = parse_bain_options(&tail);
        ModMeta { general, tail, ini_tweaks, bain_options, crlf, dirty: false }
    }

    /// The INI-tweak fragments the user enabled for this mod, in application
    /// order (file names relative to the mod's `INI Tweaks/` directory).
    pub fn ini_tweaks(&self) -> &[String] {
        &self.ini_tweaks
    }

    /// Replace the enabled fragment list. Empty clears the section entirely.
    pub fn set_ini_tweaks(&mut self, names: &[String]) {
        if self.ini_tweaks == names {
            return;
        }
        self.ini_tweaks = names.to_vec();
        // MO2 writes the QSettings array form: `<n>\name=<file>` one-based, plus
        // the `size=` key its `beginReadArray` keys off. Without `size` MO2 reads
        // the section back as empty.
        // An empty list drops the section rather than writing `size=0`, which MO2
        // would read back as an empty array anyway.
        let body: Vec<String> = if self.ini_tweaks.is_empty() {
            Vec::new()
        } else {
            self.ini_tweaks
                .iter()
                .enumerate()
                .map(|(n, name)| format!("{}\\name={name}", n + 1))
                .chain(std::iter::once(format!("size={}", self.ini_tweaks.len())))
                .collect()
        };
        self.replace_section("INI Tweaks", &body);
        self.dirty = true;
    }

    /// The BAIN sub-packages the last install of this mod ticked, in install
    /// order, so a reinstall can pre-select them.
    pub fn bain_options(&self) -> &[String] {
        &self.bain_options
    }

    /// Record the ticked BAIN sub-packages. Empty clears the section.
    pub fn set_bain_options(&mut self, options: &[String]) {
        if self.bain_options == options {
            return;
        }
        self.bain_options = options.to_vec();
        let body: Vec<String> = self
            .bain_options
            .iter()
            .enumerate()
            .map(|(n, name)| format!("{BAIN_KEY_PREFIX}option{n}={name}"))
            .collect();
        self.replace_section("Plugins", &body);
        self.dirty = true;
    }

    /// Replace one named section of `tail` with `body` (the lines under its
    /// header), leaving every other byte verbatim. An empty `body` removes the
    /// section; a section that was not there is appended.
    ///
    /// The span is recomputed on each call rather than cached, so rewriting one
    /// section cannot leave another's offsets pointing at the wrong bytes.
    fn replace_section(&mut self, name: &str, body: &[String]) {
        let eol = if self.crlf { "\r\n" } else { "\n" };
        let mut section = String::new();
        if !body.is_empty() {
            section.push_str(&format!("[{name}]{eol}"));
            for line in body {
                section.push_str(line);
                section.push_str(eol);
            }
        }
        match section_span(&self.tail, name) {
            Some((start, end)) => self.tail.replace_range(start..end, &section),
            None if section.is_empty() => {}
            None => {
                // A tail that does not end in a newline would swallow our header
                // into its last line.
                if !self.tail.is_empty() && !self.tail.ends_with('\n') {
                    self.tail.push_str(eol);
                }
                self.tail.push_str(&section);
            }
        }
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

    /// Set the mod's categories: `primary` first, then the rest, in MO2's on-disk
    /// form. `None` writes the uncategorised placeholder.
    ///
    /// The value is QUOTED, and that is not cosmetic. MO2 stores this through
    /// QSettings, where an unquoted value containing a comma is a string LIST:
    /// written bare, `14,9,` comes back as a QStringList and MO2's
    /// `value("category").toString()` yields nothing, silently uncategorising the
    /// mod. Every `category=` MO2 itself writes is quoted (`category="-1,"`), so
    /// this reproduces it exactly. The trailing comma is MO2's too - its parser
    /// splits on `,` and drops the empty tail.
    pub fn set_categories(&mut self, primary: Option<i32>, others: &[i32]) {
        let raw = crate::categories::format_categories(primary, others);
        self.set("category", &format!("\"{raw}\""));
    }

    /// Whether Nexus still serves this mod's page, as of the last update check.
    ///
    /// `None` means never checked - which is NOT the same as "available", and the
    /// row must not draw a warning for a mod nobody has asked about yet.
    pub fn nexus_available(&self) -> Option<bool> {
        match self.raw("nexusAvailable").map(str::trim) {
            Some("1") | Some("true") => Some(true),
            Some("0") | Some("false") => Some(false),
            _ => None,
        }
    }

    /// Record what the last update check saw. Not an MO2 field: MO2 tracks file
    /// status per FILE, which costs a second request per mod; this is the
    /// mod-level answer that the update check already receives for free.
    pub fn set_nexus_available(&mut self, available: bool) {
        self.set("nexusAvailable", if available { "1" } else { "0" });
    }

    pub fn installation_file(&self) -> Option<String> {
        self.string("installationFile")
    }

    /// The download sidecar's `modName` (the mod's display name on Nexus).
    pub fn mod_name(&self) -> Option<String> {
        self.string("modName")
    }

    /// The archive's total size in bytes, written when the download STARTS.
    ///
    /// Not an MO2 field. MO2 does not need one: its own process owns the
    /// transfer and knows the content length in memory. Eidos downloads in a
    /// separate `eidos nxm` process, so the size has to be on disk for the
    /// window to draw a percentage - otherwise a running download can only be
    /// reported as a number of bytes that keeps going up, with no end in sight.
    pub fn total_size(&self) -> Option<u64> {
        self.raw("totalsize").and_then(|v| v.trim().parse().ok()).filter(|&n| n != 0)
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
        // Atomic (tmp + rename): a meta.ini carries user labour - notes,
        // endorsements, tweak selections - and a crash mid-write would replace
        // the mod's only copy of it with a torn file.
        crate::write_atomic(path, out.as_bytes())
    }
}

/// The byte range a named section occupies in `text`, from its header line to
/// the byte before the next header (or EOF). `None` if the section is absent.
fn section_span(text: &str, name: &str) -> Option<(usize, usize)> {
    let mut start = None;
    let mut pos = 0usize;
    for seg in text.split_inclusive('\n') {
        let body = seg.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(header) = eidos_ini::section_header(body) {
            if start.is_some() {
                return start.map(|s| (s, pos));
            }
            if header.eq_ignore_ascii_case(name) {
                start = Some(pos);
            }
        }
        pos += seg.len();
    }
    start.map(|s| (s, text.len()))
}

/// The `key=value` lines of a named section, in file order.
fn section_entries<'a>(text: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let Some((start, end)) = section_span(text, name) else { return Vec::new() };
    text[start..end]
        .lines()
        .skip(1) // the header itself
        .filter_map(eidos_ini::key_value)
        .collect()
}

/// Pull the `[INI Tweaks]` array out of a `meta.ini` tail: the enabled fragment
/// names in array order.
///
/// The on-disk shape is Qt's `QSettings` array: `1\name=foo.ini`, `2\name=bar.ini`,
/// `size=2`. Entries are ordered by their index, not by the order they appear,
/// and a gap in the numbering is simply skipped rather than treated as an error -
/// a hand-edited file should still load.
fn parse_ini_tweaks(tail: &str) -> Vec<String> {
    let mut entries: Vec<(u32, &str)> = section_entries(tail, "INI Tweaks")
        .into_iter()
        .filter_map(|(k, v)| {
            let idx = k.strip_suffix("\\name")?.trim().parse::<u32>().ok()?;
            Some((idx, v.trim())).filter(|(_, v)| !v.is_empty())
        })
        .collect();
    entries.sort_by_key(|(i, _)| *i);
    entries.into_iter().map(|(_, n)| n.to_string()).collect()
}

/// How MO2 names the BAIN installer's per-mod settings inside `[Plugins]`: Qt's
/// `QSettings` writes the nested group as a `\`-joined key and percent-escapes the
/// space, giving `BAIN%20Installer\option0`. Reading accepts the unescaped
/// spelling too, so a hand-edited file still loads.
const BAIN_KEY_PREFIX: &str = "BAIN%20Installer\\";

/// The BAIN sub-package selection recorded by the last install, in index order
/// (MO2's `optionN` keys under `[Plugins]`).
fn parse_bain_options(tail: &str) -> Vec<String> {
    let mut entries: Vec<(u32, &str)> = section_entries(tail, "Plugins")
        .into_iter()
        .filter_map(|(k, v)| {
            let rest = k
                .strip_prefix(BAIN_KEY_PREFIX)
                .or_else(|| k.strip_prefix("BAIN Installer\\"))?;
            let idx = rest.strip_prefix("option")?.trim().parse::<u32>().ok()?;
            Some((idx, v.trim())).filter(|(_, v)| !v.is_empty())
        })
        .collect();
    entries.sort_by_key(|(i, _)| *i);
    entries.into_iter().map(|(_, n)| n.to_string()).collect()
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
    fn ini_tweaks_round_trip_in_mo2s_array_form() {
        // MO2 writes the QSettings array with `size` last and the entries in the
        // order it applies them. Note the deliberately out-of-order indices: the
        // index is what orders them, not the line position.
        let p = tmp_ini(concat!(
            "[General]\r\n",
            "modid=1\r\n",
            "\r\n",
            "[INI Tweaks]\r\n",
            "2\\name=fov.ini\r\n",
            "1\\name=shadows.ini\r\n",
            "size=2\r\n",
            "\r\n",
            "[installedFiles]\r\n",
            "size=0\r\n",
        ));
        let mut m = ModMeta::read(&p);
        assert_eq!(m.ini_tweaks(), ["shadows.ini", "fov.ini"]);

        m.set_ini_tweaks(&["shadows.ini".into()]);
        m.write(&p).unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("1\\name=shadows.ini\r\n"), "{text}");
        assert!(!text.contains("fov.ini"), "{text}");
        assert!(text.contains("size=1\r\n"), "{text}");
        // The sections around it are untouched, CRLF included.
        assert!(text.contains("[installedFiles]\r\nsize=0\r\n"), "{text}");
        assert_eq!(ModMeta::read(&p).ini_tweaks(), ["shadows.ini"]);
        fs::remove_file(&p).ok();
    }

    #[test]
    fn enabling_a_tweak_on_a_file_that_has_no_such_section_appends_one() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        assert!(m.ini_tweaks().is_empty());
        m.set_ini_tweaks(&["a.ini".into(), "b.ini".into()]);
        m.write(&p).unwrap();
        let reread = ModMeta::read(&p);
        assert_eq!(reread.ini_tweaks(), ["a.ini", "b.ini"]);
        // And everything that was already there survived.
        assert_eq!(reread.mod_id(), Some(32117));
        assert!(fs::read_to_string(&p).unwrap().contains("[installedFiles]"));

        // Clearing the list removes the section rather than leaving `size=0`,
        // which MO2 would read back as an empty array anyway.
        let mut m = ModMeta::read(&p);
        m.set_ini_tweaks(&[]);
        m.write(&p).unwrap();
        assert!(!fs::read_to_string(&p).unwrap().contains("[INI Tweaks]"));
        fs::remove_file(&p).ok();
    }

    #[test]
    fn bain_options_round_trip_and_do_not_disturb_the_tweaks_section() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        m.set_ini_tweaks(&["a.ini".into()]);
        m.set_bain_options(&["00 Core".into(), "03 Alt textures".into()]);
        m.write(&p).unwrap();

        // Two sections were spliced into the same tail; rewriting one must not
        // have shifted the other's bytes out from under it.
        let reread = ModMeta::read(&p);
        assert_eq!(reread.ini_tweaks(), ["a.ini"]);
        assert_eq!(reread.bain_options(), ["00 Core", "03 Alt textures"]);
        assert_eq!(reread.mod_id(), Some(32117));
        assert!(fs::read_to_string(&p).unwrap().contains("[installedFiles]"));
        fs::remove_file(&p).ok();
    }

    #[test]
    fn a_hand_written_unescaped_bain_key_still_reads() {
        let p = tmp_ini("[General]\nmodid=1\n\n[Plugins]\nBAIN Installer\\option0=00 Core\n");
        assert_eq!(ModMeta::read(&p).bain_options(), ["00 Core"]);
        fs::remove_file(&p).ok();
    }

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
    fn never_checked_is_not_the_same_as_available() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        // A mod nobody has asked about must not draw a warning, and must not be
        // claimed to be fine either.
        assert_eq!(m.nexus_available(), None);

        m.set_nexus_available(false);
        m.write(&p).unwrap();
        assert_eq!(ModMeta::read(&p).nexus_available(), Some(false));
        let mut m = ModMeta::read(&p);
        m.set_nexus_available(true);
        m.write(&p).unwrap();
        assert_eq!(ModMeta::read(&p).nexus_available(), Some(true));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn set_categories_writes_mo2s_quoted_comma_list() {
        let p = tmp_ini(SAMPLE);
        let mut m = ModMeta::read(&p);
        assert_eq!(m.category().as_deref(), Some("-1,"));

        m.set_categories(Some(9), &[27, 43]);
        m.write(&p).unwrap();
        // Quoted, or QSettings reads it back as a LIST and MO2 shows no category.
        let on_disk = fs::read_to_string(&p).unwrap();
        assert!(on_disk.contains("category=\"9,27,43,\"\r\n"), "{on_disk}");
        assert_eq!(on_disk, SAMPLE.replace("category=\"-1,\"", "category=\"9,27,43,\""));

        // And it round-trips through our own reader.
        let back = ModMeta::read(&p);
        assert_eq!(crate::categories::parse_primary(&back.category().unwrap()), Some(9));
        assert_eq!(crate::categories::parse_all(&back.category().unwrap()), vec![9, 27, 43]);

        // Clearing goes back to exactly what MO2 writes for uncategorised.
        let mut m = ModMeta::read(&p);
        m.set_categories(None, &[]);
        m.write(&p).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), SAMPLE);
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
