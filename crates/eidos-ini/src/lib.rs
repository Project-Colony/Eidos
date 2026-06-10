//! A tiny shared INI toolkit: the low-level primitives Eidos's three INI users
//! build on - the per-mod `meta.ini` reader, the instance manifest, and the
//! game-INI editor - so the section / key / newline handling lives in one place.
//!
//! MO2 leans on Qt's single `QSettings` for the same reason. Eidos has no Qt, and
//! each consumer keeps its own *fit-for-purpose* value handling on top of these
//! primitives (notably MO2 `meta.ini` values are kept RAW so the file round-trips
//! byte-for-byte), so this is deliberately a set of primitives, not a document
//! model that would re-escape values and corrupt MO2 files.

/// The newline style of INI text: `"\r\n"` if it contains any CRLF, else `"\n"`.
/// Bethesda and MO2 INIs are CRLF; preserving the style avoids churning files.
pub fn newline_style(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// If `line` is a section header `[name]`, returns the trimmed `name` (no
/// brackets). INI section names are matched case-insensitively by callers.
pub fn section_header(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[').and_then(|s| s.strip_suffix(']')).map(str::trim)
}

/// Split a `key=value` line into `(trimmed key, raw value)`. The value keeps
/// everything after the first `=` verbatim - callers that must preserve MO2's
/// quoted/escaped values (`meta.ini`) rely on this; callers wanting a clean value
/// trim it themselves.
pub fn key_value(line: &str) -> Option<(&str, &str)> {
    line.split_once('=').map(|(k, v)| (k.trim(), v))
}

/// Set `[section] key=value` in INI `text`: update the key in place if present,
/// else add it to the section (creating the section, or the whole document, if
/// needed). Everything else is preserved, including the newline style. Section
/// and key match case-insensitively. This is the engine behind the game-INI
/// editor (BSA invalidation, per-profile save redirection).
pub fn set_key(text: &str, section: &str, key: &str, value: &str) -> String {
    let nl = newline_style(text);
    let mut lines: Vec<String> = text.lines().map(String::from).collect();

    let header = format!("[{section}]");
    let section_at = lines.iter().position(|l| l.trim().eq_ignore_ascii_case(&header));
    let new_line = format!("{key}={value}");

    match section_at {
        Some(start) => {
            // Search the section body (until the next section header or EOF).
            let mut key_at = None;
            for (i, line) in lines.iter().enumerate().skip(start + 1) {
                if section_header(line).is_some() {
                    break;
                }
                if let Some((k, _)) = key_value(line) {
                    if k.eq_ignore_ascii_case(key) {
                        key_at = Some(i);
                        break;
                    }
                }
            }
            match key_at {
                Some(i) => lines[i] = new_line,
                None => lines.insert(start + 1, new_line),
            }
        }
        None => {
            lines.push(header);
            lines.push(new_line);
        }
    }

    let mut out = lines.join(nl);
    out.push_str(nl);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newline_detection() {
        assert_eq!(newline_style("a\r\nb"), "\r\n");
        assert_eq!(newline_style("a\nb"), "\n");
        assert_eq!(newline_style(""), "\n");
    }

    #[test]
    fn section_and_key_value() {
        assert_eq!(section_header("  [Archive]  "), Some("Archive"));
        assert_eq!(section_header("key=val"), None);
        // The key is trimmed; the value is raw (spaces and all).
        assert_eq!(key_value("  k = 1 "), Some(("k", " 1 ")));
        assert_eq!(key_value("noequals"), None);
    }

    #[test]
    fn set_key_creates_updates_and_preserves() {
        // Create in an empty document.
        let s = set_key("", "Archive", "bInvalidateOlderFiles", "1");
        assert!(s.contains("[Archive]"));
        assert!(s.contains("bInvalidateOlderFiles=1"));

        // Update in place, keeping siblings, the other section, and CRLF.
        let src = "[Display]\r\niSize W=1920\r\n\r\n[Archive]\r\nbInvalidateOlderFiles=0\r\nsResourceArchiveList=x.bsa\r\n";
        let out = set_key(src, "Archive", "bInvalidateOlderFiles", "1");
        assert!(out.contains("bInvalidateOlderFiles=1"));
        assert!(!out.contains("bInvalidateOlderFiles=0"));
        assert!(out.contains("iSize W=1920"));
        assert!(out.contains("sResourceArchiveList=x.bsa"));
        assert!(out.contains("\r\n"));

        // Add a key to an existing section.
        let out2 = set_key("[Archive]\nsResourceArchiveList=x.bsa\n", "Archive", "bInvalidateOlderFiles", "1");
        assert!(out2.contains("bInvalidateOlderFiles=1"));
        assert!(out2.contains("sResourceArchiveList=x.bsa"));
    }
}
