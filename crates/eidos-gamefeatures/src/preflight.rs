//! Read-only checks of PE and SKSE compatibility declarations. No DLL is loaded.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub [u16; 4]);

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [a, b, c, d] = self.0;
        write!(f, "{a}.{b}.{c}.{d}")
    }
}

#[derive(Debug, Clone)]
pub struct SkseDeclaration {
    pub data_version: u32,
    pub name: String,
    pub plugin_version: u32,
    pub version_independence: u32,
    pub version_independence_ex: u32,
    pub compatible_versions: Vec<Version>,
    pub skse_required: Version,
}

#[derive(Debug, Clone)]
pub struct PeInfo {
    pub machine: u16,
    pub file_version: Option<Version>,
    pub skse_version: Option<SkseDeclaration>,
    pub timestamp: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightDiagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub path: PathBuf,
    pub origin_mod: String,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct SkseContext<'a> {
    pub runtime: Option<Version>,
    pub skse: Option<Version>,
    /// The resolved winning versionlib file for `runtime`, not a mod directory.
    pub address_library: Option<&'a Path>,
}

pub fn address_library_name(runtime: Version) -> String {
    let [a, b, c, _] = runtime.0;
    format!("versionlib-{a}-{b}-{c}-0.bin")
}

/// Inspect the files the game will see. Mod roots are highest priority first;
/// Overwrite wins over them, and the installed game is the final fallback.
pub fn scan_skse(
    game: &eidos_gamedef::GameDef,
    game_data: &Path,
    game_root: &Path,
    mods: &[(String, PathBuf)],
    overwrite: &Path,
) -> Vec<PreflightDiagnostic> {
    if !matches!(game.id, "skyrimse" | "enderalse" | "skyrimvr") {
        return Vec::new();
    }
    let Some(extender) = game.script_extender.as_ref() else {
        return Vec::new();
    };
    let mut data_layers = mods.to_vec();
    data_layers.push(("Game".into(), game_data.to_path_buf()));
    let data = eidos_core::LayerStack::new(
        data_layers.iter().map(|(_, p)| p.clone()).collect(),
        overwrite.to_path_buf(),
    );
    let dlls: Vec<_> = data
        .list_dir("SKSE/Plugins")
        .into_iter()
        .filter(|(name, path)| name.to_ascii_lowercase().ends_with(".dll") && !path.is_dir())
        .collect();
    if dlls.is_empty() {
        return Vec::new();
    }
    let mut root_layers: Vec<_> = mods
        .iter()
        .filter_map(|(name, path)| {
            std::fs::read_dir(path)
                .ok()?
                .flatten()
                .find(|e| {
                    e.file_name().to_string_lossy().eq_ignore_ascii_case("root")
                        && e.path().is_dir()
                })
                .map(|e| (name.clone(), e.path()))
        })
        .collect();
    root_layers.push(("Game".into(), game_root.to_path_buf()));
    let root_overwrite = overwrite.join("Root");
    let root = eidos_core::LayerStack::new(
        root_layers.iter().map(|(_, p)| p.clone()).collect(),
        root_overwrite.clone(),
    );
    let mut diagnostics = Vec::new();
    let mut version = |name: &str, code, label: &str| {
        let resolved = root.resolve_read(name);
        let result = resolved
            .as_ref()
            .ok_or_else(|| format!("{name} is absent from the merged game root"))
            .and_then(|p| inspect_pe(p))
            .and_then(|p| {
                if p.machine != 0x8664 {
                    return Err("the winning executable is not x64".into());
                }
                p.file_version
                    .ok_or_else(|| "no readable file-version resource".into())
            });
        match result {
            Ok(version) => Some(version),
            Err(why) => {
                let path = resolved.unwrap_or_else(|| game_root.join(name));
                diagnostics.push(PreflightDiagnostic {
                    code,
                    severity: DiagnosticSeverity::Unverified,
                    origin_mod: layer_origin(&path, &root_layers, &root_overwrite),
                    path,
                    detail: format!("Unverified {label} version: {why}."),
                });
                None
            }
        }
    };
    let runtime = version(game.game_binary, "runtime_metadata_unverified", "runtime");
    let skse = version(extender.loader, "skse_metadata_unverified", "SKSE");
    let library = runtime
        .and_then(|v| data.resolve_read(&format!("SKSE/Plugins/{}", address_library_name(v))));
    let context = SkseContext {
        runtime,
        skse,
        address_library: library.as_deref(),
    };
    for (_, path) in dlls {
        diagnostics.extend(check_skse_plugin(
            &path,
            &layer_origin(&path, &data_layers, overwrite),
            &context,
        ));
    }
    diagnostics
}

