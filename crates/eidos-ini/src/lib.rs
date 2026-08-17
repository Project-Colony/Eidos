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

/// Read `[section] key` out of INI `text`, or `None` when the section or the key
/// is absent. Section and key match case-insensitively, and the first occurrence
/// wins - Bethesda INIs in the wild do carry a duplicated key, and the engine's
/// own parser keeps the first one it sees.
///
/// The value is returned RAW (everything after the first `=`, see [`key_value`]),
/// so callers that must round-trip MO2's quoting keep it and callers wanting a
/// clean value trim it. This is the read counterpart of [`set_key`].
pub fn get_key<'a>(text: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut in_section = false;
    // No trimming of the line itself: `section_header` and `key_value` already trim
    // what they must, so the value comes back exactly as written.
    for line in text.lines() {
        if let Some(s) = section_header(line) {
            in_section = s.eq_ignore_ascii_case(section);
            continue;
        }
        if in_section {
            if let Some((k, v)) = key_value(line) {
                if k.eq_ignore_ascii_case(key) {
                    return Some(v);
                }
            }
        }
    }
    None
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
    // Matched through `section_header`, the same primitive `get_key` reads with -
    // never by comparing the raw line to `[name]`. The raw comparison missed a
    // header written `[ Archive ]` (inner spaces), so a set created a SECOND
    // `[Archive]` section at the end while get_key kept reading the first one:
    // write "1", read back "0", demonstrated before it was fixed. One matching
    // rule for the whole crate, or the primitives disagree about which sections
    // exist.
    let section_at = lines
        .iter()
        .position(|l| section_header(l).is_some_and(|s| s.eq_ignore_ascii_case(section)));
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

/// Remove `[section] key` from INI `text`, leaving everything else - including
/// the now-possibly-empty section header - untouched. Absent key, absent section
/// and empty text are all no-ops.
///
/// The counterpart of [`set_key`] for undoing a write: restoring a key that was
/// ABSENT before means deleting it, not setting it empty, because `key=` and no
/// key at all are different to the engines (an empty `sResourceDataDirsFinal` is
/// the whole point of that tweak, for instance).
pub fn delete_key(text: &str, section: &str, key: &str) -> String {
    let nl = newline_style(text);
    let mut in_section = false;
    let mut out: Vec<&str> = Vec::new();
    let mut removed = false;

    for line in text.lines() {
        // Same matching rule as `get_key` and `set_key`: the section NAME via
        // `section_header`, never the raw line - see the comment in `set_key`.
        if let Some(s) = section_header(line) {
            in_section = s.eq_ignore_ascii_case(section);
            out.push(line);
            continue;
        }
        // Only the FIRST match goes: a duplicate key later in the section was
        // already dead to the parser, and dropping it too would change more than
        // this call was asked to.
        if in_section && !removed {
            if let Some((k, _)) = key_value(line) {
                if k.eq_ignore_ascii_case(key) {
                    removed = true;
                    continue;
                }
            }
        }
        out.push(line);
    }
    if !removed {
        return text.to_string();
    }
    let mut joined = out.join(nl);
    if !joined.is_empty() {
        joined.push_str(nl);
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_key_removes_only_the_named_key_in_the_named_section() {
        let text = "[Display]\niSize W=1920\niSize H=1080\n\n[Archive]\niSize W=1\n";
        let out = delete_key(text, "Display", "isize w");
        assert_eq!(out, "[Display]\niSize H=1080\n\n[Archive]\niSize W=1\n");
        // Absent key, absent section: unchanged, and cheaply so.
        assert_eq!(delete_key(text, "Display", "nope"), text);
        assert_eq!(delete_key(text, "Nope", "iSize W"), text);
        assert_eq!(delete_key("", "A", "b"), "");
    }

    #[test]
    fn delete_key_keeps_crlf() {
        let out = delete_key("[A]\r\nx=1\r\ny=2\r\n", "A", "x");
        assert_eq!(out, "[A]\r\ny=2\r\n");
    }

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
    fn get_key_reads_the_right_section() {
        let src = "[Display]\r\nsResourceArchiveList=wrong.bsa\r\n\r\n[Archive]\r\n\
                   sResourceArchiveList= a.bsa, b.bsa \r\nsResourceArchiveList2=c.bsa\r\n";
        // Section-scoped: the same key in another section is not picked up.
        assert_eq!(get_key(src, "Archive", "sResourceArchiveList"), Some(" a.bsa, b.bsa "));
        assert_eq!(get_key(src, "archive", "SRESOURCEARCHIVELIST2"), Some("c.bsa")); // case-insensitive
        assert_eq!(get_key(src, "Archive", "missing"), None);
        assert_eq!(get_key(src, "Missing", "sResourceArchiveList"), None);
        assert_eq!(get_key("", "Archive", "k"), None);
    }

    #[test]
    fn get_key_survives_malformed_input() {
        // Truncated mid-write: an unterminated header, a bare key, a stray value.
        let src = "[Archive\nnoequals\n=orphanvalue\n[Archive]\nk=v";
        assert_eq!(get_key(src, "Archive", "k"), Some("v")); // last line, no trailing newline
        assert_eq!(get_key(src, "Archive", ""), None); // the `=orphanvalue` line is in no section
        // A duplicated key keeps the first, like the engine's parser.
        assert_eq!(get_key("[A]\nk=1\nk=2\n", "A", "k"), Some("1"));
    }

    #[test]
    fn all_three_primitives_agree_on_which_sections_exist() {
        // `[ Archive ]` with inner spaces: get_key always matched it (it trims
        // the extracted name), while set_key and delete_key compared the raw
        // line to `[Archive]` and missed - so a set created a DUPLICATE section
        // at the end and get_key kept answering from the first one. Write "1",
        // read back "0". One matching rule for the whole crate.
        let text = "[ Archive ]\nbInvalidateOlderFiles=0\n";
        let out = set_key(text, "Archive", "bInvalidateOlderFiles", "1");
        assert_eq!(get_key(&out, "Archive", "bInvalidateOlderFiles"), Some("1"), "{out}");
        assert_eq!(out.matches("Archive").count(), 1, "no duplicate section: {out}");
        let gone = delete_key(text, "Archive", "bInvalidateOlderFiles");
        assert_eq!(get_key(&gone, "Archive", "bInvalidateOlderFiles"), None, "{gone}");
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
