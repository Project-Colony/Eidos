> Automated 10-agent field-by-field comparison of the Eidos tools/executables
> system vs the real MO2 source (executableslist, editexecutablesdialog). 2026-06-19.

# Does Eidos have an MO2-style executables/tools system, and is it identical?

## 1. DIRECT ANSWER

**Yes, Eidos has a real MO2-style executables/tools system** - a `Tool` model (`crates/eidos-instance/src/tools.rs:16-31`), a `tools.ini` store, per-game default seeding (script extender), user-wins-collision merge, a two-pane "Executables" editor GUI (`crates/eidos-gui/src/main.rs:4841-4935`), and a CLI (`eidos tool ... add/rm/run`). The core loop - add a tool by title/exe/args/workdir, see it merged with read-only defaults, run it through the merged view - is a faithful port.

**No, it is not totally identical to MO2.** The headline: Eidos covers the *four base fields* (title, binary, arguments, working dir) plus a Linux-only `prereqs` field, but is missing **every MO2 flag and advanced override** - Steam AppID, custom output mod (per-tool Overwrite redirect), forced libraries, the four UI flags (toolbar/icon/tray/hide), the Reset button, and Browse buttons. Of these, only **two gaps actually matter on Linux** (Steam AppID and per-tool output mod); the rest are either Windows-isms Eidos should NOT port or low-value polish.

## 2. FIELD-BY-FIELD MATRIX

Every MO2 Executable field/option vs Eidos. "Different" = present but implemented differently.

