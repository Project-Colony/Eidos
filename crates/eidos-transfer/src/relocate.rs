//! Repairing the absolute paths a move breaks.
//!
//! An instance holds a handful of values that name a place on the machine that
//! made it: the tools it can run (`tools.ini`), and the archive each mod was
//! installed from (`mods/<name>/meta.ini`). Copy the tree to `/home/you/Eidos`
//! and the mods are all there, while every tool button points at a folder that
//! does not exist. MO2 has the same problem and answers it with a portable
//! instance and relative paths; Eidos answers it here, because the manifest
//! records where the instance USED to be, and that is exactly what is needed to
//! tell a path that must be rewritten from one that was always meant to be
//! absolute.
//!
//! Only paths at or under the old instance root are touched. `/opt/xedit` is
//! left exactly as it was and reported as a tool to reinstall - guessing where
//! somebody else's xEdit went is not repair, it is invention.

use std::fs;
use std::io;
use std::path::Path;

/// What the relocation pass did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Relocated {
    /// Instance-relative paths of the files that changed.
    pub files: Vec<String>,
    /// How many individual values were rewritten.
    pub values: u64,
    /// Files that hold a value to rewrite but could not be repaired, with the
    /// reason. Left exactly as they were rather than mangled.
    pub problems: Vec<(String, String)>,
}

/// Keys in `tools.ini` whose value is, or embeds, a path into the instance.
/// `arg` stands for the numbered `arg0`, `arg1`, ... keys.
const TOOL_KEYS: [&str; 3] = ["exe", "workdir", "arg"];

/// Replace every occurrence of `from` in `s` that is followed by `/` or ends the
/// string, and nothing else.
///
/// The boundary check is the whole point: without it, an instance at
/// `/mnt/Eidos` would rewrite the middle of `/mnt/Eidos-Fallout4/mods/...` and
/// point a perfectly good path at a folder that never existed.
fn swap_root(s: &str, from: &str, to: &str) -> Option<String> {
    if from.is_empty() || from == to {
        return None;
    }
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let mut hit = false;
    while let Some(pos) = s[i..].find(from) {
        let at = i + pos;
        let end = at + from.len();
        out.push_str(&s[i..at]);
        if end == s.len() || s[end..].starts_with('/') {
            out.push_str(to);
            hit = true;
        } else {
            out.push_str(from);
        }
        i = end;
    }
    if !hit {
        return None;
    }
    out.push_str(&s[i..]);
    Some(out)
}

/// [`swap_root`] on an INI value, seeing through the quotes MO2 wraps some of
/// its values in so a quoted path is repaired too.
fn relocate_value(raw: &str, from: &str, to: &str) -> Option<String> {
    let trimmed = raw.trim();
    let quoted = trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"');
    let inner = if quoted {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    let swapped = swap_root(inner, from, to)?;
    Some(if quoted {
        format!("\"{swapped}\"")
    } else {
        swapped
    })
}

/// Whether this line carries a value to relocate, and its rewritten form.
fn relocate_line(line: &str, keys: &[&str], from: &str, to: &str) -> Option<String> {
    let (k, v) = eidos_ini::key_value(line)?;
    let matches = keys
        .iter()
        .any(|want| k.eq_ignore_ascii_case(want) || (*want == "arg" && starts_with_arg(k)));
    if !matches {
        return None;
    }
    let new = relocate_value(v, from, to)?;
    Some(format!("{k}={new}"))
}

/// `arg0`, `arg1`, ... - one key per argument, and an argument may embed a path
/// (`-D:/mnt/Jeux/Eidos-Skyrim/mods/...`).
fn starts_with_arg(k: &str) -> bool {
    k.len() > 3
        && k[..3].eq_ignore_ascii_case("arg")
        && k[3..].chars().all(|c| c.is_ascii_digit())
}

/// Rewrite one file in place, preserving its line endings byte for byte.
/// Returns how many values changed, or why it was left alone.
fn rewrite(path: &Path, keys: &[&str], from: &str, to: &str) -> Result<u64, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("could not be read as text and was left alone ({e})"))?;
    let mut out = String::with_capacity(text.len());
    let mut changed = 0u64;
    for chunk in text.split_inclusive('\n') {
        let body = chunk.trim_end_matches('\n').trim_end_matches('\r');
        let tail = &chunk[body.len()..];
        match relocate_line(body, keys, from, to) {
            Some(new) => {
                out.push_str(&new);
                changed += 1;
            }
            None => out.push_str(body),
        }
        out.push_str(tail);
    }
    if changed > 0 {
        eidos_instance::write_atomic(path, out.as_bytes())
            .map_err(|e| format!("could not be rewritten ({e})"))?;
    }
    Ok(changed)
}

/// Point an unpacked instance at where it now lives.
///
/// `from` is the old instance root, out of the backup manifest; `root` is where
/// it has just been unpacked. Doing nothing - because the two are the same, or
/// because the backup predates the manifest - is a success, not a failure.
pub fn relocate(root: &Path, from: &str) -> io::Result<Relocated> {
    let mut done = Relocated::default();
    let from = from.trim_end_matches('/');
    let to = root.to_string_lossy();
    let to = to.trim_end_matches('/');
    if from.is_empty() || from == to {
        return Ok(done);
    }

    let mut targets: Vec<(std::path::PathBuf, &[&str])> =
        vec![(root.join("tools.ini"), &TOOL_KEYS[..])];
    // Every mod's `meta.ini`, for the archive it was installed from.
    if let Ok(mods) = fs::read_dir(root.join("mods")) {
        let mut metas: Vec<std::path::PathBuf> = mods
            .flatten()
            .map(|e| e.path().join("meta.ini"))
            .filter(|p| p.is_file())
            .collect();
        metas.sort();
        targets.extend(metas.into_iter().map(|p| (p, &META_KEYS[..])));
    }

    for (path, keys) in targets {
        if !path.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        match rewrite(&path, keys, from, to) {
            Ok(0) => {}
            Ok(n) => {
                done.values += n;
                done.files.push(rel);
            }
            Err(why) => done.problems.push((rel, why)),
        }
    }
    Ok(done)
}

