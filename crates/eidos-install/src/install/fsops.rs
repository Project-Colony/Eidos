//! Filesystem plumbing shared by every install path: merges, overlays,
//! case-collision repair, case-insensitive resolution.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::SystemTime;

use super::*;

/// Move every top-level entry of `src` into `dest` (rename, so same-filesystem
/// installs are instant).
pub(crate) fn move_dir_contents(src: &Path, dest: &Path) -> io::Result<()> {
    for e in fs::read_dir(src)?.flatten() {
        fs::rename(e.path(), dest.join(e.file_name()))?;
    }
    Ok(())
}

pub(crate) fn is_nonempty_dir(p: &Path) -> bool {
    fs::read_dir(p)
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false)
}

/// Find a directory entry by case-insensitive name.
pub(crate) fn find_ci(dir: &Path, name_lower: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name_lower)
        })
        .map(|e| e.path())
}

/// Resolve a `/`-joined relative path under `root`, matching each component
/// case-insensitively (FOMOD sources are Windows-cased and may not match on disk).
pub(crate) fn resolve_ci(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for part in rel.split(['/', '\\']).filter(|s| !s.is_empty()) {
        // Security: refuse a `..` segment so an attacker-controlled FOMOD source
        // path can't read files outside the extracted archive root.
        if part == ".." {
            return None;
        }
        let exact = cur.join(part);
        if exact.exists() {
            cur = exact;
            continue;
        }
        cur = find_ci(&cur, &part.to_ascii_lowercase())?;
    }
    Some(cur)
}

pub(crate) fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)?.flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Copy every entry of `src` over `dst`, later-wins. Unlike [`copy_dir_all`] this
/// tolerates a TYPE conflict between two overlays - a file landing where an earlier
/// sub-package left a directory, or the reverse - by dropping the loser, because
/// BAIN's contract is that the later sub-package wins outright. Without it,
/// `fs::copy` onto a directory fails EISDIR and aborts an otherwise fine install.
///
/// Symlinks are treated as opaque entries (never recursed into), so a symlink loop
/// inside a crafted archive cannot make this recurse forever.
pub(crate) fn overlay_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)?.flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        // Whatever occupies the name loses, unless both sides are directories (which
        // merge). `symlink_metadata` deliberately does not follow: a DANGLING symlink
        // still occupies the name, and - the reason this matters - removing the link
        // before writing is what stops the copy below from being redirected THROUGH a
        // symlink an earlier sub-package planted, which would write outside the mod.
        let occupant = fs::symlink_metadata(&to).map(|m| m.file_type()).ok();
        let both_dirs = is_real_dir(&from) && occupant.is_some_and(|t| t.is_dir());
        if !both_dirs {
            match occupant {
                Some(t) if t.is_dir() => fs::remove_dir_all(&to)?,
                Some(_) => fs::remove_file(&to)?,
                None => {}
            }
        }
        if is_real_dir(&from) {
            overlay_dir(&from, &to)?;
            continue;
        }
        // `read_link` succeeds only for a symlink (EINVAL otherwise), so this is the
        // discriminator. Recreate the link instead of copying through it: the link is
        // the content, and copying a DANGLING one would fail the whole install over an
        // entry the simple path (a plain rename) would have installed fine.
        match fs::read_link(&from) {
            Ok(target) => std::os::unix::fs::symlink(target, &to)?,
            Err(_) => {
                fs::copy(&from, &to)?;
            }
        }
    }
    Ok(())
}

/// Clear whatever occupies `p` when it is not a directory, so a directory can be
/// created or overlaid there.
///
/// `overlay_dir` clears occupants of its children but NOT of its own destination -
/// its first statement is `create_dir_all(dst)`, which is `EEXIST` on a regular
/// file. Every pre-existing caller passes a freshly created mod directory, so the
/// gap never mattered; the root split is the first to pass an archive-named path.
/// `symlink_metadata` does not follow, so a dangling link counts as an occupant.
pub(crate) fn clear_non_dir(p: &Path) -> io::Result<()> {
    match fs::symlink_metadata(p).map(|m| m.file_type()) {
        Ok(t) if !t.is_dir() => fs::remove_file(p),
        _ => Ok(()),
    }
}

/// Collapse every directory's case-colliding children into a single canonical,
/// lower-cased entry, recursively and in place. Best-effort and idempotent.
///
/// Resolution rules (NTFS-equivalent, deterministic):
/// - dir + dir: merge children into one lower-cased dir, recurse.
/// - file + file: keep the oldest `mtime` (the author's original; later same-name
///   entries are usually repack dupes), breaking ties by lexicographic name.
/// - file + dir: the file wins the canonical name; the dir is moved aside to
///   `<name>_dir` so its contents are never dropped.
/// - symlinks: treated as opaque, name-only entries; never followed (no loops).
/// - non-UTF8 names: logged and skipped; siblings are still normalised.
pub(crate) fn normalize_case_collisions(dir: &Path) -> io::Result<()> {
    resolve_dir_collisions(dir)?;
    // Recurse into the now-collision-free real subdirectories so nested collisions
    // (and collisions inside dirs merged above) settle too.
    for e in fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if is_real_dir(&p) {
            normalize_case_collisions(&p)?;
        }
    }
    Ok(())
}

/// Resolve case-collisions among the immediate children of `dir` (one level).
pub(crate) fn resolve_dir_collisions(dir: &Path) -> io::Result<()> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for e in fs::read_dir(dir)?.flatten() {
        match e.file_name().to_str() {
            Some(s) => groups
                .entry(s.to_ascii_lowercase())
                .or_default()
                .push(e.path()),
            None => eprintln!(
                "eidos install: skipping case-normalisation of non-UTF8 name {:?}",
                e.file_name()
            ),
        }
    }
    for (key, members) in groups {
        if members.len() > 1 {
            resolve_group(dir, &key, members)?;
        }
    }
    Ok(())
}

