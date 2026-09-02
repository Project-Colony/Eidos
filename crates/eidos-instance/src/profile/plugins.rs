//! Per-profile plugin state: plugins.txt, the locked order, the snapshot,
//! and the loss guard that refuses to persist a gutted list.

//! A profile: one named set of enabled mods + their order (and, later, its own
//! `plugins.txt`, INIs and saves), all sharing the instance's single `mods/`
//! pool. This is what lets one mod collection serve several playthroughs.
//!
//! Mirrors Mod Organizer 2: a profile is just a directory under
//! `<instance>/profiles/<name>/`; its `modlist.txt` carries both the enabled set
//! and the priority order, while the mods themselves stay global.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::*;

/// A list smaller than this is never second-guessed - losing two of five entries
/// is an ordinary edit.
pub(crate) const MIN_ACTIVES: usize = 10;

/// Both thresholds must be crossed for a loss to look accidental rather than
/// deliberate. Shared by the plugin-order capture and the mod-list reconciliation
/// so "this looks like an accident" means one thing across the instance.
pub(crate) const MAX_ABSOLUTE_DROP: usize = 10;

pub(crate) const MAX_RELATIVE_DROP: f64 = 0.30;

/// Why a capture would lose too much of the active set to be trusted, or `None`
/// when it looks like a legitimate edit.
///
/// Fires on a total wipe at any size, on a large loss from a large list, and on
/// a MAJORITY loss from any list (dropping a couple of plugins is what a user
/// does on purpose; losing most of the actives is what crash artifacts look
/// like at every size). Since the check became a warn-with-one-click-dismiss
/// rather than a silent refusal, a rare false flag on a deliberate mass-disable
/// costs one click; the old silent acceptance cost the load order.
pub(crate) fn active_loss(
    profile: &Path,
    candidate: &Path,
    mechanism: eidos_plugins::LoadOrderMechanism,
) -> Option<String> {
    let before = count_actives(profile, mechanism)?;
    let after = count_actives(candidate, mechanism)?;
    if after >= before {
        return None;
    }
    // Losing EVERY active plugin is never an edit, at any size. The proportional
    // rule below has a floor so that dropping two of five is not second-guessed,
    // and a load order under that floor was therefore unprotected - which is not a
    // rare case but the normal one for someone adding mods a few at a time.
    // Observed: Skyrim rewrote plugins.txt with nothing but its own header, the
    // 7-plugin profile went to zero unchallenged, and the next launch silently
    // re-enabled everything discovery could find - including plugins the user had
    // deliberately turned off. Same rule as the mod list's `ListTrust::judge`.
    if after == 0 {
        return Some(format!(
            "it clears the active set entirely ({before} plugin(s) lost)"
        ));
    }
    let dropped = before - after;
    let relative = dropped as f64 / before as f64;
    // Two proportional rules. The big-list rule is the original; the majority
    // rule closes the hole underneath it: a partial crash artifact leaving 1 of
    // 7 actives slid under the >10-dropped floor and was accepted unchallenged.
    // Since the check became a warn-with-restore rather than a silent refusal,
    // a rare false flag on a deliberate mass-disable costs one click on
    // "Keep the current set" - the old silent acceptance cost the load order.
    if dropped > MAX_ABSOLUTE_DROP && relative > MAX_RELATIVE_DROP {
        return Some(format!(
            "it drops {dropped} of {before} active plugins ({:.0}%)",
            relative * 100.0
        ));
    }
    (dropped > 2 && relative > 0.50).then(|| {
        format!(
            "it drops {dropped} of {before} active plugins ({:.0}%)",
            relative * 100.0
        )
    })
}

/// Active (`*`-prefixed) entries in a plugins.txt, or `None` if unreadable.
///
/// MUST go through `read_decoded`: plugins.txt is CP1252 on disk (the encoding
/// Eidos itself writes), and reading it as strict UTF-8 made this return `None`
/// for any list containing one accented plugin name - which made `active_loss`
/// return `None` too, silently disarming the only guard between a crash artifact
/// and the profile. A French load order is one translated mod away from that.
pub(crate) fn count_actives(
    path: &Path,
    mechanism: eidos_plugins::LoadOrderMechanism,
) -> Option<usize> {
    let text = eidos_plugins::read_decoded(path)?;
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    Some(match mechanism {
        // Asterisk: `*` marks active. Counting `*` on a PlainList file - where
        // NO line ever has one - read every healthy Fallout list as 0 actives
        // and every wipe as no-change, leaving those games without a backstop.
        eidos_plugins::LoadOrderMechanism::Asterisk => lines.filter(|l| l.starts_with('*')).count(),
        // PlainList: every listed plugin IS active.
        eidos_plugins::LoadOrderMechanism::PlainList => lines.count(),
    })
}

