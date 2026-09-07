//! `eidos-backup.ini`: what the archive says about itself.
//!
//! The tree in the archive is only half of a backup. The other half is what a
//! plain copy cannot carry: where the instance USED to live (so the absolute
//! paths inside it can be repaired), which tools it expects to find on the new
//! machine, and what was deliberately left out - so the person unpacking it is
//! told, rather than discovering it a week later.
//!
//! Our own `key=value` dialect, like `eidos-instance.ini` next to it and for the
//! same reason: it is read by a human at least as often as by Eidos, and the
//! answer to "what is in this file" should be `cat`.

use std::fmt::Write as _;

use eidos_instance::Instance;

use crate::plan::{Outside, Plan};
use crate::{Options, SCHEMA_VERSION};

/// How many individually skipped items the manifest records before it stops
/// listing and starts counting. The rules themselves are always listed; this cap
/// is for the per-file surprises (a symlink, a name that is not UTF-8), where a
/// thousand lines would bury the eight that matter.
const LEFT_OUT_CAP: usize = 200;

/// What `eidos-backup.ini` holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupManifest {
    pub schema_version: u32,
    pub eidos_version: String,
    pub created: String,
    pub game_id: String,
    /// `true` for a portable instance, `false` for a central one.
    pub portable: bool,
    /// The instance root on the machine that made the backup. This is what makes
    /// the relocation pass possible - without it, an absolute path inside the
    /// archive cannot be told apart from one that was always meant to be
    /// absolute.
    pub source_root: String,
    pub active_profile: String,
    pub profiles: Vec<String>,
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
    /// Whether `downloads/` is in the archive.
    pub downloads: bool,
    pub tools_outside: Vec<Outside>,
    /// `(path, reason)`, capped at [`LEFT_OUT_CAP`].
    pub left_out: Vec<(String, String)>,
    /// How many more were left out beyond the ones listed.
    pub left_out_more: u64,
}

/// Whether a value is a single ordinary path component: no separator, no
/// traversal, no drive letter, nothing that could be read as absolute.
///
/// Deliberately strict rather than clever. A game id is `skyrimse`; anything
/// that is not shaped like one is not worth guessing about when the answer
/// decides where 77 GB gets written.
fn is_one_plain_segment(v: &str) -> bool {
    !v.is_empty()
        && v != "."
        && v != ".."
        && v.len() <= 64
        && !v.contains(['/', '\\', '\0'])
        && !v.contains(':')
        && !v.chars().any(char::is_control)
}

/// A value safe to write into a one-line `key=value` file. Filenames on Linux
/// may contain newlines; a manifest that a filename can tear in half is a
/// manifest that stops parsing halfway through a backup's description.
fn one_line(v: &str) -> String {
    v.replace(['\r', '\n'], " ")
}

impl BackupManifest {
    /// Describe an instance and the plan about to be packed.
    pub fn describe(inst: &Instance, plan: &Plan, opt: &Options) -> BackupManifest {
        let stored = inst.read_manifest();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut left_out: Vec<(String, String)> = plan
            .left
            .iter()
            .take(LEFT_OUT_CAP)
            .map(|l| (l.path.clone(), l.why.to_string()))
            .collect();
        left_out.sort();
        BackupManifest {
            schema_version: SCHEMA_VERSION,
            eidos_version: env!("CARGO_PKG_VERSION").to_string(),
            created: eidos_instance::format_stamp(now),
            game_id: stored
                .as_ref()
                .map(|m| m.game_id.clone())
                .or_else(|| inst.game_id())
                .unwrap_or_default(),
            portable: !matches!(
                stored.as_ref().map(|m| m.kind),
                Some(eidos_instance::InstanceKind::Global)
            ),
            source_root: inst.root.to_string_lossy().into_owned(),
            active_profile: inst.active_profile(),
            profiles: inst.profiles(),
            files: plan.files,
            directories: plan.dirs,
            bytes: plan.bytes,
            downloads: opt.downloads,
            tools_outside: plan.tools_outside.clone(),
            left_out,
            left_out_more: (plan.left.len().saturating_sub(LEFT_OUT_CAP)) as u64,
        }
    }

