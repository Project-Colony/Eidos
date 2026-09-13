//! Runtime-specific paths that cannot be represented by an install-relative
//! game descriptor alone.

use crate::DetectedGame;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

impl DetectedGame {
    /// Morrowind's own plugin declares gameDirectory()/Saves, independent of
    /// Wine: ModOrganizer2/modorganizer-game_morrowind, gamemorrowind.cpp:67–75.
    pub fn saves_path(&self) -> Option<PathBuf> {
        if self.def.id == "morrowind" {
            return Some(self.install_path.join("Saves"));
        }
        Some(
            eidos_plugins::documents_my_games_dir(&self.prefix()?, &self.plugin_spec()?)
                .join("Saves"),
        )
    }

    /// The actual mod mountpoint for a launch. 7 Days to Die uses
    /// UserDataFolder/Mods, not the install's shipped Mods (which includes TFP
    /// Harmony). Primary developer statement, 2024-08-02:
    /// community.thefunpimps.com/threads/v1-x-developer-diary.35064/page-177
    ///
    /// Use the actual game command to distinguish native and Windows defaults;
    /// old compatdata is not proof Steam still uses Proton. Explicit overrides
    /// are read from -UserDataFolder= or the selected -configfile= XML. Paths
    /// must be absolute and unambiguous. This method never creates directories.
    pub fn mod_data_path_for_launch(&self, home: &Path, command: &[String]) -> io::Result<PathBuf> {
        if self.def.id != "7daystodie" {
            return Ok(self.data_path.clone());
        }
        let mut windows = None;
        let mut server = false;
        for arg in command {
            let name = arg
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(arg)
                .to_ascii_lowercase();
            let candidate = match name.as_str() {
                "7daystodie.exe" | "7daystodieserver.exe" | "7daystodie_eac.exe" => Some(true),
                "7daystodie.x86_64"
                | "7daystodieserver.x86_64"
                | "7daystodie_eac.x86_64"
                | "7daystodie"
                | "7daystodieserver" => Some(false),
                _ => None,
            };
            if let Some(candidate) = candidate {
                server |= name.starts_with("7daystodieserver");
                if windows
                    .replace(candidate)
                    .is_some_and(|old| old != candidate)
                {
                    return Err(invalid("Conflicting native and Windows game commands"));
                }
            }
        }
        let requested = option(command, "-UserDataFolder")?;
        let selected_config = option(command, "-configfile")?;
        let default_config = server && self.install_path.join("serverconfig.xml").is_file();
        let config = selected_config.or(default_config.then_some("serverconfig.xml"));
        // Read a selected config even when a command override exists: malformed
        // XML is not a safe basis for projecting a different directory.
        let configured = config
            .map(|value| {
                let path = if Path::new(value).is_relative() && !value.contains([':', '\\']) {
                    normal_relative(value).map(|p| self.install_path.join(p))?
                } else {
                    self.userdata_path(value, windows)?
                };
                read_userdata_config(&path)
            })
            .transpose()?
            .flatten();
        if let (Some(argument), Some(config)) = (requested, configured.as_deref()) {
            if self.userdata_path(argument, windows)? != self.userdata_path(config, windows)? {
                return Err(invalid("Command and server config name different UserDataFolder paths; make them agree before launch"));
            }
        }
        let userdata = if let Some(value) = requested.or(configured.as_deref()) {
            self.userdata_path(value, windows)?
        } else if windows == Some(false) {
            if !home.is_absolute() || home.parent().is_none() {
                return Err(invalid("Missing native home directory"));
            }
            home.join(".local/share/7DaysToDie")
        } else if windows == Some(true) {
            self.wine_user_dir()?.join("AppData/Roaming/7DaysToDie")
        } else {
            return Err(invalid("Cannot identify the 7 Days to Die runtime; provide an explicit -UserDataFolder= absolute path"));
        };
        Ok(userdata.join("Mods"))
    }

