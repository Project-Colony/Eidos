//! A mod's Nexus page, description and changelog, read as plain text.
//!
//! Authors write BBCode and HTML. [`plain_text`] turns it into words and line
//! breaks, so nothing downstream ever renders markup or fetches an image.

use crate::collections::{gate, loose_u64};
use crate::{AdultPolicy, HiddenReason};
use serde_json::Value;

/// How many versions of the changelog are kept, newest first.
pub const CHANGELOG_VERSIONS: usize = 30;

/// What a mod's Nexus page says, as plain text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModPage {
    /// Empty when the page is hidden.
    pub description: String,
    /// Each version and its lines, newest upload first.
    pub changelog: Vec<(String, Vec<String>)>,
    /// Why the page is withheld; its text then never leaves this crate.
    pub hidden: Option<HiddenReason>,
}

const PAGE_QUERY: &str = r#"
query modPage($ids: [CompositeDomainWithIdInput!]!) {
  legacyModsByDomain(ids: $ids, count: 1) { nodes { modId gameId adultContent description } }
}
"#;

const CHANGELOG_QUERY: &str = r#"
query modChangelog($modId: ID!, $gameId: ID!) {
  modFiles(modId: $modId, gameId: $gameId) { version date changelogText }
}
"#;

pub(crate) fn description_body(domain: &str, mod_id: u64) -> Value {
    serde_json::json!({
        "query": PAGE_QUERY,
        "variables": { "ids": [{ "gameDomain": domain, "modId": mod_id }] }
    })
}

/// The mod's numeric game id (the changelog query needs it) and its page,
/// without a changelog. A hidden page keeps no description.
pub(crate) fn description_from_payload(
    v: &Value,
    policy: AdultPolicy,
) -> Result<(u64, ModPage), String> {
    let Some((node, game_id)) = v
        .pointer("/data/legacyModsByDomain/nodes/0")
        .and_then(|n| Some((n, loose_u64(n.get("gameId"))?)))
    else {
        return Err("Nexus has no page for this mod".to_string());
    };
    let hidden = gate(node.get("adultContent").and_then(Value::as_bool), policy);
    let description = match hidden {
        Some(_) => String::new(),
        None => plain_text(
            node.get("description")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
    };
    Ok((
        game_id,
        ModPage {
            description,
            changelog: Vec::new(),
            hidden,
        },
    ))
}

/// Nexus types both ids as `ID`, which it takes as strings.
pub(crate) fn changelog_body(game_id: u64, mod_id: u64) -> Value {
    serde_json::json!({
        "query": CHANGELOG_QUERY,
        "variables": { "modId": mod_id.to_string(), "gameId": game_id.to_string() }
    })
}

/// One entry per version, newest upload first, each with its non-empty lines.
pub(crate) fn changelog_from_payload(v: &Value) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut files: Vec<&Value> = v
        .pointer("/data/modFiles")
        .and_then(Value::as_array)
        .ok_or_else(|| "Nexus returned no file list".to_string())?
        .iter()
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(loose_u64(f.get("date")).unwrap_or(0)));
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for f in files {
        if out.len() == CHANGELOG_VERSIONS {
            break;
        }
        let version = f
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if version.is_empty() || out.iter().any(|(v, _)| v == version) {
            continue;
        }
        let lines: Vec<String> = f
            .get("changelogText")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .flat_map(|t| {
                plain_text(t)
                    .split('\n')
                    .map(|l| l.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|l| !l.is_empty())
            .collect();
        if !lines.is_empty() {
            out.push((version.to_string(), lines));
        }
    }
    Ok(out)
}

/// Nexus markup (BBCode and HTML) as plain text: the words stay, line breaks
/// and list bullets stay, a link keeps its address, and images, videos,
/// scripts, styles and every other tag go. Entities are decoded once the tags
/// are gone, so an escaped tag stays text.
pub fn plain_text(markup: &str) -> String {
    let s = join_br_lines(&markup.replace('\r', ""));
    let s = entities(&html(&bbcode(&links(&drop_spans(&s)))));
    let s: String = s
        .chars()
        .filter(|&c| {
            !(c.is_control() && c != '\n' && c != '\t')
                && !('\u{202A}'..='\u{202E}').contains(&c)
                && !('\u{2066}'..='\u{2069}').contains(&c)
        })
        .collect();
    let mut s = s
        .split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s.trim().to_string()
}

/// Nexus ends each line with `\n<br />`: keep one break, not two.
fn join_br_lines(s: &str) -> String {
    let mut lines = s.split('\n');
    let mut out = lines.next().unwrap_or_default().to_string();
    for line in lines {
        let rest = line.trim_start_matches(' ');
        if rest.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("<br")) {
            out.push_str(rest);
        } else {
            out.push('\n');
            out.push_str(line);
        }
    }
    out
}

