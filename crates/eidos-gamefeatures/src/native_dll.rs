//! Native Microsoft DLL provisioning for Proton, the Linux-native analogue of
//! MO2's forced libraries.
//!
//! Some Skyrim graphics mods (Community Shaders, ENB, ReShade) IMPORT
//! `d3dcompiler_47.dll` - Microsoft's HLSL compiler - to compile shaders at
//! runtime. No mainstream Proton (Valve Proton, Proton-GE) ships the genuine MS
//! DLL: every flavour links `d3dcompiler_47.dll` to Wine's builtin
//! reimplementation, which those mods reject (so the plugin fails to load with
//! `D3DCOMPILER_47.dll not found`). Crucially, the mods do not *ship* the DLL -
//! they *import* it - so a "does a mod ship this file" check never fires for them.
//!
//! Eidos therefore (1) scans enabled mods' DLLs' PE import tables for a
//! `d3dcompiler_47.dll` import, and if found (2) deploys the bundled genuine MS
//! redistributable into the prefix's `system32`/`syswow64`, where Wine's loader
//! finds it once forced native (`WINEDLLOVERRIDES=d3dcompiler_47=n,b`). Bundling
//! (rather than downloading at runtime) keeps Eidos self-contained, matching how
//! MO2 ships the same DLL in its `dlls/` folder. The same bundle-and-copy mechanism
//! also provisions the genuine MS DirectX helpers some modding TOOLS need
//! (d3dcompiler_43 / d3dx9_43 / d3dx11_43). See `assets/PROVENANCE.md`.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use object::pe::{ImageNtHeaders32, ImageNtHeaders64};
use object::read::pe::{ImageNtHeaders, ImportTable, PeFile};
use object::LittleEndian as LE;

/// A genuine Microsoft redistributable DLL Eidos bundles to drop into a Proton
/// prefix (the x86_64 build for `system32`, the i386 build for `syswow64`), because
/// Wine's builtin is either a broken stub (d3dcompiler) or one some tools reject
/// (d3dx). `verb` is the winetricks-style name, also the on-disk file stem.
struct NativeDll {
    verb: &'static str,
    x86_64: &'static [u8],
    i386: &'static [u8],
}

/// The bundled native DLLs. See `assets/PROVENANCE.md` for provenance + hashes.
/// d3dcompiler_47 fixes runtime HLSL compilation (Community Shaders/ENB/ReShade);
/// d3dcompiler_43 + d3dx9_43 + d3dx11_43 cover the modding TOOLS that need the
/// genuine MS DirectX helpers (BodySlide/Outfit Studio 3D preview, DynDOLOD/TexGen,
/// CAO texture work).
const NATIVE_DLLS: &[NativeDll] = &[
    NativeDll {
        verb: "d3dcompiler_47",
        x86_64: include_bytes!("../assets/d3dcompiler_47/x86_64.dll"),
        i386: include_bytes!("../assets/d3dcompiler_47/i386.dll"),
    },
    NativeDll {
        verb: "d3dcompiler_43",
        x86_64: include_bytes!("../assets/d3dcompiler_43/x86_64.dll"),
        i386: include_bytes!("../assets/d3dcompiler_43/i386.dll"),
    },
    NativeDll {
        verb: "d3dx9_43",
        x86_64: include_bytes!("../assets/d3dx9_43/x86_64.dll"),
        i386: include_bytes!("../assets/d3dx9_43/i386.dll"),
    },
    NativeDll {
        verb: "d3dx11_43",
        x86_64: include_bytes!("../assets/d3dx11_43/x86_64.dll"),
        i386: include_bytes!("../assets/d3dx11_43/i386.dll"),
    },
];

/// The DLL the graphics mods import; lower-cased for case-insensitive matching.
const D3DCOMPILER_47: &str = "d3dcompiler_47.dll";

