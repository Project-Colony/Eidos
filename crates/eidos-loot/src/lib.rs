//! LOOT-based plugin sorting for Eidos, via the pure-Rust `libloot` crate (the
//! LOOT team's 2025 Rust port). Eidos resolves the plugin set itself
//! (`eidos-plugins`); this crate runs LOOT's real graph sort over it using a
//! per-game masterlist, then hands the ordered names back. The caller reorders its
//! own `PluginList` and re-runs the existing invariant pass, so LOOT's order is a
//! suggestion that Eidos's master-before-dependent guard always backstops.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use libloot::metadata::{
    MessageContent, MessageType as LootMessageType, PluginCleaningData, PluginMetadata,
    select_message_content,
};
use libloot::{EvalMode, Game, GameType, MergeMode};

/// The masterlist metadata-syntax branch libloot 0.29 reads (LOOT versions the
/// per-game masterlist repos by a `v<major>.<minor>` branch). Bump alongside libloot.
const MASTERLIST_BRANCH: &str = "v0.26";

#[derive(Debug)]
pub enum LootError {
    /// The game has no LOOT support wired (timestamp-ordered games, or unmapped).
    Unsupported(String),
    Io(std::io::Error),
    /// A masterlist/prelude download failed.
    Fetch(String),
    /// A libloot operation failed (game handle, masterlist parse, sort, ...).
    Loot(String),
}

impl std::fmt::Display for LootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LootError::Unsupported(g) => write!(f, "LOOT sorting is not supported for {g}"),
            LootError::Io(e) => write!(f, "{e}"),
            LootError::Fetch(e) => write!(f, "masterlist download failed: {e}"),
            LootError::Loot(e) => write!(f, "libloot: {e}"),
        }
    }
}

impl std::error::Error for LootError {}

impl From<std::io::Error> for LootError {
    fn from(e: std::io::Error) -> Self {
        LootError::Io(e)
    }
}

/// LOOT support for an Eidos game id: the libloot `GameType` and the masterlist
/// repo slug under `github.com/loot/<repo>`. `None` for games LOOT can't sort
/// (Morrowind/Oblivion are timestamp-ordered; Eidos doesn't manage those anyway).
pub fn loot_support(game_id: &str) -> Option<(GameType, &'static str)> {
    Some(match game_id {
        "skyrimse" => (GameType::SkyrimSE, "skyrimse"),
        "skyrim" => (GameType::Skyrim, "skyrim"),
        "skyrimvr" => (GameType::SkyrimVR, "skyrimvr"),
        "fallout4" => (GameType::Fallout4, "fallout4"),
        "fallout4vr" => (GameType::Fallout4VR, "fallout4vr"),
        "falloutnv" => (GameType::FalloutNV, "falloutnv"),
        "fallout3" => (GameType::Fallout3, "fallout3"),
        "starfield" => (GameType::Starfield, "starfield"),
        _ => return None,
    })
}

/// Whether Eidos can LOOT-sort this game.
pub fn is_supported(game_id: &str) -> bool {
    loot_support(game_id).is_some()
}

/// Ensure the masterlist + prelude are cached under `cache_dir`, fetching from
/// `github.com/loot/<repo>` (and the shared prelude) when missing or `update`.
/// Returns their paths.
pub fn ensure_masterlist(
    repo: &str,
    cache_dir: &Path,
    update: bool,
) -> Result<(PathBuf, PathBuf), LootError> {
    let masterlist = cache_dir.join("masterlist.yaml");
    let prelude = cache_dir.join("prelude.yaml");
    fs::create_dir_all(cache_dir)?;
    // A refresh is best-effort: if the download fails but a cached copy exists,
    // sorting proceeds with it (LOOT stays usable offline). Only a MISSING file
    // makes a failed fetch fatal.
    let refresh = |url: String, dest: &Path| -> Result<(), LootError> {
        match fetch(&url, dest) {
            Ok(()) => Ok(()),
            Err(e) if dest.is_file() => {
                eprintln!("eidos: keeping the cached {}: {e}", dest.display());
                Ok(())
            }
            Err(e) => Err(e),
        }
    };
    if update || !masterlist.is_file() {
        refresh(
            format!("https://raw.githubusercontent.com/loot/{repo}/{MASTERLIST_BRANCH}/masterlist.yaml"),
            &masterlist,
        )?;
    }
    if update || !prelude.is_file() {
        refresh(
            format!("https://raw.githubusercontent.com/loot/prelude/{MASTERLIST_BRANCH}/prelude.yaml"),
            &prelude,
        )?;
    }
    Ok((masterlist, prelude))
}

