# Vendored `d3dcompiler_47.dll`

Eidos bundles Microsoft's native Direct3D HLSL compiler so it can provision it
into a Proton prefix at launch (no mainstream Proton flavour ships it - they all
link `d3dcompiler_47.dll` to Wine's builtin reimplementation, which graphics mods
like Community Shaders / ENB / ReShade reject). Bundling, rather than downloading
at runtime, keeps Eidos self-contained and avoids fetching an executable over the
network. This mirrors Mod Organizer 2, which ships the same DLL in its `dlls/`
folder (`src/dlls.manifest.qt6`).

| File | Arch | Deployed to | Size (bytes) | SHA-256 |
|------|------|-------------|--------------|---------|
| `x86_64.dll` | x86-64 (PE32+) | prefix `drive_c/windows/system32` | 4691496 | `9489124759292316d11eae5ffb67b74bfaf0e1853b968137b047567f31c76232` |
| `i386.dll` | i386 (PE32) | prefix `drive_c/windows/syswow64` | 3657992 | `2ad0d4987fc4624566b190e747c9d95038443956ed816abfd1e2d389b5ec0851` |

Both are the **redistributable** build (PE version `10.0.26100.1`, file
description "Direct3D HLSL Compiler for Redistribution", (c) Microsoft
Corporation), as shipped in the Windows 10/11 SDK and the DirectX End-User
Runtime redistributable. The `for Redistribution` build is the one Microsoft
marks as distributable alongside an application (the copy preinstalled inside
Windows itself is *not*).

`x86_64.dll` is byte-identical (same SHA-256) to the copy MO2 ships and to the
copy Proton/winetricks seed into a provisioned prefix's `system32`.

## License

Governed by the Microsoft Windows SDK / DirectX redistributable license. Confirm
the redistribution terms for this specific SDK build (10.0.26100) and the
interaction with Eidos's GPL-3.0 license before any public release/redistribution
of a build that embeds these binaries.

## Not bundled (intentionally)

- `d3dcompiler_43.dll` - older D3DX; not needed by current SKSE-era graphics mods.
- `vcruntime140.dll` / `msvcp140.dll` - Proton already provides these.
