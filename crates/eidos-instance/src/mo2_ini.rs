//! A `ModOrganizer.ini` at the instance root, for the tools that read an MO2
//! instance from disk instead of asking a running MO2.
//!
//! PGPatcher and PGLite have a "Mod Organizer 2" mode that attributes every
//! file to the mod it came from and honours the mod priority. They take an
//! instance FOLDER, accept it only if it holds `modorganizer.ini`, and read
//! from that file the game path, the selected profile and the mods, profiles
//! and overwrite folders; the rest is MO2's layout, which Eidos already uses
//! (`profiles/<name>/modlist.txt`, `mods/<name>/`, `overwrite/`).
//!
//! The file is written the way MO2's QSettings writes it, and read back the
//! way PGPatcher reads it: one key per line, found by its `key=` prefix with no
//! regard for sections, so every key starts its line and no key is written
//! twice.

use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::{GameExecutables, Instance, Tool};

/// The first line of a file Eidos wrote. QSettings drops comments when it
/// saves, so a `ModOrganizer.ini` without this line is MO2's or the user's,
/// and Eidos never touches it.
const MARKER: &str = "; Written by Eidos for tools that read a Mod Organizer 2 instance";

const FILE_NAME: &str = "ModOrganizer.ini";

/// What [`Instance::write_mo2_ini`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Mo2Ini {
    Written,
    Unchanged,
    /// A `ModOrganizer.ini` Eidos did not write is at `path`, so nothing was
    /// written. `profile` is its `selected_profile`, the one MO2-mode tools
    /// read instead of the profile Eidos runs.
    Foreign {
        path: PathBuf,
        profile: Option<String>,
    },
}

impl Instance {
    /// Write `ModOrganizer.ini` for `profile` and the game installed at
    /// `game_path`, unless a `ModOrganizer.ini` Eidos did not write is there.
    ///
    /// An unchanged file is left alone, so a launch does not touch it for
    /// nothing.
    pub fn write_mo2_ini(
        &self,
        game_name: &str,
        game_path: &Path,
        profile: &str,
    ) -> io::Result<Mo2Ini> {
        if let Some(path) = self.foreign_mo2_ini() {
            let profile = fs::read(&path)
                .ok()
                .and_then(|old| ini_field(&String::from_utf8_lossy(&old), "selected_profile"));
            return Ok(Mo2Ini::Foreign { path, profile });
        }
        let text = self.mo2_ini_text(game_name, game_path, profile)?;
        let path = self.root.join(FILE_NAME);
        if fs::read(&path).is_ok_and(|old| old == text) {
            return Ok(Mo2Ini::Unchanged);
        }
        crate::write_atomic(&path, &text)?;
        Ok(Mo2Ini::Written)
    }

    /// The `ModOrganizer.ini`, in any case, that Eidos did not write: Mod
    /// Organizer 2's own, in a folder adopted from it.
    pub fn foreign_mo2_ini(&self) -> Option<PathBuf> {
        // Wine opens `modorganizer.ini` case-insensitively, so ANY spelling of
        // a foreign one may be the file the tool reads.
        fs::read_dir(&self.root)
            .ok()?
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(FILE_NAME))
            })
            .map(|e| e.path())
            // An empty file holds no MO2 settings to protect, and a crash soon
            // after the first write can leave one: counting it as foreign
            // would lock Eidos out of it for good. One that cannot be read is
            // counted as foreign, so Eidos never writes over it.
            .find(|p| {
                fs::read(p).map_or(true, |old| {
                    !old.trim_ascii().is_empty() && !old.starts_with(MARKER.as_bytes())
                })
            })
    }

    fn mo2_ini_text(
        &self,
        game_name: &str,
        game_path: &Path,
        profile: &str,
    ) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut push = |line: &[&[u8]]| {
            out.extend(line.concat());
            out.push(b'\n');
        };
        push(&[MARKER.as_bytes(), b". Eidos rewrites it at each launch."]);
        push(&[b"[General]"]);
        push(&[b"gameName=", &qt_escape(game_name.as_bytes(), false)]);
        // MO2 stores these two as QByteArray, and PGPatcher decodes them as such.
        push(&[b"gamePath=", &byte_array(&wine_path(game_path)?)]);
        push(&[b"selected_profile=", &byte_array(profile.as_bytes())]);
        push(&[]);
        // Absolute folders keep the file independent of how a tool spells the
        // instance folder: without them each one is relative to that spelling.
        push(&[b"[Settings]"]);
        let downloads = self.downloads_dir();
        for (key, dir) in [
            (
                "download_directory",
                downloads.is_dir().then_some(downloads),
            ),
            ("mod_directory", Some(self.mods_dir())),
            ("overwrite_directory", Some(self.overwrite_dir())),
            ("profiles_directory", Some(self.profiles_dir())),
        ] {
            if let Some(dir) = dir {
                push(&[key.as_bytes(), b"=", &qt_escape(&wine_path(&dir)?, false)]);
            }
        }
        Ok(out)
    }
}