fn fetch(url: &str, dest: &Path) -> Result<(), LootError> {
    // Bounded timeouts: a stalled connection must fail the fetch, not hang the
    // caller forever (the GUI sorts off-thread but its Sort button stays greyed).
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .build();
    let body = agent
        .get(url)
        .call()
        .map_err(|e| LootError::Fetch(format!("{url}: {e}")))?
        .into_string()
        .map_err(|e| LootError::Fetch(format!("{url}: {e}")))?;
    // Guard against an HTML error page or a truncated body poisoning the cache:
    // a masterlist is YAML and starts with real content, never a doctype.
    if body.trim_start().starts_with('<') || body.len() < 64 {
        return Err(LootError::Fetch(format!("{url}: unexpected response (not a masterlist)")));
    }
    // Atomic replace so a failed/interrupted download never truncates a good
    // cached copy - the cached masterlist keeps working offline.
    let tmp = dest.with_extension("yaml.tmp");
    fs::write(&tmp, body)?;
    match fs::rename(&tmp, dest) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e.into())
        }
    }
}

/// Run LOOT's sort and return the optimised order of `plugins` (by name).
///
/// `plugins` is `(name, real-path)` for each plugin to sort, in current order
/// (Eidos resolves the real paths across mod folders / Overwrite). `game_path` is
/// the game install dir; `game_local_path` is the prefix's AppData/Local game dir
/// (where `plugins.txt`/`loadorder.txt` live). Conditions in the masterlist are
/// evaluated against `game_path`.
pub fn sort(
    game_id: &str,
    game_path: &Path,
    game_local_path: &Path,
    plugins: &[(String, PathBuf)],
    masterlist: &Path,
    prelude: &Path,
    userlist: Option<&Path>,
) -> Result<Vec<String>, LootError> {
    let (game_type, _repo) =
        loot_support(game_id).ok_or_else(|| LootError::Unsupported(game_id.to_string()))?;

    let mut game = Game::with_local_path(game_type, game_path, game_local_path)
        .map_err(|e| LootError::Loot(e.to_string()))?;

    {
        let db = game.database();
        let mut db = db.write().map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        db.load_masterlist_with_prelude(masterlist, prelude)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        if let Some(ul) = userlist {
            if ul.is_file() {
                db.load_userlist(ul).map_err(|e| LootError::Loot(e.to_string()))?;
            }
        }
    }

    let paths: Vec<&Path> = plugins.iter().map(|(_, p)| p.as_path()).collect();
    game.load_plugin_headers(&paths).map_err(|e| LootError::Loot(e.to_string()))?;
    game.load_current_load_order_state().map_err(|e| LootError::Loot(e.to_string()))?;

    let names: Vec<&str> = plugins.iter().map(|(n, _)| n.as_str()).collect();
    game.sort_plugins(&names).map_err(|e| LootError::Loot(e.to_string()))
}

/// Severity of a LOOT plugin message, mirroring libloot's `MessageType` without
/// leaking the libloot type into the GUI. `Say` is informational, `Warn` is an
/// advisory the user may want to act on, `Error` needs user action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Say,
    Warn,
    Error,
}

/// A single LOOT masterlist/userlist message for a plugin, with its severity and
/// English text (conditions already evaluated, so only true messages appear).
#[derive(Debug, Clone)]
pub struct LootMessage {
    pub kind: MessageType,
    pub text: String,
}

