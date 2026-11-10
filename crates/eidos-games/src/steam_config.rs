//! Read-only access to Steam's per-account `localconfig.vdf`: the launch
//! option a player typed for a game. Eidos never writes this file.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::steam_root;

enum Tok {
    Str(String),
    Open,
    Close,
}

/// Quoted strings (`\"`, `\\`, `\n`, `\t` unescaped), braces; `//` comments
/// outside strings. Unquoted tokens do not occur in these files and are skipped.
fn tokens(text: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' => out.push(Tok::Open),
            '}' => out.push(Tok::Close),
            '/' if it.peek() == Some(&'/') => {
                for c in it.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                while let Some(c) = it.next() {
                    match c {
                        '"' => break,
                        '\\' => match it.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(e @ ('"' | '\\')) => s.push(e),
                            Some(e) => {
                                s.push('\\');
                                s.push(e);
                            }
                            None => break,
                        },
                        c => s.push(c),
                    }
                }
                out.push(Tok::Str(s));
            }
            _ => {}
        }
    }
    out
}

fn parse_launch_options(text: &str, app_id: u32) -> String {
    let want = app_id.to_string();
    let mut stack: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;
    let mut found = String::new();
    for tok in tokens(text) {
        match tok {
            Tok::Str(s) => match pending.take() {
                None => pending = Some(s),
                Some(key) => {
                    let in_app = stack.len() >= 2
                        && stack[stack.len() - 2].eq_ignore_ascii_case("apps")
                        && stack[stack.len() - 1].eq_ignore_ascii_case(&want);
                    if in_app && key.eq_ignore_ascii_case("LaunchOptions") {
                        found = s;
                    }
                }
            },
            Tok::Open => stack.push(pending.take().unwrap_or_default()),
            Tok::Close => {
                stack.pop();
            }
        }
    }
    found
}

/// The most recently modified `userdata/*/config/localconfig.vdf`.
fn newest_localconfig(home: &Path) -> Option<(PathBuf, fs::Metadata)> {
    let userdata = steam_root(home)?.join("userdata");
    fs::read_dir(userdata)
        .ok()?
        .flatten()
        .map(|e| e.path().join("config/localconfig.vdf"))
        .filter_map(|p| fs::metadata(&p).ok().map(|m| (p, m)))
        .filter(|(_, m)| m.is_file())
        .max_by_key(|(_, m)| m.modified().unwrap_or(SystemTime::UNIX_EPOCH))
}

type MemoKey = (PathBuf, u64, Option<SystemTime>, u32);

/// LaunchOptions for `app_id` from the most recently modified
/// `<steam_root(home)>/userdata/*/config/localconfig.vdf`.
/// `None`: no such file. `Some("")`: the option is unset.
pub fn steam_launch_options(home: &Path, app_id: u32) -> Option<String> {
    // The file can be megabytes and Health reruns on every mod toggle.
    static LAST: Mutex<Option<(MemoKey, String)>> = Mutex::new(None);
    let (path, meta) = newest_localconfig(home)?;
    let key: MemoKey = (path.clone(), meta.len(), meta.modified().ok(), app_id);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = last.as_ref() {
        if *k == key {
            return Some(v.clone());
        }
    }
    let text = String::from_utf8_lossy(&fs::read(&path).ok()?).into_owned();
    let v = parse_launch_options(&text, app_id);
    *last = Some((key, v.clone()));
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const VDF: &str = r#""UserLocalConfigStore"
{
	"Software" { "Valve" { "Steam" { "apps" {
		"377160" { "LaunchOptions"		"%command%" }
		"489830" { "LastPlayed"		"1" "LaunchOptions"		"PROTON_ENABLE_NVAPI=1 \"/home/u/My Games/eidos-gui\" %command% // not a comment" }
		"22380" { "LastPlayed"		"1" }
	} } } }
}"#;

    fn home(tag: &str) -> PathBuf {
        let h = std::env::temp_dir().join(format!("eidos-steamcfg-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&h);
        fs::create_dir_all(h.join(".local/share/Steam/config")).unwrap();
        fs::write(h.join(".local/share/Steam/config/config.vdf"), "").unwrap();
        h
    }

    fn account(h: &Path, id: &str, text: &str) -> PathBuf {
        let d = h
            .join(".local/share/Steam/userdata")
            .join(id)
            .join("config");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("localconfig.vdf");
        fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn launch_options_unescape_quotes_and_backslashes() {
        let h = home("unescape");
        account(&h, "1", VDF);
        assert_eq!(
            steam_launch_options(&h, 489830).as_deref(),
            Some(
                r#"PROTON_ENABLE_NVAPI=1 "/home/u/My Games/eidos-gui" %command% // not a comment"#
            )
        );
    }

    #[test]
    fn launch_options_are_read_for_the_right_app_only() {
        let h = home("right-app");
        account(&h, "1", VDF);
        assert_eq!(
            steam_launch_options(&h, 377160).as_deref(),
            Some("%command%")
        );
    }

    #[test]
    fn an_app_without_launch_options_reads_as_empty() {
        let h = home("empty");
        account(&h, "1", VDF);
        assert_eq!(steam_launch_options(&h, 22380).as_deref(), Some(""));
    }

    #[test]
    fn launch_options_come_from_the_newest_account() {
        let h = home("newest");
        let app =
            |v: &str| format!(r#""S" {{ "apps" {{ "489830" {{ "LaunchOptions" "{v}" }} }} }}"#);
        let old = account(&h, "111", &app("old"));
        account(&h, "222", &app("new"));
        let day_ago = SystemTime::now() - Duration::from_secs(86400);
        fs::File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_modified(day_ago)
            .unwrap();
        assert_eq!(steam_launch_options(&h, 489830).as_deref(), Some("new"));
    }

    #[test]
    fn no_steam_userdata_is_none() {
        let h = home("none");
        assert_eq!(steam_launch_options(&h, 489830), None);
    }
}
