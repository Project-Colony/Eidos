# Community Shaders, DLSS and frame generation

Community Shaders 1.4+ ships its own upscaling (DLSS 4 / FSR 3.1 / XeSS, via the
separate "Upscaling - Community Shaders" package) and FSR 3.1 frame generation.
All of it works through Eidos on Linux - CS and its packages install as ordinary
mods and the union serves their DLLs like any other file - but three settings
cannot be seen from inside the game, and each one makes the feature do nothing
without an error: the NVAPI variable, exclusive fullscreen and the Reflex FPS
cap.

The Diagnostics tab checks the first two, and the ENB line further down, for a
game that has a Wine prefix (Steam's Proton prefix, or the one Heroic set up).
The Reflex cap is not checked. When the instance runs the game from a copy such
as `Stock Game`, the checks read the copy's files, not the store install.
`eidos check <instance>` prints the same cards in a terminal, without the Fix
buttons.

- The NVAPI card appears for a Steam copy of Skyrim SE or Enderal SE when an
  NVIDIA driver is loaded (`/proc/driver/nvidia/version` exists), the merged
  `Data` has something in `Shaders/Upscaling`, the game's Proton build leaves
  NVAPI off by default, and none of Steam's launch option, Eidos's own
  environment or the profile's `launch-env.ini` sets `PROTON_ENABLE_NVAPI=1`.
- The exclusive fullscreen card appears only with the Upscaling package
  installed, when the profile's `SkyrimPrefs.ini` has `bFull Screen=1` and
  `SKSE/Plugins/SSEDisplayTweaks.ini` does not set both `Fullscreen=false` and
  `Borderless=true`. Its Fix button writes `bFull Screen=0` and `bBorderless=1`
  into the profile's `SkyrimPrefs.ini`. The check reads `SkyrimPrefs.ini` for
  Enderal SE too, and an Enderal SE profile holds `EnderalPrefs.ini`, so on
  Enderal SE use the Display card below instead.
- The ENB card appears for any game whose winning `enblocal.ini` in the root
  view has no `LinuxVersion=true` in `[GLOBAL]`. It has a Fix button too.

## The launch option DLSS needs

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Use the absolute path of your own `eidos-gui` or `eidos-linux`, as in
[install.md](install.md). The card gives the line to set: the game's current
launch option with `PROTON_ENABLE_NVAPI=1` in front, or, when there is none,
the line built from the path of the running Eidos.

Without the NVIDIA NVAPI layer (dxvk-nvapi), CS cannot initialise DLSS and
falls back to FSR upscaling, with nothing on screen saying why. Whether a Proton
build turns the layer on by itself is written in its `proton` script, and Eidos
reads the `use_nvapi` line there. An opt-out switch (`disablenvapi`) means on by
default: GE-Proton11-9 and Proton 11.0 ship that form (checked 2026-11-09). An
opt-in switch (`enablenvapi`) means off, and only that build needs the
variable. Setting it costs nothing on non-NVIDIA machines. Frame generation
itself is FSR 3.1 and does not need NVAPI; only the DLSS upscaler does.

To set it for one profile only, put `env=PROTON_ENABLE_NVAPI=1` in that
profile's `launch-env.ini` (View -> INI editor, last tab).

## Frame generation requires borderless windowed

CS's frame generation runs on a D3D12 presentation proxy and refuses exclusive
fullscreen outright. With `bFull Screen=1` in `SkyrimPrefs.ini` it never
engages: no error, no message, and the game stays at its base frame rate. SSE
Display Tweaks sets the mode at the engine level, whatever the INIs say:

```ini
[Render]
Fullscreen=false
Borderless=true
```

The window looks identical (borderless at native resolution). Only the mode
the engine reports changes, and that mode is what CS checks.

The Display card in Diagnostics sets the size and the mode in one step, for
Skyrim SE, Enderal SE and Fallout 4. It shows the size of the largest connected
screen. With Borderless window selected, Apply writes `iSize W`, `iSize H`,
`bFull Screen=0` and `bBorderless=1` into the profile's `SkyrimPrefs.ini`
(`EnderalPrefs.ini` for Enderal SE, `Fallout4Prefs.ini` for Fallout 4). When
SSE Display Tweaks sets the window, Apply also writes `Resolution`,
`Fullscreen=false` and `Borderless=true` into a copy of its INI in the
Overwrite (`SKSE/Plugins/SSEDisplayTweaks.ini`), and the mod's own file stays
as it is. With **Start the game at once when a store opens Eidos** on
(Preferences, General, Running a game), the window still opens when the
profile has no window size yet and SSE Display Tweaks does not set the window,
so the size can be set before the first start.

Two more activation conditions, with the same silent failure:

- The display refreshes at 120 Hz or higher, or `frameGenerationForceEnable` is
  set in CS's upscaling settings. Frame generation doubles the presented rate,
  so CS refuses to arm it on displays that cannot show the result.
- The Upscaling package is installed (its `Data/Shaders/Upscaling/` tree holds
  the Streamline and FidelityFX DLLs). CS without it shows the menu entries and
  can enable nothing.

## The Reflex frame-rate limit can strangle the output

CS's Reflex settings carry their own FPS cap (`reflexFPSLimit`, with
`reflexUseFPSLimit`). A cap left at an earlier value - ours was 79 from an
old tuning pass - sits downstream of frame generation and clips exactly the
frames it produces: base 60 doubled to 120, capped back to 79, reads as
"frame gen does nothing". On a 144 Hz display the conventional Reflex cap is
~138. Check it whenever generated frames seem to be missing.

