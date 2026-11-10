//! A17: host limits and file systems that crash games under Proton.

use std::path::{Path, PathBuf};

use eidos_games::DetectedGame;
use eidos_instance::Instance;

use super::{Finding, Level};

/// The default on Arch Linux, Fedora 39+ and Ubuntu 24.04+.
pub const MIN_MAP_COUNT: u64 = 1_048_576;
/// The hard `NOFILE` limit systemd 240+ sets, and what Proton's esync asks for.
pub const MIN_OPEN_FILES: u64 = 524_288;
/// `ntfs` is the kernel's old driver, `fuseblk` covers ntfs-3g and exfat-fuse.
pub const WINDOWS_FS: [&str; 5] = ["ntfs", "ntfs3", "fuseblk", "exfat", "vfat"];

pub struct SystemFacts {
    pub max_map_count: Option<u64>,
    pub open_files_hard: Option<u64>,
    /// The game folder, the prefix and the instance root, each with its file system type.
    pub filesystems: Vec<(PathBuf, String)>,
}

pub fn read_max_map_count() -> Option<u64> {
    std::fs::read_to_string("/proc/sys/vm/max_map_count")
        .ok()?
        .trim()
        .parse()
        .ok()
}

pub fn read_open_files_hard() -> Option<u64> {
    open_files_hard(&std::fs::read_to_string("/proc/self/limits").ok()?)
}

/// The "Max open files" row's hard column; None for "unlimited" or an unreadable row.
pub fn open_files_hard(limits: &str) -> Option<u64> {
    let row = limits
        .lines()
        .find_map(|l| l.strip_prefix("Max open files"))?;
    row.split_whitespace().nth(1)?.parse().ok()
}

/// The fstype of the deepest mount in `mountinfo` whose mount point is a
/// component prefix of `path`; a later line wins a tie (an over-mount).
pub fn mount_fs_type(mountinfo: &str, path: &Path) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for line in mountinfo.lines() {
        // Fields before ` - ` hold no raw space: the kernel escapes it in the mount point.
        let Some((mount, fs)) = line.split_once(" - ") else {
            continue;
        };
        let (Some(point), Some(fs)) = (mount.split(' ').nth(4), fs.split(' ').next()) else {
            continue;
        };
        // `\134` last, so a decoded backslash never starts another escape.
        let point = point
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\");
        let point = Path::new(&point);
        let depth = point.components().count();
        if path.starts_with(point) && best.is_none_or(|(d, _)| depth >= d) {
            best = Some((depth, fs));
        }
    }
    best.map(|(_, fs)| fs.to_string())
}

/// Reads the three values; each path is canonicalized when it exists.
pub fn system_facts(game: &DetectedGame, inst: &Instance) -> SystemFacts {
    let mountinfo = std::fs::read("/proc/self/mountinfo").unwrap_or_default();
    let mountinfo = String::from_utf8_lossy(&mountinfo);
    let filesystems = [
        Some(game.install_path.clone()),
        game.prefix(),
        Some(inst.root.clone()),
    ]
    .into_iter()
    .flatten()
    .filter_map(|p| {
        let p = std::fs::canonicalize(&p).unwrap_or(p);
        let fs = mount_fs_type(&mountinfo, &p)?;
        Some((p, fs))
    })
    .collect();
    SystemFacts {
        max_map_count: read_max_map_count(),
        open_files_hard: read_open_files_hard(),
        filesystems,
    }
}