/// Whether `verb` is a bundled native DLL Eidos can file-copy into a prefix (Tier 1),
/// as opposed to an installer verb (vcrun/dotnet) that must run winetricks (Tier 2).
pub fn is_tier1_dll(verb: &str) -> bool {
    NATIVE_DLLS.iter().any(|d| d.verb == verb)
}

/// Don't slurp an implausibly large `.dll` (a mislabelled archive) into memory
/// during the scan; real plugin DLLs are far smaller. The PE header + import table
/// sit near the front, but we read the whole file, so cap it.
const MAX_SCAN_DLL_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug)]
pub enum NativeDllError {
    Io(io::Error),
    /// The file is not a parseable PE (e.g. a text file with a `.dll` name).
    Parse(object::Error),
}

impl std::fmt::Display for NativeDllError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeDllError::Io(e) => write!(f, "{e}"),
            NativeDllError::Parse(e) => write!(f, "not a PE file: {e}"),
        }
    }
}

impl std::error::Error for NativeDllError {}

impl From<io::Error> for NativeDllError {
    fn from(e: io::Error) -> Self {
        NativeDllError::Io(e)
    }
}

impl From<object::Error> for NativeDllError {
    fn from(e: object::Error) -> Self {
        NativeDllError::Parse(e)
    }
}

/// The set of DLL names a PE file imports, lower-cased. `Err` if the file is not a
/// readable PE (32-bit or 64-bit).
pub fn imported_dlls(path: &Path) -> Result<BTreeSet<String>, NativeDllError> {
    let data = fs::read(path)?;
    // Try PE32+ (64-bit) first; a 32-bit PE fails the optional-header magic check
    // and falls through to the 32-bit parser. A non-PE fails both -> Parse error.
    match parse::<ImageNtHeaders64>(&data) {
        Ok(set) => Ok(set),
        Err(_) => Ok(parse::<ImageNtHeaders32>(&data)?),
    }
}

fn parse<Pe: ImageNtHeaders>(data: &[u8]) -> Result<BTreeSet<String>, object::Error> {
    let file: PeFile<Pe> = PeFile::parse(data)?;
    let mut out = BTreeSet::new();
    let import_table: ImportTable = match file.import_table()? {
        Some(t) => t,
        None => return Ok(out),
    };
    let mut descriptors = import_table.descriptors()?;
    while let Some(descriptor) = descriptors.next()? {
        let name_rva = descriptor.name.get(LE);
        // One malformed descriptor name must not discard the file's other imports
        // (a later, valid d3dcompiler_47 import), so skip a bad one rather than `?`.
        if let Ok(name_bytes) = import_table.name(name_rva) {
            out.insert(String::from_utf8_lossy(name_bytes).to_ascii_lowercase());
        }
    }
    Ok(out)
}

/// Whether any DLL under `roots` (recursively - graphics plugins live nested in
/// `SKSE/Plugins/`, not at the mod root) imports `d3dcompiler_47.dll`.
pub fn scan_imports_provisionable(roots: &[PathBuf]) -> bool {
    scan_imports_for(roots, D3DCOMPILER_47)
}

/// Whether any DLL under `roots` (recursively) imports `needle` (a DLL name).
/// Short-circuits on the first hit. A non-PE or unreadable `.dll` is skipped.
fn scan_imports_for(roots: &[PathBuf], needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    roots.iter().any(|r| walk_imports(r, &needle, 0))
}

