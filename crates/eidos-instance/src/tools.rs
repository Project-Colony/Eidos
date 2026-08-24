//! Per-instance tool list (MO2's `ExecutablesList`): the executables a user runs
//! through the merged view besides the game - xEdit, FNIS, BodySlide, Wrye Bash.
//!
//! Mirrors MO2's model: each `Executable` is `{title, binary, arguments,
//! workingDirectory}`; the list shown to the user merges per-game defaults
//! (MO2: the game plugin's `executables()`; Eidos: seeded from `GameDef`) with
//! the user's own entries, user entries winning on a title collision. Stored as
//! `<instance>/tools.ini`, one section per tool - Eidos's own file, so a plain
//! rewrite (no MO2 byte-preservation needed).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One tool the user can run through the merged view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// Display name and lookup key (case-insensitive), e.g. `SSEEdit`.
    pub title: String,
    /// The Windows executable: absolute, or relative to the game install dir.
    pub exe: PathBuf,
    /// Extra command-line arguments.
    pub args: Vec<String>,
    /// Working directory; `None` = the executable's own directory (MO2 default).
    pub workdir: Option<PathBuf>,
    /// Runtime prerequisites to ensure in the prefix before this tool runs
    /// (winetricks-style verbs, e.g. `["dotnet8", "vcrun2022"]` for Synthesis, or
    /// `["d3dx9_43", "d3dcompiler_47"]` for BodySlide's 3D preview). Empty for the
    /// Delphi tools (xEdit/SSEEdit) that need nothing extra.
    pub prereqs: Vec<String>,
    /// A mod to capture this tool's output into, instead of leaving it in the
    /// Overwrite (MO2's "Create files in mod instead of overwrite").
    ///
    /// A folder name under `mods/`, never a path: it is validated on read so a
    /// hand-edited `tools.ini` cannot aim the capture outside the instance.
    pub output_mod: Option<String>,
}

/// Whether a string names a mod FOLDER and nothing else: no separators, no
/// traversal, not empty, no control characters.
///
/// The same guard `overwrite_into_mod` applies, hoisted so `tools.ini` cannot
/// carry a value the capture would later have to refuse - a tool configured with
/// a bad target would otherwise appear to work until the first run.
pub fn is_mod_folder_name(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty()
        && n != "."
        && n != ".."
        && !n.contains('/')
        && !n.contains('\\')
        && !n.chars().any(char::is_control)
}

/// Read `tools.ini`. A missing file is an empty list.
pub fn read_tools(path: &Path) -> Vec<Tool> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out: Vec<Tool> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = eidos_ini::section_header(line) {
            if let Some(title) = section.strip_prefix("Tool/") {
                out.push(Tool {
                    title: title.to_string(),
                    exe: PathBuf::new(),
                    args: Vec::new(),
                    workdir: None,
                    prereqs: Vec::new(),
                    output_mod: None,
                });
            }
            continue;
        }
        let (Some(tool), Some((k, v))) = (out.last_mut(), eidos_ini::key_value(line)) else {
            continue;
        };
        let v = v.trim();
        match k {
            "exe" => tool.exe = PathBuf::from(v),
            // One key per argument, written in order as arg0=, arg1=, ... (file
            // order == argument order). Lossless for arguments containing spaces.
            _ if k.len() > 3 && k.starts_with("arg") && k.as_bytes()[3..].iter().all(u8::is_ascii_digit) => {
                tool.args.push(v.to_string());
            }
            // Legacy single-line form (pre-per-key tools.ini): space-split.
            "args" if !v.is_empty() && tool.args.is_empty() => {
                tool.args = v.split(' ').map(String::from).collect();
            }
            "workdir" if !v.is_empty() => tool.workdir = Some(PathBuf::from(v)),
            // Comma-separated prerequisite verbs (names are `[a-z0-9_]`, no escaping).
            "prereqs" if !v.is_empty() => {
                tool.prereqs =
                    v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
            }
            // A mods/ folder NAME. Rejected here rather than at the capture, so
            // a hand-edited or migrated file cannot point the move at `..`, at
            // an absolute path, or at the mods directory itself.
            "output_mod" if is_mod_folder_name(v) => tool.output_mod = Some(v.to_string()),
            _ => {}
        }
    }
    out.retain(|t| !t.exe.as_os_str().is_empty());
    out
}

