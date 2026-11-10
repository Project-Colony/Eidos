# Supporting games beyond Bethesda

A design note, not a changelog. It covers one question: what has to change for a
game declared in a `<id>.toml` to actually *install its mods*, and how much of that
can stay declarative.

Short answer: one function, five call sites, four new fields. The engine is not
involved at any point.

## Status (2026-11-10)

**Two of the four proposed fields shipped in [#5](https://github.com/Project-Colony/Eidos/pull/5)
on 2026-08-03; `valid_files` and `mod_unit` followed.** The section below called "The blocker, as it stood before #5" describes a state
that no longer exists; it is kept because the reasoning still explains the design,
but read it as history.

| | State |
|---|---|
| `valid_folders` | **shipped** - on `GameDef` and in the TOML schema |
| `valid_suffixes` | **shipped** - on `GameDef` and in the TOML schema |
| `valid_files` | **shipped** (`stardewvalley.toml`, `7daystodie.toml` use them) |
| `mod_unit` | **shipped** (`stardewvalley.toml`, `7daystodie.toml` use them) |
| `known_tools` | **shipped** in the TOML schema (2026-08-25), not part of the original proposal |
| `nexus_sources` | **shipped** in 1.23, not part of the original proposal: other Nexus domains whose mods the game accepts (`enderalse.toml` lists `skyrimspecialedition`) |

So a game whose mod root is marked by a FOLDER, an EXTENSION or a FILE is
declarable today, with no recompile: Gamebryo/Creation, BepInEx/Unity,
Cyberpunk, Unreal `~mods`, and the FILE markers (`manifest.json`,
`ModInfo.xml`, `info.json`). A game whose unit of install is a named FOLDER
rather than loose files is declarable too, through `mod_unit`: that is what
SMAPI/Stardew and 7 Days to Die use.

Four of the 15 built-in games are not Bethesda games:

- Stellar Blade deploys paks into `SB/Content/Paks/~mods`; UE4SS mods travel
  through `Root/`.
- Stardew Valley swaps in SMAPI (`StardewModdingAPI.exe`), which the user
  installs. The descriptor covers the Windows/Proton layout only.
- 7 Days to Die covers the install-local `Mods` folder only. `UserDataFolder/Mods`
  and `~/.local/share/7DaysToDie/Mods` are not projected.
- Crimson Desert takes code mods (ASI plugins, Ultimate ASI Loader proxies,
  ReShade) in `bin64`. A PAZ data mod that needs a new group and a rebuilt
  `meta/0.papgt` is not handled. A mod that replaces a vanilla group in place
  goes through `Root/`.

## The state of play

The VFS engine knows plugin extensions for one purpose and nothing else about
games. `eidos-fuse/src/timestamps.rs` projects profile-owned modification times
for `.esp`/`.esm`/`.esl` names in the Data root, which is how timestamp-ordered
games (Oblivion, Morrowind) get their load order. Apart from that, the engine
stacks directories and resolves reads; which game those directories belong to
never reaches it.

The catalogue is already declarative.
[`eidos-gamedef`](../../crates/eidos-gamedef/src/lib.rs) mirrors MO2's
`IPluginGame` schema, and a `<id>.toml` in `~/.config/Colony/Eidos/games/` joins the
registry with no recompile, exactly like MO2's `basic_games` plugin.
[`internals/adding-games.md`](../internals/adding-games.md) documents it.

So detection works for any Steam game, and for GOG and Epic copies that Heroic
or Legendary installed when the descriptor lists their ids (`gog_ids`,
`epic_ids`), and the file union mounts over any `data_dir`. Since 1.24 the
union can also mount a copy of the game folder kept in the instance in place of
the install (`eidos stock-game`), except for a game that keeps its plugin list
in the game folder (Morrowind, and Oblivion with `bUseMyGamesDirectory=0`). What
does not work is the step between downloading a mod and having it on disk.

## The blocker, as it stood before #5

[`ArchiveTree::data_looks_valid`](../../crates/eidos-install/src/lib.rs) is MO2's
`ModDataChecker`. It **was** hardcoded to the Gamebryo family; it now takes a
`LayoutRules` built from the game's descriptor, and the constants below survive
only as the default for a game that declares nothing:

```rust
const GAMEBRYO_FOLDERS: &[&str] = &["fonts", "interface", "meshes", "textures", ...];
const GAMEBRYO_SUFFIXES: &[&str] = &["esp", "esm", "esl", "bsa", "ba2", "modgroups", "ini"];
```

The comment above it said "Per-game checkers can specialise this later." This
note is that later.

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

That was the whole gap. It is closed for folder- and extension-marked games, and
since `valid_files` and `mod_unit` shipped, for the two families below whose
marker is a file or whose unit is a folder: before that, the game was declarable
and the mount was correct, but the installer refused the archive.

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
tree rather than just judge it. The analysis above was derived from what SMAPI
requires on disk. `stardewvalley.toml` now cites the sources: MO2's
`basic_games` `game_stardewvalley.py` for the ids and loader, and SMAPI's
`docs/technical/mod-package.md` for the package layout.)

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
| Gamebryo / Creation | (today's list) | esp, esm, esl, bsa, ba2, modgroups, ini | - | files |
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
load at all. Since 1.22, `winhttp` and `wininet` are among the shadowed DLL names
Eidos watches for: a mod that ships `winhttp.dll` at its top level or in `Root/`
gets `winhttp=n,b` at launch, which is what BepInEx needs. `tools.ini` also takes
a per-tool `env=`. The `ScriptExtender { launcher, loader }` swap covers SMAPI
(swap `Stardew Valley.exe` for `StardewModdingAPI.exe`). Since 1.24 a profile
can carry its own launch variables and wrappers in
`profiles/<name>/launch-env.ini` (`env=NAME=value` and `wrapper=<command line>`
lines), so a game that needs another variable can get it one profile at a
time. What is still missing is a per-game launch environment in the TOML
schema: `GameDef` has no environment field.

## What it touches in code

This section is the design as it stood at #5. Then, `eidos-install` did not
depend on `eidos-gamedef`, and `eidos-gamedef` was a leaf (serde + toml only), so
adding the edge created no cycle. Today `eidos-install` depends on
`eidos-gamedef`, and `eidos-gamedef` depends on `eidos-paths` besides serde and
toml.

The shape that shipped is a narrow value type rather than `&GameDef` passed
around. At #5 it had two fields; it now also has `files`, `mod_unit` and
`data_dir`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRules {
    pub folders: &'static [&'static str],
    pub suffixes: &'static [&'static str],
    pub files: &'static [&'static str],
    pub mod_unit: ModUnit,
    pub data_dir: &'static str,
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