fn layer_origin(path: &Path, layers: &[(String, PathBuf)], overwrite: &Path) -> String {
    if path.starts_with(overwrite) {
        return "Overwrite".into();
    }
    layers
        .iter()
        .find(|(_, root)| path.starts_with(root))
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| "Unverified source".into())
}

// These bounds apply to untrusted PE directory sizes, before ReadCache allocates.
// A large game EXE is fine: only its headers and the requested metadata are read.
const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Limited<'a> {
    cache: &'a object::read::ReadCache<std::fs::File>,
    remaining: &'a std::cell::Cell<u64>,
}
impl<'a> object::read::ReadRef<'a> for Limited<'a> {
    fn len(self) -> Result<u64, ()> {
        self.cache.len()
    }
    fn read_bytes_at(self, offset: u64, size: u64) -> Result<&'a [u8], ()> {
        let left = self.remaining.get().checked_sub(size).ok_or(())?;
        self.remaining.set(left);
        self.cache.read_bytes_at(offset, size)
    }
    fn read_bytes_at_until(
        self,
        range: std::ops::Range<u64>,
        delimiter: u8,
    ) -> Result<&'a [u8], ()> {
        let end = range.end.min(range.start.saturating_add(4096));
        let bytes = self.read_bytes_at(range.start, end.checked_sub(range.start).ok_or(())?)?;
        let end = bytes.iter().position(|b| *b == delimiter).ok_or(())?;
        Ok(&bytes[..end])
    }
}

/// Inspect metadata through bounded reads. Never map an image or execute its code.
pub fn inspect_pe(path: &Path) -> Result<PeInfo, String> {
    use object::pe::{ImageNtHeaders32, ImageNtHeaders64};
    use object::read::pe::optional_header_magic;
    if !std::fs::metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("not a regular PE file".into());
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("not a regular PE file".into());
    }
    let cache = object::read::ReadCache::new(file);
    let remaining = std::cell::Cell::new(MAX_METADATA_BYTES);
    let read = Limited {
        cache: &cache,
        remaining: &remaining,
    };
    match optional_header_magic(read).map_err(|e| e.to_string())? {
        0x20b => inspect::<ImageNtHeaders64>(read),
        0x10b => inspect::<ImageNtHeaders32>(read),
        _ => Err("unsupported PE optional-header format".into()),
    }
}

fn read_rva<'a>(
    read: Limited<'a>,
    sections: object::read::pe::SectionTable<'a>,
    rva: u32,
    size: u32,
) -> Result<&'a [u8], String> {
    use object::read::ReadRef;
    let (offset, available) = sections
        .pe_file_range_at(rva)
        .ok_or("metadata RVA is outside the PE sections")?;
    if size > available {
        return Err("metadata extends beyond its PE section".into());
    }
    read.read_bytes_at(u64::from(offset), u64::from(size))
        .map_err(|_| "truncated or oversized PE metadata".into())
}

