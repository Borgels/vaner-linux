<!--
  Preferences pane — Linux equivalent of vaner-desktop-macos
  Companion/PreferencesPane.swift. Mirrors the macOS section structure:

    1. Active setup card
    2. Silent hours        ← toggle + from/to + weekdays-only
    3. Startup             ← Launch at login (Linux: XDG autostart)
    4. Memory              ← Export / Privacy / Clear

  Persona / tone sliders (chattiness / learnDepth / interrupt / voice)
  from seed.js were design speculation that never shipped on macOS.
  Removed.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import { setup, loadStatus, loadHardware, loadPolicy } from "$lib/stores/setup.js";
  import { silentHours } from "$lib/stores/silent-hours.js";
  import { showToast } from "$lib/stores/toast.js";

  // Friendly labels for the underlying enum strings the daemon emits.
  // `bundle.label` and `bundle.description` already carry human copy
  // when present, so we prefer those; the maps below are fallbacks for
  // older daemons that only return ids and tier slugs.
  const TIER_LABEL: Record<string, string> = {
    light: "Light",
    capable: "Capable",
    high_performance: "High performance",
    unknown: "—",
    other: "—",
  };
  const BUNDLE_LABEL: Record<string, string> = {
    local_light: "Light (local)",
    local_balanced: "Balanced (local)",
    local_heavy: "Performance (local)",
    hybrid_balanced: "Balanced (hybrid)",
    hybrid_advanced: "Advanced (hybrid)",
    cloud_balanced: "Balanced (cloud)",
    cloud_advanced: "Advanced (cloud)",
  };
  function bundleLabel(b: { id?: string; label?: string } | null | undefined): string {
    if (!b) return "—";
    if (b.label && b.label.trim().length > 0) return b.label;
    return BUNDLE_LABEL[b.id ?? ""] ?? (b.id ?? "—");
  }
  function tierLabel(t: string | undefined): string {
    if (!t) return "";
    return TIER_LABEL[t] ?? t;
  }
  // Performance presets, systemd-engine + linger toggles all moved
  // to the Engine pane (companion/panes/EnginePane.svelte). They are
  // engine-lifecycle settings, not app UX. Preferences is now strictly
  // about how the desktop *itself* behaves: silent hours, launch-at-
  // login (XDG autostart of vaner-desktop, not the engine), memory
  // export.

  // Silent-hours window — From / To, weekdays-only. Persisted to
  // localStorage for v0.2.2 alongside the simple `silentHours` toggle
  // store. Daemon-side enforcement lands when the engine ships its
  // silent-hours endpoint.
  type SilentWindow = {
    startHour: number;
    endHour: number;
    weekdaysOnly: boolean;
  };
  const SILENT_KEY = "vaner.pref.silentWindow";
  const defaultsWindow: SilentWindow = {
    startHour: 9,
    endHour: 12,
    weekdaysOnly: true,
  };
  let silentWin = $state<SilentWindow>(loadSilent());
  function loadSilent(): SilentWindow {
    try {
      const raw = localStorage.getItem(SILENT_KEY);
      if (!raw) return { ...defaultsWindow };
      return { ...defaultsWindow, ...JSON.parse(raw) };
    } catch {
      return { ...defaultsWindow };
    }
  }
  $effect(() => {
    try {
      localStorage.setItem(SILENT_KEY, JSON.stringify(silentWin));
    } catch {
      /* localStorage unavailable */
    }
  });
  const fmtHour = (h: number) => `${String(h).padStart(2, "0")}:00`;

  // Launch-at-login on Linux is XDG autostart — we drop a .desktop
  // file at ~/.config/autostart/vaner-desktop.desktop. Toggle persists
  // to localStorage; actual file write is a v0.2.3 follow-up wiring.
  let launchAtLogin = $state<boolean>(
    (() => {
      try {
        return localStorage.getItem("vaner.pref.launchAtLogin") === "true";
      } catch {
        return false;
      }
    })(),
  );
  $effect(() => {
    try {
      localStorage.setItem("vaner.pref.launchAtLogin", String(launchAtLogin));
    } catch {
      /* noop */
    }
  });

  let confirmClear = $state(false);
  async function exportMemory() {
    try {
      // No /docs/memory page exists yet; the privacy page is the
      // closest doc that explains where data lives until the
      // daemon ships an export endpoint.
      await invoke("open_external_url", { url: "https://vaner.ai/privacy" });
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err), "attention", 3500);
    }
  }
  async function openPrivacyDocs() {
    try {
      await invoke("open_external_url", { url: "https://vaner.ai/privacy" });
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err), "attention", 3500);
    }
  }
  function clearMemory() {
    confirmClear = false;
    showToast("Daemon-side memory wipe isn't wired yet.", "info", 3500);
  }

  onMount(() => {
    loadStatus();
    loadHardware();
    loadPolicy();
  });

  const bundle = $derived($setup.bundle);
  const tier = $derived($setup.hardware?.tier);
</script>

<header class="hd">
  <div class="kicker-row">
    <V1Kicker text="Preferences" />
    <DocsLink path="/policy-bundles" />
  </div>
  <V1Headline text="How Vaner sounds and when it speaks" size={22} />
</header>

