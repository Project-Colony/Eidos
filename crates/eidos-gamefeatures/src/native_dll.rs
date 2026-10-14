//! Native Microsoft DLLs for Proton, the Linux-native analogue of MO2's forced
//! libraries.
//!
//! Some Skyrim graphics mods (Community Shaders, ENB, ReShade) IMPORT
//! `d3dcompiler_47.dll` - Microsoft's HLSL compiler - to compile shaders at
//! runtime. No mainstream Proton (Valve Proton, Proton-GE) ships the genuine MS
//! DLL: every flavour links `d3dcompiler_47.dll` to Wine's builtin
//! reimplementation, which those mods reject (so the plugin fails to load with
//! `D3DCOMPILER_47.dll not found`). Crucially, the mods do not *ship* the DLL -
//! they *import* it - so a "does a mod ship this file" check never fires for them.
//!
//! Eidos therefore scans enabled mods' DLLs' PE import tables for a
//! `d3dcompiler_47.dll` import, and some modding TOOLS declare the genuine MS
//! DirectX helpers they need (d3dcompiler_43 / d3dx9_43 / d3dx11_43). Eidos does
//! not ship any of these Microsoft binaries: they are winetricks verbs, installed
//! into the prefix by [`crate::install_tier2_verb`] on the user's consent (`eidos
//! prereqs --install`), with winetricks doing the arch-aware placement. At launch
//! Eidos only forces them native (`WINEDLLOVERRIDES=d3dcompiler_47=n,b`) and
//! checks, with [`native_dll_present`], whether the prefix actually has them.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use object::pe::{ImageNtHeaders32, ImageNtHeaders64};
use object::read::pe::{ImageNtHeaders, PeFile};
use object::LittleEndian as LE;

/// The Microsoft DirectX DLLs Eidos knows how to request, by winetricks verb
/// (also the on-disk file stem). d3dcompiler_47 fixes runtime HLSL compilation
/// (Community Shaders/ENB/ReShade); d3dcompiler_43 + d3dx9_43 + d3dx11_43 cover
/// the modding TOOLS that need the genuine helpers (BodySlide/Outfit Studio 3D
/// preview, DynDOLOD/TexGen, CAO texture work). Wine's builtin is either a broken
/// stub (d3dcompiler) or one some tools reject (d3dx).
pub(crate) const NATIVE_DLL_VERBS: &[&str] =
    &["d3dcompiler_47", "d3dcompiler_43", "d3dx9_43", "d3dx11_43"];

/// The DLL the graphics mods import; lower-cased for case-insensitive matching.
const D3DCOMPILER_47: &str = "d3dcompiler_47.dll";

