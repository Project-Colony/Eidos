# Community Shaders, DLSS and frame generation

Community Shaders 1.4+ ships its own upscaling (DLSS 4 / FSR 3.1 / XeSS, via the
separate "Upscaling - Community Shaders" package) and FSR 3.1 frame generation.
All of it works through Eidos on Linux - CS and its packages install as ordinary
mods and the union serves their DLLs like anything else - but three things are
NOT discoverable from inside the game, and each one makes the feature silently
do nothing. This page is the list of them, learned the hard way on a real
setup.

## The launch option DLSS needs

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton disables its NVIDIA NVAPI layer (dxvk-nvapi) unless the game is on
Valve's allowlist, and Skyrim is not. Without it CS cannot initialise DLSS and
falls back to FSR upscaling - quietly, with nothing on screen saying why.
Setting the variable costs nothing on non-NVIDIA machines, so the guide-safe
launch option is simply the line above. Frame generation itself is FSR 3.1 and
does not need NVAPI; only the DLSS upscaler does.

## Frame generation requires borderless windowed

CS's frame generation runs on a D3D12 presentation proxy and refuses exclusive
fullscreen outright. `bFull Screen=1` in `SkyrimPrefs.ini` means it never
engages - no error, no message, just base framerate. The robust fix is SSE
Display Tweaks, which enforces the mode at the engine level whatever the INIs
say:

```ini
[Render]
Fullscreen=false
Borderless=true
```

The window looks identical (borderless at native resolution); only what the
engine believes changes - and what the engine believes is what CS checks.

Two more activation conditions, same silent-failure behaviour:

- **Display refresh 120 Hz or higher**, or set `frameGenerationForceEnable` in
  CS's upscaling settings. Frame generation doubles the presented rate, so CS
  refuses to arm it on displays that cannot show the result.
- **The Upscaling package installed** (its `Data/Shaders/Upscaling/` tree holds
  the Streamline and FidelityFX DLLs). CS without it shows the menu entries and
  can enable nothing.

## The Reflex frame-rate limit can strangle the output

CS's Reflex settings carry their own FPS cap (`reflexFPSLimit`, with
`reflexUseFPSLimit`). A cap left at some earlier value - ours was 79 from an
old tuning pass - sits downstream of frame generation and clips exactly the
frames it produces: base 60 doubled to 120, capped back to 79, reads as
"frame gen does nothing". On a 144 Hz display the conventional Reflex cap is
~138. Check it whenever generated output seems missing; it is the second
silent killer after exclusive fullscreen.

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
undoing a decision that should never have been made.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) inverts
it. You install whatever textures you like, of any shader type, and it rewrites
the meshes and plugins so each surface uses the right one - including the
alternate-texture records in plugins, which the old method left uncovered. It is
the difference between a texture pack that renders and one that merely loads,
and it matters most on a mixed setup, which is what any real load order is.

Two things to know before running it on Linux:

- it needs the **`--ignore-mo2vfscheck`** argument or it exits at once,
  complaining about MO2. Eidos supplies it - see
  [Tools](tools.md#why-pgpatcher-needs---ignore-mo2vfscheck) for what the flag
  is and why the check fails here;
- it edits meshes, so it runs **after** every texture mod is installed and
  **before** DynDOLOD, which has to see the final meshes. Changing your textures
  later means running both again, in that order.

Its output goes to the Overwrite like any other tool's, where one click turns it
into a mod. Give that mod a high priority: it is meant to win.

## Reading the numbers afterwards

Generated frames are presentation-side only: the engine still simulates at the
base rate, Havok still ticks at the base rate, and anything that counts *engine*
frames (CS's own counters included) keeps reporting ~60 while the display shows
~120. That is correct behaviour, not a broken counter - and it is why frame
generation is physics-safe where raising the engine's own framerate is not.
`DXVK_HUD=fps` in the launch options shows a counter if you want one on screen.

One rule: driver-level interpolation (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) and CS's frame generation are competing
technologies. Run one or the other, never both.
