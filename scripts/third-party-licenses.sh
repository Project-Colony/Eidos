#!/usr/bin/env bash
# Copy the license notices of every crate compiled into the shipped binaries
# into OUT_DIR: one folder per crate holding the LICENSE / COPYING / NOTICE
# files its package carries, plus INDEX.tsv with each crate's declared SPDX
# expression. MIT, BSD and Apache-2.0 all require these notices to travel with
# a binary, and the release bundle and the Arch package both ship one.
#
# Usage: scripts/third-party-licenses.sh OUT_DIR [CARGO_FLAG...]
# Extra flags (for example --frozen) go to cargo; --locked is the default.
# Needs the dependencies fetched already; it builds nothing.
set -euo pipefail

out=${1:?usage: $0 OUT_DIR [CARGO_FLAG...]}
shift
flags=("$@")
[[ ${#flags[@]} -gt 0 ]] || flags=(--locked)
target=x86_64-unknown-linux-gnu
mkdir -p "$out"

# Normal edges only: build scripts and proc-macros run on the build host and
# never reach the binary.
shipped=$(cargo tree "${flags[@]}" --edges normal --target "$target" \
	--prefix none --format '{p}' \
	-p eidos -p eidos-gui -p eidos-fuse -p eidos-launch |
	sed 's/ (.*//' | sort -u)

printf 'crate\tlicense\n' > "$out/INDEX.tsv"
cargo metadata "${flags[@]}" --format-version 1 --filter-platform "$target" |
	jq -r '.packages[] | select(.source != null)
		| [.name + " v" + .version, (.license // "see the crate"),
		   (.manifest_path | rtrimstr("/Cargo.toml"))] | @tsv' |
	sort |
	while IFS=$'\t' read -r pkg license dir; do
		grep -qxF "$pkg" <<<"$shipped" || continue
		dest="$out/${pkg/ v/-}"
		mkdir -p "$dest"
		find "$dir" -maxdepth 1 -type f \( -iname 'licen[cs]e*' \
			-o -iname 'copying*' -o -iname 'notice*' \) -exec cp {} "$dest/" \;
		# A package that carries no notice file still gets its index line.
		rmdir --ignore-fail-on-non-empty "$dest"
		printf '%s\t%s\n' "$pkg" "$license" >> "$out/INDEX.tsv"
	done