    fn wine_user_dir(&self) -> io::Result<PathBuf> {
        let prefix = self
            .prefix()
            .ok_or_else(|| invalid("Windows game has no selected Wine prefix"))?;
        let users = prefix.join("drive_c/users");
        if self.is_steam() {
            return Ok(users.join("steamuser"));
        }
        let candidates: Vec<_> = fs::read_dir(&users)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|entry| {
                !["public", "default", "default user", "all users"].contains(
                    &entry
                        .file_name()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .as_str(),
                )
            })
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.path())
            .collect();
        if candidates.len() != 1 {
            return Err(invalid(
                "External Wine prefix has no unique user; supply -UserDataFolder= explicitly",
            ));
        }
        Ok(candidates[0].clone())
    }

    fn userdata_path(&self, value: &str, windows: Option<bool>) -> io::Result<PathBuf> {
        if value.is_empty() || value.contains('\0') || value.starts_with(['"', '\'']) {
            return Err(invalid("Empty or quoted UserDataFolder path"));
        }
        if value.as_bytes().get(1) == Some(&b':') {
            if windows == Some(false) {
                return Err(invalid("Windows UserDataFolder used with a native game"));
            }
            let drive = value.as_bytes()[0].to_ascii_lowercase();
            if !drive.is_ascii_alphabetic()
                || !value
                    .as_bytes()
                    .get(2)
                    .is_some_and(|b| *b == b'/' || *b == b'\\')
            {
                return Err(invalid("Drive-relative UserDataFolder is unsupported"));
            }
            let prefix = self
                .prefix()
                .ok_or_else(|| invalid("Windows UserDataFolder has no selected Wine prefix"))?;
            let mapped = prefix
                .join("dosdevices")
                .join(format!("{}:", drive as char));
            let base = if fs::symlink_metadata(&mapped).is_ok() {
                mapped.canonicalize()?
            } else {
                match drive {
                    b'c' => prefix.join("drive_c"),
                    b'z' => PathBuf::from("/"),
                    _ => return Err(invalid("Unmapped Wine drive in UserDataFolder")),
                }
            };
            let relative = value[3..].replace('\\', "/");
            if relative.is_empty() {
                return Ok(base);
            }
            return Ok(base.join(normal_relative(&relative)?));
        }
        if windows == Some(true) {
            return Err(invalid("Use a Windows absolute path (for example Z:\\home\\...) for a Windows game's UserDataFolder"));
        }
        let path = Path::new(value);
        if !path.is_absolute()
            || path.parent().is_none()
            || value.starts_with("//")
            || path
                .components()
                .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
            || value.split('/').any(|c| c == "." || c == "..")
        {
            return Err(invalid(
                "UserDataFolder must be a non-root absolute path without traversal",
            ));
        }
        Ok(path.to_path_buf())
    }
}

