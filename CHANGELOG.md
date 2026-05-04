# Changelog

## [Unreleased]

### Added

- **macOS .dmg in the release workflow** — `build-macos` job (macos-14, Apple Silicon runner, `tauri build --bundles dmg --target universal-apple-darwin`) produces a universal-2 .dmg that runs on both arm64 and x86_64 Macs. Wired into the staging + signing + release-publish steps; SHA256SUMS covers the .dmg too. Tauri's auto-updater `latest.json` carries a `darwin-universal` block when a minisign sig is present (TAURI_SIGNING_PRIVATE_KEY set).
- The macOS build runs unsigned by default — sets `APPLE_*` env vars from optional repo secrets so users can flip on Developer ID signing + notarization later without touching the workflow. Until those are set, the .dmg ships unsigned and users see "Apple cannot verify Vaner.app" on first launch (right-click → Open to bypass).
- **Winget manifest triple** at `winget/manifests/v/Vaner/Desktop/0.2.4/` — version, installer, en-US locale. Pinned to the published sha256 of `vaner-desktop_0.2.4_x64-setup.exe`. PR target is `microsoft/winget-pkgs`.
- **Auto-PR winget manifest on tag push** — `release.yml` installs `wingetcreate` and submits a manifest update to `microsoft/winget-pkgs` for every tagged release. Reads the new `.exe` URL straight from the just-published GitHub release. Fail-soft: missing `WINGET_TOKEN` secret prints a warning and skips the step instead of failing the release. Set up a fork of `microsoft/winget-pkgs` under your account and store a PAT (`public_repo` scope) as `WINGET_TOKEN`.
- **Auto-bump Homebrew Cask on tag push** — `release.yml` clones `Borgels/homebrew-vaner`, rewrites `Casks/vaner-desktop.rb` with the new version + .dmg sha256, switches the URL to the just-published `Borgels/vaner-desktop` release, and pushes. Same fail-soft pattern: missing `HOMEBREW_TAP_TOKEN` secret skips the step. Set a fine-grained PAT with `Contents: Read & Write` on `Borgels/homebrew-vaner` as `HOMEBREW_TAP_TOKEN`.

### Changed

- Homebrew Cask source switches from `Borgels/vaner-desktop-macos` to `Borgels/vaner-desktop` (this repo) the first time the auto-bump runs against a release that includes a Tauri .dmg. After that, `vaner-desktop-macos` (the Swift app) can be sunset.

## [0.3.0] - 2026-05-02

### Added

- Companion Focus pane backed by daemon `/focus`, with Work Here, Pin, Pause,
  Resume, Manual Only, and Auto controls.
- Thin Tauri commands for daemon-owned focus, resource, and job status.
- Auto Focus privacy copy explaining the supported-client detection boundary.

## [0.2.4] - 2026-05-02

### Added

- **Single-source engine status** — one Rust-side poll loop now owns the
  cadence (5 s base, 500 ms during a boost window after Restart) and
  emits `engine:status` events to every webview. The popover and the
  companion Engine pane subscribe to the same event channel, so they
  can no longer disagree about whether the engine is up. Pre-fix each
  webview ran its own `setInterval`; a stale stub in the companion
  could happily contradict a fresh probe in the popover.