/// LOOT dirty-plugin info: which utility flagged it, plus the record counts that
/// drive the "needs cleaning" advisory (ITM = identical-to-master, UDR = deleted
/// references, NAV = deleted navmeshes). A non-empty entry means LOOT recommends
/// cleaning the plugin (e.g. with xEdit's Quick Auto Clean).
#[derive(Debug, Clone)]
pub struct LootDirtyInfo {
    pub crc: u32,
    pub cleaning_utility: String,
    pub itm_count: u32,
    pub deleted_reference_count: u32,
    pub deleted_navmesh_count: u32,
}

/// All per-plugin LOOT metadata Eidos surfaces in the Plugins tab: evaluated
/// messages, Bash Tag suggestions (`+Name` to add, `-Name` to remove), dirty-info
/// entries, and the plugin's CRC if libloot computed one.
#[derive(Debug, Clone, Default)]
pub struct PluginMetadataBundle {
    pub messages: Vec<LootMessage>,
    pub bash_tags: Vec<String>,
    pub dirty_info: Vec<LootDirtyInfo>,
    pub crc: Option<u32>,
}

impl PluginMetadataBundle {
    /// Whether LOOT has any advisory for this plugin worth showing an icon for.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
            && self.bash_tags.is_empty()
            && self.dirty_info.is_empty()
            && self.crc.is_none()
    }

    /// Whether LOOT flagged this plugin as dirty (needs cleaning).
    pub fn needs_cleaning(&self) -> bool {
        !self.dirty_info.is_empty()
    }

    /// The highest-severity message level present, if any. Useful for picking a
    /// single status icon (error beats warning beats note).
    pub fn highest_severity(&self) -> Option<MessageType> {
        self.messages.iter().map(|m| m.kind).max_by_key(severity_rank)
    }
}

/// Rank a severity so error > warn > say when picking the worst message.
fn severity_rank(kind: &MessageType) -> u8 {
    match kind {
        MessageType::Say => 0,
        MessageType::Warn => 1,
        MessageType::Error => 2,
    }
}

fn convert_message_type(kind: LootMessageType) -> MessageType {
    match kind {
        LootMessageType::Say => MessageType::Say,
        LootMessageType::Warn => MessageType::Warn,
        LootMessageType::Error => MessageType::Error,
    }
}

/// Pick the English (or only-available) text out of a message's localised content.
/// Returns `None` when the message has no content at all.
fn english_text(content: &[MessageContent]) -> Option<String> {
    select_message_content(content, MessageContent::DEFAULT_LANGUAGE).map(|c| c.text().to_owned())
}

/// Build the bash-tag display strings for a plugin: `Name` for additions,
/// `-Name` for removals, matching LOOT/Wrye Bash conventions.
fn collect_bash_tags(metadata: &PluginMetadata) -> Vec<String> {
    metadata
        .tags()
        .iter()
        .map(|tag| {
            if tag.is_addition() {
                tag.name().to_owned()
            } else {
                format!("-{}", tag.name())
            }
        })
        .collect()
}

/// Convert one libloot cleaning-data entry into our owned `LootDirtyInfo`.
fn convert_dirty(data: &PluginCleaningData) -> LootDirtyInfo {
    LootDirtyInfo {
        crc: data.crc(),
        cleaning_utility: data.cleaning_utility().to_owned(),
        itm_count: data.itm_count(),
        deleted_reference_count: data.deleted_reference_count(),
        deleted_navmesh_count: data.deleted_navmesh_count(),
    }
}

/// Build a `PluginMetadataBundle` from an evaluated `PluginMetadata` plus the
/// plugin's CRC (read separately from the loaded plugin header).
fn bundle_from_metadata(metadata: &PluginMetadata, crc: Option<u32>) -> PluginMetadataBundle {
    let messages = metadata
        .messages()
        .iter()
        .filter_map(|m| english_text(m.content()).map(|text| LootMessage {
            kind: convert_message_type(m.message_type()),
            text,
        }))
        .collect();

    PluginMetadataBundle {
        messages,
        bash_tags: collect_bash_tags(metadata),
        dirty_info: metadata.dirty_info().iter().map(convert_dirty).collect(),
        crc,
    }
}

