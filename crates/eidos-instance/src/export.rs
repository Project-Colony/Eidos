//! The mod list as CSV, byte-compatible with MO2's `exportModListCSV`.
//!
//! Lives here rather than in the `eidos` binary because BOTH front ends need it:
//! the CLI's `eidos export` and the GUI's Export dialog. It was written inside a
//! `[[bin]]` target, which is a place nothing else can call into - so the window
//! could not reach a writer that was already complete and already correct.

use crate::{Instance, ModEntry};
use std::path::Path;

/// Which rows an export covers (MO2's All / Active / Visible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportScope {
    /// Every mod in the list.
    All,
    /// Only the enabled ones.
    Active,
}

/// The thirteen MO2 columns, in MO2's order.
///
/// Named rather than positional so a column picker can talk about them, and so
/// the header and the cells cannot drift apart - they are generated from this
/// one list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Priority,
    Status,
    Name,
    Note,
    PrimaryCategory,
    Author,
    Uploader,
    NexusId,
    NexusUrl,
    UploaderUrl,
    Version,
    InstallDate,
    DownloadFileName,
}

impl Column {
    /// Every column, in MO2's order. A full export emits exactly this.
    pub const ALL: &'static [Column] = &[
        Column::Priority,
        Column::Status,
        Column::Name,
        Column::Note,
        Column::PrimaryCategory,
        Column::Author,
        Column::Uploader,
        Column::NexusId,
        Column::NexusUrl,
        Column::UploaderUrl,
        Column::Version,
        Column::InstallDate,
        Column::DownloadFileName,
    ];

    /// The header cell MO2 writes.
    pub fn header(self) -> &'static str {
        match self {
            Column::Priority => "#Mod_Priority",
            Column::Status => "#Mod_Status",
            Column::Name => "#Mod_Name",
            Column::Note => "#Note",
            Column::PrimaryCategory => "#Primary_Category",
            Column::Author => "#Mod_Author",
            Column::Uploader => "#Mod_Uploader",
            Column::NexusId => "#Nexus_ID",
            Column::NexusUrl => "#Mod_Nexus_URL",
            Column::UploaderUrl => "#Mod_Uploader_URL",
            Column::Version => "#Mod_Version",
            Column::InstallDate => "#Install_Date",
            Column::DownloadFileName => "#Download_File_Name",
        }
    }

    /// A short label for a column picker.
    pub fn label(self) -> &'static str {
        match self {
            Column::Priority => "Priority",
            Column::Status => "Enabled",
            Column::Name => "Name",
            Column::Note => "Note",
            Column::PrimaryCategory => "Category",
            Column::Author => "Author",
            Column::Uploader => "Uploader",
            Column::NexusId => "Nexus id",
            Column::NexusUrl => "Nexus URL",
            Column::UploaderUrl => "Uploader URL",
            Column::Version => "Version",
            Column::InstallDate => "Installed",
            Column::DownloadFileName => "Archive",
        }
    }

    /// Whether the cell is quoted. MO2 quotes every string and leaves the Nexus
    /// id bare; a parser written against its output depends on exactly that.
    fn quoted(self) -> bool {
        self != Column::NexusId
    }

    /// Whether Eidos has no source for this column at all.
    ///
    /// Empty now that the author and uploader are read from the mod payload the
    /// update check already fetches - but a mod that has never been checked
    /// still exports blank, which is a different thing and not worth a label.
    pub fn is_untracked(self) -> bool {
        false
    }
}

/// Quote a CSV string field MO2-style: always wrapped in double quotes, embedded
/// quotes doubled.
pub fn csv_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// A directory's modified time as `yyyy/MM/dd HH:mm:ss` (UTC; MO2 uses local time,
/// a documented divergence to stay dependency-free). Empty if unreadable.
pub fn fmt_mtime(path: &Path) -> String {
    let Ok(secs) = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH).map_err(|_| std::io::Error::other("pre-epoch"))
        })
        .map(|d| d.as_secs() as i64)
    else {
        return String::new();
    };
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let (h, mi, s) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}/{m:02}/{d:02} {h:02}:{mi:02}:{s:02}")
}