/// The one `meta.ini` key that names a place on the old machine.
const META_KEYS: [&str; 1] = ["installationFile"];

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "eidos-relocate-{}-{name}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_sibling_folder_with_the_same_prefix_is_not_rewritten() {
        // The defect this exists for: `/mnt/Eidos` must not rewrite the middle of
        // `/mnt/Eidos-Fallout4`, which is a real, working path of its own.
        assert_eq!(swap_root("/mnt/Eidos-Fallout4/mods/x", "/mnt/Eidos", "/new"), None);
        assert_eq!(
            swap_root("/mnt/Eidos/mods/x", "/mnt/Eidos", "/new").as_deref(),
            Some("/new/mods/x")
        );
        // The root on its own, with nothing after it.
        assert_eq!(
            swap_root("/mnt/Eidos", "/mnt/Eidos", "/new").as_deref(),
            Some("/new")
        );
    }

    #[test]
    fn an_embedded_path_inside_an_argument_is_repaired() {
        // xEdit takes `-D:<path>`; the path is not at the start of the value.
        assert_eq!(
            swap_root("-D:/old/root/mods/x", "/old/root", "/new").as_deref(),
            Some("-D:/new/mods/x")
        );
    }

    #[test]
    fn a_quoted_value_keeps_its_quotes() {
        assert_eq!(
            relocate_value("\"/old/root/a b/c\"", "/old/root", "/new").as_deref(),
            Some("\"/new/a b/c\"")
        );
    }

    #[test]
    fn only_the_keys_that_hold_paths_are_touched() {
        let keys = ["exe", "workdir", "arg"];
        assert!(relocate_line("exe=/old/x", &keys, "/old", "/new").is_some());
        assert!(relocate_line("workdir=/old/x", &keys, "/old", "/new").is_some());
        assert!(relocate_line("arg0=-D:/old/x", &keys, "/old", "/new").is_some());
        // Not a path key, even though the value looks like one.
        assert!(relocate_line("prereqs=/old/x", &keys, "/old", "/new").is_none());
        assert!(relocate_line("[Tool/X]", &keys, "/old", "/new").is_none());
        // `args` is a legacy single-line key, not `arg<N>`.
        assert!(starts_with_arg("arg0"));
        assert!(starts_with_arg("arg12"));
        assert!(!starts_with_arg("args"));
        assert!(!starts_with_arg("arg"));
    }

    #[test]
    fn crlf_survives_a_rewrite() {
        // These files round-trip through MO2, which writes CRLF. Rewriting one
        // value must not renumber every line ending in the file.
        let root = tmp("crlf");
        let p = root.join("tools.ini");
        fs::write(&p, "[Tool/X]\r\nexe=/old/root/a.exe\r\nprereqs=dotnet8\r\n").unwrap();
        assert_eq!(rewrite(&p, &TOOL_KEYS, "/old/root", "/new"), Ok(1));
        let out = fs::read_to_string(&p).unwrap();
        assert_eq!(
            out, "[Tool/X]\r\nexe=/new/a.exe\r\nprereqs=dotnet8\r\n",
            "{out:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_whole_instance_is_pointed_at_its_new_home() {
        let root = tmp("instance");
        fs::create_dir_all(root.join("mods/A")).unwrap();
        fs::create_dir_all(root.join("mods/B")).unwrap();
        fs::write(
            root.join("tools.ini"),
            "[Tool/Inside]\nexe=/old/root/mods/BodySlide/BodySlide.exe\n\n\
             [Tool/Outside]\nexe=/opt/xedit/SSEEdit.exe\n",
        )
        .unwrap();
        fs::write(
            root.join("mods/A/meta.ini"),
            "[General]\ninstallationFile=/old/root/downloads/A.7z\n",
        )
        .unwrap();
        fs::write(
            root.join("mods/B/meta.ini"),
            "[General]\ninstallationFile=/home/somebody/Downloads/B.7z\n",
        )
        .unwrap();

        let done = relocate(&root, "/old/root").unwrap();
        assert_eq!(done.values, 2);
        assert_eq!(done.files, vec!["tools.ini", "mods/A/meta.ini"]);

        let tools = fs::read_to_string(root.join("tools.ini")).unwrap();
        assert!(
            tools.contains(&format!("exe={}/mods/BodySlide/BodySlide.exe", root.display())),
            "{tools}"
        );
        // A tool that was never inside the instance is left exactly as it was:
        // guessing where somebody else's xEdit went is invention, not repair.
        assert!(tools.contains("exe=/opt/xedit/SSEEdit.exe"), "{tools}");
        let b = fs::read_to_string(root.join("mods/B/meta.ini")).unwrap();
        assert!(b.contains("/home/somebody/Downloads/B.7z"), "{b}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unpacking_where_it_came_from_changes_nothing() {
        let root = tmp("same");
        fs::write(root.join("tools.ini"), "[Tool/X]\nexe=/x/y.exe\n").unwrap();
        let same = root.to_string_lossy().into_owned();
        let done = relocate(&root, &same).unwrap();
        assert_eq!(done, Relocated::default());
        // A trailing slash on the recorded root is the same root.
        let done = relocate(&root, &format!("{same}/")).unwrap();
        assert_eq!(done, Relocated::default());
        let _ = fs::remove_dir_all(&root);
    }
}