/// Load the LOOT metadata for each plugin: masterlist messages/warnings,
/// dirty-plugin info (needs-cleaning / ITM-UDR), and Bash Tag suggestions.
///
/// Takes the same inputs as [`sort`] (game id, install + local paths, the plugin
/// `(name, real-path)` set, and the masterlist/prelude/userlist paths) and loads
/// the game exactly the same way, so the GUI can call this right after a sort
/// with the identical arguments. Conditions are evaluated against the load order,
/// so only condition-true messages/tags appear; user metadata is merged in.
///
/// Returns a map keyed by **lowercased** plugin name (LOOT name-matching is
/// case-insensitive, so callers should look up by `name.to_lowercase()`). Plugins
/// with no LOOT metadata are omitted from the map.
pub fn metadata(
    game_id: &str,
    game_path: &Path,
    game_local_path: &Path,
    plugins: &[(String, PathBuf)],
    masterlist: &Path,
    prelude: &Path,
    userlist: Option<&Path>,
) -> Result<HashMap<String, PluginMetadataBundle>, LootError> {
    let (game_type, _repo) =
        loot_support(game_id).ok_or_else(|| LootError::Unsupported(game_id.to_string()))?;

    let mut game = Game::with_local_path(game_type, game_path, game_local_path)
        .map_err(|e| LootError::Loot(e.to_string()))?;

    {
        let db = game.database();
        let mut db = db.write().map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        db.load_masterlist_with_prelude(masterlist, prelude)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        if let Some(ul) = userlist {
            if ul.is_file() {
                db.load_userlist(ul).map_err(|e| LootError::Loot(e.to_string()))?;
            }
        }
    }

    // Load headers + load-order state so condition evaluation (and CRCs) work the
    // same way they do during a sort.
    let paths: Vec<&Path> = plugins.iter().map(|(_, p)| p.as_path()).collect();
    game.load_plugin_headers(&paths).map_err(|e| LootError::Loot(e.to_string()))?;
    game.load_current_load_order_state().map_err(|e| LootError::Loot(e.to_string()))?;

    let mut out: HashMap<String, PluginMetadataBundle> = HashMap::new();
    for (name, _) in plugins {
        let crc = game.plugin(name).and_then(|p| p.crc());

        let evaluated = {
            let db = game.database();
            let db = db.read().map_err(|_| LootError::Loot("database lock poisoned".into()))?;
            db.plugin_metadata(name, MergeMode::WithUserMetadata, EvalMode::Evaluate)
                .map_err(|e| LootError::Loot(e.to_string()))?
        };

        let bundle = match evaluated {
            Some(meta) => bundle_from_metadata(&meta, crc),
            None => PluginMetadataBundle {
                crc,
                ..PluginMetadataBundle::default()
            },
        };

        if !bundle.is_empty() {
            out.insert(name.to_lowercase(), bundle);
        }
    }

    Ok(out)
}

/// One plugin's entry in a [`LootReport`]: the problems LOOT found worth showing
/// the user after a sort - missing masters, evaluated messages, and dirty-plugin
/// (needs-cleaning) advisories. Only plugins with at least one of these appear in
/// the report (mirrors MO2, which only lists problem plugins in its LOOT dialog).
#[derive(Debug, Clone)]
pub struct PluginReport {
    pub name: String,
    /// Masters this plugin declares that are not present in the load order.
    pub missing_masters: Vec<String>,
    pub messages: Vec<LootMessage>,
    pub dirty: Vec<LootDirtyInfo>,
}

impl PluginReport {
    fn has_issues(&self) -> bool {
        !self.missing_masters.is_empty() || !self.messages.is_empty() || !self.dirty.is_empty()
    }
}