/// A Linux path as Wine shows it on `Z:`, which is `/` in every prefix.
fn wine_path(path: &Path) -> io::Result<Vec<u8>> {
    let abs = std::path::absolute(path)?;
    let mut out = b"Z:".to_vec();
    out.extend(abs.as_os_str().as_bytes().iter().map(|&b| match b {
        b'/' => b'\\',
        b => b,
    }));
    Ok(out)
}

fn byte_array(bytes: &[u8]) -> Vec<u8> {
    qt_escape(&[b"@ByteArray(", bytes, b")"].concat(), true)
}

/// A value escaped the way QSettings' `iniEscapedString` writes it (Qt 6, the
/// one MO2 2.5 ships). A QByteArray reaches it as Latin-1, so each byte above
/// 0x7E becomes `\xNN`; a string is written as UTF-8. Qt reads a `\x` escape
/// for as many hex digits as follow, so a hex digit right after one is escaped
/// too, and `;`, `,` or `=` anywhere quotes the whole value.
fn qt_escape(value: &[u8], byte_array: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.len() + 2);
    let mut quote = false;
    let mut after_hex = false;
    for &b in value {
        quote |= matches!(b, b';' | b',' | b'=');
        if after_hex && b.is_ascii_hexdigit() {
            out.extend(format!("\\x{b:x}").bytes());
            continue;
        }
        after_hex = false;
        match b {
            0 => {
                out.extend(b"\\0");
                after_hex = true;
            }
            0x07 => out.extend(b"\\a"),
            0x08 => out.extend(b"\\b"),
            0x0c => out.extend(b"\\f"),
            b'\n' => out.extend(b"\\n"),
            b'\r' => out.extend(b"\\r"),
            b'\t' => out.extend(b"\\t"),
            0x0b => out.extend(b"\\v"),
            b'"' | b'\\' => out.extend([b'\\', b]),
            _ if b <= 0x1f || (b >= 0x7f && byte_array) => {
                out.extend(format!("\\x{b:x}").bytes());
                after_hex = true;
            }
            _ => out.push(b),
        }
    }
    if quote || out.first() == Some(&b' ') || out.last() == Some(&b' ') {
        out.insert(0, b'"');
        out.push(b'"');
    }
    out
}

/// Why an MO2 executable was not imported. Each holds the binary as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mo2Skip {
    /// Not on `Z:` and not under a folder named like this instance's own.
    Unplaced(String),
    /// Its mapped path is not a file.
    NotFound(String),
}

/// One `[customExecutables]` entry of an MO2 `ModOrganizer.ini`, and the tool
/// it maps to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mo2Executable {
    pub title: String,
    pub binary: String,
    pub arguments: String,
    pub result: Result<Tool, Mo2Skip>,
}

