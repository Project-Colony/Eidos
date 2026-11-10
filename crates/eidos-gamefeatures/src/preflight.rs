//! Read-only checks of PE and script extender compatibility declarations. No
//! DLL is loaded.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

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

/// `F4SEPluginVersionData`; `SFSEPluginVersionData` has the same layout.
#[derive(Debug, Clone)]
pub struct XseDeclaration {
    /// The export it was read from: `F4SEPlugin_Version` or `SFSEPlugin_Version`.
    pub export: &'static str,
    pub data_version: u32,
    pub name: String,
    pub address_independence: u32,
    pub structure_independence: u32,
    pub compatible_versions: Vec<Version>,
    pub reserved_breaking: u32,
}

#[derive(Debug, Clone)]
pub struct PeInfo {
    pub machine: u16,
    pub file_version: Option<Version>,
    pub skse_version: Option<SkseDeclaration>,
    pub xse_version: Option<XseDeclaration>,
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

/// Inspect the script extender plugins the game will see. Mod roots are
/// highest priority first; Overwrite wins over them, and the installed game is
/// the final fallback.
pub fn scan_extender_plugins(
    game: &eidos_gamedef::GameDef,
    game_data: &Path,
    game_root: &Path,
    mods: &[(String, PathBuf)],
    overwrite: &Path,
) -> Vec<PreflightDiagnostic> {
    let machine = match game.id {
        "skyrimse" | "enderalse" | "skyrimvr" => {
            return scan_skse(game, game_data, game_root, mods, overwrite)
        }
        "fallout4" | "starfield" => 0x8664,
        "oblivion" | "falloutnv" | "fallout3" => 0x14c,
        _ => return Vec::new(),
    };
    let Some(row) = extender(game.id) else {
        return Vec::new();
    };
    let data = merged_data(game_data, mods, overwrite);
    let dlls = plugin_dlls(&data, row.plugins_dir);
    if dlls.is_empty() {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    let mut rules = None;
    if machine == 0x8664 {
        let root = merged_root(game_root, mods, overwrite);
        let runtime = version_or_card(
            &root,
            game_root,
            game.game_binary,
            "runtime_metadata_unverified",
            "runtime",
            &mut diagnostics,
        );
        rules = runtime.and_then(|runtime| {
            let rules = xse_rules(game.id, runtime)?;
            let library = data.resolve(&format!(
                "{}/{}",
                row.plugins_dir,
                (rules.library_name)(runtime)
            ));
            Some((runtime, rules, library))
        });
    }
    for (_, path) in dlls {
        let origin = data.origin(&path);
        match &rules {
            Some((runtime, rules, library)) => diagnostics.extend(check_xse_plugin(
                &path,
                &origin,
                *runtime,
                rules,
                library.as_deref(),
            )),
            // Old-gen Fallout 4, an unread runtime, or a 32-bit game.
            None => diagnostics.extend(plugin_pe(&path, &origin, machine).err()),
        }
    }
    diagnostics
}

fn scan_skse(
    game: &eidos_gamedef::GameDef,
    game_data: &Path,
    game_root: &Path,
    mods: &[(String, PathBuf)],
    overwrite: &Path,
) -> Vec<PreflightDiagnostic> {
    let Some(extender) = game.script_extender.as_ref() else {
        return Vec::new();
    };
    let data = merged_data(game_data, mods, overwrite);
    let dlls = plugin_dlls(&data, "SKSE/Plugins");
    if dlls.is_empty() {
        return Vec::new();
    }
    let root = merged_root(game_root, mods, overwrite);
    let mut diagnostics = Vec::new();
    let runtime = version_or_card(
        &root,
        game_root,
        game.game_binary,
        "runtime_metadata_unverified",
        "runtime",
        &mut diagnostics,
    );
    let skse = version_or_card(
        &root,
        game_root,
        extender.loader,
        "skse_metadata_unverified",
        "SKSE",
        &mut diagnostics,
    );
    let library =
        runtime.and_then(|v| data.resolve(&format!("SKSE/Plugins/{}", address_library_name(v))));
    let context = SkseContext {
        runtime,
        skse,
        address_library: library.as_deref(),
    };
    for (_, path) in dlls {
        diagnostics.extend(check_skse_plugin(&path, &data.origin(&path), &context));
    }
    diagnostics
}

/// The `.dll` files of the merged directory `dir`.
fn plugin_dlls(data: &Merged, dir: &str) -> Vec<(String, PathBuf)> {
    data.list_dir(dir)
        .into_iter()
        .filter(|(name, path)| name.to_ascii_lowercase().ends_with(".dll") && !path.is_dir())
        .collect()
}

/// `runtime_version` of `name`, or None once an Unverified `code` card says why.
fn version_or_card(
    root: &Merged,
    game_root: &Path,
    name: &str,
    code: &'static str,
    label: &str,
    diagnostics: &mut Vec<PreflightDiagnostic>,
) -> Option<Version> {
    match runtime_version(root, name) {
        Ok(version) => Some(version),
        Err(why) => {
            let path = root.resolve(name).unwrap_or_else(|| game_root.join(name));
            diagnostics.push(PreflightDiagnostic {
                code,
                severity: DiagnosticSeverity::Unverified,
                origin_mod: root.origin(&path),
                path,
                detail: format!("Unverified {label} version: {why}."),
            });
            None
        }
    }
}

/// A merged view of one tree (Data, or the game root): enabled mods highest
/// priority first, the instance Overwrite on top, the installed game last.
/// Unindexed on purpose: each caller answers a listing and a couple of lookups,
/// and the GUI's health checks run on every mod toggle. An index would first
/// walk every file of every mod (and, for the root, the whole game install).
pub struct Merged {
    stack: eidos_core::LayerStack,
    layers: Vec<(String, PathBuf)>,
    overwrite: PathBuf,
}

impl Merged {
    fn new(layers: Vec<(String, PathBuf)>, overwrite: PathBuf) -> Self {
        let stack = eidos_core::LayerStack::new_unindexed(
            layers.iter().map(|(_, p)| p.clone()).collect(),
            overwrite.clone(),
        );
        Self {
            stack,
            layers,
            overwrite,
        }
    }

    pub fn resolve(&self, rel: &str) -> Option<PathBuf> {
        self.stack.resolve_read(rel)
    }

    pub fn list_dir(&self, rel: &str) -> Vec<(String, PathBuf)> {
        self.stack.list_dir(rel)
    }

    /// The mod name, "Overwrite", "Game" or "Unverified source".
    pub fn origin(&self, path: &Path) -> String {
        layer_origin(path, &self.layers, &self.overwrite)
    }
}

pub fn merged_data(game_data: &Path, mods: &[(String, PathBuf)], overwrite: &Path) -> Merged {
    let mut layers = mods.to_vec();
    layers.push(("Game".into(), game_data.to_path_buf()));
    Merged::new(layers, overwrite.to_path_buf())
}

/// Each mod's `Root/` child (any case) plus the game root; `overwrite` is the
/// instance Overwrite, `Root` is joined inside.
pub fn merged_root(game_root: &Path, mods: &[(String, PathBuf)], overwrite: &Path) -> Merged {
    let mut layers: Vec<_> = mods
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
    layers.push(("Game".into(), game_root.to_path_buf()));
    Merged::new(layers, overwrite.join("Root"))
}

/// The file version of the winning x64 executable `game_binary` in `root`.
pub fn runtime_version(root: &Merged, game_binary: &str) -> Result<Version, String> {
    let path = root
        .resolve(game_binary)
        .ok_or_else(|| format!("{game_binary} is absent from the merged game root"))?;
    let info = inspect_pe(&path)?;
    if info.machine != 0x8664 {
        return Err("the winning executable is not x64".into());
    }
    info.file_version
        .ok_or_else(|| "no readable file-version resource".into())
}

/// Where a game's script extender keeps its plugins and, for the 64-bit
/// extenders, the prefix of the runtime DLL the loader needs next to itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extender {
    pub plugins_dir: &'static str,
    pub runtime_dll: Option<&'static str>,
    pub page: &'static str,
}

pub fn extender(game_id: &str) -> Option<Extender> {
    const SKSE: &str = "https://skse.silverlock.org/";
    const F4SE: &str = "https://f4se.silverlock.org/";
    let (plugins_dir, runtime_dll, page) = match game_id {
        "skyrimse" | "enderalse" => ("SKSE/Plugins", Some("skse64"), SKSE),
        "skyrimvr" => ("SKSE/Plugins", Some("sksevr"), SKSE),
        "skyrim" => ("SKSE/Plugins", None, SKSE),
        "fallout4" => ("F4SE/Plugins", Some("f4se"), F4SE),
        "fallout4vr" => ("F4SE/Plugins", Some("f4sevr"), F4SE),
        "starfield" => ("SFSE/Plugins", Some("sfse"), "https://sfse.silverlock.org/"),
        "falloutnv" => (
            "NVSE/Plugins",
            None,
            "https://github.com/xNVSE/NVSE/releases/latest",
        ),
        "fallout3" => ("FOSE/Plugins", None, "https://fose.silverlock.org/"),
        "oblivion" => ("OBSE/Plugins", None, "https://obse.silverlock.org/"),
        _ => return None,
    };
    Some(Extender {
        plugins_dir,
        runtime_dll,
        page,
    })
}

/// `skse64_1_6_1170.dll`: the loader looks for this file for runtime 1.6.1170.
pub fn extender_runtime_dll(prefix: &str, runtime: Version) -> String {
    let [a, b, c, _] = runtime.0;
    format!("{prefix}_{a}_{b}_{c}.dll")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtenderFacts {
    pub loader: &'static str,
    pub loader_found: bool,
    pub plugins_dir: &'static str,
    pub page: &'static str,
    pub plugin_dlls: usize,
    pub runtime: Option<Version>,
    pub runtime_dll: Option<String>,
    pub runtime_dll_found: bool,
}

/// None when the game has no `script_extender` or no `extender()` row.
pub fn extender_facts(
    game: &eidos_gamedef::GameDef,
    game_data: &Path,
    game_root: &Path,
    mods: &[(String, PathBuf)],
    overwrite: &Path,
) -> Option<ExtenderFacts> {
    let se = game.script_extender.as_ref()?;
    let row = extender(game.id)?;
    let root = merged_root(game_root, mods, overwrite);
    let is_file = |name: &str| root.resolve(name).is_some_and(|p| p.is_file());
    let plugin_dlls = plugin_dlls(&merged_data(game_data, mods, overwrite), row.plugins_dir).len();
    let runtime = if game.game_binary.is_empty() {
        None
    } else {
        runtime_version(&root, game.game_binary).ok()
    };
    let runtime_dll = runtime
        .zip(row.runtime_dll)
        .map(|(v, prefix)| extender_runtime_dll(prefix, v));
    let runtime_dll_found = runtime_dll.as_deref().is_some_and(is_file);
    Some(ExtenderFacts {
        loader: se.loader,
        loader_found: is_file(se.loader),
        plugins_dir: row.plugins_dir,
        page: row.page,
        plugin_dlls,
        runtime,
        runtime_dll,
        runtime_dll_found,
    })
}

/// The fonts MO2 treats as always present: `Skyrim - Interface.bsa` ships them.
const VANILLA_FONTS: [&str; 3] = [
    "interface/fonts_console.swf",
    "interface/fonts_en.swf",
    "interface/fonts_cclub.swf",
];

/// Warn about `fontlib "Interface\X.swf"` lines of the winning
/// `Interface/fontconfig.txt` whose file is in no layer and in no archive of a
/// layer. The paths are Data-relative, as MO2's check reads them. The vanilla
/// fonts count as present, the root is listed once, and the archive search
/// stops once every font is found.
pub fn scan_fontconfig(
    game: &eidos_gamedef::GameDef,
    game_data: &Path,
    mods: &[(String, PathBuf)],
    overwrite: &Path,
) -> Vec<PreflightDiagnostic> {
    if !matches!(game.id, "skyrim" | "skyrimse" | "skyrimvr" | "enderalse") {
        return Vec::new();
    }
    let mut layers = mods.to_vec();
    layers.push(("Game".into(), game_data.to_path_buf()));
    let data = eidos_core::LayerStack::new_unindexed(
        layers.iter().map(|(_, p)| p.clone()).collect(),
        overwrite.to_path_buf(),
    );
    let Some(config) = data.resolve_read("Interface/fontconfig.txt") else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read(&config) else {
        return Vec::new();
    };
    // Each entry: the name as the file writes it, and its lowercased Data path.
    let mut missing: Vec<(String, String)> = Vec::new();
    for font in fontlibs(&String::from_utf8_lossy(&text)) {
        let rel = font.replace('\\', "/");
        let key = rel.to_ascii_lowercase();
        if !VANILLA_FONTS.contains(&key.as_str())
            && !missing.iter().any(|(_, k)| *k == key)
            && data.resolve_read(&rel).is_none()
        {
            missing.push((font, key));
        }
    }
    if !missing.is_empty() {
        for (name, path) in data.list_dir("") {
            let name = name.to_ascii_lowercase();
            if !(name.ends_with(".bsa") || name.ends_with(".ba2")) {
                continue;
            }
            // fontlib files are Scaleform .swf movies; keeping only those keeps
            // the memo small for a 100k-member mesh archive.
            let swfs = memoised(&SWF_MEMBERS, &path, || {
                eidos_conflicts::archives::read_archive_members(&path)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|m| m.to_ascii_lowercase())
                    .filter(|m| m.ends_with(".swf"))
                    .collect()
            });
            missing.retain(|(_, key)| !swfs.contains(key));
            if missing.is_empty() {
                break;
            }
        }
    }
    if missing.is_empty() {
        return Vec::new();
    }
    vec![PreflightDiagnostic {
        code: "fontconfig_missing_font",
        severity: DiagnosticSeverity::Warning,
        origin_mod: layer_origin(&config, &layers, overwrite),
        path: config,
        detail: missing
            .into_iter()
            .map(|(font, _)| font)
            .collect::<Vec<_>>()
            .join(", "),
    }]
}

// shortcut: entries are never evicted (one small entry per archive path seen in
// a session); prune them if instances ever hold tens of thousands of archives.
type ArchiveMemo<T> = Mutex<HashMap<PathBuf, (u64, SystemTime, T)>>;
/// The `.swf` members of each archive, lowercased.
static SWF_MEMBERS: LazyLock<ArchiveMemo<Vec<String>>> = LazyLock::new(Default::default);
/// How many compressed audio members each Fallout 4 (or VR) BA2 holds.
static PACKED_AUDIO: LazyLock<ArchiveMemo<usize>> = LazyLock::new(Default::default);

/// `read()` for `path`, reused while the file keeps its size and mtime. The GUI
/// reruns the Health checks on its own thread after every plugin toggle and mod
/// change, so an archive directory is parsed again only when the file changes.
fn memoised<T: Clone>(memo: &ArchiveMemo<T>, path: &Path, read: impl FnOnce() -> T) -> T {
    let stamp = std::fs::metadata(path)
        .ok()
        .and_then(|m| Some((m.len(), m.modified().ok()?)));
    let Some((len, mtime)) = stamp else {
        return read();
    };
    let mut memo = memo.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((l, t, value)) = memo.get(path) {
        if (*l, *t) == (len, mtime) {
            return value.clone();
        }
    }
    let value = read();
    memo.insert(path.to_path_buf(), (len, mtime, value.clone()));
    value
}

/// Warn about mod BA2 archives that store `.wav` or `.xwm` audio compressed
/// (Fallout 4 and Fallout 4 VR can crash on it). The installed game's own
/// archives are not read.
pub fn scan_compressed_audio(
    game: &eidos_gamedef::GameDef,
    mods: &[(String, PathBuf)],
    overwrite: &Path,
) -> Vec<PreflightDiagnostic> {
    if !matches!(game.id, "fallout4" | "fallout4vr") {
        return Vec::new();
    }
    let data = eidos_core::LayerStack::new_unindexed(
        mods.iter().map(|(_, p)| p.clone()).collect(),
        overwrite.to_path_buf(),
    );
    data.list_dir("")
        .into_iter()
        .filter(|(name, _)| name.to_ascii_lowercase().ends_with(".ba2"))
        .filter_map(|(name, path)| {
            let bad = memoised(&PACKED_AUDIO, &path, || {
                eidos_conflicts::archives::compressed_audio_members(&path).map_or(0, |b| b.len())
            });
            (bad > 0).then(|| PreflightDiagnostic {
                code: "ba2_compressed_audio",
                severity: DiagnosticSeverity::Warning,
                origin_mod: layer_origin(&path, mods, overwrite),
                path,
                detail: format!("{name}: {bad}"),
            })
        })
        .collect()
}

/// Names in `fontlib "X.swf"` lines. Not anchored to the end of the line, so a
/// CRLF file parses; `#` starts a comment.
fn fontlibs(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line
                .get(..7)
                .filter(|k| k.eq_ignore_ascii_case("fontlib"))?;
            let rest = line[rest.len()..].trim_start().strip_prefix('"')?;
            let name = &rest[..rest.find('"')?];
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
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
    use object::read::pe::{ExportTable, ExportTarget, PeFile, ResourceDirectory};
    use object::LittleEndian as LE;
    let file: PeFile<'_, Pe, _> = PeFile::parse(read).map_err(|e| e.to_string())?;
    let sections = file.section_table();
    let mut declaration = None;
    let mut xse = None;
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
            let Some(export) = [
                "SKSEPlugin_Version",
                "F4SEPlugin_Version",
                "SFSEPlugin_Version",
            ]
            .into_iter()
            .find(|export| export.as_bytes() == name) else {
                continue;
            };
            let ExportTarget::Address(rva) =
                table.target_by_index(index).map_err(|e| e.to_string())?
            else {
                return Err(format!("forwarded {export} export is unverified"));
            };
            if export == "SKSEPlugin_Version" {
                declaration = Some(parse_declaration(read_rva(read, sections, rva, 848)?)?);
            } else {
                xse = Some(parse_xse_declaration(
                    export,
                    read_rva(read, sections, rva, 604)?,
                )?);
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
        xse_version: xse,
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

/// The `name[256]` field at offset 8, which every extender's declaration has.
fn plugin_name(bytes: &[u8]) -> Result<String, String> {
    let name = bytes.get(8..264).ok_or("truncated plugin name")?;
    let end = name
        .iter()
        .position(|b| *b == 0)
        .ok_or("unterminated plugin name")?;
    if end == 0 {
        return Err("empty plugin name".into());
    }
    Ok(std::str::from_utf8(&name[..end])
        .map_err(|_| "invalid plugin name encoding")?
        .to_string())
}

/// The packed `compatibleVersions[16]` at `at`, up to the first zero.
fn packed_versions(bytes: &[u8], at: usize) -> Result<Vec<Version>, String> {
    let mut versions = Vec::new();
    for at in (at..at + 64).step_by(4) {
        let packed = dword(bytes, at)?;
        if packed == 0 {
            break;
        }
        versions.push(unpack(packed));
    }
    Ok(versions)
}

fn parse_declaration(bytes: &[u8]) -> Result<SkseDeclaration, String> {
    Ok(SkseDeclaration {
        data_version: dword(bytes, 0)?,
        plugin_version: dword(bytes, 4)?,
        name: plugin_name(bytes)?,
        version_independence_ex: dword(bytes, 772)?,
        version_independence: dword(bytes, 776)?,
        compatible_versions: packed_versions(bytes, 780)?,
        skse_required: unpack(dword(bytes, 844)?),
    })
}

/// `dataVersion` 0, `name` 8, `addressIndependence` 520, `structureIndependence`
/// 524, `compatibleVersions` 528, `reservedBreaking` 600 (f4se and sfse PluginAPI.h).
fn parse_xse_declaration(export: &'static str, bytes: &[u8]) -> Result<XseDeclaration, String> {
    Ok(XseDeclaration {
        export,
        data_version: dword(bytes, 0)?,
        name: plugin_name(bytes)?,
        address_independence: dword(bytes, 520)?,
        structure_independence: dword(bytes, 524)?,
        compatible_versions: packed_versions(bytes, 528)?,
        reserved_breaking: dword(bytes, 600)?,
    })
}

/// What a 64-bit Fallout 4 or Starfield loader accepts for one runtime, from
/// its `CheckPluginCompatibility`. The masks are `addressIndependence` and
/// `structureIndependence` bits.
#[derive(Debug, Clone, Copy)]
pub struct XseRules {
    pub export: &'static str,
    /// Any of these bits makes the plugin address independent.
    pub address_ok: u32,
    /// Any of these bits makes the plugin structure independent.
    pub structure_ok: u32,
    /// Any of these bits needs the Address Library file for the runtime.
    pub library_flags: u32,
    pub known_address: u32,
    pub known_structure: u32,
    pub library_name: fn(Version) -> String,
}

/// None for a game or runtime whose loader does not read the declaration
/// (old-gen Fallout 4 before 1.10.980, Starfield before 1.15.216).
pub fn xse_rules(game_id: &str, runtime: Version) -> Option<XseRules> {
    let fallout4 = |ok, library_flags| XseRules {
        export: "F4SEPlugin_Version",
        address_ok: ok,
        structure_ok: ok,
        library_flags,
        known_address: 0b111,
        known_structure: 0b111,
        library_name: fallout4_library_name,
    };
    match game_id {
        "fallout4" if runtime >= Version([1, 11, 137, 0]) => Some(fallout4(0b101, 0b100)),
        "fallout4" if runtime >= Version([1, 10, 980, 0]) => Some(fallout4(0b011, 0b010)),
        "starfield" if runtime >= Version([1, 15, 216, 0]) => Some(XseRules {
            export: "SFSEPlugin_Version",
            address_ok: 0b101,
            structure_ok: 0b1001,
            library_flags: 0b110,
            known_address: 0b111,
            known_structure: 0b1111,
            library_name: address_library_name,
        }),
        _ => None,
    }
}

/// Fallout 4's Address Library file for `runtime`: `version-1-10-984-0.bin`.
fn fallout4_library_name(runtime: Version) -> String {
    let [a, b, c, _] = runtime.0;
    format!("version-{a}-{b}-{c}-0.bin")
}

fn readable_file(path: &Path) -> bool {
    std::fs::File::open(path)
        .and_then(|f| f.metadata())
        .is_ok_and(|m| m.is_file() && m.len() > 0)
}

/// The PE metadata of an extender plugin built for `machine` (0x8664 or
/// 0x14c), or the `pe_unverified` or `pe_architecture` card.
fn plugin_pe(path: &Path, origin_mod: &str, machine: u16) -> Result<PeInfo, PreflightDiagnostic> {
    let card = |code, severity, detail| PreflightDiagnostic {
        code,
        severity,
        path: path.to_path_buf(),
        origin_mod: origin_mod.into(),
        detail,
    };
    let info = inspect_pe(path).map_err(|e| {
        card(
            "pe_unverified",
            DiagnosticSeverity::Unverified,
            format!("Could not inspect PE metadata: {e}. Compatibility is unverified."),
        )
    })?;
    if info.machine != machine {
        let bits = if machine == 0x14c { 32 } else { 64 };
        return Err(card(
            "pe_architecture",
            DiagnosticSeverity::Error,
            format!(
                "PE machine 0x{:04X} cannot load in a {bits}-bit game process.",
                info.machine
            ),
        ));
    }
    Ok(info)
}

/// Check a winning F4SE or SFSE plugin's declaration as the loader does for
/// `runtime`, without loading it. `address_library` is the resolved winning
/// library file for `runtime`, not a mod directory.
pub fn check_xse_plugin(
    path: &Path,
    origin_mod: &str,
    runtime: Version,
    rules: &XseRules,
    address_library: Option<&Path>,
) -> Vec<PreflightDiagnostic> {
    use DiagnosticSeverity::{Error, Unverified, Warning};
    let info = match plugin_pe(path, origin_mod, 0x8664) {
        Ok(info) => info,
        Err(card) => return vec![card],
    };
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
    let export = rules.export;
    let Some(v) = info.xse_version.filter(|v| v.export == export) else {
        add("xse_no_version_data", Warning, format!("No {export} export: the script extender skips this DLL. A plugin built for the old-gen game will not load; a helper library that is not a plugin is fine."));
        return out;
    };
    if v.data_version == 0 {
        add(
            "xse_bad_version_data",
            Error,
            format!("{export} has data version 0; the loader reports: disabled, bad version data."),
        );
        return out;
    }
    if v.data_version != 1 {
        add(
            "xse_declaration_unverified",
            Unverified,
            format!(
                "{export} data version {} is unsupported; compatibility is unverified.",
                v.data_version
            ),
        );
        return out;
    }
    if v.address_independence & !rules.known_address != 0
        || v.structure_independence & !rules.known_structure != 0
    {
        add(
            "xse_flags_unverified",
            Unverified,
            format!("Unknown compatibility flags in {export}; their meaning is unverified."),
        );
        return out;
    }
    if v.address_independence & rules.library_flags != 0
        && !address_library.is_some_and(readable_file)
    {
        // F4SEPlugin_Version: F4SE/Plugins.
        let dir = export.trim_end_matches("Plugin_Version");
        add(
            "address_library_missing",
            Error,
            format!(
                "The plugin requires a readable {} in the merged {dir}/Plugins directory.",
                (rules.library_name)(runtime)
            ),
        );
    }
    let independent = v.address_independence & rules.address_ok != 0
        && v.structure_independence & rules.structure_ok != 0
        && v.reserved_breaking == 0;
    if !independent && !v.compatible_versions.contains(&runtime) {
        let declared = if v.compatible_versions.is_empty() {
            "no compatible runtime".to_string()
        } else {
            v.compatible_versions
                .iter()
                .map(Version::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        };
        add(
            "runtime_mismatch",
            Error,
            format!("The plugin declares {declared}; the game is {runtime}."),
        );
    }
    out
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
                if !context.address_library.is_some_and(readable_file) {
                    add(
                        "address_library_missing",
                        Error,
                        format!(
                            "The plugin requires a readable {} in the merged SKSE/Plugins directory.",
                            address_library_name(runtime)
                        ),
                    );
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

    /// `fixture()` with an `F4SEPluginVersionData` layout (SFSE shares it)
    /// under `export` in place of the SKSE declaration.
    fn xse_fixture(export: &[u8; 18], addr: u32, structure: u32, versions: &[u32]) -> Vec<u8> {
        let mut b = fixture();
        b[0x250..0x262].copy_from_slice(export);
        b[0x300..0x300 + 848].fill(0);
        u32at(&mut b, 0x300, 1);
        b[0x308..0x30d].copy_from_slice(b"Test\0");
        u32at(&mut b, 0x300 + 520, addr);
        u32at(&mut b, 0x300 + 524, structure);
        for (i, v) in versions.iter().enumerate() {
            u32at(&mut b, 0x300 + 528 + i * 4, *v);
        }
        b
    }

    fn game(id: &str) -> &'static eidos_gamedef::GameDef {
        eidos_gamedef::builtin()
            .iter()
            .find(|g| g.id == id)
            .unwrap()
    }

    fn temp_tree(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "eidos-preflight-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn f4se_ng_plugin_on_ae_runtime_is_a_runtime_mismatch() {
        let f = Tmp::new(&xse_fixture(b"F4SEPlugin_Version", 2, 2, &[0x010A3D80]));
        let rules = xse_rules("fallout4", Version([1, 11, 137, 0])).unwrap();
        let d = check_xse_plugin(&f.0, "Old Mod", Version([1, 11, 137, 0]), &rules, None);
        assert!(
            d.iter().any(|d| d.code == "runtime_mismatch"
                && d.severity == DiagnosticSeverity::Error
                && d.detail.contains("1.10.984.0")
                && d.detail.contains("1.11.137.0")),
            "{d:?}"
        );
    }

    #[test]
    fn f4se_ae_plugin_with_the_library_flag_needs_the_versioned_bin() {
        let f = Tmp::new(&xse_fixture(b"F4SEPlugin_Version", 4, 4, &[]));
        let rules = xse_rules("fallout4", Version([1, 11, 137, 0])).unwrap();
        let d = check_xse_plugin(&f.0, "M", Version([1, 11, 137, 0]), &rules, None);
        assert!(d
            .iter()
            .any(|d| d.code == "address_library_missing"
                && d.detail.contains("version-1-11-137-0.bin")));
        let lib = Tmp::new(b"library");
        assert!(
            check_xse_plugin(&f.0, "M", Version([1, 11, 137, 0]), &rules, Some(&lib.0)).is_empty()
        );
    }

    #[test]
    fn f4se_ng_rules_apply_between_980_and_137() {
        let f = Tmp::new(&xse_fixture(b"F4SEPlugin_Version", 2, 2, &[]));
        let lib = Tmp::new(b"library");
        let rules = xse_rules("fallout4", Version([1, 10, 984, 0])).unwrap();
        assert!(
            check_xse_plugin(&f.0, "M", Version([1, 10, 984, 0]), &rules, Some(&lib.0)).is_empty()
        );
    }

    #[test]
    fn a_dll_without_the_export_is_skipped_by_the_extender() {
        let mut b = fixture();
        u32at(&mut b, 0x98 + 112, 0); // no export directory
        u32at(&mut b, 0x98 + 116, 0);
        let f = Tmp::new(&b);
        let rules = xse_rules("fallout4", Version([1, 10, 984, 0])).unwrap();
        let d = check_xse_plugin(&f.0, "M", Version([1, 10, 984, 0]), &rules, None);
        assert_eq!(
            (d[0].code, d[0].severity),
            ("xse_no_version_data", DiagnosticSeverity::Warning)
        );
        // Another extender's declaration is not the one this loader reads.
        let f = Tmp::new(&xse_fixture(b"SFSEPlugin_Version", 2, 2, &[]));
        let d = check_xse_plugin(&f.0, "M", Version([1, 10, 984, 0]), &rules, None);
        assert_eq!(d[0].code, "xse_no_version_data", "{d:?}");
    }

    #[test]
    fn unknown_xse_flag_bits_are_unverified() {
        let f = Tmp::new(&xse_fixture(b"F4SEPlugin_Version", 8, 1, &[]));
        let rules = xse_rules("fallout4", Version([1, 11, 137, 0])).unwrap();
        assert!(
            check_xse_plugin(&f.0, "M", Version([1, 11, 137, 0]), &rules, None)
                .iter()
                .any(|d| d.code == "xse_flags_unverified")
        );
    }

    #[test]
    fn xse_data_version_zero_is_rejected_and_other_versions_are_unverified() {
        let rules = xse_rules("fallout4", Version([1, 11, 137, 0])).unwrap();
        for (data_version, code, severity) in [
            (0, "xse_bad_version_data", DiagnosticSeverity::Error),
            (
                2,
                "xse_declaration_unverified",
                DiagnosticSeverity::Unverified,
            ),
        ] {
            let mut b = xse_fixture(b"F4SEPlugin_Version", 4, 4, &[]);
            u32at(&mut b, 0x300, data_version);
            let f = Tmp::new(&b);
            let d = check_xse_plugin(&f.0, "M", Version([1, 11, 137, 0]), &rules, None);
            assert_eq!(
                d.iter().map(|d| (d.code, d.severity)).collect::<Vec<_>>(),
                [(code, severity)]
            );
        }
    }

    #[test]
    fn sfse_v2_library_and_1_14_70_layout_are_independent() {
        let f = Tmp::new(&xse_fixture(b"SFSEPlugin_Version", 4, 8, &[]));
        let rules = xse_rules("starfield", Version([1, 15, 216, 0])).unwrap();
        let d = check_xse_plugin(&f.0, "M", Version([1, 15, 216, 0]), &rules, None);
        assert!(
            d.iter().any(|d| d.code == "address_library_missing"
                && d.detail.contains("versionlib-1-15-216-0.bin")),
            "{d:?}"
        );
        assert!(!d.iter().any(|d| d.code == "runtime_mismatch"), "{d:?}");
    }

    #[test]
    fn old_gen_runtime_reports_nothing_per_plugin() {
        let root = temp_tree("f4se");
        let plugins = root.join("Data/F4SE/Plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        let mut exe = fixture();
        u32at(&mut exe, 0x930, 0x0001000A);
        u32at(&mut exe, 0x934, 163 << 16);
        std::fs::write(root.join("Fallout4.exe"), &exe).unwrap();
        std::fs::write(root.join("f4se_loader.exe"), b"loader").unwrap();
        let mut dll = fixture();
        u32at(&mut dll, 0x98 + 112, 0);
        u32at(&mut dll, 0x98 + 116, 0);
        std::fs::write(plugins.join("a.dll"), dll).unwrap();
        let (data, ow) = (root.join("Data"), root.join("overwrite"));
        assert!(xse_rules("fallout4", Version([1, 10, 163, 0])).is_none());
        let old_gen = scan_extender_plugins(game("fallout4"), &data, &root, &[], &ow);
        // The same tree on 1.11.137, plus an AE plugin whose library is present.
        u32at(&mut exe, 0x930, 0x0001000B);
        u32at(&mut exe, 0x934, 137 << 16);
        std::fs::write(root.join("Fallout4.exe"), &exe).unwrap();
        let ae = xse_fixture(b"F4SEPlugin_Version", 4, 4, &[]);
        std::fs::write(plugins.join("b.dll"), ae).unwrap();
        std::fs::write(plugins.join("version-1-11-137-0.bin"), b"library").unwrap();
        let next_gen = scan_extender_plugins(game("fallout4"), &data, &root, &[], &ow);
        std::fs::remove_dir_all(&root).unwrap();
        assert!(old_gen.is_empty(), "{old_gen:?}");
        assert_eq!(next_gen.len(), 1, "{next_gen:?}");
        assert_eq!(next_gen[0].code, "xse_no_version_data");
        assert_eq!(next_gen[0].path, plugins.join("a.dll"));
        assert_eq!(next_gen[0].origin_mod, "Game");
    }

    #[test]
    fn a_64_bit_dll_in_a_32_bit_game_is_an_architecture_error() {
        let root = temp_tree("obse");
        let plugins = root.join("Data/OBSE/Plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(plugins.join("x.dll"), fixture()).unwrap();
        let mut x86 = fixture();
        u16at(&mut x86, 0x84, 0x14c);
        std::fs::write(plugins.join("y.dll"), x86).unwrap();
        let (data, ow) = (root.join("Data"), root.join("overwrite"));
        let d = scan_extender_plugins(game("oblivion"), &data, &root, &[], &ow);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(
            (d[0].code, d[0].severity),
            ("pe_architecture", DiagnosticSeverity::Error)
        );
        assert_eq!(d[0].path, plugins.join("x.dll"));
        std::fs::remove_file(plugins.join("x.dll")).unwrap();
        std::fs::write(plugins.join("z.dll"), b"MZ").unwrap();
        let d = scan_extender_plugins(game("falloutnv"), &data, &root, &[], &ow);
        assert!(d.is_empty(), "NVSE/Plugins is not OBSE/Plugins: {d:?}");
        let d = scan_extender_plugins(game("oblivion"), &data, &root, &[], &ow);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            d.iter().map(|d| (d.code, d.severity)).collect::<Vec<_>>(),
            [("pe_unverified", DiagnosticSeverity::Unverified)]
        );
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
        let game = eidos_gamedef::builtin()
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap();
        let found = scan_extender_plugins(game, &base.join("Data"), &base, &mods, &ow);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].code, "address_library_missing");
        assert_eq!(found[0].origin_mod, "Higher");
        assert_eq!(found[0].path, high.join("SKSE/Plugins/TEST.DLL"));
        std::fs::create_dir_all(ow.join("Root")).unwrap();
        std::fs::write(ow.join("Root/.eidoswh.skyrimse.exe"), []).unwrap();
        let found = scan_extender_plugins(game, &base.join("Data"), &base, &mods, &ow);
        assert!(found.iter().any(|d| d.code == "runtime_metadata_unverified"
            && d.severity == DiagnosticSeverity::Unverified));
        assert!(!found.iter().any(|d| d.code == "runtime_mismatch"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn every_extender_row_has_an_https_page() {
        for id in [
            "skyrim",
            "skyrimse",
            "enderalse",
            "skyrimvr",
            "fallout4",
            "fallout4vr",
            "starfield",
            "falloutnv",
            "fallout3",
            "oblivion",
        ] {
            assert!(extender(id).unwrap().page.starts_with("https://"), "{id}");
        }
    }

    #[test]
    fn extender_runtime_dll_names_follow_the_loader_convention() {
        assert_eq!(
            extender_runtime_dll("skse64", Version([1, 6, 1170, 0])),
            "skse64_1_6_1170.dll"
        );
        assert_eq!(extender("fallout4").unwrap().plugins_dir, "F4SE/Plugins");
        assert_eq!(extender("starfield").unwrap().runtime_dll, Some("sfse"));
        assert_eq!(extender("falloutnv").unwrap().runtime_dll, None);
        assert!(extender("stellarblade").is_none());
    }

    #[test]
    fn runtime_version_reads_the_winning_root_executable() {
        let root = std::env::temp_dir().join(format!(
            "eidos-preflight-ext-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let high = root.join("high");
        let ow = root.join("overwrite");
        let base = root.join("game");
        std::fs::create_dir_all(base.join("Data")).unwrap();
        std::fs::create_dir_all(high.join("Root")).unwrap();
        std::fs::create_dir_all(&ow).unwrap();
        std::fs::write(base.join("SkyrimSE.exe"), b"bad lower executable").unwrap();
        std::fs::write(high.join("Root/SKYRIMSE.EXE"), fixture()).unwrap();
        let mods = vec![("Downgrade".to_string(), high.clone())];
        let merged = merged_root(&base, &mods, &ow);
        assert_eq!(
            runtime_version(&merged, "SkyrimSE.exe"),
            Ok(Version([1, 7, 104, 0]))
        );
        assert_eq!(merged.origin(&high.join("Root/SKYRIMSE.EXE")), "Downgrade");
        // Same tree plus a loader and a plugin, but no skse64_1_7_104.dll in any layer.
        std::fs::write(high.join("Root/skse64_loader.exe"), b"loader").unwrap();
        std::fs::create_dir_all(high.join("SKSE/Plugins")).unwrap();
        std::fs::write(high.join("SKSE/Plugins/A.dll"), b"x").unwrap();
        let game = eidos_gamedef::builtin()
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap();
        let f = extender_facts(game, &base.join("Data"), &base, &mods, &ow).unwrap();
        assert!(f.loader_found);
        assert_eq!(f.plugin_dlls, 1);
        assert_eq!(f.runtime_dll.as_deref(), Some("skse64_1_7_104.dll"));
        assert!(!f.runtime_dll_found);
        std::fs::write(high.join("Root/skse64_1_7_104.dll"), b"rt").unwrap();
        let f = extender_facts(game, &base.join("Data"), &base, &mods, &ow).unwrap();
        assert!(f.runtime_dll_found);
        let stellar = eidos_gamedef::builtin()
            .iter()
            .find(|g| g.id == "stellarblade");
        if let Some(g) = stellar {
            assert!(extender_facts(g, &base.join("Data"), &base, &mods, &ow).is_none());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn merged_preflight_reports_unknown_loader_and_ignores_another_extenders_plugins() {
        let root = std::env::temp_dir().join(format!(
            "eidos-preflight-loader-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("Data/SKSE/Plugins")).unwrap();
        std::fs::write(root.join("Data/SKSE/Plugins/Test.dll"), fixture()).unwrap();
        std::fs::write(root.join("SkyrimSE.exe"), fixture()).unwrap();
        let game = eidos_gamedef::builtin()
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap();
        let found = scan_extender_plugins(
            game,
            &root.join("Data"),
            &root,
            &[],
            &root.join("overwrite"),
        );
        assert!(found.iter().any(|d| d.code == "skse_metadata_unverified"));
        let game = eidos_gamedef::builtin()
            .iter()
            .find(|g| g.id == "fallout4")
            .unwrap();
        assert!(scan_extender_plugins(
            game,
            &root.join("Data"),
            &root,
            &[],
            &root.join("overwrite")
        )
        .is_empty());
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
        assert!(check_skse_plugin(&f.0, "M", &context)
            .iter()
            .any(|d| d.code == "address_library_missing"));
        context.address_library = Some(&library.0);
        assert!(!check_skse_plugin(&f.0, "M", &context)
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error));
        u32at(&mut b, 0x300 + 776, 1);
        let f = Tmp::new(&b);
        assert!(check_skse_plugin(&f.0, "M", &context)
            .iter()
            .any(|d| d.code == "runtime_structures"));
    }
    #[test]
    fn malformed_and_truncated_files_never_claim_compatibility() {
        for bytes in [vec![], b"MZ".to_vec(), fixture()[..600].to_vec()] {
            let f = Tmp::new(&bytes);
            assert!(inspect_pe(&f.0).is_err());
            assert!(check_skse_plugin(&f.0, "M", &SkseContext::default())
                .iter()
                .any(|d| d.code == "pe_unverified"));
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
        assert!(check_skse_plugin(&f.0, "M", &SkseContext::default())
            .iter()
            .any(|d| d.code == "skse_declaration_unverified"));
        let mut b = fixture();
        u32at(&mut b, 0x300 + 772, 0x80000000);
        let f = Tmp::new(&b);
        assert!(check_skse_plugin(&f.0, "M", &SkseContext::default())
            .iter()
            .any(|d| d.code == "skse_flags_unverified"));
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
        assert!(check_skse_plugin(&f.0, "M", &context)
            .iter()
            .any(|d| d.code == "runtime_structures"));
        let context = SkseContext {
            runtime: Some(Version([1, 5, 97, 0])),
            ..context
        };
        assert!(check_skse_plugin(&f.0, "M", &context)
            .iter()
            .any(|d| d.code == "legacy_runtime_unverified"));
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
        assert!(check_skse_plugin(&f.0, "M", &context)
            .iter()
            .any(|d| d.code == "skse_too_old"));
        u16at(&mut b, 0x84, 0x14c);
        let f = Tmp::new(&b);
        assert!(check_skse_plugin(&f.0, "M", &context)
            .iter()
            .any(|d| d.code == "pe_architecture"));
    }

    /// Minimal BSA v105 holding one empty member; enough for the directory reader.
    fn bsa_with(member: &str) -> Vec<u8> {
        let (folder, name) = member.rsplit_once('/').unwrap();
        let start = 36 + 24;
        let names = start + 1 + folder.len() + 1 + 16;
        let payload = names + name.len() + 1;
        let mut b = vec![0; payload + 1];
        b[..4].copy_from_slice(b"BSA\0");
        for (at, n) in [
            (4, 105),
            (8, 36),
            (12, 3),
            (16, 1),
            (20, 1),
            (24, folder.len() as u32 + 1),
            (28, name.len() as u32 + 1),
            (44, 1),
        ] {
            u32at(&mut b, at, n);
        }
        let offset = (start + name.len() + 1) as u64;
        b[52..60].copy_from_slice(&offset.to_le_bytes());
        b[start] = (folder.len() + 1) as u8;
        b[start + 1..start + 1 + folder.len()].copy_from_slice(folder.as_bytes());
        let file_rec = start + folder.len() + 2;
        u32at(&mut b, file_rec + 8, 1);
        u32at(&mut b, file_rec + 12, payload as u32);
        b[names..names + name.len()].copy_from_slice(name.as_bytes());
        b
    }

    fn font_scan(config: &str, loose: &[&str], bsa: Option<&str>) -> Vec<PreflightDiagnostic> {
        let root = std::env::temp_dir().join(format!(
            "eidos-fontconfig-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let (m, ow, data) = (root.join("mod"), root.join("ow"), root.join("Data"));
        for d in [&m, &ow, &data] {
            std::fs::create_dir_all(d.join("Interface")).unwrap();
        }
        std::fs::write(m.join("Interface/fontconfig.txt"), config).unwrap();
        for f in loose {
            let f = m.join(f);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, b"swf").unwrap();
        }
        if let Some(member) = bsa {
            std::fs::write(data.join("Skyrim - Interface.bsa"), bsa_with(member)).unwrap();
        }
        let game = eidos_gamedef::builtin()
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap();
        let found = scan_fontconfig(game, &data, &[("Fonts".into(), m)], &ow);
        let _ = std::fs::remove_dir_all(&root);
        found
    }

    #[test]
    fn fontconfig_warns_for_a_font_found_nowhere() {
        // Real fontlib paths are Data-relative, and the vanilla fonts live in
        // Skyrim - Interface.bsa, which counts as present.
        let config = "fontlib \"Interface\\fonts_console.swf\"\nfontlib \"Interface\\gone.swf\"\n";
        let found = font_scan(config, &[], None);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "fontconfig_missing_font");
        assert_eq!(found[0].detail, "Interface\\gone.swf");
        assert_eq!(found[0].origin_mod, "Fonts");
    }

    #[test]
    fn fontconfig_accepts_loose_and_archived_fonts_and_crlf() {
        let text = "# comment\r\n\
            fontlib \"Interface\\Loose.swf\"\r\n\
            fontlib \"Interface\\Fonts\\Sub.swf\"\r\n\
            FONTLIB \"Interface\\fonts\\fonts_ru.swf\"\r\n\
            fontlib \"Interface/fonts_en.swf\"\r\n";
        let loose = ["Interface/loose.swf", "Interface/fonts/sub.swf"];
        let bsa = Some("Interface/Fonts/Fonts_RU.swf");
        assert!(font_scan(text, &loose, bsa).is_empty());
        // The same names one folder too deep are not the same files.
        let found = font_scan(text, &["Interface/Interface/loose.swf"], None);
        assert_eq!(
            found[0].detail,
            "Interface\\Loose.swf, Interface\\Fonts\\Sub.swf, Interface\\fonts\\fonts_ru.swf"
        );
    }

    #[test]
    fn an_archive_is_read_again_only_when_it_changes() {
        let f = Tmp::new(b"one");
        let memo = ArchiveMemo::default();
        let reads = std::cell::Cell::new(0);
        let read = || {
            reads.set(reads.get() + 1);
            reads.get()
        };
        assert_eq!(memoised(&memo, &f.0, read), 1);
        assert_eq!(memoised(&memo, &f.0, read), 1);
        std::fs::write(&f.0, b"three").unwrap();
        assert_eq!(memoised(&memo, &f.0, read), 2);
    }

    #[test]
    fn compressed_audio_warns_for_a_packed_wav_in_a_fallout4_ba2() {
        let root = std::env::temp_dir().join(format!("eidos-ba2-audio-{}", std::process::id()));
        let (m, ow) = (root.join("mod"), root.join("ow"));
        std::fs::create_dir_all(&m).unwrap();
        std::fs::create_dir_all(&ow).unwrap();
        let name = b"sound/a.wav";
        let mut b = vec![0u8; 24 + 36 + 1 + 2 + name.len()];
        b[..4].copy_from_slice(b"BTDX");
        b[4..8].copy_from_slice(&1u32.to_le_bytes());
        b[8..12].copy_from_slice(b"GNRL");
        b[12..16].copy_from_slice(&1u32.to_le_bytes());
        b[16..24].copy_from_slice(&61u64.to_le_bytes());
        b[40..48].copy_from_slice(&60u64.to_le_bytes());
        b[48..52].copy_from_slice(&1u32.to_le_bytes());
        b[52..56].copy_from_slice(&1u32.to_le_bytes());
        b[61..63].copy_from_slice(&(name.len() as u16).to_le_bytes());
        b[63..].copy_from_slice(name);
        std::fs::write(m.join("Voice - Main.ba2"), b).unwrap();
        let game = |id| {
            eidos_gamedef::builtin()
                .iter()
                .find(|g| g.id == id)
                .unwrap()
        };
        let mods = [("Voices".to_string(), m)];
        let found = scan_compressed_audio(game("fallout4"), &mods, &ow);
        let vr = scan_compressed_audio(game("fallout4vr"), &mods, &ow);
        let other = scan_compressed_audio(game("skyrimse"), &mods, &ow);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "ba2_compressed_audio");
        assert_eq!(found[0].detail, "Voice - Main.ba2: 1");
        assert_eq!(found[0].origin_mod, "Voices");
        assert_eq!(
            vr.len(),
            1,
            "Fallout 4 VR reads the same BA2 v1 GNRL archives"
        );
        assert!(other.is_empty());
    }
}