/// The result of a LOOT run, mirroring MO2's post-sort report dialog: LOOT's
/// general messages plus a per-plugin list of problems (missing masters, messages,
/// dirty info). Built by [`report`] from the same inputs as [`sort`].
#[derive(Debug, Clone, Default)]
pub struct LootReport {
    /// LOOT's general messages (not tied to a plugin): masterlist news, global
    /// advice/warnings. Conditions are already evaluated, so only true ones appear.
    pub general: Vec<LootMessage>,
    /// Per-plugin problems, in load order. Only plugins with an issue are listed.
    pub plugins: Vec<PluginReport>,
}

impl LootReport {
    /// No general messages and no plugin has any issue (a clean report).
    pub fn is_empty(&self) -> bool {
        self.general.is_empty() && self.plugins.is_empty()
    }

    /// Total error-severity messages (general + per-plugin).
    pub fn error_count(&self) -> usize {
        self.count_kind(MessageType::Error)
    }

    /// Total warning-severity messages (general + per-plugin).
    pub fn warning_count(&self) -> usize {
        self.count_kind(MessageType::Warn)
    }

    fn count_kind(&self, kind: MessageType) -> usize {
        let general = self.general.iter().filter(|m| m.kind == kind).count();
        let plugin: usize = self
            .plugins
            .iter()
            .map(|p| p.messages.iter().filter(|m| m.kind == kind).count())
            .sum();
        general + plugin
    }

    /// Number of plugins LOOT reports a missing master for.
    pub fn missing_master_count(&self) -> usize {
        self.plugins.iter().filter(|p| !p.missing_masters.is_empty()).count()
    }

    /// Number of plugins LOOT flagged as dirty (needs cleaning).
    pub fn dirty_count(&self) -> usize {
        self.plugins.iter().filter(|p| !p.dirty.is_empty()).count()
    }
}

/// Convert a slice of libloot messages (general or per-plugin) to [`LootMessage`]s,
/// keeping only those whose evaluated English text is non-empty.
fn convert_messages(messages: &[libloot::metadata::Message]) -> Vec<LootMessage> {
    messages
        .iter()
        .filter_map(|m| {
            english_text(m.content()).map(|text| LootMessage {
                kind: convert_message_type(m.message_type()),
                text,
            })
        })
        .collect()
}

/// Build a full LOOT [`LootReport`] (general messages + per-plugin missing
/// masters / messages / dirty info), mirroring MO2's post-sort report dialog.
///
/// `plugins` is the full set (loaded so conditions evaluate exactly as in [`sort`];
/// the actual active state is read from the game's load order). `enabled_lower` is
/// the set of **enabled** plugin names, lowercased: only enabled plugins are
/// reported (MO2 drops disabled ones), and a master is "missing" when a plugin
/// declares it but it is not enabled - matching Eidos's own crash predictor, since
/// a disabled master is a guaranteed CTD. Plugins with no issues are omitted.
// Mirrors `sort`/`metadata`'s input list (game id + paths + masterlist set) plus the
// enabled set; splitting these into a struct would only obscure the call site.
#[allow(clippy::too_many_arguments)]
pub fn report(
    game_id: &str,
    game_path: &Path,
    game_local_path: &Path,
    plugins: &[(String, PathBuf)],
    enabled_lower: &std::collections::HashSet<String>,
    masterlist: &Path,
    prelude: &Path,
    userlist: Option<&Path>,
) -> Result<LootReport, LootError> {
    let (game_type, _repo) =
        loot_support(game_id).ok_or_else(|| LootError::Unsupported(game_id.to_string()))?;

    let mut game = Game::with_local_path(game_type, game_path, game_local_path)
        .map_err(|e| LootError::Loot(e.to_string()))?;

    {
        let db = game.database();
        let mut db = db.write().map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        db.load_masterlist_with_prelude(masterlist, prelude)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        if let Some(ul) = userlist {
            if ul.is_file() {
                db.load_userlist(ul).map_err(|e| LootError::Loot(e.to_string()))?;
            }
        }
    }

    let paths: Vec<&Path> = plugins.iter().map(|(_, p)| p.as_path()).collect();
    game.load_plugin_headers(&paths).map_err(|e| LootError::Loot(e.to_string()))?;
    game.load_current_load_order_state().map_err(|e| LootError::Loot(e.to_string()))?;

    // A master is "present" only if it is enabled; a disabled master won't load and
    // will crash any enabled dependent, so it must count as missing (matches
    // `eidos_plugins::PluginList::missing_masters`).
    let present = enabled_lower;

    let general = {
        let db = game.database();
        let db = db.read().map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        let msgs = db
            .general_messages(MergeMode::WithUserMetadata, EvalMode::Evaluate)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        convert_messages(&msgs)
    };

    let mut plugin_reports = Vec::new();
    for (name, _) in plugins {
        // MO2 only reports on enabled plugins; a disabled plugin's issues don't
        // matter because it won't load.
        if !enabled_lower.contains(&name.to_ascii_lowercase()) {
            continue;
        }
        let missing_masters = game
            .plugin(name)
            .and_then(|p| p.masters().ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|m| !present.contains(&m.to_ascii_lowercase()))
            .collect::<Vec<_>>();

        let evaluated = {
            let db = game.database();
            let db = db.read().map_err(|_| LootError::Loot("database lock poisoned".into()))?;
            db.plugin_metadata(name, MergeMode::WithUserMetadata, EvalMode::Evaluate)
                .map_err(|e| LootError::Loot(e.to_string()))?
        };

        let (messages, dirty) = match evaluated {
            Some(meta) => (
                convert_messages(meta.messages()),
                meta.dirty_info().iter().map(convert_dirty).collect(),
            ),
            None => (Vec::new(), Vec::new()),
        };

        let entry = PluginReport { name: name.clone(), missing_masters, messages, dirty };
        if entry.has_issues() {
            plugin_reports.push(entry);
        }
    }

    Ok(LootReport { general, plugins: plugin_reports })
}

