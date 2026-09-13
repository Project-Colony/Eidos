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
    select_message_content, MessageContent, MessageType as LootMessageType, PluginCleaningData,
    PluginMetadata,
};
use libloot::{EvalMode, Game, GameType, MergeMode};

/// The masterlist metadata-syntax branch libloot reads. LOOT versions the
/// per-game masterlist repos by a `v<major>.<minor>` branch matching the library,
/// and **the old branches are not maintained** - they are frozen at whatever the
/// syntax bump found there.
///
/// This must be bumped with the `libloot` dependency, and once was not: the crate
/// went to 0.29 while this stayed on `v0.26`, so sorting ran on metadata that had
/// stopped being updated on 2026-04-11 while upstream kept moving. Three and a
/// half months of new mods, dirty-plugin records and incompatibility rules, all
/// invisible, with nothing failing to say so.
const MASTERLIST_BRANCH: &str = "v0.29";

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
/// Timestamp engines use the same metadata engine and project the saved order at launch.
pub fn loot_support(game_id: &str) -> Option<(GameType, &'static str)> {
    Some(match game_id {
        "skyrimse" => (GameType::SkyrimSE, "skyrimse"),
        "enderalse" => (GameType::SkyrimSE, "enderal"),
        "morrowind" => (GameType::Morrowind, "morrowind"),
        "oblivion" => (GameType::Oblivion, "oblivion"),
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
            format!(
                "https://raw.githubusercontent.com/loot/{repo}/{MASTERLIST_BRANCH}/masterlist.yaml"
            ),
            &masterlist,
        )?;
    }
    if update || !prelude.is_file() {
        refresh(
            format!(
                "https://raw.githubusercontent.com/loot/prelude/{MASTERLIST_BRANCH}/prelude.yaml"
            ),
            &prelude,
        )?;
    }
    Ok((masterlist, prelude))
}

fn fetch(url: &str, dest: &Path) -> Result<(), LootError> {
    // Bounded timeouts: a stalled connection must fail the fetch, not hang the
    // caller forever. It would hang more than the fetch: the GUI sorts on iced's
    // executor, which is smol's single-threaded one, so a stalled download also
    // holds up every other Task and every timer subscription behind it.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        // `timeout_global` is ureq 2's plain `timeout`: the whole exchange, not
        // one socket operation. A masterlist is a megabyte, so a minute is
        // generous even on a bad line.
        .timeout_global(Some(std::time::Duration::from_secs(60)))
        .build()
        .into();
    let body = agent
        .get(url)
        .call()
        .map_err(|e| LootError::Fetch(format!("{url}: {e}")))?
        .into_body()
        .read_to_string()
        .map_err(|e| LootError::Fetch(format!("{url}: {e}")))?;
    // Guard against an HTML error page or a truncated body poisoning the cache:
    // a masterlist is YAML and starts with real content, never a doctype.
    if body.trim_start().starts_with('<') || body.len() < 64 {
        return Err(LootError::Fetch(format!(
            "{url}: unexpected response (not a masterlist)"
        )));
    }
    // Atomic replace so a failed/interrupted download never truncates a good
    // cached copy - the cached masterlist keeps working offline.
    // Unique per process and per call: a fixed temp name is not atomic against
    // a second WRITER, and the GUI's sort and a `eidos sort` both refresh this
    // cache. Two of them sharing one temp interleave their bytes, and the last
    // rename publishes the mixture as a masterlist.
    let tmp = dest.with_extension(format!(
        "yaml.eidos-tmp.{}.{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::write(&tmp, body)?;
    match fs::rename(&tmp, dest) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e.into())
        }
    }
}

/// Distinguishes concurrent writes from the same process.
static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Run LOOT's sort and return the optimised order of `plugins` (by name).
///
/// `plugins` is `(name, real-path)` for each plugin to sort, in current order
/// (Eidos resolves the real paths across mod folders / Overwrite). `game_path` is
/// the game install dir; `game_local_path` is the prefix's AppData/Local game dir
/// (where `plugins.txt`/`loadorder.txt` live). Conditions in the masterlist are
/// evaluated against `game_path`.
pub fn sort(view: &GameView<'_>) -> Result<Vec<String>, LootError> {
    let (game_id, plugins) = (view.game_id, view.plugins);
    let (masterlist, prelude, userlist) = (view.masterlist, view.prelude, view.userlist);
    let (game_type, _repo) =
        loot_support(game_id).ok_or_else(|| LootError::Unsupported(game_id.to_string()))?;

    let mut game = Game::with_local_path(game_type, view.game_path, view.local_path)
        .map_err(|e| LootError::Loot(e.to_string()))?;
    set_mod_dirs(&mut game, view.mod_dirs)?;

    {
        let db = game.database();
        let mut db = db
            .write()
            .map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        db.load_masterlist_with_prelude(masterlist, prelude)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        if let Some(ul) = userlist {
            if ul.is_file() {
                db.load_userlist(ul)
                    .map_err(|e| LootError::Loot(e.to_string()))?;
            }
        }
    }

    let paths: Vec<&Path> = plugins.iter().map(|(_, p)| p.as_path()).collect();
    // WHOLE plugins, not headers. LOOT's sort has five stages, and the fourth -
    // overlap - is the one that reorders plugins nobody wrote a masterlist rule
    // for: two plugins touching a common record get an edge from the one
    // overriding more to the one overriding fewer. It needs the records.
    //
    // With headers only, esplugin stores no record ids, so
    // `override_record_count` is 0, and libloot skips archive scanning, so
    // `asset_count` is 0 - and `add_overlap_edges` opens by skipping every
    // plugin where both are zero. The entire stage was dead, silently: no
    // error, no warning, and a status line still reading "LOOT checked 211
    // plugins". What survived was masters, groups and the tie-break, and the
    // tie-break is the user's current order - so a dragged plugin stayed where
    // it was dropped, which is precisely what MO2 does not do. MO2's lootcli
    // passes `loadHeadersOnly = false`; this is the same call.
    game.load_plugins(&paths)
        .map_err(|e| LootError::Loot(e.to_string()))?;
    game.load_current_load_order_state()
        .map_err(|e| LootError::Loot(e.to_string()))?;

    let names: Vec<&str> = plugins.iter().map(|(n, _)| n.as_str()).collect();
    game.sort_plugins(&names)
        .map_err(|e| LootError::Loot(e.to_string()))
}