- **`.ollamaMissing` popover state + in-app installer** — Vaner's
  default backend is Ollama on `127.0.0.1:11434`; if it isn't installed
  the model loop 502s on every MCP call. The popover now names the
  *cause* ("Ollama is missing") rather than the *symptom* ("engine
  unavailable"), with a primary CTA that launches `curl -fsSL
  https://ollama.com/install.sh | sh` inside a real terminal so the
  sudo prompt + layer progress are visible to the user. Falls through
  the `gnome-terminal → konsole → xfce4-terminal → kitty → alacritty
  → foot → xterm` lookup order.
- **Onboarding "select clients to wire" UX** — the wizard's "Vaner is
  ready" slide now shows one checkbox per detected client, pre-checked
  for any client that isn't already `ready`. Already-wired clients
  show a ✓ + "Already wired" muted line — no checkbox, no action.
  Single primary "Install Vaner into N agent(s)" button at the bottom;
  per-client failures surface their own toast without aborting the
  bulk run. After the bulk install, the panel flips to the verify
  view in-place so the user sees status badges + verification phrase
  for any partials.
- **Companion → Agents pane: per-layer status** — embeds the same
  leverage panel from onboarding (MCP / Primer / Skill / Plugin per
  client) alongside the existing install/uninstall list. Per-row
  refresh on Repair, no more reloading the whole table.
- **Docs deep-links** — every companion pane header has a "Docs ↗"
  pill that opens the matching docs.vaner.ai page (Prepared, Agents,
  Models, Engine, Preferences, Diagnostics).
- **GPU-gated runtime presets** — Light / Balanced / Performance now
  write `compute.idle_gpu_threshold` (the meaningful gate for a
  GPU-bound loop) in addition to `compute.idle_cpu_threshold`. CPU
  fraction stays as a runaway-loop ceiling, not the dimension that
  decides preset.
- **Companion-window geometry persistence** — `state.json` carries
  the last position + size; the companion lands where you left it.
- **Open Vaner button on completion** — actually opens the companion
  before closing onboarding (was: closed onboarding into an empty
  desktop).

### Changed

- **Popover reframe when no workspace is set** — pre-fix the popover
  shouted "ENGINE UNAVAILABLE" any time the cockpit was silent. After
  setup a daemon legitimately isn't running until an MCP client calls
  it, so the new copy is "Vaner is set up. The engine starts when
  your first agent calls it. Open Cursor / Claude Code / Zed in a
  repo and Vaner kicks in." The scary copy stays only when a workspace
  IS persisted and the engine genuinely flopped.
- **Friendly preset / bundle / tier labels** — `local_balanced`,
  `high_performance`, etc. are no longer surfaced as raw enum strings.
- **Hydration skeleton on the Models pane** — picker shows a greyed-
  out skeleton until `loadBackendConfig` resolves; no more "nothing
  selected for two seconds, then it pops in" flicker.
- **Engine-pane preset highlight by behaviour** — `classifyPreset` now
  matches by `idle_only` + `max_cycle_seconds` instead of exact
  `cpu_fraction` values, so the preset the wizard wrote highlights as
  "Balanced" instead of falling through to "no preset selected".
- **Helper text trimmed** across Engine / Models / Preferences. Long
  prose pushed to docs.vaner.ai via the new DocsLink pill.

### Fixed

- **Popover ↔ companion engine-state drift** — the headline architecture
  fix (see Added). On fresh installs the companion's Engine pane was
  showing "Running ✓" while the popover showed "ENGINE UNAVAILABLE",
  same engine, same instant. Resolved by routing all reads through the
  Rust-side cache.
- **`tauri-plugin-positioner` zbus-executor panic** — `Tray position
  not set` at `ext.rs:301:17` printed a stack trace to stderr on every
  popover open under SNI panels that don't report tray geometry. Skip
  `Position::TrayBottomCenter` entirely on Linux (the fallback
  monitor-edge placement was the deterministic landing zone anyway)
  and install a panic hook that swallows that specific message.
- **`bring_up_engine` no longer pretends `$HOME` is a repo** — pre-fix
  the Restart-engine button asked the daemon to mkdir `/.vaner` and
  watch it explode. Returns `NoWorkspace` cleanly when the user hasn't
  picked one.
- **`set_local_model` passes `--path`** — the Models pane's "switch
  to model X" stopped breaking when launched from a `.desktop` entry
  whose cwd is `/`.
- **Setup wizard "Save Vaner settings" lands on first try** — the wizard
  now resolves the workspace via the same `repo_root_arg()` helper
  the engine uses, so first-install completion no longer races the
  workspace probe.

### Internal

- New stores: `ollama-health.ts`, `daemon-audit.ts`. New components:
  `OllamaMissing.svelte`, `StrayDaemonsBanner.svelte`,
  `NotWiredToAnyClient.svelte`, `DocsLink.svelte`. New Rust modules:
  `engine_status_task.rs`, `ollama_health_task.rs`, `daemon_audit.rs`.
- Reducer: 24 / 24 unit tests passing including the three new Ollama-
  precedence cases (Ollama-missing overrides engine-error,
  cliMissing wins over Ollama-missing, Ollama-present-but-cockpit-
  silent stays as `.error`).
- `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check`
  clean; `pnpm check` 0 errors; `pnpm test -- --run` 24 / 24.

## [0.2.3] - 2026-05-01

### Fixed
- **Popover not draggable on Linux** — decorationless windows on Linux
  can't be moved without an explicit `data-tauri-drag-region`. The 8px
  invisible strip that was meant to fix this in 0.2.2 was undiscoverable
  in practice. The whole `QuietShell` header is now the drag region, so
  every popover state (engineMissing, error, watching, prepared, …) is
  draggable from the brand area at the top.
- **Misleading "Engine error" on fresh install** — installing the
  `.deb` on a machine that has never seen the `vaner` CLI showed the
  attention-styled "Engine isn't responding on localhost / Restart
  engine" panel. Now `engine_status` distinguishes "CLI not found" (a
  fresh-install case) from "CLI present, daemon down" (a real error),
  and the reducer routes the former to `.notInstalled` (clear install
  CTA) instead of `.error`.

