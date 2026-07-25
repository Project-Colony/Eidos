#!/usr/bin/env bash
#
# Eidos tarball installer.
#
# Copies the binaries into place, applies the CAP_SYS_ADMIN file capability that
# kernel FUSE passthrough needs, and registers the nxm:// handler so the Nexus
# "Mod Manager Download" button lands in your instance. Re-running it is safe and
# is in fact the supported way to upgrade: every step overwrites rather than
# appends, and the capability is re-applied because a new binary is a new inode
# and never inherits the old one's.
#
# Usage:
#   ./install.sh                     install into ~/.local/bin (default)
#   ./install.sh --system            install into /usr/local/bin (uses sudo)
#   ./install.sh --bindir DIR        install into DIR
#   ./install.sh --from DIR          take the binaries from DIR
#   ./install.sh --no-cap            skip setcap (print the command instead)
#
# On Arch, prefer the package: packaging/PKGBUILD applies the capability through
# the package payload, so pacman owns it. See docs/packaging.md.

set -euo pipefail

# Everything ships together; the GUI locates the privileged CLI as its own
# sibling before falling back to PATH, so they must share a directory.
REQUIRED_BINS=(eidos eidos-gui)
OPTIONAL_BINS=(eidos-fuse eidos-launch)

# Only `eidos` is capped. Capabilities are per-file and this is the binary the
# product mounts through; handing CAP_SYS_ADMIN to more files than that widens
# the blast radius for no gain.
CAP_BIN=eidos
CAP=cap_sys_admin+ep

bindir=""
srcdir=""
system=0
apply_cap=1
cap_skip_reason=""

die() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }
say() { printf '  %s\n' "$*"; }

usage() {
	cat <<-'EOF'
		Install Eidos: binaries, the CAP_SYS_ADMIN file capability that kernel FUSE
		passthrough needs, and the nxm:// download handler. Safe to re-run, and
		re-running is how you upgrade - a rebuilt binary never inherits the old
		one's capability.

		usage:
		  ./install.sh                install into ~/.local/bin (default)
		  ./install.sh --system       install into /usr/local/bin (uses sudo)
		  ./install.sh --bindir DIR   install into DIR
		  ./install.sh --from DIR     take the binaries from DIR
		  ./install.sh --no-cap       skip setcap, print the command instead

		On Arch, prefer the package: packaging/PKGBUILD ships the capability in the
		package payload, so pacman owns it. See docs/packaging.md.
	EOF
}

while [[ $# -gt 0 ]]; do
	case "$1" in
		--system)  system=1; shift ;;
		--bindir)  bindir="${2:-}"; [[ -n "$bindir" ]] || die "--bindir needs a directory"; shift 2 ;;
		--from)    srcdir="${2:-}"; [[ -n "$srcdir" ]] || die "--from needs a directory"; shift 2 ;;
		--no-cap)  apply_cap=0; shift ;;
		-h|--help) usage; exit 0 ;;
		*)         die "unknown option '$1' (try --help)" ;;
	esac
done

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Where the binaries are. A release tarball puts them beside this script; a
# source checkout puts them under target/. Take the first layout that has them.
if [[ -z "$srcdir" ]]; then
	for candidate in "$here" "$here/bin" "$here/../target/release" "$here/../target/debug"; do
		if [[ -x "$candidate/eidos" && -x "$candidate/eidos-gui" ]]; then
			srcdir="$candidate"
			break
		fi
	done
fi
[[ -n "$srcdir" ]] || die "no built binaries found - run 'cargo build --release' first, or pass --from DIR"
[[ -d "$srcdir" ]] || die "no such directory: $srcdir"
srcdir="$(cd "$srcdir" && pwd)"
for b in "${REQUIRED_BINS[@]}"; do
	[[ -x "$srcdir/$b" ]] || die "missing $srcdir/$b"
done

# Where they go.
if [[ -z "$bindir" ]]; then
	if (( system )); then
		bindir=/usr/local/bin
	else
		bindir="${XDG_BIN_HOME:-$HOME/.local/bin}"
	fi
fi

# Questions about a directory we have not created yet (may I write it? which
# filesystem is it on?) are really questions about its nearest existing
# ancestor. On a fresh account neither ~/.local/bin nor ~/.local exists.
nearest_existing() {
	local p="$1"
	while [[ ! -e "$p" ]]; do
		local parent
		parent="$(dirname "$p")"
		[[ "$parent" == "$p" ]] && break
		p="$parent"
	done
	printf '%s\n' "$p"
}

# Testing only the immediate parent would answer "no" on a fresh account and
# send the whole install through sudo, which then creates root-owned directories
# inside the user's own home. Walk up instead.
can_write() {
	[[ -w "$(nearest_existing "$1")" ]]
}

# Anything we cannot write ourselves goes through sudo, one step at a time,
# rather than asking for the whole script to be run as root: running the desktop
# registration as root would write the handler into root's home, where nothing
# would ever read it.
sudo_if_needed() {
	local target_dir="$1"; shift
	if can_write "$target_dir"; then
		"$@"
	elif command -v sudo >/dev/null; then
		sudo "$@"
	else
		die "need root to write $target_dir and sudo is not installed - re-run as root, or use --bindir"
	fi
}

# setcap is different: being able to write the file is not enough, it needs
# CAP_SETFCAP, which in practice means root. Owning ~/.local/bin buys you
# nothing here, so never route this through the writability test above.
run_as_root() {
	if [[ "$(id -u)" -eq 0 ]]; then
		"$@"
	elif command -v sudo >/dev/null; then
		sudo "$@"
	else
		return 1
	fi
}