/// One plugin rule a collection contributes, in neutral terms.
///
/// Neutral on purpose: this crate must not learn about collections, and the
/// collection crate must not learn about libloot. The caller translates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserRule {
    pub plugin: String,
    /// Plugins this one must load after.
    pub after: Vec<String>,
    /// The LOOT group it belongs in, if the collection assigns one.
    pub group: Option<String>,
}

/// A LOOT group a collection defines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupDef {
    pub name: String,
    pub after: Vec<String>,
}

/// What merging a collection's rules changed, and what it refused to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Merged {
    pub rules_added: usize,
    pub groups_added: usize,
    /// Plugins whose group the USER had already set, so the collection's was
    /// not applied. Reported, because the load order will differ from the
    /// collection author's and the user is entitled to know why.
    pub kept_user_group: Vec<String>,
    /// Group assignments dropped because no such group exists.
    ///
    /// This one is not politeness, it is survival: LOOT refuses to sort at all
    /// when a plugin names a group nothing defines, and the failure surfaces as
    /// no load order rather than as a bad one.
    pub dangling_groups: Vec<String>,
    /// `after` entries dropped from a collection's own group definitions,
    /// as `"<group> -> <name>"`.
    ///
    /// The same survival rule, one level up: libloot builds the group graph
    /// before it sorts anything, and a group whose `after` names nothing makes
    /// EVERY later sort of that instance fail - including the ones the user runs
    /// for reasons of their own, long after the collection is forgotten.
    pub dangling_after: Vec<String>,
    /// Groups the collection defined that already existed, and whose `after`
    /// list was folded into the existing one rather than added again.
    pub groups_extended: Vec<String>,
}