## Known interaction: black screen with SSE Display Tweaks

The FG + Display Tweaks + DXVK combination has a known black-screen failure.
Fix, in order:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. If that is not enough, a `dxvk.conf` next to the game executable (a mod's
   `Root/` directory places one there) with
   `dxvk.enableGraphicsPipelineLibrary = False`

## Mixed texture mods: PGPatcher

Community Shaders can render parallax, complex material and PBR, but only where
a mesh is set up for it. Historically that meant installing pre-patched meshes
and then a mod to switch the effect back off wherever the textures were missing:
coverage limited to the meshes you happened to have, and CPU spent at runtime
undoing a choice made at install time.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) inverts
it. You install whatever textures you like, of any shader type, and it rewrites
the meshes and plugins so each surface uses the right one - including the
alternate-texture records in plugins, which the old method left uncovered.
Without it, a texture that needs a shader its mesh does not request loads but
does not render as intended, which happens most on a setup that mixes texture
types.

Two things to know before running it on Linux:

- it needs the `--ignore-mo2vfscheck` argument or it exits at once,
  complaining about MO2. Eidos supplies it - see
  [Tools](tools.md#why-pgpatcher-needs---ignore-mo2vfscheck) for what the flag
  is and why the check fails here;
- it edits meshes, so it runs after every texture mod is installed and
  before DynDOLOD, which has to see the final meshes. Changing your textures
  later means running both again, in that order.

Its output does not go to the Overwrite. PGPatcher writes only to the Output
location set in its launcher, and it refuses a folder inside `Data`. Point that
location at a folder of its own, such as a new `PGPatcher Output` folder under
the instance's `mods/`. Keep that mod disabled while PGPatcher runs, because
PGPatcher empties the folder before writing it again and Eidos expects the
enabled mods to stay unchanged while the view is mounted. Enable it afterwards
with a high priority: it is meant to win.

Do not give PGPatcher's entry a **Capture output into** mod and then use that
mod as the Output location. Eidos moves PGPatcher's own `cfg/` and `log/` into
that mod after each run, and PGPatcher then stops with "Output directory has
non-PGPatcher related files".

## ENB under Proton: LinuxVersion

Jackify writes `LinuxVersion=true` in the `[GLOBAL]` section of `enblocal.ini`
for every ENB it installs under Proton. ENB's own documentation does not
describe the key. A Health card in the Diagnostics tab flags an `enblocal.ini`
without the line, and its Fix button writes a copy of the winning
`enblocal.ini` into `overwrite/Root/` with the line set, so the ENB mod keeps
its file.

Another card says when an ENB sits in the game folder (an `enblocal.ini`, an
`enbseries.ini` or an `enbseries/` folder) while an enabled mod ships
Community Shaders (`SKSE/Plugins/CommunityShaders.dll`). The two can run
together. If the picture looks wrong, turn one of them off in its INI.

## Reading the numbers afterwards

Generated frames are presentation-side only: the engine still simulates at the
base rate, Havok still ticks at the base rate, and anything that counts *engine*
frames (CS's own counters included) keeps reporting ~60 while the display shows
~120. The counter is right, and this is why frame generation is physics-safe
where raising the engine's own framerate is not.
`DXVK_HUD=fps` in the launch options shows a counter if you want one on screen.

One rule: driver-level interpolation (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) and CS's frame generation are competing
technologies. Run one or the other, never both.
