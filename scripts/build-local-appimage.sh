#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"

export VANER_REPO="${VANER_REPO:-/home/abo/repos/vaner-0.9.0-auto-focus}"
export PKG_CONFIG_PATH="/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/share/pkgconfig:${PKG_CONFIG_PATH:-}"

exec pnpm -C "$REPO_ROOT" tauri build --bundles appimage --config '{"bundle":{"createUpdaterArtifacts":false}}' "$@"
