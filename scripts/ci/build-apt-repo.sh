#!/usr/bin/env bash
# build-apt-repo.sh — assemble (or update) a signed apt repository
# rooted at $1, using reprepro + the Vaner release GPG key.
#
# On each release the workflow:
#   1. Clones (or fresh-creates) the `gh-pages` branch into a staging
#      directory.
#   2. Calls this script with the staging dir as the root.
#   3. Pushes the staging dir back to `gh-pages`.
#
# GitHub Pages serves the resulting tree at
#   https://apt.vaner.ai/
# (or the custom domain the user configures via a CNAME file at the
# repo root).
#
# Users then install with:
#
#   curl -fsSL https://apt.vaner.ai/release-key.asc \
#       | sudo gpg --dearmor -o /etc/apt/keyrings/vaner.gpg
#   echo "deb [signed-by=/etc/apt/keyrings/vaner.gpg] \
#        https://apt.vaner.ai stable main" \
#       | sudo tee /etc/apt/sources.list.d/vaner.list
#   sudo apt update && sudo apt install vaner-desktop
#
# Required env:
#   VANER_RELEASE_GPG_PRIVKEY, VANER_RELEASE_GPG_PASSPHRASE,
#   VANER_RELEASE_GPG_FINGERPRINT  (same keys sign-artifacts.sh uses)
#
# Usage:
#   build-apt-repo.sh <repo-root> <path-to-signed.deb> [<more-debs...>]

set -euo pipefail

: "${VANER_RELEASE_GPG_PRIVKEY:?set VANER_RELEASE_GPG_PRIVKEY}"
: "${VANER_RELEASE_GPG_PASSPHRASE:?set VANER_RELEASE_GPG_PASSPHRASE}"
: "${VANER_RELEASE_GPG_FINGERPRINT:?set VANER_RELEASE_GPG_FINGERPRINT}"

repo_root=${1:?usage: build-apt-repo.sh <repo-root> <deb>...}
shift

command -v reprepro >/dev/null || { echo "reprepro missing; sudo apt install reprepro" >&2; exit 2; }

# Isolated GNUPGHOME. Loopback pinentry so loose gpg calls (not
# reprepro's own signing, which we skip — see below) can accept a
# passphrase from the command line.
gnupghome=$(mktemp -d); chmod 700 "$gnupghome"
trap 'rm -rf "$gnupghome"' EXIT
export GNUPGHOME="$gnupghome"
echo "allow-loopback-pinentry" > "$gnupghome/gpg-agent.conf"
echo "pinentry-mode loopback"  > "$gnupghome/gpg.conf"
echo "$VANER_RELEASE_GPG_PRIVKEY" | base64 -d | gpg --batch --import

imported_fpr=$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr:/ {print $10; exit}')
[[ "$imported_fpr" == "${VANER_RELEASE_GPG_FINGERPRINT//[[:space:]]/}" ]] \
  || { echo "ERROR: fingerprint mismatch" >&2; exit 3; }

mkdir -p "$repo_root/conf"

# `conf/distributions` — describes the `stable` dist. One component
# (`main`), one arch (`amd64`) for v0.1; ARM64 / Debian-testing stanzas
# can be appended later. SignWith is deliberately omitted — reprepro's
# gpgme integration can't easily pick up the passphrase from an
# ephemeral gpg-agent, and wrangling gpg-preset-passphrase in CI is
# fragile. Instead we let reprepro produce an unsigned Release, then
# sign it below with plain `gpg --clearsign / --detach-sign` against
# the loopback-passphrase invocation, which is rock-solid.
cat > "$repo_root/conf/distributions" <<EOF
Origin: Vaner
Label: Vaner Desktop Linux
Codename: stable
Suite: stable
Components: main
Architectures: amd64
DebIndices: Packages Release . .gz .bz2
DscIndices: Sources Release . .gz .bz2
Tracking: keep includechanges
Description: Signed apt repository for the Vaner desktop Linux companion.
EOF

cat > "$repo_root/conf/options" <<EOF
verbose
basedir $repo_root
EOF