    /// The file, as it goes into the archive.
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(
            "# Written by Eidos when this backup was made, and read by `eidos unpack`.\n\
             # Deleting it turns the file back into an ordinary 7-Zip archive: the mods\n\
             # are still all there, but nothing knows where they came from.\n\n",
        );
        s.push_str("[backup]\n");
        let _ = writeln!(s, "schema_version={}", self.schema_version);
        let _ = writeln!(s, "eidos_version={}", one_line(&self.eidos_version));
        let _ = writeln!(s, "created={}", one_line(&self.created));
        let _ = writeln!(s, "game_id={}", one_line(&self.game_id));
        let _ = writeln!(
            s,
            "kind={}",
            if self.portable { "portable" } else { "global" }
        );
        let _ = writeln!(s, "source_root={}", one_line(&self.source_root));
        let _ = writeln!(s, "active_profile={}", one_line(&self.active_profile));
        let _ = writeln!(s, "files={}", self.files);
        let _ = writeln!(s, "directories={}", self.directories);
        let _ = writeln!(s, "bytes={}", self.bytes);
        let _ = writeln!(s, "downloads={}", self.downloads);

        s.push_str("\n[profiles]\n");
        for p in &self.profiles {
            let _ = writeln!(s, "profile={}", one_line(p));
        }

        s.push_str(
            "\n# Tools this instance uses that live outside it. They are NOT in the\n\
             # archive - install them on the new machine and point Eidos at them.\n\
             [tools-outside]\n",
        );
        for t in &self.tools_outside {
            let _ = writeln!(s, "name={}", one_line(&t.title));
            let _ = writeln!(s, "exe={}", one_line(&t.exe));
        }

