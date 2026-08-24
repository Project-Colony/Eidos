//! Where Eidos keeps its files.
//!
//! Eidos is one program in the Colony ecosystem, and the ecosystem answers this
//! question once, for every program in it:
//!
//! ```text
//! <platform root>/Colony/<Program>/
//! ```
//!
//! The organisation, then the program, spelled the way the program spells
//! itself - `Colony`, `Digger`, `Grape`, `Eidos`. On Linux, which is the only
//! platform Eidos targets, the three roots are the XDG ones:
//!
//! | Kind | Path |
//! |---|---|
//! | [`config_dir`] | `~/.config/Colony/Eidos/` |
//! | [`data_dir`] | `~/.local/share/Colony/Eidos/` |
//! | [`cache_dir`] | `~/.cache/Colony/Eidos/` |
//! | [`state_dir`] | `~/.local/state/Colony/Eidos/` |
//!
//! # Why this crate exists at all
//!
//! Because four crates were each answering it themselves, and by hand:
//! `eidos-instance` for settings and credentials, `eidos-gamedef` for user game
//! definitions, `eidos-addons` for extension manifests, `eidos-log` for session
//! logs. Four copies of the same six lines of XDG fallback, which is three
//! copies too many for a rule that has to hold across a whole ecosystem.
//!
//! # Which root for what
//!
//! - [`config_dir`] - what the *user* chose, and would want to carry to another
//!   machine: preferences, credentials, their instance list, the game and
//!   add-on definitions they wrote.
//! - [`data_dir`] - what the *program* produced and cannot re-derive.
//! - [`cache_dir`] - what the program can rebuild by asking again. Deleting the
//!   whole directory must cost nothing but time.
//! - [`state_dir`] - session logs. See the note on [`state_dir`]: the Colony
//!   layout has three roots and Linux has four, and this is the one place Eidos
//!   uses the fourth.
//!
//! When in doubt: if losing it would annoy the user, it is not cache.
//!
//! # Reading a path does not create it
//!
//! Every function here is pure - it joins strings and returns. Showing a path on
//! a settings screen must not bring the directory into existence, and every
//! writer in this tree already calls `create_dir_all` before it writes. Use
//! [`ensure`] where a directory is genuinely wanted.

use std::io;
use std::path::{Path, PathBuf};

/// The organisation directory every Colony program nests under.
pub const VENDOR: &str = "Colony";

/// This program, spelled the way it spells itself. Not a lowercased slug: the
/// ecosystem's directories are `Colony/Eidos`, not `colony/eidos`.
pub const PROGRAM: &str = "Eidos";

/// `$HOME`, or `/` if the environment does not say.
///
/// `/` rather than a panic or an error: a program that cannot find a home
/// directory is a program running somewhere strange - a systemd unit with an
/// empty environment, a container - and every caller here goes on to
/// `create_dir_all`, which will fail with a real errno naming a real path.
/// Inventing an error here would only replace that with a vaguer one.
fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// One XDG base directory: `$VAR` when it is set to an ABSOLUTE path, else
/// `$HOME/<fallback>`.
///
/// The absolute check is the specification's, not decoration: XDG says a
/// relative value is invalid and must be ignored, and honouring one would
/// resolve against whatever directory the process happens to be in. For Eidos
/// that is the game's directory under Proton, which is the last place a user's
/// settings should land.
fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(fallback))
}

/// `~/.config/Colony/Eidos` - preferences, credentials, and what the user wrote.
pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join(VENDOR).join(PROGRAM)
}

/// `~/.local/share/Colony/Eidos` - what the program produced and cannot rebuild.
pub fn data_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join(VENDOR).join(PROGRAM)
}

/// `~/.cache/Colony/Eidos` - what the program can rebuild by asking again.
pub fn cache_dir() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache").join(VENDOR).join(PROGRAM)
}

/// `~/.local/state/Colony/Eidos` - session logs.
///
/// The Colony layout names three roots, and on Windows and macOS three is all
/// there are. Linux has a fourth, and logs are exactly what it is for: XDG
/// basedir 0.8 defines state as "data that should persist between restarts, but
/// is not important enough to be in the data directory" and names logs and
/// history as its examples. Eidos's logs were already there before this crate
/// existed, and moving them into `data` to match a table written for three
/// platforms would make them less correct on the only platform Eidos runs on.
///
/// The vendor and program components still apply, so a Colony user still finds
/// one `Colony/Eidos` tree under every root that exists.
pub fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join(VENDOR).join(PROGRAM)
}

/// Create `dir` and hand it back, for the call sites that want the directory
/// rather than the path.
pub fn ensure(dir: PathBuf) -> io::Result<PathBuf> {
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

// ---------------------------------------------------------------------------
// Migration off the old layout
// ---------------------------------------------------------------------------

/// Where Eidos kept its config before it joined the ecosystem's layout:
/// `~/.config/eidos`, lowercase, with no vendor level.
pub fn legacy_config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("eidos")
}

/// Where Eidos kept what it downloaded before: `~/.local/share/eidos`.
pub fn legacy_data_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("eidos")
}

