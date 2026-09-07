//! What goes into a backup, what does not, and why.
//!
//! Everything under the instance root is packed except the items listed here,
//! and each exclusion is a claim that the destination is BETTER OFF without it -
//! not that it is merely big. A backup that quietly drops something the user
//! needed is worse than one that is a gigabyte larger, so every exclusion is
//! named in the manifest with its reason, and anything skipped for a reason the
//! rules did not anticipate (a symbolic link, a name that is not valid UTF-8) is
//! reported rather than silently left behind.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use eidos_instance::Instance;

use crate::{Options, MANIFEST_NAME};

/// Why something under the instance root is not in the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// `.base` / `.base-root`: empty mountpoints where the GAME's own files are
    /// bind-stashed during a session. Their contents belong to the game install,
    /// not to the instance, and on a machine where nothing is mounted they are
    /// empty anyway.
    Mountpoint,
    /// A record of THIS machine: session logs, the lock naming a live process.
    Local,
    /// A claim about the Proton prefix, which is not in the backup. Carrying
    /// `prereqs.done` to a machine with no prefix would tell Eidos the runtime
    /// libraries are already installed there, and the first tool launch would
    /// fail with a missing DLL instead of installing them.
    Prefix,
    /// A cache Eidos re-fetches on demand (the LOOT masterlist), which is also
    /// where the only symlinks in a typical instance live.
    Refetchable,
    /// Written fresh by this backup.
    Regenerated,
    /// A half-written file: an atomic write in flight, or a paused download.
    InFlight,
    /// Left out because the user asked (`--no-downloads`).
    ByRequest,
    /// A symbolic link. 7-Zip would FOLLOW it and copy whatever it points at,
    /// which for an absolute link means silently pulling a foreign tree into the
    /// backup; storing it as a link would carry a path that means nothing on the
    /// destination. Neither is what a backup should do without saying so.
    Symlink,
    /// A name that is not valid UTF-8, so it cannot be written into the list
    /// file 7-Zip reads.
    NotUtf8,
    /// The filesystem refused.
    Unreadable(String),
}

impl std::fmt::Display for Why {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Why::Mountpoint => f.write_str("a mountpoint for the game's own files"),
            Why::Local => f.write_str("specific to this machine"),
            Why::Prefix => f.write_str("describes a Proton prefix the backup does not carry"),
            Why::Refetchable => f.write_str("a cache Eidos re-fetches"),
            Why::Regenerated => f.write_str("written fresh by this backup"),
            Why::InFlight => f.write_str("half-written"),
            Why::ByRequest => f.write_str("left out on request"),
            Why::Symlink => f.write_str("a symbolic link"),
            Why::NotUtf8 => f.write_str("the name is not valid UTF-8"),
            Why::Unreadable(e) => write!(f, "unreadable: {e}"),
        }
    }
}

/// One thing that is not in the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Left {
    /// Instance-relative, as a person would type it.
    pub path: String,
    pub why: Why,
}

/// A tool named in `tools.ini` whose executable lives outside the instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outside {
    pub title: String,
    pub exe: String,
}

/// Everything a pack needs to know, computed before a single byte is written -
/// so `--dry-run` can print exactly what the real run would do.
#[derive(Debug, Clone)]
pub struct Plan {
    pub root: PathBuf,
    /// Instance-relative paths for the list file, sorted: every file, plus every
    /// EMPTY directory (a directory with content is implied by its files, an
    /// empty one is not, and 64 of them in a real 57 000-file instance is 64
    /// mods that would come back subtly different).
    pub entries: Vec<String>,
    /// `downloads/*.meta` files that carry a signed download URL. They are NOT
    /// in `entries`: a scrubbed copy is packed in their place.
    pub scrub: Vec<String>,
    pub files: u64,
    pub dirs: u64,
    pub empty_dirs: u64,
    pub bytes: u64,
    /// How much of `bytes` is `downloads/`, so a user weighing `--no-downloads`
    /// can see the number rather than guess it.
    pub downloads_bytes: u64,
    pub left: Vec<Left>,
    /// Entries whose name contains `*` or `?`. 7-Zip treats those as wildcards
    /// in a list file unless told otherwise; see the `-spd` handling in the pack.
    pub wildcards: Vec<String>,
    pub tools_outside: Vec<Outside>,
}

impl Plan {
    /// Everything that will be in the archive, the manifest aside.
    pub fn total_entries(&self) -> usize {
        self.entries.len() + self.scrub.len()
    }
}

