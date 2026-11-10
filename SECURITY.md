# Security Policy

Eidos mounts a filesystem, and its launch binary can optionally carry a file
capability. That is unusual for a mod manager, so this document is specific
about what it does, what could go wrong, and what we want to hear about.

## Two things to know before you install

**1. Eidos runs a FUSE daemon as your user.** Every read and write a launched
game makes against its `Data` directory, and its install root when a mod ships
`Root/` files or the instance runs the game from a copy of its folder, goes
through Eidos's code (`crates/eidos-fuse`) while the game runs. Tools started
with `eidos tool` and commands run with `eidos shell` go through the same view.
The mount lives in a private mount namespace and is invisible to the rest of
the system, and the daemon exits when the game, tool or command does. Nothing
is mounted when Eidos is not running a game, a tool or an `eidos shell`
command.

**2. Eidos runs rootless by default; `cap_sys_admin+ep` is an opt-in.** Without
any capability Eidos mounts inside a user+mount namespace, and that is the
supported mode: mods deploy identically either way. The installer grants the
capability only when asked (`./install.sh --cap`, or `just setcap` for a source
build). With it, Eidos takes a plain mount namespace instead, and that is the
only way to make FUSE kernel passthrough available. Passthrough itself stays
off unless `EIDOS_FUSE_PASSTHROUGH=1` is also set, because with it on, the
daemon before 1.22 stopped Skyrim SE opening its own archives and plugins. Since
1.22 the daemon registers one backing file per inode, which removes the most
likely cause, but that has not been re-measured in a game. If you do grant the
capability, know what it means:

- `CAP_SYS_ADMIN` is the broadest capability in Linux. Anyone who can execute
  that binary gets it, because `+ep` raises the capability on exec for every
  caller, not just for you.
- A program that can mount inside a namespace and then exec a command of the
  caller's choosing is the textbook local privilege escalation primitive. That is
  exactly why unprivileged `unshare --mount` is not allowed in the first place.
  Eidos is not written as a hardened setuid-style tool, and it does not attempt
  to be one.
- On a single-user desktop this grants you nothing you did not already have, and
  that is the case Eidos is built for.