Call sites, all of them, as of #5 (the paths and line numbers have moved since):

| Site | Change |
|---|---|
| `install.rs:256, 288, 323` (`install_extracted`) | thread the rules |
| `install.rs:1180, 1183, 1191` (`open_archive`) | **new parameter** |
| `gui/main.rs:9906` (`root_looks_valid`) | thread the rules |
| `lib.rs:224, 241, 273, 323, 351` | internal recursion |

`install.rs` is now the module `crates/eidos-install/src/install/`:
`open_archive` is at `install/mod.rs:215` and `install_extracted` at
`install/simple.rs:92`. The GUI calls `root_looks_valid` from
`crates/eidos-gui/src/modinfo/install.rs:744`.

Only `open_archive` gains a parameter. Every other public entry point
(`install_extracted`, `install_bain`, `install_manual`, `finish_fomod`,
`install_archive_with_policy`) **already takes the game id**, and the GUI already
passes `def.id` into all of them. The identity is threaded; it was just never
used for classification.

Test cost: the existing checker tests in `eidos-install/src/lib.rs` all become
`&LayoutRules::default()` calls and keep asserting exactly what they assert today.

## What cannot become data

Two `match` arms on game ids will stay in Rust, and that is correct:

- [`eidos-plugins/src/lib.rs:85`](../../crates/eidos-plugins/src/lib.rs) maps to
  `esplugin::GameId`
- [`eidos-loot/src/lib.rs:63`](../../crates/eidos-loot/src/lib.rs) maps to LOOT's
  `GameType`

Both are enum variants owned by third-party crates. A TOML string cannot name a
variant that does not exist, and a game LOOT does not support cannot be given
LOOT support by declaring it.

Two others *are* data and could migrate later, though neither blocks anything:

- [`eidos-gamefeatures/src/se_log.rs:49`](../../crates/eidos-gamefeatures/src/se_log.rs)
  hardcodes each script extender's log path
- [`eidos-gamefeatures/src/lib.rs:86`](../../crates/eidos-gamefeatures/src/lib.rs)
  hardcodes BSA invalidation per engine

## Two things found in passing

Both are fixed, by staging step 4 below.

**`meta.ini` recorded the wrong game name.** Fixed in step 4. The GUI passed
`def.id` (`skyrimse`) into the installer's `game_name`, which wrote
`gameName=skyrimse`. MO2 writes the short name (`gameName=SkyrimSE`), and
`eidos-instance/src/meta.rs` even has a test asserting `SkyrimSE` when read from
a Nexus sidecar. Nothing reads the field back for behaviour, so this was
interop-only: MO2 opening an Eidos-installed mod saw an id it did not recognise.
`install/meta.rs` now writes `GameDef::short_name`.

**The tab bar was unconditional.** Fixed in step 4. `gui/main.rs:8920` pushed
every tab regardless of game, so a game with `load_order = "None"` showed an empty
Plugins tab. The tab bar now lives in
[`modinfo/main_screen.rs`](../../crates/eidos-gui/src/modinfo/main_screen.rs)
and offers Plugins only where `GameSpec::for_id` resolves.