/// Collapse one collision group (>= 2 entries whose names lower-case to `key`,
/// `existing` ones living in `parent`) into a single canonical entry in `parent`.
pub(crate) fn resolve_group(parent: &Path, key: &str, members: Vec<PathBuf>) -> io::Result<()> {
    let (dirs, opaques): (Vec<PathBuf>, Vec<PathBuf>) =
        members.into_iter().partition(|p| is_real_dir(p));

    if opaques.is_empty() {
        // All directories: merge into one lower-cased canonical dir.
        merge_dirs_into(parent, key, &dirs)?;
        return Ok(());
    }

    // A file/symlink wins the canonical name. Any colliding directories move aside
    // to `<key>_dir` so their contents survive.
    if !dirs.is_empty() {
        merge_dirs_into(parent, &format!("{key}_dir"), &dirs)?;
    }
    let survivor = pick_oldest(&opaques);
    for o in &opaques {
        if *o != survivor {
            fs::remove_file(o)?; // removes the symlink/regular file, never a target
        }
    }
    rename_if_needed(&survivor, &parent.join(key))
}

/// Merge `dirs` (case-variants of one name in `parent`) into a single directory
/// named `target_name`. Staged under a fresh temp name first so an in-place rename
/// can never clobber a doomed sibling on case-sensitive ext4, then published.
pub(crate) fn merge_dirs_into(
    parent: &Path,
    target_name: &str,
    dirs: &[PathBuf],
) -> io::Result<()> {
    let staging = parent.join(format!(
        ".eidos-case-{}",
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&staging)?;
    for d in dirs {
        merge_into(d, &staging)?;
        // `d` should be empty now; remove it. If a skipped non-UTF8 collision left a
        // child behind, leave the residual dir (no data loss) and note it.
        if fs::remove_dir(d).is_err() {
            eprintln!(
                "eidos install: left residual dir after case-merge: {}",
                d.display()
            );
        }
    }
    // Belt-and-braces: settle any collision the staged union still holds.
    resolve_dir_collisions(&staging)?;
    publish_staging(parent, target_name, &staging)
}

/// Publish a finished `staging` directory under `parent/target_name`. If an entry
/// already holds that name: a directory is merged into (case folds together); a
/// FILE or symlink is left intact and the staged union lands at the first free
/// `target_name_<n>` instead - so neither the pre-existing entry nor the merged
/// contents are ever lost (the alternative `merge_into` on a file is ENOTDIR, which
/// would abort the whole install).
pub(crate) fn publish_staging(parent: &Path, target_name: &str, staging: &Path) -> io::Result<()> {
    let target = parent.join(target_name);
    if target.exists() {
        if is_real_dir(&target) {
            merge_into(staging, &target)?;
            let _ = fs::remove_dir_all(staging);
            return Ok(());
        }
        // Occupied by a file/symlink: find the first free suffixed name.
        let mut n = 1;
        let free = loop {
            let cand = parent.join(format!("{target_name}_{n}"));
            if !cand.exists() {
                break cand;
            }
            n += 1;
        };
        return fs::rename(staging, free);
    }
    fs::rename(staging, &target)
}

/// Move every child of `src` into `dst` (both existing dirs), resolving a
/// case-insensitive collision with an existing `dst` child by the same rules.
pub(crate) fn merge_into(src: &Path, dst: &Path) -> io::Result<()> {
    for e in fs::read_dir(src)?.flatten() {
        let name = e.file_name();
        let child = e.path();
        let key = match name.to_str() {
            Some(s) => s.to_ascii_lowercase(),
            None => {
                // Non-UTF8: best-effort move as-is; skip (don't clobber) if taken.
                let target = dst.join(&name);
                if !target.exists() {
                    fs::rename(&child, &target)?;
                } else {
                    eprintln!("eidos install: skipping non-UTF8 case-merge of {child:?}");
                }
                continue;
            }
        };
        match ci_find(dst, &key)? {
            None => fs::rename(&child, dst.join(&name))?, // no collision: preserve casing
            Some(existing) => resolve_group(dst, &key, vec![existing, child])?,
        }
    }
    Ok(())
}

/// The oldest-`mtime` member, breaking ties by the lexicographically-smallest
/// file name so the choice is deterministic across runs.
pub(crate) fn pick_oldest(paths: &[PathBuf]) -> PathBuf {
    paths
        .iter()
        .min_by(|a, b| {
            mtime(a)
                .cmp(&mtime(b))
                .then_with(|| a.file_name().cmp(&b.file_name()))
        })
        .cloned()
        .expect("a collision group is never empty")
}

pub(crate) fn mtime(p: &Path) -> SystemTime {
    fs::symlink_metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// A real directory, NOT a symlink to one (symlinks are treated as opaque).
pub(crate) fn is_real_dir(p: &Path) -> bool {
    fs::symlink_metadata(p)
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false)
}

/// The existing child of `dir` whose name lower-cases to `key`, if any.
pub(crate) fn ci_find(dir: &Path, key: &str) -> io::Result<Option<PathBuf>> {
    for e in fs::read_dir(dir)?.flatten() {
        if e.file_name()
            .to_str()
            .is_some_and(|s| s.eq_ignore_ascii_case(key))
        {
            return Ok(Some(e.path()));
        }
    }
    Ok(None)
}

/// Rename `from` to `to` unless they are already the same path.
pub(crate) fn rename_if_needed(from: &Path, to: &Path) -> io::Result<()> {
    if from != to {
        fs::rename(from, to)?;
    }
    Ok(())
}