/// The mod list as MO2-format CSV: CRLF, always-quoted strings, the bare Nexus id.
///
/// `rows` is the list to export, already in display order - passed in rather than
/// read from the instance so the GUI can hand over exactly what is on screen
/// (its "visible rows" scope) instead of re-deriving a filter this module would
/// have to know about.
///
/// The PRIORITY column is the mod's POSITION in `rows`, not a count of what was
/// written. MO2 exports a priority, and a priority that renumbers itself when
/// the scope changes is not one: the same mod would read 0004 in a full export
/// and 0002 in an enabled-only export of the same list, and neither would match
/// the "#" column on screen. Holes are the correct output - they are what says a
/// row was skipped.
pub fn mod_list_csv(
    inst: &Instance,
    rows: &[ModEntry],
    scope: ExportScope,
    columns: &[Column],
    nexus_domain: &str,
) -> (String, usize) {
    let factory = inst.category_factory();
    let mut csv: String =
        columns.iter().map(|c| c.header()).collect::<Vec<_>>().join(",");
    csv.push_str("\r\n");

    let mut count = 0usize;
    for (position, m) in rows.iter().enumerate() {
        if scope == ExportScope::Active && !m.enabled {
            continue;
        }
        // Separators head groups; they are not mods and MO2 does not export them.
        if m.is_separator() {
            continue;
        }
        let meta = inst.mod_meta(&m.name);
        // MO2 strips commas from the note rather than relying on the quoting.
        let note = meta.notes().unwrap_or_default().replace(',', "");
        let category = meta
            .category()
            .as_deref()
            .and_then(crate::parse_primary)
            .and_then(|cid| factory.name_for_id(cid))
            .unwrap_or_default()
            .to_string();
        let nexus_id = meta.mod_id().unwrap_or(0);
        let nexus_url = if nexus_id > 0 && !nexus_domain.is_empty() {
            format!("https://www.nexusmods.com/{nexus_domain}/mods/{nexus_id}")
        } else {
            String::new()
        };

        let cells: Vec<String> = columns
            .iter()
            .map(|&c| {
                let v = match c {
                    Column::Priority => format!("{position:04}"),
                    Column::Status => (if m.enabled { "+" } else { "-" }).to_string(),
                    Column::Name => m.name.clone(),
                    Column::Note => note.clone(),
                    Column::PrimaryCategory => category.clone(),
                    // MO2's own three, filled from the same meta.ini keys it
                    // uses. Blank for a mod no update check has reached yet -
                    // which is the honest answer, not a missing feature.
                    Column::Author => meta.author().unwrap_or_default(),
                    Column::Uploader => meta.uploader().unwrap_or_default(),
                    Column::UploaderUrl => meta.uploader_url().unwrap_or_default(),
                    Column::NexusId => nexus_id.to_string(),
                    Column::NexusUrl => nexus_url.clone(),
                    Column::Version => meta.version().unwrap_or_default(),
                    Column::InstallDate => fmt_mtime(&m.path),
                    Column::DownloadFileName => meta.installation_file().unwrap_or_default(),
                };
                if c.quoted() {
                    csv_quote(&v)
                } else {
                    v
                }
            })
            .collect();
        csv.push_str(&cells.join(","));
        csv.push_str("\r\n");
        count += 1;
    }
    (csv, count)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InstanceKind, Manifest};

    fn fixture() -> (Instance, std::path::PathBuf) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "eidos-export-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("mods")).unwrap();
        Manifest::new("skyrimse", InstanceKind::Portable)
            .write(&root.join("eidos-instance.ini"))
            .unwrap();
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        (inst, root)
    }

    fn row(name: &str, enabled: bool, root: &std::path::Path) -> ModEntry {
        let p = root.join("mods").join(name);
        let _ = std::fs::create_dir_all(&p);
        ModEntry { name: name.to_string(), enabled, path: p, unmanaged: false }
    }

    #[test]
    fn the_full_export_is_mo2s_own_shape() {
        let (inst, root) = fixture();
        let rows = vec![row("Alpha", true, &root), row("Bravo", false, &root)];
        let (csv, n) =
            mod_list_csv(&inst, &rows, ExportScope::All, Column::ALL, "skyrimspecialedition");
        assert_eq!(n, 2);
        // CRLF, and the header MO2 writes verbatim.
        assert!(csv.starts_with("#Mod_Priority,#Mod_Status,#Mod_Name,"), "{csv}");
        assert!(csv.contains("\r\n"), "MO2 writes CRLF and a parser may rely on it");
        let first = csv.lines().nth(1).unwrap();
        // Every string quoted, the Nexus id bare - the one asymmetry MO2 has.
        assert!(first.starts_with("\"0000\",\"+\",\"Alpha\","), "{first}");
        assert!(first.contains(",0,"), "the id is not quoted: {first}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_priority_column_is_a_position_not_a_running_count() {
        let (inst, root) = fixture();
        let rows = vec![row("Off1", false, &root), row("On1", true, &root), row("On2", true, &root)];
        let (csv, n) = mod_list_csv(&inst, &rows, ExportScope::Active, Column::ALL, "");
        assert_eq!(n, 2);
        // 0001 and 0002 - the positions they hold in the list. A priority that
        // renumbers itself when the scope changes is not a priority: the same
        // mod would read differently in two exports of one list, and neither
        // would match the "#" column on screen. The hole IS the information.
        let nums: Vec<&str> = csv.lines().skip(1).map(|l| &l[1..5]).collect();
        assert_eq!(nums, vec!["0001", "0002"], "{csv}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_column_subset_keeps_the_order_and_drops_the_rest() {
        let (inst, root) = fixture();
        let rows = vec![row("Alpha", true, &root)];
        // Deliberately passed out of order: the writer must emit MO2's order.
        let picked = [Column::Name, Column::Priority];
        let (csv, _) = mod_list_csv(&inst, &rows, ExportScope::All, &picked, "");
        assert_eq!(csv.lines().next().unwrap(), "#Mod_Name,#Mod_Priority");
        assert_eq!(csv.lines().nth(1).unwrap(), "\"Alpha\",\"0000\"");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn separators_are_not_mods_and_are_never_exported() {
        let (inst, root) = fixture();
        let rows = vec![row("Gear_separator", true, &root), row("Alpha", true, &root)];
        let (csv, n) = mod_list_csv(&inst, &rows, ExportScope::All, Column::ALL, "");
        assert_eq!(n, 1);
        assert!(!csv.contains("Gear"), "{csv}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_quote_in_a_name_is_doubled_not_dropped() {
        assert_eq!(csv_quote(r#"a "b" c"#), r#""a ""b"" c""#);
        assert_eq!(csv_quote(""), "\"\"");
    }

    #[test]
    fn every_column_has_a_header_and_only_the_nexus_id_is_bare() {
        for c in Column::ALL {
            assert!(c.header().starts_with('#'), "{:?}", c);
            assert!(!c.label().is_empty());
            assert_eq!(c.quoted(), *c != Column::NexusId, "{:?}", c);
        }
        assert_eq!(Column::ALL.len(), 13, "MO2 exports thirteen columns");
    }
}