| MO2 field / option | MO2 ref | Eidos state | Eidos ref | Notes |
|---|---|---|---|---|
| **title** | `executableslist.cpp:99` | has | `tools.rs:19` | identical (case-insensitive key) |
| **binary** (exe path) | `executableslist.cpp:99` | has | `tools.rs:21` | abs or relative-to-install; resolved at `main.rs:512-516` |
| **arguments** | `executableslist.cpp:99` | **different (better)** | `tools.rs:23,99-100` | MO2 = one space-joined string; Eidos = `Vec<String>`, one `arg0=/arg1=` key each → lossless for args with spaces. **Keep.** |
| **workingDirectory** | `executableslist.cpp:99` | has | `tools.rs:25` | same default semantics (exe's own dir): MO2 `processrunner.cpp:560-563`, Eidos `main.rs:542` |
| **steamAppID** + overwrite checkbox | `executableslist.h:89`, `editexecutablesdialog.ui:342-370` | **absent** | - | `proton_command` takes `app_id: u32` by value (`proton.rs:178`); CLI always passes `game.def.steam_app_id` (`main.rs:527`). No per-tool override path. |
| **custom_overwrites** (output mod) | `editexecutablesdialog.cpp:140-149`, `.ui:376-395` | **absent** | - | All tool output lands in the single Overwrite layer (`main.rs:287`). MO2 stores this per-profile. |
| **forcedLibraries** (DLL preload) | `editexecutablesdialog.ui:400-431`, `forcedloaddialog.*` | **different / absent UI** | `main.rs:319-400` | Eidos HAS a forced-DLL mechanism (`forced_dll_overrides` → `WINEDLLOVERRIDES n,b`, auto-detected by import scan + Tier-1 prereqs), but it is **automatic and not user-configurable per tool**. No checkbox, no Configure Libraries dialog. |
| **Flag: ShowInToolbar** (0x02) | `executableslist.h:48` | **absent** | - | Eidos has no per-tool toolbar; run target is a combobox (`main.rs:4239`). |
| **Flag: UseApplicationIcon** (0x04) | `executableslist.h:49` | **absent** | - | Eidos has no desktop-shortcut creation. Windows-ism. |
| **Flag: Hide** (0x08) | `executableslist.h:50` | **absent** | - | All user tools always shown in run picker. Cross-platform-useful. |
| **Flag: MinimizeToSystemTray** (0x16) | `executableslist.h:51` | **absent** | - | Tray support varies by DE on Linux. |
| **executablesBlacklist** (global USVFS opt-out) | `settings.h:800-805` | **absent** | - | No VFS-hook opt-out; tools always run through the FUSE mount. |
| **GUI: Browse buttons** (binary + workdir) | `editexecutablesdialog.ui:285-317` | **absent** | `main.rs:4894-4895` | Text-only fields; user types paths. |
| **GUI: Add split menu** (from file / empty / clone) | `editexecutablesdialog.ui:92-112` | **partial** | `main.rs:2073-2088` | Eidos Add inserts a blank "New Tool" only. No from-file, no clone. |
| **GUI: Reset button** (restore plugin defaults) | `editexecutablesdialog.cpp:574-593` | **absent** | - | Defaults are recomputed fresh every open (`main.rs:771-775`), so they can't be "lost" - but no explicit reset/rename-on-collision action. |
| **GUI: Apply button** | `editexecutablesdialog.cpp:858-859` | **absent** | `main.rs:4925-4930` | Only Cancel / Save. |
| **GUI: live title-conflict revert** | `editexecutablesdialog.cpp:671-705` | **absent** | - | Eidos validates at Save time only. |
| **JAR special-casing** (auto javaw + `-jar`) | `editexecutablesdialog.cpp:797-816` | **absent** | - | Rare in Bethesda modding. |
| **list reordering** | drag-drop `editexecutablesdialog.ui:204-209` | **different** | `main.rs:4884-4885` | Eidos = Up/Down buttons (keyboard-friendly), no drag-drop. |
| **prereqs** (winetricks verbs / Tier-1 DLLs) | *no MO2 equivalent* | **Eidos-only addition** | `tools.rs:26-30,152-174` | Linux/Proton-specific. Auto-seeded for known tools (BodySlide → `d3dx9_43,d3dcompiler_47`). |
| **per-game defaults via plugin** | `executableslist.cpp:136-149` | **partial** | `tools.rs:131-146` | Eidos seeds only the script-extender loader, from `GameDef`. No general per-game tool list, no "Explore Virtual Folder". |

**Linux-only Eidos additions (the `prereqs` column):** the `prereqs` field is the one place Eidos adds capability MO2 lacks - it declares the Wine/Proton runtime a tool needs. Split into Tier-1 (bundled DirectX DLLs, provisioned at launch via `ensure_native_dll`, `main.rs:380-391`) and Tier-2 (vcrun/dotnet, installed by `eidos prereqs --install`). Auto-mapped per tool name at `tools.rs:152-174`. This is the correct Linux analogue of "what does this Windows tool need to run", and has no MO2 counterpart.

## 3. BEHAVIOUR DIFFERENCES (run path)

| Behaviour | MO2 | Eidos | Diverges? Matters on Linux? |
|---|---|---|---|
| **Working dir resolution** | empty → exe's own dir (`processrunner.cpp:560-563`) | `tool.workdir` or exe's parent (`main.rs:542`) | **Identical.** No divergence. |
| **Steam AppID at launch** | sets `SteamAppId`/`SteamGameId`/`SteamAPPId` from per-exe override or game default (`spawn.cpp:618-622`) | sets all three from the **game's** app id only (`proton.rs:199-201`); no per-tool override | **Diverges, and it matters.** Creation Kit / some tools have their own AppID; Eidos can't point a tool at a different one. Real gap (see plan). |
| **Argument passing** | space-joined string re-split | `Vec<String>` passed straight to `Command` (`main.rs:539`) | Diverges, **Eidos is more correct** (no re-split corruption). Does not matter negatively. |
| **Output → Overwrite** | per-tool redirectable to a named mod (`custom_overwrites`) | always the single Overwrite layer (`main.rs:287`) | **Diverges, and it matters** for BodySlide-type tools. Real gap. |
| **Auto-detection of tools** | game plugin provides a full list (launcher, CK, xEdit stubs) + hardcoded Explore Virtual Folder (`executableslist.cpp:136-166`) | only the script-extender loader if present (`tools.rs:131-146`) | Diverges. Medium value: a richer default list helps onboarding but isn't blocking. |
| **Forced libraries** | user-listed DLLs injected via `usvfsForceLoadLibrary` (`usvfsconnector.cpp:285-295`) | auto-composed `WINEDLLOVERRIDES n,b` from import-scan + ship-scan + tool Tier-1 prereqs (`main.rs:319-400`) | **Diverges in mechanism, mostly equivalent in effect.** MO2's DLL injection is a Windows/USVFS concept; the Linux equivalent is `WINEDLLOVERRIDES`, which Eidos already does - just *automatically*, not via a per-tool UI. The Windows-style manual list should NOT be ported verbatim. |
| **Environment** | prepends MO2 dir to PATH (`spawn.cpp:447`) | inherits env, relies on Proton's own PATH | Diverges, does not matter (Proton manages its PATH). |
| **Blacklist / VFS opt-out** | per-exe skip of USVFS hooks | none | Diverges; low value on FUSE (no hook-injection crash class to dodge). |
| **Post-run refresh** | triggers FS + mod-tree refresh (`processrunner.cpp:805-841`) | captures INIs back to profile (`main.rs:298-303`); GUI mod list not auto-refreshed | Minor divergence; low value. |

**Net:** the run path is faithful on the things that affect every launch (cwd, args, exit-code propagation, INI capture, forced graphics DLLs). It diverges meaningfully on exactly two user-facing capabilities: **per-tool Steam AppID** and **per-tool output mod**.

## 4. THE BODYSLIDE WORKFLOW

**Can a user add + run BodySlide in Eidos today?** Yes, end to end, and the runtime story is actually good. But adding it is clunkier than MO2 and the output story is worse.

**Adding via CLI - smooth, arguably better than MO2:**
```
eidos tool skyrimse add "BodySlide" "Data/CalienteTools/BodySlide/BodySlide x64.exe"
```
auto-seeds `prereqs = d3dx9_43, d3dcompiler_47` (`main.rs:468` → `default_prereqs`, `tools.rs:161-162`), and at run time those native DLLs are provisioned into the prefix and forced (`main.rs:380-391`). MO2 needs the user to know to add `d3dcompiler` via the forced-libraries dialog; Eidos knows it by name. **This is a genuine Eidos win.**

**Adding via GUI - clunky:**
1. Tools → Executables, click **Add** → inserts a blank "New Tool" with **no Browse button** - you must type the full path to `BodySlide x64.exe` by hand (`main.rs:2076-2087`, `4894`).
2. The GUI Add does **not** call `default_prereqs` (unlike the CLI), so the Prereqs field starts empty - you must know to type `d3dx9_43, d3dcompiler_47` yourself, or the 3D preview may fail. **This GUI/CLI inconsistency is the sharpest BodySlide rough edge.**
3. No Steam AppID, no output-mod, no flags fields exist.

**Running:** `eidos tool skyrimse run BodySlide` (GUI shells out to the same, `main.rs:796-803`) resolves Proton, mounts the merged view, provisions the DLLs, and runs. Works.

**What's missing/clunky for BodySlide specifically:**
- **Output goes to Overwrite, not a "BodySlide Output" mod.** In MO2 the canonical workflow is redirecting BodySlide's generated meshes to a dedicated mod. Eidos can't - everything lands in Overwrite, which the user then has to sort by hand. This is the single biggest *functional* BodySlide gap.
- **No Browse button** + **GUI doesn't auto-seed prereqs** = the GUI path is materially worse than the CLI path for the exact tool the maintainer named.

## 5. RANKED PLAN TO MO2 PARITY

Ranked by **value / effort**. Each notes whether it's real parity or a Windows-ism to deliberately skip.

### Tier 1 - Do these (real Linux value)

1. **GUI: auto-seed prereqs on Add + add Browse buttons** - *what:* call `default_prereqs(title)` when a tool is added/title-typed in the GUI (mirror CLI `main.rs:468`), and add file/dir picker buttons next to the exe and workdir fields. *effort:* small-medium. *value:* high - fixes the exact BodySlide GUI clunkiness, closes the CLI/GUI inconsistency. **The highest value-to-effort item.**

2. **Per-tool Steam AppID override** - *what:* add `steam_app_id: Option<String>` to `Tool` (`tools.rs:17`), serialize as `steamappid=`, thread it into `proton_command` (replace `game.def.steam_app_id` when set, `proton.rs:178` / `main.rs:527`), GUI checkbox + field. *effort:* medium. *value:* medium-high - needed for Creation Kit and tools with their own Steam entry; fully meaningful under Proton. **Real parity, not a Windows-ism.**

3. **Per-tool custom output mod** - *what:* `output_mod: Option<String>`; at launch, point `LaunchSpec.overwrite` (`main.rs:287`) at `mods/<output_mod>` instead of the default Overwrite; GUI checkbox + mod combobox. *effort:* large (MO2 stores it per-profile; Eidos `tools.ini` is instance-wide, so either accept tool-level scope or add profile-keyed storage). *value:* high for BodySlide/xEdit workflows. **Real parity.** Recommend tool-level scope first (simpler) unless per-profile is requested.

### Tier 2 - Nice to have (low effort, modest value)

4. **`hidden` flag** - *what:* `hidden: bool`, filter from the run-target picker (`main.rs:4239`) but still show in the editor. *effort:* small. *value:* low-medium - the only one of the four MO2 flags that's cross-platform-useful. **Port this flag; skip the other three.**

5. **Reset / restore-defaults button** - *what:* re-merge `default_tools` and rename colliding user tools (`SKSE` → `SKSE-old`). *effort:* small. *value:* low (defaults already recompute on open, so recovery is implicit). Mostly UX polish.

6. **Richer per-game defaults** - *what:* optional `per_game_tools` in `GameDef` (launcher, CK) beyond the script extender (`tools.rs:131-146`); optionally a synthetic "Browse Merged View" that opens the file manager at the mount point. *effort:* medium. *value:* medium for onboarding.

7. **Add split menu (from-file / empty / clone) + Apply button** - *effort:* small each. *value:* low UX polish.

8. **MO2 `ModOrganizer.ini` executables import** (`eidos import mo2-executables`) - *effort:* small. *value:* low-medium, only for migrating users.

### Tier 3 - Intentionally DIFFER from MO2 (do NOT blindly port)

- **Forced libraries as a per-tool DLL list + Configure Libraries dialog** - this is a Windows/USVFS concept (`usvfsForceLoadLibrary`). Eidos's Linux-correct equivalent already exists: automatic `WINEDLLOVERRIDES n,b` driven by import-scan + ship-scan + Tier-1 prereqs (`main.rs:319-400`). **Do NOT add MO2's manual DLL-injection UI.** If anything, expose the *existing* automatic behavior read-only ("forced DLLs for this launch: d3dcompiler_47, d3d11") rather than asking users to hand-list DLLs.
- **UseApplicationIcon** - desktop-shortcut icon extraction is a Windows-ism; Eidos has no shortcut creation. **Skip.**
- **MinimizeToSystemTray** - tray support is DE-dependent and low value for a CLI-driven launch. **Skip** unless specifically requested.
- **ShowInToolbar** - Eidos uses a run-target combobox, not a toolbar of tool icons. The flag is meaningless without that UI. **Skip.**
- **executablesBlacklist** (USVFS hook opt-out) - no hook-injection failure class exists under FUSE the way it does under USVFS. **Skip** unless a concrete tool needs to bypass the mount.
- **Arguments as a single space-joined line** - Eidos's per-key `arg0=/arg1=` format (`tools.rs:99-100`) is strictly better (lossless for spaces). **Keep Eidos's; do not regress to MO2's format.**

**Bottom line for the maintainer:** Eidos has the system and the base data model is faithful (and the args + prereqs handling is better than MO2). To call it "parity" for real Linux modding you need exactly three things - **Tier 1 items 1-3 (auto-seed-prereqs + Browse, Steam AppID, output mod)**. Everything else MO2 has is either a Windows-ism to skip or low-value polish.

**Key files:** model `crates/eidos-instance/src/tools.rs:16-174`; CLI `crates/eidos/src/main.rs:410-577` (add `:461-469`, run `:493-570`, forced DLLs `:319-400`); GUI dialog `crates/eidos-gui/src/main.rs:333-396` (state), `:767-792` (open), `:2073-2088` (Add), `:4841-4948` (render); Proton launch `crates/eidos-games/src/proton.rs:176-210`.