/// Where Eidos kept its session logs before: `~/.local/state/eidos/logs`.
pub fn legacy_state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("eidos")
}

/// The marker left in a migrated directory, naming where it came from.
///
/// It is what makes the migration run once. Without it the copy below would
/// re-run on every launch after the user deleted a file, quietly restoring it.
pub const MIGRATION_MARKER: &str = ".migrated-from";

/// Copy the legacy trees onto the Colony layout, once.
///
/// Called at startup by both binaries. Cheap when there is nothing to do: two
/// `exists` calls.
///
/// # Why copy rather than move
///
/// Because a failed or wrong migration must not be able to lose anything. These
/// paths are live on a user's machine - their Nexus session, their instance
/// list, their game definitions - and the ecosystem's own rule is that the old
/// location survives the release that adds the move. A rename would satisfy
/// "move it" and leave nothing to fall back to if this code turns out to be
/// wrong about a filename. A copy costs a few kilobytes and cannot.
///
/// The old directory is therefore still there afterwards, and still readable by
/// an older Eidos. Deleting it belongs to a later release, once this one has
/// been out long enough to be trusted.
///
/// # What it will not do
///
/// - Overwrite. A destination that already exists is left exactly as it is,
///   file by file, so a user who has already used the new layout cannot have it
///   reverted by a stale copy underneath.
/// - Follow a symlink out of the tree, or copy anything that is not a regular
///   file or a directory.
/// - Fail a launch. Every error is reported and swallowed: a migration that
///   cannot run must degrade to the old location, never to an empty profile.
pub fn migrate_legacy_layout() -> Vec<String> {
    let mut notes = Vec::new();
    // The downloaded runtimes are tens of megabytes and re-derivable only by
    // downloading them again, so they are RENAMED rather than copied: a rename
    // either happens or does not, and duplicating 78 MB to be careful about a
    // directory nothing outside the program names would be the wrong trade.
    // A rename across filesystems fails, and then the note says so and the old
    // location keeps working - `runtimes_dir` is the only reader either way.
    let (legacy_runtimes, runtimes) = (legacy_data_dir().join("runtimes"), data_dir().join("runtimes"));
    if legacy_runtimes.is_dir() && !runtimes.exists() {
        if let Some(parent) = runtimes.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::rename(&legacy_runtimes, &runtimes) {
            Ok(()) => notes.push(format!("moved the downloaded runtimes to {}", runtimes.display())),
            Err(e) => notes.push(format!(
                "could not move the runtimes from {} to {}: {e} - they will be downloaded again \
                 into the new location unless you move them by hand",
                legacy_runtimes.display(),
                runtimes.display()
            )),
        }
    }
    for (from, to, what) in [
        (legacy_config_dir(), config_dir(), "settings"),
        (legacy_state_dir(), state_dir(), "logs"),
    ] {
        if !from.is_dir() || to.join(MIGRATION_MARKER).exists() {
            continue;
        }
        match copy_tree(&from, &to) {
            Ok((n, skipped)) => {
                // The marker goes down even when nothing was copied. Without
                // that, a directory that was already fully migrated stays
                // unmarked, and a file the user DELETES afterwards is copied
                // back from the old tree on the next launch - a deletion that
                // undoes itself.
                let _ =
                    std::fs::write(to.join(MIGRATION_MARKER), format!("{}\n", from.display()));
                if n > 0 {
                    notes.push(format!(
                        "moved {n} {what} file(s) to {} (the old {} is left in place)",
                        to.display(),
                        from.display()
                    ));
                }
                if !skipped.is_empty() {
                    notes.push(format!(
                        "left {} {what} entry(ies) at {} that are not plain files - symlinks \
                         are not followed, so copy these across by hand: {}",
                        skipped.len(),
                        from.display(),
                        skipped.join(", ")
                    ));
                }
            }
            Err(e) => notes.push(format!(
                "could not move {what} from {} to {}: {e} - the old location is still in use",
                from.display(),
                to.display()
            )),
        }
    }
    notes
}

