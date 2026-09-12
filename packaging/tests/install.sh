#!/usr/bin/env bash
# Exercise the default installer using dummy binaries and an isolated user tree.
set -euo pipefail
root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT
mkdir -p "$root/source" "$root/tools"
for binary in eidos eidos-gui; do
    printf '#!/bin/sh\nexit 0\n' > "$root/source/$binary"
    chmod +x "$root/source/$binary"
done
for command in sudo setcap; do
    printf '#!/bin/sh\n: > "$EIDOS_INSTALL_TEST_GUARD"\nexit 97\n' > "$root/tools/$command"
    chmod +x "$root/tools/$command"
done
script="$(cd "$(dirname "$0")/.." && pwd)/install.sh"
EIDOS_INSTALL_TEST_GUARD="$root/privileged" PATH="$root/tools:$PATH" XDG_DATA_HOME="$root/data" XDG_BIN_HOME="$root/Eidos Tools" \
    bash "$script" --from "$root/source" > "$root/install.log"
test -x "$root/Eidos Tools/eidos"
test -x "$root/Eidos Tools/eidos-gui"
test ! -e "$root/privileged"
if [ "$(id -u)" != 0 ]; then
    grep -Fx "Exec=\"$root/Eidos Tools/eidos-gui\"" "$root/data/applications/eidos.desktop"
fi
# The opt-in remains available without changing normal installation defaults.
bash "$script" --help | grep -q -- '--cap'
echo 'Default installation passed without sudo or setcap.'