fn option<'a>(command: &'a [String], key: &str) -> io::Result<Option<&'a str>> {
    let mut found = None;
    for arg in command {
        if arg.eq_ignore_ascii_case(key) {
            return Err(invalid(format!("Use {key}=VALUE as one argument")));
        }
        if let Some((name, value)) = arg.split_once('=') {
            if name.eq_ignore_ascii_case(key) && found.replace(value).is_some() {
                return Err(invalid(format!("Repeated {key} override")));
            }
        }
    }
    Ok(found)
}
fn normal_relative(value: &str) -> io::Result<&Path> {
    let path = Path::new(value);
    if value.is_empty()
        || value.split('/').any(|c| c == "." || c == "..")
        || !path.components().all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(invalid(
            "Relative config or userdata suffix contains traversal",
        ));
    }
    Ok(path)
}
fn read_userdata_config(path: &Path) -> io::Result<Option<String>> {
    use quick_xml::events::Event;
    const MAX_CONFIG: u64 = 1024 * 1024;
    if !fs::metadata(path)?.is_file() {
        return Err(invalid("Server config is not a regular file"));
    }
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_CONFIG {
        return Err(invalid(
            "Server config exceeds 1 MiB or is not a regular file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CONFIG {
        return Err(invalid("Server config grew beyond 1 MiB"));
    }
    let mut reader = quick_xml::Reader::from_reader(bytes.as_slice());
    let mut depth = 0usize;
    let mut seen_root = false;
    let mut result = None;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| invalid(format!("Invalid server config XML: {e}")))?;
        let opened = matches!(event, Event::Start(_));
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if depth == 0 {
                    if seen_root || element.name().as_ref() != b"ServerSettings" {
                        return Err(invalid("Expected one ServerSettings XML root"));
                    }
                    seen_root = true;
                } else if depth == 1 && element.name().as_ref() == b"property" {
                    let mut name = None;
                    let mut value = None;
                    for attr in element.attributes() {
                        let attr = attr.map_err(|e| invalid(e.to_string()))?;
                        let text = attr
                            .decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )
                            .map_err(|e| invalid(e.to_string()))?
                            .into_owned();
                        match attr.key.as_ref() {
                            b"name" => name = Some(text),
                            b"value" => value = Some(text),
                            _ => {}
                        }
                    }
                    if name.as_deref() == Some("UserDataFolder") {
                        if result.is_some() {
                            return Err(invalid("Repeated UserDataFolder in server config"));
                        }
                        result = Some(value.ok_or_else(|| invalid("UserDataFolder has no value"))?);
                    }
                }
                if opened {
                    depth += 1;
                    if depth > 64 {
                        return Err(invalid("Server config nesting exceeds 64"));
                    }
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("Unexpected XML closing tag"))?;
            }
            Event::DocType(_) => return Err(invalid("Server config DTD is unsupported")),
            Event::Eof => break,
            Event::Text(text) if depth == 0 && text.iter().any(|b| !b.is_ascii_whitespace()) => {
                return Err(invalid("Text outside server config root"))
            }
            _ => {}
        }
    }
    if !seen_root || depth != 0 {
        return Err(invalid("Incomplete server config XML"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog, GameSource, Store};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let p = std::env::temp_dir().join(format!(
                "eidos-runtime-paths-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
        fn game(&self, id: &str) -> DetectedGame {
            DetectedGame {
                def: catalog().iter().find(|d| d.id == id).unwrap(),
                source: GameSource::Steam,
                install_path: self.0.join("game"),
                data_path: self.0.join("game/Mods"),
                compatdata: Some(self.0.join("compat")),
                steam_name: String::new(),
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn morrowind_saves_are_at_install_root_without_a_prefix() {
        let f = Fixture::new();
        let mut game = f.game("morrowind");
        game.compatdata = None;
        assert_eq!(game.saves_path(), Some(f.0.join("game/Saves")));
        game.source = GameSource::External {
            store: Store::Gog,
            app_id: "x".into(),
            prefix: Some(f.0.join("actual-wine")),
            heroic: true,
        };
        assert_eq!(game.saves_path(), Some(f.0.join("game/Saves")));
    }

    #[test]
    fn seven_days_default_follows_actual_native_or_windows_command() {
        let f = Fixture::new();
        let mut game = f.game("7daystodie");
        let home = f.0.join("home");
        let native = args(&["/game/7DaysToDie.x86_64"]);
        assert_eq!(
            game.mod_data_path_for_launch(&home, &native).unwrap(),
            home.join(".local/share/7DaysToDie/Mods")
        );
        let windows = args(&["proton", "waitforexitandrun", "/game/7DaysToDie.exe"]);
        assert_eq!(
            game.mod_data_path_for_launch(&home, &windows).unwrap(),
            f.0.join("compat/pfx/drive_c/users/steamuser/AppData/Roaming/7DaysToDie/Mods")
        );
        game.compatdata = None;
        assert!(game.mod_data_path_for_launch(&home, &windows).is_err());
        assert!(game
            .mod_data_path_for_launch(&home, &args(&["custom-wrapper"]))
            .is_err());
        game.source = GameSource::External {
            store: Store::Gog,
            app_id: "fixture".into(),
            prefix: Some(f.0.join("external")),
            heroic: true,
        };
        fs::create_dir_all(f.0.join("external/drive_c/users/alice/AppData/Roaming")).unwrap();
        assert_eq!(
            game.mod_data_path_for_launch(&home, &windows).unwrap(),
            f.0.join("external/drive_c/users/alice/AppData/Roaming/7DaysToDie/Mods")
        );
    }

    #[test]
    fn seven_days_explicit_userdata_and_xml_override_defaults_without_writes() {
        let f = Fixture::new();
        let game = f.game("7daystodie");
        let custom = f.0.join("custom data");
        let command = vec![
            "custom-wrapper".into(),
            format!("-UserDataFolder={}", custom.display()),
        ];
        assert_eq!(
            game.mod_data_path_for_launch(&f.0, &command).unwrap(),
            custom.join("Mods")
        );
        assert!(!custom.exists());
        let config = f.0.join("config.xml");
        fs::write(&config, format!("<ServerSettings><!-- ignored --><property name='UserDataFolder' value='{}'/></ServerSettings>", custom.display())).unwrap();
        let command = vec![
            "7DaysToDieServer.x86_64".into(),
            format!("-configfile={}", config.display()),
        ];
        assert_eq!(
            game.mod_data_path_for_launch(&f.0, &command).unwrap(),
            custom.join("Mods")
        );
        let command = args(&["7DaysToDie.exe", "-UserDataFolder=C:\\Mods Data"]);
        assert_eq!(
            game.mod_data_path_for_launch(&f.0, &command).unwrap(),
            f.0.join("compat/pfx/drive_c/Mods Data/Mods")
        );
        for invalid in [
            "relative/path",
            "C:relative",
            "//server/share",
            "/",
            "/tmp/../outside",
        ] {
            assert!(
                game.mod_data_path_for_launch(
                    &f.0,
                    &args(&["7DaysToDie.exe", &format!("-UserDataFolder={invalid}")])
                )
                .is_err(),
                "{invalid}"
            );
        }
        assert_eq!(
            fs::read_dir(&f.0).unwrap().count(),
            1,
            "path planning must not create userdata or prefix directories"
        );
    }

    #[test]
    fn malformed_ambiguous_xml_and_command_paths_fail_closed() {
        let f = Fixture::new();
        let game = f.game("7daystodie");
        let config = f.0.join("config.xml");
        let command = vec![
            "7DaysToDieServer.x86_64".into(),
            format!("-configfile={}", config.display()),
        ];
        for text in ["<ServerSettings><property", "<!DOCTYPE x><ServerSettings/>", "<ServerSettings><property name='UserDataFolder' value='/one'/><property name='UserDataFolder' value='/two'/></ServerSettings>"] {
            fs::write(&config, text).unwrap(); assert!(game.mod_data_path_for_launch(&f.0, &command).is_err());
        }
        assert!(game
            .mod_data_path_for_launch(
                &f.0,
                &args(&[
                    "7DaysToDie.x86_64",
                    "-UserDataFolder=/one",
                    "-UserDataFolder=/two"
                ])
            )
            .is_err());
        let before = game.data_path.clone();
        assert_eq!(
            f.game("skyrimse")
                .mod_data_path_for_launch(&f.0, &[])
                .unwrap(),
            before
        );
    }
    #[test]
    fn server_default_config_is_used_and_conflicting_override_is_refused() {
        let f = Fixture::new();
        let game = f.game("7daystodie");
        fs::create_dir_all(&game.install_path).unwrap();
        let folder = f.0.join("server data");
        fs::write(
            game.install_path.join("serverconfig.xml"),
            format!(
                "<ServerSettings><property name='UserDataFolder' value='{}'/></ServerSettings>",
                folder.display()
            ),
        )
        .unwrap();
        assert_eq!(
            game.mod_data_path_for_launch(&f.0, &args(&["7DaysToDieServer.x86_64"]))
                .unwrap(),
            folder.join("Mods")
        );
        assert_eq!(
            game.mod_data_path_for_launch(&f.0, &args(&["7DaysToDie.x86_64"]))
                .unwrap(),
            f.0.join(".local/share/7DaysToDie/Mods")
        );
        assert!(game
            .mod_data_path_for_launch(
                &f.0,
                &args(&["7DaysToDieServer.x86_64", "-UserDataFolder=/different"])
            )
            .is_err());
    }
}
