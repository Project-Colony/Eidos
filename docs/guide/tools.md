# Tools: xEdit, BodySlide, DynDOLOD, FNIS

A tool run through Eidos sees **the merged view**, inside the game's own Proton
prefix. It reads what the game will read - every enabled mod, in priority order -
and whatever it writes lands in the Overwrite, where one click turns it into a
real mod.

## Adding one

In the GUI: **Tools -> Executables**, then Add. From the command line:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
```

The script extender, the game binary and the launcher are detected
automatically; only extra tools need registering.

### Point it at the real file, wherever that is

Register the executable where it actually sits. If the tool was installed as a
mod, that is inside the mod folder:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(that is the global instance's path - for a portable instance the same rule
applies under its own folder, `<instance>/mods/...`; note an absolute path like
this is the one thing that does not survive MOVING a portable folder later).

Eidos rewrites that path to the merged one before launching, so the tool runs
from `<game>/Data/CalienteTools/BodySlide/` and sees every other mod's files
there too. This matters more than it sounds: BodySlide ships an **empty**
`SliderSets` directory, and every body it can build comes from CBBE and the
outfit mods. Launched from its own mod folder it finds nothing and looks broken.

MO2 does the same rewriting, for the same reason - its own comment names FNIS.

A tool in a **disabled** mod cannot be rewritten, because its files are not in
the view either. Eidos says so and runs it from its own folder rather than
pretending.

## Sending a tool's output to its own mod

A generator - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - writes hundreds of
files. By default they land in the Overwrite with everything else. Set **Capture
output into** in the Executables editor and this run's output goes into that mod
instead:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

The mod is created if it does not exist. Only the files THIS run produced move;
anything that was already in the Overwrite stays there, so two tools with capture
targets do not steal each other's output. A run that wrote nothing leaves no
empty mod behind.

It is done after the run rather than by pointing the write layer at the mod,
which is how MO2 does it. Pointing the write layer at a mod would promote it to
top priority for the whole run - flipping every conflict it is in and flipping
them back afterwards - and would write straight through the mod's own files with
no copy-up. The capture reaches the same end state without either.

If the target mod is disabled, the output is still written but the game will not
see it, so the tool would regenerate the same files on the next run. Eidos warns
when that is the case.

## The DLLs a tool needs are chosen by its NAME

This is the surprising part, so it is worth stating plainly: **the title you give
a tool decides which runtime prerequisites Eidos provisions for it.** The match
is a case-insensitive substring of the title.

| If the title contains | Eidos requests |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| anything else | nothing |

So a tool registered as **`BodySlide`** gets its DirectX DLLs; the same
executable registered as **`BS`** gets nothing and may fail to start with an
error that says nothing about DLLs. Name tools after the program.

The list is in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), and the
`Prereqs` field in the Executables dialog is editable - the detection is a
default, not a rule.

### Three kinds of prerequisite

**Tier 1 - bundled DLLs** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos
ships them and copies them into the prefix at launch. Nothing to do, no network.

**Tier 2 - winetricks verbs** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). These write registry keys, the GAC and CLR hosts, so they
cannot be file-copied. They **download from Microsoft**.

**Tier 3 - runtimes** (`dotnet10`). A modern .NET runtime is 193 files that live
in their own directory and are found through `DOTNET_ROOT`: never registered,
never installed into the prefix at all, so neither of the other tiers can carry
it. Eidos downloads it itself, checks it against a checksum built into the
binary, and caches it in `~/.local/share/Colony/Eidos/runtimes/` - **outside any
instance**, because 78 MB is not per-game and not per-profile.

Nothing in tiers 2 or 3 runs silently:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

In the GUI the same states sit under the Prereqs field, and the missing ones are
buttons. A verb that is neither bundled, nor a runtime, nor a known winetricks
verb is reported as a probable typo rather than offered as a download.

### Why DynDOLOD needs `dotnet10`

DynDOLOD does not build object LOD itself: it shells out to LODGen, and it ships
three of them. `LODGenx64.exe` targets .NET Framework 4.8, which under Proton is
routed to Wine's Mono - whose `System.Uri` initialiser calls a method Mono does
not implement. It dies before its first line of work, leaving a log holding a
version banner and nothing else, and a DynDOLOD dialog that says only "failed for
one or more worlds".

Installing the real .NET Framework does not fix it: Proton replaces `mscoree.dll`
- the loader that would find it - with a symlink into its own tree, and re-does
that on every prefix update.

The build that works is `LODGenx64Win10.exe`, which targets modern .NET and never
touches `mscoree`. Point `DOTNET_ROOT` at a .NET 10 runtime and it runs. That is
what `dotnet10` provisions, and Eidos sets the variable when launching any tool
that declares it.

Eidos runs the system `winetricks` against Proton's own `wine` and the game
prefix, which sidesteps Steam's pressure-vessel container and the
protontricks + Proton-GE mismatch. A tool that declares an uninstalled Tier-2
verb still launches, with a warning naming the verb and the command to fix it -
the user may have it from elsewhere.

## The game path in the prefix

Windows tools find their game by reading
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, a key the game's own
installer writes - and which Steam under Proton never runs. Without it xEdit,
Wrye Bash and DynDOLOD open on an empty path. Eidos writes it before running a
tool: idempotent, additive, and skipped if the prefix is uninitialised or in use.

## A tool's own settings are still its own

Eidos puts a tool in the right place with the right DLLs. What the tool then
does with its configuration is between you and the tool, and the failure is
usually silent.

The worked example, because it costs an hour otherwise: BodySlide's **Game Data
Path** (Settings) must point at the game's `Data` directory, not the game folder
above it. Set one level too high, a batch build reports "All sets processed
successfully" and writes 1439 meshes where the game will never look for them.
Eidos catches them - they land in `Overwrite/Root/` rather than in your
installation - but nothing is wrong from the game's point of view except that
your bodies are not built.

Tool output belongs in the Overwrite. When a run produces something worth
keeping, **Overwrite -> Create mod...** turns it into an ordinary mod that can be
ordered, disabled and removed like any other.