/// Write the full `tools.ini`.
pub fn write_tools(path: &Path, tools: &[Tool]) -> io::Result<()> {
    let mut s = String::new();
    for t in tools {
        // A control char or empty title cannot round-trip the `[Tool/<title>]`
        // header (it would split the section across lines or vanish on re-read,
        // re-attaching this tool's keys to the previous one), so skip such an
        // entry rather than corrupt the whole file. The `add` command rejects
        // these up front; this guards a hand-edited or migrated bad entry.
        let title = t.title.trim();
        if title.is_empty() || title.chars().any(char::is_control) {
            continue;
        }
        s.push_str(&format!("[Tool/{title}]\n"));
        s.push_str(&format!("exe={}\n", t.exe.display()));
        // One key per argument: an argument may itself contain spaces (e.g. xEdit's
        // `-D:D:\My Mods\Data`), which a single space-joined `args=` line would
        // corrupt on the next read. File order == argument order.
        for (i, a) in t.args.iter().enumerate() {
            s.push_str(&format!("arg{i}={a}\n"));
        }
        if let Some(w) = &t.workdir {
            s.push_str(&format!("workdir={}\n", w.display()));
        }
        if !t.prereqs.is_empty() {
            s.push_str(&format!("prereqs={}\n", t.prereqs.join(",")));
        }
        if let Some(m) = t.output_mod.as_deref().filter(|m| is_mod_folder_name(m)) {
            s.push_str(&format!("output_mod={m}\n"));
        }
        s.push('\n');
    }
    // Atomic (unique tmp + rename), like the profile writers: this rewrites the
    // WHOLE list on every save, and a crash mid-write would take the user's
    // custom arguments and workdirs with it.
    crate::write_atomic(path, s.as_bytes())
}

/// Merge per-game defaults with the user's tools: user entries first and they
/// win a title collision (case-insensitive), like MO2's `ExecutablesList::load`.
pub fn merge_tools(user: Vec<Tool>, defaults: Vec<Tool>) -> Vec<Tool> {
    let mut out = user;
    for d in defaults {
        if !out.iter().any(|t| t.title.eq_ignore_ascii_case(&d.title)) {
            out.push(d);
        }
    }
    out
}

/// The per-game executables Eidos auto-detects as default tools, mirroring MO2's
/// game-plugin `executables()`: the script-extender loader, the vanilla launcher,
/// and the game's own binary. Decoupled inputs (no GameDef dependency) so both the
/// CLI and the GUI can build it. Any empty / `None` field is skipped.
#[derive(Debug, Clone, Copy)]
pub struct GameExecutables<'a> {
    /// Display name of the game, used to title the launcher + binary entries.
    pub game_name: &'a str,
    /// The vanilla launcher Steam runs, e.g. `SkyrimSELauncher.exe`.
    pub launcher: Option<&'a str>,
    /// The game's own binary, e.g. `SkyrimSE.exe`.
    pub binary: Option<&'a str>,
    /// The script-extender loader, e.g. `skse64_loader.exe`.
    pub script_extender: Option<&'a str>,
}

/// Add a default tool for `exe` only if its file is present in `install` (detection
/// is by file existence, like MO2's game plugins).
fn push_tool_if_present(v: &mut Vec<Tool>, search: &[PathBuf], title: String, exe: &str) {
    if !exe.is_empty() && search.iter().any(|d| d.join(exe).is_file()) {
        v.push(Tool {
            prereqs: default_prereqs(&title),
            title,
            exe: PathBuf::from(exe),
            args: Vec::new(),
            workdir: None,
            output_mod: None,
        });
    }
}

/// The default tool list for a game, auto-detected by file existence in `install`
/// (MO2's game-plugin `executables()`): the script extender (the usual play
/// target) first, then the vanilla launcher, then the bare game binary - each ONLY
/// when its file is actually present. Because this is re-checked on every load, a
/// tool installed later (e.g. SKSE after the instance was created) appears on the
/// next load with no user action, exactly like MO2.
pub fn default_tools(execs: GameExecutables, install: &Path) -> Vec<Tool> {
    default_tools_in(execs, install, &[])
}