#[cfg(test)]
mod tests {
    use super::*;

    use libloot::metadata::{
        Message, MessageType as LootMessageType, PluginCleaningData, PluginMetadata, Tag,
        TagSuggestion,
    };

    #[test]
    fn game_support_mapping() {
        assert!(matches!(loot_support("skyrimse"), Some((GameType::SkyrimSE, "skyrimse"))));
        assert!(matches!(loot_support("fallout4"), Some((GameType::Fallout4, "fallout4"))));
        assert!(loot_support("morrowind").is_none()); // timestamp-ordered, unsupported
        assert!(loot_support("nonsense").is_none());
        assert!(is_supported("skyrimse") && !is_supported("oblivion"));
    }

    #[test]
    fn message_type_conversion_maps_each_level() {
        assert_eq!(convert_message_type(LootMessageType::Say), MessageType::Say);
        assert_eq!(convert_message_type(LootMessageType::Warn), MessageType::Warn);
        assert_eq!(convert_message_type(LootMessageType::Error), MessageType::Error);
    }

    #[test]
    fn severity_rank_orders_error_above_warn_above_say() {
        assert!(severity_rank(&MessageType::Error) > severity_rank(&MessageType::Warn));
        assert!(severity_rank(&MessageType::Warn) > severity_rank(&MessageType::Say));
    }

    #[test]
    fn english_text_picks_default_language_and_handles_empty() {
        assert_eq!(english_text(&[]), None);

        let single = [MessageContent::new("only entry".into())];
        assert_eq!(english_text(&single).as_deref(), Some("only entry"));

        let multilingual = [
            MessageContent::new("French note".into()).with_language("fr".into()),
            MessageContent::new("English note".into()).with_language("en".into()),
        ];
        assert_eq!(english_text(&multilingual).as_deref(), Some("English note"));
    }

    #[test]
    fn collect_bash_tags_prefixes_removals_with_minus() {
        let mut meta = PluginMetadata::new("Test.esp").unwrap();
        meta.set_tags(vec![
            Tag::new("Relev".into(), TagSuggestion::Addition),
            Tag::new("C.Water".into(), TagSuggestion::Removal),
        ]);

        assert_eq!(collect_bash_tags(&meta), vec!["Relev", "-C.Water"]);
    }