        s.push_str(
            "\n# What was deliberately not packed, and why.\n\
             [left-out]\n",
        );
        for (path, why) in &self.left_out {
            let _ = writeln!(s, "path={}", one_line(path));
            let _ = writeln!(s, "why={}", one_line(why));
        }
        if self.left_out_more > 0 {
            let _ = writeln!(s, "more={}", self.left_out_more);
        }
        s
    }

    /// Parse one back. `None` when the text carries no `game_id`, which is the
    /// one field without which the archive cannot be identified as a backup at
    /// all - the same test `eidos-instance.ini` applies to itself.
    pub fn parse(text: &str) -> Option<BackupManifest> {
        let mut m = BackupManifest {
            schema_version: SCHEMA_VERSION,
            downloads: true,
            // Absent `kind` means portable, which is the cautious reading: it
            // makes unpack want a destination rather than assume the central
            // one and quietly write over an instance already there.
            portable: true,
            ..BackupManifest::default()
        };
        let mut have_game = false;
        let mut section = String::new();
        // `name=`/`exe=` and `path=`/`why=` come in pairs of lines rather than as
        // one `key=value`, because both halves are free text from a filesystem
        // and either could contain an `=`.
        let mut pending: Option<String> = None;
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
                continue;
            }
            if let Some(name) = eidos_ini::section_header(t) {
                section = name.to_ascii_lowercase();
                pending = None;
                continue;
            }
            let Some((k, v)) = eidos_ini::key_value(t) else {
                continue;
            };
            let v = v.trim();
            match section.as_str() {
                "backup" => match k {
                    "schema_version" => m.schema_version = v.parse().unwrap_or(SCHEMA_VERSION),
                    "eidos_version" => m.eidos_version = v.to_string(),
                    "created" => m.created = v.to_string(),
                    "game_id" => {
                        // This value BECOMES A PATH COMPONENT: with no folder
                        // given, `eidos unpack` restores a central instance to
                        // `$XDG_DATA_HOME/eidos/<game_id>`. A backup is a file
                        // handed between people, so a `game_id` of
                        // `../../.config/systemd/user` would choose its own
                        // destination. Refusing it here rather than at the one
                        // call site covers every future caller, and a manifest
                        // with no usable game id is already the case `parse`
                        // answers with `None` and `peek` refuses by name.
                        if is_one_plain_segment(v) {
                            m.game_id = v.to_string();
                            have_game = true;
                        }
                    }
                    "kind" => m.portable = !v.eq_ignore_ascii_case("global"),
                    "source_root" => m.source_root = v.to_string(),
                    "active_profile" => m.active_profile = v.to_string(),
                    "files" => m.files = v.parse().unwrap_or(0),
                    "directories" => m.directories = v.parse().unwrap_or(0),
                    "bytes" => m.bytes = v.parse().unwrap_or(0),
                    "downloads" => m.downloads = !v.eq_ignore_ascii_case("false"),
                    _ => {}
                },
                "profiles" if k == "profile" && !v.is_empty() => m.profiles.push(v.to_string()),
                "tools-outside" => match k {
                    "name" => pending = Some(v.to_string()),
                    "exe" => {
                        if let Some(title) = pending.take() {
                            m.tools_outside.push(Outside {
                                title,
                                exe: v.to_string(),
                            });
                        }
                    }
                    _ => {}
                },
                "left-out" => match k {
                    "path" => pending = Some(v.to_string()),
                    "why" => {
                        if let Some(path) = pending.take() {
                            m.left_out.push((path, v.to_string()));
                        }
                    }
                    "more" => m.left_out_more = v.parse().unwrap_or(0),
                    _ => {}
                },
                _ => {}
            }
        }
        have_game.then_some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BackupManifest {
        BackupManifest {
            schema_version: 1,
            eidos_version: "1.14.3".into(),
            created: "2026-09-07 15:04".into(),
            game_id: "skyrimse".into(),
            portable: true,
            source_root: "/mnt/Jeux/Eidos-Skyrim".into(),
            active_profile: "Default".into(),
            profiles: vec!["Default".into(), "Ultra".into()],
            files: 57555,
            directories: 6060,
            bytes: 77_538_508_832,
            downloads: false,
            tools_outside: vec![Outside {
                title: "DynDoLod".into(),
                exe: "/mnt/Jeux/Tools/DynDOLOD/DynDOLODx64.exe".into(),
            }],
            left_out: vec![("logs".into(), "specific to this machine".into())],
            left_out_more: 3,
        }
    }

    #[test]
    fn a_manifest_round_trips() {
        let m = sample();
        assert_eq!(BackupManifest::parse(&m.render()).unwrap(), m);
    }

    #[test]
    fn a_name_carrying_an_equals_sign_still_pairs_correctly() {
        // The reason name/exe are two lines rather than `name=exe`: both halves
        // are free text, and a tool called "SSEEdit = clean" is legal.
        let mut m = sample();
        m.tools_outside = vec![Outside {
            title: "SSEEdit = clean".into(),
            exe: "/opt/x=y/SSEEdit.exe".into(),
        }];
        m.left_out = vec![("mods/a=b/c".into(), "a symbolic link".into())];
        let back = BackupManifest::parse(&m.render()).unwrap();
        assert_eq!(back.tools_outside, m.tools_outside);
        assert_eq!(back.left_out, m.left_out);
    }

    #[test]
    fn a_newline_in_a_filename_cannot_tear_the_manifest_in_half() {
        let mut m = sample();
        m.left_out = vec![("mods/we\nird".into(), "a symbolic link".into())];
        let back = BackupManifest::parse(&m.render()).unwrap();
        assert_eq!(
            back.left_out,
            vec![("mods/we ird".into(), "a symbolic link".into())]
        );
        assert_eq!(back.game_id, "skyrimse", "the rest of the file still parsed");
    }

    #[test]
    fn a_game_id_that_could_choose_its_own_destination_is_refused() {
        // With no folder given, `eidos unpack` puts a central instance back at
        // `$XDG_DATA_HOME/eidos/<game_id>`. A .eidos file is passed between
        // people, so this value is untrusted input that names a directory.
        for hostile in [
            "../../../../home/somebody/.ssh",
            "/etc/systemd/system",
            "..",
            ".",
            "a/b",
            "a\\b",
            "C:",
            "with\u{7f}control",
            "",
        ] {
            let text = format!("[backup]\ngame_id={hostile}\n");
            assert!(
                BackupManifest::parse(&text).is_none(),
                "'{hostile}' should not be accepted as a game id"
            );
        }
        // And the real ones still are.
        for ok in ["skyrimse", "fallout4", "skyrimvr", "enderal-se"] {
            let text = format!("[backup]\ngame_id={ok}\n");
            assert_eq!(BackupManifest::parse(&text).unwrap().game_id, ok);
        }
    }

    #[test]
    fn text_without_a_game_id_is_not_a_backup_manifest() {
        assert!(BackupManifest::parse("[backup]\nschema_version=1\n").is_none());
        assert!(BackupManifest::parse("").is_none());
        // Neither is somebody else's INI that happens to be in an archive.
        assert!(BackupManifest::parse("[General]\nmodID=18780\n").is_none());
    }

    #[test]
    fn an_unknown_future_field_is_ignored_rather_than_fatal() {
        let text = "[backup]\ngame_id=fallout4\nsomething_new=42\n\n[whatever]\nx=y\n";
        let m = BackupManifest::parse(text).unwrap();
        assert_eq!(m.game_id, "fallout4");
        // Absent `kind` means portable, which is the safe default: it makes
        // unpack ask for a destination rather than assume the central one.
        assert!(m.portable);
    }
}
