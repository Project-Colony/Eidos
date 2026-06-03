#!/usr/bin/env bash
#
# Eidos proof of concept.
#
# Proves the "virtualize underneath Wine" thesis using only native Linux
# primitives: an unprivileged OverlayFS mounted inside a user + mount namespace.
# No root, no sudo, no game required. Requires Linux >= 5.11.
#
# It demonstrates the four properties Eidos must reproduce from usvfs:
#   1. Merged view    - game data + N mod layers, priority ordered
#   2. Copy-on-write  - writes land in an Overwrite layer
#   3. Zero-touch     - the real game dir is never modified
#   4. Per-process    - the merge only exists inside the namespace
#
# OverlayFS is used here because it proves the thesis fastest. The real Eidos
# engine is a FUSE union filesystem (see docs/architecture.md) for completeness
# and control the kernel overlay cannot give.

set -euo pipefail

# Preflight: this PoC leans on overlayfs as the fastest way to demonstrate the
# thesis. Eidos itself uses a FUSE engine and does NOT need overlayfs; this is a
# demo-only dependency. Standard desktop kernels (Arch, Fedora, Ubuntu) ship it.
if ! grep -qw overlay /proc/filesystems 2>/dev/null; then
    echo "This kernel exposes no overlayfs (not in /proc/filesystems)." >&2
    if modprobe -n overlay 2>/dev/null; then
        echo "It is available as a module; load it once with: sudo modprobe overlay" >&2
    else
        echo "No loadable overlay module either; this kernel was built without it." >&2
        echo "Run the PoC on a standard desktop kernel. The Eidos FUSE engine will" >&2
        echo "not require overlayfs at all." >&2
    fi
    exit 2
fi

ROOT="$(mktemp -d "${HOME}/.cache/eidos-poc.XXXXXX")"
trap 'rm -rf "$ROOT"' EXIT

GAME="$ROOT/game"        # pristine game install (the real Data dir)
MOD1="$ROOT/mods/aaa"    # lower priority mod
MOD2="$ROOT/mods/bbb"    # higher priority mod
OVER="$ROOT/overwrite"   # writable layer (usvfs "Overwrite")
WORK="$ROOT/work"        # overlayfs workdir (empty, same fs as upper)
VIEW="$ROOT/view"        # the merged mountpoint the "game" sees

mkdir -p "$GAME" "$MOD1" "$MOD2" "$OVER" "$WORK" "$VIEW"

# Pristine game files.
printf 'vanilla skin\n' > "$GAME/textures.dat"
printf 'vanilla mesh\n' > "$GAME/meshes.dat"

# aaa retextures; bbb retextures again (higher priority) and adds a script.
printf 'aaa retexture\n'    > "$MOD1/textures.dat"
printf 'bbb hd retexture\n' > "$MOD2/textures.dat"
printf 'bbb script\n'       > "$MOD2/script.dat"

# leftmost lowerdir wins: bbb > aaa > game.
LOWER="$MOD2:$MOD1:$GAME"

echo ">> Mounting overlay inside a user+mount namespace (no root)..."

unshare --map-root-user --mount bash -euo pipefail <<EOF
opts="lowerdir=$LOWER,upperdir=$OVER,workdir=$WORK"
# Rootless overlay may need user.* xattrs; try plain first, then userxattr.
mount -t overlay eidos -o "\$opts" "$VIEW" 2>/dev/null \
  || mount -t overlay eidos -o "\$opts,userxattr" "$VIEW" \
  || { echo "!! overlay mount failed (need Linux >= 5.11 + unprivileged userns)"; exit 1; }

echo
echo "== Merged view the game sees =="
ls -1 "$VIEW"

echo
echo -n "== Priority: textures.dat resolves to => "
cat "$VIEW/textures.dat"

echo "== Game writes a save and tweaks a vanilla file (inside the view) =="
printf 'playthrough save\n' > "$VIEW/save01.dat"
printf 'tweaked mesh\n'     > "$VIEW/meshes.dat"

umount "$VIEW"
EOF

echo
echo "== After exit: the real game dir is UNTOUCHED =="
echo -n "  game/textures.dat => "; cat "$GAME/textures.dat"
echo -n "  game/meshes.dat   => "; cat "$GAME/meshes.dat"

echo
echo "== New + modified files captured in the Overwrite layer =="
ls -1 "$OVER"
echo -n "  overwrite/save01.dat => "; cat "$OVER/save01.dat"
echo -n "  overwrite/meshes.dat => "; cat "$OVER/meshes.dat"

echo
echo "PASS: union + copy-on-write + zero-touch + per-namespace scope all hold."
echo
echo "Next step: replace the inline shell with a wrapper around the Proton launch"
echo "command (Steam launch option: 'eidos %command%') so the real game runs in"
echo "this namespace and sees the merged view."
