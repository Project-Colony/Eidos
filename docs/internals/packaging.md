# Packaging and distribution

This document exists so nobody has to ask "why is there no Flatpak" again.

Short version: Eidos needs a **file capability** on one binary, and a file
capability is a property of an inode on a normal filesystem. Every distribution
format that isolates the application from the filesystem (Flatpak) or serves it
from a runtime-mounted image (AppImage) destroys that property by construction.
Not by policy, not by an unset option: by the kernel rules that make those
formats safe in the first place. So Eidos ships as a native package, a tarball
with an installer, or a source build.

Everything asserted below was measured on Linux 7.1.4. The commands are included
so you can re-measure it.

## Install

| Channel | Command | Capability applied by |
|---|---|---|
| Arch package | `cd packaging && makepkg -si` | the package payload (pacman restores it) |
| Tarball | `./install.sh` | the installer, via one `sudo setcap` |
| From source | `just build` | the `build` recipe, every time |

## The constraint

`eidos` mounts the merged view and negotiates **kernel FUSE passthrough**, which
is what lets the kernel serve reads and `mmap` straight from the real backing
file. That is not a performance nicety: it is what allows Windows script-extender
DLLs (SKSE and its ~50 plugins) to image-map natively from the mount. The kernel
grants passthrough only to a process holding `CAP_SYS_ADMIN` **in the initial
user namespace** (see the comment in `crates/eidos-fuse/src/lib.rs`).

The minimal way to get that is a file capability on the one binary that needs it:

```sh
sudo setcap cap_sys_admin+ep /path/to/eidos
```

Not setuid-root, not "run the mod manager with sudo". One capability, one binary.
The GUI is unprivileged and shells out to `eidos` for the mount.

Three consequences that drive everything else in this document:

1. **A capability lives on the inode**, in the `security.capability` xattr.
2. **Only real root can set it.** As a normal user, even on your own file:
   ```
   $ setcap cap_sys_admin+ep ./myfile
   unable to set CAP_SETFCAP effective capability: Operation not permitted
   ```
3. **Every rebuild wipes it**, because `cargo build` writes a new file. This is
   not an Eidos quirk, it is how the kernel works. It is also why `just build`
   rebuilds and re-applies in one step, and why you should stop typing bare
   `cargo build` in this repo.

Without the capability Eidos still runs. It mounts rootless, reads are correct,
and everything is served by the daemon instead of the kernel. What you lose is
passthrough, and with it the reliable image-mapping of script-extender DLLs. It
degrades quietly, which is exactly why `just doctor` exists.

## Why Flatpak is impossible

Two independent kernel rules, either one fatal.

**A sandboxed process cannot gain privileges at exec.** `flatpak run` builds its
sandbox with bubblewrap, and bubblewrap sets `no_new_privs`:

```
$ bwrap --dev-bind / / --unshare-user grep NoNewPrivs /proc/self/status
NoNewPrivs:	1
```

With that bit set, `execve` promises not to grant anything the caller did not
already have, and file capabilities are part of "anything".

**Everything in the sandbox is mounted nosuid.** Same command, looking at the
mount instead of the process:

```
$ bwrap --dev-bind / / --unshare-user cat /proc/self/mountinfo
947 433 0:28 /@ / rw,nosuid,relatime ... - btrfs ...
```

`nosuid` tells the kernel to ignore setuid bits *and file capabilities* on that
mount. Both rules are load-bearing for sandbox security. Neither is a flag
Flatpak forgot to flip.

And even if both somehow fell away, the capability would be granted inside the
sandbox's own user namespace, while kernel FUSE passthrough demands
`CAP_SYS_ADMIN` in the **initial** namespace. A nested capability is the wrong
capability. Add that a user-mode `flatpak install` runs unprivileged and so
cannot write `security.capability` into its deploy tree at all (rule 2 above),
and there is no angle left to attack.

## Why AppImage is a trap

AppImage looks like it should work, because it is "just a binary". It is not: it
is a squashfs image that mounts itself at runtime through **squashfuse**, and
every unprivileged FUSE mount is `nosuid`. `fusermount3` hardcodes it. Every FUSE
mount on a running system shows it, including Eidos's own:

```
$ grep fuse /proc/self/mountinfo          # columns trimmed
/run/user/1000/gvfs   rw,nosuid,nodev,relatime - fuse.gvfsd-fuse ...
/tmp/eidos-audit-...  rw,nosuid,nodev,relatime - fuse eidos ...
```

So the payload binary is executed from a nosuid mount, and the kernel ignores its
capabilities. The AppImage would launch, run, mount rootless, and never tell you
why the DLLs did not load. That is worse than not shipping one.

The `--appimage-extract-and-run` fallback does not save it either: the extraction
is done by the unprivileged user running the AppImage, who cannot write
`security.capability` onto the extracted copy (rule 2 again).

## Verify it yourself

No root needed, no sudo, nothing touched outside a private namespace. This maps
`nosuid` and `no_new_privs` onto one binary and shows what the kernel grants:

```sh
unshare -Urm --map-users=auto --map-groups=auto --propagation private sh -c '
  mkdir -p /tmp/capexp && mount -t tmpfs tmpfs /tmp/capexp
  cp /usr/bin/capsh /tmp/capexp/capsh
  setcap cap_sys_admin+ep /tmp/capexp/capsh

  echo "A. normal mount, normal exec:"
  setpriv --reuid=1 --regid=1 --clear-groups /tmp/capexp/capsh --print | grep ^Current:

  echo "B. same binary, nosuid mount (AppImage, and inside any bwrap sandbox):"
  mount -o remount,nosuid,bind /tmp/capexp
  setpriv --reuid=1 --regid=1 --clear-groups /tmp/capexp/capsh --print | grep ^Current:

  echo "C. normal mount, exec under no_new_privs (every Flatpak):"
  mount -o remount,suid,bind /tmp/capexp
  python3 -c "
import ctypes, os
ctypes.CDLL(None).prctl(38, 1, 0, 0, 0)   # PR_SET_NO_NEW_PRIVS
os.setgid(1); os.setuid(1)
os.execv(\"/tmp/capexp/capsh\", [\"capsh\", \"--print\"])" | grep ^Current:
'
```

```
A. normal mount, normal exec:
Current: cap_sys_admin=ep
B. same binary, nosuid mount (AppImage, and inside any bwrap sandbox):
Current: =
C. normal mount, exec under no_new_privs (every Flatpak):
Current: =
```

One binary, one capability, three environments. Only the plain one works.

Note that `setpriv --no-new-privs` does **not** reproduce case C: it still shows
`cap_sys_admin=ep`. Set the bit with `prctl` directly, as bubblewrap does, and
the capability disappears. Worth knowing before you conclude from a quick
`setpriv` test that sandboxing is harmless here.

## The Arch package

`packaging/PKGBUILD`. The received wisdom is that a PKGBUILD cannot apply a file
capability and needs a `.install` scriptlet with a `post_install` hook running
`setcap`. That is not true, and the scriptlet route is worse.

A plain `setcap` in `package()` works, because:

1. makepkg runs `package()` under **fakeroot**, which intercepts the `*xattr`
   syscalls and records `security.capability` in its own bookkeeping. So `setcap`
   succeeds without real root.
2. makepkg then archives `$pkgdir` with **bsdtar in the same fakeroot session**.
   bsdtar reads the emulated xattr back and writes it into the payload as
   `SCHILY.xattr.security.capability`. No extra flag is required: makepkg's plain
   `bsdtar --no-fflags --no-read-sparse -cnLf -` already captures it.
3. **pacman restores xattrs** when it extracts as root, so the capability lands
   on the installed file.

Proof that does not require building anything, on any Arch box:

```sh
getcap /usr/bin/btop                       # cap_dac_read_search,cap_perfmon=ep
ls /var/lib/pacman/local/btop-*/install    # nothing: btop has no scriptlet
```

btop carries a file capability and its package has no install scriptlet, because
the capability came out of the payload. It is visible in the package itself, as a
pax header on `usr/bin/btop`:

```
LIBARCHIVE.xattr.security.capability
SCHILY.xattr.security.capability
```

A `post_install` scriptlet would be strictly worse: it would have to re-run on
every upgrade, and the capability would be invisible to `pacman -Qkk`, so
verification would report the binary as unmodified while its privileges came from
somewhere pacman does not track. Putting it in the payload makes it owned,
verifiable and removed with the package.

Two ways this still fails, both environmental and neither fixable by packaging:

- `/usr` mounted `nosuid`. Nothing can grant a capability there.
- A filesystem that does not carry `security.*` xattrs. ext4, btrfs, xfs and f2fs
  all do.

Only Arch is implemented and verified. Nothing has been attempted for `.deb` or
`.rpm`; on those, use the tarball installer, which applies the capability
explicitly and tells you what it did.

## The tarball installer

`packaging/install.sh`. Copies the binaries, applies the capability with one
`sudo setcap`, registers the `nxm://` handler through `eidos nxm --register`
(so there is one definition of that desktop entry, not two), and prints every
path it wrote.

It is idempotent, and re-running it is the supported way to upgrade. It also
refuses to pretend: if the install directory is on a `nosuid` mount it says so
and skips the capability rather than running a `setcap` that would report success
and change nothing. `/tmp` is `nosuid` on many systems, which is worth
remembering if you ever point `CARGO_TARGET_DIR` there.

The GUI locates the privileged `eidos` as its own sibling before falling back to
`PATH`, so the binaries must stay in the same directory. The installer keeps them
together; if you move things by hand, move them together.

## Building from source

```sh
just build        # rebuild + re-apply the capability. Use this, not cargo build.
just doctor       # capability state of every eidos on the machine, plus kernel
                  # passthrough support, /dev/fuse, fusermount3, user namespaces
just run-gui      # build, re-apply, launch the GUI
just test         # unit tests + the real-mount FUSE integration test
just install      # build, then run packaging/install.sh
```

`just doctor` is the first thing to run when a launch behaves as though the mods
are not there. It answers the actual first question, which is whether the binary
still has its capability, for every copy on the machine, and it flags a `nosuid`
install directory even when `getcap` looks fine.