/// The last component of a path as a `String` (`"downloads"`), or empty.
fn leaf(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The top-level names a backup leaves out, taken from the instance itself
/// wherever the layout exposes them, so a renamed folder cannot leave a rule
/// pointing at nothing.
fn root_rules(inst: &Instance, opt: &Options) -> Vec<(String, Why)> {
    let mut rules = vec![
        (leaf(&inst.base_dir()), Why::Mountpoint),
        (leaf(&inst.base_root_dir()), Why::Mountpoint),
        // Session logs of THIS machine, and the lock file naming a live process
        // on it. Both are written as `<root>/<name>` by the GUI and by the
        // instance lock; neither has an accessor to borrow.
        ("logs".to_string(), Why::Local),
        (".eidos.lock".to_string(), Why::Local),
        ("prereqs.done".to_string(), Why::Prefix),
        ("prereqs.log".to_string(), Why::Prefix),
        ("loot".to_string(), Why::Refetchable),
        (MANIFEST_NAME.to_string(), Why::Regenerated),
    ];
    if !opt.downloads {
        rules.push((leaf(&inst.downloads_dir()), Why::ByRequest));
    }
    rules.retain(|(n, _)| !n.is_empty());
    rules
}

/// The one file under `loot/` that is the user's own work rather than a cache:
/// LOOT's local rule overrides, which `eidos sort` reads and nothing re-fetches.
const USERLIST: &str = "userlist.yaml";

/// Whether a name is a file being written right now rather than one to keep.
///
/// Two shapes, because the workspace writes both: `write_atomic` and friends
/// use `<name>.eidos-tmp` or an `eidos-tmp.<pid>.<n>` extension, and an
/// interrupted or paused download is `<archive>.unfinished`.
pub(crate) fn is_in_flight(name: &str) -> bool {
    name.ends_with(".eidos-tmp") || name.contains(".eidos-tmp.") || name.ends_with(".unfinished")
}

/// Walk the instance and decide what goes in.
///
/// Infallible by construction: a directory that cannot be read is recorded in
/// `left` rather than aborting the plan, because one unreadable mod folder is
/// not a reason to refuse to back up the other two hundred - but it IS a reason
/// to say so before the user believes they have a complete copy.
pub fn plan(inst: &Instance, opt: &Options) -> Plan {
    let rules = root_rules(inst, opt);
    let mut out = Plan {
        root: inst.root.clone(),
        entries: Vec::new(),
        scrub: Vec::new(),
        files: 0,
        dirs: 0,
        empty_dirs: 0,
        bytes: 0,
        downloads_bytes: 0,
        left: Vec::new(),
        wildcards: Vec::new(),
        tools_outside: Vec::new(),
    };
    let downloads_prefix = format!("{}/", leaf(&inst.downloads_dir()));

    // Depth-first with an explicit stack: a mod archive can nest far enough that
    // recursion is a needless risk, and the order does not matter because the
    // entries are sorted at the end.
    let mut stack: Vec<(String, PathBuf)> = vec![(String::new(), inst.root.clone())];
    while let Some((rel, abs)) = stack.pop() {
        let reader = match fs::read_dir(&abs) {
            Ok(r) => r,
            Err(e) => {
                out.left.push(Left {
                    path: if rel.is_empty() { "." } else { &rel }.to_string(),
                    why: Why::Unreadable(e.to_string()),
                });
                // The directory itself still goes in. Its PARENT was marked as
                // contributing the moment this was pushed, so without this line
                // a mod folder whose only child cannot be read produces no
                // entries at all - and the whole ancestor chain vanishes from
                // the archive while `left` names only the deepest one. 7-Zip
                // stores a directory it cannot read and warns, which is the
                // honest outcome: the shape survives, the loss is reported.
                if !rel.is_empty() {
                    out.entries.push(rel);
                }
                continue;
            }
        };
        let at_root = rel.is_empty();
        // Whether anything under this directory will reach the archive. A
        // directory that contributes nothing has to be listed itself, or it
        // comes back missing.
        let mut contributes = false;
        for entry in reader {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    out.left.push(Left {
                        path: rel.clone(),
                        why: Why::Unreadable(e.to_string()),
                    });
                    continue;
                }
            };
            let raw = entry.file_name();
            let Some(name) = raw.to_str() else {
                out.left.push(Left {
                    path: format!("{rel}{}", raw.to_string_lossy()),
                    why: Why::NotUtf8,
                });
                continue;
            };
            let child = if at_root {
                name.to_string()
            } else {
                format!("{rel}/{name}")
            };
            if at_root {
                if let Some((_, why)) = rules.iter().find(|(n, _)| n.as_str() == name) {
                    // `loot/` is a masterlist cache Eidos re-fetches - except for
                    // `userlist.yaml`, which nobody re-fetches because the user
                    // WROTE it. Those are their own LOOT rules, and dropping
                    // them is dropping hand-made work, so the one file rides
                    // along while the cache around it does not.
                    if *why == Why::Refetchable && abs.join(name).join(USERLIST).is_file() {
                        out.files += 1;
                        contributes = true;
                        if let Ok(md) = fs::metadata(abs.join(name).join(USERLIST)) {
                            out.bytes += md.len();
                        }
                        out.entries.push(format!("{child}/{USERLIST}"));
                    }
                    out.left.push(Left {
                        path: child,
                        why: why.clone(),
                    });
                    continue;
                }
            }
            if is_in_flight(name) {
                out.left.push(Left {
                    path: child,
                    why: Why::InFlight,
                });
                continue;
            }
            let kind = match entry.file_type() {
                Ok(k) => k,
                Err(e) => {
                    out.left.push(Left {
                        path: child,
                        why: Why::Unreadable(e.to_string()),
                    });
                    continue;
                }
            };
            if kind.is_symlink() {
                out.left.push(Left {
                    path: child,
                    why: Why::Symlink,
                });
                continue;
            }
            if kind.is_dir() {
                out.dirs += 1;
                contributes = true;
                stack.push((child, entry.path()));
                continue;
            }
            // A regular file, or something exotic (a fifo, a socket) that 7-Zip
            // will refuse far more clearly than a guess here would.
            out.files += 1;
            contributes = true;
            if let Ok(md) = entry.metadata() {
                out.bytes += md.len();
                if child.starts_with(&downloads_prefix) {
                    out.downloads_bytes += md.len();
                }
            }
            out.entries.push(child);
        }
        if !contributes && !at_root {
            out.empty_dirs += 1;
            out.entries.push(rel);
        }
    }

    move_scrubbable_metas(&mut out, &downloads_prefix);
    out.tools_outside = tools_outside(inst);
    out.entries.sort();
    out.scrub.sort();
    // From the FINISHED list, not from each file as it is seen. An empty
    // directory is named in the list file exactly like a file is, so a mod
    // folder called `Weapons * Armour` with nothing in it needs `-spd` just as
    // much - and counting only files let it through the check that decides
    // whether packing without `-spd` is safe.
    out.wildcards = out
        .entries
        .iter()
        .chain(out.scrub.iter())
        .filter(|e| e.contains('*') || e.contains('?'))
        .cloned()
        .collect();
    out.wildcards.sort();
    out.left.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Move the `downloads/*.meta` files that carry a download URL out of the bulk
