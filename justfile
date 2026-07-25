# Eidos task runner (https://just.systems). `just` lists the recipes.
#
# The recipe that earns its keep is `build`. A file capability is an xattr on an
# inode, and every `cargo build` writes a brand new inode over the old binary, so
# the CAP_SYS_ADMIN that FUSE passthrough needs is gone after every single
# rebuild. Nothing errors out when that happens: the launch path falls back to a
# rootless mount, reads stop going through the kernel, and script-extender DLLs
# quietly fail to image-map in-game. Hours get lost to that. So `build` rebuilds
# *and* re-applies the capability, and you never run bare `cargo build` again.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Cargo profile to build. Override per invocation: `just profile=dev build`.
# Any profile the workspace declares works - `release`, `dev`, or a custom one
# added later to the workspace Cargo.toml (`just profile=fast build`).
profile := "release"

# Respect an out-of-tree target dir, which is common on machines that keep
# builds off a slow or nosuid-mounted home.
target_dir := env_var_or_default("CARGO_TARGET_DIR", "target")

# `dev` is the one profile whose output directory is not named after it.
out_dir := target_dir / (if profile == "dev" { "debug" } else { profile })

# Only `eidos` needs the capability: the GUI shells out to `eidos play ...` for
# the actual mount (see find_eidos_binary() in eidos-gui), so the privileged
# work happens in the CLI binary and the GUI stays unprivileged.
eidos_bin := out_dir / "eidos"
gui_bin := out_dir / "eidos-gui"

# List the recipes.
default:
    @just --list --unsorted

# Rebuild the workspace, then re-apply the capability the rebuild just wiped.
build: && setcap
    cargo build --workspace --profile {{ profile }}

# Re-apply CAP_SYS_ADMIN to the built `eidos` binary. Needs sudo; idempotent.
setcap:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="{{ eidos_bin }}"
    if [[ ! -x "$bin" ]]; then
        echo "just: no binary at $bin - run 'just build' first." >&2
        exit 1
    fi
    # A nosuid mount makes the kernel ignore file capabilities outright, so
    # setcap would report success and change precisely nothing at exec time.
    # Catch that here rather than three hours later, in-game, with no DLLs.
    opts=",$(findmnt -no OPTIONS --target "$bin" 2>/dev/null || true),"
    if [[ "$opts" == *,nosuid,* ]]; then
        echo "just: $bin sits on a nosuid mount - the kernel ignores file" >&2
        echo "      capabilities there, so setcap cannot help. Build elsewhere" >&2
        echo "      (CARGO_TARGET_DIR=/some/normal/path just build)." >&2
        exit 1
    fi
    if getcap "$bin" | grep -q cap_sys_admin; then
        echo "cap_sys_admin already present on $bin"
        exit 0
    fi
    echo "+ sudo setcap cap_sys_admin+ep $bin"
    sudo setcap cap_sys_admin+ep "$bin"
    getcap "$bin"

# Runs the built binary rather than `cargo run` on purpose: the GUI locates the
# privileged `eidos` as its own sibling, so it has to start from the directory
# the build just wrote, not from a cargo shim.

# Build (with the capability re-applied) and start the GUI.
run-gui: build
    {{ gui_bin }}

# Build and run the CLI: `just run games`, `just run play skyrimse`.
run *args: build
    {{ eidos_bin }} {{ args }}

# The whole suite: unit tests plus the real-mount FUSE integration test.
test:
    cargo test --workspace

# This one mounts a real union in a private user+mount namespace and drives it
# through the kernel. It skips itself when the namespace or /dev/fuse is
# unavailable, so a green run is not proof that it actually mounted - `just
# doctor` tells you whether this machine can.

# Only the real-mount FUSE integration test.
test-fuse:
    cargo test -p eidos-fuse --test union

# Clippy over every target, warnings fatal (what CI enforces).
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format the workspace.
fmt:
    cargo fmt --all

# Check formatting without rewriting anything (what CI enforces).
fmt-check:
    cargo fmt --all -- --check

# Everything CI runs, in CI's order.
ci: fmt-check lint test

# The first question in every bug report is "does your eidos binary still have
# the capability". This answers it for every copy on the machine, and checks the
# three other things that silently downgrade a mount.

# Diagnose the mount path: capability state, kernel passthrough, FUSE plumbing.
doctor:
    #!/usr/bin/env bash
    set -uo pipefail

    echo "== eidos doctor =="
    echo

    kernel="$(uname -r)"
    # Kernel FUSE passthrough (FUSE_PASSTHROUGH) landed in 6.9. Below that the
    # capability buys nothing, because there is no passthrough to negotiate.
    if [[ "$(printf '%s\n6.9\n' "${kernel%%-*}" | sort -V | head -1)" == "6.9" ]]; then
        echo "kernel        $kernel (>= 6.9, FUSE passthrough available)"
    else
        echo "kernel        $kernel (< 6.9: no FUSE passthrough, reads stay in userspace)"
    fi

    [[ -c /dev/fuse ]] \
        && echo "/dev/fuse     present$([[ -w /dev/fuse ]] && echo ", writable" || echo ", NOT writable by you")" \
        || echo "/dev/fuse     MISSING - no mount is possible (modprobe fuse)"

    if command -v fusermount3 >/dev/null; then
        echo "fusermount3   $(command -v fusermount3)"
    else
        echo "fusermount3   MISSING - rootless mounts need it (install fuse3)"
    fi

    userns="$(cat /proc/sys/kernel/unprivileged_userns_clone 2>/dev/null || echo 1)"
    [[ "$userns" == "1" ]] \
        && echo "user ns       unprivileged user namespaces enabled" \
        || echo "user ns       DISABLED - the per-launch private namespace cannot be created"

    echo
    echo "-- eidos binaries and their capability state --"

    # Both build profiles, not just the selected one: the copy you forgot to
    # re-cap is exactly the one you are not thinking about right now.
    found=0
    seen=""
    for bin in "{{ eidos_bin }}" "{{ target_dir }}/debug/eidos" "{{ target_dir }}/release/eidos" \
               "$HOME/.local/bin/eidos" "$HOME/.cargo/bin/eidos" \
               /usr/local/bin/eidos /usr/bin/eidos; do
        [[ -f "$bin" ]] || continue
        [[ "$seen" == *"|$bin|"* ]] && continue
        seen="$seen|$bin|"
        found=1
        caps="$(getcap "$bin" 2>/dev/null)"
        opts=",$(findmnt -no OPTIONS --target "$bin" 2>/dev/null || true),"
        if [[ "$opts" == *,nosuid,* ]]; then
            state="NOSUID MOUNT - file capabilities are ignored here, whatever getcap says"
        elif [[ "$caps" == *cap_sys_admin* ]]; then
            state="OK - cap_sys_admin present"
        else
            state="MISSING - rootless fallback, script-extender DLLs may not load"
        fi
        printf '  %-40s %s\n' "$bin" "$state"
    done
    (( found )) || echo "  (none found - run 'just build')"

    echo
    echo "Fix any MISSING with:  sudo setcap cap_sys_admin+ep <path>"
    echo "or just:               just setcap        (for {{ eidos_bin }})"
    echo "Every rebuild of a binary wipes its capability. That is expected."

# Install into ~/.local/bin (binaries, capability, nxm:// handler).
install: build
    packaging/install.sh --from {{ out_dir }}

# Remove build output.
clean:
    cargo clean