/// The executables MO2 lists in `ini`, mapped onto the instance at
/// `instance_root`: `Z:` is the Linux root, and a path on another drive is
/// rebased from the last folder named like the instance folder. The game's own
/// loader, launcher and binary become the bare file names the built-in entries
/// use, so they run from the game folder the root union covers.
pub fn mo2_executables(
    ini: &str,
    instance_root: &Path,
    game: GameExecutables<'_>,
) -> Vec<Mo2Executable> {
    let mut entries: std::collections::BTreeMap<u32, Vec<(&str, String)>> = Default::default();
    let mut in_list = false;
    for line in ini.lines() {
        if let Some(section) = eidos_ini::section_header(line) {
            in_list = section.eq_ignore_ascii_case("customExecutables");
            continue;
        }
        let Some((key, value)) = eidos_ini::key_value(line).filter(|_| in_list) else {
            continue;
        };
        let Some((n, field)) = key.split_once('\\') else {
            continue;
        };
        if let Ok(n) = n.parse() {
            entries
                .entry(n)
                .or_default()
                .push((field, crate::meta::qt_value(value)));
        }
    }
    entries
        .into_values()
        .map(|fields| {
            let get = |name: &str| {
                fields
                    .iter()
                    .find(|(f, _)| f.eq_ignore_ascii_case(name))
                    .map_or("", |(_, v)| v.as_str())
            };
            let binary = get("binary").to_string();
            let arguments = get("arguments").to_string();
            let file_name = binary.rsplit(['/', '\\']).next().unwrap_or_default();
            let title = match get("title") {
                "" => Path::new(file_name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                t => t.to_string(),
            };
            let game_exe = [game.script_extender, game.launcher, game.binary]
                .into_iter()
                .flatten()
                .find(|e| e.eq_ignore_ascii_case(file_name));
            let placed = match game_exe {
                Some(exe) => Ok((PathBuf::from(exe), None)),
                None => match mo2_path(&binary, instance_root) {
                    None => Err(Mo2Skip::Unplaced(binary.clone())),
                    Some(p) if !p.is_file() => Err(Mo2Skip::NotFound(binary.clone())),
                    Some(p) => Ok((
                        p,
                        mo2_path(get("workingDirectory"), instance_root).filter(|d| d.is_dir()),
                    )),
                },
            };
            let result = placed.map(|(exe, workdir)| Tool {
                title: title.clone(),
                exe,
                args: split_windows_args(&arguments)
                    .into_iter()
                    .map(|a| mo2_argument(a, instance_root))
                    // `tools.ini` stores one argument per line.
                    .filter(|a| !a.chars().any(char::is_control))
                    .collect(),
                workdir,
                app_id: get("steamAppID").parse().ok().filter(|&id| id != 0),
                hidden: get("hide").eq_ignore_ascii_case("true"),
                ..Default::default()
            });
            Mo2Executable {
                title,
                binary,
                arguments,
                result,
            }
        })
        .collect()
}

/// A Windows path from an MO2 list as a Linux path: `Z:` as it is when it
/// exists, else the folder-name rebase; nothing for any other form.
fn mo2_path(value: &str, root: &Path) -> Option<PathBuf> {
    let value = value.replace('\\', "/");
    let path = drive_path(&value)?;
    if !value[..1].eq_ignore_ascii_case("z") {
        return rebase(path, root);
    }
    let linux = PathBuf::from(path);
    if linux.exists() {
        return Some(linux);
    }
    rebase(path, root).or(Some(linux))
}

/// What follows `X:` in a value that starts with a drive letter and a `/`.
fn drive_path(value: &str) -> Option<&str> {
    let b = value.as_bytes();
    (b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/').then(|| &value[2..])
}

/// `path` (after its drive letter, with `/`) moved under `root` from the last
/// folder that has the same name as `root`, ignoring case.
fn rebase(path: &str, root: &Path) -> Option<PathBuf> {
    let name = root.file_name()?.to_str()?.to_lowercase();
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let at = parts.iter().rposition(|p| p.to_lowercase() == name)?;
    let tail = &parts[at + 1..];
    (!tail.contains(&"..")).then(|| tail.iter().fold(root.to_path_buf(), |p, c| p.join(c)))
}

/// One argument with a path on a drive other than `Z:` rebased and written
/// back in Wine's form. A path that cannot be rebased stays as written.
fn mo2_argument(arg: String, root: &Path) -> String {
    let b = arg.as_bytes();
    let Some(at) = (0..b.len().saturating_sub(2)).find(|&i| {
        b[i].is_ascii_alphabetic() && b[i + 1] == b':' && matches!(b[i + 2], b'/' | b'\\')
    }) else {
        return arg;
    };
    if b[at].eq_ignore_ascii_case(&b'z') {
        return arg;
    }
    match rebase(&arg[at + 2..].replace('\\', "/"), root).and_then(|p| wine_path(&p).ok()) {
        Some(wine) => format!("{}{}", &arg[..at], String::from_utf8_lossy(&wine)),
        None => arg,
    }
}

/// A command line split the way Windows' `CommandLineToArgvW` splits it.
fn split_windows_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut in_token, mut quoted) = (false, false);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let mut n = 1;
                while chars.next_if_eq(&'\\').is_some() {
                    n += 1;
                }
                if chars.peek() == Some(&'"') {
                    // 2n give n and the quote toggles; 2n+1 give n and a quote.
                    cur.extend(std::iter::repeat_n('\\', n / 2));
                    if n % 2 == 1 {
                        chars.next();
                        cur.push('"');
                    }
                } else {
                    cur.extend(std::iter::repeat_n('\\', n));
                }
                in_token = true;
            }
            '"' => {
                quoted = !quoted;
                in_token = true;
            }
            ' ' | '\t' if !quoted => {
                if in_token {
                    out.push(std::mem::take(&mut cur));
                    in_token = false;
                }
            }
            c => {
                cur.push(c);
                in_token = true;
            }
        }
    }
    if in_token {
        out.push(cur);
    }
    out
}