echo "Eidos install"
echo "  from: $srcdir"
echo "  to:   $bindir"
echo

# A file capability is silently ignored on a nosuid mount: setcap reports
# success, getcap shows the bits, and the kernel grants nothing at exec time.
# Catching it here is the difference between a clear message now and a
# "why are my SKSE plugins not loading" hunt later.
mount_opts=",$(findmnt -no OPTIONS --target "$(nearest_existing "$bindir")" 2>/dev/null || true),"
if (( apply_cap )) && [[ "$mount_opts" == *,nosuid,* ]]; then
	echo "WARNING: $bindir is on a nosuid mount. The kernel ignores file"
	echo "         capabilities there, so Eidos would mount rootless whatever we do."
	echo "         Install somewhere else (--bindir DIR, or --system) if you want"
	echo "         FUSE passthrough."
	echo
	apply_cap=0
	cap_skip_reason=nosuid
fi

echo "binaries"
sudo_if_needed "$bindir" install -d -m755 "$bindir"
for b in "${REQUIRED_BINS[@]}" "${OPTIONAL_BINS[@]}"; do
	[[ -x "$srcdir/$b" ]] || continue
	sudo_if_needed "$bindir" install -m755 "$srcdir/$b" "$bindir/$b"
	say "$bindir/$b"
done
echo

echo "capability"
capped=0
if (( apply_cap )); then
	if ! command -v setcap >/dev/null; then
		say "setcap not found - install libcap, then run:"
		say "  sudo setcap $CAP $bindir/$CAP_BIN"
	elif run_as_root setcap "$CAP" "$bindir/$CAP_BIN"; then
		capped=1
		say "$(getcap "$bindir/$CAP_BIN" 2>/dev/null || echo "$bindir/$CAP_BIN $CAP")"
		say "kernel FUSE passthrough enabled (script-extender DLLs will image-map)"
	else
		say "could not apply it - run this yourself:"
		say "  sudo setcap $CAP $bindir/$CAP_BIN"
	fi
elif [[ "$cap_skip_reason" == nosuid ]]; then
	say "not applied - a nosuid mount cannot carry one, so setcap would lie to you"
else
	say "skipped - run this yourself when you want passthrough:"
	say "  sudo setcap $CAP $bindir/$CAP_BIN"
fi
(( capped )) || say "without it Eidos still works, but mounts rootless"
echo

echo "nxm:// handler"
# Delegate to the product's own registration so there is exactly one definition
# of that desktop entry. It writes ~/.local/share/applications/eidos-nxm.desktop
# with this binary's real path and points xdg-mime at it.
if [[ -n "${SUDO_USER:-}" ]]; then
	# We were invoked through sudo: register for the human, not for root.
	if sudo -u "$SUDO_USER" "$bindir/eidos" nxm --register >/dev/null 2>&1; then
		say "registered for $SUDO_USER"
	else
		say "could not register - run as your own user: eidos nxm --register"
	fi
elif [[ "$(id -u)" -eq 0 ]]; then
	say "running as root, so skipping - run as your own user: eidos nxm --register"
elif "$bindir/eidos" nxm --register >/dev/null 2>&1; then
	say "$HOME/.local/share/applications/eidos-nxm.desktop"
	say "the Nexus \"Mod Manager Download\" button now downloads through Eidos"
else
	say "could not register - run it yourself: eidos nxm --register"
fi
echo

echo "desktop entry"
# The GUI launcher. No application icon ships in the tree yet, so use a stock
# freedesktop name rather than a dangling Icon= key that renders as a blank tile.
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
if [[ "$(id -u)" -eq 0 && -z "${SUDO_USER:-}" ]]; then
	say "skipped (running as root)"
else
	[[ -n "${SUDO_USER:-}" ]] && apps="$(getent passwd "$SUDO_USER" | cut -d: -f6)/.local/share/applications"
	mkdir -p "$apps"
	cat > "$apps/eidos.desktop" <<-EOF
		[Desktop Entry]
		Type=Application
		Name=Eidos
		Comment=Mod manager for games running under Proton
		Exec=$bindir/eidos-gui
		Icon=applications-games
		Categories=Game;Utility;
		Terminal=false
		StartupNotify=true
	EOF
	[[ -n "${SUDO_USER:-}" ]] && chown "$SUDO_USER" "$apps/eidos.desktop"
	command -v update-desktop-database >/dev/null && update-desktop-database "$apps" 2>/dev/null || true
	say "$apps/eidos.desktop"
fi
echo

case ":$PATH:" in
	*":$bindir:"*) ;;
	*)
		echo "NOTE: $bindir is not on your PATH. Add it to your shell profile:"
		echo "        export PATH=\"$bindir:\$PATH\""
		echo
		;;
esac

echo "Done. Next:"
echo "  $bindir/eidos games          # what Eidos can see on this machine"
echo "  $bindir/eidos-gui            # the GUI"
echo
echo "Steam launch option for a game (absolute path: Steam does not read your PATH):"
echo "  WINEDLLOVERRIDES=\"d3dcompiler_47=n\" $bindir/eidos-gui %command%"
echo
echo "Re-run this script after every upgrade. A rebuilt binary is a new file and"
echo "does not inherit the capability - that is a kernel rule, not an Eidos quirk."