<!-- Active bundle -->
<div class="card">
  <div class="card-head"><span class="rail" style="background: var(--vd-purple);"></span><span>Active setup</span></div>
  {#if bundle}
    <div class="bundle-row">
      <span class="bundle-name">{bundleLabel(bundle)}</span>
      {#if tier}
        <span class="tier">{tierLabel(tier)}</span>
      {/if}
    </div>
    <div class="actions">
      <V1GhostButton title="Re-run setup" onclick={() => (window.location.href = "/setup")} />
    </div>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>

<!-- Silent hours -->
<div class="card">
  <div class="card-head"><span class="rail" style="background: var(--vd-fg-3);"></span><span>Silent hours</span></div>
  <label class="row">
    <input type="checkbox" bind:checked={$silentHours} />
    <span class="row-text">
      <span class="row-title">Hold notifications during deep work</span>
    </span>
  </label>

  <div class="window" class:dim={!$silentHours}>
    <span class="window-label">From</span>
    <button class="step" onclick={() => silentWin.startHour = (silentWin.startHour + 23) % 24} disabled={!$silentHours}>−</button>
    <span class="window-time">{fmtHour(silentWin.startHour)}</span>
    <button class="step" onclick={() => silentWin.startHour = (silentWin.startHour + 1) % 24} disabled={!$silentHours}>+</button>

    <span class="window-label">to</span>
    <button class="step" onclick={() => silentWin.endHour = (silentWin.endHour + 23) % 24} disabled={!$silentHours}>−</button>
    <span class="window-time">{fmtHour(silentWin.endHour)}</span>
    <button class="step" onclick={() => silentWin.endHour = (silentWin.endHour + 1) % 24} disabled={!$silentHours}>+</button>
  </div>

  <label class="row toggle-only" class:dim={!$silentHours}>
    <input type="checkbox" bind:checked={silentWin.weekdaysOnly} disabled={!$silentHours} />
    <span class="row-title">Weekdays only</span>
  </label>

</div>

<!-- Startup -->
<div class="card">
  <div class="card-head"><span class="rail" style="background: var(--vd-st-active);"></span><span>Startup</span></div>
  <label class="row">
    <input type="checkbox" bind:checked={launchAtLogin} />
    <span class="row-text">
      <span class="row-title">Launch Vaner at login</span>
    </span>
  </label>
</div>

<!-- Memory -->
<div class="card">
  <div class="card-head"><span class="rail" style="background: var(--vd-st-attention);"></span><span>Memory</span></div>
  <div class="actions">
    <V1GhostButton title="Export…" onclick={exportMemory} />
    <V1GhostButton title="Privacy" onclick={openPrivacyDocs} />
    <V1GhostButton title="Clear…" destructive onclick={() => (confirmClear = true)} />
  </div>
  {#if confirmClear}
    <div class="confirm">
      <p>This deletes everything Vaner has learned about your work. Sources stay connected.</p>
      <div class="actions">
        <V1GhostButton title="Cancel" onclick={() => (confirmClear = false)} />
        <V1GhostButton title="Clear" destructive onclick={clearMemory} />
      </div>
    </div>
  {/if}
</div>

<style>
  .hd {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 24px;
  }
  .kicker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .card {
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-card);
    padding: 18px 20px;
    margin-bottom: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--vd-fg-1);
    margin-bottom: 4px;
  }
  .card-head .rail {
    width: 2px;
    height: 14px;
    border-radius: 1px;
    flex: 0 0 auto;
  }
  .hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--vd-fg-3);
    line-height: 1.45;
  }
  .muted {
    margin: 0;
    font-size: 12px;
    color: var(--vd-fg-2);
    line-height: 1.5;
  }
  .bundle-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .bundle-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--vd-fg-1);
    font-family: var(--vd-font-mono);
  }
  .tier {
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--vd-fg-3);
  }
  .actions {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
  }
  .row.toggle-only {
    align-items: center;
  }
  .row.dim,
  .window.dim {
    opacity: 0.45;
    pointer-events: none;
  }
  .row input[type="checkbox"] {
    margin-top: 3px;
    accent-color: var(--vd-purple);
  }
  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .row-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--vd-fg-1);
  }
  .row-detail {
    font-size: 11px;
    color: var(--vd-fg-3);
    line-height: 1.5;
  }
  .row-detail code {
    font-family: var(--vd-font-mono);
    font-size: 11px;
    color: var(--vd-fg-2);
  }
  .window {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 4px;
    background: rgba(255, 255, 255, 0.02);
    border: 0.5px solid var(--vd-hair);
    border-radius: var(--vd-r-chip);
  }
  .window-label {
    font-size: 11px;
    color: var(--vd-fg-3);
    padding: 0 6px;
  }
  .window-time {
    font-family: var(--vd-font-mono);
    font-size: 12px;
    color: var(--vd-fg-1);
    min-width: 44px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .step {
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    color: var(--vd-fg-2);
    border: 0.5px solid var(--vd-line);
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--vd-font);
    font-size: 14px;
    line-height: 1;
    padding: 0;
    transition: background 0.12s, color 0.12s;
  }
  .step:hover:not(:disabled) {
    background: var(--vd-bg-2);
    color: var(--vd-fg-1);
  }
  .step:disabled {
    cursor: not-allowed;
  }
  .confirm {
    margin-top: 6px;
    padding: 12px 14px;
    background: color-mix(in srgb, var(--vd-st-attention) 6%, transparent);
    border: 0.5px solid color-mix(in srgb, var(--vd-st-attention) 30%, transparent);
    border-radius: var(--vd-r-chip);
  }
  .confirm p {
    margin: 0 0 8px;
    font-size: 12px;
    color: var(--vd-fg-2);
    line-height: 1.5;
  }
</style>
