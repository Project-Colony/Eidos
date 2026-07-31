# Security Policy

Eidos mounts a filesystem and asks for a file capability. That is unusual for a
mod manager, so this document is specific about what it does, what could go
wrong, and what we want to hear about.

## Two things to know before you install

**1. Eidos runs a FUSE daemon as your user.** Every read and write a launched
game makes against its `Data` directory goes through Eidos's code
(`crates/eidos-fuse`) while the game runs. The mount lives in a private mount
namespace and is invisible to the rest of the system, and the daemon exits when
the game does. Nothing is mounted when Eidos is not running a game.

**2. Eidos asks for `setcap cap_sys_admin+ep` on its launch binary.** This is
what lets it unshare a plain mount namespace and turn on FUSE kernel
passthrough, without which script-extender plugin DLLs may fail to load. Be
clear-eyed about what it means:

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
- You can decline the capability entirely. Eidos falls back to a fully rootless
  user+mount namespace, prints a warning, and flags it in the GUI's Diagnostics
  tab. Everything works except FUSE passthrough.

The capability lives on the file, so replacing the file drops it. A rebuild or a
reinstall silently returns you to the rootless path.
`getcap "$(command -v eidos)"` is the check.

## Supported versions

Eidos is pre-1.0. Only the most recent release tag and the current `main` get
security fixes. There is no backport branch, and older tags are not patched.

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
  archives, FOMOD XML, INI or plugin (`.esp`/`.esm`/`.esl`) files.
- **The `nxm://` handler.** Eidos can register itself as the system handler for
  `nxm://` links, which means a web page can hand it a URL. Anything in that path
  that turns a crafted URL into a file written outside the download directory, a
  request to an attacker-chosen host, or command execution.
- **Credential handling.** The Nexus API key: where it is stored, its file
  permissions, and anywhere it could be logged or sent to a host other than
  Nexus.
- **Runtime provisioning.** Eidos can download a .NET runtime (the `dotnet10`
  prerequisite) and unpack it into `~/.local/share/eidos/runtimes/`. The expected
  SHA-256 is compiled into the binary rather than fetched alongside the file, and
  a mismatch refuses the install rather than warning about it; the archive is
  unpacked into a staging directory and renamed into place only once complete.
  Anything that defeats that - a path escaping the runtime directory during
  extraction, a checksum that can be bypassed, a download that can be pointed at
  another host - is in scope.

## Out of scope

- **Mods themselves.** Eidos mounts what you tell it to mount and launches what
  you tell it to launch. It does not sandbox mod content, and a mod behaving
  badly inside the game is not a vulnerability in Eidos. A mod archive that
  escapes its own directory during install is in scope; a mod that does something
  unpleasant once you deliberately enable it is not.
- **A missing capability.** Running without `CAP_SYS_ADMIN` is a supported
  degraded mode, not a vulnerability.
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

- Releases are produced by the tagged workflow in `.github/workflows/release.yml`
  and nowhere else. Each release publishes a SHA-256 checksum next to the
  tarball; verify with `sha256sum -c`.
- The workflows use only first-party `actions/*` steps, and the single job that
  holds a write token is the one that creates the release.
- The Rust toolchain version is pinned in the workflows, so a release is built by
  a known compiler rather than by whatever was current that day.
- There is no Flatpak or AppImage build. Both formats strip or ignore the file
  capability Eidos needs, so shipping one would mean shipping a build that
  silently degrades. The mechanism is documented in the release workflow.
