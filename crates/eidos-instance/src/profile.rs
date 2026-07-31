//! A profile: one named set of enabled mods + their order (and, later, its own
//! `plugins.txt`, INIs and saves), all sharing the instance's single `mods/`
//! pool. This is what lets one mod collection serve several playthroughs.
//!
//! Mirrors Mod Organizer 2: a profile is just a directory under
//! `<instance>/profiles/<name>/`; its `modlist.txt` carries both the enabled set
//! and the priority order, while the mods themselves stay global.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ModEntry;

/// One profile of an instance.
#[derive(Debug, Clone)]
pub struct Profile {
    pub instance_root: PathBuf,
    pub name: String,
}

/// One save file in a profile's `saves/` directory (MO2's savegame list).
#[derive(Debug, Clone)]
pub struct SaveEntry {
    /// The file's name (e.g. `Save1_quicksave.ess`).
    pub filename: String,
    /// The absolute path on disk.
    pub path: PathBuf,
    /// Size in bytes.
    pub size: u64,
    /// Last-modified time (used as the in-game date proxy).
    pub mtime: std::time::SystemTime,
}

impl Profile {
    /// `<instance>/profiles/<name>/`.
    pub fn dir(&self) -> PathBuf {
        self.instance_root.join("profiles").join(&self.name)
    }

    /// The shared mod pool (instance-wide, not per-profile).
    fn mods_dir(&self) -> PathBuf {
        self.instance_root.join("mods")
    }