    #[test]
    fn convert_dirty_carries_all_counts() {
        let data = PluginCleaningData::new(0xDEAD_BEEF, "xEdit".into())
            .with_itm_count(7)
            .with_deleted_reference_count(4)
            .with_deleted_navmesh_count(2);

        let info = convert_dirty(&data);
        assert_eq!(info.crc, 0xDEAD_BEEF);
        assert_eq!(info.cleaning_utility, "xEdit");
        assert_eq!(info.itm_count, 7);
        assert_eq!(info.deleted_reference_count, 4);
        assert_eq!(info.deleted_navmesh_count, 2);
    }

    #[test]
    fn bundle_from_metadata_collects_messages_tags_and_dirty() {
        let mut meta = PluginMetadata::new("Test.esp").unwrap();
        meta.set_messages(vec![
            Message::new(LootMessageType::Say, "a note".into()),
            Message::new(LootMessageType::Error, "an error".into()),
        ]);
        meta.set_tags(vec![Tag::new("Relev".into(), TagSuggestion::Addition)]);
        meta.set_dirty_info(vec![
            PluginCleaningData::new(0x1234_5678, "xEdit".into()).with_itm_count(3),
        ]);

        let bundle = bundle_from_metadata(&meta, Some(0x1234_5678));

        assert!(!bundle.is_empty());
        assert!(bundle.needs_cleaning());
        assert_eq!(bundle.crc, Some(0x1234_5678));
        assert_eq!(bundle.bash_tags, vec!["Relev"]);
        assert_eq!(bundle.messages.len(), 2);
        assert_eq!(bundle.messages[1].text, "an error");
        assert_eq!(bundle.highest_severity(), Some(MessageType::Error));
        assert_eq!(bundle.dirty_info.len(), 1);
        assert_eq!(bundle.dirty_info[0].itm_count, 3);
    }

    #[test]
    fn empty_bundle_is_empty_and_not_dirty() {
        let bundle = PluginMetadataBundle::default();
        assert!(bundle.is_empty());
        assert!(!bundle.needs_cleaning());
        assert_eq!(bundle.highest_severity(), None);

        // A CRC alone still counts as content worth carrying.
        let crc_only = PluginMetadataBundle { crc: Some(1), ..PluginMetadataBundle::default() };
        assert!(!crc_only.is_empty());
        assert!(!crc_only.needs_cleaning());
    }

    fn msg(kind: MessageType, text: &str) -> LootMessage {
        LootMessage { kind, text: text.to_string() }
    }

    #[test]
    fn loot_report_counts_aggregate_general_and_per_plugin() {
        let report = LootReport {
            general: vec![msg(MessageType::Warn, "g warn"), msg(MessageType::Say, "g note")],
            plugins: vec![
                PluginReport {
                    name: "A.esp".into(),
                    missing_masters: vec!["Base.esm".into()],
                    messages: vec![msg(MessageType::Error, "p err")],
                    dirty: vec![],
                },
                PluginReport {
                    name: "B.esp".into(),
                    missing_masters: vec![],
                    messages: vec![],
                    dirty: vec![LootDirtyInfo {
                        crc: 1,
                        cleaning_utility: "xEdit".into(),
                        itm_count: 2,
                        deleted_reference_count: 0,
                        deleted_navmesh_count: 0,
                    }],
                },
            ],
        };

        assert!(!report.is_empty());
        assert_eq!(report.error_count(), 1, "one per-plugin error");
        assert_eq!(report.warning_count(), 1, "one general warning");
        assert_eq!(report.missing_master_count(), 1, "only A.esp is missing a master");
        assert_eq!(report.dirty_count(), 1, "only B.esp is dirty");
    }

    #[test]
    fn empty_loot_report_is_empty_with_zero_counts() {
        let report = LootReport::default();
        assert!(report.is_empty());
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
        assert_eq!(report.missing_master_count(), 0);
        assert_eq!(report.dirty_count(), 0);
    }
}