/// Merge a collection's plugin rules into the instance's userlist.
///
/// ADDITIONS only. The user's own userlist is theirs: nothing is removed, and
/// where the two disagree about a plugin's group the user wins. That is Vortex's
/// rule too, and it is the right way round - a collection is a suggestion about
/// a mod list the user owns.
///
/// The masterlist is loaded as well, because the set of groups that EXIST is
/// the masterlist's plus the userlist's plus whatever the collection defines,
/// and a group assignment naming none of them is the dangling case above.
pub fn merge_user_rules(
    view: &GameView<'_>,
    rules: &[UserRule],
    groups: &[GroupDef],
) -> Result<Merged, LootError> {
    let (game_type, _repo) = loot_support(view.game_id)
        .ok_or_else(|| LootError::Unsupported(view.game_id.to_string()))?;
    let userlist = view
        .userlist
        .ok_or_else(|| LootError::Loot("no userlist path to merge into".into()))?;

    let game = Game::with_local_path(game_type, view.game_path, view.local_path)
        .map_err(|e| LootError::Loot(e.to_string()))?;
    let db = game.database();
    let mut db = db
        .write()
        .map_err(|_| LootError::Loot("database lock poisoned".into()))?;
    db.load_masterlist_with_prelude(view.masterlist, view.prelude)
        .map_err(|e| LootError::Loot(e.to_string()))?;
    if userlist.is_file() {
        db.load_userlist(userlist)
            .map_err(|e| LootError::Loot(e.to_string()))?;
    }

    let mut out = Merged::default();

    // Groups first: a plugin can only be put in a group that exists by the time
    // the assignments are considered.
    let existing: std::collections::HashSet<String> = db
        .groups(libloot::MergeMode::WithUserMetadata)
        .iter()
        .map(|g| g.name().to_string())
        .collect();
    let mut user_groups = db.user_groups().to_vec();
    // Every name that will exist once this merge is written: what LOOT already
    // knows, plus what the collection is about to define. An `after` may point
    // at any of them, including a sibling defined in the same batch.
    let mut known: std::collections::HashSet<String> = existing.clone();
    for g in groups {
        if !g.name.is_empty() {
            known.insert(g.name.clone());
        }
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for g in groups {
        if g.name.is_empty() {
            continue;
        }
        // A userlist group may be named twice in one file only by mistake, and
        // libloot's reader REFUSES a userlist with a duplicate entry - so the
        // next load of the file this function is about to write would fail.
        if !seen.insert(g.name.clone()) {
            continue;
        }
        // An `after` naming nothing is the same disaster as a plugin naming
        // nothing, one level up, and it is not otherwise checked anywhere.
        let after: Vec<String> = g
            .after
            .iter()
            .filter(|a| !a.is_empty())
            .filter(|a| {
                if known.contains(*a) {
                    true
                } else {
                    out.dangling_after.push(format!("{} -> {a}", g.name));
                    false
                }
            })
            .cloned()
            .collect();

        // In LOOT a userlist group sharing a masterlist group's name does not
        // replace it, it EXTENDS it: the two `after` lists are unioned. Dropping
        // the collection's entry, which is what "the name is taken" used to do,
        // silently discards the author's placement.
        if let Some(slot) = user_groups.iter_mut().find(|u| u.name() == g.name) {
            let mut merged: Vec<String> =
                slot.after_groups().iter().map(|a| a.to_string()).collect();
            let mut grew = false;
            for a in &after {
                if !merged.iter().any(|m| m.eq_ignore_ascii_case(a)) {
                    merged.push(a.clone());
                    grew = true;
                }
            }
            if grew {
                *slot = libloot::metadata::Group::new(g.name.clone()).with_after_groups(merged);
                out.groups_extended.push(g.name.clone());
            }
            continue;
        }
        if existing.contains(&g.name) {
            // A masterlist group. A userlist entry of the same name extends it,
            // so it is worth adding - but only if it says something.
            if after.is_empty() {
                continue;
            }
            user_groups
                .push(libloot::metadata::Group::new(g.name.clone()).with_after_groups(after));
            out.groups_extended.push(g.name.clone());
            continue;
        }
        user_groups.push(libloot::metadata::Group::new(g.name.clone()).with_after_groups(after));
        out.groups_added += 1;
    }
    if out.groups_added > 0 || !out.groups_extended.is_empty() {
        db.set_user_groups(user_groups);
    }

    for rule in rules {
        if rule.plugin.is_empty() {
            continue;
        }
        let mut meta = db
            .plugin_user_metadata(&rule.plugin, libloot::EvalMode::DoNotEvaluate)
            .map_err(|e| LootError::Loot(e.to_string()))?
            .unwrap_or(
                libloot::metadata::PluginMetadata::new(&rule.plugin)
                    .map_err(|e| LootError::Loot(e.to_string()))?,
            );

        // `after`: a union with whatever is already there. Never a replacement.
        let mut after: Vec<libloot::metadata::File> = meta.load_after_files().to_vec();
        let mut changed = false;
        for name in rule.after.iter().filter(|n| !n.is_empty()) {
            let already = after
                .iter()
                .any(|f| f.name().as_str().eq_ignore_ascii_case(name));
            if !already {
                after.push(libloot::metadata::File::new(name.clone()));
                changed = true;
            }
        }
        if changed {
            meta.set_load_after_files(after);
        }

        if let Some(group) = rule.group.as_ref().filter(|g| !g.is_empty()) {
            if meta.group().is_some_and(|g| g != group.as_str()) {
                // The user already decided something ELSE. Theirs stands.
                //
                // Equal is not a conflict: this function runs at the end of
                // every collection run, and re-running is the documented way to
                // finish an interrupted install, so the second pass always meets
                // its own output. Reporting that as the user's decision would
                // put a line in the report on every re-run and hide the real
                // ones.
                out.kept_user_group.push(rule.plugin.clone());
            } else if meta.group().is_some() {
                // Already exactly what the collection asks for.
            } else if known.contains(group) {
                meta.set_group(group.clone());
                changed = true;
            } else {
                // Assigning it would make LOOT refuse to sort AT ALL, and the
                // symptom is an empty load order with no explanation.
                out.dangling_groups
                    .push(format!("{} -> {group}", rule.plugin));
            }
        }

        if changed {
            db.set_plugin_user_metadata(meta);
            out.rules_added += 1;
        }
    }

    if out.rules_added > 0 || out.groups_added > 0 || !out.groups_extended.is_empty() {
        if let Some(dir) = userlist.parent() {
            std::fs::create_dir_all(dir).map_err(|e| LootError::Loot(e.to_string()))?;
        }
        // Truncate: this REPLACES the userlist file with what the database now
        // holds, which is the merge of what was in it and what was added. It is
        // not a fresh file - the load above is what makes it a merge.
        let mut opts = libloot::MetadataWriteOptions::new();
        opts.set_truncate(true);
        db.write_user_metadata(userlist, &opts)
            .map_err(|e| LootError::Loot(e.to_string()))?;
    }
    Ok(out)
}

/// Everything LOOT needs to look at one game: who it is, where it lives, and
/// which trees count as its data.
///
/// A struct rather than nine positional parameters because `sort` and `report`
/// must be given the SAME view - a report built from a different set of data
/// paths than the sort would explain a decision that was never made - and
/// because two adjacent `&Path` arguments are a swap waiting to happen.
#[derive(Debug, Clone, Copy)]
pub struct GameView<'a> {
    pub game_id: &'a str,
    /// The game install directory.
    pub game_path: &'a Path,
    /// Where the load-order files live (the profile's plugins dir).
    pub local_path: &'a Path,
    /// Every plugin by `(name, real resolved path)`.
    pub plugins: &'a [(String, PathBuf)],
    /// The mod trees, highest priority first. See [`set_mod_dirs`].
    pub mod_dirs: &'a [PathBuf],
    pub masterlist: &'a Path,
    pub prelude: &'a Path,
    pub userlist: Option<&'a Path>,
}

/// Tell libloot where the mods live.
///
/// Without this, LOOT sees only the game's own `Data` directory - and under
/// Eidos that directory holds nothing but vanilla, because the mods are separate
/// folders that only become one tree inside the launch namespace. Every
/// masterlist rule conditioned on a file (`file("SomeMod.esp")`,
/// `checksum(...)`, the dirty-plugin and incompatibility conditions) therefore
/// evaluated FALSE, and LOOT sorted with a fraction of the metadata it has. It
/// looked like LOOT simply had no opinion about most plugins.
///
/// Highest priority first: libloot uses these in the order given, and they take
/// precedence over the game's main data path - the same direction the union
/// resolves in.
fn set_mod_dirs(game: &mut Game, dirs: &[PathBuf]) -> Result<(), LootError> {
    if dirs.is_empty() {
        return Ok(());
    }
    game.set_additional_data_paths(dirs.to_vec())
        .map_err(|e| LootError::Loot(e.to_string()))
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
    /// Whole-record scale checks: None means unverified (or not applicable).
    pub record_validity: Option<bool>,
}