impl Profile {
    /// This profile's plugin-state directory: `profiles/<name>/plugins/`, owning
    /// `plugins.txt` + `loadorder.txt` plus the sidecar files the game keeps next
    /// to them (`ContentCatalog.txt`, tool settings).
    ///
    /// A DIRECTORY, not two loose files, because at launch it is bind-mounted
    /// over the game's AppData plugin dir - MO2's usvfs virtualization, done the
    /// way the saves already are. One copy of the truth: the GUI, the CLI sort
    /// and the game itself all write the same files, so there is no post-run
    /// capture to revert anything and no deploy for a crash to skip. A directory
    /// bind is mandatory, not a preference: the game replaces `Plugins.txt` with
    /// a fresh inode (measured), and a FILE bind dies on that with EBUSY.
    ///
    /// Accessing it migrates the legacy layout (the two files at the profile
    /// top level) in, best-effort: every reader and writer goes through here, so
    /// this is the one place the move can be guaranteed to precede any use.
    /// This profile's `plugins.txt`, inside [`Self::plugins_state_dir`]. Note the
    /// game may write a case variant next to it - readers that must see the
    /// game's own writes go through `newest_variant`, not this exact path.
    pub fn plugins_txt_path(&self) -> PathBuf {
        self.plugins_state_dir().join("plugins.txt")
    }

    /// This profile's stored load order (`loadorder.txt`), the companion to
    /// [`Self::plugins_txt_path`] that records the FULL order - including the
    /// primaries and Creations that plugins.txt deliberately omits.
    pub fn loadorder_txt_path(&self) -> PathBuf {
        self.plugins_state_dir().join("loadorder.txt")
    }

    /// The load-order positions the user pinned (MO2's `lockedorder.txt`), one
    /// `name|index` per line.
    ///
    /// Deliberately at the profile top level and NOT in [`Self::plugins_state_dir`],
    /// which is bind-mounted over the game's own AppData at launch: this is
    /// Eidos's bookkeeping, and the game has no business being shown it.
    pub fn locked_order_path(&self) -> PathBuf {
        self.dir().join("lockedorder.txt")
    }

    /// Read the pinned positions. A malformed or unreadable file yields no pins
    /// rather than an error: a lost pin is a cosmetic regression, while refusing
    /// to open the profile over one would not be.
    pub fn read_locked_order(&self) -> BTreeMap<String, usize> {
        let Some(body) = fs::read_to_string(self.locked_order_path()).ok() else {
            return BTreeMap::new();
        };
        body.lines()
            .filter_map(|l| {
                let (name, idx) = l.trim().split_once('|')?;
                let name = name.trim();
                if name.is_empty() {
                    return None;
                }
                Some((name.to_ascii_lowercase(), idx.trim().parse().ok()?))
            })
            .collect()
    }