/// Whether `verb` names one of the Microsoft DirectX DLLs that must be forced
/// native at launch once winetricks has installed it.
pub fn is_native_dll_verb(verb: &str) -> bool {
    NATIVE_DLL_VERBS.contains(&verb)
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
    if let Some(import_table) = file.import_table()? {
        let mut descriptors = import_table.descriptors()?;
        while let Some(descriptor) = descriptors.next()? {
            let name_rva = descriptor.name.get(LE);
            // Preserve other imports when one descriptor's name is malformed.
            if let Ok(name_bytes) = import_table.name(name_rva) {
                out.insert(String::from_utf8_lossy(name_bytes).to_ascii_lowercase());
            }
        }
    }
    // DELAY-LOADED imports count too: a plugin that binds d3dcompiler lazily
    // needs the DLL present just the same - the bind simply happens at first
    // call instead of at load, so missing it fails LATER and further from the
    // cause. The table is optional and often absent (measured on this machine:
    // 3 of 243 PEs have one at all); absence is not an error, and a malformed
    // table must not discard the classic imports already collected.
    if let Ok(Some(delay)) = file.delay_load_import_table() {
        if let Ok(mut descriptors) = delay.descriptors() {
            while let Ok(Some(descriptor)) = descriptors.next() {
                if let Ok(name_bytes) = delay.name(descriptor.dll_name_rva.get(LE)) {
                    out.insert(String::from_utf8_lossy(name_bytes).to_ascii_lowercase());
                }
            }
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
    let Ok(rd) = fs::read_dir(dir) else {
        return false;
    };
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
        if fs::metadata(&path)
            .map(|m| m.len() > MAX_SCAN_DLL_BYTES)
            .unwrap_or(true)
        {
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
    let Ok(rd) = fs::read_dir(install_path) else {
        return false;
    };
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
    roots
        .iter()
        .any(|r| ci_resolve(r, &COMMUNITY_SHADERS_REL).is_some())
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

/// Whether the prefix holds Microsoft's own `<verb>.dll`, not Wine's.
///
/// `windows_dir` is the prefix's `drive_c/windows`. A 64-bit (WoW64) prefix -
/// Proton's default, even for 32-bit games - needs the 64-bit DLL in `system32`
/// and the 32-bit one in `syswow64`; a pure 32-bit prefix has no `syswow64` and
/// keeps the 32-bit DLL in `system32`. Every place the prefix's layout calls for
/// must hold a native DLL.
///
/// Proton seeds these names as symlinks into its own `lib/wine/` builtins, and
/// Wine writes its builtin and placeholder DLLs with a signature at offset 0x40
/// (Proton tells them apart from natives the same way), so a file that is
/// absent, unreadable, not a PE, or signed by Wine reads as missing. A native
/// DLL Eidos copied in before it stopped shipping them counts as present.
pub fn native_dll_present(windows_dir: &Path, verb: &str) -> bool {
    let fname = format!("{verb}.dll");
    let syswow64 = windows_dir.join("syswow64");
    let mut dirs = vec![windows_dir.join("system32")];
    if syswow64.is_dir() {
        dirs.push(syswow64);
    }
    dirs.iter().all(|d| is_native_pe(&d.join(&fname)))
}

/// A PE file (following symlinks) that does not carry Wine's builtin or
/// placeholder signature.
fn is_native_pe(path: &Path) -> bool {
    let mut head = [0_u8; 0x40 + 20];
    let read = fs::File::open(path).and_then(|mut f| f.read_exact(&mut head));
    read.is_ok()
        && head.starts_with(b"MZ")
        && !head[0x40..].starts_with(b"Wine builtin DLL")
        && !head[0x40..].starts_with(b"Wine placeholder DLL")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "eidos-nd-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// A minimal DLL that imports `dll` by ordinal, through the classic import
    /// table or the delay-load one, as a PE32+ (`pe64`) or a PE32. One `.rdata`
    /// section maps file offset 512 to RVA 4096 and holds the descriptor (RVA
    /// 4096), the DLL name (4224) and the address and name thunks (4256, 4272).
    fn synthetic_pe(dll: &str, pe64: bool, delay: bool) -> Vec<u8> {
        let mut pe = vec![0_u8; 1024];
        let put16 =
            |pe: &mut Vec<u8>, at: usize, v: u16| pe[at..at + 2].copy_from_slice(&v.to_le_bytes());
        let put32 =
            |pe: &mut Vec<u8>, at: usize, v: u32| pe[at..at + 4].copy_from_slice(&v.to_le_bytes());
        pe[..2].copy_from_slice(b"MZ");
        put32(&mut pe, 60, 128);
        pe[128..132].copy_from_slice(b"PE\0\0");
        let (machine, opt_size, magic, flags, rva_count_at, dirs_at) = if pe64 {
            (0x8664, 240, 0x20b, 0x2022, 108, 112)
        } else {
            (0x14c, 224, 0x10b, 0x2102, 92, 96)
        };
        let opt = 152;
        put16(&mut pe, 132, machine);
        put16(&mut pe, 134, 1);
        put16(&mut pe, 148, opt_size);
        put16(&mut pe, 150, flags);
        put16(&mut pe, opt, magic);
        for (at, v) in [
            (32, 4096),
            (36, 512),
            (56, 8192),
            (60, 512),
            (rva_count_at, 16),
        ] {
            put32(&mut pe, opt + at, v);
        }
        // Data directory 1 is the import table, 13 the delay-load one.
        let dir = opt + dirs_at + if delay { 13 * 8 } else { 8 };
        put32(&mut pe, dir, 4096);
        put32(&mut pe, dir + 4, 64);
        let section = opt + opt_size as usize;
        pe[section..section + 8].copy_from_slice(b".rdata\0\0");
        for (at, v) in [(8, 512), (12, 4096), (16, 512), (20, 512), (36, 0x40000040)] {
            put32(&mut pe, section + at, v);
        }
        let descriptor: &[(usize, u32)] = if delay {
            // Attributes, DllNameRVA, ImportAddressTableRVA, ImportNameTableRVA.
            &[(512, 1), (516, 4224), (524, 4256), (528, 4272)]
        } else {
            // OriginalFirstThunk, Name, FirstThunk; the next 20 bytes stay zero
            // and end the table.
            &[(512, 4272), (524, 4224), (528, 4256)]
        };
        for &(at, v) in descriptor {
            put32(&mut pe, at, v);
        }
        pe[640..640 + dll.len()].copy_from_slice(dll.as_bytes());
        for at in [672, 688] {
            if pe64 {
                pe[at..at + 8].copy_from_slice(&0x8000_0000_0000_0001_u64.to_le_bytes());
            } else {
                put32(&mut pe, at, 0x8000_0001);
            }
        }
        pe
    }

    #[test]
    fn classic_imports_are_read_from_64_and_32_bit_pes() {
        // Named members, not just non-emptiness: a parser that returned a
        // non-empty but WRONG set would sail through an is_empty check, and the
        // launch override this feeds would silently stop matching - killing
        // Community Shaders / ENB at load with a green test suite.
        let dir = tmp_dir();
        for pe64 in [true, false] {
            let dll = dir.join(format!("classic{pe64}.dll"));
            fs::write(&dll, synthetic_pe("D3DCOMPILER_47.dll", pe64, false)).unwrap();
            let set = imported_dlls(&dll).unwrap();
            assert_eq!(
                set,
                BTreeSet::from([D3DCOMPILER_47.to_string()]),
                "pe64={pe64}"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delay_only_imports_trigger_the_native_dll_prerequisite() {
        let dir = tmp_dir();
        let dll = dir.join("delay.dll");
        fs::write(&dll, synthetic_pe("d3dcompiler_47.dll", true, true)).unwrap();
        assert!(imported_dlls(&dll).unwrap().contains(D3DCOMPILER_47));
        assert!(scan_imports_provisionable(&[dir.clone()]));
        fs::remove_dir_all(dir).unwrap();
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

    #[test]
    fn native_dll_verbs_are_the_directx_dlls() {
        for verb in ["d3dcompiler_47", "d3dcompiler_43", "d3dx9_43", "d3dx11_43"] {
            assert!(is_native_dll_verb(verb), "{verb}");
        }
        assert!(!is_native_dll_verb("vcrun2022"));
        assert!(!is_native_dll_verb("dotnet8"));
        // Engine Fixes ships its own d3dx9_42 preloader; Eidos never requests it.
        assert!(!is_native_dll_verb("d3dx9_42"));
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
        fs::write(
            nested.join("foo.dll"),
            synthetic_pe("rpcrt4.dll", true, false),
        )
        .unwrap();
        assert!(
            scan_imports_for(&[dir.clone()], "RPCRT4.dll"),
            "recursive scan must find it"
        );
        assert!(!scan_imports_for(
            &[dir.clone()],
            "nothing_imports_this_zzz.dll"
        ));
        assert!(!scan_imports_provisionable(&[dir.clone()]));
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

    /// What Wine writes for a builtin: a PE with its signature at offset 0x40.
    fn wine_builtin(tag: &[u8]) -> Vec<u8> {
        let mut pe = synthetic_pe("kernel32.dll", true, false);
        pe[0x40..0x40 + tag.len()].copy_from_slice(tag);
        pe
    }

    #[test]
    fn a_wow64_prefix_needs_the_native_dll_in_both_system_dirs() {
        let (win, s32, sw64) = win64_prefix();
        assert!(!native_dll_present(&win, "d3dx9_43"), "nothing there yet");
        fs::write(
            s32.join("d3dx9_43.dll"),
            synthetic_pe("kernel32.dll", true, false),
        )
        .unwrap();
        assert!(
            !native_dll_present(&win, "d3dx9_43"),
            "a 32-bit tool would still load Wine's builtin from syswow64"
        );
        fs::write(
            sw64.join("d3dx9_43.dll"),
            synthetic_pe("kernel32.dll", false, false),
        )
        .unwrap();
        assert!(native_dll_present(&win, "d3dx9_43"));
        let _ = fs::remove_dir_all(&win);
    }

    #[test]
    fn a_32bit_prefix_needs_it_in_system32_only() {
        let win = tmp_dir();
        let s32 = win.join("system32");
        fs::create_dir_all(&s32).unwrap();
        let dll = synthetic_pe("kernel32.dll", false, false);
        fs::write(s32.join("d3dcompiler_47.dll"), dll).unwrap();
        assert!(native_dll_present(&win, "d3dcompiler_47"));
        let _ = fs::remove_dir_all(&win);
    }

    #[test]
    fn wine_builtins_and_proton_symlinks_are_not_native() {
        use std::os::unix::fs::symlink;
        let (win, s32, sw64) = win64_prefix();
        // Proton seeds the name as a symlink into its shared builtins.
        let builtin = win.join("builtin_d3dcompiler_47.dll");
        fs::write(&builtin, wine_builtin(b"Wine builtin DLL")).unwrap();
        symlink(&builtin, s32.join("d3dcompiler_47.dll")).unwrap();
        symlink(&builtin, sw64.join("d3dcompiler_47.dll")).unwrap();
        assert!(!native_dll_present(&win, "d3dcompiler_47"));

        // A copied placeholder (a "fake DLL") is not native either.
        for d in [&s32, &sw64] {
            fs::remove_file(d.join("d3dcompiler_47.dll")).unwrap();
            fs::write(
                d.join("d3dcompiler_47.dll"),
                wine_builtin(b"Wine placeholder DLL"),
            )
            .unwrap();
        }
        assert!(!native_dll_present(&win, "d3dcompiler_47"));

        // Neither is a text file wearing the name.
        for d in [&s32, &sw64] {
            fs::write(d.join("d3dcompiler_47.dll"), b"not a PE").unwrap();
        }
        assert!(!native_dll_present(&win, "d3dcompiler_47"));
        let _ = fs::remove_dir_all(&win);
    }
}