/// [`default_tools`], but also looking in `root_layers` - the `Root/` directories
/// of enabled mods, which is where a script extender installed AS A MOD lives.
///
/// The stored path stays relative to the game root: detection runs outside the
/// mount, but at launch the root union projects those same files onto the game
/// directory, so `skse64_loader.exe` resolves there for real. Without this a
/// Root-provided script extender would never appear as a tool at all.
pub fn default_tools_in(
    execs: GameExecutables,
    install: &Path,
    root_layers: &[PathBuf],
) -> Vec<Tool> {
    let search: Vec<PathBuf> =
        std::iter::once(install.to_path_buf()).chain(root_layers.iter().cloned()).collect();
    let install = &search;
    let mut v = Vec::new();
    if let Some(loader) = execs.script_extender.filter(|s| !s.is_empty()) {
        push_tool_if_present(&mut v, install, loader.trim_end_matches(".exe").to_string(), loader);
    }
    if let Some(launcher) = execs.launcher.filter(|s| !s.is_empty()) {
        let title = if execs.game_name.is_empty() {
            launcher.trim_end_matches(".exe").to_string()
        } else {
            format!("{} Launcher", execs.game_name)
        };
        push_tool_if_present(&mut v, install, title, launcher);
    }
    if let Some(binary) = execs.binary.filter(|s| !s.is_empty()) {
        let title = if execs.game_name.is_empty() {
            binary.trim_end_matches(".exe").to_string()
        } else {
            execs.game_name.to_string()
        };
        push_tool_if_present(&mut v, install, title, binary);
    }
    v
}