### Changed (release metadata)
- `bundle.publisher` / `bundle.copyright` / `bundle.homepage` /
  `bundle.license` are now populated. Without them the `.deb` showed
  `Maintainer: vaner` and the Windows installer showed `Publisher:
  Unknown`.
- `Cargo.toml` `[package].authors` and `homepage` populated to match.
- `package.json` carries `author`, `license`, `homepage`, `repository`.

### Added (release-time guards)
- `release.yml` refuses to build a release when:
  - `bundle.publisher`, `bundle.copyright`, `bundle.homepage`,
    `bundle.license`, or `bundle.shortDescription` is empty in
    `tauri.conf.json`.
  - The version triple (`tauri.conf.json`, `package.json`,
    `src-tauri/Cargo.toml`) drifts apart.
  - `src-tauri/Cargo.toml` is missing `[package].authors`.

  Same shape as the existing fingerprint / pubkey placeholder checks.
  Means a future "shipped a release with placeholder maintainer"
  failure is impossible at the workflow level, not just by convention.

## [0.2.2] - 2026-05-01

### Late additions (post-redesign, before tag)

After the popover redesign landed, three more PRs went into 0.2.2:

- **Companion panes + popover pinning + diagnostics surface** (#10
  follow-up): broke the companion window into a clean three-column
  layout, added a Diagnostics pane with live engine probes, and
  promoted "pin popover" from a dev hack to a real menu item.
- **UX-feedback pass**: flatter setup wizard layout, real per-step
  progress instead of a fake bar, drag handles on the popover
  footer, more forgiving cursor targets across the chrome.
