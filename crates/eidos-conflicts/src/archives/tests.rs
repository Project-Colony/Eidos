use super::*;
use std::io::Cursor;
fn put32(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes())
}
fn put64(b: &mut [u8], at: usize, n: u64) {
    b[at..at + 8].copy_from_slice(&n.to_le_bytes())
}
fn bsa_fixture(version: u32, member: &str) -> Vec<u8> {
    let (folder, name) = member.rsplit_once('/').unwrap();
    let rec = if version == 105 { 24 } else { 16 };
    let start = 36 + rec;
    let names = start + 1 + folder.len() + 1 + 16;
    let payload = names + name.len() + 1;
    let mut b = vec![0; payload + 1];
    b[..4].copy_from_slice(b"BSA\0");
    for (at, n) in [
        (4, version),
        (8, 36),
        (12, 3),
        (16, 1),
        (20, 1),
        (24, folder.len() as u32 + 1),
        (28, name.len() as u32 + 1),
        (44, 1),
    ] {
        put32(&mut b, at, n)
    }
    let offset = start + name.len() + 1;
    if version == 105 {
        put64(&mut b, 52, offset as u64)
    } else {
        put32(&mut b, 48, offset as u32)
    }
    b[start] = (folder.len() + 1) as u8;
    b[start + 1..start + 1 + folder.len()].copy_from_slice(folder.as_bytes());
    let file_rec = start + folder.len() + 2;
    put32(&mut b, file_rec + 8, 1);
    put32(&mut b, file_rec + 12, payload as u32);
    b[names..names + name.len()].copy_from_slice(name.as_bytes());
    b
}
fn ba2_fixture(version: u32, texture: bool, member: &str) -> Vec<u8> {
    let header = match version {
        2 => 32,
        3 => 36,
        _ => 24,
    };
    let records = if texture { 48 } else { 36 };
    let payload = header + records;
    let names = payload + 1;
    let mut b = vec![0; names + 2 + member.len()];
    b[..4].copy_from_slice(b"BTDX");
    put32(&mut b, 4, version);
    b[8..12].copy_from_slice(if texture { b"DX10" } else { b"GNRL" });
    put32(&mut b, 12, 1);
    put64(&mut b, 16, names as u64);
    if texture {
        b[header + 13] = 1;
        b[header + 14] = 24;
        put64(&mut b, header + 24, payload as u64);
        put32(&mut b, header + 36, 1)
    } else {
        put64(&mut b, header + 16, payload as u64);
        put32(&mut b, header + 28, 1)
    }
    b[names..names + 2].copy_from_slice(&(member.len() as u16).to_le_bytes());
    b[names + 2..].copy_from_slice(member.as_bytes());
    b
}
#[test]
fn reads_bsa_generations_and_ba2_variants() {
    for version in [103, 104, 105] {
        assert_eq!(
            read_members(Cursor::new(bsa_fixture(version, "Textures/A.dds"))).unwrap(),
            ["Textures/A.dds"]
        )
    }
    for version in [1, 2, 3, 7, 8] {
        for texture in [false, true] {
            assert_eq!(
                read_members(Cursor::new(ba2_fixture(
                    version,
                    texture,
                    "Textures\\A.dds"
                )))
                .unwrap(),
                ["Textures/A.dds"]
            )
        }
    }
}
#[test]
fn reads_tes3_directory() {
    let name = b"textures\\a.dds\0";
    let hash = 24 + name.len();
    let payload = hash + 8;
    let mut b = vec![0; payload + 1];
    put32(&mut b, 0, 0x100);
    put32(&mut b, 4, (hash - 12) as u32);
    put32(&mut b, 8, 1);
    put32(&mut b, 12, 1);
    b[24..hash].copy_from_slice(name);
    assert_eq!(read_members(Cursor::new(b)).unwrap(), ["textures/a.dds"]);
}
#[test]
fn rejects_malformed_offsets_lengths_counts_and_versions() {
    let base = bsa_fixture(105, "textures/a.dds");
    for (at, n) in [
        (4, 83),
        (16, u32::MAX),
        (20, 2),
        (24, 0),
        (28, u32::MAX),
        (52, 0),
        (start_file_record(&base) + 12, 0),
    ] {
        let mut b = base.clone();
        put32(&mut b, at, n);
        assert!(read_members(Cursor::new(b)).is_err(), "offset {at}")
    }
    for end in 0..base.len() - 1 {
        assert!(
            read_members(Cursor::new(&base[..end])).is_err(),
            "truncation {end}"
        )
    }
    for texture in [false, true] {
        let base = ba2_fixture(3, texture, "textures/a.dds");
        for (at, n) in [(4, 9), (12, u32::MAX), (16, 0), (16, u32::MAX)] {
            let mut b = base.clone();
            put32(&mut b, at, n);
            assert!(read_members(Cursor::new(b)).is_err())
        }
    }
    for path in ["../a.dds", "/a.dds", "C:/a.dds", "textures//a.dds", "a\0b"] {
        assert!(read_members(Cursor::new(ba2_fixture(1, false, path))).is_err())
    }
}
fn start_file_record(b: &[u8]) -> usize {
    60 + 1 + b[60] as usize
}
#[test]
fn actual_archive_override_precedes_member_order_and_loose_wins() {
    let root = std::env::temp_dir().join(format!("eidos-archive-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let a = root.join("a");
    let b = root.join("b");
    std::fs::create_dir_all(a.join("textures")).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("A.bsa"), bsa_fixture(105, "Textures/shared.dds")).unwrap();
    std::fs::write(b.join("a.BSA"), bsa_fixture(105, "textures/shadowed.dds")).unwrap();
    std::fs::write(b.join("B.bsa"), bsa_fixture(105, "textures/shared.dds")).unwrap();
    std::fs::write(a.join("textures/shared.dds"), b"loose").unwrap();
    let parts = vec![
        (
            Layer {
                origin: 1,
                name: "a".into(),
                root: a.clone(),
            },
            crate::collect_files(&a),
        ),
        (
            Layer {
                origin: 2,
                name: "b".into(),
                root: b.clone(),
            },
            crate::collect_files(&b),
        ),
    ];
    let active = vec![
        ActiveArchive {
            order_uncertain: false,
            name: "A.bsa".into(),
            plugin: Some("A.esp".into()),
        },
        ActiveArchive {
            order_uncertain: false,
            name: "B.bsa".into(),
            plugin: Some("B.esp".into()),
        },
    ];
    let map = ConflictMap::build_with_archives_from(&parts, &active);
    assert!(map.archive_diagnostics.is_empty());
    assert!(!map.asset_files.contains_key("textures/shadowed.dds"));
    let n = &map.asset_files["textures/shared.dds"];
    assert_eq!(n.winner.origin, 1);
    assert!(n.winner.archive.is_none());
    assert_eq!(
        n.alternatives.iter().map(|p| p.origin).collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(n.alternatives[0].plugin.as_deref(), Some("B.esp"));
    assert!(!map.asset_mods[&1].overwrites.contains(&1));
    assert!(map.asset_mods[&1].overwrites.contains(&2));
    let mut parts = parts;
    parts[0].1.0.retain(|p| p != "textures/shared.dds");
    let map = ConflictMap::build_with_archives_from(&parts, &active);
    assert_eq!(map.asset_files["textures/shared.dds"].winner.origin, 2);
    let map = ConflictMap::build_with_archives_from(&parts, &active[..1]);
    assert_eq!(map.asset_files["textures/shared.dds"].winner.origin, 1);
    assert!(
        map.asset_files["textures/shared.dds"]
            .alternatives
            .is_empty()
    );
    std::fs::write(a.join("A.bsa"), b"broken").unwrap();
    let map = ConflictMap::build_with_archives_from(&parts, &active);
    assert_eq!(map.archive_diagnostics.len(), 1);
    assert_eq!(map.archive_diagnostics[0].origin, Some(1));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn bsa_name_table_zero_padding_is_valid_but_extra_names_are_not() {
    let mut b = bsa_fixture(105, "textures/a.dds");
    let old = b.len() - 1;
    let file_rec = start_file_record(&b);
    let name_len = u32_at(&b, 28) as u32;
    b.splice(old..old, std::iter::repeat_n(0, 9));
    put32(&mut b, 28, name_len + 9);
    let folder = u64_at(&b, 52);
    put64(&mut b, 52, folder + 9);
    put32(&mut b, file_rec + 12, (old + 9) as u32);
    assert_eq!(read_members(Cursor::new(&b)).unwrap(), ["textures/a.dds"]);
    b[old] = b'x';
    assert!(read_members(Cursor::new(&b)).is_err());
}
#[test]
fn directory_reads_never_touch_member_payload_and_decode_windows_names() {
    struct Guard {
        c: Cursor<Vec<u8>>,
        start: u64,
        end: u64,
    }
    impl Read for Guard {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let p = self.c.position();
            assert!(
                p + out.len() as u64 <= self.start || p >= self.end,
                "payload read"
            );
            self.c.read(out)
        }
    }
    impl Seek for Guard {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            self.c.seek(p)
        }
    }
    let b = ba2_fixture(1, false, "a.dds");
    let end = u64_at(&b, 16);
    let guarded = Guard {
        c: Cursor::new(b),
        start: end - 1,
        end,
    };
    assert_eq!(read_members(guarded).unwrap(), ["a.dds"]);
    assert_eq!(path(b"textures/\x80.dds").unwrap(), "textures/€.dds");
}
#[test]
fn malformed_ba2_chunks_and_payload_offsets_are_rejected() {
    let base = ba2_fixture(8, true, "a.dds");
    for at in [24 + 13, 24 + 14] {
        let mut b = base.clone();
        b[at] = 0;
        assert!(read_members(Cursor::new(b)).is_err())
    }
    for n in [0, u64::MAX] {
        let mut b = base.clone();
        put64(&mut b, 48, n);
        assert!(read_members(Cursor::new(b)).is_err())
    }
    let mut b = base;
    let names = u64_at(&b, 16) as usize;
    b[names..names + 2].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(read_members(Cursor::new(b)).is_err());
}
#[test]
fn uncertain_ties_keep_sources_but_do_not_fabricate_directional_flags() {
    let root = std::env::temp_dir().join(format!("eidos-archive-tie-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let files = ["A - Main.ba2", "A - Extra.ba2"];
    for file in files {
        std::fs::write(root.join(file), ba2_fixture(1, false, "x.dds")).unwrap()
    }
    let parts = vec![(
        Layer {
            origin: 1,
            name: "A".into(),
            root: root.clone(),
        },
        crate::collect_files(&root),
    )];
    let active: Vec<_> = files
        .iter()
        .map(|name| ActiveArchive {
            name: name.to_string(),
            plugin: Some("A.esp".into()),
            order_uncertain: true,
        })
        .collect();
    let map = ConflictMap::build_with_archives_from(&parts, &active);
    assert!(map.asset_files["x.dds"].precedence_uncertain);
    assert_eq!(map.asset_files["x.dds"].alternatives.len(), 1);
    assert_eq!(map.state(1), ConflictState::None);
    assert_eq!(map.asset_conflicts[&1], ["x.dds"]);
    std::fs::write(root.join("B.ba2"), ba2_fixture(1, false, "x.dds")).unwrap();
    let mut parts = parts;
    parts[0].1 = crate::collect_files(&root);
    let mut active = active;
    active.push(ActiveArchive {
        name: "B.ba2".into(),
        plugin: Some("B.esp".into()),
        order_uncertain: false,
    });
    let map = ConflictMap::build_with_archives_from(&parts, &active);
    assert!(
        !map.asset_files["x.dds"].precedence_uncertain,
        "a lower tie does not change the known winner"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_archive_scans_cannot_prove_redundancy_or_a_lower_archive_winner() {
    let root = std::env::temp_dir().join(format!("eidos-archive-unknown-{}", std::process::id()));
    let low = root.join("Low");
    let high = root.join("High");
    std::fs::create_dir_all(&low).unwrap();
    std::fs::create_dir_all(&high).unwrap();
    std::fs::write(low.join("Broken.bsa"), b"unsupported archive").unwrap();
    std::fs::write(low.join("shared.txt"), b"lower loose file").unwrap();
    std::fs::write(high.join("shared.txt"), b"winning loose file").unwrap();
    std::fs::write(high.join("Known.bsa"), bsa_fixture(105, "textures/a.dds")).unwrap();
    let parts: Vec<_> = [(2, "High", &high), (1, "Low", &low)]
        .into_iter()
        .map(|(origin, name, root)| {
            (
                Layer {
                    origin,
                    name: name.into(),
                    root: root.clone(),
                },
                crate::collect_files(root),
            )
        })
        .collect();
    let mut active = vec![
        ActiveArchive {
            name: "Known.bsa".into(),
            plugin: Some("Known.esp".into()),
            order_uncertain: false,
        },
        ActiveArchive {
            name: "Broken.bsa".into(),
            plugin: Some("Broken.esp".into()),
            order_uncertain: false,
        },
    ];
    let map = ConflictMap::build_with_archives_from(&parts, &active);
    assert_eq!(map.archive_diagnostics.len(), 1);
    assert_ne!(
        map.state(1),
        ConflictState::Redundant,
        "unreadable archived assets might still win"
    );
    assert!(
        map.asset_files["textures/a.dds"].precedence_uncertain,
        "a higher unreadable archive may override this asset"
    );
    assert!(
        !map.asset_files["shared.txt"].precedence_uncertain,
        "loose files still win over unreadable archives"
    );
    active.reverse();
    let map = ConflictMap::build_with_archives_from(&parts, &active);
    assert!(
        !map.asset_files["textures/a.dds"].precedence_uncertain,
        "an unreadable lower archive cannot change a higher known winner"
    );
    std::fs::remove_dir_all(root).unwrap();
}

include!("payload_tests.rs");