impl PluginMetadataBundle {
    /// Whether LOOT has any advisory for this plugin worth showing an icon for.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
            && self.bash_tags.is_empty()
            && self.dirty_info.is_empty()
            && self.crc.is_none()
            && self.record_validity.is_none()
    }

    /// Whether LOOT flagged this plugin as dirty (needs cleaning).
    pub fn needs_cleaning(&self) -> bool {
        !self.dirty_info.is_empty()
    }

    /// The highest-severity message level present, if any. Useful for picking a
    /// single status icon (error beats warning beats note).
    pub fn highest_severity(&self) -> Option<MessageType> {
        self.messages
            .iter()
            .map(|m| m.kind)
            .max_by_key(severity_rank)
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
        .filter_map(|m| {
            english_text(m.content()).map(|text| LootMessage {
                kind: convert_message_type(m.message_type()),
                text,
            })
        })
        .collect();

    PluginMetadataBundle {
        messages,
        bash_tags: collect_bash_tags(metadata),
        dirty_info: metadata.dirty_info().iter().map(convert_dirty).collect(),
        crc,
        record_validity: None,
    }
}

fn record_validity(plugin: &libloot::Plugin) -> (Option<bool>, Vec<LootMessage>) {
    let mut checks = Vec::new();
    if plugin.is_light_plugin() {
        checks.push(("light", plugin.is_valid_as_light_plugin()));
    }
    if plugin.is_medium_plugin() {
        checks.push(("medium", plugin.is_valid_as_medium_plugin()));
    }
    if plugin.is_update_plugin() {
        checks.push(("update", plugin.is_valid_as_update_plugin()));
    }
    let mut valid = (!checks.is_empty()).then_some(true);
    let mut messages = Vec::new();
    for (kind, result) in checks {
        match result {
            Ok(true) => {}
            Ok(false) => {
                valid = valid.map(|_| false);
                messages.push(LootMessage {
                    kind: MessageType::Error,
                    text: format!("Invalid {kind} plugin records: new FormIDs do not satisfy the declared plugin type's limits. Inspect the plugin in xEdit before enabling it."),
                });
            }
            Err(e) => {
                valid = None;
                messages.push(LootMessage {
                    kind: MessageType::Warn,
                    text: format!("Unverified {kind} plugin records: {e}."),
                });
            }
        }
    }
    (valid, messages)
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
    /// Per-plugin LOOT metadata for the Plugins tab (evaluated messages, Bash Tag
    /// suggestions, dirty-plugin info, CRC), keyed by ASCII-lowercased plugin
    /// name - the same convention as `enabled_lower`. Enabled plugins only, like
    /// the report rows, and plugins with nothing to say are omitted.
    ///
    /// Built inside [`report`] from the SAME loaded game as the problem rows, so
    /// the icons and the report can never disagree - and so the tab costs no
    /// second masterlist load (this map used to be a standalone `metadata()`
    /// entry point that re-loaded everything and that nothing ever called).
    pub plugin_meta: HashMap<String, PluginMetadataBundle>,
}