- **Final-slide wizard verification panel** (#11): the four-layer
  leverage stack (MCP, primer, skill, plugin/hook) now reports
  per-client status on the last wizard slide so first-run users see
  exactly what was wired before they leave the flow.

### Changed (0.2.2 — full popover redesign matching macOS)

This release rebuilds the popover to match the SwiftUI sibling
1:1. Same data plumbing (Tauri commands, stores, contract types,
SSE flow), entirely new UI on top.

#### Bug fixes from 0.2.1 smoke test (WS0)
- **NVIDIA grey-window**: `WEBKIT_DISABLE_DMABUF_RENDERER=1` +
  `WEBKIT_DISABLE_COMPOSITING_MODE=1` are set from `main.rs` before
  Tauri boots. Forces a CPU paint that always works on NVIDIA
  proprietary drivers (was rendering a blank grey rect on Ubuntu
  24.04 GNOME/X11). Power users on Intel/AMD can override via env.
- **Tray-anchoring panic**: `popover::show` now wraps
  `move_window(TrayCenter)` in `catch_unwind` with a `TopRight`
  fallback. The positioner plugin panics — does not return Err —
  when its tray-bounds cache is empty (e.g. Open-Vaner from menu
  before any tray click). The `on_tray_event` hook in `tray.rs`
  populates the cache reliably; the catch_unwind is defense in depth.
- **Pause defer**: tray Pause item is disabled until the daemon
  ships `POST /engine/pause`.

#### Design system (WS1)
- `src/lib/tokens.css` is now the canonical 136-line file from the
  Vaner Control handoff package — single source of truth for colors,
  type scale, radii, shadows, animations. Light-mode overrides
  vendored but unwired (v0.3.0).
- Three OFL-licensed fonts (Space Grotesk variable, JetBrains Mono
  variable, Share Tech Mono regular) bundled at `static/fonts/`,
  declared via `@font-face` in `app.css`. The popover never depends
  on a remote font CDN.
- Brand SVG marks copied into `src/lib/assets/brand/`. Tray PNG
  rasterized from the official mono-white mark instead of the old
  placeholder.

#### Primitives (WS2)
15 Svelte components ported 1:1 from
`vaner-desktop-macos/vaner/Primitives/`: `VMark`, `VStateBadge`,
`VMenuBarIcon`, `V1Kicker` / `V1Headline` / `V1Body`,
`V1PrimaryButton` / `V1GhostButton` / `V1Slider`, `VContextCard`,
`QuietShell`, `SourceGlyph`, `Spinner`, `VMenuRow`, `VSectionLabel`.
Storyboard route at `/dev/primitives/` renders all 15 in isolation.

#### State machine (WS3)
- `src/lib/state/types.ts` mirrors the macOS `VanerState` enum as a
  12-variant discriminated union.
- `src/lib/state/reducer.ts` ports `StateReducer.swift` line-by-line
  (precedence: engine reachability → permissions → connected sources
  → indexing learning → 0.8.0 predictions → reactive prepared → fall
  through to watching).
- 12/12 Vitest fixtures passing (`pnpm test`).
- Five new stub stores feed the reducer; the reducer is permissive
  and falls through to safe states when daemon endpoints are missing.

#### Popover state views (WS4)
12 components under `src/lib/components/popover-states/`:
EngineMissing, NotInstalled, InstalledNotConnected, Learning,
Watching, Prepared, Attention, PermissionNeeded, NoActiveAgent,
ActivePredictions, Error, Idle. Each wraps QuietShell + composes
the new primitives. `src/routes/+page.svelte` is a thin switch
on `\$vanerState.kind`. Storyboard at `/dev/states/?kind=…` renders
any state in isolation.

#### Companion window (WS5)
Second Tauri window (820×560 min) opened on demand from the popover
footer's Details button or the tray's `Show Companion…` /
`Preferences…` items. Three-column layout (200px sidebar · center
pane · optional 260px right timeline when Prepared is active). Seven
panes: Prepared, Sources, Agents, Models, Engine, Preferences,
Diagnostics. Existing `MCPClientsPanel` + `EnginePanel` from
`/preferences` are reused inside the Agents and Engine panes; data
plumbing is unchanged.

#### Onboarding window (WS6)
Third Tauri window (720×540) opened automatically on first launch
when `setup_status.completed_at` is null. Welcome screen with a
breathing brand mark, the "one question" framing copy, and three
state-color-coded bullets explaining what Vaner does. \"Get started\"
navigates to the existing `/setup` wizard inside the same window;
the wizard's apply step detects the onboarding window and calls
`close_onboarding` on completion (instead of `goto('/')`).

#### Live reducer inputs (WS8)
- New `engine_status` Tauri command shells `vaner status --json`
  and projects `cockpit.reachable` + `scenarios_ready` onto the
  reducer's `EngineStatus` shape. Polled every 5s by
  `startEngineStatusPolling` in the popover's layout.
- New `detect_agents` Tauri command scans `/proc/*/comm` on Linux
  for known agent binaries (cursor, claude, code, code-insiders,
  zed, zeditor, continue) and returns `running_count` plus a static
  suggestion list. Polled every 8s.
- `setSourcesCount` overlays the source count derived from
  `setup_status.completed_at` so the reducer can distinguish
  `.installedNotConnected` from `.watching`.

#### Deferred to v0.2.3
- `/setup` wizard styling reskin — the wizard inherits the new
  tokens automatically but still uses its older inline styles.
- Real `prepared_list` + `blocked_sources` Tauri commands (daemon
  endpoints not all shipped yet; reducer falls through cleanly).
- TimelinePane right-column content (placeholder for now).
- Light-mode toggle (overrides vendored, listener pending).

### Fixed (0.2.1 — startup panic on real Linux hosts)
- **`sse_task::spawn`** used a bare `tokio::spawn` from inside the
  Tauri setup callback, which has no Tokio reactor in scope and
  panicked immediately: *"there is no reactor running, must be
  called from the context of a Tokio 1.x runtime"*. The app exited
  with code 101 before any UI showed up. Switched to
  `tauri::async_runtime::spawn` and updated `AppState.sse_handle`
  to the matching `tauri::async_runtime::JoinHandle` type. Bug had
  been present since the L4 scaffold but only surfaced when the
  v0.2.0 AppImage was smoke-tested on Ubuntu 24.04.

### Changed (0.2.0 — cross-platform: Linux + Windows in one repo)
- **Repo renamed `vaner-desktop-linux` → `vaner-desktop`.** GitHub
  redirects keep old URLs working; the apt repo at `apt.vaner.ai`
  is unaffected (CNAME survives the rename).
- **Crate / package rename.** `vaner-linux` → `vaner-desktop` in
  `Cargo.toml` and `package.json`; lib renamed to `vaner_desktop_lib`.
  Built binary on Linux is now `vaner-desktop` (not `vaner-linux`).
- **NSIS Windows bundle** target added to `tauri.conf.json` —
  `pnpm tauri build --bundles nsis` produces a per-user `.exe`
  installer. Windows 10 1809+ supported via WebView2.
- **`vaner_cli` shared module** replaces the duplicated POSIX
  `which vaner` shell-outs in `clients.rs` / `setup.rs`. Uses the
  cross-platform `which` crate so `.exe` resolution works on Windows.
- **`session.rs` AppIndicator nudge** is now `cfg`-gated to Linux —
  no-op on Windows / macOS.
- **Updater endpoint** points at the renamed repo's `latest.json`.

### Added (0.8.6 WS-DESK-LINUX — Simple-Mode setup)
- **`/setup` first-run wizard** (`src/routes/setup/+page.svelte`) — five-step Simple-Mode flow mirroring the macOS desktop. Welcome → work styles + priority → compute / cloud / background posture → recommendation review (with hardware-tier readout, "Why this bundle?" disclosure, runner-ups) → confirm + apply. Triggered by `setup.completed_at == null` from the root layout. Cloud-widening confirm dialog matches the macOS pattern: when `setup_apply` returns `widens_cloud_posture=true, written=false`, the wizard re-asks before re-calling with `confirm_cloud_widening=true`.
- **Engine and Telemetry preferences tabs** (`src/routes/preferences/EnginePanel.svelte`, `TelemetryPanel.svelte`) — previously stubbed "coming in 0.8.6". Engine tab carries a Simple/Advanced segmented control backed by localStorage (`vaner.pref.setupMode`); Simple shows the user's answers + bundle summary + "Why this bundle?" reasons + a "Re-run setup wizard" button; Advanced lists every bundle-managed knob read-only with a hint to use `vaner setup advanced` for direct TOML edits. Telemetry tab renders the HardwareProfile, the in-flight prediction count by source + ETA bucket, and the active bundle's tuning knobs.
- **Tauri `setup_*` commands** (`src-tauri/src/setup.rs`) — eight new commands: `setup_questions`, `setup_recommend`, `setup_apply`, `setup_status`, `policy_show`, `policy_refresh`, `hardware_profile`, `deep_run_defaults`. Each shells out to `vaner setup ... --json` (matches the 0.8.5 WS12-D `clients_*` pattern). `setup_apply` implements the cloud-widening pre-flight by pre-resolving the bundle id and comparing postures. `policy_refresh` and `deep_run_defaults` are best-effort / synthesised today and flip to HTTP probes when the engine 0.8.6 PR chain ships `POST /policy/refresh` and `GET /deep-run/defaults`.
- **`src/lib/stores/setup.ts`** — Svelte store mirroring the `clients.ts` shape. Exposes `setup` (snapshot), `setupMode` (Simple/Advanced UI toggle, persisted to localStorage), `loadStatus`, `loadQuestions`, `recommend`, `apply`, `loadHardware`, `loadPolicy`, `refresh`, `loadDeepRunDefaults`.
- **Hand-mirrored TypeScript types** (`src/lib/contract/setup-types.ts`) — `SetupAnswers`, `VanerPolicyBundle`, `SelectionResult`, `AppliedPolicy`, `HardwareProfile`, `SetupQuestion`, `SetupStatus`, `DeepRunDefaults`. Marked TODO until the `vaner-contract` ts-rs setup-type PR lands; the predev script's rsync excludes `setup-types.ts` so the hand-mirror survives a regen.
- **`scripts/regen-contract-bindings.mjs`** + `predev` / `prebuild` package.json hooks — regenerates ts-rs bindings from the local Vaner workspace and rsyncs them into `src/lib/contract/`. Honours `VANER_REPO=<path>` (defaults to `../Vaner`); skips silently when cargo / rsync / the workspace are unavailable.
- **First-run gating** (`src/routes/+layout.svelte`) — the GNOME app-indicator nudge in `FirstRunGuidance.svelte` now fires *after* the setup wizard completes, not before.

### Added
- **Preferences route + MCP Clients panel** (`src/routes/preferences/`) — first preferences UI in the Linux app. Tray menu *Preferences…* now opens this route (lands on the Clients tab). Lists every detected MCP client (Cursor, Claude Desktop, Claude Code, Cline, Continue, Zed, Windsurf, VS Code, Codex CLI, Roo) with Install / Reinstall / Remove per row + *Install for all* + drift banner with one-click *Update All*. Backed by the new Vaner CLI `vaner clients` (0.8.5 WS12-A); idempotent and backup-safe.
- **`clients` Tauri commands** (`src-tauri/src/clients.rs`) — first CLI shell-out from this app. New commands: `clients_detect`, `clients_install`, `clients_install_all`, `clients_uninstall`, `clients_doctor`. Each shells out to the bundled `vaner` binary (resolved via `$VANER_BIN` override or PATH) and parses the `--format json` output via serde.
- **`src/lib/stores/clients.ts`** — Svelte store mirroring the predictions store shape; exposes `clients`, `rescan`, `install`, `installAll`, `uninstall`. Auto-fetches drift report on every rescan.
- **Vaner 0.8.5 contract sync** (`src/lib/contract/types.ts`) — optional `readiness_label`, `eta_bucket`, `eta_bucket_label`, `adoptable`, `rank`, `ui_summary`, `suppression_reason`, `source_label` on `PredictedPrompt`, plus the `EtaBucket` type alias. Pre-0.8.5 daemons keep working — every new field is optional. Mirrors the additive changes in `vaner-contract` v0.2.0.
- **`src/lib/contract/card.ts`** — display helpers (`etaBucketLabel`, `readinessLabel`, `cardIsAdoptable`) that prefer server-supplied strings and fall back to canonical enum→label maps. Pinned glyphs (en-dash in `~10–20s`) match the daemon's `vaner.intent.readiness` source of truth and the Rust conformance fixtures.

## [0.1.0] - 2026-04-24

Initial release of the Vaner Linux desktop companion.

### Added (L5 — UX)
- System tray with colored Vaner brand mark (sourced from the
  `docs/handoff/vaner-desktop/brand/` package). Both left-click and
  right-click surface the menu (Linux convention).
- Tray menu: **Open Vaner** / Preferences… / Pause / Quit. Open Vaner
  pops the borderless popover window; the other items fire Tauri
  events the Svelte layer consumes.
- Popover lifecycle (`src-tauri/src/popover.rs`): show, hide, toggle,
  and auto-hide on `Focused(false)`. Position via
  `tauri-plugin-positioner::Position::TrayCenter` on X11; fallback
  on Wayland compositors that refuse fine-grained positioning.
- First-run AppIndicator-missing modal (`FirstRunGuidance.svelte`)
  triggered by `setup:appindicator-missing` from the Rust session
  probe. One-time, dismissable, copy-paste install command.
- Toast store + stack (`$lib/stores/toast.ts`,
  `$lib/components/ToastStack.svelte`). Adopt success/error, menu
  events, and future UI feedback route through it.

### Added (L6 — Release)
- **Dedicated Vaner release GPG key** signs every `.deb`. Key
  generation, GitHub Secrets upload, keyserver publication, and
  rotation policy are documented internally.
- `scripts/ci/sign-deb.sh` — embedded (`dpkg-sig`) + detached
  (`.deb.asc`) + signed `SHA256SUMS.asc`. Fingerprint sanity-check
  against the repo-committed pubkey before any signing happens.
- `scripts/ci/verify-deb.sh` — runs immediately after sign, red CI
  if any signature fails verification.
- `.github/workflows/release.yml` — fires on `v*.*.*` tags, builds
  with `tauri-action@v0`, signs, verifies, publishes the GitHub
  Release. Dry-run path via `workflow_dispatch` so we can exercise
  the pipeline without cutting tags. Hard-fails if the repo still
  holds the placeholder fingerprint or placeholder pubkey.
- `scripts/install.sh` — user-facing bootstrap. Downloads the
  signed `.deb` + detached sig from the GitHub Release, imports the
  committed pubkey, **pins** the fingerprint in its own source
  (install aborts on mismatch), `apt install`s.
- `scripts/release-key.asc` — placeholder; swap for the real
  armored export before tagging v0.1.0. Refusal logic in
  release.yml enforces this.

### Added (L7 — Ship gate)
- `Dockerfile.ship-gate` — reproducible Ubuntu 24.04 environment
  matching what end users run.
- `scripts/ship-gate.sh` — end-to-end automated test: signature
  verify, `apt install`, daemon boot, /predictions/active fetch,
  Adopt POST, handoff-file write, `/vaner:next` step-0 simulation,
  `apt purge` cleanup check.

### Changed
- Repo renamed from `vaner-linux` to `vaner-desktop-linux` to align
  with `vaner-desktop-macos`. GitHub redirects the old URL; internal
  references updated to the canonical name.

### Added (scope extension — pre-v0.1.0, not v0.1.1)
- **`.AppImage` bundle target.** Both `.deb` and `.AppImage` built,
  GPG-signed (detached `.asc`) and listed in `SHA256SUMS.asc` on every
  release.
- **Tauri auto-updater (`tauri-plugin-updater`).** Background check
  on app start emits `update:available` to the Svelte layer, which
  renders a calm top banner (`UpdateBanner.svelte`) with Install/
  Later actions and an inline progress bar. Updates verified against
  a dedicated minisign key (separate trust domain from the GPG `.deb`
  signing key) whose pubkey is embedded in `tauri.conf.json`.
- **Signed apt repository via GitHub Pages.**
  `scripts/ci/build-apt-repo.sh` runs `reprepro` against the release
  GPG key to produce a Debian dists/pool structure; the release
  workflow pushes the result to the `gh-pages` branch for GitHub
  Pages to serve. Users install via a standard
  `deb [signed-by=…] https://borgels.github.io/vaner-desktop-linux stable main`
  line; `apt upgrade` pulls new releases automatically. Landing
  page index.html + `.nojekyll` so Pages serves dotfile directories
  verbatim.
- `scripts/install.sh` default mode switched to `apt` (registers the
  repo + `apt install`); `VANER_MODE=deb` keeps the one-off .deb
  flow for ephemeral installs and CI.
- Release-key and apt-repo setup, minisign updater key generation,
  and the `apt.vaner.ai` custom-domain wiring are documented
  internally.

### Deferred to 0.1.1
- Preferences window content.
- Signed updater delta diffs (full-bundle replacement for now).

### Changed
- Apt repository is published at `https://apt.vaner.ai` (custom
  domain on the gh-pages branch via `CNAME`). Install URLs in
  `scripts/install.sh`, README, and the Vaner docs site all point
  at the new host. The fallback
  `https://borgels.github.io/vaner-desktop-linux` stays valid via
  the GitHub-managed rewrite.
- `pnpm-lock.yaml` committed. CI + release workflows both run
  `pnpm install --frozen-lockfile` now — deterministic installs
  across runs, and lock drift fails loudly in CI instead of
  silently reconciling.



### Added
- Initial scaffold: Tauri v2 + SvelteKit + shared `vaner-contract`
  Rust crate (pinned to `feat/vaner-contract-crate` on the Vaner
  monorepo until L1 lands on main).
- SvelteKit popover layout with readiness pills, confidence meter,
  and Adopt button (calm-primary-only; disabled when row isn't
  adoptable).
- Tauri commands:
  - `active_predictions()` → `Vec<PredictedPrompt>`
  - `adopt_prediction(predictionId)` → `String` intent
- Background SSE task (`sse_task`) bridging
  `vaner_contract::stream_prediction_events` to the WebView via
  `app.emit("predictions:snapshot", ...)`.
- Adopt-handoff flow:
  1. POST `/predictions/{id}/adopt` via the shared HTTP client.
  2. Drop the raw Resolution at `$XDG_STATE_HOME/vaner/pending-adopt.json`
     (handled off the Tauri async runtime via `spawn_blocking`).
  3. Paste-fallback clipboard: `predicted_response ?? prepared_briefing
     ?? intent`.
- First-run guidance scaffold: detects Wayland + GNOME without the
  AppIndicator extension and emits `setup:appindicator-missing` so
  the UI can nudge the user (UI modal itself TODO in L5).
- CI: `pnpm check` / `pnpm build` / `cargo fmt` / `cargo clippy` /
  `cargo check` on `ubuntu-22.04`.

### Deferred to L5
- Tray-icon lifecycle (click to toggle popover).
- Popover window positioning (`tauri-plugin-positioner`, tray-center
  on X11, top-right fallback on Wayland).
- First-run AppIndicator-missing modal UI.
- Theme toggle (light/dark via `.vd-light` class).

### Deferred to L6
- `.deb` bundle signing and release workflow.
- AppImage follow-up (v1.1).

### Known limitations (v0.1 scaffold)
- No tray icon yet; the app shows its main window directly.
- Icons in `src-tauri/icons/` are intentionally absent — add before
  release builds. `bundle.active = false` keeps CI happy.
- `@tauri-apps/plugin-clipboard-manager` / `plugin-dialog` listed as
  dependencies but the UI only uses clipboard (via the Rust side) in
  the adopt flow; dialog is there for L5's settings sheet.