/// list and into `scrub`, so the pack can put a cleaned copy in their place.
///
/// The URL is a signed Nexus CDN link: it carries the account's `user_id`, an
/// `expires` stamp and a signature. It stops working within hours, so it is of
/// no use on the destination, and it identifies the person who downloaded the
/// file - which matters because a `.eidos` file is made to be handed to somebody
/// else. Everything else in the `.meta` (the mod id, the file id, the version)
/// is what makes the download re-findable, and stays.
fn move_scrubbable_metas(plan: &mut Plan, downloads_prefix: &str) {
    let root = plan.root.clone();
    let mut scrub = Vec::new();
    plan.entries.retain(|rel| {
        let is_meta = rel.starts_with(downloads_prefix) && rel.ends_with(".meta");
        if is_meta && has_url(&root.join(rel)) {
            scrub.push(rel.clone());
            return false;
        }
        true
    });
    plan.scrub = scrub;
}

/// Whether a `.meta` file must be scrubbed before it travels.
///
/// A file that cannot be read as text counts as YES, which routes it into
/// `scrub` - where the pack, unable to clean it either, leaves it out and says
/// so. The alternative reading of an unreadable `.meta` is "pack it verbatim",
/// and verbatim is exactly the signed URL, with the downloader's account id in
/// it, going to whoever the backup is handed to.
fn has_url(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return true;
    };
    text.lines().any(|l| is_url_line(l).is_some())
}

