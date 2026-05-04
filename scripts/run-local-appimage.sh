#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"

export VANER_REPO="${VANER_REPO:-/home/abo/repos/vaner-0.9.0-auto-focus}"
export VANER_BIN="${VANER_BIN:-$HERE/vaner-local-cli.sh}"
export VANER_PATH="${VANER_PATH:-$VANER_REPO}"
export VANER_DESKTOP_LOCAL_BUILD="${VANER_DESKTOP_LOCAL_BUILD:-1}"
export VANER_DESKTOP_SHOW_ON_START="${VANER_DESKTOP_SHOW_ON_START:-1}"
export VANER_DISABLE_UPDATER="${VANER_DISABLE_UPDATER:-1}"

APPIMAGE="$REPO_ROOT/src-tauri/target/release/bundle/appimage/Vaner_0.3.1_amd64.AppImage"

if [[ ! -x "$APPIMAGE" ]]; then
  chmod +x "$APPIMAGE"
fi

if [[ "${1:-}" == "--replace" ]]; then
  shift
  pkill -f "$APPIMAGE" 2>/dev/null || true
  pkill -x vaner-desktop 2>/dev/null || true
  sleep 0.5
fi

printf '[vaner-desktop-local] appimage=%s\n' "$APPIMAGE"
printf '[vaner-desktop-local] VANER_BIN=%s\n' "$VANER_BIN"
printf '[vaner-desktop-local] VANER_PATH=%s\n' "$VANER_PATH"
python3 - <<'PY'
import json
import os
from pathlib import Path

config = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))) / "vaner-desktop"
config.mkdir(parents=True, exist_ok=True)
path = config / "state.json"
try:
    state = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}
except Exception:
    state = {}
state["workspace"] = os.environ["VANER_PATH"]
path.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")
print(f"[vaner-desktop-local] state={path}")
PY
"$VANER_BIN" --version || true
"$VANER_BIN" status --json --path "$VANER_PATH" || true

if [[ "${VANER_LOCAL_START_ENGINE:-1}" == "1" ]]; then
  "$VANER_BIN" up --detach --json --path "$VANER_PATH" || true
fi

exec "$APPIMAGE" "$@"