fn inspect<Pe: object::read::pe::ImageNtHeaders>(read: Limited<'_>) -> Result<PeInfo, String> {
    use object::LittleEndian as LE;
    use object::read::pe::{ExportTable, ExportTarget, PeFile, ResourceDirectory};
    let file: PeFile<'_, Pe, _> = PeFile::parse(read).map_err(|e| e.to_string())?;
    let sections = file.section_table();
    let mut declaration = None;
    if let Some(dir) = file.data_directory(object::pe::IMAGE_DIRECTORY_ENTRY_EXPORT) {
        let rva = dir.virtual_address.get(LE);
        let bytes = read_rva(read, sections, rva, dir.size.get(LE))?;
        let table = ExportTable::parse(bytes, rva).map_err(|e| e.to_string())?;
        if table.name_pointers().len() > 65536 {
            return Err("too many PE exports to verify".into());
        }
        for (pointer, index) in table.name_iter() {
            let name = table
                .name_from_pointer(pointer)
                .map_err(|e| e.to_string())?;
            if name == b"SKSEPlugin_Version" {
                let ExportTarget::Address(rva) =
                    table.target_by_index(index).map_err(|e| e.to_string())?
                else {
                    return Err("forwarded SKSEPlugin_Version export is unverified".into());
                };
                declaration = Some(parse_declaration(read_rva(read, sections, rva, 848)?)?);
                break;
            }
        }
    }
    let mut version = None;
    if let Some(dir) = file.data_directory(object::pe::IMAGE_DIRECTORY_ENTRY_RESOURCE) {
        let resource = ResourceDirectory::new(read_rva(
            read,
            sections,
            dir.virtual_address.get(LE),
            dir.size.get(LE),
        )?);
        let root = resource.root().map_err(|e| e.to_string())?;
        for kind in root
            .entries
            .iter()
            .filter(|e| e.name_or_id().id() == Some(16))
        {
            let names = kind
                .data(resource)
                .map_err(|e| e.to_string())?
                .table()
                .ok_or("invalid RT_VERSION table")?;
            if names.entries.len() > 256 {
                return Err("too many version resources".into());
            }
            for name in names.entries {
                let languages = name
                    .data(resource)
                    .map_err(|e| e.to_string())?
                    .table()
                    .ok_or("invalid version language table")?;
                if languages.entries.len() > 256 {
                    return Err("too many version languages".into());
                }
                for language in languages.entries {
                    let data = language
                        .data(resource)
                        .map_err(|e| e.to_string())?
                        .data()
                        .ok_or("invalid version resource data")?;
                    let found = parse_file_version(read_rva(
                        read,
                        sections,
                        data.offset_to_data.get(LE),
                        data.size.get(LE),
                    )?)?;
                    if version.is_some_and(|v| v != found) {
                        return Err("conflicting PE file versions".into());
                    }
                    version = Some(found);
                }
            }
        }
    }
    Ok(PeInfo {
        machine: file.nt_headers().file_header().machine.get(LE).0,
        timestamp: file.nt_headers().file_header().time_date_stamp.get(LE),
        file_version: version,
        skse_version: declaration,
    })
}

fn word(bytes: &[u8], at: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at + 2)
            .ok_or("truncated metadata word")?
            .try_into()
            .unwrap(),
    ))
}
fn dword(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or("truncated metadata integer")?
            .try_into()
            .unwrap(),
    ))
}
fn unpack(v: u32) -> Version {
    Version([
        (v >> 24) as u16,
        ((v >> 16) & 255) as u16,
        ((v >> 4) & 4095) as u16,
        (v & 15) as u16,
    ])
}