pub fn system_findings(f: &SystemFacts) -> Vec<Finding> {
    let advice = |code, value: u64, want: u64| Finding {
        code,
        level: Level::Advice,
        args: vec![("value", value.to_string()), ("want", want.to_string())],
    };
    let mut out = Vec::new();
    if let Some(v) = f.max_map_count.filter(|v| *v < MIN_MAP_COUNT) {
        out.push(advice("max_map_count_low", v, MIN_MAP_COUNT));
    }
    if let Some(v) = f.open_files_hard.filter(|v| *v < MIN_OPEN_FILES) {
        out.push(advice("open_files_low", v, MIN_OPEN_FILES));
    }
    let windows: Vec<String> = f
        .filesystems
        .iter()
        .filter(|(_, fs)| WINDOWS_FS.contains(&fs.as_str()))
        .map(|(p, fs)| format!("{} ({fs})", p.display()))
        .collect();
    if !windows.is_empty() {
        out.push(Finding {
            code: "windows_filesystem",
            level: Level::Advice,
            args: vec![("paths", windows.join("; "))],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // /mnt/Jeux2 comes before /mnt/Jeux at the same depth, so a string-prefix match
    // with the later-line tie rule would give /mnt/Jeux2 paths btrfs.
    const MOUNTS: &str = "22 1 259:2 / / rw,relatime shared:1 - ext4 /dev/nvme0n1p2 rw\n\
41 22 8:1 / /mnt/Jeux2 rw shared:21 - ntfs3 /dev/sda1 rw\n\
40 22 259:5 / /mnt/Jeux rw,relatime shared:20 - btrfs /dev/nvme1n1p1 rw\n\
42 22 8:2 / /run/media/lin/Win\\040Games rw,nosuid shared:22 - fuseblk /dev/sda2 rw,user_id=0\n\
43 40 8:17 / /mnt/Jeux/usb rw - exfat /dev/sdb1 rw\n\
44 43 8:18 / /mnt/Jeux/usb rw - vfat /dev/sdb2 rw\n\
45 22 8:33 / /mnt/back\\134040slash rw - xfs /dev/sdc1 rw\n";

    #[test]
    fn mount_fs_type_decodes_escapes_and_matches_whole_components() {
        let t = |p: &str| mount_fs_type(MOUNTS, Path::new(p));
        assert_eq!(
            t("/mnt/Jeux2/SteamLibrary/steamapps/compatdata/377160").as_deref(),
            Some("ntfs3")
        );
        assert_eq!(t("/mnt/Jeux/SteamLibrary").as_deref(), Some("btrfs"));
        assert_eq!(
            t("/run/media/lin/Win Games/SteamLibrary").as_deref(),
            Some("fuseblk")
        );
        // The over-mount on /mnt/Jeux/usb is the later line, so it wins the tie.
        assert_eq!(t("/mnt/Jeux/usb/Eidos").as_deref(), Some("vfat"));
        // `\134040` is a backslash then "040", never a second escape.
        assert_eq!(t("/mnt/back\\040slash/Games").as_deref(), Some("xfs"));
        assert_eq!(t("/mnt/back slash/Games").as_deref(), Some("ext4"));
        assert_eq!(t("/home/lin").as_deref(), Some("ext4"));
    }

    #[test]
    fn open_files_hard_reads_the_hard_column() {
        let limits = "Limit                     Soft Limit           Hard Limit           Units     \n\
                      Max open files            1024                 524288               files     \n";
        assert_eq!(open_files_hard(limits), Some(524288));
        assert_eq!(
            open_files_hard(
                "Max open files            1024                 unlimited            files\n"
            ),
            None
        );
    }

    #[test]
    fn low_limits_and_windows_file_systems_are_advice() {
        let f = system_findings(&SystemFacts {
            max_map_count: Some(65530),
            open_files_hard: Some(4096),
            filesystems: vec![
                (
                    "/mnt/Win/SteamLibrary/steamapps/common/Fallout 4".into(),
                    "ntfs3".into(),
                ),
                ("/home/lin/Eidos".into(), "ext4".into()),
            ],
        });
        assert_eq!(
            f.iter().map(|f| f.code).collect::<Vec<_>>(),
            ["max_map_count_low", "open_files_low", "windows_filesystem"]
        );
        assert!(f.iter().all(|f| f.level == Level::Advice));
        assert_eq!(
            (f[0].arg("value"), f[0].arg("want")),
            (Some("65530"), Some("1048576"))
        );
        assert_eq!(
            f[2].arg("paths"),
            Some("/mnt/Win/SteamLibrary/steamapps/common/Fallout 4 (ntfs3)")
        );
    }

    #[test]
    fn default_limits_linux_file_systems_and_unread_values_say_nothing() {
        assert!(system_findings(&SystemFacts {
            max_map_count: Some(1_048_576),
            open_files_hard: Some(524_288),
            filesystems: vec![("/x".into(), "btrfs".into())]
        })
        .is_empty());
        assert!(system_findings(&SystemFacts {
            max_map_count: None,
            open_files_hard: None,
            filesystems: vec![]
        })
        .is_empty());
    }
}