- **On a shared or multi-user machine, do not install the capability-carrying
  binary somewhere every user can execute.** Keep it in your own prefix
  (`~/.local/bin`, the installer's default) and restrict it, for example
  `chmod 0750 ~/.local/bin/eidos` with a home directory other users cannot
  traverse.
- Declining it is the default. Eidos then uses the rootless user+mount
  namespace, and everything works except FUSE passthrough, which is off by
  default anyway. The launch and the GUI's Diagnostics tab only speak up when
  `EIDOS_FUSE_PASSTHROUGH` is set and the capability is missing.

The capability lives on the file, so replacing the file drops it. A rebuild or a
reinstall silently returns you to the rootless path.
`getcap "$(command -v eidos)"` is the check.

**Never set the capability on `eidos-linux`**, the single-file release asset
Colony installs. That file is the window as well as the CLI, so a capability on
it would cover a GUI toolkit, image decoders and archive previews that have no
use for one. The window clears every capability it holds before it opens, but
the opt-in belongs on the separate `eidos` binary of the release bundle or a
source build, and nowhere else.

## Supported versions

Eidos follows semantic versioning and is past 1.0. Only the most recent release
and the current `main` get security fixes. There is no backport branch, and
older releases are not patched: the fix ships in the next release.

## Reporting a vulnerability

**Do not open a public issue.** Use GitHub's private reporting:

<https://github.com/Project-Colony/Eidos/security/advisories/new>

Include whatever you have:

- What an attacker controls and what they get.
- Version (release tag or commit), distro, kernel version.
- Whether the `eidos` binary carried `cap_sys_admin=ep` at the time
  (`getcap "$(command -v eidos)"`), since it changes the impact of most findings.
- Reproduction steps, or a proof of concept. A mod archive or a config file that
  triggers it is ideal.

What to expect: this is a small volunteer project, so no response-time guarantee
is made, but the aim is an acknowledgement within 7 days and an assessment within
30. Fixes land on `main` and then in a tagged release. You get credit in the
advisory unless you ask otherwise. Please give us a reasonable window before
publishing, 90 days as a default. There is no bug bounty.

## In scope

The interesting attack surface, roughly in order of severity:

- **Local privilege escalation through the capability-carrying binary.** Any
  input, argument, environment variable, instance file or config that steers the
  privileged launch path into mounting or binding somewhere it should not, or
  into executing something the caller did not choose. This is the highest
  severity class in Eidos.
- **The FUSE daemon (`crates/eidos-fuse`).** Path resolution that escapes the
  configured layers, copy-up or whiteout handling that writes outside the
  Overwrite layer, case-folding or inode confusion that exposes or corrupts a
  file the caller should not reach, and memory safety in the `unsafe` blocks that
  talk to libc.
- **Namespace and mount setup (`crates/eidos-launch`).** Anything that leaks a
  mount into the host namespace, leaves a mount behind after exit, or lets a
  process inside the namespace affect the host mount table.
- **Untrusted mod content.** Mod archives are attacker-controlled data from the
  internet. Extraction that writes outside the destination directory ("zip
  slip"), absolute or `..` paths in an archive or in a FOMOD, symlinks that
  escape the mod folder, and parser crashes or memory-safety bugs on malformed
  input. Eidos parses archives, FOMOD XML, INI files, plugins
  (`.esp`/`.esm`/`.esl`), BSA/BA2 archives, NIF models (in the C++
  `eidos-nif-preview` helper), DDS previews, the PE import tables of mod DLLs,
  save-game headers and crash logs. A mod's Nexus description and changelog are
  author-written BBCode and HTML, which Eidos turns into plain text without
  rendering markup or fetching images. A Nexus collection's `collection.json`
  is input too: it drives FOMOD answers, `userlist.yaml` rules and INI tweaks.
  So is an `.eidos` pack made by someone else, whose `tools.ini` carries the
  `exe` entries of its tools and whose profiles can carry `launch-env.ini`
  `env=` and `wrapper=` lines that apply at the next launch, and the
  `ModOrganizer.ini` of a Wabbajack list, whose executables
  `eidos tool <instance> import-mo2 --write` adds as tools.
- **The `nxm://` handler.** Eidos can register itself as the system handler for
  `nxm://` links, which means a web page can hand it a URL. Anything in that path
  that turns a crafted URL into a file written outside the download directory, a
  request to an attacker-chosen host, or command execution. With "Install
  downloads when they finish" turned on in Preferences (off by default), a
  finished Nexus download installs without a click while the window is open,
  and a FOMOD opens its installer.
- **Credential handling.** The Nexus OAuth session (access and refresh tokens
  in `~/.config/Colony/Eidos/nexus.ini`, written with mode 0600), the sign-in
  flow's loopback listener, and anywhere a token could be logged or sent to a
  host other than Nexus. Eidos does not accept personal API keys.
- **Runtime provisioning.** Eidos pins five Microsoft files by URL and SHA-256,
  both compiled into the binary rather than fetched alongside the file. Two
  are .NET zips from `builds.dotnet.microsoft.com`: the .NET 10.0.10 runtime
  (the `dotnet10` prerequisite) and the .NET SDK 10.0.403 that Synthesis needs
  (`dotnetsdk10`), unpacked into `~/.local/share/Colony/Eidos/runtimes/`. Three
  are installers from `download.visualstudio.microsoft.com`: `VC_redist.x86.exe`
  and `VC_redist.x64.exe` for `vcrun2022`, and
  `windowsdesktop-runtime-8.0.12-win-x64.exe` for `dotnet8` and
  `dotnetdesktop8`, which Eidos runs in the game's prefix with
  `/install /quiet /norestart`. Every file but the SDK ships in the release
  bundle's `runtimes/`, which `install.sh` copies to that same folder. Eidos
  uses a copy found there, or in a `runtimes/` folder beside its own binary,
  only when its SHA-256 matches the pin; otherwise it downloads the file, and
  a mismatch refuses the install rather than warning about it. A zip is
  unpacked into a staging directory and renamed into place only once complete.
  Anything that defeats that is in scope: a path escaping the runtime directory
  during extraction, a checksum that can be bypassed, a download that can be
  pointed at another host, or a file in `runtimes/` that is used without
  matching its pin. The other prerequisites (`dotnet48`, `xact`, `xact_x64`,
  `dotnet6`, `dotnet7`, `dotnetdesktop6`) go to the system `winetricks`, which
  downloads and checks its own files.
  The SDK also changes two trust settings in the game's prefix. Its runs get
  `NUGET_CERT_REVOCATION_MODE=offline`, and Eidos writes the SDK's
  `codesignctl.pem` and `timestampctl.pem` roots (385 certificates) into the
  prefix's `HKCU\Software\Microsoft\SystemCertificates\Root` store with
  `regedit`, so NuGet can check package signatures under Wine. A way to get
  other certificates into that store is in scope.
- **Other network requests.** Once a day, Eidos sends one GET to
  `api.github.com/repos/Project-Colony/Eidos/releases/latest` to see whether a
  newer release is out (new in 1.24). It is on by default, off with
  `release_check=false` in `settings.ini` (the Preferences switch writes it),
  and skipped in offline mode or for a copy under `/usr` outside `/usr/local`,
  which a package manager updates. The release tag is a trust boundary:
  `tag_version` admits only 1 to 32 digits and inner dots before the value
  reaches `settings.ini` and the menu bar, and a tag that gets anything else
  through is in scope. A LOOT sort downloads the masterlist and
  prelude from `raw.githubusercontent.com/loot`, and libloot parses them.

## Out of scope

- **Mods themselves.** Eidos mounts what you tell it to mount and launches what
  you tell it to launch. It does not sandbox mod content, and a mod behaving
  badly inside the game is not a vulnerability in Eidos. A mod archive that
  escapes its own directory during install is in scope; a mod that does something
  unpleasant once you deliberately enable it is not.
- **A missing capability.** Running without `CAP_SYS_ADMIN` is the default
  mode, not a vulnerability.
- **The capability itself, reported as a finding.** That a `cap_sys_admin+ep`
  binary is powerful is documented above, by design, and optional. A concrete way
  to abuse it is very much in scope; the general observation is not.
- **Bugs in Proton, Wine, Steam, the kernel's FUSE implementation, or the games.**
  Report those upstream. Tell us anyway if Eidos makes one meaningfully worse.
- **Vulnerabilities in third-party crates.** Report them to the crate, then open a
  normal issue here so the dependency can be bumped, unless Eidos's specific use
  makes the impact worse than upstream's advisory says.
- Anything requiring an attacker who already has your user account, or physical
  access to an unlocked machine.

## Build and release integrity

- Releases are produced by `.github/workflows/release.yml` and nowhere else:
  merging the release pull request tags the commit, and the same run builds the
  tag, tests it and hands the files to the organisation's shared signing
  workflow. Every asset is published with an ed25519 `.sig`, a `.meta` naming
  its version, file name and sha256, and a `.meta.sig`. How to check them is in
  the README's [Code signing policy](README.md#code-signing-policy).
- The signing key is an organisation secret that only the shared workflow's
  publishing job receives. That job checks out nothing and builds nothing, so
  no code from this repository or its dependencies ever runs next to the key.
- Every action in the workflows is pinned to a full commit SHA, and so is the
  shared signing workflow. Each job's token is read-only unless it publishes:
  the release-please job, the one place a third-party action
  (`googleapis/release-please-action`) runs, and the shared publishing job.
- The Rust toolchain version is pinned in the workflows, so a release is built by
  a known compiler rather than by whatever was current that day.
- `eidos`, `eidos-gui` and `eidos-linux` (a copy of `eidos-gui`) carry eight
  Microsoft DirectX redistributable DLLs, embedded with `include_bytes!` as
  gzip files made with `gzip -9 -n`: `d3dcompiler_47`, `d3dcompiler_43`,
  `d3dx9_43` and `d3dx11_43`, each in an x86_64 and an i386 build. The size
  and SHA-256 of each unpacked DLL are in
  `crates/eidos-gamefeatures/assets/PROVENANCE.md`, and a test unpacks every
  one and compares its SHA-256 with that file. Eidos unpacks only the DLL it
  deploys, copies it into the prefix's `system32`/`syswow64` and forces it
  native when a mod DLL imports `d3dcompiler_47` or a tool declares the verb as
  a prerequisite. The release
  bundle also ships `eidos-nif-preview`, a C++ helper built from nifly vendored
  at commit `f445ebc2`, with the vendored files recorded in
  `crates/eidos-nif-preview/VENDORED-SHA256SUMS`. A shipped DLL that does not
  match `PROVENANCE.md` is in scope.
- The release job downloads the Microsoft runtimes that
  `packaging/runtimes.txt` lists and stops if one's SHA-256 differs from that
  list. A test compares the list with the pins in the code and with
  `PROVENANCE.md`, and the release also stops while a notice in
  `packaging/licenses/microsoft-runtimes/` still holds a `TODO(owner)`
  placeholder.
- There is no Flatpak or AppImage build. Neither has been implemented or
  verified (see [docs/internals/packaging.md](docs/internals/packaging.md)). A
  file capability does not survive either format, but the default rootless mode
  does not use one.