    /// Persist the pinned positions, removing the file when nothing is pinned so
    /// an empty profile does not carry a stray artifact.
    pub fn write_locked_order(&self, locked: &BTreeMap<String, usize>) -> io::Result<()> {
        let path = self.locked_order_path();
        if locked.is_empty() {
            match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => return Ok(()),
            }
        }
        let body: String = locked.iter().map(|(n, i)| format!("{n}|{i}\n")).collect();
        copy_atomic_bytes(&path, body.as_bytes())
    }

    pub fn plugins_state_dir(&self) -> PathBuf {
        let dir = self.dir().join("plugins");
        let _ = fs::create_dir_all(&dir);
        for name in ["plugins.txt", "loadorder.txt"] {
            let legacy = self.dir().join(name);
            let new = dir.join(name);
            if legacy.is_file() && !new.exists() {
                let _ = fs::rename(&legacy, &new);
            }
        }
        dir
    }

    /// Whether this profile already owns a plugin state (so it should drive the
    /// load order rather than the prefix's copy). Reads through the case-variant
    /// resolver: the game may have written `Plugins.txt` into the bound dir.
    pub fn has_plugin_state(&self) -> bool {
        eidos_plugins::newest_variant(&self.plugins_state_dir(), "plugins.txt").is_some()
    }

    /// One-time migration, mirroring [`Self::seed_inis`]: adopt the prefix's
    /// existing plugin dir (`src_dir` = the game's AppData dir) into this
    /// profile, without overwriting anything the profile already owns.
    ///
    /// `plugins.txt`/`loadorder.txt` are adopted via the case-variant resolver.
    /// Every OTHER regular file is adopted too - `ContentCatalog.txt`,
    /// `Plugins.sseviewsettings` and whatever else the game or a tool keeps
    /// there - because the whole directory is bind-mounted at launch, and a
    /// profile dir missing those would present the game an emptier directory
    /// than the one it wrote them into. Returns how many files were seeded.
    pub fn seed_plugin_state(
        &self,
        src_dir: &Path,
        spec: &eidos_plugins::GameSpec,
    ) -> io::Result<u32> {
        let dst_dir = self.plugins_state_dir();
        let mut n = 0;
        for name in ["plugins.txt", "loadorder.txt"] {
            let Some(src) = eidos_plugins::newest_variant(src_dir, name) else {
                continue;
            };
            let dst = dst_dir.join(name);
            if eidos_plugins::newest_variant(&dst_dir, name).is_none() {
                // Adopt VERBATIM, always. An earlier version refused a file that
                // looked like a crash artifact - but "refusing" did not leave the
                // state alone: the same run then derived everything-ENABLED from
                // discovery and shadow-wrote that over the prefix, destroying a
                // deliberate all-off state. And the refused signature (names
                // listed, no `*`) is exactly what every HEALTHY PlainList
                // plugins.txt looks like, so Fallout and Skyrim LE setups were
                // being refused wholesale. Adoption is non-destructive in every
                // case: profile == prefix, prefix bytes untouched, and a wrong
                // founding state is one GUI re-enable away. So: adopt, and for
                // Asterisk games - where the signature MEANS something - warn.
                if name == "plugins.txt"
                    && spec.mechanism == eidos_plugins::LoadOrderMechanism::Asterisk
                    && looks_like_crash_artifact(&src)
                {
                    eprintln!(
                        "eidos: the adopted plugins.txt lists plugins but activates none - if \
                         that is not how you left it, a crash wrote it; re-enable your \
                         plugins in the Plugins tab of profile '{}'",
                        self.name
                    );
                }
                copy_atomic(&src, &dst)?;
                // Preserve the source mtimes: which of the two files is newer is
                // a real signal (the freshness tiebreak in apply_prefix_state),
                // and a copy that stamps both to "now" erases it - adopting a
                // divergent pair with the stale half promoted to authority.
                if let Ok(mtime) = fs::metadata(&src).and_then(|m| m.modified()) {
                    if let Ok(f) = fs::File::options().write(true).open(&dst) {
                        let _ = f.set_modified(mtime);
                    }
                }
                n += 1;
            }
        }
        let Ok(rd) = fs::read_dir(src_dir) else {
            return Ok(n);
        };
        for e in rd.flatten() {
            let name = e.file_name();
            let lower = name.to_string_lossy().to_ascii_lowercase();
            // The two state files are handled above (case-variant aware); a
            // leftover `.tmp` is a crashed write, not content.
            if lower.eq_ignore_ascii_case("plugins.txt")
                || lower.eq_ignore_ascii_case("loadorder.txt")
                || lower.ends_with(".tmp")
                || !e.path().is_file()
            {
                continue;
            }
            let dst = dst_dir.join(&name);
            if !dst.exists() {
                copy_atomic(&e.path(), &dst)?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Where the pre-session copy of `plugins.txt` lives: NEXT TO the plugins
    /// dir, never inside it - the game must not see it through the bind.
    pub fn plugins_snapshot_path(&self) -> PathBuf {
        self.dir().join("plugins.txt.pre-session")
    }

    /// Record the pre-session state of `plugins.txt`, so
    /// [`Self::plugin_loss_since_snapshot`] can tell a session that legitimately
    /// edited the active set from one that wrecked it.
    ///
    /// This replaces the old capture-time guard: with the plugins dir
    /// bind-mounted, the game writes the profile file DIRECTLY and there is no
    /// copy moment left to refuse. The snapshot restores the choke point as a
    /// warn-and-offer-restore, without resurrecting the two-copy design.
    pub fn snapshot_plugin_state(&self) -> io::Result<()> {
        let snap = self.plugins_snapshot_path();
        match eidos_plugins::newest_variant(&self.plugins_state_dir(), "plugins.txt") {
            Some(src) => copy_atomic(&src, &snap),
            None => {
                // No state yet: a stale snapshot would compare a future session
                // against some other session's list.
                let _ = fs::remove_file(&snap);
                Ok(())
            }
        }
    }

    /// Whether the current `plugins.txt` lost too much of the pre-session active
    /// set to look like an edit (see [`active_loss`]) - the post-session half of
    /// the backstop. `None` means healthy, no snapshot, or no state.
    pub fn plugin_loss_since_snapshot(&self, spec: &eidos_plugins::GameSpec) -> Option<String> {
        let snap = self.plugins_snapshot_path();
        if !snap.is_file() {
            return None;
        }
        let current = eidos_plugins::newest_variant(&self.plugins_state_dir(), "plugins.txt")?;
        active_loss(&snap, &current, spec.mechanism)
    }

    /// Put the pre-session `plugins.txt` back - the restore half of the backstop,
    /// wired to a GUI button so recovering from a wrecked session is one click
    /// and not a file-manager expedition.
    pub fn restore_plugin_snapshot(&self) -> io::Result<()> {
        let snap = self.plugins_snapshot_path();
        if !snap.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no pre-session copy exists",
            ));
        }
        let dir = self.plugins_state_dir();
        copy_atomic(&snap, &eidos_plugins::canonical_path(&dir, "plugins.txt"))
    }

    /// Enabled mods, highest priority first: the layers to mount at launch.
    /// Separators are group dividers, not content, so they are never mounted.
    ///
    /// `modlist()` returns MO2 display order (lowest priority first), so reverse to
    /// the highest-priority-first order the union mount expects (layer 0 wins).
    pub fn load_order(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self
            .modlist()
            .into_iter()
            // Unmanaged content is already IN the directory we mount over, so
            // mounting it again as a layer would stack the game's own Data on
            // top of itself. `modlist()` never yields one, but the filter states
            // the invariant where it matters rather than relying on that.
            .filter(|m| m.is_active() && !m.unmanaged)
            .map(|m| m.path)
            .collect();
        v.reverse();
        v
    }
}
