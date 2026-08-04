//! Per-profile INIs: seeding, deploying into the prefix, capturing back,
//! and the reversible tweak fragments.

//! A profile: one named set of enabled mods + their order (and, later, its own
//! `plugins.txt`, INIs and saves), all sharing the instance's single `mods/`
//! pool. This is what lets one mod collection serve several playthroughs.
//!
//! Mirrors Mod Organizer 2: a profile is just a directory under
//! `<instance>/profiles/<name>/`; its `modlist.txt` carries both the enabled set
//! and the priority order, while the mods themselves stay global.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};



use super::*;

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
pub(crate) fn merge_tweak(text: &mut String, fragment: &str, record: &mut Vec<TweakedKey>) -> bool {
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

impl Profile {
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
}