/// Tags removed with everything up to their closing tag: (opening prefix,
/// the character that ends the opening tag, closing tag).
const DROPPED: [(&str, char, &str); 4] = [
    ("[img", ']', "[/img]"),
    ("[youtube", ']', "[/youtube]"),
    ("<script", '>', "</script>"),
    ("<style", '>', "</style>"),
];

fn drop_spans(s: &str) -> String {
    // ASCII lowercasing keeps every byte offset, so `lower` indexes `s`.
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    'scan: while let Some(c) = s[i..].chars().next() {
        for (open, end, close) in DROPPED {
            if !lower[i..].starts_with(open) {
                continue;
            }
            let after = &s[i + open.len()..];
            // `[img]`, `[img=..]`, `<script src=..>`, but not `[imgur]`.
            if !after.starts_with([end, '=', ' ', '\t', '\n']) {
                continue;
            }
            let Some(tag_end) = after.find(end) else {
                continue;
            };
            let body = i + open.len() + tag_end + 1;
            // An unclosed one drops only the opening tag.
            i = lower[body..]
                .find(close)
                .map_or(body, |at| body + at + close.len());
            continue 'scan;
        }
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// `[url=X]L[/url]` becomes `L (X)`, `L` when it is the address, `X` when it is
/// empty. `[url]X[/url]` is left to [`bbcode`], which drops both tags.
fn links(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while let Some(at) = lower[i..].find("[url=") {
        let open = i + at;
        let Some(target_end) = s[open..].find(']').map(|e| open + e) else {
            break;
        };
        out.push_str(&s[i..open]);
        let target = s[open + 5..target_end].trim();
        i = target_end + 1;
        // An unclosed one drops only the opening tag.
        let Some(label_end) = lower[i..].find("[/url]").map(|e| i + e) else {
            continue;
        };
        let label = &s[i..label_end];
        match label.trim() {
            "" => out.push_str(target),
            l if l == target => out.push_str(label),
            _ => {
                out.push_str(label);
                out.push_str(" (");
                out.push_str(target);
                out.push(')');
            }
        }
        i = label_end + "[/url]".len();
    }
    out.push_str(&s[i..]);
    out
}

/// BBCode tags: (name, what replaces it, whether the opening tag may carry
/// `=value`). Any other bracketed text stays.
const BB_TAGS: &[(&str, &str, bool)] = &[
    ("list", "\n", true),
    ("quote", "\n", true),
    ("spoiler", "\n", false),
    ("code", "\n", false),
    ("center", "\n", false),
    ("left", "\n", false),
    ("right", "\n", false),
    ("justify", "\n", false),
    ("indent", "\n", false),
    ("heading", "\n", false),
    ("h1", "\n", false),
    ("h2", "\n", false),
    ("h3", "\n", false),
    ("line", "\n", false),
    ("hr", "\n", false),
    ("table", "\n", false),
    ("tr", "\n", false),
    ("b", "", false),
    ("i", "", false),
    ("u", "", false),
    ("s", "", false),
    ("size", "", true),
    ("color", "", true),
    ("colour", "", true),
    ("font", "", true),
    ("sup", "", false),
    ("sub", "", false),
    ("td", "", false),
    ("th", "", false),
    ("url", "", false),
    ("email", "", false),
];

fn bb_replacement(tag: &str) -> Option<&'static str> {
    let tag = tag.to_ascii_lowercase();
    if tag == "*" {
        return Some("\n• ");
    }
    let (closing, body) = match tag.strip_prefix('/') {
        Some(body) => (true, body),
        None => (false, tag.as_str()),
    };
    let (name, has_value) = body
        .split_once('=')
        .map_or((body, false), |(n, _)| (n, true));
    BB_TAGS
        .iter()
        .find(|(n, _, _)| *n == name)
        .filter(|(_, _, takes_value)| !has_value || (*takes_value && !closing))
        .map(|(_, replacement, _)| *replacement)
}

