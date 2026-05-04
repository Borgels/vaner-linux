# vaner-desktop

Cross-platform (macOS + Linux + Windows) desktop companion for
[Vaner](https://github.com/Borgels/vaner) — the local-first preparation
engine. Menu-bar / tray app that surfaces Prepared Work from the daemon:
review notes, bug hypotheses, docs drift, virtual diffs, research briefs, and
ready prediction-backed opportunities.

The app is intentionally non-mutating. It can inspect or export Vaner-owned
content, but it does not apply patches or edit project files automatically.

Tauri v2 + SvelteKit. Rust backend depends on the shared
[`vaner-contract`](https://github.com/Borgels/vaner/tree/main/crates/vaner-contract)
crate from the Vaner monorepo. macOS, Linux, and Windows release artifacts
ship from this repository.

Targets:

- **macOS:** macOS 14+ recommended. Universal `.dmg` runs on Apple Silicon
  and Intel Macs. Unsigned for the 0.x prereleases — Gatekeeper requires
  right-click → **Open** on first launch.
- **Linux:** Ubuntu 22.04+ / Debian 12+, X11 or KDE Wayland. Stock
  GNOME on Wayland needs `gnome-shell-extension-appindicator` for the
  tray icon to appear — the app detects this at first launch and
  surfaces install guidance.
- **Windows:** Windows 10 1809+ (uses the system WebView2 runtime).
  Unsigned for the 0.x prereleases — SmartScreen will show "Windows
  protected your PC → More info → Run anyway" on first install.

## Install

### macOS

Download the universal `.dmg` from the latest GitHub Release, open it,
and drag Vaner into Applications.

```bash
VER=$(curl -fsSL https://api.github.com/repos/Borgels/vaner-desktop/releases/latest | jq -r .tag_name)
curl -LO https://github.com/Borgels/vaner-desktop/releases/download/$VER/vaner-desktop_${VER#v}_universal.dmg
open vaner-desktop_${VER#v}_universal.dmg
```

> **Gatekeeper:** the 0.x DMGs are unsigned. On first launch,
> right-click Vaner → **Open**.

### Linux

Three paths, all signed — pick whichever fits your workflow:

### 1. Apt repository (recommended — auto-upgrades via `apt upgrade`)

The installer adds a signed apt repo at `https://apt.vaner.ai` and
installs the `vaner` desktop package. Every future release arrives
through `apt upgrade` without you running anything else. The executable
is still `vaner-desktop`; the Python engine/CLI remains the PyPI
package and command `vaner`. If the CLI is ever shipped through apt, it
should use the package name `vaner-cli`.

```bash
curl -fsSL https://vaner.ai/desktop.sh | bash
```

Prefer the plain-apt form (identical result, no pipe-to-bash):

```bash
curl -fsSL https://apt.vaner.ai/release-key.asc \
  | sudo gpg --dearmor -o /etc/apt/keyrings/vaner.gpg
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/vaner.gpg] https://apt.vaner.ai stable main" \
  | sudo tee /etc/apt/sources.list.d/vaner.list
sudo apt update && sudo apt install vaner
```

`arch=amd64` keeps apt from asking the repo for i386 package lists
(the repo is amd64-only).

### 2. One-off `.deb` (no apt-repo registration)

```bash
VANER_MODE=deb curl -fsSL https://vaner.ai/desktop.sh | bash
```

Same fingerprint-pin + detached-sig verification as the apt path;
subsequent releases don't auto-install unless you re-run.

### 3. Manual GPG verify then `apt install`

```bash
VER=$(curl -fsSL https://api.github.com/repos/Borgels/vaner-desktop/releases/latest | jq -r .tag_name)
curl -LO https://github.com/Borgels/vaner-desktop/releases/download/$VER/vaner_${VER#v}_amd64.deb
curl -LO https://github.com/Borgels/vaner-desktop/releases/download/$VER/vaner_${VER#v}_amd64.deb.asc
curl -LO https://github.com/Borgels/vaner-desktop/releases/download/$VER/release-key.asc

gpg --import release-key.asc
gpg --verify vaner_${VER#v}_amd64.deb.asc vaner_${VER#v}_amd64.deb
sudo apt install ./vaner_${VER#v}_amd64.deb
```

The release key fingerprint is
`506B8FA959917D530E5EE7203D219B47A7E4F046` — pinned in
[`scripts/install.sh`](scripts/install.sh), published on
[keys.openpgp.org](https://keys.openpgp.org/search?q=release@vaner.ai),
and also available as `scripts/release-key.asc` on `main`.

### 4. `.AppImage` (no apt, no install)

Every release ships an `.AppImage` alongside the `.deb`. Download,
verify, `chmod +x`, run:

```bash
VER=$(curl -fsSL https://api.github.com/repos/Borgels/vaner-desktop/releases/latest | jq -r .tag_name)
curl -LO https://github.com/Borgels/vaner-desktop/releases/download/$VER/vaner_${VER#v}_amd64.AppImage
curl -LO https://github.com/Borgels/vaner-desktop/releases/download/$VER/vaner_${VER#v}_amd64.AppImage.asc
curl -LO https://github.com/Borgels/vaner-desktop/releases/download/$VER/release-key.asc
gpg --import release-key.asc
gpg --verify vaner_${VER#v}_amd64.AppImage.asc vaner_${VER#v}_amd64.AppImage
chmod +x vaner_${VER#v}_amd64.AppImage
./vaner_${VER#v}_amd64.AppImage
```

### Windows

Download the NSIS installer from the latest GitHub Release and run
it. The installer is per-user — no admin prompt — and registers an
auto-updater that follows the same minisign-signed `latest.json`
flow as the AppImage.

```powershell
$ver = (Invoke-RestMethod https://api.github.com/repos/Borgels/vaner-desktop/releases/latest).tag_name
$url = "https://github.com/Borgels/vaner-desktop/releases/download/$ver/vaner-desktop_${ver -replace '^v',''}_x64-setup.exe"
Invoke-WebRequest $url -OutFile vaner-desktop-setup.exe
Start-Process .\vaner-desktop-setup.exe
```

> **SmartScreen:** the 0.x installers are unsigned. Windows will say
> "Windows protected your PC". Click **More info → Run anyway**.
> Code-signing is on the roadmap for 1.0.

## Updates

The app checks for updates on every launch via
[`tauri-plugin-updater`](https://v2.tauri.app/plugin/updater/); every
update is signed with a separate minisign key whose public half is
embedded in the app. A small banner appears in the popover when a
new release is ready; click **Install** to download + verify +
replace in place. The apt-repo path gets the same updates through
your system's normal update flow — pick one, not both.

## Status

- [x] L1: `vaner-contract` crate (upstream)
- [x] L2: conformance fixtures bridge (upstream)
- [x] L3: Swift conformance test consumption (scheduled with Vaner tag)
- [x] L4: Tauri app skeleton
- [x] L5: tray + popover + menu + first-run AppIndicator modal
- [x] L6: signed `.deb` release workflow + install.sh verification
- [x] L7: Docker ship-gate (`Dockerfile.ship-gate` + `scripts/ship-gate.sh`)

## Build

Prereqs (Linux):
```bash
# Ubuntu 24.04 system deps for WebKitGTK-based Tauri:
sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf
# Rust toolchain (1.85+ for edition 2024):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Node 20+ and pnpm:
corepack enable && corepack prepare pnpm@latest --activate
```

Dev:
```bash
pnpm install
pnpm tauri dev
```

Build a local Linux bundle:
```bash
pnpm tauri build --bundles deb,appimage
```

### macOS

Prereqs (macOS): Xcode command-line tools, Rust 1.85+, Node 20+ with
corepack, and the Tauri macOS build dependencies.

```bash
pnpm install
pnpm tauri build --bundles dmg --target universal-apple-darwin
```

### Windows

Prereqs (Windows): Rust 1.85+ with the `x86_64-pc-windows-msvc` target,
Node 20+ with corepack, and the WebView2 runtime (Windows 11 has it
preinstalled; Tauri auto-installs it on Windows 10 if missing).

```powershell
pnpm install
pnpm tauri build --bundles nsis
```

The installer lands at
`src-tauri/target/release/bundle/nsis/vaner-desktop_<version>_x64-setup.exe`.

## Architecture

Short version:

```
┌─────────────────────────────────────────────┐
│          vaner daemon (Python)              │
│     /prepared-work  •  /work-products       │
│     /events/stream  •  /status              │
└──────────────────────┬──────────────────────┘
                       │ HTTP / SSE (loopback)
                       │
          ┌────────────▼──────────────┐
          │  vaner-contract (Rust)    │
          │  • models + enums         │
          │  • HTTP client + SSE      │
          │  • reducer + handoff      │
          └────────────┬──────────────┘
                       │ (compiled in)
                       │
          ┌────────────▼──────────────┐
          │  Tauri v2 Rust backend    │
          │  commands + SSE task      │
          └────────────┬──────────────┘
                       │ invoke / emit
                       │
          ┌────────────▼──────────────┐
          │   SvelteKit (WebView)     │
          │   QuietShell popover UI   │
          └───────────────────────────┘
```

Design tokens (`src/lib/tokens.css`) are vendored from Vaner's
`ui/cockpit/src/styles/tokens.css` so the visual language stays 1:1
with the web cockpit and the SwiftUI macOS app.

The normal UI asks `/prepared-work` for card-shaped data and follows the
server-provided action links for inspect, export, dismiss, and feedback. Older
prediction endpoints remain available for diagnostic flows but are no longer
the primary product surface.

## License

Apache-2.0, inherited from the Vaner project.