/// The known runtime prerequisites for a well-known modding tool, by title (so a
/// freshly-added SSEEdit/Synthesis/BodySlide gets the right verbs without the user
/// typing them; a user-declared `prereqs=` in tools.ini always wins). Verbs match
/// the bundled DirectX DLLs (Tier 1) and the winetricks installer verbs (Tier 2).
pub fn default_prereqs(title: &str) -> Vec<String> {
    let t = title.to_ascii_lowercase();
    let has = |needle: &str| t.contains(needle);
    let verbs: &[&str] = if has("synthesis") {
        &["dotnet8", "vcrun2022"]
    } else if has("pandora") {
        &["dotnetdesktop8"]
    } else if has("fnis") {
        &["dotnet48"]
    } else if has("bodyslide") || has("outfit") {
        &["d3dx9_43", "d3dcompiler_47"]
    } else if has("dyndolod") || has("texgen") || has("xlodgen") {
        // `dotnet10` is not decoration. These tools shell out to LODGen to build
        // object LOD, and the LODGen that works under Proton is the .NET 10
        // build - the .NET Framework one is routed to Wine's Mono, whose
        // `System.Uri` initialiser is missing a method it calls, so it dies
        // before its first line of work with a 214-byte log and no error.
        &["d3dcompiler_47", "d3dx9_43", "d3dx11_43", "dotnet10"]
    } else if has("cathedral") || has("cao") {
        &["vcrun2022", "d3dcompiler_47", "d3dx11_43"]
    } else if has("nemesis") || has("loot") {
        &["vcrun2022"]
    } else {
        // xEdit/SSEEdit/TES5Edit/FO4Edit + script extenders: nothing extra.
        &[]
    };
    verbs.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("eidos-tools-{}-{}.ini", std::process::id(), n))
    }

    fn tmp_dir() -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("eidos-tools-dir-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn default_tools_detects_present_executables_and_picks_up_new_ones() {
        let dir = tmp_dir();
        let execs = GameExecutables {
            game_name: "Skyrim Special Edition",
            launcher: Some("SkyrimSELauncher.exe"),
            binary: Some("SkyrimSE.exe"),
            script_extender: Some("skse64_loader.exe"),
        };

        // Only the launcher + binary exist; SKSE is not installed yet.
        std::fs::write(dir.join("SkyrimSELauncher.exe"), b"").unwrap();
        std::fs::write(dir.join("SkyrimSE.exe"), b"").unwrap();
        let titles: Vec<String> =
            default_tools(execs, &dir).into_iter().map(|t| t.title).collect();
        assert!(titles.contains(&"Skyrim Special Edition Launcher".to_string()));
        assert!(titles.contains(&"Skyrim Special Edition".to_string()));
        assert!(!titles.iter().any(|t| t.contains("skse")), "absent SKSE is not listed");

        // Install SKSE afterwards: a fresh detection (MO2 re-runs this on every load)
        // picks it up automatically, no user action.
        std::fs::write(dir.join("skse64_loader.exe"), b"").unwrap();
        let titles2: Vec<String> =
            default_tools(execs, &dir).into_iter().map(|t| t.title).collect();
        assert!(titles2.contains(&"skse64_loader".to_string()), "SKSE auto-detected after install");
        // The script extender comes first (the usual play target).
        assert_eq!(titles2.first().map(String::as_str), Some("skse64_loader"));

        // A game with nothing present yields no default tools.
        let empty = tmp_dir();
        assert!(default_tools(execs, &empty).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn round_trips_tools() {
        let p = tmp();
        let tools = vec![
            Tool {
                title: "SSEEdit".into(),
                exe: PathBuf::from("/mnt/Tools/SSEEdit/SSEEdit.exe"),
                // An argument containing spaces must survive the round trip - the
                // old space-joined `args=` line split it into four.
                args: vec!["-D:D:\\My Mods\\Data".into(), "-IKnowWhatImDoing".into()],
                workdir: None,
                prereqs: Vec::new(),
                output_mod: None,
            },
            Tool {
                title: "BodySlide".into(),
                exe: PathBuf::from("Data/CalienteTools/BodySlide/BodySlide x64.exe"),
                args: Vec::new(),
                workdir: Some(PathBuf::from("/mnt/Tools")),
                prereqs: vec!["d3dx9_43".into(), "d3dcompiler_47".into()],
                output_mod: Some("BodySlide Output".into()),
            },
        ];
        write_tools(&p, &tools).unwrap();
        assert_eq!(read_tools(&p), tools);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(read_tools(Path::new("/no/such/tools.ini")).is_empty());
    }

    #[test]
    fn legacy_space_joined_args_still_read() {
        // A pre-per-key tools.ini (single `args=` line) must still load.
        let p = tmp();
        fs::write(&p, "[Tool/Old]\nexe=/x/old.exe\nargs=-a -b -c\n").unwrap();
        let t = read_tools(&p);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].args, vec!["-a", "-b", "-c"]);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn control_char_title_is_skipped_on_write() {
        // A newline in a title would split the [Tool/..] header and corrupt the
        // file; write_tools must drop such an entry, not emit it.
        let p = tmp();
        let tools = vec![
            Tool { title: "Bad\nTitle".into(), exe: PathBuf::from("/x/a.exe"), args: vec![], workdir: None, prereqs: vec![], output_mod: None },
            Tool { title: "Good".into(), exe: PathBuf::from("/x/b.exe"), args: vec![], workdir: None, prereqs: vec![], output_mod: None },
        ];
        write_tools(&p, &tools).unwrap();
        let back = read_tools(&p);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].title, "Good");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn an_output_mod_that_is_not_a_folder_name_never_reaches_disk() {
        // tools.ini is hand-editable and survives a migration, so a value that
        // could aim the capture at `..` or an absolute path must be refused on
        // BOTH sides - written and read - not just once.
        for bad in ["", " ", ".", "..", "a/b", "a\\b", "x\ny"] {
            assert!(!is_mod_folder_name(bad), "{bad:?} must not pass");
        }
        assert!(is_mod_folder_name("FNIS Output"));

        let p = tmp();
        let t = Tool {
            title: "T".into(),
            exe: PathBuf::from("/x/t.exe"),
            args: vec![],
            workdir: None,
            prereqs: vec![],
            output_mod: Some("../escape".into()),
        };
        write_tools(&p, &[t]).unwrap();
        assert!(!fs::read_to_string(&p).unwrap().contains("output_mod"), "not written");
        // And a file that already carries one is not trusted on read either.
        fs::write(&p, "[Tool/T]\nexe=/x/t.exe\noutput_mod=../escape\n").unwrap();
        assert_eq!(read_tools(&p)[0].output_mod, None);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn merge_user_wins_title_collisions() {
        let user = vec![Tool {
            title: "SKSE".into(),
            exe: PathBuf::from("/custom/skse64_loader.exe"),
            args: vec!["-forcesteamloader".into()],
            workdir: None,
            prereqs: Vec::new(),
            output_mod: None,
        }];
        let defaults = vec![
            Tool {
                title: "skse".into(), // case-insensitive collision
                exe: PathBuf::from("skse64_loader.exe"),
                args: Vec::new(),
                workdir: None,
                prereqs: Vec::new(),
                output_mod: None,
            },
            Tool {
                title: "Launcher".into(),
                exe: PathBuf::from("SkyrimSELauncher.exe"),
                args: Vec::new(),
                workdir: None,
                prereqs: Vec::new(),
                output_mod: None,
            },
        ];
        let merged = merge_tools(user, defaults);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].exe, PathBuf::from("/custom/skse64_loader.exe")); // user won
        assert_eq!(merged[1].title, "Launcher");
    }

    #[test]
    fn default_prereqs_maps_known_tools() {
        assert_eq!(default_prereqs("Synthesis"), vec!["dotnet8", "vcrun2022"]);
        assert_eq!(default_prereqs("BodySlide x64"), vec!["d3dx9_43", "d3dcompiler_47"]);
        assert_eq!(default_prereqs("FNIS"), vec!["dotnet48"]);
        assert!(default_prereqs("SSEEdit").is_empty()); // Delphi, needs nothing extra
        assert!(default_prereqs("skse64_loader").is_empty());
    }
}