fn bbcode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('[') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        // A tag ends at the first `]`, with no `[` before it.
        let tag = tail
            .find(['[', ']'])
            .filter(|&e| tail.as_bytes()[e] == b']')
            .map(|e| &tail[..e]);
        match tag.and_then(|t| Some((t, bb_replacement(t)?))) {
            Some((t, replacement)) => {
                out.push_str(replacement);
                rest = &tail[t.len() + 1..];
            }
            None => {
                out.push('[');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

fn html_replacement(tag: &str) -> &'static str {
    let tag = tag.to_ascii_lowercase();
    let (closing, body) = match tag.strip_prefix('/') {
        Some(body) => (true, body),
        None => (false, tag.as_str()),
    };
    let name = body
        .split(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .unwrap_or_default();
    match (closing, name) {
        (false, "br") | (_, "p" | "div" | "ul" | "ol") | (true, "li") => "\n",
        (false, "li") => "\n• ",
        _ => "",
    }
}

/// A `<` followed by a letter, `/` or `!` opens a tag up to the next `>`; any
/// other `<` is text.
fn html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        if !tail.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/' || c == '!') {
            out.push('<');
            rest = tail;
            continue;
        }
        // No `>` left means no tag can close: the rest is text.
        let Some(end) = tail.find('>') else {
            out.push('<');
            rest = tail;
            break;
        };
        out.push_str(html_replacement(&tail[..end]));
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{A0}'),
        _ => {
            let n = name.strip_prefix('#')?;
            let code = match n.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => n.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

fn entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        // `&#x10FFFF;` is the longest entity decoded here.
        let decoded = tail
            .bytes()
            .take(10)
            .position(|b| b == b';')
            .and_then(|e| Some((e, entity(&tail[..e])?)));
        match decoded {
            Some((e, c)) => {
                out.push(c);
                rest = &tail[e + 1..];
            }
            None => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../tests/data/mod_page.json");
    const CHANGELOG: &str = include_str!("../tests/data/mod_changelog.json");

    #[test]
    fn plain_text_keeps_the_words_and_drops_the_markup() {
        let raw = "[center][b][size=6]Engine Fixes[/size][/b][/center]\n<br />A WIP plugin.\n<br />\n<br />[list][*]One[*]Two[/list]";
        assert_eq!(
            plain_text(raw),
            "Engine Fixes\n\nA WIP plugin.\n\n• One\n• Two"
        );
        assert_eq!(
            plain_text("<ul><li>One</li><li>Two</li></ul><p>End</p>"),
            "• One\n\n• Two\n\nEnd"
        );
    }

    #[test]
    fn plain_text_drops_images_scripts_and_videos_and_keeps_link_targets() {
        assert_eq!(
            plain_text("See [url=https://example.org/a]the guide[/url] and [url]https://example.org/b[/url]."),
            "See the guide (https://example.org/a) and https://example.org/b."
        );
        assert_eq!(
            plain_text("[img]https://evil.example/t.png[/img]Text<img src=x onerror=alert(1)>"),
            "Text"
        );
        assert_eq!(
            plain_text("A<script>alert(1)</script>B<style>p{}</style>C"),
            "ABC"
        );
        assert_eq!(plain_text("[youtube]abc123[/youtube]Video"), "Video");
    }

    #[test]
    fn plain_text_keeps_unknown_brackets_and_decodes_entities() {
        assert_eq!(
            plain_text("[SSE] [1.6.1170] needs [b]SKSE[/b]"),
            "[SSE] [1.6.1170] needs SKSE"
        );
        assert_eq!(
            plain_text("Tom &amp; Jerry &lt;3 &quot;hi&quot; &#39;x&#39; &#233;t&#xE9;"),
            "Tom & Jerry <3 \"hi\" 'x' été"
        );
        assert_eq!(
            plain_text("&lt;script&gt;"),
            "<script>",
            "an escaped tag is text"
        );
        assert_eq!(plain_text("a\u{202E}b\u{0007}c"), "abc");
        assert_eq!(plain_text("x < y and y > z"), "x < y and y > z");
    }

    #[test]
    fn plain_text_handles_unclosed_tags_link_forms_case_and_non_ascii() {
        assert_eq!(
            plain_text("é[IMG]x.png é <SCRIPT>x"),
            "éx.png é x",
            "an unclosed one drops only the tag"
        );
        assert_eq!(
            plain_text("[url=https://a.example]https://a.example[/url]"),
            "https://a.example"
        );
        assert_eq!(
            plain_text("[url=https://a.example][/url] [URL=x]é"),
            "https://a.example é"
        );
        assert_eq!(
            plain_text("[B]Ü[/B] [list=1][*]ö[/list] <BR>[Quote=Me]q[/quote]"),
            "Ü\n\n• ö\n\nq"
        );
        assert_eq!(plain_text("a <b and &amp;lt; [b"), "a <b and &lt; [b");
    }

    #[test]
    fn a_page_reply_gives_the_game_id_and_plain_text() {
        let v: serde_json::Value = serde_json::from_str(PAGE).unwrap();
        let (game, page) = description_from_payload(&v, AdultPolicy::Denied).unwrap();
        assert_eq!(game, 1704);
        assert!(page.hidden.is_none() && page.changelog.is_empty());
        assert_eq!(
            page.description,
            "Engine Fixes\n\nA plugin that fixes bugs."
        );
        assert_eq!(
            description_body("skyrimspecialedition", 17230)["variables"]["ids"][0]["modId"],
            17230
        );
        assert_eq!(
            description_body("skyrimspecialedition", 17230)["variables"]["ids"][0]["gameDomain"],
            "skyrimspecialedition"
        );
    }

    #[test]
    fn an_adult_or_unrated_page_is_withheld_whole() {
        let mut v: serde_json::Value = serde_json::from_str(PAGE).unwrap();
        v["data"]["legacyModsByDomain"]["nodes"][0]["adultContent"] = serde_json::json!(true);
        let (_, page) = description_from_payload(&v, AdultPolicy::Denied).unwrap();
        assert_eq!(
            (page.hidden, page.description.as_str()),
            (Some(HiddenReason::AdultDenied), "")
        );
        assert_eq!(
            description_from_payload(&v, AdultPolicy::Unknown)
                .unwrap()
                .1
                .hidden,
            Some(HiddenReason::AdultUnknown)
        );
        assert!(description_from_payload(&v, AdultPolicy::Allowed)
            .unwrap()
            .1
            .hidden
            .is_none());
        v["data"]["legacyModsByDomain"]["nodes"][0]["adultContent"] = serde_json::Value::Null;
        assert_eq!(
            description_from_payload(&v, AdultPolicy::Allowed)
                .unwrap()
                .1
                .hidden,
            Some(HiddenReason::RatingUnknown)
        );
        let none = serde_json::json!({"data": {"legacyModsByDomain": {"nodes": []}}});
        assert!(description_from_payload(&none, AdultPolicy::Allowed).is_err());
    }

    #[test]
    fn the_changelog_is_grouped_by_version_newest_first() {
        let v: serde_json::Value = serde_json::from_str(CHANGELOG).unwrap();
        assert_eq!(
            changelog_from_payload(&v).unwrap(),
            vec![
                (
                    "2.0".to_string(),
                    vec![
                        "Fixed the crash on load.".to_string(),
                        "Added Dawnguard support.".to_string()
                    ]
                ),
                ("1.1".to_string(), vec!["First fix.".to_string()]),
            ]
        );
        let body = changelog_body(1704, 17230);
        assert_eq!(
            (
                body["variables"]["gameId"].as_str(),
                body["variables"]["modId"].as_str()
            ),
            (Some("1704"), Some("17230"))
        );
        assert!(changelog_from_payload(&serde_json::json!({"data": {}})).is_err());
    }

    #[test]
    fn the_changelog_keeps_only_the_newest_versions() {
        let files: Vec<_> = (0..40)
            .map(|n| serde_json::json!({"version": n.to_string(), "date": n.to_string(), "changelogText": ["x"]}))
            .collect();
        let log =
            changelog_from_payload(&serde_json::json!({"data": {"modFiles": files}})).unwrap();
        assert_eq!(log.len(), CHANGELOG_VERSIONS);
        assert_eq!(
            (log[0].0.as_str(), log[29].0.as_str()),
            ("39", "10"),
            "dates sent as strings sort too"
        );
    }
}
