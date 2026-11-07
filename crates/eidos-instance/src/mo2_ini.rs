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
use std::path::Path;

use crate::Instance;

/// The first line of a file Eidos wrote. QSettings drops comments when it
/// saves, so a `ModOrganizer.ini` without this line is MO2's or the user's,
/// and Eidos never touches it.
const MARKER: &str = "; Written by Eidos for tools that read a Mod Organizer 2 instance";

const FILE_NAME: &str = "ModOrganizer.ini";

impl Instance {
    /// Write `ModOrganizer.ini` for `profile` and the game installed at
    /// `game_path`, unless a `ModOrganizer.ini` Eidos did not write is there.
    ///
    /// Returns whether the file was written: an unchanged file is left alone,
    /// so a launch does not touch it for nothing.
    pub fn write_mo2_ini(
        &self,
        game_name: &str,
        game_path: &Path,
        profile: &str,
    ) -> io::Result<bool> {
        // Wine opens `modorganizer.ini` case-insensitively, so ANY spelling of
        // a foreign one may be the file the tool reads.
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(FILE_NAME))
                && !fs::read(entry.path())?.starts_with(MARKER.as_bytes())
            {
                return Ok(false);
            }
        }
        let text = self.mo2_ini_text(game_name, game_path, profile)?;
        let path = self.root.join(FILE_NAME);
        if fs::read(&path).is_ok_and(|old| old == text) {
            return Ok(false);
        }
        crate::write_atomic(&path, &text)?;
        Ok(true)
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

    /// PGPatcher 2.3.1's `mo2INIField`, ported: the first line starting with
    /// `key=`, outer quotes trimmed, then `decodeQtByteArrayValue` on the
    /// inside of `@ByteArray(...)` or `\\` -> `\` on a plain value.
    fn pgpatcher_field(text: &str, key: &str) -> Option<String> {
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
        Some(String::from_utf8(out).unwrap())
    }

    #[test]
    fn the_file_is_mo2_shaped_and_reads_back_as_pgpatcher_reads_it() {
        let root = temp("shape").join("Mods Été");
        fs::create_dir_all(&root).unwrap();
        let inst = Instance::portable(root.clone());
        let game = Path::new("/mnt/Steam Library/common/Skyrim Spécial,1");
        assert!(inst
            .write_mo2_ini("Skyrim Special Edition", game, "Défaut 2")
            .unwrap());
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
            pgpatcher_field(&text, "gamePath").unwrap(),
            r"Z:\mnt\Steam Library\common\Skyrim Spécial,1"
        );
        assert_eq!(
            pgpatcher_field(&text, "selected_profile").unwrap(),
            "Défaut 2"
        );
        assert_eq!(
            pgpatcher_field(&text, "mod_directory").unwrap(),
            win(&root.join("mods"))
        );
        assert_eq!(
            pgpatcher_field(&text, "profiles_directory").unwrap(),
            win(&root.join("profiles"))
        );
        assert_eq!(
            pgpatcher_field(&text, "overwrite_directory").unwrap(),
            win(&root.join("overwrite"))
        );
        // PGPatcher's own default for a key that is absent.
        assert_eq!(pgpatcher_field(&text, "base_directory"), None);

        fs::create_dir_all(inst.downloads_dir()).unwrap();
        assert!(inst
            .write_mo2_ini("Skyrim Special Edition", game, "Défaut 2")
            .unwrap());
        let text = fs::read_to_string(root.join(FILE_NAME)).unwrap();
        assert_eq!(
            pgpatcher_field(&text, "download_directory").unwrap(),
            win(&root.join("downloads"))
        );
    }

    #[test]
    fn a_foreign_ini_in_any_case_is_never_touched() {
        let root = temp("foreign");
        let inst = Instance::portable(root.clone());
        let mo2 = "[General]\ngamePath=@ByteArray(C:\\\\Games\\\\Skyrim)\n";
        fs::write(root.join("modorganizer.ini"), mo2).unwrap();
        assert!(!inst
            .write_mo2_ini("Skyrim Special Edition", Path::new("/g"), "Default")
            .unwrap());
        assert_eq!(
            fs::read_to_string(root.join("modorganizer.ini")).unwrap(),
            mo2
        );
        assert!(!root.join(FILE_NAME).exists());
    }

    #[test]
    fn a_marked_ini_follows_the_profile_and_is_not_rewritten_unchanged() {
        let root = temp("marked");
        let inst = Instance::portable(root.clone());
        let game = Path::new("/g");
        assert!(inst.write_mo2_ini("Fallout 4", game, "Default").unwrap());
        let path = root.join(FILE_NAME);
        let ino = |p: &Path| std::os::unix::fs::MetadataExt::ino(&fs::metadata(p).unwrap());
        let first = ino(&path);
        // Unchanged: no write at all, so the atomic rename did not swap inodes.
        assert!(!inst.write_mo2_ini("Fallout 4", game, "Default").unwrap());
        assert_eq!(ino(&path), first);

        assert!(inst.write_mo2_ini("Fallout 4", game, "Survival").unwrap());
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(
            pgpatcher_field(&text, "selected_profile").unwrap(),
            "Survival"
        );
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