    fn modlist_path(&self) -> PathBuf {
        self.dir().join("modlist.txt")
    }

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
            let Some(src) = eidos_plugins::newest_variant(src_dir, name) else { continue };
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
        let Ok(rd) = fs::read_dir(src_dir) else { return Ok(n) };
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
    pub fn plugin_loss_since_snapshot(
        &self,
        spec: &eidos_plugins::GameSpec,
    ) -> Option<String> {
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
            return Err(io::Error::new(io::ErrorKind::NotFound, "no pre-session copy exists"));
        }
        let dir = self.plugins_state_dir();
        copy_atomic(&snap, &eidos_plugins::canonical_path(&dir, "plugins.txt"))
    }

    /// This profile's stored copy of a game INI (e.g. `Skyrim.ini`). The profile
    /// owns its INIs; they are deployed into the Proton prefix at launch.
    pub fn ini_path(&self, ini_file: &str) -> PathBuf {
        self.dir().join(ini_file)
    }

    /// One-time migration: copy the user's existing prefix INIs (`src_dir` = the
    /// prefix `Documents/My Games/<game>`) into this profile, but only those the
    /// profile doesn't already own - so an existing setup is adopted, not lost.
    /// Returns how many were seeded.
    ///
    /// Divergence from MO2: MO2 seeds a new profile's INIs from the *vanilla game
    /// folder* (a clean baseline - it owns the INIs from the start). Eidos adopts a
    /// pre-existing Proton setup, so we seed from the user's *current* prefix INIs
    /// to keep their working settings (resolution, language, tweaks). Like MO2 we
    /// never overwrite an INI the profile already has.
    pub fn seed_inis(&self, src_dir: &Path, ini_files: &[&str]) -> io::Result<u32> {
        fs::create_dir_all(self.dir())?;
        let mut n = 0;
        for f in ini_files {
            let dst = self.ini_path(f);
            let src = src_dir.join(f);
            if !dst.exists() && src.is_file() {
                copy_atomic(&src, &dst)?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Deploy this profile's INIs into `dst_dir` (the prefix Documents) before
    /// launch, so the game reads this profile's settings. Only INIs the profile
    /// actually has are written. Returns how many were deployed.
    pub fn deploy_inis(&self, dst_dir: &Path, ini_files: &[&str]) -> io::Result<u32> {
        fs::create_dir_all(dst_dir)?;
        let mut n = 0;
        for f in ini_files {
            let src = self.ini_path(f);
            if src.is_file() {
                copy_atomic(&src, &dst_dir.join(f))?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Capture the (game-modified) INIs from `src_dir` back into the profile after
    /// the game exits, so in-game settings changes persist to the profile. Returns
    /// how many were captured.
    pub fn capture_inis(&self, src_dir: &Path, ini_files: &[&str]) -> io::Result<u32> {
        fs::create_dir_all(self.dir())?;
        let mut n = 0;
        for f in ini_files {
            let src = src_dir.join(f);
            if !src.is_file() {
                continue;
            }
            // The same skepticism the plugin capture has had all along, at last
            // applied to the INIs: a game killed mid-write leaves an empty or
            // truncated file, and committing it silently destroys the profile's
            // only copy of the user's settings. Empty is never a real INI, and a
            // capture under half the profile copy's size is a wreck, not an edit
            // (in-game settings changes move an INI by bytes, not halves).
            let src_len = fs::metadata(&src).map(|m| m.len()).unwrap_or(0);
            let dst = self.ini_path(f);
            let dst_len = fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
            // A crash truncation is random; the engine's own compact rewrite of
            // a fat INI is STABLE. If the refused size repeats within 10% on the
            // next run, it is the engine's real format and refusing forever
            // would mean in-game settings never persist again.
            let refused_marker = self.dir().join(format!("{f}.refused-len"));
            let last_refused: Option<u64> =
                fs::read_to_string(&refused_marker).ok().and_then(|t| t.trim().parse().ok());
            let stable_repeat = src_len > 0
                && last_refused.is_some_and(|prev| {
                    // Relative tolerance only: a floor let "empty then anything
                    // small" count as a repeat. Empty never records a marker
                    // (an empty INI is never a real format), so prev > 0 here.
                    prev > 0 && src_len.abs_diff(prev) <= (prev / 10).max(16)
                });
            if src_len == 0 && dst_len == 0 {
                // Both empty: BethINI-style placeholder Custom INIs. Nothing to
                // capture, nothing to warn about - the old message called a
                // 0-vs-0 no-op a crash artifact on every single run, and
                // promised a repeat-acceptance that (rightly) can never fire
                // for empty files.
                continue;
            }
            if (src_len == 0 && dst_len > 0)
                || (dst_len > 0 && src_len < dst_len / 2 && !stable_repeat)
            {
                eprintln!(
                    "eidos: NOT capturing {f} back into profile '{}': the prefix copy is {src_len} \
                     bytes against the profile's {dst_len} - a crash artifact, not an edit. \
                     The profile keeps its own copy (a repeat at this size will be accepted).",
                    self.name
                );
                if src_len > 0 {
                    let _ = fs::write(&refused_marker, src_len.to_string());
                }
                continue;
            }
            if stable_repeat {
                // The repeat is being accepted on a heuristic, and the sources
                // that DEFEAT the heuristic are deterministic wrecks (same-point
                // exit crashes). Stash what the acceptance displaces, so being
                // wrong costs a restore instead of the only intact copy.
                let _ = copy_atomic(&dst, &self.dir().join(format!("{f}.pre-accept")));
                eprintln!(
                    "eidos: accepting {f}'s stable compact rewrite into profile '{}'; the \
                     displaced copy is kept as {f}.pre-accept in the profile folder",
                    self.name
                );
            }
            let _ = fs::remove_file(&refused_marker);
            // Atomic, because this runs right after the game exits and the
            // profile copy is the only durable one: a torn capture is a lost
            // config, not a transient.
            copy_atomic(&src, &dst)?;
            n += 1;
        }
        Ok(n)
    }

    /// The profile's own INI tweak file, applied after every mod's fragments so
    /// the user always has the last word (MO2's `getProfileTweaks`). Optional: a
    /// profile without one simply contributes nothing.
    pub fn tweaks_path(&self) -> PathBuf {
        self.dir().join("initweaks.ini")
    }

    /// Merge INI-tweak fragments into a deployed game INI, in the order given
    /// (later wins), and return what each write displaced so [`untweak_ini`] can
    /// put it back after the run.
    ///
    /// `fragments` are the enabled `INI Tweaks/*.ini` files in mod priority order,
    /// lowest first; the profile's own tweak file, if any, is applied last.
    /// A fragment that cannot be read is skipped rather than failing the launch -
    /// a missing tweak must not stop the game from starting.
    pub fn apply_ini_tweaks(
        &self,
        deployed_ini: &Path,
        fragments: &[PathBuf],
    ) -> io::Result<Vec<TweakedKey>> {
        let mut record: Vec<TweakedKey> = Vec::new();
        // NEVER "unreadable = empty". That equation deployed the tweak fragment
        // ALONE as the whole INI whenever the real file held one CP1252 byte -
        // deterministically, every run, which even defeated the size-based
        // capture guard downstream (a stable artifact looks like a format).
        let Some((mut text, cp1252)) = read_text_lossy(deployed_ini) else {
            eprintln!(
                "eidos: WARNING - {} is unreadable; INI tweaks skipped for it this run",
                deployed_ini.display()
            );
            return Ok(record);
        };
        let mut any = false;
        for frag in fragments.iter().chain(std::iter::once(&self.tweaks_path())) {
            let Some((body, _)) = read_text_lossy(frag) else { continue };
            any |= merge_tweak(&mut text, &body, &mut record);
        }
        if any {
            if let Some(parent) = deployed_ini.parent() {
                fs::create_dir_all(parent)?;
            }
            // Same encoding as found: the game reads ANSI, and a silent UTF-8
            // conversion would mojibake every accented value in-game.
            write_text(deployed_ini, &text, cp1252)?;
        }
        Ok(record)
    }

    /// This profile's own save-games directory (`profil/saves`), MO2's
    /// `savePath()`. At launch it is bind-mounted over the prefix's save dir so
    /// the game reads and writes this profile's saves.
    pub fn saves_dir(&self) -> PathBuf {
        self.dir().join("saves")
    }

    /// One-time migration: copy the user's existing saves from `src_saves` (the
    /// prefix `Documents/My Games/<game>/Saves`) into this profile, but only if
    /// the profile has no saves yet - so an existing playthrough is adopted, not
    /// hidden when this profile's saves get bound over the prefix at launch.
    /// Returns how many save files were copied (0 if the profile already has any).
    pub fn seed_saves(&self, src_saves: &Path) -> io::Result<u32> {
        let dst = self.saves_dir();
        // "Already seeded" is a persistent MARKER, not the directory being
        // non-empty. The emptiness probe re-armed the seeding whenever the user
        // emptied the dir from the GUI - resurrecting years-old prefix saves
        // with fresh mtimes that then sorted above the real playthrough - and a
        // dir holding only a stray subdirectory blocked adoption forever.
        let marker = dst.join(".seeded");
        // Two marker states: "partial" means a previous adoption hit unreadable
        // files and the missing ones should be retried; anything else (including
        // the empty marker older binaries wrote) means done. Without the state,
        // the non-empty-dir shortcut below re-stamped a partial adoption as
        // complete on the very next run.
        let mut pending: std::collections::HashSet<String> = Default::default();
        let resuming = match fs::read_to_string(&marker) {
            Ok(state) => {
                let mut lines = state.lines();
                if lines.next().map(str::trim) != Some("partial") {
                    return Ok(0);
                }
                pending = lines.map(|l| l.trim().to_ascii_lowercase()).collect();
                true
            }
            Err(_) => false,
        };
        fs::create_dir_all(&dst)?;
        if !resuming {
            let already =
                fs::read_dir(&dst).map(|it| it.flatten().next().is_some()).unwrap_or(false);
            if already {
                // Pre-marker profiles: their saves are the adoption. Record and stop.
                let _ = fs::write(&marker, b"done");
                return Ok(0);
            }
        }
        let Ok(rd) = fs::read_dir(src_saves) else {
            // NO marker: an absent or unreadable source is a transient (wrong
            // drive not mounted, prefix not created yet), and stamping "done"
            // here disarmed adoption forever before anything existed to adopt.
            return Ok(0);
        };
        let mut n = 0;
        let mut failed: Vec<String> = Vec::new();
        let mut saw_any = false;
        for e in rd.flatten() {
            // Save data only, across every supported family (.ess/.fos/.sfs +
            // co-saves): steam_autocloud.vdf and .bak files are not saves and
            // used to show up in the Saves tab as one.
            if !is_save_data(&e.file_name().to_string_lossy()) || !e.path().is_file() {
                continue;
            }
            saw_any = true;
            let to = dst.join(e.file_name());
            if to.exists() {
                continue; // adopted on a previous (partial) pass
            }
            if resuming && !pending.contains(&e.file_name().to_string_lossy().to_ascii_lowercase())
            {
                // A resume only fetches what FAILED last time. Re-copying every
                // file missing from the profile resurrected saves the user had
                // deliberately deleted since the first pass.
                continue;
            }
            // One unreadable file must not abort the adoption of a whole
            // playthrough - it used to, silently, mid-loop, and the partial
            // profile then blocked completion forever via the emptiness probe.
            if copy_atomic(&e.path(), &to).is_err() {
                failed.push(e.file_name().to_string_lossy().into_owned());
                continue;
            }
            // Keep the save's real date: the Saves tab sorts by it, and a copy
            // stamped "now" made ancient saves sort above the live playthrough.
            if let Ok(mtime) = e.metadata().and_then(|m| m.modified()) {
                if let Ok(f) = fs::File::options().write(true).open(&to) {
                    let _ = f.set_modified(mtime);
                }
            }
            n += 1;
        }
        if !failed.is_empty() {
            // The marker records WHICH files failed: the resume fetches exactly
            // those, and stamping "done" over a partial adoption made the holes
            // permanent - unfixable even by deleting the marker, since a
            // non-empty dir also counted as adopted.
            eprintln!(
                "eidos: WARNING - {} save file(s) could not be adopted into profile '{}' \
                 (unreadable); adopted {n}, will retry the rest next launch",
                failed.len(),
                self.name
            );
            let body = format!("partial\n{}", failed.join("\n"));
            let _ = fs::write(&marker, body);
        } else if saw_any {
            let _ = fs::write(&marker, b"done");
        }
        // A readable dir holding NO save data writes no marker at all: Steam
        // creates the Saves dir (with its autocloud sidecar) before the user
        // ever saves, and stamping "done" then disarmed adoption forever for
        // saves that appeared five minutes later.
        Ok(n)
    }

    /// List this profile's save files (`profil/saves/<file>`), newest first and
    /// capped to a sane number. Directories and dotfiles are skipped; this never
    /// errors - a missing or unreadable saves dir yields an empty list.
    pub fn savegames(&self) -> Vec<SaveEntry> {
        let mut out: Vec<SaveEntry> = fs::read_dir(self.saves_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let md = e.metadata().ok()?;
                if !md.is_file() {
                    return None;
                }
                let filename = e.file_name().into_string().ok()?;
                // Real saves only, for every supported family: the Saves tab
                // listed steam_autocloud.vdf as a playthrough entry, and an
                // .ess-only filter would blank the tab for Fallout/Starfield.
                if filename.starts_with('.') || !is_save_listing(&filename) {
                    return None;
                }
                Some(SaveEntry {
                    filename,
                    path: e.path(),
                    size: md.len(),
                    mtime: md.modified().ok()?,
                })
            })
            .collect();
        out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        out
    }

    /// Where to read the mod list from: the profile's own file, or - for the
    /// `Default` profile of a not-yet-migrated instance - the legacy flat
    /// `<instance>/modlist.txt`.
    fn modlist_source(&self) -> PathBuf {
        let own = self.modlist_path();
        if own.exists() {
            return own;
        }
        let legacy = self.instance_root.join("modlist.txt");
        if self.name == "Default" && legacy.exists() {
            return legacy;
        }
        own
    }

    pub fn create(&self) -> io::Result<()> {
        fs::create_dir_all(self.dir())
    }

    /// Create this profile by copying another profile's files (its modlist, plugin
    /// and INI files) and its subdirectories (`saves/`). MO2's profile copy is
    /// fully recursive; copying `saves/` means the new profile starts from this
    /// profile's saves rather than later adopting the stale prefix saves.
    pub fn create_from(&self, other: &Profile) -> io::Result<()> {
        fs::create_dir_all(self.dir())?;
        // Propagate per-file failures: silently skipping files would report a
        // successful copy while producing a profile missing its modlist or INIs.
        if let Ok(rd) = fs::read_dir(other.dir()) {
            for e in rd.flatten() {
                let from = e.path();
                let to = self.dir().join(e.file_name());
                if from.is_file() {
                    fs::copy(&from, &to)?;
                } else if from.is_dir() {
                    copy_dir_recursive(&from, &to)?;
                }
            }
        }
        Ok(())
    }

    pub fn rename(&self, new_name: &str) -> io::Result<Profile> {
        let dest = self.instance_root.join("profiles").join(new_name);
        fs::rename(self.dir(), &dest)?;
        Ok(Profile { instance_root: self.instance_root.clone(), name: new_name.to_string() })
    }

    pub fn delete(&self) -> io::Result<()> {
        fs::remove_dir_all(self.dir())
    }

    /// The mod list for this profile: every folder in the shared `mods/`, with
    /// enabled state, reconciled with this profile's `modlist.txt`.
    ///
    /// Returned in MO2's DISPLAY order: index 0 = TOP of the list = LOWEST priority
    /// (loaded first, loses file conflicts); the last entry = highest priority (wins,
    /// just above the always-on Overwrite). The on-disk `modlist.txt` keeps MO2's
    /// storage convention (highest priority FIRST), so it round-trips with MO2; this
    /// reverses it for display. Consumers that need launch/priority order (the FUSE
    /// `load_order`, plugin discovery, conflict origins) re-reverse to highest-first.
    pub fn modlist(&self) -> Vec<ModEntry> {
        self.modlist_checked().0
    }

    /// [`Profile::modlist`] plus whether the result is SAFE TO PERSIST.
    ///
    /// Like MO2, the disk decides which mods exist and `modlist.txt` decides only
    /// their order and enabled state, so an entry whose folder is gone drops out
    /// of the list. Unlike MO2, the drop is not automatically written back:
    /// `MO2::refreshModStatus` rewrites `modlist.txt` inside the same refresh, and
    /// its one apparent guard (`if (m_ModStatus.empty()) return;`, profile.cpp:254)
    /// is unreachable because the synthetic overwrite entry always makes the count
    /// non-zero. Point MO2 at an empty mods folder and the curated order is gone.
    ///
    /// That case is not exotic here. A mod pool is hundreds of gigabytes, so it
    /// routinely lives on another drive reached by a bind mount or symlink - and an
    /// unmounted bind-mount target is an existing, readable, EMPTY directory. The
    /// scan would report "you have no mods" for a fully intact pool, and the next
    /// click would make that permanent. So the caller is told when a result is only
    /// fit to display, and [`Profile::save_modlist`] refuses to write it.
    pub fn modlist_checked(&self) -> (Vec<ModEntry>, ListTrust) {
        let mods_dir = self.mods_dir();
        // `file_type()` reads the dirent's d_type instead of stat()ing the target.
        // Two things follow, both load-bearing: a directory that is readable but
        // not SEARCHABLE (mode 0600, an exFAT mount whose dmask drops +x) still
        // reports its children's kinds, where `path().is_dir()` returns false for
        // every one of them and empties the whole list; and a DANGLING symlink is
        // distinguishable from an absent folder.
        let scan = fs::read_dir(&mods_dir).map(|rd| {
            let mut names: Vec<String> = rd
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() || t.is_symlink()))
                .filter_map(|e| e.file_name().into_string().ok())
                // Eidos's own in-flight/crashed extraction temps are not mods. Only
                // ours: a leading dot is legal in a mod name and real mods use it
                // (".NET Script Framework" is a near-universal Skyrim SE
                // dependency), so filtering every dot-dir would silently erase them.
                .filter(|n| !n.starts_with(INSTALL_TEMP_PREFIX))
                .collect();
            names.sort();
            names
        });
        // An unreadable mods/ is NOT an empty one. Saying so is the difference
        // between "the user deleted their mods" and "the drive is not mounted".
        let (present, readable) = match scan {
            Ok(names) => (names, true),
            Err(_) => (Vec::new(), false),
        };

        let mut out: Vec<ModEntry> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut listed = 0usize;

        if let Ok(content) = fs::read_to_string(self.modlist_source()) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let (enabled, unmanaged, name) = if let Some(n) = line.strip_prefix('+') {
                    (true, false, n.trim())
                } else if let Some(n) = line.strip_prefix('-') {
                    (false, false, n.trim())
                } else if let Some(n) = line.strip_prefix('*') {
                    // MO2's mark for the game's own content - DLCs, Creation Club.
                    // Kept as a POSITION, which is the whole point: without a line
                    // of its own the row can only be pinned to the top of the list,
                    // and nothing the user owns can ever sit above it.
                    (true, true, n.trim())
                } else {
                    (true, false, line)
                };
                listed += 1;
                if unmanaged {
                    // The path is not knowable here: it lives in the GAME's data
                    // directory, which this layer has never been told about. The
                    // caller that does know reconciles these against what the game
                    // actually ships - see `Instance::unmanaged_mods`.
                    if seen.insert(name.to_string()) {
                        out.push(ModEntry {
                            name: name.to_string(),
                            enabled,
                            path: PathBuf::new(),
                            unmanaged: true,
                        });
                    }
                } else if present.iter().any(|p| p == name) && seen.insert(name.to_string()) {
                    out.push(ModEntry { name: name.to_string(), enabled, path: mods_dir.join(name), unmanaged: false });
                }
            }
        }
        // A folder nobody listed: a mod dropped in by hand. MO2 appends it at the
        // highest priority and leaves it DISABLED - it has no idea where in the
        // conflict order it belongs, and enabling it silently could overwrite half
        // the load order's files on the next launch.
        for name in present {
            if seen.insert(name.clone()) {
                out.push(ModEntry { path: mods_dir.join(&name), name, enabled: false, unmanaged: false });
            }
        }
        let trust = ListTrust::judge(readable, listed, out.len());
        // `out` is built highest-priority-first (file order). MO2 DISPLAYS the list
        // the other way up - lowest priority at the top - so reverse for the return.
        // (This preserves every entry's priority; only the vec orientation flips.)
        out.reverse();
        (out, trust)
    }

    /// Persist this profile's mod list (`+Name` enabled, `-Name` disabled).
    ///
    /// Written atomically (MO2's `SafeWriteFile`/`QSaveFile`): the content goes to
    /// a `.tmp` sibling in the *same* profile directory and is then `rename()`d over
    /// `modlist.txt`, an atomic swap within one filesystem. A crash or ENOSPC
    /// mid-write thus leaves the previous `modlist.txt` intact instead of an
    /// empty/partial file - which [`Profile::modlist`] would otherwise rebuild as
    /// "everything enabled, alphabetical", destroying the curated order. The
    /// previous list is also copied one-deep to `modlist.txt.bak` first.
    pub fn save_modlist(&self, mods: &[ModEntry]) -> io::Result<()> {
        if let Some(why) = self.unsafe_to_persist() {
            // Loud and non-destructive: the caller surfaces this and the curated
            // order stays on disk untouched.
            return Err(io::Error::new(io::ErrorKind::InvalidData, why));
        }
        fs::create_dir_all(self.dir())?;
        let mut s = String::new();
        // `mods` is in MO2 DISPLAY order (lowest priority first); the file stores
        // highest priority first (MO2's on-disk convention), so write it reversed.
        //
        // Unmanaged entries - the game's own DLCs and Creation Club content - are
        // written with MO2's `*`, which says "this row has a position, but Eidos
        // does not own the files". Dropping them, as this did, cost the user the
        // one thing a row needs: somewhere to be. They were re-discovered from the
        // game's data directory on every refresh and pinned to the top, so no
        // separator could ever sit above them and the block could not be collapsed.
        //
        // Nothing else changes: `load_order` filters `!unmanaged`, so a `*` row is
        // never mounted, and reconciliation against the game directory drops a line
        // whose content is gone.
        for m in mods.iter().rev() {
            s.push(if m.unmanaged {
                '*'
            } else if m.enabled {
                '+'
            } else {
                '-'
            });
            s.push_str(&m.name);
            s.push('\n');
        }

        let target = self.modlist_path();
        // Keep a one-deep backup of the previous list before swapping it out.
        if target.exists() {
            let _ = fs::copy(&target, target.with_extension("txt.bak"));
        }
        // Write to a temp file in the same directory, then atomically rename it over
        // the target so a partial write can never clobber the curated order.
        let tmp = target.with_extension("txt.tmp");
        fs::write(&tmp, s)?;
        match fs::rename(&tmp, &target) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
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
            .filter(|m| m.enabled && !m.is_separator() && !m.unmanaged)
            .map(|m| m.path)
            .collect();
        v.reverse();
        v
    }
}