/// The value of a `url=` line, if that is what this line is. Split out so the
/// reader and the rewriter can never disagree about which lines are URLs.
pub(crate) fn is_url_line(line: &str) -> Option<&str> {
    let (k, v) = eidos_ini::key_value(line)?;
    if !k.eq_ignore_ascii_case("url") {
        return None;
    }
    let v = v.trim();
    let v = v.strip_prefix('"').unwrap_or(v);
    let v = v.strip_suffix('"').unwrap_or(v);
    (!v.is_empty()).then_some(v)
}

/// A `url=` line with its value emptied, or the line unchanged.
pub(crate) fn scrub_url_line(line: &str) -> String {
    match is_url_line(line) {
        Some(_) => "url=".to_string(),
        None => line.to_string(),
    }
}

/// `.meta` text with every download URL removed.
pub(crate) fn scrub_meta(text: &str) -> String {
    let nl = eidos_ini::newline_style(text);
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        out.push_str(&scrub_url_line(line));
        out.push_str(nl);
    }
    out
}

/// The tools whose executable is an absolute path outside the instance.
///
/// These are what a move actually loses: the instance comes back complete and
/// then a tool button points at `/mnt/Jeux/Tools/DynDOLOD/DynDOLODx64.exe` on a
/// machine that has never heard of it. Naming them in the manifest turns that
/// into a list the user can work through. A RELATIVE `exe` is not listed: it is
/// resolved against the game install, which the destination has by definition.
fn tools_outside(inst: &Instance) -> Vec<Outside> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for t in inst.tools() {
        if !t.exe.is_absolute() || t.exe.starts_with(&inst.root) {
            continue;
        }
        let exe = t.exe.to_string_lossy().into_owned();
        if seen.insert(exe.clone()) {
            out.push(Outside {
                title: t.title.clone(),
                exe,
            });
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "eidos-transfer-{}-{}-{name}",
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

    fn instance(root: &Path) -> Instance {
        fs::create_dir_all(root.join("mods")).unwrap();
        eidos_instance::Manifest::new("skyrimse", eidos_instance::InstanceKind::Portable)
            .write(&root.join("eidos-instance.ini"))
            .unwrap();
        Instance::portable(root.to_path_buf())
    }

    #[test]
    fn an_empty_directory_is_listed_and_a_full_one_is_not() {
        // The defect this exists for: a list file of FILES ONLY drops every empty
        // directory, and a real instance has dozens - each one a mod that comes
        // back missing a folder the game or a tool expects to find.
        let root = tmp("empty");
        let inst = instance(&root);
        fs::create_dir_all(root.join("mods/A/Textures")).unwrap();
        fs::create_dir_all(root.join("mods/B/Meshes")).unwrap();
        fs::write(root.join("mods/B/Meshes/x.nif"), b"x").unwrap();
        let p = plan(&inst, &Options::default());
        assert!(
            p.entries.contains(&"mods/A/Textures".to_string()),
            "{:?}",
            p.entries
        );
        assert!(
            !p.entries.contains(&"mods/B/Meshes".to_string()),
            "a directory with a file in it is implied by the file"
        );
        assert!(p.entries.contains(&"mods/B/Meshes/x.nif".to_string()));
        assert_eq!(p.empty_dirs, 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_directory_holding_only_excluded_things_still_comes_back() {
        // Its content is dropped on purpose; the folder itself is part of the
        // shape of the mod and has to survive.
        let root = tmp("onlyskipped");
        let inst = instance(&root);
        fs::create_dir_all(root.join("mods/A/Data")).unwrap();
        fs::write(root.join("mods/A/Data/big.7z.unfinished"), b"x").unwrap();
        let p = plan(&inst, &Options::default());
        assert!(p.entries.contains(&"mods/A/Data".to_string()));
        assert!(p.left.iter().any(|l| l.why == Why::InFlight));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_machines_own_records_are_left_out_with_a_reason() {
        let root = tmp("rules");
        let inst = instance(&root);
        for d in [".base", ".base-root", "logs", "loot"] {
            fs::create_dir_all(root.join(d)).unwrap();
            fs::write(root.join(d).join("f"), b"x").unwrap();
        }
        for f in [".eidos.lock", "prereqs.done", "prereqs.log", MANIFEST_NAME] {
            fs::write(root.join(f), b"x").unwrap();
        }
        let p = plan(&inst, &Options::default());
        for excluded in [
            ".base",
            ".base-root",
            "logs",
            "loot",
            ".eidos.lock",
            "prereqs.done",
            "prereqs.log",
            MANIFEST_NAME,
        ] {
            assert!(
                p.left.iter().any(|l| l.path == excluded),
                "{excluded} should be left out with a reason, got {:?}",
                p.left
            );
            assert!(
                !p.entries.iter().any(|e| e.starts_with(excluded)),
                "{excluded} leaked into the archive"
            );
        }
        // The manifest is regenerated, not carried: a stale one would describe
        // the wrong backup.
        assert!(p
            .left
            .iter()
            .any(|l| l.path == MANIFEST_NAME && l.why == Why::Regenerated));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_users_own_loot_rules_survive_the_cache_around_them() {
        let root = tmp("userlist");
        let inst = instance(&root);
        fs::create_dir_all(root.join("loot")).unwrap();
        fs::write(root.join("loot/masterlist.yaml"), b"cache").unwrap();
        fs::write(root.join("loot/userlist.yaml"), b"my rules").unwrap();
        let p = plan(&inst, &Options::default());
        assert!(
            p.entries.contains(&"loot/userlist.yaml".to_string()),
            "hand-written LOOT rules are not a cache: {:?}",
            p.entries
        );
        assert!(!p.entries.iter().any(|e| e.contains("masterlist")));
        // And an instance that has never sorted still just skips the folder.
        let bare = tmp("userlist-none");
        let inst2 = instance(&bare);
        fs::create_dir_all(bare.join("loot")).unwrap();
        fs::write(bare.join("loot/masterlist.yaml"), b"cache").unwrap();
        let p2 = plan(&inst2, &Options::default());
        assert!(!p2.entries.iter().any(|e| e.starts_with("loot")));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&bare);
    }

    #[test]
    fn an_empty_directory_named_like_a_pattern_still_counts_as_one() {
        // The `-spd` decision is made from this list. Counting only FILES let a
        // mod folder with nothing in it slip past, into an archive built in
        // pattern mode.
        let root = tmp("wildcard-dir");
        let inst = instance(&root);
        fs::create_dir_all(root.join("mods/Weapons * Armour/empty")).unwrap();
        let p = plan(&inst, &Options::default());
        assert!(
            p.wildcards
                .iter()
                .any(|w| w == "mods/Weapons * Armour/empty"),
            "{:?}",
            p.wildcards
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_directory_nobody_can_read_still_leaves_its_shape_behind() {
        // Its parent was marked as contributing the moment it was pushed, so
        // without an entry of its own the parent is not listed either and a
        // whole mod folder leaves the archive with only the deepest name
        // mentioned anywhere.
        let root = tmp("unreadable");
        let inst = instance(&root);
        fs::create_dir_all(root.join("mods/A/B")).unwrap();
        fs::write(root.join("mods/A/B/f.dds"), b"x").unwrap();
        let mut perms = fs::metadata(root.join("mods/A/B")).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o000);
        fs::set_permissions(root.join("mods/A/B"), perms).unwrap();

        let p = plan(&inst, &Options::default());
        assert!(
            p.entries.contains(&"mods/A/B".to_string()),
            "the folder itself must still be named: {:?}",
            p.entries
        );
        assert!(p
            .left
            .iter()
            .any(|l| l.path == "mods/A/B" && matches!(l.why, Why::Unreadable(_))));

        let mut perms = fs::metadata(root.join("mods/A/B")).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        let _ = fs::set_permissions(root.join("mods/A/B"), perms);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_download_record_that_cannot_be_read_does_not_travel_verbatim() {
        // Verbatim IS the signed URL with the downloader's account id in it.
        let root = tmp("badmeta");
        let inst = instance(&root);
        fs::create_dir_all(root.join("downloads")).unwrap();
        fs::write(root.join("downloads/Mod.7z.meta"), [0xffu8, 0xfe, 0xfd]).unwrap();
        let p = plan(&inst, &Options::default());
        assert_eq!(p.scrub, vec!["downloads/Mod.7z.meta".to_string()]);
        assert!(!p.entries.iter().any(|e| e.ends_with(".meta")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_symlink_is_never_followed_into_the_archive() {
        // 7-Zip would follow it and copy whatever it points at - for an absolute
        // link, a foreign tree pulled into a backup nobody asked to include.
        let root = tmp("symlink");
        let inst = instance(&root);
        std::os::unix::fs::symlink("/etc", root.join("mods/link")).unwrap();
        let p = plan(&inst, &Options::default());
        assert!(p
            .left
            .iter()
            .any(|l| l.path == "mods/link" && l.why == Why::Symlink));
        assert!(!p.entries.iter().any(|e| e.starts_with("mods/link")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn downloads_can_be_left_out_and_their_size_is_always_reported() {
        let root = tmp("downloads");
        let inst = instance(&root);
        fs::create_dir_all(root.join("downloads")).unwrap();
        fs::write(root.join("downloads/mod.7z"), vec![0u8; 4096]).unwrap();
        fs::write(root.join("mods/keep.txt"), vec![0u8; 16]).unwrap();

        let with = plan(&inst, &Options::default());
        assert_eq!(with.downloads_bytes, 4096);
        assert!(
            with.bytes >= 4096 + 16 && with.bytes > with.downloads_bytes,
            "the mods count towards the total too: {}",
            with.bytes
        );
        assert!(with.entries.contains(&"downloads/mod.7z".to_string()));

        let without = plan(
            &inst,
            &Options {
                downloads: false,
                ..Options::default()
            },
        );
        assert!(!without.entries.iter().any(|e| e.starts_with("downloads")));
        assert!(without
            .left
            .iter()
            .any(|l| l.path == "downloads" && l.why == Why::ByRequest));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_signed_download_url_is_replaced_by_a_scrubbed_copy() {
        // The URL carries the downloader's user_id and a signature, and it is a
        // backup made to be handed to somebody else.
        let root = tmp("meta");
        let inst = instance(&root);
        fs::create_dir_all(root.join("downloads")).unwrap();
        fs::write(
            root.join("downloads/Mod.7z.meta"),
            "[General]\r\nmodID=18780\r\nurl=\"https://cdn/x?user_id=42&h=sig\"\r\n",
        )
        .unwrap();
        fs::write(root.join("downloads/Clean.7z.meta"), "[General]\nurl=\n").unwrap();
        let p = plan(&inst, &Options::default());
        assert_eq!(p.scrub, vec!["downloads/Mod.7z.meta".to_string()]);
        assert!(!p.entries.contains(&"downloads/Mod.7z.meta".to_string()));
        // One with nothing to hide stays in the bulk pass.
        assert!(p.entries.contains(&"downloads/Clean.7z.meta".to_string()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scrubbing_empties_the_url_and_keeps_everything_else_byte_for_byte() {
        let text = "[General]\r\nmodID=18780\r\nurl=\"https://cdn/x?user_id=42\"\r\nversion=2.5\r\n";
        let out = scrub_meta(text);
        assert!(out.contains("modID=18780"), "{out}");
        assert!(out.contains("version=2.5"), "{out}");
        assert!(out.contains("url="), "{out}");
        assert!(!out.contains("user_id"), "{out}");
        // CRLF in, CRLF out: these files round-trip through MO2 too.
        assert!(out.contains("\r\n"), "{out:?}");
        // A file with nothing to scrub is unchanged apart from its terminator.
        assert_eq!(scrub_meta("[General]\nurl=\n"), "[General]\nurl=\n");
    }

    #[test]
    fn in_flight_names_are_both_shapes_the_workspace_writes() {
        assert!(is_in_flight("plugins.eidos-tmp"));
        assert!(is_in_flight("nexus.ini.eidos-tmp.1234.7"));
        assert!(is_in_flight("Mod-1.0.7z.unfinished"));
        assert!(!is_in_flight("Mod-1.0.7z"));
        assert!(!is_in_flight("eidos-tmp-notes.txt"));
    }

    #[test]
    fn a_tool_inside_the_instance_is_not_something_the_user_must_reinstall() {
        let root = tmp("tools");
        let inst = instance(&root);
        let ini = format!(
            "[Tool/Inside]\nexe={}/mods/BodySlide/BodySlide.exe\n\n\
             [Tool/Outside]\nexe=/opt/xedit/SSEEdit.exe\n\n\
             [Tool/Relative]\nexe=Data/Tool.exe\n",
            root.display()
        );
        fs::write(root.join("tools.ini"), ini).unwrap();
        let p = plan(&inst, &Options::default());
        let titles: Vec<&str> = p.tools_outside.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["Outside"], "{:?}", p.tools_outside);
        let _ = fs::remove_dir_all(&root);
    }
}