/// Bounded (depth-capped, symlinked subdirs not followed - mod roots may be
/// symlinks but their listed contents are real) recursive search.
fn walk_imports(dir: &Path, needle: &str, depth: u32) -> bool {
    if depth > 8 {
        return false;
    }
    let Ok(rd) = fs::read_dir(dir) else { return false };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            // No-follow recursion: a symlinked subdir is skipped (avoids loops), while
            // a real subdir - e.g. SKSE/Plugins/ - is walked.
            if walk_imports(&e.path(), needle, depth + 1) {
                return true;
            }
            continue;
        }
        // A regular file OR a symlink to one (mod stores may link individual DLLs);
        // `fs::read`/`fs::metadata` below follow the link.
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        if !name.ends_with(".dll") {
            continue;
        }
        if fs::metadata(&path).map(|m| m.len() > MAX_SCAN_DLL_BYTES).unwrap_or(true) {
            continue;
        }
        if let Ok(imports) = imported_dlls(&path) {
            if imports.contains(needle) {
                return true;
            }
        }
    }
    false
}

// ---- ENB / Community Shaders coexistence advisory ------------------------------
//
// ENB and Community Shaders both inject into the Direct3D 11 pipeline. They can
// load together without crashing, but users routinely run both unknowingly and
// then chase visual artifacts or redundant post-processing. Eidos surfaces a soft,
// informational note before launch when it sees both - never a block.
//
// The two live in different places, so detection differs:
//   * ENB ships its `d3d11.dll` wrapper + `enbseries.ini`/`enblocal.ini` +
//     `enbseries/` in the GAME ROOT (next to the .exe), which is OUTSIDE Eidos's
//     Data-only mount - so we scan `install_path` directly.
//   * Community Shaders is an ordinary SKSE plugin under Data, shipped by a mod as
//     `SKSE/Plugins/CommunityShaders.dll` - so we scan the enabled mod roots.

/// Community Shaders' conventional location, relative to a Data-relative mod root.
const COMMUNITY_SHADERS_REL: [&str; 3] = ["SKSE", "Plugins", "CommunityShaders.dll"];

/// Whether an ENB is installed in the GAME ROOT (next to the executable).
///
/// Scans `install_path` itself - never a mod root, never a mounted layer, since
/// ENB lives outside Eidos's Data mount. Keys on the unambiguous `enb`-prefixed
/// markers (`enblocal.ini`, `enbseries.ini`, or the `enbseries/` folder), matched
/// case-insensitively. A bare `d3d11.dll` is deliberately NOT treated as ENB: it is
/// also how ReShade and other wrappers ship, and would mislabel them.
pub fn enb_in_game_root(install_path: &Path) -> bool {
    let Ok(rd) = fs::read_dir(install_path) else { return false };
    rd.flatten().any(|e| {
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        match name.as_str() {
            "enblocal.ini" | "enbseries.ini" => true,
            "enbseries" => e.file_type().map(|t| t.is_dir()).unwrap_or(false),
            _ => false,
        }
    })
}

/// Whether any of `roots` (enabled, Data-relative mod folders) ships the Community
/// Shaders SKSE plugin at `SKSE/Plugins/CommunityShaders.dll`, each path segment
/// matched case-insensitively (Windows-authored mods vary the casing of `SKSE`).
pub fn community_shaders_in_roots(roots: &[PathBuf]) -> bool {
    roots.iter().any(|r| ci_resolve(r, &COMMUNITY_SHADERS_REL).is_some())
}

/// The Wave-3d coexistence advisory: an ENB in the game root AND Community Shaders
/// in an enabled mod. Soft signal only - both can load together without crashing.
pub fn enb_cs_conflict(install_path: &Path, enabled_mod_roots: &[PathBuf]) -> bool {
    enb_in_game_root(install_path) && community_shaders_in_roots(enabled_mod_roots)
}

/// Resolve `rel` under `root`, matching each component case-insensitively. Returns
/// the real on-disk path if every component matches, else `None`. Cheap (one
/// `read_dir` per component); it resolves names rather than following symlinks.
fn ci_resolve(root: &Path, rel: &[&str]) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for comp in rel {
        let want = comp.to_ascii_lowercase();
        let hit = fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().to_ascii_lowercase() == want)?;
        cur = hit.path();
    }
    Some(cur)
}