/// A list smaller than this is never second-guessed - losing two of five entries
/// is an ordinary edit.
const MIN_ACTIVES: usize = 10;
/// Both thresholds must be crossed for a loss to look accidental rather than
/// deliberate. Shared by the plugin-order capture and the mod-list reconciliation
/// so "this looks like an accident" means one thing across the instance.
const MAX_ABSOLUTE_DROP: usize = 10;
const MAX_RELATIVE_DROP: f64 = 0.30;

/// Why a capture would lose too much of the active set to be trusted, or `None`
/// when it looks like a legitimate edit.
///
/// Fires on a total wipe at any size, on a large loss from a large list, and on
/// a MAJORITY loss from any list (dropping a couple of plugins is what a user
/// does on purpose; losing most of the actives is what crash artifacts look
/// like at every size). Since the check became a warn-with-one-click-dismiss
/// rather than a silent refusal, a rare false flag on a deliberate mass-disable
/// costs one click; the old silent acceptance cost the load order.
fn active_loss(
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
        return Some(format!("it clears the active set entirely ({before} plugin(s) lost)"));
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
        format!("it drops {dropped} of {before} active plugins ({:.0}%)", relative * 100.0)
    })
}

/// The save-file extensions across the supported game families: Bethesda's
/// `.ess` (Skyrim LE/SE), `.fos` (Fallout 3/NV/4) and `.sfs` (Starfield).
const SAVE_EXTS: &[&str] = &["ess", "fos", "sfs"];
/// Script-extender co-saves that travel WITH a save: same stem, own extension.
const COSAVE_EXTS: &[&str] = &["skse", "f4se", "nvse", "fose", "sfse", "obse"];

fn ext_of(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// Save DATA: a save or its co-save - what the seed and the cloud sync move.
/// An earlier filter hard-coded `.ess`/`.skse` and killed the entire save
/// pipeline for the Fallout and Starfield families in one line.
pub fn is_save_data(name: &str) -> bool {
    let e = ext_of(name);
    SAVE_EXTS.contains(&e.as_str()) || COSAVE_EXTS.contains(&e.as_str())
}

/// A save as the user thinks of one - what the Saves tab lists. Co-saves travel
/// with their `.ess`/`.fos`/`.sfs` and are not shown separately.
pub fn is_save_listing(name: &str) -> bool {
    SAVE_EXTS.contains(&ext_of(name).as_str())
}

/// The co-save paths that belong to `save` (same stem, co-save extensions), for
/// operations that must treat the pair as one unit - deleting a save while
/// leaving its co-save made an invisible orphan the cloud sync pushed forever.
pub fn cosave_siblings(save: &Path) -> Vec<PathBuf> {
    let Some(stem) = save.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()) else {
        return Vec::new();
    };
    let Some(dir) = save.parent() else { return Vec::new() };
    // Case-insensitive on BOTH halves, like every other save predicate: the game
    // wrote these on a filesystem it thought folded case, so `Quicksave.SKSE`
    // next to `quicksave.ess` is normal - and an exact-case join recreated the
    // invisible-orphan class this helper exists to close.
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            let Some((s, ext)) = name.rsplit_once('.') else { return false };
            s == stem && COSAVE_EXTS.contains(&ext) && e.path().is_file()
        })
        .map(|e| e.path())
        .collect()
}

/// Whether a plugins.txt has the signature of a crash artifact: several plugins
/// LISTED, none active. A deliberate everything-off edit also matches - but at
/// seed time there is no user history to defer to, and deriving from discovery
/// beats founding a profile on a wreck.
fn looks_like_crash_artifact(path: &Path) -> bool {
    let Some(text) = eidos_plugins::read_decoded(path) else { return false };
    let mut listed = 0usize;
    let mut active = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        listed += 1;
        if line.starts_with('*') {
            active += 1;
        }
    }
    listed >= 3 && active == 0
}