# Historical package-name rename (2026-04 / pre-0.1.0 release):
# the desktop client was briefly published as the apt package `vaner`
# before we reserved that name for the daemon CLI. Evict any stale
# entry of the old name so an existing clone of gh-pages doesn't keep
# advertising it. `reprepro remove` is a no-op once the entry is gone.
echo "→ reprepro remove stable vaner (legacy package name, safe no-op if absent)"
reprepro --basedir "$repo_root" \
         --gnupghome "$gnupghome" \
         remove stable vaner || true

# Include each provided .deb. reprepro is idempotent on re-runs for
# the same version; if we're re-releasing the same .deb it just no-
# ops. A newer version supersedes the older one in the pool.
for d in "$@"; do
  [[ -f "$d" ]] || { echo "ERROR: .deb not found: $d" >&2; exit 4; }
  echo "→ reprepro includedeb stable $d"
  reprepro --basedir "$repo_root" \
           --gnupghome "$gnupghome" \
           includedeb stable "$d"
done

# Sign the Release file for every distribution reprepro just wrote.
# `Release.gpg` is the detached signature (older apt), `InRelease` is
# the clearsigned variant (modern apt prefers this).
echo "→ signing Release files under dists/"
while IFS= read -r release; do
  dist_dir=$(dirname "$release")
  echo "   $release"
  gpg --batch --yes --pinentry-mode loopback \
      --passphrase "$VANER_RELEASE_GPG_PASSPHRASE" \
      --local-user "$imported_fpr" \
      --armor --detach-sign --output "$dist_dir/Release.gpg" "$release"
  gpg --batch --yes --pinentry-mode loopback \
      --passphrase "$VANER_RELEASE_GPG_PASSPHRASE" \
      --local-user "$imported_fpr" \
      --clearsign --output "$dist_dir/InRelease" "$release"
done < <(find "$repo_root/dists" -maxdepth 3 -type f -name "Release")

# Copy the public key to the repo root so users can fetch it from a
# predictable URL without navigating GitHub.
cp "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/release-key.asc" \
   "$repo_root/release-key.asc"

# CNAME file so GitHub Pages serves from apt.vaner.ai instead of
# borgels.github.io. Needs a matching CNAME DNS record:
#   apt  CNAME  borgels.github.io.
# and Repo Settings → Pages → Custom domain set to apt.vaner.ai.
# Regenerated on every release so a hand-rm'd CNAME can't persist.
echo "apt.vaner.ai" > "$repo_root/CNAME"

# Drop a little human-friendly index.html so browsers visiting the
# root see what they're looking at.
cat > "$repo_root/index.html" <<'EOF'
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Vaner Desktop Linux — apt repository</title>
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <style>
    body { font-family: ui-sans-serif, system-ui, sans-serif; max-width: 680px; margin: 48px auto; padding: 0 20px; line-height: 1.55; color: #1a1720; }
    h1 { font-size: 22px; font-weight: 600; margin-bottom: 4px; }
    h1 .accent { color: #b27a2e; }
    pre { background: #f4efe8; padding: 14px; border-radius: 8px; overflow-x: auto; font-size: 13px; }
    code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
    a { color: #6d4f9a; }
    .hint { font-size: 13px; color: #555; }
  </style>
</head>
<body>
  <h1>vaner<span class="accent">_</span> · apt repository</h1>
  <p>Signed <code>.deb</code> packages for the Vaner Linux desktop companion.</p>

  <pre><code>curl -fsSL https://apt.vaner.ai/release-key.asc \
  | sudo gpg --dearmor -o /etc/apt/keyrings/vaner.gpg

echo "deb [signed-by=/etc/apt/keyrings/vaner.gpg] https://apt.vaner.ai stable main" \
  | sudo tee /etc/apt/sources.list.d/vaner.list

sudo apt update
sudo apt install vaner-desktop</code></pre>

  <p class="hint">Release key fingerprint:
    <code>506B8FA959917D530E5EE7203D219B47A7E4F046</code>
  </p>
  <p class="hint">Source: <a href="https://github.com/Borgels/vaner-desktop">github.com/Borgels/vaner-desktop</a>
    · Docs: <a href="https://docs.vaner.ai/desktop">docs.vaner.ai/desktop</a></p>
</body>
</html>
EOF

echo
echo "apt repo updated at $repo_root"
find "$repo_root/dists" -maxdepth 3 -name "Release*" -o -name "InRelease" -o -name "Packages*" 2>/dev/null | sort