/// Deploy a bundled native DLL (`verb`, e.g. "d3dx9_43") into a prefix, arch-aware.
/// `windows_dir` is the prefix's `drive_c/windows`. A 64-bit (WoW64) prefix - Proton's
/// default, even for 32-bit games - keeps 64-bit DLLs in `system32` and 32-bit in
/// `syswow64`, so we write both. A pure 32-bit prefix has no `syswow64` and keeps the
/// 32-bit DLL in `system32`. We branch on the prefix's actual layout, not the game's
/// bitness. Returns `Ok(false)` if `verb` is not a bundled DLL. Idempotent and
/// best-effort; returns whether anything was written.
pub fn ensure_native_dll(windows_dir: &Path, verb: &str) -> io::Result<bool> {
    let Some(dll) = NATIVE_DLLS.iter().find(|d| d.verb == verb) else {
        return Ok(false);
    };
    let fname = format!("{}.dll", dll.verb);
    let system32 = windows_dir.join("system32");
    let syswow64 = windows_dir.join("syswow64");
    if syswow64.is_dir() {
        let a = deploy_native(&system32.join(&fname), dll.x86_64)?;
        let b = deploy_native(&syswow64.join(&fname), dll.i386)?;
        Ok(a || b)
    } else {
        // 32-bit prefix: the 32-bit DLL is the one the game loads, from system32.
        deploy_native(&system32.join(&fname), dll.i386)
    }
}

/// Convenience for the game path: ensure the native HLSL compiler (the mod
/// import-scan trigger). Equivalent to `ensure_native_dll(windows_dir, "d3dcompiler_47")`.
pub fn ensure_d3dcompiler_47(windows_dir: &Path) -> io::Result<bool> {
    ensure_native_dll(windows_dir, "d3dcompiler_47")
}

/// Write `bytes` to `target`, handling the three states an existing entry can be
/// in. Returns whether the file was (re)written.
fn deploy_native(target: &Path, bytes: &[u8]) -> io::Result<bool> {
    match fs::symlink_metadata(target) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                // Proton seeds d3dcompiler_47.dll as a SYMLINK into its shared
                // lib/wine/ builtin. Remove the LINK first - never write through it,
                // or we would corrupt Proton's install for every other prefix.
                fs::remove_file(target)?;
            } else if meta.len() == bytes.len() as u64 && same_contents(target, bytes)? {
                // Genuine native already in place (e.g. a prior run, or winetricks).
                return Ok(false);
            } else {
                // A different real file (Wine builtin copy, or an older native):
                // keep ONE real backup. Only treat a regular file at the .eidos-bak
                // path as an existing backup - a directory/symlink/other there must
                // not let us delete the user's original without preserving it.
                let bak = backup_path(target);
                let have_backup =
                    fs::symlink_metadata(&bak).map(|m| m.file_type().is_file()).unwrap_or(false);
                if have_backup {
                    fs::remove_file(target)?;
                } else {
                    let _ = fs::remove_dir_all(&bak);
                    let _ = fs::remove_file(&bak);
                    fs::rename(target, &bak)?;
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, bytes)?;
    Ok(true)
}

/// `<path>.eidos-bak` (appended, so `foo.dll` -> `foo.dll.eidos-bak`).
fn backup_path(target: &Path) -> PathBuf {
    let mut s: OsString = target.as_os_str().to_os_string();
    s.push(".eidos-bak");
    PathBuf::from(s)
}

