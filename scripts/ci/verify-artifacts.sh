#!/usr/bin/env bash
# verify-artifacts.sh — belt-and-braces pre-upload check on every
# signature produced by sign-artifacts.sh.

set -euo pipefail

bundle_dir=${1:?usage: verify-artifacts.sh <bundle-dir>}
# Absolute path — the SHA256SUMS cross-check subshell `cd`s into the
# individual artifact dirs and would otherwise resolve $bundle_dir
# against the wrong cwd.
bundle_dir=$(cd "$bundle_dir" && pwd -P)
pubkey_path="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/release-key.asc"
[[ -f "$pubkey_path" ]] || { echo "ERROR: scripts/release-key.asc missing" >&2; exit 2; }

gnupghome=$(mktemp -d); chmod 700 "$gnupghome"
trap 'rm -rf "$gnupghome"' EXIT
export GNUPGHOME="$gnupghome"

gpg --batch --import "$pubkey_path"

# Detached sig for every .deb and .AppImage.
mapfile -t asc_files < <(find "$bundle_dir" -maxdepth 3 -type f -name "*.asc" | sort)
for sig in "${asc_files[@]}"; do
  payload="${sig%.asc}"
  [[ -f "$payload" ]] || { echo "orphan signature without payload: $sig" >&2; exit 3; }
  echo "→ verify $(basename "$payload")"
  gpg --verify "$sig" "$payload"
done

# SHA256SUMS + cross-check digests.
[[ -f "$bundle_dir/SHA256SUMS" && -f "$bundle_dir/SHA256SUMS.asc" ]] \
  || { echo "SHA256SUMS(.asc) missing in $bundle_dir" >&2; exit 4; }
gpg --verify "$bundle_dir/SHA256SUMS.asc" "$bundle_dir/SHA256SUMS"

# Run the checksum check from each leaf directory so the relative
# paths in SHA256SUMS resolve.
while read -r _ file; do
  path=$(find "$bundle_dir" -maxdepth 3 -type f -name "$file" | head -1)
  [[ -n "$path" ]] || { echo "missing listed file: $file" >&2; exit 5; }
  echo "→ checksum $file"
  (cd "$(dirname "$path")" && sha256sum -c <<<"$(awk -v f="$file" '$2==f' "$bundle_dir/SHA256SUMS")")
done < "$bundle_dir/SHA256SUMS"

# Embedded dpkg-sig check (only for .deb, only if tool present).
if command -v dpkg-sig >/dev/null; then
  while read -r d; do
    echo "→ dpkg-sig --verify $(basename "$d")"
    dpkg-sig --verify "$d"
  done < <(find "$bundle_dir" -maxdepth 3 -type f -name "*.deb")
fi

# Desktop launcher sanity for Debian artifacts. Ubuntu/GNOME indexing is
# sensitive to launcher metadata, so catch regressions before upload.
while read -r d; do
  echo "→ inspect desktop launcher $(basename "$d")"
  tmpdir=$(mktemp -d)
  dpkg-deb -x "$d" "$tmpdir"
  mapfile -t desktop_files < <(find "$tmpdir/usr/share/applications" -maxdepth 1 -type f -iname "*vaner*.desktop" 2>/dev/null | sort)
  [[ "${#desktop_files[@]}" -gt 0 ]] || {
    printf 'expected at least one Vaner desktop file in %s\n' "$d" >&2
    rm -rf "$tmpdir"
    exit 6
  }
  if command -v desktop-file-validate >/dev/null; then
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
  [[ "${#visible_desktop_files[@]}" -eq 1 ]] || {
    printf 'expected one visible Vaner desktop launcher in %s, found %s\n' "$d" "${#visible_desktop_files[@]}" >&2
    rm -rf "$tmpdir"
    exit 6
  }
  desktop_file="${visible_desktop_files[0]}"
  desktop_name=$(basename "$desktop_file")
  [[ "$desktop_name" != *" "* ]] || {
    echo "desktop file name contains spaces: $desktop_name" >&2
    rm -rf "$tmpdir"
    exit 6
  }
  grep -qx 'Name=Vaner' "$desktop_file" || {
    echo "desktop launcher does not display as Vaner" >&2
    rm -rf "$tmpdir"
    exit 6
  }
  grep -qx 'Exec=/usr/bin/env VANER_DESKTOP_SHOW_ON_START=1 /usr/bin/vaner-desktop' "$desktop_file" || {
    echo "desktop launcher must show on start and launch /usr/bin/vaner-desktop to avoid user PATH shadowing" >&2
    rm -rf "$tmpdir"
    exit 6
  }
  grep -qx 'StartupNotify=true' "$desktop_file" || {
    echo "desktop launcher lacks StartupNotify=true" >&2
    rm -rf "$tmpdir"
    exit 6
  }
  rm -rf "$tmpdir"
done < <(find "$bundle_dir" -maxdepth 3 -type f -name "*.deb")

echo "all signatures verified"