/// PGPatcher 2.3.1's `mo2INIField`, ported: the first line starting with
/// `key=`, outer quotes trimmed, then `decodeQtByteArrayValue` on the
/// inside of `@ByteArray(...)` or `\\` -> `\` on a plain value.
fn ini_field(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    let raw = text.lines().find_map(|l| l.strip_prefix(prefix.as_str()))?;
    let raw = raw.trim_matches('"');
    let Some(inner) = raw
        .strip_prefix("@ByteArray(")
        .and_then(|r| r.strip_suffix(')'))
    else {
        return Some(raw.replace("\\\\", "\\"));
    };
    let b = inner.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            if b[i + 1] == b'x'
                && i + 3 < b.len()
                && b[i + 2].is_ascii_hexdigit()
                && b[i + 3].is_ascii_hexdigit()
            {
                let hex = std::str::from_utf8(&b[i + 2..i + 4]).unwrap();
                out.push(u8::from_str_radix(hex, 16).unwrap());
                i += 4;
                continue;
            }
            if b[i + 1] == b'\\' {
                out.push(b'\\');
                i += 2;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    Some(String::from_utf8_lossy(&out).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eidos-mo2ini-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_file_is_mo2_shaped_and_reads_back_as_pgpatcher_reads_it() {
        let root = temp("shape").join("Mods Été");
        fs::create_dir_all(&root).unwrap();
        let inst = Instance::portable(root.clone());
        let game = Path::new("/mnt/Steam Library/common/Skyrim Spécial,1");
        assert_eq!(
            inst.write_mo2_ini("Skyrim Special Edition", game, "Défaut 2")
                .unwrap(),
            Mo2Ini::Written
        );
        let text = fs::read_to_string(root.join(FILE_NAME)).unwrap();
        let win = |p: &Path| format!("Z:{}", p.display()).replace('/', "\\");

        assert!(text.starts_with(MARKER));
        assert!(text.contains("\n[General]\ngameName=Skyrim Special Edition\n"));
        // Byte for byte what QSettings writes: é is C3 A9, the `c` after it
        // is a hex digit and so escaped too, and the comma quotes the value.
        assert!(text.contains(
            "gamePath=\"@ByteArray(Z:\\\\mnt\\\\Steam Library\\\\common\\\\Skyrim Sp\\xc3\\xa9\\x63ial,1)\"\n"
        ));
        assert!(text.contains("selected_profile=@ByteArray(D\\xc3\\xa9\\x66\\x61ut 2)\n"));
        // No downloads folder, no download_directory.
        assert!(!text.contains("download_directory"));

        assert_eq!(
            ini_field(&text, "gamePath").unwrap(),
            r"Z:\mnt\Steam Library\common\Skyrim Spécial,1"
        );
        assert_eq!(ini_field(&text, "selected_profile").unwrap(), "Défaut 2");
        assert_eq!(
            ini_field(&text, "mod_directory").unwrap(),
            win(&root.join("mods"))
        );
        assert_eq!(
            ini_field(&text, "profiles_directory").unwrap(),
            win(&root.join("profiles"))
        );
        assert_eq!(
            ini_field(&text, "overwrite_directory").unwrap(),
            win(&root.join("overwrite"))
        );
        // PGPatcher's own default for a key that is absent.
        assert_eq!(ini_field(&text, "base_directory"), None);

        fs::create_dir_all(inst.downloads_dir()).unwrap();
        assert_eq!(
            inst.write_mo2_ini("Skyrim Special Edition", game, "Défaut 2")
                .unwrap(),
            Mo2Ini::Written
        );
        let text = fs::read_to_string(root.join(FILE_NAME)).unwrap();
        assert_eq!(
            ini_field(&text, "download_directory").unwrap(),
            win(&root.join("downloads"))
        );
    }

    #[test]
    fn a_foreign_ini_in_any_case_is_never_touched() {
        let root = temp("foreign");
        let inst = Instance::portable(root.clone());
        let mo2 = "[General]\r\ngamePath=@ByteArray(C:\\\\Games\\\\Skyrim)\r\nselected_profile=@ByteArray(Survival)\r\n";
        fs::write(root.join("modorganizer.ini"), mo2).unwrap();
        assert_eq!(
            inst.write_mo2_ini("Skyrim Special Edition", Path::new("/g"), "Default")
                .unwrap(),
            Mo2Ini::Foreign {
                path: root.join("modorganizer.ini"),
                profile: Some("Survival".into()),
            }
        );
        assert_eq!(
            fs::read_to_string(root.join("modorganizer.ini")).unwrap(),
            mo2
        );
        assert!(!root.join(FILE_NAME).exists());
    }

    #[test]
    fn an_empty_ini_is_eidos_own() {
        let root = temp("empty");
        let inst = Instance::portable(root.clone());
        fs::write(root.join(FILE_NAME), "").unwrap();
        assert_eq!(
            inst.write_mo2_ini("Fallout 4", Path::new("/g"), "Default")
                .unwrap(),
            Mo2Ini::Written
        );
        assert!(fs::read_to_string(root.join(FILE_NAME))
            .unwrap()
            .starts_with(MARKER));
    }

    #[test]
    fn a_marked_ini_follows_the_profile_and_is_not_rewritten_unchanged() {
        let root = temp("marked");
        let inst = Instance::portable(root.clone());
        let game = Path::new("/g");
        assert_eq!(
            inst.write_mo2_ini("Fallout 4", game, "Default").unwrap(),
            Mo2Ini::Written
        );
        let path = root.join(FILE_NAME);
        let ino = |p: &Path| std::os::unix::fs::MetadataExt::ino(&fs::metadata(p).unwrap());
        let first = ino(&path);
        // Unchanged: no write at all, so the atomic rename did not swap inodes.
        assert_eq!(
            inst.write_mo2_ini("Fallout 4", game, "Default").unwrap(),
            Mo2Ini::Unchanged
        );
        assert_eq!(ino(&path), first);

        assert_eq!(
            inst.write_mo2_ini("Fallout 4", game, "Survival").unwrap(),
            Mo2Ini::Written
        );
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(ini_field(&text, "selected_profile").unwrap(), "Survival");
    }

    const INI: &str = r#"[General]
gameName=Skyrim Special Edition
[customExecutables]
size=5
1\title=SKSE
1\binary=Z:/old/MyList/Stock Game/skse64_loader.exe
1\workingDirectory=Z:/old/MyList/Stock Game
2\title=SSEEdit
2\binary=D:/Wabbajack/MyList/tools/SSEEdit/SSEEdit.exe
2\arguments="-IKnowWhatImDoing -o:\"D:\\Wabbajack\\MyList\\mods\\Edit Output\""
3\title=Explorer
3\binary=C:/Windows/explorer.exe
4\title=Gone
4\binary=Z:/nowhere/gone.exe
5\title=Hidden
5\binary=D:/Elsewhere/MyList/tools/x.exe
5\hide=true
5\steamAppID=1946180
"#;

    fn skyrim_execs() -> GameExecutables<'static> {
        GameExecutables {
            game_name: "Skyrim Special Edition",
            launcher: Some("SkyrimSELauncher.exe"),
            binary: Some("SkyrimSE.exe"),
            script_extender: Some("skse64_loader.exe"),
            known_tools: &[],
        }
    }

    /// An instance folder named `name` holding the two tools the INI points at.
    fn list_folder(test: &str, name: &str) -> PathBuf {
        let root = temp(test).join(name);
        fs::create_dir_all(root.join("tools/SSEEdit")).unwrap();
        fs::write(root.join("tools/SSEEdit/SSEEdit.exe"), b"").unwrap();
        fs::write(root.join("tools/x.exe"), b"").unwrap();
        root
    }

    #[test]
    fn custom_executables_map_paths_and_explain_skips() {
        let root = list_folder("custom", "MyList");
        let found = mo2_executables(INI, &root, skyrim_execs());
        assert_eq!(found.len(), 5);
        let skse = found[0].result.as_ref().unwrap();
        assert_eq!(
            (skse.exe.as_path(), skse.workdir.as_ref()),
            (Path::new("skse64_loader.exe"), None)
        );
        let edit = found[1].result.as_ref().unwrap();
        assert_eq!(edit.exe, root.join("tools/SSEEdit/SSEEdit.exe"));
        let wine = format!("Z:{}", root.join("mods/Edit Output").display()).replace('/', "\\");
        assert_eq!(
            edit.args,
            vec!["-IKnowWhatImDoing".to_string(), format!("-o:{wine}")]
        );
        assert_eq!(
            found[2].result,
            Err(Mo2Skip::Unplaced("C:/Windows/explorer.exe".into()))
        );
        assert_eq!(
            found[3].result,
            Err(Mo2Skip::NotFound("Z:/nowhere/gone.exe".into()))
        );
        let hidden = found[4].result.as_ref().unwrap();
        assert!(hidden.hidden);
        assert_eq!(hidden.app_id, Some(1946180));
    }

    #[test]
    fn a_renamed_list_folder_is_reported_not_guessed() {
        let root = list_folder("renamed", "Renamed");
        let found = mo2_executables(INI, &root, skyrim_execs());
        assert_eq!(
            found[1].result,
            Err(Mo2Skip::Unplaced(
                "D:/Wabbajack/MyList/tools/SSEEdit/SSEEdit.exe".into()
            ))
        );
        assert_eq!(
            found[4].result,
            Err(Mo2Skip::Unplaced("D:/Elsewhere/MyList/tools/x.exe".into()))
        );
    }

    #[test]
    fn windows_arguments_split_like_command_line_to_argv() {
        assert_eq!(
            split_windows_args(r#"a "b c" d\"e g\\"h i" j\\\"k l\m "" n"#),
            ["a", "b c", "d\"e", "g\\h i", "j\\\"k", "l\\m", "", "n"]
        );
    }

    #[test]
    fn foreign_mo2_ini_ignores_eidos_own_and_empty_files() {
        let root = temp("foreign-any");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let p = root.join("modorganizer.ini");
        assert_eq!(inst.foreign_mo2_ini(), None);
        fs::write(&p, "").unwrap();
        assert_eq!(inst.foreign_mo2_ini(), None);
        fs::write(&p, format!("{MARKER}\n[General]\n")).unwrap();
        assert_eq!(inst.foreign_mo2_ini(), None);
        fs::write(&p, "[General]\ngameName=Skyrim Special Edition\n").unwrap();
        assert_eq!(inst.foreign_mo2_ini(), Some(p));
    }

    #[test]
    fn sample_for_a_portable_skyrim_instance() {
        // The file for a portable instance at /mnt/Jeux/Eidos-Skyrim, rendered
        // from a stand-in folder so the real one is not even read.
        let root = temp("sample");
        let inst = Instance::portable(root.clone());
        let text = inst
            .mo2_ini_text(
                "Skyrim Special Edition",
                Path::new("/mnt/Jeux/SteamLibrary/steamapps/common/Skyrim Special Edition"),
                "Default",
            )
            .unwrap();
        let stand_in = format!("Z:{}", root.display()).replace('/', "\\\\");
        let text = String::from_utf8(text)
            .unwrap()
            .replace(&stand_in, "Z:\\\\mnt\\\\Jeux\\\\Eidos-Skyrim");
        println!("{text}");
        assert!(text.contains(
            "gamePath=@ByteArray(Z:\\\\mnt\\\\Jeux\\\\SteamLibrary\\\\steamapps\\\\common\\\\Skyrim Special Edition)\n"
        ));
        assert!(text.contains("mod_directory=Z:\\\\mnt\\\\Jeux\\\\Eidos-Skyrim\\\\mods\n"));
    }
}