/// Active (`*`-prefixed) entries in a plugins.txt, or `None` if unreadable.
///
/// MUST go through `read_decoded`: plugins.txt is CP1252 on disk (the encoding
/// Eidos itself writes), and reading it as strict UTF-8 made this return `None`
/// for any list containing one accented plugin name - which made `active_loss`
/// return `None` too, silently disarming the only guard between a crash artifact
/// and the profile. A French load order is one translated mod away from that.
fn count_actives(path: &Path, mechanism: eidos_plugins::LoadOrderMechanism) -> Option<usize> {
    let text = eidos_plugins::read_decoded(path)?;
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    Some(match mechanism {
        // Asterisk: `*` marks active. Counting `*` on a PlainList file - where
        // NO line ever has one - read every healthy Fallout list as 0 actives
        // and every wipe as no-change, leaving those games without a backstop.
        eidos_plugins::LoadOrderMechanism::Asterisk => {
            lines.filter(|l| l.starts_with('*')).count()
        }
        // PlainList: every listed plugin IS active.
        eidos_plugins::LoadOrderMechanism::PlainList => lines.count(),
    })
}

/// Read a text file that may be UTF-8 or Windows ANSI (CP1252): game INIs come
/// in both. Returns the text plus whether it was CP1252, so a rewrite can keep
/// the encoding the GAME expects instead of silently converting the file.
///
/// Reading these with strict UTF-8 was a disease with three outbreaks: the
/// plugins wipe-guard went silent, the tweak merge treated the whole INI as
/// EMPTY (deploying the tweak fragment ALONE as the file), and the untweak pass
/// no-op'd - each triggered by a single accented byte.
pub fn read_text_lossy(path: &Path) -> Option<(String, bool)> {
    let bytes = fs::read(path).ok()?;
    match std::str::from_utf8(&bytes) {
        Ok(t) => Some((t.to_string(), false)),
        Err(_) => Some((encoding_rs::WINDOWS_1252.decode(&bytes).0.into_owned(), true)),
    }
}

/// Write `text` back in the encoding [`read_text_lossy`] found it in.
pub fn write_text(path: &Path, text: &str, cp1252: bool) -> io::Result<()> {
    if cp1252 {
        let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(text);
        fs::write(path, &bytes)
    } else {
        fs::write(path, text)
    }
}

/// Copy `src` to `dst` atomically: write a sibling `.tmp`, then rename over. A
/// plain `fs::copy` truncates the destination first, so a reader (or a crash)
/// mid-copy sees a torn file - and for the files this module moves around, a
/// torn `plugins.txt` is a wiped load order and a torn INI is a lost config.
pub(crate) fn copy_atomic(src: &Path, dst: &Path) -> io::Result<()> {
    let tmp = dst.with_extension("eidos-tmp");
    fs::copy(src, &tmp)?;
    match fs::rename(&tmp, dst) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Write `bytes` to `dst` through a temp file and a rename, so a reader never
/// sees a half-written file and a failure leaves the previous contents intact.
pub(crate) fn copy_atomic_bytes(dst: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = dst.with_extension("eidos-tmp");
    fs::write(&tmp, bytes)?;
    match fs::rename(&tmp, dst) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Recursively copy a directory tree, skipping symlinks (matching MO2's `copyDir`
/// NoSymLinks). Best-effort per entry.
fn copy_dir_recursive(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)?.flatten() {
        let Ok(meta) = e.metadata() else { continue };
        if meta.file_type().is_symlink() {
            continue;
        }
        let dst = to.join(e.file_name());
        if meta.is_dir() {
            let _ = copy_dir_recursive(&e.path(), &dst);
        } else {
            let _ = fs::copy(e.path(), &dst);
        }
    }
    Ok(())
}

impl Profile {
    /// Why writing `modlist.txt` right now would destroy the curated order rather
    /// than record an edit, or `None` when it is safe.
    ///
    /// The check is deliberately absolute rather than proportional, because the
    /// disaster is absolute: the mod pool is unreachable, so the in-memory list is
    /// missing EVERYTHING and any save flattens the order to nothing. A user who
    /// really did delete every mod hits this too and has to say so by removing
    /// `modlist.txt` themselves - an annoyance, weighed against permanently losing
    /// the one thing on disk that cannot be re-derived: which of forty overlapping
    /// mods wins each file conflict, and which are installed but deliberately off.
    ///
    /// MO2 has no equivalent. `Profile::refreshModStatus` rewrites the file inside
    /// the same refresh that dropped the entries, and the guard that looks like
    /// protection (`if (m_ModStatus.empty()) return;`) cannot fire because the
    /// synthetic overwrite entry always makes the count non-zero.
    fn unsafe_to_persist(&self) -> Option<String> {
        let listed = match fs::read_to_string(self.modlist_source()) {
            Ok(text) => text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .count(),
            // No list yet: nothing to lose, and this is how a fresh instance starts.
            Err(_) => return None,
        };
        if listed == 0 {
            return None;
        }
        match fs::read_dir(self.mods_dir()) {
            Err(e) => Some(format!(
                "the mods folder could not be read ({e}); refusing to overwrite a list of \
                 {listed} mod(s). Is the drive it lives on mounted?"
            )),
            Ok(rd) => {
                let any = rd
                    .flatten()
                    .any(|e| e.file_type().is_ok_and(|t| t.is_dir() || t.is_symlink()));
                (!any).then(|| {
                    format!(
                        "the mods folder is empty while the saved list has {listed} mod(s); \
                         refusing to overwrite it. If the mods really are gone, delete \
                         {} to start over.",
                        self.modlist_path().display()
                    )
                })
            }
        }
    }
}

/// Prefix of Eidos's own extraction temporaries under `mods/`. Only directories
/// starting with this are hidden from the mod list; every other name, leading dot
/// included, is a mod the user installed.
const INSTALL_TEMP_PREFIX: &str = ".eidos-install-";

/// Whether a freshly-scanned mod list may be written back over `modlist.txt`.
///
/// The reconciliation is destructive by nature - it exists to forget mods that are
/// gone - so it needs the same kind of sanity check the plugin-order capture has
/// (see [`active_loss`]). The order and enabled state are pure user labour,
/// derivable from nothing else on disk; the mod files themselves are replaceable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListTrust {
    /// The scan agrees with the file, or differs by a plausible amount. Safe to save.
    Good,
    /// The scan lost implausibly much. Fine to DISPLAY, never to persist. Carries a
    /// sentence explaining what was seen, for the status bar and the log.
    Suspect(String),
}

impl ListTrust {
    pub fn is_good(&self) -> bool {
        matches!(self, ListTrust::Good)
    }

    /// The reason a list is not to be trusted, if it is not.
    pub fn reason(&self) -> Option<&str> {
        match self {
            ListTrust::Good => None,
            ListTrust::Suspect(why) => Some(why),
        }
    }

    /// Judge a scan: `listed` entries parsed out of `modlist.txt`, `kept` of them
    /// still backed by a folder.
    ///
    /// Thresholds mirror [`active_loss`] deliberately - one rule of thumb for
    /// "this looks like an accident, not an edit" across the whole instance.
    fn judge(readable: bool, listed: usize, kept: usize) -> ListTrust {
        if !readable {
            return ListTrust::Suspect(
                "the mods folder could not be read (is the drive it lives on mounted?)".to_string(),
            );
        }
        let lost = listed.saturating_sub(kept);
        if lost == 0 {
            return ListTrust::Good;
        }
        // Everything vanishing at once is never a real edit. This is the unmounted
        // drive, the wrong instance root, the permissions accident.
        if kept == 0 && listed > 0 {
            return ListTrust::Suspect(format!(
                "all {listed} listed mod(s) are missing from the mods folder"
            ));
        }
        if listed > MIN_ACTIVES
            && lost > MAX_ABSOLUTE_DROP
            && (lost as f64 / listed as f64) > MAX_RELATIVE_DROP
        {
            return ListTrust::Suspect(format!(
                "{lost} of {listed} listed mod(s) are missing from the mods folder"
            ));
        }
        ListTrust::Good
    }
}

/// One key an INI tweak overwrote, kept so the post-run capture can restore the
/// profile's own value instead of adopting the tweak permanently.
///
/// Without this, tweaks are a one-way door: the launch writes them into the
/// deployed INI, `capture_inis` copies that file back into the profile, and by
/// the second launch the tweak is indistinguishable from a setting the user
/// chose. Disabling the fragment would then change nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TweakedKey {
    pub section: String,
    pub key: String,
    /// The value before any tweak touched it; `None` if the key was absent.
    pub before: Option<String>,
    /// What the last fragment wrote, so the capture can tell a value that is
    /// still ours from one the game or the user changed while running.
    pub after: String,
}

/// Apply one INI fragment to `text`, recording what it displaced. Returns whether
/// anything was written.
///
/// A deliberately dumb line parser, matching MO2's `mergeTweak` (profile.cpp:778):
/// blanks and `;` / `#` comments are skipped, `[Section]` sets the current
/// section, and a line is split on its FIRST `=` with both sides trimmed. Values
/// therefore may contain `=` and nothing a fragment says can corrupt the target -
/// the worst a malformed fragment can do is set nothing.
///
/// Keys outside any section are dropped: an INI's leading keys belong to no
/// section, and guessing one would write the tweak somewhere the engine never
/// reads.
fn merge_tweak(text: &mut String, fragment: &str, record: &mut Vec<TweakedKey>) -> bool {
    let mut section = String::new();
    let mut wrote = false;
    for line in fragment.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(s) = eidos_ini::section_header(line) {
            section = s.to_string();
            continue;
        }
        if section.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() {
            continue;
        }
        let current = eidos_ini::get_key(text, &section, key).map(|v| v.trim().to_string());
        // Only the FIRST fragment to touch a key records the original value; a
        // later one overwriting it must not record the earlier tweak as "before".
        match record
            .iter_mut()
            .find(|r| r.section.eq_ignore_ascii_case(&section) && r.key.eq_ignore_ascii_case(key))
        {
            Some(existing) => existing.after = value.to_string(),
            None => record.push(TweakedKey {
                section: section.clone(),
                key: key.to_string(),
                before: current,
                after: value.to_string(),
            }),
        }
        *text = eidos_ini::set_key(text, &section, key, value);
        wrote = true;
    }
    wrote
}