fn parse_file_version(bytes: &[u8]) -> Result<Version, String> {
    let bytes = bytes
        .get(..usize::from(word(bytes, 0)?))
        .ok_or("truncated VS_VERSION_INFO")?;
    if word(bytes, 2)? != 52 || word(bytes, 4)? != 0 {
        return Err("unsupported VS_VERSION_INFO value".into());
    }
    let mut at = 6;
    for expected in "VS_VERSION_INFO\0".encode_utf16() {
        if word(bytes, at)? != expected {
            return Err("invalid VS_VERSION_INFO key".into());
        }
        at += 2;
    }
    at = (at + 3) & !3;
    if dword(bytes, at)? != 0xFEEF04BD || dword(bytes, at + 4)? != 0x10000 {
        return Err("invalid VS_FIXEDFILEINFO signature/version".into());
    }
    bytes.get(at..at + 52).ok_or("truncated VS_FIXEDFILEINFO")?;
    let ms = dword(bytes, at + 8)?;
    let ls = dword(bytes, at + 12)?;
    Ok(Version([
        (ms >> 16) as u16,
        ms as u16,
        (ls >> 16) as u16,
        ls as u16,
    ]))
}

fn parse_declaration(bytes: &[u8]) -> Result<SkseDeclaration, String> {
    let name = bytes.get(8..264).ok_or("truncated SKSE name")?;
    let end = name
        .iter()
        .position(|b| *b == 0)
        .ok_or("unterminated SKSE name")?;
    if end == 0 {
        return Err("empty SKSE plugin name".into());
    }
    let name = std::str::from_utf8(&name[..end])
        .map_err(|_| "invalid SKSE name encoding")?
        .to_string();
    let mut versions = Vec::new();
    for at in (780..844).step_by(4) {
        let packed = dword(bytes, at)?;
        if packed == 0 {
            break;
        }
        versions.push(unpack(packed));
    }
    Ok(SkseDeclaration {
        data_version: dword(bytes, 0)?,
        plugin_version: dword(bytes, 4)?,
        name,
        version_independence_ex: dword(bytes, 772)?,
        version_independence: dword(bytes, 776)?,
        compatible_versions: versions,
        skse_required: unpack(dword(bytes, 844)?),
    })
}

