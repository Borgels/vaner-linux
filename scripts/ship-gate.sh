#!/usr/bin/env bash
#
# ship-gate.sh — L7. End-to-end verification that a signed .deb
# installs cleanly on a vanilla Ubuntu 24.04 and that the Adopt →
# handoff-file → /vaner:next loop works.
#
# Runs inside Dockerfile.ship-gate. Assumes:
#
#   * A signed .deb already built (either downloaded from a GitHub
#     Release or produced locally by `pnpm tauri build --bundles deb`).
#   * Vaner daemon wheel + CLI installed via pipx (we install it
#     inside the gate so the test is self-contained).
#
# Exits 0 on full green; non-zero on any gate failure.
#
# Usage:
#   ./scripts/ship-gate.sh [path/to/vaner_*.deb]
# If the path argument is omitted, the script looks in
# `src-tauri/target/release/bundle/deb/`.

set -euo pipefail

# ---------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
DEB_PATH=${1:-}
COCKPIT=${VANER_COCKPIT_URL:-http://127.0.0.1:8473}
HANDOFF=${XDG_STATE_HOME:-$HOME/.local/state}/vaner/pending-adopt.json
GATE_ID=$(date +%s)
LOG_DIR=$(mktemp -d)
trap 'echo; echo "[gate] logs kept at $LOG_DIR"; [[ ${KILL_DAEMON:-} ]] && kill $KILL_DAEMON 2>/dev/null || true' EXIT

step() { printf "\n[gate] %s\n" "$*"; }
fail() { printf "[gate] FAIL: %s\n" "$*" >&2; exit 1; }

# ---------------------------------------------------------------------
# 1. Locate the .deb
# ---------------------------------------------------------------------
step "locate .deb"
if [[ -z "$DEB_PATH" ]]; then
  DEB_PATH=$(find "$HERE/src-tauri/target/release/bundle/deb" -maxdepth 1 -name "*.deb" -print -quit 2>/dev/null || true)
fi
[[ -n "$DEB_PATH" && -f "$DEB_PATH" ]] || fail "no .deb at $DEB_PATH"
echo "  using: $DEB_PATH"

# ---------------------------------------------------------------------
# 2. Signature verify (only runs if the sidecar .asc is present)
# ---------------------------------------------------------------------
if [[ -f "${DEB_PATH}.asc" && -f "$HERE/scripts/release-key.asc" ]]; then
  step "verify detached signature against committed pubkey"
  GNUPGHOME=$(mktemp -d)
  export GNUPGHOME
  chmod 700 "$GNUPGHOME"
  gpg --batch --import "$HERE/scripts/release-key.asc" >/dev/null 2>&1
  gpg --verify "${DEB_PATH}.asc" "$DEB_PATH" 2> "$LOG_DIR/gpg.log" \
    || { cat "$LOG_DIR/gpg.log" >&2; fail "detached signature did not verify"; }
  echo "  signature: OK"
else
  echo "[gate] (skipping signature check — no .deb.asc present; dev build path)"
fi

# ---------------------------------------------------------------------
# 3. Install
# ---------------------------------------------------------------------
step "apt install the .deb"
sudo apt install -y "$DEB_PATH" >"$LOG_DIR/apt-install.log" 2>&1 \
  || { tail -40 "$LOG_DIR/apt-install.log" >&2; fail "apt install failed"; }

# Sanity: the visible app launcher got placed.
mapfile -t desktop_files < <(find /usr/share/applications -maxdepth 1 -iname "*vaner*.desktop" -print | sort)
[[ "${#desktop_files[@]}" -gt 0 ]] || fail "no .desktop file found after install"
if command -v desktop-file-validate >/dev/null 2>&1; then
  for candidate in "${desktop_files[@]}"; do
    desktop-file-validate "$candidate"
  done
fi
visible_desktop_files=()
for candidate in "${desktop_files[@]}"; do
  if ! grep -qx 'NoDisplay=true' "$candidate"; then
    visible_desktop_files+=("$candidate")
  fi
done
[[ "${#visible_desktop_files[@]}" -eq 1 ]] || fail "expected one visible Vaner desktop launcher, found ${#visible_desktop_files[@]}"
desktop_file="${visible_desktop_files[0]}"
desktop_name=$(basename "$desktop_file")
[[ "$desktop_name" != *" "* ]] || fail "desktop file name contains spaces: $desktop_name"
grep -qx 'Name=Vaner' "$desktop_file" || fail "desktop launcher does not display as Vaner"
grep -qx 'Exec=/usr/bin/env VANER_DESKTOP_SHOW_ON_START=1 /usr/bin/vaner-desktop' "$desktop_file" \
  || fail "desktop launcher must show on start and launch /usr/bin/vaner-desktop to avoid user PATH shadowing"
grep -qx 'StartupNotify=true' "$desktop_file" || fail "desktop launcher lacks StartupNotify=true"
echo "  desktop file: installed ($desktop_name)"

# Sanity: the binary is on PATH.
command -v vaner-desktop >/dev/null 2>&1 \
  || fail "no Vaner binary on PATH after install"
echo "  binary: on PATH"

# ---------------------------------------------------------------------
# 4. Start the Vaner daemon (requires `vaner` CLI in the gate image)
# ---------------------------------------------------------------------
if command -v vaner >/dev/null 2>&1; then
  step "start the Vaner daemon in the background"
  mkdir -p "$HOME/ship-gate-repo" && cd "$HOME/ship-gate-repo"
  [[ -d .git ]] || git init -q
  echo "# gate" > README.md; git add README.md
  git -c user.email=gate@example.com -c user.name=gate commit -qm init 2>/dev/null || true
  vaner init --path . >"$LOG_DIR/vaner-init.log" 2>&1 || fail "vaner init failed"
  vaner up --path . --detach --no-open >"$LOG_DIR/vaner-up.log" 2>&1 || fail "vaner up failed"
  # Wait up to 30s for the cockpit to answer /health.
  for _ in {1..30}; do
    if curl -fsSL "$COCKPIT/health" >/dev/null 2>&1; then break; fi
    sleep 1
  done
  curl -fsSL "$COCKPIT/health" >/dev/null 2>&1 || fail "daemon did not become ready"
  echo "  daemon: ready"
else
  echo "[gate] (skipping daemon boot — no \`vaner\` CLI in this image)"
fi

# ---------------------------------------------------------------------
# 5. Drive the Adopt flow headlessly
#
# The desktop UI writes the handoff file via the clicked Adopt
# button. To exercise the *same* underlying contract without GUI
# automation, we:
#   a. POST /predictions/{id}/adopt directly (when the daemon has a
#      live prediction).
#   b. Write the returned Resolution to $XDG_STATE_HOME/vaner/pending-adopt.json
#      using the same contract the app's AdoptHandoff uses
#      (plus a stashed_at top-level field).
#
# This proves the wire format the skill reads hasn't drifted — a
# direct click-through test requires Xvfb + xdotool; a v1.1 follow-up
# can add that on top of this baseline.
# ---------------------------------------------------------------------
if command -v curl >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
  step "adopt a prediction (if the daemon has one surfacable)"
  preds_json=$(curl -fsSL "$COCKPIT/predictions/active" || echo '{"predictions":[]}')
  adoptable_id=$(printf '%s' "$preds_json" | jq -r '
    .predictions[]? | select(.run.readiness | IN("ready","drafting")) | .id' \
    | head -n1 || true)

  if [[ -n "$adoptable_id" ]]; then
    resp=$(curl -fsSL -X POST -H 'Content-Type: application/json' --data '{}' \
           "$COCKPIT/predictions/${adoptable_id}/adopt") \
      || fail "adopt POST failed"
    mkdir -p "$(dirname "$HANDOFF")"
    printf '%s' "$resp" | jq --argjson ts "$(date +%s)" \
      '. + {stashed_at: ($ts | tonumber)}' > "$HANDOFF"
    [[ -s "$HANDOFF" ]] || fail "handoff file empty after adopt"
    jq -e '.adopted_from_prediction_id and .resolution_id and .provenance.mode' \
      "$HANDOFF" >/dev/null \
      || fail "handoff file is missing required Resolution fields"
    echo "  handoff: $HANDOFF (adopted prediction $adoptable_id)"
  else
    echo "[gate] (no drafting/ready prediction available — skipping adopt step; install-only gate)"
  fi
fi

# ---------------------------------------------------------------------
# 6. Simulate the /vaner:next skill's step 0 read of the handoff
# ---------------------------------------------------------------------
if [[ -f "$HANDOFF" ]]; then
  step "simulate /vaner:next step-0 consumption"
  python3 - <<'PY' "$HANDOFF"
import json, sys, time

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as fh:
    body = json.load(fh)

required = ("intent", "resolution_id", "provenance", "adopted_from_prediction_id", "stashed_at")
missing = [k for k in required if k not in body]
if missing:
    sys.exit(f"handoff missing required keys: {missing}")

age = time.time() - float(body["stashed_at"])
if age > 600:
    sys.exit(f"handoff older than 10 min ({age:.0f}s); skill would ignore it")

print(f"  step-0 would inject: {body['intent']!r} (age {age:.1f}s, resolution {body['resolution_id']})")
PY
else
  echo "[gate] (no handoff file to simulate; install-only gate)"
fi

# ---------------------------------------------------------------------
# 7. Remove the app + check cleanup
# ---------------------------------------------------------------------
step "uninstall the .deb — no lingering files under /usr/*"
pkg_name=$(dpkg-deb -f "$DEB_PATH" Package)
sudo apt purge -y "$pkg_name" >"$LOG_DIR/apt-purge.log" 2>&1 || fail "apt purge failed"
if find /usr/bin /usr/lib /usr/share/applications -name "*vaner*" 2>/dev/null | grep -q .; then
  fail "files remain after purge — package scripts are leaking state"
fi
echo "  uninstall: clean"

step "ship gate GREEN"
echo "  gate id: $GATE_ID"