impl LootReport {
    /// No general messages and no plugin has any ISSUE (a clean report).
    /// Deliberately ignores [`Self::plugin_meta`]: bash tags or a CRC are
    /// information, not problems, and must not stop the "all clear" dialog.
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
        self.plugins
            .iter()
            .filter(|p| !p.missing_masters.is_empty())
            .count()
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
pub fn report(
    view: &GameView<'_>,
    enabled_lower: &std::collections::HashSet<String>,
) -> Result<LootReport, LootError> {
    let (game_id, plugins) = (view.game_id, view.plugins);
    let (masterlist, prelude, userlist) = (view.masterlist, view.prelude, view.userlist);
    let (game_type, _repo) =
        loot_support(game_id).ok_or_else(|| LootError::Unsupported(game_id.to_string()))?;

    let mut game = Game::with_local_path(game_type, view.game_path, view.local_path)
        .map_err(|e| LootError::Loot(e.to_string()))?;
    // The report's conditions need the same view of the world as the sort, or a
    // plugin could be sorted by a rule the report then says does not apply.
    set_mod_dirs(&mut game, view.mod_dirs)?;

    {
        let db = game.database();
        let mut db = db
            .write()
            .map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        db.load_masterlist_with_prelude(masterlist, prelude)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        if let Some(ul) = userlist {
            if ul.is_file() {
                db.load_userlist(ul)
                    .map_err(|e| LootError::Loot(e.to_string()))?;
            }
        }
    }

    let paths: Vec<&Path> = plugins.iter().map(|(_, p)| p.as_path()).collect();
    // Whole plugins here too, for the same reason plus one of its own: the
    // report's CRC column comes from `plugin.crc()`, which libloot only computes
    // for a whole-plugin load - so with headers it was permanently blank.
    game.load_plugins(&paths)
        .map_err(|e| LootError::Loot(e.to_string()))?;
    game.load_current_load_order_state()
        .map_err(|e| LootError::Loot(e.to_string()))?;

    // A master is "present" only if it is enabled; a disabled master won't load and
    // will crash any enabled dependent, so it must count as missing (matches
    // `eidos_plugins::PluginList::missing_masters`).
    let present = enabled_lower;

    let general = {
        let db = game.database();
        let db = db
            .read()
            .map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        let msgs = db
            .general_messages(MergeMode::WithUserMetadata, EvalMode::Evaluate)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        convert_messages(&msgs)
    };

    let mut plugin_reports = Vec::new();
    let mut plugin_meta: HashMap<String, PluginMetadataBundle> = HashMap::new();
    for (name, _) in plugins {
        // MO2 only reports on enabled plugins; a disabled plugin's issues don't
        // matter because it won't load.
        if !enabled_lower.contains(&name.to_ascii_lowercase()) {
            continue;
        }
        let plugin = game.plugin(name).ok_or_else(|| {
            LootError::Loot(format!(
                "Unverified plugin {name}: unavailable after loading"
            ))
        })?;
        let missing_masters = plugin
            .masters()
            .map_err(|e| LootError::Loot(format!("Unverified masters for {name}: {e}")))?
            .into_iter()
            .filter(|m| !present.contains(&m.to_ascii_lowercase()))
            .collect::<Vec<_>>();

        let evaluated = {
            let db = game.database();
            let db = db
                .read()
                .map_err(|_| LootError::Loot("database lock poisoned".into()))?;
            db.plugin_metadata(name, MergeMode::WithUserMetadata, EvalMode::Evaluate)
                .map_err(|e| LootError::Loot(e.to_string()))?
        };

        let (mut messages, dirty) = match &evaluated {
            Some(meta) => (
                convert_messages(meta.messages()),
                meta.dirty_info().iter().map(convert_dirty).collect(),
            ),
            None => (Vec::new(), Vec::new()),
        };

        // The Plugins-tab bundle, from the SAME evaluated metadata as the report
        // row: bash tags on top of the messages/dirty above, plus the CRC libloot
        // computed while loading the header.
        let crc = plugin.crc();
        let mut bundle = match &evaluated {
            Some(meta) => bundle_from_metadata(meta, crc),
            None => PluginMetadataBundle {
                crc,
                ..PluginMetadataBundle::default()
            },
        };
        let (validity, findings) = record_validity(&plugin);
        bundle.record_validity = validity;
        bundle.messages.extend(findings.clone());
        messages.extend(findings);
        if !bundle.is_empty() {
            plugin_meta.insert(name.to_ascii_lowercase(), bundle);
        }

        let entry = PluginReport {
            name: name.clone(),
            missing_masters,
            messages,
            dirty,
        };
        if entry.has_issues() {
            plugin_reports.push(entry);
        }
    }

    Ok(LootReport {
        general,
        plugins: plugin_reports,
        plugin_meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use libloot::metadata::{
        Message, MessageType as LootMessageType, PluginCleaningData, PluginMetadata, Tag,
        TagSuggestion,
    };

    fn record_fixture(flags: u32, form_id: u32, version: f32) -> Vec<u8> {
        let mut b = vec![0; 42 + 48];
        b[..4].copy_from_slice(b"TES4");
        b[4..8].copy_from_slice(&18u32.to_le_bytes());
        b[8..12].copy_from_slice(&flags.to_le_bytes());
        b[20..22].copy_from_slice(&44u16.to_le_bytes());
        b[24..28].copy_from_slice(b"HEDR");
        b[28..30].copy_from_slice(&12u16.to_le_bytes());
        b[30..34].copy_from_slice(&version.to_le_bytes());
        b[34..38].copy_from_slice(&1u32.to_le_bytes());
        b[42..46].copy_from_slice(b"GRUP");
        b[46..50].copy_from_slice(&48u32.to_le_bytes());
        b[50..54].copy_from_slice(b"STAT");
        b[66..70].copy_from_slice(b"STAT");
        b[78..82].copy_from_slice(&form_id.to_le_bytes());
        b
    }

    #[test]
    fn whole_report_checks_light_medium_and_update_records() {
        for (id, flags, form, version, kind) in [
            ("skyrimse", 0x200, 0x1000, 1.7, "light"),
            ("starfield", 0x400, 0x800, 0.96, "medium"),
            ("starfield", 0x200, 0x01000800, 0.96, "update"),
        ] {
            let root = std::env::temp_dir()
                .join(format!("eidos-loot-records-{}-{kind}", std::process::id()));
            fs::create_dir_all(root.join("Data")).unwrap();
            fs::create_dir_all(root.join("local")).unwrap();
            let plugin = root.join("Data/Test.esp");
            let make = |form| {
                let mut b = record_fixture(flags, form, version);
                if kind == "update" {
                    let mut master = b"MAST".to_vec();
                    master.extend(11u16.to_le_bytes());
                    master.extend(b"Master.esm\0");
                    master.extend(b"DATA");
                    master.extend(8u16.to_le_bytes());
                    master.extend([0; 8]);
                    b[4..8].copy_from_slice(&(18u32 + master.len() as u32).to_le_bytes());
                    b.splice(42..42, master);
                }
                b
            };
            fs::write(&plugin, make(form)).unwrap();
            let masterlist = root.join("masterlist.yaml");
            let prelude = root.join("prelude.yaml");
            fs::write(&masterlist, "plugins: []\n").unwrap();
            fs::write(&prelude, "plugins: []\n").unwrap();
            let mut plugins = vec![("Test.esp".into(), plugin)];
            if kind == "update" {
                let master = root.join("Data/Master.esm");
                fs::write(&master, record_fixture(1, 0x800, version)).unwrap();
                plugins.push(("Master.esm".into(), master));
            }
            let local = root.join("local");
            let view = GameView {
                game_id: id,
                plugins: &plugins,
                game_path: &root,
                local_path: &local,
                mod_dirs: &[],
                masterlist: &masterlist,
                prelude: &prelude,
                userlist: None,
            };
            let enabled = std::collections::HashSet::from(["test.esp".into(), "master.esm".into()]);
            let report = report(&view, &enabled).unwrap();
            let expected_valid = kind == "medium";
            assert_eq!(
                report.plugins.iter().any(|p| p
                    .messages
                    .iter()
                    .any(|m| m.kind == MessageType::Error && m.text.contains(kind))),
                !expected_valid,
                "{kind}: {report:?}"
            );
            assert_eq!(
                report.plugin_meta["test.esp"].record_validity,
                Some(expected_valid)
            );
            fs::write(&plugins[0].1, make(0x800)).unwrap();
            let checked = super::report(&view, &enabled).unwrap();
            assert_eq!(checked.plugin_meta["test.esp"].record_validity, Some(true));
            assert_eq!(checked.error_count(), 0);
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn game_support_mapping() {
        assert!(matches!(
            loot_support("skyrimse"),
            Some((GameType::SkyrimSE, "skyrimse"))
        ));
        assert!(matches!(
            loot_support("fallout4"),
            Some((GameType::Fallout4, "fallout4"))
        ));
        assert_eq!(
            loot_support("morrowind"),
            Some((GameType::Morrowind, "morrowind"))
        );
        assert_eq!(
            loot_support("enderalse"),
            Some((GameType::SkyrimSE, "enderal"))
        );
        assert!(loot_support("nonsense").is_none());
        assert!(is_supported("skyrimse") && is_supported("oblivion"));
    }

    #[test]
    fn enderal_local_masterlist_is_parsed_with_the_skyrim_se_engine() {
        let root =
            std::env::temp_dir().join(format!("eidos-enderal-metadata-{}", std::process::id()));
        fs::create_dir_all(root.join("Data")).unwrap();
        fs::create_dir_all(root.join("local")).unwrap();
        let masterlist = root.join("masterlist.yaml");
        let prelude = root.join("prelude.yaml");
        fs::write(&masterlist,"globals:\n  - type: say\n    content: Enderal local metadata loaded\nplugins:\n  - name: 'Enderal - Forgotten Stories.esm'\n    group: default\n").unwrap();
        fs::write(&prelude, "plugins: []\n").unwrap();
        let view = GameView {
            game_id: "enderalse",
            game_path: &root,
            local_path: &root.join("local"),
            plugins: &[],
            mod_dirs: &[],
            masterlist: &masterlist,
            prelude: &prelude,
            userlist: None,
        };
        let result = report(&view, &Default::default()).unwrap();
        assert!(result
            .general
            .iter()
            .any(|m| m.text == "Enderal local metadata loaded"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn message_type_conversion_maps_each_level() {
        assert_eq!(convert_message_type(LootMessageType::Say), MessageType::Say);
        assert_eq!(
            convert_message_type(LootMessageType::Warn),
            MessageType::Warn
        );
        assert_eq!(
            convert_message_type(LootMessageType::Error),
            MessageType::Error
        );
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
            PluginCleaningData::new(0x1234_5678, "xEdit".into()).with_itm_count(3)
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
        let crc_only = PluginMetadataBundle {
            crc: Some(1),
            ..PluginMetadataBundle::default()
        };
        assert!(!crc_only.is_empty());
        assert!(!crc_only.needs_cleaning());
    }

    fn msg(kind: MessageType, text: &str) -> LootMessage {
        LootMessage {
            kind,
            text: text.to_string(),
        }
    }

    #[test]
    fn loot_report_counts_aggregate_general_and_per_plugin() {
        let report = LootReport {
            general: vec![
                msg(MessageType::Warn, "g warn"),
                msg(MessageType::Say, "g note"),
            ],
            plugin_meta: Default::default(),
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
        assert_eq!(
            report.missing_master_count(),
            1,
            "only A.esp is missing a master"
        );
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

// ---- case bridge -----------------------------------------------------------

/// Build a directory of symlinks that hands libloot the exact SPELLING the
/// masterlist asks for, for files that are on disk under a different one.
///
/// The masterlist is written on and for Windows, where file names have no case.
/// Linux does, and libloot's condition evaluator is a bare `exists()` -
/// `loot-condition-interpreter`'s `evaluate_file_path` is literally
/// `resolve_path(state, file_path).exists()`. So a rule written
/// `file("scripts/skse.pex")` does not see `Scripts/skse.pex`, the condition is
/// false, and a rule of the shape `not file(...)` fires a warning about
/// something that is correctly installed. That is how a complete SKSE install
/// gets reported as missing its scripts.
///
/// The fix belongs upstream, in the interpreter. Until it is there, this closes
/// the gap from the one side Eidos controls: the additional data paths it hands
/// libloot. For every literal path the masterlist mentions, if it does not
/// resolve exactly but does resolve ignoring case, a symlink is created here
/// under the masterlist's own spelling, pointing at the real file.
///
/// Deliberately driven by the masterlist rather than by the mods. Mirroring
/// every mod tree in every casing is unbounded and mostly useless; the
/// masterlist names a finite set of files it actually asks about - on a real
/// Skyrim SE setup, 1636 literal paths, of which seven needed a link.
///
/// # Layout
///
/// ```text
/// <out>/            <- root-relative links, reached as `../x` from below
/// <out>/data/       <- what the caller appends to `mod_dirs`
/// ```
///
/// Because `resolve_path` joins a relative path onto each base in turn,
/// `../d3dx9_42.dll` evaluated against `<out>/data` lands in `<out>`. The two
/// levels exist for exactly that.
///
/// Returns the paths bridged, for the caller to log or show.
pub fn build_case_bridge(
    masterlist: &Path,
    bases: &[PathBuf],
    game_path: &Path,
    out: &Path,
) -> std::io::Result<Vec<String>> {
    // Rebuilt from scratch every time: mods come and go, and a link left over
    // from a mod that has been removed would answer for a file that is no
    // longer there - the exact failure this is meant to prevent, inverted.
    let _ = fs::remove_dir_all(out);
    let data = out.join("data");
    fs::create_dir_all(&data)?;

    let Ok(text) = fs::read_to_string(masterlist) else {
        return Ok(Vec::new());
    };
    let mut bridged = Vec::new();
    for rel in masterlist_literal_paths(&text) {
        // `..` is the game root; anything else is data-relative.
        let (search_root, tail, link_dir) = match rel.strip_prefix("../") {
            Some(t) => (game_path.to_path_buf(), t.to_string(), out.to_path_buf()),
            None => (PathBuf::new(), rel.clone(), data.clone()),
        };
        let roots: Vec<PathBuf> = if search_root.as_os_str().is_empty() {
            bases.to_vec()
        } else {
            vec![search_root]
        };
        // Resolve each layer in priority order. A lower layer with exact casing
        // must not replace the higher layer's differently spelled winning file.
        let Some((real, exact)) = roots.iter().find_map(|b| {
            let exact = b.join(&tail);
            let real = if exact.exists() {
                Some(exact.clone())
            } else {
                resolve_ignoring_case(b, &tail)
            }?;
            Some((real, exact))
        }) else {
            continue;
        };
        if real == exact {
            continue;
        }
        let link = link_dir.join(&tail);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        if std::os::unix::fs::symlink(&real, &link).is_ok() {
            bridged.push(rel);
        }
    }
    Ok(bridged)
}

/// Where the caller should point libloot, given a bridge built at `out`.
/// Prepend this directory before mod roots so lower exact-case files cannot
/// shadow a bridge to a higher-priority winner.
pub fn case_bridge_data_dir(out: &Path) -> PathBuf {
    out.join("data")
}

/// Build the case bridge and put its winning paths first in LOOT's search order.
pub fn add_case_bridge(
    masterlist: &Path,
    bases: &mut Vec<PathBuf>,
    game_path: &Path,
    out: &Path,
) -> std::io::Result<Vec<String>> {
    let bridged = build_case_bridge(masterlist, bases, game_path, out)?;
    if !bridged.is_empty() {
        bases.insert(0, case_bridge_data_dir(out));
    }
    Ok(bridged)
}

/// Every literal path a masterlist condition names. Patterns containing regex
/// metacharacters are skipped: those are matched by the interpreter against
/// directory listings, which it already does case-insensitively.
fn masterlist_literal_paths(text: &str) -> Vec<String> {
    const FNS: [&str; 7] = [
        "file(",
        "version(",
        "product_version(",
        "checksum(",
        "readable(",
        "is_executable(",
        "active(",
    ];
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let mut rest = line;
        while let Some(at) = FNS
            .iter()
            .filter_map(|f| rest.find(f).map(|i| (i, f.len())))
            .min()
        {
            rest = &rest[at.0 + at.1..];
            let Some(open) = rest.find('"') else { break };
            let after = &rest[open + 1..];
            let Some(close) = after.find('"') else { break };
            let p = &after[..close];
            rest = &after[close + 1..];
            if p.is_empty() || p.contains(['[', ']', '(', ')', '*', '+', '?', '\\']) {
                continue;
            }
            if !out.iter().any(|q| q == p) {
                out.push(p.to_string());
            }
        }
    }
    out
}

/// Walk `rel` under `base` comparing each component without case, and return the
/// REAL path when every component matches. `None` as soon as one does not.
fn resolve_ignoring_case(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        let hit = fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    cur.exists().then_some(cur)
}

#[cfg(test)]
mod case_bridge_tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("eidos-cb-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn only_literal_paths_are_taken_from_the_masterlist() {
        let ml = r#"
    condition: 'not file("scripts/skse.pex") and (file("../skse64_loader.exe"))'
    condition: 'version("SKSE/Plugins/EngineFixes.dll", "7.0.0", >=)'
    condition: 'file("SKSE/Plugins/([^\.]+\.dll)")'
    condition: 'checksum("Plugin.esp", DEADBEEF)'
"#;
        let got = masterlist_literal_paths(ml);
        assert!(got.contains(&"scripts/skse.pex".to_string()));
        assert!(got.contains(&"../skse64_loader.exe".to_string()));
        assert!(got.contains(&"SKSE/Plugins/EngineFixes.dll".to_string()));
        assert!(got.contains(&"Plugin.esp".to_string()));
        // The regex form is matched against directory listings by the
        // interpreter, which already ignores case there - bridging it would be
        // both wrong and unbounded.
        assert!(
            !got.iter().any(|p| p.contains('[')),
            "regex patterns must not be treated as paths: {got:?}"
        );
        // Each path once, however many rules mention it.
        let mut sorted = got.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), got.len());
    }

    #[test]
    fn a_file_whose_case_differs_gets_a_link_under_the_masterlists_spelling() {
        let root = tmp("link");
        let mod_dir = root.join("mods/SKSE");
        fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
        fs::write(mod_dir.join("Scripts/skse.pex"), b"pex").unwrap();
        let ml = root.join("masterlist.yaml");
        fs::write(&ml, "condition: 'not file(\"scripts/skse.pex\")'").unwrap();

        let out = root.join("bridge");
        let bridged = build_case_bridge(&ml, &[mod_dir.clone()], &root.join("game"), &out).unwrap();
        assert_eq!(bridged, vec!["scripts/skse.pex".to_string()]);

        // What libloot will actually do: join the relative path onto the base.
        let seen = case_bridge_data_dir(&out).join("scripts/skse.pex");
        assert!(
            seen.exists(),
            "libloot's exists() must succeed through the link"
        );
        assert_eq!(fs::read(&seen).unwrap(), b"pex", "and read the real file");
    }

    #[test]
    fn a_file_already_spelled_correctly_is_left_alone() {
        // A link here would be a second answer to a question that already had
        // one, and would go stale independently of the file it duplicates.
        let root = tmp("exact");
        let mod_dir = root.join("mods/M");
        fs::create_dir_all(mod_dir.join("scripts")).unwrap();
        fs::write(mod_dir.join("scripts/skse.pex"), b"x").unwrap();
        let ml = root.join("m.yaml");
        fs::write(&ml, "condition: 'file(\"scripts/skse.pex\")'").unwrap();
        let out = root.join("b");
        assert!(build_case_bridge(&ml, &[mod_dir], &root.join("game"), &out)
            .unwrap()
            .is_empty());
        assert!(!case_bridge_data_dir(&out).join("scripts/skse.pex").exists());
    }

    #[test]
    fn a_game_root_path_lands_one_level_above_the_data_dir() {
        // `../x` is evaluated by joining it onto the base, so it has to resolve
        // from the directory the caller hands libloot - not inside it.
        let root = tmp("root");
        let game = root.join("game");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("SKSE64_Loader.exe"), b"exe").unwrap();
        let ml = root.join("m.yaml");
        fs::write(&ml, "condition: 'file(\"../skse64_loader.exe\")'").unwrap();

        let out = root.join("b");
        let bridged = build_case_bridge(&ml, &[], &game, &out).unwrap();
        assert_eq!(bridged.len(), 1);
        let base = case_bridge_data_dir(&out);
        assert!(
            base.join("../skse64_loader.exe").exists(),
            "reached as libloot reaches it"
        );
    }

    #[test]
    fn a_missing_file_is_not_invented() {
        // The bridge must never make a condition true that was honestly false:
        // that would suppress a warning the user needs.
        let root = tmp("absent");
        let ml = root.join("m.yaml");
        fs::write(&ml, "condition: 'file(\"scripts/nothere.pex\")'").unwrap();
        let out = root.join("b");
        assert!(build_case_bridge(&ml, &[root.clone()], &root, &out)
            .unwrap()
            .is_empty());
        assert!(!case_bridge_data_dir(&out)
            .join("scripts/nothere.pex")
            .exists());
    }

    #[test]
    fn the_case_bridge_preserves_priority_in_evaluated_loot_conditions() {
        let root = tmp("priority");
        let high = root.join("high");
        let low = root.join("low");
        for dir in [&high, &low, &root.join("Data"), &root.join("local")] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(high.join("SCRIPT.PEX"), b"x").unwrap();
        fs::write(low.join("script.pex"), b"y").unwrap();
        let masterlist = root.join("masterlist.yaml");
        let prelude = root.join("prelude.yaml");
        fs::write(&masterlist,
            "globals:\n  - type: say\n    content: Winning mod was inspected\n    condition: 'checksum(\"script.pex\", 8CDC1683)'\n",
        ).unwrap();
        fs::write(&prelude, "plugins: []\n").unwrap();
        let out = root.join("bridge");
        let mut dirs = vec![high, low];
        let bridged = add_case_bridge(&masterlist, &mut dirs, &root, &out).unwrap();
        assert_eq!(bridged, ["script.pex"]);
        let view = GameView {
            game_id: "skyrimse",
            game_path: &root,
            local_path: &root.join("local"),
            plugins: &[],
            mod_dirs: &dirs,
            masterlist: &masterlist,
            prelude: &prelude,
            userlist: None,
        };
        let result = report(&view, &Default::default()).unwrap();
        assert_eq!(
            result.general.len(),
            1,
            "LOOT must inspect the union's winning file"
        );
        assert_eq!(result.general[0].text, "Winning mod was inspected");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_rebuild_drops_links_for_files_that_are_gone() {
        // A mod removed between two sorts must not keep answering through a
        // stale link - that is this bug inverted, and worse.
        let root = tmp("stale");
        let mod_dir = root.join("mods/M");
        fs::create_dir_all(mod_dir.join("Scripts")).unwrap();
        fs::write(mod_dir.join("Scripts/skse.pex"), b"x").unwrap();
        let ml = root.join("m.yaml");
        fs::write(&ml, "condition: 'file(\"scripts/skse.pex\")'").unwrap();
        let out = root.join("b");
        build_case_bridge(&ml, &[mod_dir.clone()], &root, &out).unwrap();
        assert!(case_bridge_data_dir(&out).join("scripts/skse.pex").exists());

        fs::remove_dir_all(&mod_dir).unwrap();
        let again = build_case_bridge(&ml, &[], &root, &out).unwrap();
        assert!(again.is_empty());
        assert!(!case_bridge_data_dir(&out).join("scripts/skse.pex").exists());
    }
}
