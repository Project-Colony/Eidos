//! Uncompressed TES4 BSA writer for the fixed Horse Armor recipe.
//! Folder/file records are sorted by the Bethesda name hash and carry bounded offsets.
use super::*;

fn hash(name: &str, extension: &str) -> u64 {
    let name = name.to_ascii_lowercase();
    let extension = extension.to_ascii_lowercase();
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return 0;
    }
    let mut low = u32::from(*bytes.last().unwrap())
        | (u32::from(if bytes.len() < 3 {
            0
        } else {
            bytes[bytes.len() - 2]
        }) << 8)
        | ((bytes.len() as u32) << 16)
        | (u32::from(bytes[0]) << 24);
    low |= match extension.as_str() {
        ".kf" => 0x80,
        ".nif" => 0x8000,
        ".dds" => 0x8080,
        ".wav" => 0x80000000,
        _ => 0,
    };
    let mut high = 0u32;
    for b in bytes.iter().skip(1).take(bytes.len().saturating_sub(3)) {
        high = high.wrapping_mul(0x1003f).wrapping_add(u32::from(*b));
    }
    let ext = extension.bytes().fold(0u32, |v, b| {
        v.wrapping_mul(0x1003f).wrapping_add(u32::from(b))
    });
    (u64::from(high.wrapping_add(ext)) << 32) | u64::from(low)
}
fn file_hash(name: &str) -> u64 {
    let (stem, ext) = name
        .rsplit_once('.')
        .map(|(a, b)| (a, format!(".{b}")))
        .unwrap_or((name, String::new()));
    hash(stem, &ext)
}
fn word(out: &mut Vec<u8>, v: usize) -> E<()> {
    out.extend_from_slice(
        &u32::try_from(v)
            .map_err(|_| error("BSA offset overflow"))?
            .to_le_bytes(),
    );
    Ok(())
}

pub(super) fn generate(tree: &BTreeMap<String, Vec<u8>>, cancel: &AtomicBool) -> E<Vec<u8>> {
    if tree.is_empty() || tree.len() > 100_000 {
        return Err(error("invalid BSA member count"));
    }
    let mut groups: BTreeMap<String, Vec<(String, &[u8])>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut total = 0usize;
    let mut file_names = 0usize;
    let mut file_flags = 0usize;
    for (path, data) in tree {
        check_cancel(cancel)?;
        let normalized = checked_path(path)?.to_ascii_lowercase();
        if !normalized.is_ascii() || normalized.len() > 255 || !seen.insert(normalized.clone()) {
            return Err(error("BSA requires unique bounded ASCII member paths"));
        }
        if data.len() > MAX_MEMBER as usize {
            return Err(error("BSA member exceeds bound"));
        }
        total = total
            .checked_add(data.len())
            .ok_or_else(|| error("BSA size overflow"))?;
        if total > MAX_GENERATED {
            return Err(error("BSA payload exceeds bound"));
        }
        let (folder, name) = normalized.rsplit_once('/').unwrap_or(("", &normalized));
        let folder = folder.to_ascii_lowercase().replace('/', "\\");
        if folder.len() + 1 > 255 || name.is_empty() {
            return Err(error("BSA folder/name exceeds bound"));
        }
        groups
            .entry(folder)
            .or_default()
            .push((name.to_owned(), data));
        file_names += name.len() + 1;
        file_flags |= match name
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "nif" => 1,
            "dds" => 2,
            "xml" => 4,
            "wav" => 8,
            "mp3" => 16,
            "spt" => 32,
            "fnt" | "tex" | "fon" => 64,
            _ => 256,
        };
    }
    for path in &seen {
        for (i, _) in path.match_indices('/') {
            if seen.contains(&path[..i]) {
                return Err(error("BSA member is an ancestor of another member"));
            }
        }
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(p, _)| (hash(p, ""), p.clone()));
    for (_, files) in &mut groups {
        files.sort_by_key(|(n, _)| (file_hash(n), n.to_ascii_lowercase()));
    }
    let folder_names: usize = groups.iter().map(|(n, _)| n.len() + 1).sum();
    let blocks_start = 36 + groups.len() * 16;
    let blocks_size = groups.len() + folder_names + tree.len() * 16;
    let payload_start = blocks_start + blocks_size + file_names;
    let full = payload_start
        .checked_add(total)
        .ok_or_else(|| error("BSA size overflow"))?;
    if full > MAX_GENERATED {
        return Err(error("BSA including directory exceeds bound"));
    }
    let mut out = Vec::with_capacity(full);
    out.extend_from_slice(b"BSA\0");
    for v in [
        103,
        36,
        0x603,
        groups.len(),
        tree.len(),
        folder_names,
        file_names,
        file_flags,
    ] {
        word(&mut out, v)?;
    }
    let mut offset = blocks_start;
    for (folder, files) in &groups {
        out.extend_from_slice(&hash(folder, "").to_le_bytes());
        word(&mut out, files.len())?;
        word(&mut out, offset + file_names)?;
        offset += 1 + folder.len() + 1 + files.len() * 16;
    }
    let mut payload = payload_start;
    for (folder, files) in &groups {
        out.push((folder.len() + 1) as u8);
        out.extend_from_slice(folder.as_bytes());
        out.push(0);
        for (name, bytes) in files {
            out.extend_from_slice(&file_hash(name).to_le_bytes());
            word(&mut out, bytes.len())?;
            word(&mut out, payload)?;
            payload += bytes.len();
        }
    }
    for (_, files) in &groups {
        for (name, _) in files {
            out.extend_from_slice(name.to_ascii_lowercase().as_bytes());
            out.push(0);
        }
    }
    for (_, files) in &groups {
        for (_, bytes) in files {
            check_cancel(cancel)?;
            out.extend_from_slice(bytes);
        }
    }
    debug_assert_eq!(out.len(), full);
    Ok(out)
}
