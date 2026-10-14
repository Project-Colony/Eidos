# Vendored native Microsoft DLLs

Eidos bundles a few genuine Microsoft redistributable DLLs to drop into a Proton
prefix at launch, because Wine's builtin is either a broken stub (the d3dcompiler
HLSL compiler) or one some tools reject (the D3DX helpers). Bundling, rather than
downloading at runtime, keeps Eidos self-contained. This mirrors Mod Organizer 2,
which ships d3dcompiler_47.dll in its `dlls/` folder (`src/dlls.manifest.qt6`).

Each is deployed arch-aware: the `x86_64.dll` build into the prefix
`drive_c/windows/system32`, the `i386.dll` build into `syswow64` (a WoW64 prefix);
on a pure win32 prefix the `i386.dll` goes into `system32`.

| Verb | Arch | File | Size (bytes) | SHA-256 |
|------|------|------|--------------|---------|
| `d3dcompiler_47` | x86-64 | `d3dcompiler_47/x86_64.dll` | 4691496 | `9489124759292316d11eae5ffb67b74bfaf0e1853b968137b047567f31c76232` |
| `d3dcompiler_47` | i386   | `d3dcompiler_47/i386.dll`   | 3657992 | `2ad0d4987fc4624566b190e747c9d95038443956ed816abfd1e2d389b5ec0851` |
| `d3dcompiler_43` | x86-64 | `d3dcompiler_43/x86_64.dll` | 2526056 | `44c3a7e330b54a35a9efa015831392593aa02e7da1460be429d17c3644850e8a` |
| `d3dcompiler_43` | i386   | `d3dcompiler_43/i386.dll`   | 2106216 | `2f23182ec6f4889397ac4bf03d62536136c5bdba825c7d2c4ef08c827f3a8a1c` |
| `d3dx9_43`       | x86-64 | `d3dx9_43/x86_64.dll`       | 2401112 | `84b900dbd7fa978d6e0caee26fc54f2f61d92c9c75d10b35f00e3e82cd1d67b4` |
| `d3dx9_43`       | i386   | `d3dx9_43/i386.dll`         | 1998168 | `0b28546be22c71834501f7d7185ede5d79742457331c7ee09efc14490dd64f5f` |
| `d3dx11_43`      | x86-64 | `d3dx11_43/x86_64.dll`      | 276832  | `981e42629df751217406e7150477cddc853b79abd6a8568a1566298ed8f7bd59` |
| `d3dx11_43`      | i386   | `d3dx11_43/i386.dll`        | 248672  | `492e960cb3ccfc8c25fc83f7c464ba77c86a20411347a1a9b3e5d3e8c9180a8d` |

## What each is for

- **`d3dcompiler_47`** - the modern Direct3D HLSL compiler. Community Shaders / ENB /
  ReShade import it to compile shaders at runtime; Wine's builtin is a non-functional
  stub. Provisioned automatically when a mod's DLL imports it.
- **`d3dcompiler_43` / `d3dx9_43` / `d3dx11_43`** - the legacy DirectX helper libraries
  the modding TOOLS need: BodySlide / Outfit Studio's 3D preview, DynDOLOD / TexGen,
  and CAO's texture compression. Provisioned when a tool declares them as a prerequisite.

All are the genuine Microsoft DirectX redistributable builds (the `d3dcompiler_47`
pair is the "Direct3D HLSL Compiler for Redistribution", PE 10.0.26100.1; the others
are the DirectX End-User Runtime helper DLLs), not the Wine builtins.

## License

Governed by the Microsoft Windows SDK / DirectX redistributable license. Confirm the
redistribution terms for these specific builds and their interaction with Eidos's
GPL-3.0-or-later license before any public release that embeds these binaries.

## Not bundled (intentionally)

- `vcruntime140.dll` / `msvcp140.dll` (the VC++ 2015-2022 runtime) - SkyrimSE.exe and
  many SKSE plugins dynamically import these, but unlike the d3dcompiler stub, Wine's
  builtin CRT (shipped by every Proton) is functionally adequate and satisfies the
  imports, so the game and plugins load without the genuine MS redist. A specific C++
  tool that the builtin can't satisfy gets `vcrun2022` via the Tier-2 winetricks path.
- The **.NET** runtimes/SDK (`dotnet8`, `dotnetdesktop8`, ...) - too large to bundle and
  must run an in-prefix installer (CLR host + registry + GAC). Tools that need .NET
  (Synthesis, Pandora, FNIS) get it via the Tier-2 winetricks path, on explicit consent.

## What is NOT bundled, and why

The .NET runtime DynDOLOD's LOD generator needs (`dotnet10`) is **downloaded**,
not shipped here. .NET is MIT-licensed, so bundling it would be legal - the
reasons are size and failure mode.

It is 193 files, 78 MB unpacked, against 18 MB for everything in this directory.
Measured on a real generation run, LODGen touched 25 of those files, 25.6 MB, so
a trimmed set is tempting. But those 25 are what ONE worldspace happened to need;
another code path pulls in `System.Private.Xml` or `System.Text.Json`, and
neither is among them. A trimmed runtime missing one fails with a
`FileNotFoundException` and no indication of what is absent - which is the exact
class of silent failure the prerequisite system exists to end.

Trimming a runtime is something an application's own author does, knowing their
own code. See `crates/eidos-gamefeatures/src/runtime.rs`.

