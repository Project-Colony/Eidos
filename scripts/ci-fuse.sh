#!/usr/bin/env bash
# Prepare real FUSE tests on disposable GitHub-hosted Ubuntu runners only.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && "${RUNNER_ENVIRONMENT:-}" == github-hosted ]] || {
    echo "This script is restricted to GitHub-hosted CI runners." >&2
    exit 1
}
sudo modprobe fuse
if ! command -v fusermount3 >/dev/null; then
    sudo apt-get update -qq
    sudo apt-get install -y -qq fuse3
fi
if [[ -e /proc/sys/kernel/apparmor_restrict_unprivileged_userns ]]; then
    sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
fi
test -e /dev/fuse
command -v fusermount3
unshare --map-root-user --mount true
