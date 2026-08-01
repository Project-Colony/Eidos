# Supporting games beyond Bethesda

A design note, not a changelog. It covers one question: what has to change for a
game declared in a `<id>.toml` to actually *install its mods*, and how much of that
can stay declarative.

Short answer: one function, five call sites, three new fields. The engine is not
involved at any point.

## The state of play

The VFS engine has no game knowledge at all. Searching `eidos-core` and
`eidos-fuse` for `skyrim`, `fallout`, `bethesda`, `gamebryo`, `plugins.txt` or
`.esp` returns only test fixture filenames. The engine stacks directories and
resolves reads; which game those directories belong to never reaches it.

The catalogue is already declarative.
[`eidos-gamedef`](../../crates/eidos-gamedef/src/lib.rs) mirrors MO2's
`IPluginGame` schema, and a `<id>.toml` in `~/.config/eidos/games/` joins the
registry with no recompile, exactly like MO2's `basic_games` plugin.
[`internals/adding-games.md`](../internals/adding-games.md) documents it.

So detection works for any Steam game, and the file union mounts over any
`data_dir`. What does not work is the step between downloading a mod and having
it on disk.

## The actual blocker

[`ArchiveTree::data_looks_valid`](../../crates/eidos-install/src/lib.rs) is MO2's
`ModDataChecker`, and it is hardcoded to the Gamebryo family:

```rust
const GAMEBRYO_FOLDERS: &[&str] = &["fonts", "interface", "meshes", "textures", ...];
const GAMEBRYO_SUFFIXES: &[&str] = &["esp", "esm", "esl", "bsa", "ba2", "modgroups", "ini"];
```

The comment above it already says "Per-game checkers can specialise this later."
This note is that later.

Four public methods sit on top of it, so the hardcoding propagates:

| Method | What it drives |
|---|---|
| `simple_archive_base` | the automatic install path (strip wrappers, find the mod root) |
| `bain_subpackages` | Wrye Bash multi-package archives |
| `root_builder_split` | the `Root/` half of an ENB or preloader archive |
| `root_looks_valid` | the live "this looks valid" feedback in the manual picker |

When `data_looks_valid` says Invalid at every level, `simple_archive_base` returns
`None`, `open_archive` returns `NotSimple`, and the user gets the manual picker
with the tree and no guidance. For every non-Bethesda mod, every time.

That is the whole gap. The game is declarable, the mount is correct, and the
installer refuses the archive.

### Worked example: Stardew Valley

A SMAPI mod archive downloaded from Nexus:

```
ContentPatcher-1915-2-5-0/
  ContentPatcher/
    manifest.json
    content.json
    [CP] Foo/
```

`data_looks_valid` on the top level: no Gamebryo folder, no Gamebryo suffix,
Invalid. `single_subdir` descends into `ContentPatcher-1915-2-5-0/`. Invalid.
Descends into `ContentPatcher/`. Invalid (`manifest.json` matches nothing).
Recursion ends, `None`, manual picker.

Now suppose we simply add `manifest.json` to a per-game file list. The descent
runs again, and this time the third level *is* valid, so the base is
`ContentPatcher-1915-2-5-0/ContentPatcher/` and the mod root becomes the contents
of `ContentPatcher/`. Deployed, that is:

```
Mods/manifest.json
Mods/content.json
Mods/[CP] Foo/
```

Which SMAPI does not load. It wants `Mods/ContentPatcher/manifest.json`.

**This is the finding that shapes the design.** A folder-and-suffix list is
sufficient for Bethesda because a Bethesda mod's unit of install is a *set of
files* merged into `Data`. For loader games it is a *named folder*, and the valid
level is one above the marker, not the marker's own level. Any schema that only
lists names gets Stardew backwards.

