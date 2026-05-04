#!/usr/bin/env bash
set -euo pipefail

VANER_REPO="${VANER_REPO:-/home/abo/repos/vaner-0.9.0-auto-focus}"

cd "$VANER_REPO"
exec uv run vaner "$@"
