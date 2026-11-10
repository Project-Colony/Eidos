# The landscape, and where Eidos stands

This page covers what MO2's usvfs is, what each Linux approach costs, and which
properties are specific to Eidos. It used to be part of the README.

## The problem

MO2 is two things bolted together:

1. A Qt mod-manager UI + plugin system. Cross-platform, ports easily.
2. **`usvfs`**, the part every port has to replace: a per-process virtual
   filesystem that merges your mods over the game directory **without ever
   touching the real game files**. It works by injecting a DLL and hooking the
   Windows NT syscall layer.

Part 2 is where every Linux attempt either succeeds or compromises. The
*mechanism* usvfs uses is intrinsically Windows and cannot be ported. The
*property* it delivers - a merged view that leaves the install byte-for-byte
alone - is reachable on Linux with a FUSE filesystem, and Eidos is not the first
project to reach for one. The comparison as of mid-2026:

| Tool | Approach | What it costs you |
|---|---|---|
| [MO2 + Wine installers](https://github.com/Furglitch/modorganizer2-linux-installer) | run the real MO2 + usvfs inside the Wine prefix | usvfs under Wine: slow, fragile, and the manager itself is a Windows app |
| [Limo](https://github.com/limo-app/limo) | native C++/Qt, hardlink/symlink deploy | writes links into the game dir, no per-process isolation; no commits since 2025-05-03 |
| [Amethyst Mod Manager](https://github.com/ChrisDKN/Amethyst-Mod-Manager) | native Python, hardlink/symlink deploy across 70+ games | renames the game's `Data/` to `Data_Core/` and builds a synthetic tree in its place. Broad and actively developed, but the exact opposite of zero-touch |
| [RadTux](https://www.nexusmods.com/fallout4/mods/105285) | native daemon + DLL shim, symlinks on launch | still leans on MO2-under-Wine; symlink write-back caveats |
| [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager) | the real MO2 C++/Qt codebase with usvfs swapped for a libfuse3 low-level daemon | a genuine VFS, and the most complete Linux MO2 today. The mount is **global** (its README has you enable `user_allow_other` in `/etc/fuse.conf`), so a daemon crash leaves the real game directory stale-mounted and the project has to carry a cleanup path and a crash hook. No kernel passthrough |
| LMO (Codeberg) | native Rust on `fuse-overlayfs` | young; inherits overlay semantics rather than usvfs semantics |
| **Eidos** | native Rust FUSE union, mounted in a **private user+mount namespace**, with kernel-side metadata caching (optional passthrough) | new; one game family proven in-game so far |

### What is specific to Eidos

Fluorine-Manager got to a real FUSE VFS first, from the MO2 codebase itself, and
it is the more complete mod manager today. Two properties are specific to Eidos:
where the mount lives, and the kernel-side caching under it.

1. **The mount is private to the launched process tree.** Before mounting
   anything, Eidos calls `unshare(CLONE_NEWNS)` - or, without the capability,
   falls back to the fully rootless `CLONE_NEWUSER | CLONE_NEWNS` with
   `uid_map` / `gid_map` / `setgroups` - and then remounts `/` as
   `MS_REC | MS_PRIVATE` so nothing propagates back
   ([`crates/eidos-launch/src/lib.rs`](../../crates/eidos-launch/src/lib.rs)). The
   merged view therefore exists *only* inside the game's process tree. Your file
   manager, a second Steam game, a backup job, another user: none of them can see
   it, and none of them need permission to. It follows that **there is no
   cleanup path to get wrong.** Kill the game, pull the
   power, panic the daemon: the namespace dies with the process tree and the game
   directory is exactly what it always was. A globally mounted VFS has to survive
   its own crash and un-stale the real game directory afterwards. A private one
   has nothing to survive.
2. **Kernel-side metadata caching.** What actually stalls a Bethesda game's
   startup is not read throughput, it is the enormous number of paths Wine probes
   that do not exist. Eidos answers that in the kernel rather than in the daemon:
   negative dentries for failed lookups, long entry/attr TTLs (mod layers are
   immutable for the life of a mount), `FOPEN_CACHE_DIR` on `opendir`, directory
   opens the kernel serves without asking the daemon (`FUSE_NO_OPENDIR_SUPPORT`),
   and two case-folding hints so Wine trusts the mount to fold case instead of
   brute-force rescanning every directory: a `.ciopfs` marker and, since 1.22,
   `FS_CASEFOLD_FL` in the `FS_IOC_GETFLAGS` reply. A directory whose names need
   Unicode case folding gets neither hint, so Wine's own scan still finds them.
   Kernel `FUSE_PASSTHROUGH` (Linux 6.9+) is implemented as well but ships
   **off**. With it on, the daemon before 1.22 stopped the game opening its own
   archives and plugins; 1.22 removed the most likely cause (one backing per
   inode), and no game run has measured it since. See
   [troubleshooting.md](../guide/troubleshooting.md).

## The Eidos approach

**On Linux the game already runs in a box: Wine/Proton.** So we do not hook
anything Windows-side. We virtualize *underneath* Wine, at the Linux
filesystem level. Present a merged view at the directory Wine reads from, and the
game sees the merge for free, with no Windows-side injection at all.

Eidos reproduces the four properties that make `usvfs` valuable:

1. **Merged view** - game data + N mod layers, priority ordered (last-enabled
   mod wins on conflict).
2. **Copy-on-write** - the running game's writes (saves, regenerated configs)
   land in an *Overwrite* layer, never corrupting a mod source. A test run
   (1.24) sends one run's writes to `<instance>/test-run/` instead, and the
   user keeps or throws away each group of them afterwards.
3. **Zero-touch** - the real game directory is never modified. Since 1.24 an
   instance can run from a copy of the game folder kept in the instance
   (`eidos stock-game`), which the union mounts in place of the install.
4. **Per-process scope** - the merge only exists for the launched game, inside a
   private mount namespace; the rest of the system sees the pristine directory.

### Architecture in one picture

```
              +------------ per-launch user+mount namespace ------------+
  Steam  -->  |   eidos launch wrapper   (`eidos-linux %command%`)      |
              |        |                                                |
              |        v                                                |
  Proton   <--|   merged view  <--  Eidos FUSE union                    |
  sees ONE    |                         ^          ^           ^        |
  directory   |                    overwrite    mod N..1    game data   |
              +---------------------------------------------------------+
       (the rest of the system sees only the pristine game directory)
```

The store starts the game. Steam runs it through the game's launch option
(`eidos-linux %command%`, or `eidos play <id> -- %command%` with the separate
`eidos` binary), and Heroic and Legendary run it through a wrapper with the same
arguments. Since 1.23, Run in a window no store opened asks the store to start
the game (`steam -applaunch`, `heroic://launch`, `legendary launch`), so Eidos
never starts a game through Proton itself and the store's Proton choice and
prefix apply. Tools are the exception: `eidos tool` starts them itself in the
game's prefix, with the Proton Steam has set for the game, or with the Wine or
Proton build Heroic has set for a Heroic copy.

Engine choice: a **FUSE union filesystem in Rust**, which keeps full control of
the semantics OverlayFS cannot express (exact Windows-style case-insensitivity,
precise write redirection, no lowerdir scaling wall). Kernel **passthrough**
(Linux 6.9+) is implemented for the data path but off by default, so resolved
reads are served by the daemon from a cached backing fd. The daemon before 1.22
broke Skyrim SE's own archive and plugin opens with it on; 1.22 shares one
backing per inode, which removes the most likely cause, but no game run has
measured it since. See
[docs/internals/architecture.md](../internals/architecture.md) for the full rationale, including
why FUSE over OverlayFS for completeness and long-term stability.

The data path was never the bottleneck anyway. Metadata (`lookup`, `getattr`,
`readdir`) crosses into the daemon, and that traffic is what stalls a
Bethesda game's startup, because Wine probes enormous numbers of paths that do
not exist (DLL search-order walks, `.ini` sidecars, script-extender config
probes). Eidos answers that kernel-side: failed lookups reply as **negative
dentries** rather than a bare `ENOENT`, cached for 3600 s when the daemon records
the spelling (so a differently-cased create can invalidate it) and 60 s when it
cannot. Positive entry/attr TTLs run long because mod layers are immutable for
the life of a mount, and `opendir` sets `FOPEN_CACHE_DIR` so the kernel serves
repeat enumerations itself. Requests are served from several event loops over
`clone_fd`.

`FOPEN_KEEP_CACHE` is off by default, and `EIDOS_FUSE_KEEP_CACHE=1` turns it on.
With it on, Skyrim SE crashed after its main menu (see
[troubleshooting.md](../guide/troubleshooting.md)). That bisect ran with the
capability binary, where the kernel fails a passthrough open that carries
`FOPEN_KEEP_CACHE` with `EIO`, so every passthrough open failed. The crash does
not show that the flag is harmful on its own, and passthrough replies now drop
it. It waits for a re-test with passthrough off. Repeat reads cross into the
daemon while passthrough is off, so the flag could help there.

The escape hatches ship with the caching, because "the game sees stale data" has
to be testable against caching as the suspect in a single run:
`EIDOS_FUSE_NO_CACHE=1` turns all of it off and `EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir`
names them one at a time, which is what let the crash above be bisected to a
single flag in four launches. `EIDOS_FUSE_STATS=1` dumps per-op counters at
unmount, and `EIDOS_FUSE_THREADS=1` restores single-threaded serving when
diagnosing a concurrency bug.

The counters show where the time went. A measured 9.5-minute session, 7 mods,
passthrough off, before the `opendir` change below:

```
lookup 10702 (846 missing, 7.9%), getattr 18019, opendir 516301, readdir <low>,
open 8298, read 26999, write 69
```

Directory OPENS dominate by two orders of magnitude over reads, and almost none of
them enumerate anything: Wine opens a directory, asks whether it folds case, and
closes it again on every failed path resolution. The `.ciopfs` marker answers the
question, but the kernel has no cache for opening a directory inode, while it does
cache the `lookup` that follows, which is why `lookup` is 50x smaller and hid this
for so long. `probe` counts the handles closed without a single `readdir`, so the
split between probing and enumerating is measured rather than inferred. At
roughly 10 us per open, the opens cost about 1% of that session.

The fix is a protocol change. When the kernel advertises
`FUSE_NO_OPENDIR_SUPPORT`, the daemon answers the first `OPENDIR` with `ENOSYS`,
and the kernel then opens and reads directories from its own cache without
sending another one: 516,301 directory opens per session became 1 (see
[performance.md](../internals/performance.md)). `EIDOS_FUSE_OPENDIR=1` keeps the
handles, for bisecting a directory bug against the old path.