(MO2 handles this with per-game Python checker classes that can also *fix* the
tree rather than just judge it. I could not read `basic_games` from this machine
to quote its Stardew checker, so the above is derived from what SMAPI requires on
disk, not from MO2's source.)

## Proposed schema

Three fields on `GameDef`, all optional, all defaulting to today's Gamebryo
behaviour so no existing game changes.

```toml
# what makes a directory level a valid mod root
valid_folders  = ["bepinex", "plugins", "patchers", "config"]
valid_suffixes = ["dll"]
valid_files    = ["manifest.json"]     # exact filenames, not extensions
```

`valid_files` is new information the current checker cannot express at all:
SMAPI keys on `manifest.json`, 7 Days to Die on `ModInfo.xml`, Factorio on
`info.json`. None of those are extensions and none are folders.

And one field for the Stardew shape:

```toml
# the mod's unit of install is a named folder, so the valid level is the one
# whose CHILD directories carry the markers
mod_unit = "folder"     # default: "files"
```

With `mod_unit = "folder"`, `data_looks_valid` matches markers at depth 1 instead
of depth 0. The Stardew descent then stops at `ContentPatcher-1915-2-5-0/`,
whose child `ContentPatcher/` holds `manifest.json`, and deploys
`Mods/ContentPatcher/manifest.json`. Correct.

### Per-family data

What the three lists look like for the families worth targeting. This is the
whole per-game cost once the mechanism exists.

| Family | `valid_folders` | `valid_suffixes` | `valid_files` | `mod_unit` |
|---|---|---|---|---|
| Gamebryo / Creation | (today's list) | esp, esm, esl, bsa, ba2 | - | files |
| BepInEx (Unity) | bepinex, plugins, patchers, config, core | dll | - | files |
| SMAPI (Stardew) | - | - | manifest.json | **folder** |
| Cyberpunk 2077 | archive, bin, engine, r6, mods, red4ext | archive | - | files |
| Unreal `~mods` | - | pak, utoc, ucas | - | files |
| 7 Days to Die | - | - | ModInfo.xml | **folder** |

Two of six need the depth flag, which is why it belongs in the first cut rather
than a follow-up.

### Deliberately not in the schema

**Rebase rules.** Many BepInEx mods ship as a bare `Foo.dll` and expect the user
to drop it into `BepInEx/plugins/` by hand. Handling that means the checker must
*rewrite* the tree, not just judge it (MO2's `ModDataChecker::fix`). It is a real
need and a strictly larger change: `data_looks_valid` returns a verdict, and
teaching it to return a transformation touches the install path, not just the
classifier. `CheckReturn` already has room for a `Fixable` variant when we want
it. Not now.

**Anything BG3.** Its mods live outside the game directory
(`%LOCALAPPDATA%/Larian Studios/.../Mods`) and its load order is an XML file
(`modsettings.lsx`) that has to be rewritten. That needs a second mount surface
and a fifth `LoadOrder` variant. It is a separate piece of work and it *would*
touch more than the checker.

**`launch_env`.** BepInEx under Proton needs `WINEDLLOVERRIDES="winhttp=n,b"` to
load at all. The existing `ScriptExtender { launcher, loader }` swap covers SMAPI
(swap `Stardew Valley.exe` for `StardewModdingAPI.exe`) but cannot express an
environment variable. Needed before Unity games actually *run* modded, not before
their mods install. Separate change, separate PR.

## What it touches in code

`eidos-install` does not currently depend on `eidos-gamedef`. `eidos-gamedef` is a
leaf (serde + toml only), so adding the edge creates no cycle.

The shape that shipped is a narrow value type rather than `&GameDef` passed
around:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRules {
    pub folders: &'static [&'static str],
    pub suffixes: &'static [&'static str],
}

impl Default for LayoutRules { /* the Gamebryo set */ }
impl From<&eidos_gamedef::GameDef> for LayoutRules { /* empty list => default */ }
impl LayoutRules { pub fn for_game(game_id: &str) -> LayoutRules { … } }
```

`&'static [&'static str]` rather than `Vec<String>` because that is already
`GameDef`'s own shape, including for TOML games (the descriptor leaks its strings
at load). That makes the type `Copy`, so `ArchiveTree`'s methods take it by value
and threading it costs nothing. Matching is `eq_ignore_ascii_case`, so a
descriptor may spell its rules the way the game's documentation does.

Call sites, all of them:

| Site | Change |
|---|---|
| `install.rs:256, 288, 323` (`install_extracted`) | thread the rules |
| `install.rs:1180, 1183, 1191` (`open_archive`) | **new parameter** |
| `gui/main.rs:9906` (`root_looks_valid`) | thread the rules |
| `lib.rs:224, 241, 273, 323, 351` | internal recursion |

Only `open_archive` gains a parameter. Every other public entry point
(`install_extracted`, `install_bain`, `install_manual`, `finish_fomod`,
`install_archive_with_policy`) **already takes the game id**, and the GUI already
passes `def.id` into all of them. The identity is threaded; it was just never
used for classification.

Test cost: the existing checker tests in `eidos-install/src/lib.rs` all become
`&LayoutRules::default()` calls and keep asserting exactly what they assert today.

## What cannot become data

Two `match` arms on game ids will stay in Rust, and that is correct:

- [`eidos-plugins/src/lib.rs:81`](../../crates/eidos-plugins/src/lib.rs) maps to
  `esplugin::GameId`
- [`eidos-loot/src/lib.rs:65`](../../crates/eidos-loot/src/lib.rs) maps to LOOT's
  `GameType`

Both are enum variants owned by third-party crates. A TOML string cannot name a
variant that does not exist, and a game LOOT does not support cannot be given
LOOT support by declaring it.

Two others *are* data and could migrate later, though neither blocks anything:

- [`eidos-gamefeatures/src/se_log.rs:52`](../../crates/eidos-gamefeatures/src/se_log.rs)
  hardcodes each script extender's log path
- [`eidos-gamefeatures/src/lib.rs:77`](../../crates/eidos-gamefeatures/src/lib.rs)
  hardcodes BSA invalidation per engine

## Two things found in passing

**`meta.ini` records the wrong game name.** The GUI passes `def.id` (`skyrimse`)
into the installer's `game_name`, which writes `gameName=skyrimse`. MO2 writes the
short name (`gameName=SkyrimSE`), and `eidos-instance/src/meta.rs` even has a test
asserting `SkyrimSE` when read from a Nexus sidecar. Nothing reads the field back
for behaviour, so this is interop-only: MO2 opening an Eidos-installed mod sees an
id it does not recognise. One-line fix (`def.short_name`), unrelated to this work,
worth doing while the install path is open.

**The tab bar is unconditional.**
[`gui/main.rs:8920`](../../crates/eidos-gui/src/main.rs) pushes every tab
regardless of game, so a game with `load_order = "None"` shows an empty Plugins
tab. Cosmetic, but it is the first thing a Stardew user would notice.

## Staging

**0. Done.** A characterisation test, `crates/eidos-install/tests/corpus.rs`,
freezing the checker's verdicts on 49 real mod-root shapes and 7 downloaded
archives taken from a live Skyrim SE instance and anonymised
(`corpus/generate.py`). It passed on unmodified code before anything else moved.
Not a design step - the point is that stages 1 and 2 claim to change no behaviour,
and this is what turns that claim into something a test can refuse.

**1. Done.** `LayoutRules` with `Default` = the Gamebryo lists, and the five
checker methods reading it. Every call site passes `LayoutRules::default()`. The
golden record did not move.

**2. Done.** `valid_folders` / `valid_suffixes` on `GameDef` (empty on all eleven
built-ins) and in the TOML schema, `From<&GameDef>` falling back to the default,
`LayoutRules::for_game`, and the rules threaded from the game id at every entry
point. `open_archive` gained its parameter; nothing else did, because every other
public entry already took the game. Guarded by
`every_builtin_game_keeps_the_default_vocabulary`. The golden record still did not
move, and it is now recorded through `for_game("skyrimse")` rather than a
hand-built default, so it proves the real lookup path.

The `game_name` parameter was renamed `game_id` throughout: it has always carried
the Eidos id (the GUI passes `def.id`), never MO2's short name, and now that it
also selects the rules, a caller passing `SkyrimSE` would silently get the default
instead of an error.

**3. Done, with a different game than planned.** Stardew was the worked example
here because it is the *hardest* shape, not because anyone wanted it. Asked for a
game that mattered instead, the answer was Stellar Blade, which is installed on
the machine this was built on and already modded - so the descriptor could be read
off a real install and a real set of downloads rather than guessed.

Stellar Blade ships as a built-in game: Unreal, `LoadOrder::None`, `data_dir =
SB/Content/Paks`, `valid_folders = ["~mods", "logicmods"]`, `valid_suffixes =
["pak", "utoc", "ucas"]`. Thirteen real archives went into the corpus. Under the
Gamebryo vocabulary **none** of them resolves; under the game's own, **nine of
thirteen install with no question asked**. `the_game_vocabulary_is_what_makes_unreal_archives_install`
asserts both numbers, so a regression in either direction has to be looked at.

The four that still route to the manual picker each do so correctly: two are
variant bundles whose options are nested archives, one is a lone `.json`, and one
is a UE4SS script mod whose tree is install-root-relative.

Building it changed one design decision. The first cut had `From<&GameDef>` fall
back per list, so a game declaring only `valid_suffixes` kept the Gamebryo
folders. That is wrong for exactly the game it was written for: Stellar Blade has
no folder vocabulary, and inheriting one would let an archive shipping `textures/`
read as a valid mod root. A game now declares its vocabulary as a whole.

`mod_unit` was NOT built. Nothing in the Unreal family needs it - a `.pak` is
recognised by extension wherever it sits - and it belongs with the first game that
actually has folder-shaped mods rather than as speculative schema.

**4. Done.** `meta.ini` records MO2's short name (`SkyrimSE`) instead of the Eidos
id, with a sidecar still winning and an unknown id falling back to itself. And the
Plugins tab is only offered where `GameSpec::for_id` resolves, which excludes both
Stellar Blade (no plugin system) and Oblivion/Morrowind (timestamp order, which
Eidos does not manage either) - all three used to show a tab that opened an empty
list. `app.tab` outlives a game switch, so `effective_tab` also stops a
remembered-but-invisible tab from deciding which panel draws.

**Also outstanding**, surfaced by Stellar Blade rather than by design: UE4SS is
this game's script extender, and its Lua mods ship an install-root-relative tree.
Eidos mounts `Root/` natively, so the mechanism exists; what is missing is
`root_builder_split` recognising a game-root-relative archive that leads with the
game's own directory (`SB/`) rather than with `Data/` or `Root/`.

## Open question

`mod_unit = "folder"` is the minimum that makes Stardew correct, and it is a
binary. If a family turns up whose unit is two levels down, it becomes a depth
integer instead. I would rather ship the binary and widen it when something
forces the issue than guess at a depth today.