## Staging

**0. Done.** A characterisation test, `crates/eidos-install/tests/corpus.rs`,
freezing the checker's verdicts on 49 real mod-root shapes (the count at this
stage) and 7 downloaded archives taken from a live Skyrim SE instance and
anonymised (`corpus/generate.py`). The corpus now holds 51 Skyrim SE mod roots,
7 Skyrim SE archives, 28 Stellar Blade mod roots and 13 Stellar Blade archives. It passed on unmodified code before anything else moved.
Not a design step - the point is that stages 1 and 2 claim to change no behaviour,
and this is what turns that claim into something a test can refuse.

**1. Done.** `LayoutRules` with `Default` = the Gamebryo lists, and the five
checker methods reading it. Every call site passes `LayoutRules::default()`. The
golden record did not move.

**2. Done.** `valid_folders` / `valid_suffixes` on `GameDef` (empty on all eleven
built-ins at the time; of today's 15, the four non-Bethesda games declare their
own vocabulary) and in the TOML schema, `From<&GameDef>` falling back to the default,
`LayoutRules::for_game`, and the rules threaded from the game id at every entry
point. `open_archive` gained its parameter; nothing else did, because every other
public entry already took the game. Guarded by
`a_game_that_declares_nothing_keeps_the_gamebryo_vocabulary`, which names the 11
Bethesda ids outright so that a new game cannot shrink what it covers, and by
`the_corpus_game_resolves_to_the_frozen_vocabulary` in the corpus test. The golden record still did not
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

Stellar Blade ships as a built-in game: Unreal, `LoadOrder::None`. At this step
the descriptor had `data_dir = SB/Content/Paks` and `valid_folders = ["~mods",
"logicmods"]`. It now has `data_dir = "SB/Content/Paks/~mods"`, no
`valid_folders`, and only `valid_suffixes = ["pak", "utoc", "ucas"]`.
`stellarblade.toml` gives the reason: the mod frameworks read `~mods` by name, so
a mod deployed one level higher loads its pak and adds nothing, and a `~mods`
folder marker would let `~mods/foo.pak` survive as `~mods/~mods/foo.pak`.
Thirteen real archives went into the corpus. Under the
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

Superseded: `mod_unit` was not built in this step, but it has shipped since, for
Stardew and 7 Days to Die.

**4. Done.** `meta.ini` records MO2's short name (`SkyrimSE`) instead of the Eidos
id, with a sidecar still winning and an unknown id falling back to itself. And the
Plugins tab is only offered where `GameSpec::for_id` resolves, which excludes both
Stellar Blade (no plugin system) and Oblivion/Morrowind (timestamp order; that
half is superseded, since `GameSpec::for_id` covers both now) - all three used
to show a tab that opened an empty list. `app.tab` outlives a game switch, so `effective_tab` also stops a
remembered-but-invisible tab from deciding which panel draws.

**5. Done.** UE4SS mods, which are Stellar Blade's equivalent of SKSE plugins,
ship a tree addressed from the game INSTALL root (`SB/Binaries/Win64/ue4ss/...`)
rather than from the mod-merge root. `root_builder_split` now recognises an
archive leading with the game's own directory.

Which directory that is comes from `data_dir`: a mod root nested below the install
root (`SB/Content/Paks`) names one, a mod root that IS the install root's child
(`Data`, `Data Files`) does not. Every Bethesda game falls in the second group, so
the branch is unreachable for them - asserted, not assumed.

The real archives forced one thing further. CustomNanosuitSystem ships a UE4SS mod
**and** a pak, both under `SB/`. Routing the whole directory to `Root/` would put
the pak back at `<game>/SB/Content/Paks`, which is exactly where the Data union is
mounted, so the pak would be shadowed and never served: a mod that installs
cleanly and works by half. The split is therefore cut along the `data_dir` path -
`SB/Content/Paks` became the Data half, `SB/Binaries` travels through `Root/` at
its own path. Since the deploy root moved, the Data half is
`SB/Content/Paks/~mods`, and a `LogicMods` half under `SB/Content/Paks` travels
through `Root/` too. `game_dir` is still `SB`, so the result below holds.

That last part needed `RootSplit`'s root entries to carry their archive-relative
path rather than just a name, since `Root/Binaries` is not where the loader looks.
Existing entries are top-level, where the path and the name are the same thing, so
nothing about the Bethesda cases moved.

The corpus went with it: the anonymiser now keeps the engine's own directory names
(`sb`, `content`, `paks`, `binaries`), which it had been replacing with `dN` -
without them the corpus could not exercise the rule at all. Skyrim's verdicts are
unchanged, and Stellar Blade is at **10 of 13 real archives installing with no
question asked**.

## Open question

`mod_unit` shipped as a choice of two values, `files` or `folder`, the minimum
that makes Stardew correct. A family whose unit sits two levels down would turn
it into a depth number. No family has needed one yet.
