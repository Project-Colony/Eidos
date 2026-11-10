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

## Microsoft runtimes in the release bundle

| Verb | Version | File | SHA-256 | Source | License |
|------|---------|------|---------|--------|---------|
| `vcrun2022` | 2015-2022, x86 | `VC_redist.x86.exe` | `0c09f2611660441084ce0df425c51c11e147e6447963c3690f97e0b25c55ed64` | `https://download.visualstudio.microsoft.com/download/pr/990c720a-46ce-4dc7-87b9-c2eb615d34b9/0C09F2611660441084CE0DF425C51C11E147E6447963C3690F97E0B25C55ED64/VC_redist.x86.exe` | Microsoft Visual C++ 2015-2022 Redistributable terms |
| `vcrun2022` | 2015-2022, x64 | `VC_redist.x64.exe` | `cc0ff0eb1dc3f5188ae6300faef32bf5beeba4bdd6e8e445a9184072096b713b` | `https://download.visualstudio.microsoft.com/download/pr/990c720a-46ce-4dc7-87b9-c2eb615d34b9/CC0FF0EB1DC3F5188AE6300FAEF32BF5BEEBA4BDD6E8E445A9184072096B713B/VC_redist.x64.exe` | Microsoft Visual C++ 2015-2022 Redistributable terms |
| `dotnet8`, `dotnetdesktop8` | 8.0.12, x64 | `windowsdesktop-runtime-8.0.12-win-x64.exe` | `cb51b559f343cb56e23cad2e5af8c4d1701e221a0a2a4116193a2a9375568814` | `https://download.visualstudio.microsoft.com/download/pr/f1e7ffc8-c278-4339-b460-517420724524/f36bb75b2e86a52338c4d3a90f8dac9b/windowsdesktop-runtime-8.0.12-win-x64.exe` | .NET 8.0.12 Desktop Runtime, MIT |
| `dotnet10` | 10.0.10, x64 | `dotnet-runtime-10.0.10-win-x64.zip` | `79bb04da40ab098f0c2e4ef4652fe5bb98c27c7d5697389003b7be53d2c86f6a` | `https://builds.dotnet.microsoft.com/dotnet/Runtime/10.0.10/dotnet-runtime-10.0.10-win-x64.zip` | .NET 10.0.10 runtime, MIT |

These files are not in git: the release job (`.github/workflows/release.yml`)
downloads each file `packaging/runtimes.txt` lists, checks its SHA-256 and ships
it under `runtimes/` in `eidos-linux-bundle.tar.gz`, with its license under
`licenses/microsoft-runtimes/`. A source build and the single `eidos-linux` file
ship none of them, so Eidos downloads them from the same URLs, after the same
consent.

## Not bundled (intentionally)

- `vcruntime140.dll` / `msvcp140.dll` (the VC++ 2015-2022 runtime) - still not embedded
  in the binaries. SkyrimSE.exe and many SKSE plugins dynamically import these, but
  unlike the d3dcompiler stub, Wine's builtin CRT (shipped by every Proton) is
  functionally adequate and satisfies the imports, so the game and plugins load without
  the genuine MS redist. For a C++ tool that the builtin can't satisfy, the release
  bundle ships Microsoft's installers for `vcrun2022` (table above), which Eidos runs in
  the prefix on explicit consent.
- The **.NET** runtimes/SDK (`dotnet8`, `dotnetdesktop8`, ...) - too large to embed in
  the binaries and must run an in-prefix installer (CLR host + registry + GAC). The
  release bundle ships the .NET 8 Desktop Runtime installer, which Eidos runs in the
  prefix for `dotnet8` and `dotnetdesktop8`, on explicit consent. FNIS needs
  `dotnet48`, which still goes through the Tier-2 winetricks path.

## What is NOT bundled, and why

The .NET runtime DynDOLOD's LOD generator needs (`dotnet10`) is not embedded in
the binaries. The release bundle ships Microsoft's zip whole under `runtimes/`
(table above) and Eidos unpacks that copy; without it, Eidos downloads the same
zip. .NET is MIT-licensed, so embedding it would be legal - the reasons it stays
out of the binaries are size, and the failure mode below is why it ships whole.

It is 193 files, 78 MB unpacked, against 18 MB for everything in this directory.
Measured on a real generation run, LODGen touched 25 of those files, 25.6 MB, so
a trimmed set is tempting. But those 25 are what ONE worldspace happened to need;
another code path pulls in `System.Private.Xml` or `System.Text.Json`, and
neither is among them. A trimmed runtime missing one fails with a
`FileNotFoundException` and no indication of what is absent - which is the exact
class of silent failure the prerequisite system exists to end.

Trimming a runtime is something an application's own author does, knowing their
own code. See `crates/eidos-gamefeatures/src/runtime.rs`.

The .NET 10 SDK that Synthesis needs to compile its patchers (`dotnetsdk10`) is
downloaded, and the release bundle does not ship it. Eidos pins version 10.0.403, the win-x64 zip at
`https://builds.dotnet.microsoft.com/dotnet/Sdk/10.0.403/dotnet-sdk-10.0.403-win-x64.zip`:
300745255 bytes, 5577 files, 808 MB unpacked, and it holds the
`Microsoft.NETCore.App`, `Microsoft.AspNetCore.App` and
`Microsoft.WindowsDesktop.App` 10.0.14 runtimes. Microsoft's
`release-metadata/10.0/releases.json` gives its SHA-512 as
`7a114c318fa7f1d9822991b534b2814d0e78386ceda816b371f5141acefe741d87ccb606ede06cadac8f9ac5070e32b708a67bbd1619ff5e5a7858a27874ef48`.
The downloaded file matched it, and its SHA-256,
`f398979a9cac2dba760771c4e01f16e431dcd6bcfb2213e3e8f5b3d354095cc0`, is the hash
built into Eidos.

The SDK alone does not get a patcher through NuGet's package restore under
Wine. The prefix's root store lacks the code-signing and timestamping roots in
the SDK's `sdk/10.0.403/trustedroots/codesignctl.pem` and `timestampctl.pem`
(385 distinct certificates), so restoring `Mutagen.Bethesda.Synthesis` 0.36.6
fails with `NU3018` and `NU3028`. Eidos reads both files from the downloaded
SDK and writes each certificate to the prefix's
`HKCU\Software\Microsoft\SystemCertificates\Root\Certificates` key with
`regedit`, in the layout Wine's crypt32 uses itself: checked byte for byte
against the 325 entries .NET's `X509Store` wrote into a scratch prefix. Nothing
from these files ships with Eidos.