fn same_contents(path: &Path, bytes: &[u8]) -> io::Result<bool> {
    Ok(fs::read(path)? == bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("eidos-nd-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn enb_detected_by_enb_markers_only() {
        let root = tmp_dir();
        assert!(!enb_in_game_root(&root)); // empty game root

        // A bare d3d11.dll must NOT register as ENB (ReShade / other wrappers ship it).
        fs::write(root.join("d3d11.dll"), b"x").unwrap();
        assert!(!enb_in_game_root(&root));

        // enblocal.ini (case-insensitive) is the unambiguous ENB marker.
        fs::write(root.join("ENBLocal.ini"), b"x").unwrap();
        assert!(enb_in_game_root(&root));
        let _ = fs::remove_dir_all(&root);

        // The enbseries/ folder alone also counts.
        let root2 = tmp_dir();
        fs::create_dir_all(root2.join("enbseries")).unwrap();
        assert!(enb_in_game_root(&root2));
        let _ = fs::remove_dir_all(&root2);
    }

    #[test]
    fn community_shaders_detected_case_insensitively() {
        let m = tmp_dir();
        assert!(!community_shaders_in_roots(&[m.clone()]));
        // Windows-cased segments must still match on a case-sensitive fs.
        fs::create_dir_all(m.join("skse/Plugins")).unwrap();
        fs::write(m.join("skse/Plugins/communityshaders.dll"), b"x").unwrap();
        assert!(community_shaders_in_roots(&[m.clone()]));
        let _ = fs::remove_dir_all(&m);
    }

    #[test]
    fn conflict_needs_both_sides() {
        let root = tmp_dir();
        let modr = tmp_dir();
        fs::write(root.join("enbseries.ini"), b"x").unwrap();
        // Only ENB present -> no advisory.
        assert!(!enb_cs_conflict(&root, &[modr.clone()]));
        // Add Community Shaders -> advisory fires.
        fs::create_dir_all(modr.join("SKSE/Plugins")).unwrap();
        fs::write(modr.join("SKSE/Plugins/CommunityShaders.dll"), b"x").unwrap();
        assert!(enb_cs_conflict(&root, &[modr.clone()]));
        // CS without ENB -> no advisory.
        let empty_root = tmp_dir();
        assert!(!enb_cs_conflict(&empty_root, &[modr.clone()]));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&modr);
        let _ = fs::remove_dir_all(&empty_root);
    }

    /// Look up a bundled blob by verb + arch (x86_64 when `x64`, else i386).
    fn blob(verb: &str, x64: bool) -> &'static [u8] {
        let d = NATIVE_DLLS.iter().find(|d| d.verb == verb).unwrap();
        if x64 {
            d.x86_64
        } else {
            d.i386
        }
    }

    #[test]
    fn embedded_blobs_are_the_vendored_native_dlls() {
        // Catches a truncated/replaced vendor blob at test time (sizes from PROVENANCE.md).
        let sizes = [
            ("d3dcompiler_47", 4_691_496usize, 3_657_992usize),
            ("d3dcompiler_43", 2_526_056, 2_106_216),
            ("d3dx9_43", 2_401_112, 1_998_168),
            ("d3dx11_43", 276_832, 248_672),
        ];
        assert_eq!(NATIVE_DLLS.len(), sizes.len());
        for (verb, x64, x86) in sizes {
            assert_eq!(blob(verb, true).len(), x64, "{verb} x86_64 size");
            assert_eq!(blob(verb, false).len(), x86, "{verb} i386 size");
            assert_eq!(&blob(verb, true)[..2], b"MZ", "{verb} x86_64 MZ");
            assert_eq!(&blob(verb, false)[..2], b"MZ", "{verb} i386 MZ");
        }
    }

    #[test]
    fn tier1_classifier_knows_the_bundled_verbs() {
        assert!(is_tier1_dll("d3dcompiler_47"));
        assert!(is_tier1_dll("d3dx9_43"));
        assert!(is_tier1_dll("d3dx11_43"));
        assert!(!is_tier1_dll("vcrun2022")); // Tier 2 (winetricks installer)
        assert!(!is_tier1_dll("dotnet8"));
    }

    #[test]
    fn ensure_native_dll_deploys_a_d3dx_verb() {
        let (win, s32, sw64) = win64_prefix();
        assert!(ensure_native_dll(&win, "d3dx9_43").unwrap());
        assert_eq!(fs::read(s32.join("d3dx9_43.dll")).unwrap(), blob("d3dx9_43", true));
        assert_eq!(fs::read(sw64.join("d3dx9_43.dll")).unwrap(), blob("d3dx9_43", false));
        // An unknown verb is a clean no-op, not an error.
        assert!(!ensure_native_dll(&win, "vcrun2022").unwrap());
        let _ = fs::remove_dir_all(&win);
    }

    #[test]
    fn parses_imports_of_a_real_64bit_pe() {
        let dir = tmp_dir();
        let dll = dir.join("native.dll");
        fs::write(&dll, blob("d3dcompiler_47", true)).unwrap();
        let set = imported_dlls(&dll).unwrap();
        // Named members, not just non-emptiness: a parser that returned a
        // non-empty but WRONG set would sail through an is_empty check, and the
        // caller this feeds (`ensure_d3dcompiler_47`) would silently stop
        // matching - killing Community Shaders / ENB at load with a green test
        // suite. These names are what the real blob imports (checked against the
        // bytes, not assumed: the 64-bit build imports API sets and rpcrt4, and
        // notably does NOT import kernel32.dll directly).
        assert!(set.contains("rpcrt4.dll"), "{set:?}");
        assert!(set.contains("api-ms-win-core-file-l1-1-0.dll"), "{set:?}");
        assert!(set.len() >= 30, "the real import table has 33 entries: {set:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_imports_of_a_real_32bit_pe() {
        let dir = tmp_dir();
        let dll = dir.join("native32.dll");
        fs::write(&dll, blob("d3dcompiler_47", false)).unwrap();
        let set = imported_dlls(&dll).unwrap();
        // Same reasoning as the 64-bit test; the 32-bit build links classically.
        assert!(set.contains("kernel32.dll"), "{set:?}");
        assert!(set.contains("msvcrt.dll"), "{set:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_pe_file_errors_without_panicking() {
        let dir = tmp_dir();
        let bogus = dir.join("text.dll");
        fs::write(&bogus, b"this is not a PE file").unwrap();
        // Either an error or an empty set is acceptable - but never a panic and
        // never a false positive.
        match imported_dlls(&bogus) {
            Ok(set) => assert!(!set.contains(D3DCOMPILER_47)),
            Err(_) => {}
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_finds_an_import_and_recurses_into_subdirs() {
        let dir = tmp_dir();
        // Nest the DLL like a real SKSE plugin: <mod>/SKSE/Plugins/foo.dll.
        let nested = dir.join("SKSE/Plugins");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("foo.dll"), blob("d3dcompiler_47", true)).unwrap();

        // The bundled DLL imports a known set; pick any real import as the needle so
        // the test does not hard-code a specific Windows DLL name.
        let any_import =
            imported_dlls(&nested.join("foo.dll")).unwrap().into_iter().next().unwrap();
        assert!(scan_imports_for(&[dir.clone()], &any_import), "recursive scan must find it");
        assert!(!scan_imports_for(&[dir.clone()], "nothing_imports_this_zzz.dll"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_empty_dir_is_false() {
        let dir = tmp_dir();
        assert!(!scan_imports_for(&[dir.clone()], D3DCOMPILER_47));
        let _ = fs::remove_dir_all(&dir);
    }

    /// A `drive_c/windows` with a `syswow64` dir = a 64-bit (WoW64) prefix.
    fn win64_prefix() -> (PathBuf, PathBuf, PathBuf) {
        let win = tmp_dir();
        let s32 = win.join("system32");
        let sw64 = win.join("syswow64");
        fs::create_dir_all(&s32).unwrap();
        fs::create_dir_all(&sw64).unwrap();
        (win, s32, sw64)
    }

    #[test]
    fn ensure_deploys_then_is_idempotent() {
        let (win, s32, sw64) = win64_prefix();

        assert!(ensure_d3dcompiler_47(&win).unwrap(), "first run writes");
        assert_eq!(fs::read(s32.join("d3dcompiler_47.dll")).unwrap(), blob("d3dcompiler_47", true));
        assert_eq!(fs::read(sw64.join("d3dcompiler_47.dll")).unwrap(), blob("d3dcompiler_47", false));

        assert!(!ensure_d3dcompiler_47(&win).unwrap(), "second run is a no-op");
        let _ = fs::remove_dir_all(&win);
    }

    #[test]
    fn ensure_32bit_prefix_writes_i386_into_system32_only() {
        // No syswow64 -> a pure win32 prefix: the 32-bit DLL goes into system32.
        let win = tmp_dir();
        let s32 = win.join("system32");
        fs::create_dir_all(&s32).unwrap();

        assert!(ensure_d3dcompiler_47(&win).unwrap());
        assert_eq!(fs::read(s32.join("d3dcompiler_47.dll")).unwrap(), blob("d3dcompiler_47", false));
        assert!(!win.join("syswow64").exists(), "must not create a syswow64");
        let _ = fs::remove_dir_all(&win);
    }

    #[test]
    fn ensure_replaces_a_builtin_symlink_without_writing_through() {
        use std::os::unix::fs::symlink;
        let (win, s32, sw64) = win64_prefix();

        // A junk "builtin" target the prefix symlink points at (Proton's shared dll).
        let builtin = win.join("builtin_d3dcompiler_47.dll");
        fs::write(&builtin, b"PRETEND WINE BUILTIN - MUST NOT BE OVERWRITTEN").unwrap();
        symlink(&builtin, s32.join("d3dcompiler_47.dll")).unwrap();
        symlink(&builtin, sw64.join("d3dcompiler_47.dll")).unwrap();

        assert!(ensure_d3dcompiler_47(&win).unwrap());
        // The targets are now real files = our native, and the symlink was not
        // written through into the shared builtin.
        let placed = s32.join("d3dcompiler_47.dll");
        assert!(!fs::symlink_metadata(&placed).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&placed).unwrap(), blob("d3dcompiler_47", true));
        assert_eq!(fs::read(&builtin).unwrap(), b"PRETEND WINE BUILTIN - MUST NOT BE OVERWRITTEN");
        let _ = fs::remove_dir_all(&win);
    }

    #[test]
    fn ensure_backs_up_a_displaced_real_file() {
        let (win, s32, _sw64) = win64_prefix();
        fs::write(s32.join("d3dcompiler_47.dll"), b"old builtin copy").unwrap();

        assert!(ensure_d3dcompiler_47(&win).unwrap());
        assert_eq!(fs::read(s32.join("d3dcompiler_47.dll")).unwrap(), blob("d3dcompiler_47", true));
        assert_eq!(fs::read(s32.join("d3dcompiler_47.dll.eidos-bak")).unwrap(), b"old builtin copy");
        let _ = fs::remove_dir_all(&win);
    }

    #[test]
    fn ensure_does_not_destroy_original_when_bak_path_is_a_directory() {
        // The fs-safety review case: a stray directory at the .eidos-bak path must not
        // let us delete the user's real file without preserving it.
        let (win, s32, _sw64) = win64_prefix();
        let target = s32.join("d3dcompiler_47.dll");
        fs::write(&target, b"USER ORIGINAL").unwrap();
        fs::create_dir_all(s32.join("d3dcompiler_47.dll.eidos-bak")).unwrap();

        assert!(ensure_d3dcompiler_47(&win).unwrap());
        assert_eq!(fs::read(&target).unwrap(), blob("d3dcompiler_47", true));
        // The original was preserved (the stray dir was cleared and replaced by the
        // real backup), never silently lost.
        assert_eq!(fs::read(s32.join("d3dcompiler_47.dll.eidos-bak")).unwrap(), b"USER ORIGINAL");
        let _ = fs::remove_dir_all(&win);
    }
}