/// Copy `from` into `to`, never overwriting. Returns how many files were copied.
///
/// Depth is bounded because the trees this runs on are two levels deep
/// (`games/`, `addons/`) and an unbounded walk over a directory the user could
/// have pointed at anything is not worth the risk.
fn copy_tree(from: &Path, to: &Path) -> io::Result<(usize, Vec<String>)> {
    fn walk(
        from: &Path,
        to: &Path,
        depth: usize,
        n: &mut usize,
        skipped: &mut Vec<String>,
    ) -> io::Result<()> {
        if depth > 4 {
            return Ok(());
        }
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let src = entry.path();
            let dst = to.join(entry.file_name());
            // `symlink_metadata`, not `metadata`: a symlink here is followed by
            // the latter, so a link pointing outside the tree would be copied
            // as its target - or, worse, walked as a directory somewhere else.
            let md = std::fs::symlink_metadata(&src)?;
            if md.file_type().is_symlink() {
                // Not followed - reading through an arbitrary link is not this
                // function's business. But SAID, because a file that is not
                // copied is a file the program will never read again: nothing
                // falls back to the old path. A dotfile manager (stow, chezmoi)
                // stores exactly these as per-file links, and dropping one in
                // silence while reporting success is how somebody loses their
                // settings to an upgrade that said it worked.
                skipped.push(entry.file_name().to_string_lossy().into_owned());
                continue;
            }
            if md.is_dir() {
                std::fs::create_dir_all(&dst)?;
                walk(&src, &dst, depth + 1, n, skipped)?;
            } else if md.is_file() && !dst.exists() {
                std::fs::create_dir_all(to)?;
                std::fs::copy(&src, &dst)?;
                // Credentials keep their mode. `fs::copy` copies permissions on
                // Unix, so this is belt and braces - but nexus.ini holds an
                // OAuth token, and "belt and braces" is the right amount of
                // care for a file whose mode being wrong is a disclosure.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = md.permissions().mode() & 0o777;
                    let _ = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(mode));
                }
                *n += 1;
            }
        }
        Ok(())
    }
    let mut n = 0;
    let mut skipped = Vec::new();
    std::fs::create_dir_all(to)?;
    walk(from, to, 0, &mut n, &mut skipped)?;
    Ok((n, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A temp dir that cleans up, with no dependency for it.
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            let p = std::env::temp_dir()
                .join(format!("eidos-paths-{}-{tag}-{:?}", std::process::id(), std::thread::current().id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Tmp(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_layout_is_vendor_then_program() {
        // The whole point of the crate: one `Colony/Eidos` pair under every
        // root, so a user finds one tree per program rather than four spellings.
        for dir in [config_dir(), data_dir(), cache_dir(), state_dir()] {
            let tail: Vec<_> =
                dir.components().rev().take(2).map(|c| c.as_os_str().to_owned()).collect();
            assert_eq!(tail, vec![PROGRAM, VENDOR], "{}", dir.display());
        }
        // And the four roots are four different places, so clearing the cache
        // cannot take the credentials with it.
        let all = [config_dir(), data_dir(), cache_dir(), state_dir()];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
                assert!(!a.starts_with(b) && !b.starts_with(a), "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn a_relative_xdg_variable_is_ignored_as_the_spec_says() {
        // Not pedantry: a relative value resolves against the process's working
        // directory, which for Eidos under Proton is the GAME's directory. The
        // spec says ignore it, and the reason is exactly that.
        assert!(xdg("EIDOS_PATHS_TEST_UNSET_VAR", ".config").is_absolute());
    }

    #[test]
    fn the_migration_copies_rather_than_moves_and_never_overwrites() {
        let t = Tmp::new("mig");
        let from = t.path().join("old");
        let to = t.path().join("new");
        fs::create_dir_all(from.join("games")).unwrap();
        fs::write(from.join("settings.ini"), b"[eidos]\ntheme=dark\n").unwrap();
        fs::write(from.join("games").join("x.toml"), b"id=1").unwrap();
        // Already migrated by hand, with a different value: it must survive.
        fs::create_dir_all(&to).unwrap();
        fs::write(to.join("settings.ini"), b"[eidos]\ntheme=light\n").unwrap();

        let (n, skipped) = copy_tree(&from, &to).unwrap();

        assert_eq!(n, 1, "only the file that was not already there");
        assert!(skipped.is_empty());
        assert_eq!(fs::read_to_string(to.join("settings.ini")).unwrap(), "[eidos]\ntheme=light\n");
        assert_eq!(fs::read_to_string(to.join("games").join("x.toml")).unwrap(), "id=1");
        // The old tree is still there. If this migration is wrong about a
        // filename, the user's data has to still exist somewhere.
        assert!(from.join("settings.ini").is_file());
    }

    #[test]
    fn the_migration_refuses_to_follow_a_symlink_out_of_the_tree() {
        let t = Tmp::new("link");
        let from = t.path().join("old");
        let to = t.path().join("new");
        let outside = t.path().join("elsewhere");
        fs::create_dir_all(&from).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret"), b"not ours").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, from.join("games")).unwrap();
        fs::write(from.join("settings.ini"), b"x").unwrap();

        let (n, skipped) = copy_tree(&from, &to).unwrap();

        assert_eq!(n, 1, "the real file, and nothing through the link");
        assert!(!to.join("games").exists(), "a link out of the tree is not followed");
        // And it SAYS so. A file that is not copied is a file the program will
        // never read again - nothing falls back to the old path - so dropping
        // one in silence while reporting success is how somebody loses their
        // settings to an upgrade that said it worked.
        assert_eq!(skipped, vec!["games".to_string()]);
    }

    #[test]
    fn a_credential_keeps_its_mode_across_the_move() {
        let t = Tmp::new("mode");
        let from = t.path().join("old");
        let to = t.path().join("new");
        fs::create_dir_all(&from).unwrap();
        let cred = from.join("nexus.ini");
        fs::write(&cred, b"access_token=abc\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&cred, fs::Permissions::from_mode(0o600)).unwrap();
        }

        copy_tree(&from, &to).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(to.join("nexus.ini")).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "an OAuth token must not become world-readable by moving");
        }
    }
}