/// Undo what [`Profile::apply_ini_tweaks`] wrote, for the INI text captured back
/// from the prefix after a run.
///
/// A key is restored only if it still holds exactly what the tweak wrote. If the
/// game or the user changed it in-flight that is a real preference change and it
/// is kept - the tweak lost, which is the same rule MO2's "the user always wins"
/// ordering encodes at merge time.
pub fn untweak_ini(text: &str, record: &[TweakedKey]) -> String {
    let mut out = text.to_string();
    for r in record {
        let current = eidos_ini::get_key(&out, &r.section, &r.key).map(|v| v.trim().to_string());
        // "Unchanged since the tweak" cannot be an exact-text compare: the engine
        // re-serialises floats in its own style ("1.5" comes back "1.5000"), and
        // treating that as a user edit made the tweak permanent - the displaced
        // original became unrecoverable. Numerically equal = unchanged.
        let unchanged = match current.as_deref() {
            None => false,
            Some(c) if c == r.after => true,
            Some(c) => matches!(
                (c.parse::<f64>(), r.after.parse::<f64>()),
                // Compare at the engine's own serialisation grain (4 decimals):
                // it both re-formats ("8000" -> "8000.0000") and ROUNDS
                // ("0.66666667" -> "0.6667"), and exact f64 equality still
                // called the second case a user edit.
                (Ok(a), Ok(b)) if (a * 10_000.0).round() == (b * 10_000.0).round()
            ),
        };
        if !unchanged {
            continue;
        }
        out = match &r.before {
            Some(v) => eidos_ini::set_key(&out, &r.section, &r.key, v),
            None => eidos_ini::delete_key(&out, &r.section, &r.key),
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn inst_with_mods(mods: &[&str]) -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("eidos-prof-{}-{}", std::process::id(), n));
        for m in mods {
            fs::create_dir_all(root.join("mods").join(m)).unwrap();
        }
        root
    }

    fn prof(root: &Path, name: &str) -> Profile {
        Profile { instance_root: root.to_path_buf(), name: name.to_string() }
    }

    #[test]
    fn later_fragments_win_and_the_original_value_is_what_gets_restored() {
        let mut ini = "[Display]\nfDefaultFOV=75.0\niSize W=1920\n".to_string();
        let mut rec = Vec::new();

        assert!(merge_tweak(&mut ini, "[Display]\nfDefaultFOV=90.0\n", &mut rec));
        // A second fragment overwrites the first; `before` must still be vanilla,
        // or disabling both would leave the user on the first tweak's value.
        assert!(merge_tweak(&mut ini, "[Display]\nfDefaultFOV = 110.0\n", &mut rec));
        assert_eq!(eidos_ini::get_key(&ini, "Display", "fDefaultFOV"), Some("110.0"));
        assert_eq!(rec.len(), 1);
        assert_eq!(rec[0].before.as_deref(), Some("75.0"));
        assert_eq!(rec[0].after, "110.0");

        let restored = untweak_ini(&ini, &rec);
        assert_eq!(eidos_ini::get_key(&restored, "Display", "fDefaultFOV"), Some("75.0"));
        assert_eq!(eidos_ini::get_key(&restored, "Display", "iSize W"), Some("1920"));
    }

    #[test]
    fn a_key_the_game_changed_in_flight_keeps_its_new_value() {
        let mut ini = "[Display]\nfDefaultFOV=75.0\n".to_string();
        let mut rec = Vec::new();
        merge_tweak(&mut ini, "[Display]\nfDefaultFOV=90.0\n", &mut rec);
        // The user moved the FOV slider in-game, so the captured INI no longer
        // holds what the tweak wrote. Their choice wins over the restore.
        let captured = eidos_ini::set_key(&ini, "Display", "fDefaultFOV", "100.0");
        let restored = untweak_ini(&captured, &rec);
        assert_eq!(eidos_ini::get_key(&restored, "Display", "fDefaultFOV"), Some("100.0"));
    }

    #[test]
    fn a_key_the_tweak_invented_is_deleted_again_not_blanked() {
        let mut ini = "[Display]\niSize W=1920\n".to_string();
        let mut rec = Vec::new();
        merge_tweak(&mut ini, "[Papyrus]\nbEnableLogging=1\n", &mut rec);
        assert_eq!(rec[0].before, None);
        let restored = untweak_ini(&ini, &rec);
        // Absent, not `bEnableLogging=`: the engines read those differently.
        assert_eq!(eidos_ini::get_key(&restored, "Papyrus", "bEnableLogging"), None);
        assert!(restored.contains("[Papyrus]"));
    }

    #[test]
    fn a_fragment_cannot_corrupt_the_target() {
        let mut ini = "[Display]\niSize W=1920\n".to_string();
        let mut rec = Vec::new();
        let junk = concat!(
            "; a comment\n",
            "# another\n",
            "\n",
            "strayKey=1\n",             // outside any section: dropped
            "[[not a header\n",         // not a section either
            "[General]\n",
            "sTestFile1 = a=b=c\n",     // value keeps its own '='
            "=novalue\n",               // empty key: dropped
            "no equals sign at all\n",
        );
        merge_tweak(&mut ini, junk, &mut rec);
        assert_eq!(eidos_ini::get_key(&ini, "General", "sTestFile1"), Some("a=b=c"));
        assert_eq!(rec.len(), 1);
        // The pre-existing key survived untouched.
        assert_eq!(eidos_ini::get_key(&ini, "Display", "iSize W"), Some("1920"));
    }

    #[test]
    fn the_profile_tweak_file_is_applied_after_every_mod() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        fs::write(p.tweaks_path(), "[Display]\nfDefaultFOV=100.0\n").unwrap();

        let frag = root.join("frag.ini");
        fs::write(&frag, "[Display]\nfDefaultFOV=90.0\n").unwrap();
        let deployed = root.join("Skyrim.ini");
        fs::write(&deployed, "[Display]\nfDefaultFOV=75.0\n").unwrap();

        let rec = p.apply_ini_tweaks(&deployed, &[frag]).unwrap();
        let text = fs::read_to_string(&deployed).unwrap();
        // The profile's own file is last, so the user beats the mod.
        assert_eq!(eidos_ini::get_key(&text, "Display", "fDefaultFOV"), Some("100.0"));
        assert_eq!(rec[0].before.as_deref(), Some("75.0"));
        assert_eq!(rec[0].after, "100.0");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unmanaged_content_keeps_its_position_but_is_never_mounted() {
        // The game's own DLCs and Creation Club plugins belong in the list - four
        // mods beside eighty loading plugins is what makes a user ask whether
        // their DLC is there at all.
        //
        // They are written with MO2's `*`, which grants the row a POSITION without
        // claiming Eidos installed the files. Dropping them, as this once did,
        // meant they could only ever be re-discovered and pinned to the top: no
        // separator could sit above them, so the block could not be collapsed and
        // the noise could not be put away.
        let root = inst_with_mods(&["Real"]);
        let p = prof(&root, "Default");
        let e = |n: &str, un: bool| ModEntry {
            name: n.into(),
            enabled: true,
            path: if un { root.join("gamedata").join(n) } else { root.join("mods").join(n) },
            unmanaged: un,
        };
        p.save_modlist(&[e("Dawnguard", true), e("Real", false)]).unwrap();

        let written = fs::read_to_string(p.modlist_path()).unwrap();
        assert!(written.contains("+Real"), "{written}");
        assert!(written.contains("*Dawnguard"), "the game's content needs a line to have a place: {written}");

        // Read back, the row is still there, still marked as the game's.
        let (back, _) = p.modlist_checked();
        let dg = back.iter().find(|m| m.name == "Dawnguard").expect("row survived the round trip");
        assert!(dg.unmanaged, "a `*` line is the game's content, not a mod");
        assert!(dg.path.as_os_str().is_empty(), "this layer cannot know the game's data dir");
        // And the order is preserved: display runs lowest priority first, and it
        // was saved ahead of Real.
        assert_eq!(back.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["Dawnguard", "Real"]);

        // It is never a mount layer, whatever a caller hands us. This is what makes
        // writing the row safe: the `*` says "position only", and the one consumer
        // that could act on it refuses by name.
        let mounted = p.load_order();
        assert!(
            !mounted.iter().any(|m| m.to_string_lossy().contains("Dawnguard")),
            "{mounted:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn modlist_round_trips_per_profile() {
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        let mods = vec![
            ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
            ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A"), unmanaged: false },
            ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C"), unmanaged: false },
        ];
        p.save_modlist(&mods).unwrap();
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("B".into(), true), ("A".into(), false), ("C".into(), true)]);

        // The atomic write must leave no stray ".tmp" sibling behind.
        let leftover_tmp = fs::read_dir(p.dir())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().ends_with(".tmp"));
        assert!(!leftover_tmp, "save_modlist left a leftover .tmp file in the profile dir");

        let _ = fs::remove_dir_all(&root);
    }

    /// The curated order must survive a destroyed/partial `modlist.txt`: because
    /// the write is atomic (temp file then rename), a save never leaves an empty
    /// file that [`Profile::modlist`] would rebuild as "everything enabled,
    /// alphabetical". Guards FIX F1 (MO2 `SafeWriteFile`/`QSaveFile` parity).
    #[test]
    fn save_modlist_is_atomic_and_keeps_a_backup() {
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        let v1 = vec![
            ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C"), unmanaged: false },
            ModEntry { name: "B".into(), enabled: false, path: root.join("mods/B"), unmanaged: false },
            ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
        ];
        p.save_modlist(&v1).unwrap();

        // A second save (a toggle/move) over an existing list: backs the old one
        // up and swaps atomically.
        let v2 = vec![
            ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A"), unmanaged: false },
            ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C"), unmanaged: false },
            ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
        ];
        p.save_modlist(&v2).unwrap();

        // The live file reflects the latest curated order (not the alphabetical default).
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("A".into(), false), ("C".into(), true), ("B".into(), true)]);

        // The one-deep backup holds the previous list and sits in the same dir.
        // The file stores highest-priority first (reverse of the in-memory v1).
        let bak = p.dir().join("modlist.txt.bak");
        assert!(bak.is_file(), "expected a one-deep modlist.txt.bak backup");
        assert_eq!(fs::read_to_string(&bak).unwrap(), "+A\n-B\n+C\n");

        // No temp file lingers after a successful save.
        assert!(!p.dir().join("modlist.txt.tmp").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_from_copies_saves_subdir() {
        let root = inst_with_mods(&["A"]);
        let src = prof(&root, "Src");
        src.create().unwrap();
        // A save in the source profile's saves/ subdir + a curated modlist.
        let saves = src.dir().join("saves");
        fs::create_dir_all(&saves).unwrap();
        fs::write(saves.join("Save1.ess"), b"x").unwrap();
        src.save_modlist(&[ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A"), unmanaged: false }])
            .unwrap();

        let dst = prof(&root, "Copy");
        dst.create_from(&src).unwrap();
        // The saves/ subdir is copied recursively (MO2 parity), not skipped...
        assert!(dst.dir().join("saves/Save1.ess").is_file());
        // ...and the modlist file came across too.
        assert!(!dst.modlist().iter().find(|m| m.name == "A").unwrap().enabled);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn modlist_parses_star_and_trims_names() {
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        // MO2 '*' foreign line (enabled), and +/- with padding that must be trimmed.
        // The file is highest-priority first; modlist() returns it reversed (display order).
        fs::write(p.modlist_path(), "*A\n-  B\n+ C \n").unwrap();
        let got: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(got, vec![("C".into(), true), ("B".into(), false), ("A".into(), true)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn two_profiles_share_mods_but_keep_own_order() {
        let root = inst_with_mods(&["A", "B"]);
        prof(&root, "Default")
            .save_modlist(&[
                ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
                ModEntry { name: "B".into(), enabled: false, path: root.join("mods/B"), unmanaged: false },
            ])
            .unwrap();
        prof(&root, "Test")
            .save_modlist(&[
                ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
                ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
            ])
            .unwrap();
        let d: Vec<_> = prof(&root, "Default").modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        let t: Vec<_> = prof(&root, "Test").modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(d, vec![("A".into(), true), ("B".into(), false)]);
        assert_eq!(t, vec![("B".into(), true), ("A".into(), true)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn default_profile_falls_back_to_legacy_flat_modlist() {
        let root = inst_with_mods(&["A", "B"]);
        // A pre-profiles instance: a flat <root>/modlist.txt (highest-priority first).
        fs::write(root.join("modlist.txt"), "-A\n+B\n").unwrap();
        let p = prof(&root, "Default");
        // modlist() returns display order (reverse of the file): B (top) then A.
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("B".into(), true), ("A".into(), false)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_nobody_listed_appears_disabled() {
        let root = inst_with_mods(&["A", "New"]);
        let p = prof(&root, "Default");
        p.save_modlist(&[ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false }]).unwrap();
        // "New" exists on disk but not in the saved list. It appears, but DISABLED
        // (MO2 parity): nothing knows where in the conflict order it belongs, and
        // silently enabling it could overwrite half the load order's files on the
        // next launch. A mod installed THROUGH Eidos never takes this path - the
        // installer writes its own modlist entry.
        let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(read, vec![("New".into(), false), ("A".into(), true)]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_whose_folder_is_gone_leaves_the_list_but_not_the_file() {
        let root = inst_with_mods(&["A", "B"]);
        let p = prof(&root, "Default");
        let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n), unmanaged: false };
        p.save_modlist(&[e("A"), e("B")]).unwrap();

        fs::remove_dir_all(root.join("mods/B")).unwrap();
        let (list, trust) = p.modlist_checked();
        assert_eq!(list.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["A"]);
        // One of two gone is an ordinary edit, not an accident.
        assert!(trust.is_good(), "{trust:?}");
        // The file still says both until something saves - the drop is a view.
        assert!(fs::read_to_string(p.modlist_path()).unwrap().contains("B"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unmounted_mods_folder_cannot_flatten_the_order() {
        // The disaster case: mods/ lives on another drive via a bind mount, and the
        // mount is not up. The directory EXISTS and is READABLE and is EMPTY, so
        // every guard that only checks for existence sails straight through.
        let root = inst_with_mods(&["A", "B", "C"]);
        let p = prof(&root, "Default");
        let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n), unmanaged: false };
        p.save_modlist(&[e("A"), e("B"), e("C")]).unwrap();
        let before = fs::read_to_string(p.modlist_path()).unwrap();

        for m in ["A", "B", "C"] {
            fs::remove_dir_all(root.join("mods").join(m)).unwrap();
        }
        let (list, trust) = p.modlist_checked();
        assert!(list.is_empty());
        assert!(!trust.is_good(), "an empty scan against a non-empty list must not be trusted");

        // And the save is refused rather than silently flattening the order.
        let err = p.save_modlist(&[]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read_to_string(p.modlist_path()).unwrap(), before);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unreadable_mods_folder_is_not_an_empty_one() {
        use std::os::unix::fs::PermissionsExt;
        let root = inst_with_mods(&["A", "B"]);
        let p = prof(&root, "Default");
        let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n), unmanaged: false };
        p.save_modlist(&[e("A"), e("B")]).unwrap();

        let mods = root.join("mods");
        fs::set_permissions(&mods, fs::Permissions::from_mode(0o000)).unwrap();
        let (_, trust) = p.modlist_checked();
        let refused = p.save_modlist(&[]).is_err();
        // Restore before asserting, so a failure does not leave an unremovable dir.
        fs::set_permissions(&mods, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(!trust.is_good(), "a read error must not read as 'you have no mods'");
        assert!(refused);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_whose_name_starts_with_a_dot_is_kept() {
        // ".NET Script Framework" is a real, near-universal Skyrim SE dependency.
        // Only Eidos's own extraction temps are hidden from the list.
        let root = inst_with_mods(&[".NET Script Framework", ".eidos-install-abc123", "A"]);
        let p = prof(&root, "Default");
        let names: Vec<String> = p.modlist().iter().map(|m| m.name.clone()).collect();
        assert!(names.iter().any(|n| n == ".NET Script Framework"), "{names:?}");
        assert!(!names.iter().any(|n| n.starts_with(".eidos-install-")), "{names:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_dangling_symlink_is_still_a_row() {
        // A mod symlinked to a drive that is not mounted is BROKEN, not absent: its
        // position, enabled state and intended target are the irreplaceable part,
        // and dropping the row throws them away. `path().is_dir()` follows the link
        // and cannot tell this from a deleted mod; `file_type()` can.
        let root = inst_with_mods(&["A"]);
        std::os::unix::fs::symlink(root.join("nowhere"), root.join("mods/Linked")).unwrap();
        let p = prof(&root, "Default");
        let names: Vec<String> = p.modlist().iter().map(|m| m.name.clone()).collect();
        assert!(names.iter().any(|n| n == "Linked"), "{names:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn inis_seed_deploy_and_capture_round_trip() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        // A fake prefix Documents dir holding the user's existing INIs.
        let prefix = root.join("prefix-docs");
        fs::create_dir_all(&prefix).unwrap();
        fs::write(prefix.join("Skyrim.ini"), "[General]\nsLanguage=ENGLISH\n").unwrap();
        fs::write(prefix.join("SkyrimPrefs.ini"), "[Display]\niSize W=1920\n").unwrap();
        let inis = ["Skyrim.ini", "SkyrimPrefs.ini"];

        // Seed adopts both into the profile; seeding again copies nothing.
        assert_eq!(p.seed_inis(&prefix, &inis).unwrap(), 2);
        assert!(p.ini_path("Skyrim.ini").is_file());
        assert_eq!(p.seed_inis(&prefix, &inis).unwrap(), 0);

        // The profile is now the source of truth: edit its copy, deploy elsewhere.
        fs::write(p.ini_path("Skyrim.ini"), "[General]\nsLanguage=FRENCH\n").unwrap();
        let prefix2 = root.join("prefix2");
        assert_eq!(p.deploy_inis(&prefix2, &inis).unwrap(), 2);
        assert!(fs::read_to_string(prefix2.join("Skyrim.ini")).unwrap().contains("FRENCH"));

        // The game writes to the prefix; capture pulls the change back.
        fs::write(prefix2.join("SkyrimPrefs.ini"), "[Display]\niSize W=2560\n").unwrap();
        assert_eq!(p.capture_inis(&prefix2, &inis).unwrap(), 2);
        assert!(fs::read_to_string(p.ini_path("SkyrimPrefs.ini")).unwrap().contains("2560"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn saves_seed_adopts_existing_then_skips() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        let prefix_saves = root.join("prefix-saves");
        fs::create_dir_all(&prefix_saves).unwrap();
        fs::write(prefix_saves.join("Save1.ess"), b"x").unwrap();
        fs::write(prefix_saves.join("Save2.ess"), b"y").unwrap();

        // First run adopts the existing playthrough.
        assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 2);
        assert!(p.saves_dir().join("Save1.ess").is_file());
        // Profile already has saves -> never re-seed (would clobber progress).
        fs::write(prefix_saves.join("Save3.ess"), b"z").unwrap();
        assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn savegames_lists_files_newest_first_and_skips_dirs() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        let saves = p.saves_dir();
        fs::create_dir_all(&saves).unwrap();
        // Write `Old` first, then sleep so `New` gets a strictly later mtime.
        fs::write(saves.join("Old.ess"), b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(saves.join("New.ess"), b"newer-and-bigger").unwrap();
        // A subdirectory and a dotfile are both ignored.
        fs::create_dir_all(saves.join("backup")).unwrap();
        fs::write(saves.join(".DS_Store"), b"junk").unwrap();

        let list = p.savegames();
        let names: Vec<_> = list.iter().map(|s| s.filename.clone()).collect();
        assert_eq!(names, vec!["New.ess".to_string(), "Old.ess".to_string()]);
        assert_eq!(list[0].size, "newer-and-bigger".len() as u64);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn savegames_is_empty_when_no_saves_dir() {
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        assert!(p.savegames().is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn display_order_file_order_and_load_order_stay_consistent() {
        // The orientation contract: modlist() is MO2 display order (lowest priority
        // at the top); the file stores highest-priority first; load_order() re-reverses
        // to highest-first for the mount. A regression here silently inverts conflicts.
        let root = inst_with_mods(&["Low", "High"]);
        let p = prof(&root, "Default");
        // Display order: Low at the top (lowest priority), High at the bottom (highest).
        p.save_modlist(&[
            ModEntry { name: "Low".into(), enabled: true, path: root.join("mods/Low"), unmanaged: false },
            ModEntry { name: "High".into(), enabled: true, path: root.join("mods/High"), unmanaged: false },
        ])
        .unwrap();
        // The file is highest-priority first (MO2 on-disk convention).
        assert_eq!(fs::read_to_string(p.dir().join("modlist.txt")).unwrap(), "+High\n+Low\n");
        // modlist() round-trips the display order.
        let names: Vec<_> = p.modlist().iter().map(|m| m.name.clone()).collect();
        assert_eq!(names, vec!["Low".to_string(), "High".to_string()]);
        // load_order() mounts highest priority first, so High wins same-name conflicts.
        assert_eq!(p.load_order(), vec![root.join("mods/High"), root.join("mods/Low")]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn separator_round_trips_keeps_position_and_is_excluded_from_load_order() {
        // A separator is a real `*_separator` folder; it must round-trip in place,
        // be recognised as a separator, and never become a mount layer.
        let root = inst_with_mods(&["A", "Sec_separator", "B"]);
        let p = prof(&root, "Default");
        let mods = vec![
            ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
            ModEntry { name: "Sec_separator".into(), enabled: false, path: root.join("mods/Sec_separator"), unmanaged: false },
            ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
        ];
        p.save_modlist(&mods).unwrap();

        // modlist.txt is byte-faithful, including the `-` prefix + `_separator` suffix,
        // and stored highest-priority first (reverse of the in-memory display order).
        assert_eq!(fs::read_to_string(p.dir().join("modlist.txt")).unwrap(), "+B\n-Sec_separator\n+A\n");

        // Read back: order + the separator flag preserved, separator at index 1.
        let read = p.modlist();
        let names: Vec<_> = read.iter().map(|m| (m.name.clone(), m.enabled)).collect();
        assert_eq!(
            names,
            vec![("A".into(), true), ("Sec_separator".into(), false), ("B".into(), true)]
        );
        assert!(read[1].is_separator());
        assert_eq!(read[1].display_name(), "Sec");
        assert!(!read[0].is_separator());

        // load_order mounts only A and B - the separator is content-less - in
        // highest-priority-first order (B is below A in the display, so it wins).
        let order = p.load_order();
        assert_eq!(order, vec![root.join("mods/B"), root.join("mods/A")]);
        let _ = fs::remove_dir_all(&root);

        // An ENABLED separator (alone) still contributes no mount layer.
        let root2 = inst_with_mods(&["Solo_separator"]);
        let p2 = prof(&root2, "Default");
        p2.save_modlist(&[ModEntry {
            name: "Solo_separator".into(),
            enabled: true,
            path: root2.join("mods/Solo_separator"), unmanaged: false }])
        .unwrap();
        assert!(p2.load_order().is_empty());
        let _ = fs::remove_dir_all(&root2);
    }

    #[test]
    fn a_crash_mangled_session_is_flagged_and_restorable() {
        // Under the bind-mount design the game writes the profile's plugins.txt
        // DIRECTLY, so there is no capture moment to refuse. The pre-session
        // snapshot restores the choke point: a session that wrecked the active
        // set is flagged, and one call puts the pre-session state back.
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();

        // A real 200-plugin order, snapshotted at launch.
        let full: String = (0..200).map(|i| format!("*Mod{i}.esp\n")).collect();
        fs::write(p.plugins_txt_path(), &full).unwrap();
        p.snapshot_plugin_state().unwrap();

        // The game dies during shutdown and leaves the active set mostly cleared.
        let mangled: String = (0..200)
            .map(|i| if i < 5 { format!("*Mod{i}.esp\n") } else { format!("Mod{i}.esp\n") })
            .collect();
        fs::write(p.plugins_txt_path(), &mangled).unwrap();
        assert!(
            p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some(),
            "a crash artefact must be flagged, or the user never learns their order died"
        );
        p.restore_plugin_snapshot().unwrap();
        assert_eq!(fs::read_to_string(p.plugins_txt_path()).unwrap(), full);
        assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_none(), "restored = healthy");

        // A legitimate edit - the user turning a handful of mods off - is not
        // flagged; sessions that edit must not cry wolf.
        let edited: String = (0..200)
            .map(|i| if i < 195 { format!("*Mod{i}.esp\n") } else { format!("Mod{i}.esp\n") })
            .collect();
        fs::write(p.plugins_txt_path(), &edited).unwrap();
        assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn small_load_order_losses_follow_the_majority_rule() {
        // The check is a warn-with-one-click-dismiss now, not a silent refusal,
        // so the trade changed: turning off a couple of plugins must stay
        // silent, but losing the MAJORITY of a small list flags - that shape is
        // also what a partial crash artifact looks like, and it used to slide
        // under the big-list floor unchallenged.
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        fs::write(p.plugins_txt_path(), "*a\n*b\n*c\n*d\n*e\n*f\n").unwrap();
        p.snapshot_plugin_state().unwrap();

        // Two of six off: routine, silent.
        fs::write(p.plugins_txt_path(), "*a\n*b\n*c\n*d\ne\nf\n").unwrap();
        assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_none());

        // Four of six off: majority loss, flagged (dismissable in one click).
        fs::write(p.plugins_txt_path(), "*a\n*b\nc\nd\ne\nf\n").unwrap();
        assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_accented_plugin_name_does_not_disarm_the_wipe_guard() {
        // plugins.txt is CP1252 on disk - the encoding Eidos itself writes - so a
        // guard that reads it as strict UTF-8 returns None on the first accented
        // name and silently stops guarding. One translated mod ("Épées de
        // Bordeciel.esp") was enough to reopen the wipe this guard exists for.
        let root = inst_with_mods(&["A"]);
        let prefix = root.join("prefix");
        fs::create_dir_all(&prefix).unwrap();
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();

        // The profile's list holds an accented name, CP1252-encoded (0xC9 = 'É').
        let mut good = b"*\xC9p\xE9es de Bordeciel.esp\r\n".to_vec();
        good.extend_from_slice(b"*a.esp\r\n*b.esp\r\n*c.esp\r\n*d.esp\r\n*e.esp\r\n*f.esp\r\n");
        fs::write(p.plugins_txt_path(), &good).unwrap();
        assert!(
            std::str::from_utf8(&good).is_err(),
            "the fixture must be real CP1252, not accidentally-valid UTF-8"
        );
        p.snapshot_plugin_state().unwrap();

        // The game crashes and leaves a header-only artifact.
        fs::write(p.plugins_txt_path(), b"# ruined\r\n").unwrap();
        assert!(
            p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some(),
            "the wipe must be flagged even when the list has accented names"
        );
        p.restore_plugin_snapshot().unwrap();
        assert_eq!(fs::read(p.plugins_txt_path()).unwrap(), good);
    }

    #[test]
    fn a_small_load_order_cleared_to_nothing_is_still_refused() {
        // The case that actually bit: Skyrim rewrote plugins.txt with nothing but
        // its own header while a 7-plugin order was live. That is below the floor
        // the proportional rule uses, so it went through unchallenged and the
        // profile lost every active plugin - which is exactly the state a user
        // adding mods a few at a time is in.
        let root = inst_with_mods(&["A"]);
        let prefix = root.join("prefix");
        fs::create_dir_all(&prefix).unwrap();
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        let good = "*a.esp\n*b.esp\n*c.esp\n*d.esp\n*e.esp\n*f.esp\n*g.esp\n";
        fs::write(p.plugins_txt_path(), good).unwrap();
        p.snapshot_plugin_state().unwrap();
        fs::write(
            p.plugins_txt_path(),
            "# This file is used by Skyrim to keep track of your downloaded content.\n",
        )
        .unwrap();
        assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some());
        p.restore_plugin_snapshot().unwrap();
        assert_eq!(fs::read_to_string(p.plugins_txt_path()).unwrap(), good);

        // Turning every plugin off BY HAND still flags - the backstop cannot read
        // minds - but the names stay listed, so nothing is lost and the user just
        // dismisses the warning instead of losing their order.
        let all_off = "a.esp\nb.esp\nc.esp\nd.esp\ne.esp\nf.esp\ng.esp\n";
        fs::write(p.plugins_txt_path(), all_off).unwrap();
        assert!(
            p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some(),
            "clearing every active plugin is flagged at any size"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn plugin_state_is_seeded_and_stays_per_profile() {
        let root = inst_with_mods(&["A"]);
        let prefix = root.join("prefix");
        fs::create_dir_all(&prefix).unwrap();
        fs::write(prefix.join("plugins.txt"), b"*Alpha.esp\nBeta.esp\n").unwrap();
        fs::write(prefix.join("loadorder.txt"), b"Alpha.esp\nBeta.esp\n").unwrap();
        // The game keeps sidecar files next to them; the bind must carry those
        // too, or the bound dir shows the game less than the dir it wrote.
        fs::write(prefix.join("ContentCatalog.txt"), b"{}").unwrap();
        // A crashed write's leftover must NOT be adopted.
        fs::write(prefix.join("plugins.tmp"), b"junk").unwrap();

        // Seed: the profile adopts the prefix's existing state once.
        let a = prof(&root, "Default");
        assert!(!a.has_plugin_state());
        assert_eq!(a.seed_plugin_state(&prefix, &eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).unwrap(), 3);
        assert!(a.has_plugin_state());
        assert!(a.plugins_state_dir().join("ContentCatalog.txt").is_file());
        assert!(!a.plugins_state_dir().join("plugins.tmp").exists());
        // Seeding again must not clobber the profile's own copy.
        fs::write(a.plugins_txt_path(), b"*Alpha.esp\n").unwrap();
        assert_eq!(a.seed_plugin_state(&prefix, &eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).unwrap(), 0);
        assert_eq!(fs::read(a.plugins_txt_path()).unwrap(), b"*Alpha.esp\n");

        // A second profile has its own, independent state - the bound dir swaps
        // with the profile, so nothing leaks between them.
        let b = prof(&root, "Testing");
        assert!(!b.has_plugin_state());
        fs::write(b.plugins_txt_path(), b"*Beta.esp\n").unwrap();
        assert_eq!(fs::read(a.plugins_txt_path()).unwrap(), b"*Alpha.esp\n");
        assert_eq!(fs::read(b.plugins_txt_path()).unwrap(), b"*Beta.esp\n");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_backstop_sees_a_plainlist_wipe() {
        // PlainList files have no `*` at all, so counting asterisks read every
        // healthy Fallout list as "0 active" and every wipe as no-change - the
        // backstop was stone dead for that whole family.
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        let spec = eidos_plugins::GameSpec::for_id("falloutnv").unwrap();

        fs::write(p.plugins_txt_path(), b"FalloutNV.esm\nModA.esp\nModB.esp\n").unwrap();
        p.snapshot_plugin_state().unwrap();

        // Healthy rewrite: same actives, no flag.
        fs::write(p.plugins_txt_path(), b"FalloutNV.esm\nModA.esp\nModB.esp\n").unwrap();
        assert!(p.plugin_loss_since_snapshot(&spec).is_none());

        // The wipe: header only. Must flag at any size.
        fs::write(p.plugins_txt_path(), b"# nothing\n").unwrap();
        assert!(
            p.plugin_loss_since_snapshot(&spec).is_some(),
            "a PlainList wipe must be flagged, not read as 0-vs-0"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn seeding_adopts_verbatim_for_every_mechanism() {
        // The founding rule is ADOPT VERBATIM, always. An earlier "refuse the
        // crash artifact" version was worse than the disease: the same run then
        // derived everything-ENABLED from discovery and shadow-wrote it over the
        // prefix - and its signature (names listed, no `*`) is what every
        // healthy PlainList plugins.txt looks like, so Fallout and Skyrim LE
        // setups were refused wholesale. The artifact case is a WARNING now.
        let root = inst_with_mods(&["A"]);
        let prefix = root.join("prefix");
        fs::create_dir_all(&prefix).unwrap();
        // Asterisk game, names listed, zero active: adopted anyway, byte-for-byte.
        let artifact = b"a.esp\nb.esp\nc.esp\nd.esp\n";
        fs::write(prefix.join("plugins.txt"), artifact).unwrap();
        let p = prof(&root, "Default");
        p.seed_plugin_state(&prefix, &eidos_plugins::GameSpec::for_id("skyrimse").unwrap())
            .unwrap();
        assert!(p.has_plugin_state());
        assert_eq!(fs::read(p.plugins_txt_path()).unwrap(), artifact, "verbatim, not derived");

        // PlainList game (Fallout NV): a healthy actives-without-asterisks file
        // is NORMAL and adopts silently.
        let root2 = inst_with_mods(&["A"]);
        let prefix2 = root2.join("prefix");
        fs::create_dir_all(&prefix2).unwrap();
        let healthy = b"FalloutNV.esm\nSomeMod.esp\nOtherMod.esp\n";
        fs::write(prefix2.join("plugins.txt"), healthy).unwrap();
        let p2 = prof(&root2, "Default");
        p2.seed_plugin_state(&prefix2, &eidos_plugins::GameSpec::for_id("falloutnv").unwrap())
            .unwrap();
        assert!(p2.has_plugin_state());
        assert_eq!(fs::read(p2.plugins_txt_path()).unwrap(), healthy);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&root2);
    }

    #[test]
    fn a_truncated_ini_is_not_captured_over_the_profile() {
        let root = inst_with_mods(&["A"]);
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();

        let good = "[Display]\n".to_string() + &"iKey=1\n".repeat(50);
        fs::write(p.ini_path("Skyrim.ini"), &good).unwrap();

        // Empty: never captured.
        fs::write(docs.join("Skyrim.ini"), b"").unwrap();
        assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 0);
        assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), good);

        // Under half the profile's size: a wreck, not an edit.
        fs::write(docs.join("Skyrim.ini"), b"[Display]\niKey=1\n").unwrap();
        assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 0);
        assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), good);

        // A real edit (same order of size) captures.
        let edited = good.replace("iKey=1", "iKey=2");
        fs::write(docs.join("Skyrim.ini"), &edited).unwrap();
        assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 1);
        assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), edited);

        // The engine's own compact rewrite is STABLE: refused once, but the
        // same size on the next run is the real format and must be accepted -
        // refusing forever would mean in-game settings never persist again.
        let compact = "[Display]\niKey=3\n";
        fs::write(docs.join("Skyrim.ini"), compact).unwrap();
        assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 0, "first sight: refused");
        assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 1, "stable repeat: accepted");
        assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), compact);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn emptying_the_saves_dir_does_not_resurrect_prefix_saves() {
        let root = inst_with_mods(&["A"]);
        let prefix_saves = root.join("prefix_saves");
        fs::create_dir_all(&prefix_saves).unwrap();
        fs::write(prefix_saves.join("ancient.ess"), b"2024").unwrap();
        fs::write(prefix_saves.join("steam_autocloud.vdf"), b"junk").unwrap();

        let p = prof(&root, "Default");
        assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 1, "junk is not a save");
        assert!(p.saves_dir().join("ancient.ess").is_file());
        assert!(!p.saves_dir().join("steam_autocloud.vdf").exists());

        // The user empties the dir on purpose. The old emptiness probe re-seeded
        // the ancient save with a fresh mtime that sorted above everything.
        fs::remove_file(p.saves_dir().join("ancient.ess")).unwrap();
        assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 0, "seeding is once, ever");
        assert!(!p.saves_dir().join("ancient.ess").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_float_reserialised_by_the_engine_still_untweaks() {
        // The tweak set fShadowDistance=8000; the engine rewrote it "8000.0000".
        // Text-compare said "user changed it" and kept the tweak forever.
        let mut ini = "[Display]\nfShadowDistance=4000\n".to_string();
        let mut rec = Vec::new();
        assert!(merge_tweak(&mut ini, "[Display]\nfShadowDistance=8000\n", &mut rec));
        let engine_rewritten = ini.replace("fShadowDistance=8000", "fShadowDistance=8000.0000");
        let restored = untweak_ini(&engine_rewritten, &rec);
        assert!(
            restored.contains("fShadowDistance=4000"),
            "numerically-equal means unchanged; the original must come back: {restored}"
        );

        // A REAL user change (different number) still wins over the restore.
        let user_changed = ini.replace("fShadowDistance=8000", "fShadowDistance=6500");
        let kept = untweak_ini(&user_changed, &rec);
        assert!(kept.contains("fShadowDistance=6500"), "{kept}");
    }

    #[test]
    fn the_legacy_top_level_plugin_files_migrate_into_the_plugins_dir() {
        // Profiles created before the bind-mount design kept plugins.txt and
        // loadorder.txt at the profile top level. First access must move them in,
        // or every existing user starts from an empty load order.
        let root = inst_with_mods(&["A"]);
        let p = prof(&root, "Default");
        fs::create_dir_all(p.dir()).unwrap();
        fs::write(p.dir().join("plugins.txt"), b"*Old.esp\n").unwrap();
        fs::write(p.dir().join("loadorder.txt"), b"Old.esp\n").unwrap();

        let dir = p.plugins_state_dir();
        assert_eq!(fs::read(dir.join("plugins.txt")).unwrap(), b"*Old.esp\n");
        assert_eq!(fs::read(dir.join("loadorder.txt")).unwrap(), b"Old.esp\n");
        assert!(!p.dir().join("plugins.txt").exists(), "the legacy copy must MOVE, not fork");
        assert!(p.has_plugin_state());
        let _ = fs::remove_dir_all(&root);
    }
}