/// Check the winning Skyrim SE plugin against known runtime metadata. A matching
/// declaration does not establish that its native hooks are safe or will load.
pub fn check_skse_plugin(
    path: &Path,
    origin_mod: &str,
    context: &SkseContext<'_>,
) -> Vec<PreflightDiagnostic> {
    use DiagnosticSeverity::{Error, Unverified};
    let mut out = Vec::new();
    let mut add = |code, severity, detail| {
        out.push(PreflightDiagnostic {
            code,
            severity,
            path: path.to_path_buf(),
            origin_mod: origin_mod.into(),
            detail,
        })
    };
    let info = match inspect_pe(path) {
        Ok(info) => info,
        Err(e) => {
            add(
                "pe_unverified",
                Unverified,
                format!("Could not inspect PE metadata: {e}. Compatibility is unverified."),
            );
            return out;
        }
    };
    if info.machine != 0x8664 {
        add(
            "pe_architecture",
            Error,
            format!(
                "PE machine 0x{:04X} cannot load in Skyrim SE's x86-64 process.",
                info.machine
            ),
        );
        return out;
    }
    let Some(v) = info.skse_version else {
        add(
            "skse_export_unverified",
            Unverified,
            "No SKSEPlugin_Version export; runtime compatibility is unverified.".into(),
        );
        return out;
    };
    if v.data_version != 1 {
        add(
            "skse_declaration_unverified",
            Unverified,
            format!(
                "SKSE declaration version {} is unsupported; compatibility is unverified.",
                v.data_version
            ),
        );
        return out;
    }
    if v.version_independence & !7 != 0 || v.version_independence_ex & !3 != 0 {
        add(
            "skse_flags_unverified",
            Unverified,
            "Unknown SKSE compatibility flags; their meaning is unverified.".into(),
        );
        return out;
    }
    let mut independent = v.version_independence & 3 != 0;
    match context.runtime {
        None => add(
            "runtime_unverified",
            Unverified,
            "The game EXE runtime version is unavailable; compatibility is unverified.".into(),
        ),
        Some(runtime) => {
            // Pre-AE SKSE asks plugin code through SKSEPlugin_Query instead of
            // trusting this declaration. Do not pretend to have executed it.
            if runtime < Version([1, 6, 318, 0]) {
                add("legacy_runtime_unverified", Unverified, "This runtime uses legacy SKSEPlugin_Query compatibility checks; static declaration compatibility is unverified.".into());
                return out;
            }
            if v.version_independence & 1 != 0 {
                match context.address_library {
                    Some(p)
                        if std::fs::File::open(p)
                            .and_then(|f| f.metadata())
                            .is_ok_and(|m| m.is_file() && m.len() > 0) => {}
                    _ => add(
                        "address_library_missing",
                        Error,
                        format!(
                            "The plugin requires a readable {} in the merged SKSE/Plugins directory.",
                            address_library_name(runtime)
                        ),
                    ),
                }
                // Match SKSE's explicit pre-V5 build-window exception. Unknown
                // timestamps cannot establish which Address Library ABI was used.
                if runtime >= Version([1, 7, 99, 0]) && v.version_independence_ex & 2 == 0 {
                    if (520128000..1748217600).contains(&info.timestamp) {
                        independent = false;
                    } else {
                        add("address_library_abi_unverified", Unverified,
                            "The declaration lacks the Address Library V5 flag; binary ABI compatibility is unverified.".into());
                    }
                }
            }
            if independent
                && v.version_independence_ex & 1 == 0
                && (runtime >= Version([1, 6, 629, 0])) != (v.version_independence & 4 != 0)
            {
                add("runtime_structures", Error, "The plugin's declared pre/post-1.6.629 structure layout does not match this runtime.".into());
            }
            if !independent && !v.compatible_versions.contains(&runtime) {
                add(
                    "runtime_mismatch",
                    Error,
                    format!(
                        "The plugin's explicit compatible-runtime list does not include {runtime}."
                    ),
                );
            }
        }
    }
    if v.skse_required != Version([0; 4]) {
        match context.skse {
            Some(installed) if installed < v.skse_required => add(
                "skse_too_old",
                Error,
                format!(
                    "Requires SKSE {}; installed loader reports {installed}.",
                    v.skse_required
                ),
            ),
            None => add(
                "skse_version_unverified",
                Unverified,
                format!(
                    "Requires SKSE {}; installed loader version is unavailable.",
                    v.skse_required
                ),
            ),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(bytes: &[u8]) -> Self {
            let p = std::env::temp_dir().join(format!(
                "eidos-preflight-{}-{}.dll",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::write(&p, bytes).unwrap();
            Self(p)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    fn u16at(bytes: &mut [u8], at: usize, v: u16) {
        bytes[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn u32at(bytes: &mut [u8], at: usize, v: u32) {
        bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 0xC00];
        b[..2].copy_from_slice(b"MZ");
        u32at(&mut b, 0x3c, 0x80);
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        u16at(&mut b, 0x84, 0x8664);
        u16at(&mut b, 0x86, 1);
        u16at(&mut b, 0x94, 240);
        u16at(&mut b, 0x98, 0x20b);
        u32at(&mut b, 0x98 + 108, 16);
        u32at(&mut b, 0x98 + 112, 0x1000);
        u32at(&mut b, 0x98 + 116, 0x100);
        u32at(&mut b, 0x98 + 128, 0x1600);
        u32at(&mut b, 0x98 + 132, 0x200);
        let section = 0x188;
        b[section..section + 6].copy_from_slice(b".rdata");
        u32at(&mut b, section + 8, 0xA00);
        u32at(&mut b, section + 12, 0x1000);
        u32at(&mut b, section + 16, 0xA00);
        u32at(&mut b, section + 20, 0x200);
        u32at(&mut b, 0x200 + 16, 1);
        u32at(&mut b, 0x200 + 20, 1);
        u32at(&mut b, 0x200 + 24, 1);
        u32at(&mut b, 0x200 + 28, 0x1040);
        u32at(&mut b, 0x200 + 32, 0x1044);
        u32at(&mut b, 0x200 + 36, 0x1048);
        u32at(&mut b, 0x240, 0x1100);
        u32at(&mut b, 0x244, 0x1050);
        b[0x250..0x263].copy_from_slice(b"SKSEPlugin_Version\0");
        u32at(&mut b, 0x300, 1);
        b[0x308..0x30d].copy_from_slice(b"Test\0");
        u32at(&mut b, 0x300 + 780, 0x01070680); // 1.7.104.0
        // RT_VERSION / name / language tables, then IMAGE_RESOURCE_DATA_ENTRY.
        for (at, id, next) in [(0x800, 16, 24), (0x818, 1, 48), (0x830, 1033, 72)] {
            u16at(&mut b, at + 14, 1);
            u32at(&mut b, at + 16, id);
            u32at(
                &mut b,
                at + 20,
                next | if at == 0x830 { 0 } else { 0x80000000 },
            );
        }
        u32at(&mut b, 0x848, 0x1700);
        u32at(&mut b, 0x84c, 92);
        u16at(&mut b, 0x900, 92);
        u16at(&mut b, 0x902, 52);
        for (i, v) in "VS_VERSION_INFO\0".encode_utf16().enumerate() {
            u16at(&mut b, 0x906 + i * 2, v);
        }
        u32at(&mut b, 0x928, 0xFEEF04BD);
        u32at(&mut b, 0x92c, 0x10000);
        u32at(&mut b, 0x930, 0x00010007);
        u32at(&mut b, 0x934, 104 << 16);
        b
    }

    #[test]
    fn merged_preflight_uses_winners_and_honors_dll_and_library_whiteouts() {
        let root = std::env::temp_dir().join(format!(
            "eidos-preflight-view-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let low = root.join("low");
        let high = root.join("high");
        let ow = root.join("overwrite");
        let base = root.join("game");
        for p in [&low, &high, &ow] {
            std::fs::create_dir_all(p.join("SKSE/Plugins")).unwrap();
        }
        std::fs::create_dir_all(base.join("Data")).unwrap();
        std::fs::create_dir_all(high.join("rOoT")).unwrap();
        std::fs::write(base.join("SkyrimSE.exe"), b"bad lower executable").unwrap();
        std::fs::write(high.join("rOoT/SKYRIMSE.EXE"), fixture()).unwrap();
        let mut loader = fixture();
        u32at(&mut loader, 0x930, 0x00020003);
        u32at(&mut loader, 0x934, 1 << 16);
        std::fs::write(high.join("rOoT/skse64_loader.exe"), loader).unwrap();
        std::fs::write(low.join("SKSE/Plugins/Test.dll"), b"invalid shadowed DLL").unwrap();
        std::fs::write(low.join("SKSE/Plugins/Hidden.dll"), b"invalid hidden DLL").unwrap();
        std::fs::write(ow.join("SKSE/Plugins/.eidoswh.hidden.dll"), []).unwrap();
        let mut dll = fixture();
        u32at(&mut dll, 0x300 + 776, 5);
        u32at(&mut dll, 0x300 + 772, 2);
        u32at(&mut dll, 0x300 + 844, 0x02030010);
        std::fs::write(high.join("SKSE/Plugins/TEST.DLL"), dll).unwrap();
        std::fs::write(
            low.join("SKSE/Plugins/versionlib-1-7-104-0.bin"),
            b"library",
        )
        .unwrap();
        std::fs::write(
            ow.join("SKSE/Plugins/.eidoswh.versionlib-1-7-104-0.bin"),
            [],
        )
        .unwrap();
        let mods = vec![("Higher".into(), high.clone()), ("Lower".into(), low)];
        let game = eidos_gamedef::GAMES
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap();
        let found = scan_skse(game, &base.join("Data"), &base, &mods, &ow);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].code, "address_library_missing");
        assert_eq!(found[0].origin_mod, "Higher");
        assert_eq!(found[0].path, high.join("SKSE/Plugins/TEST.DLL"));
        std::fs::create_dir_all(ow.join("Root")).unwrap();
        std::fs::write(ow.join("Root/.eidoswh.skyrimse.exe"), []).unwrap();
        let found = scan_skse(game, &base.join("Data"), &base, &mods, &ow);
        assert!(found.iter().any(|d| d.code == "runtime_metadata_unverified"
            && d.severity == DiagnosticSeverity::Unverified));
        assert!(!found.iter().any(|d| d.code == "runtime_mismatch"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn merged_preflight_reports_unknown_loader_and_ignores_non_skse_games() {
        let root = std::env::temp_dir().join(format!(
            "eidos-preflight-loader-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("Data/SKSE/Plugins")).unwrap();
        std::fs::write(root.join("Data/SKSE/Plugins/Test.dll"), fixture()).unwrap();
        std::fs::write(root.join("SkyrimSE.exe"), fixture()).unwrap();
        let game = eidos_gamedef::GAMES
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap();
        let found = scan_skse(
            game,
            &root.join("Data"),
            &root,
            &[],
            &root.join("overwrite"),
        );
        assert!(found.iter().any(|d| d.code == "skse_metadata_unverified"));
        let game = eidos_gamedef::GAMES
            .iter()
            .find(|g| g.id == "fallout4")
            .unwrap();
        assert!(
            scan_skse(
                game,
                &root.join("Data"),
                &root,
                &[],
                &root.join("overwrite")
            )
            .is_empty()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn reads_real_pe_structure_and_packed_skse_declarations() {
        let f = Tmp::new(&fixture());
        let info = inspect_pe(&f.0).unwrap();
        assert_eq!(info.machine, 0x8664);
        assert_eq!(info.file_version, Some(Version([1, 7, 104, 0])));
        assert_eq!(
            info.skse_version.unwrap().compatible_versions,
            [Version([1, 7, 104, 0])]
        );
    }
    #[test]
    fn explicit_runtime_mismatch_has_mod_origin() {
        let f = Tmp::new(&fixture());
        let d = check_skse_plugin(
            &f.0,
            "Local patch",
            &SkseContext {
                runtime: Some(Version([1, 6, 1170, 0])),
                ..Default::default()
            },
        );
        assert!(d.iter().any(|d| d.code == "runtime_mismatch"
            && d.severity == DiagnosticSeverity::Error
            && d.origin_mod == "Local patch"));
    }
    #[test]
    fn missing_metadata_is_unverified_and_unknown_flags_are_not_compatible() {
        let mut b = fixture();
        u32at(&mut b, 0x300 + 776, 0x80000000);
        let f = Tmp::new(&b);
        let d = check_skse_plugin(&f.0, "M", &SkseContext::default());
        assert!(d.iter().any(|d| d.code == "skse_flags_unverified"));
        u32at(&mut b, 0x98 + 112, 0);
        u32at(&mut b, 0x98 + 116, 0);
        let f = Tmp::new(&b);
        let d = check_skse_plugin(&f.0, "M", &SkseContext::default());
        assert!(d.iter().any(|d| d.code == "skse_export_unverified"));
    }
    #[test]
    fn address_library_and_structure_flags_are_checked_without_loading_code() {
        let mut b = fixture();
        u32at(&mut b, 0x300 + 776, 5);
        u32at(&mut b, 0x300 + 772, 2);
        let f = Tmp::new(&b);
        let library = Tmp::new(b"address-library");
        let mut context = SkseContext {
            runtime: Some(Version([1, 7, 104, 0])),
            skse: None,
            address_library: None,
        };
        assert!(
            check_skse_plugin(&f.0, "M", &context)
                .iter()
                .any(|d| d.code == "address_library_missing")
        );
        context.address_library = Some(&library.0);
        assert!(
            !check_skse_plugin(&f.0, "M", &context)
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error)
        );
        u32at(&mut b, 0x300 + 776, 1);
        let f = Tmp::new(&b);
        assert!(
            check_skse_plugin(&f.0, "M", &context)
                .iter()
                .any(|d| d.code == "runtime_structures")
        );
    }
    #[test]
    fn malformed_and_truncated_files_never_claim_compatibility() {
        for bytes in [vec![], b"MZ".to_vec(), fixture()[..600].to_vec()] {
            let f = Tmp::new(&bytes);
            assert!(inspect_pe(&f.0).is_err());
            assert!(
                check_skse_plugin(&f.0, "M", &SkseContext::default())
                    .iter()
                    .any(|d| d.code == "pe_unverified")
            );
        }
    }
    #[test]
    fn corrupt_metadata_ranges_and_unknown_declaration_versions_are_unverified() {
        for offset in [
            0x98 + 112,
            0x98 + 116,
            0x98 + 128,
            0x98 + 132,
            0x240,
            0x848,
            0x84c,
        ] {
            let mut b = fixture();
            u32at(&mut b, offset, u32::MAX);
            let f = Tmp::new(&b);
            assert!(inspect_pe(&f.0).is_err(), "offset {offset}");
        }
        let mut b = fixture();
        u32at(&mut b, 0x300, 2);
        let f = Tmp::new(&b);
        assert!(
            check_skse_plugin(&f.0, "M", &SkseContext::default())
                .iter()
                .any(|d| d.code == "skse_declaration_unverified")
        );
        let mut b = fixture();
        u32at(&mut b, 0x300 + 772, 0x80000000);
        let f = Tmp::new(&b);
        assert!(
            check_skse_plugin(&f.0, "M", &SkseContext::default())
                .iter()
                .any(|d| d.code == "skse_flags_unverified")
        );
    }
    #[test]
    fn explicit_runtime_and_minimum_skse_can_match_without_claiming_hooks_are_safe() {
        let mut b = fixture();
        u32at(&mut b, 0x300 + 844, 0x02030010);
        let f = Tmp::new(&b);
        let context = SkseContext {
            runtime: Some(Version([1, 7, 104, 0])),
            skse: Some(Version([2, 3, 1, 0])),
            address_library: None,
        };
        assert!(check_skse_plugin(&f.0, "M", &context).is_empty());
        assert_eq!(
            address_library_name(Version([1, 7, 104, 0])),
            "versionlib-1-7-104-0.bin"
        );
    }
    #[test]
    fn post_629_structures_do_not_claim_compatibility_with_older_layouts() {
        let mut b = fixture();
        u32at(&mut b, 0x300 + 776, 5);
        u32at(&mut b, 0x300 + 772, 2);
        let f = Tmp::new(&b);
        let library = Tmp::new(b"address-library");
        let context = SkseContext {
            runtime: Some(Version([1, 6, 353, 0])),
            skse: None,
            address_library: Some(&library.0),
        };
        assert!(
            check_skse_plugin(&f.0, "M", &context)
                .iter()
                .any(|d| d.code == "runtime_structures")
        );
        let context = SkseContext {
            runtime: Some(Version([1, 5, 97, 0])),
            ..context
        };
        assert!(
            check_skse_plugin(&f.0, "M", &context)
                .iter()
                .any(|d| d.code == "legacy_runtime_unverified")
        );
    }
    #[test]
    fn minimum_skse_and_architecture_mismatches_are_errors() {
        let mut b = fixture();
        u32at(&mut b, 0x300 + 844, 0x02030010);
        let f = Tmp::new(&b);
        let context = SkseContext {
            runtime: Some(Version([1, 7, 104, 0])),
            skse: Some(Version([2, 2, 6, 0])),
            address_library: None,
        };
        assert!(
            check_skse_plugin(&f.0, "M", &context)
                .iter()
                .any(|d| d.code == "skse_too_old")
        );
        u16at(&mut b, 0x84, 0x14c);
        let f = Tmp::new(&b);
        assert!(
            check_skse_plugin(&f.0, "M", &context)
                .iter()
                .any(|d| d.code == "pe_architecture")
        );
    }
}
