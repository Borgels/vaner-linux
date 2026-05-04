<!--
  Engine pane — engine binary lifecycle + runtime settings.

  Mirrors vaner-desktop-macos `Companion/EnginePane.swift`. Two cards:
    1. Install & updates — the engine binary's rollup state (running /
       stopped / error / not-installed), version, Restart / Update
       buttons, plus the systemd-user vaner-engine.service install +
       linger toggles (these are about *engine* lifecycle, not app
       UX, so they live here rather than Preferences).
    2. Runtime settings — Performance preset (Light/Balanced/Performance)
       + Advanced disclosure (CPU cap slider, max-cycle slider, idle-
       only toggle). Lifted from PreferencesPane.

  Preferences is now strictly app-level UX (silent hours, launch-at-
  login, memory). Anything that affects how the engine runs lives
  here.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import { engineStatus, boostEngineStatusPolling } from "$lib/stores/engine-status.js";
  import { showToast } from "$lib/stores/toast.js";
  import {
    applyComputePreset,
    classifyPreset,
    computeConfig,
    loadComputeConfig,
    type ComputePreset,
  } from "$lib/stores/compute-config.js";
  import {
    engineService,
    installEngineService,
    loadEngineServiceStatus,
    setEngineServiceLinger,
    uninstallEngineService,
    type ServiceState,
  } from "$lib/stores/engine-service.js";

  // Performance presets — same compute settings the macOS app ships
  // (so a user with both desktops gets matching behaviour), but the
  // user-facing copy talks about the trade-off rather than the
  // underlying knobs. Vaner is GPU-bound for inference and tries to
  // make the most of available VRAM; surfacing CPU/GPU caps to the
  // user is misleading. The presets encode "how aggressively the
  // background loop runs" — that's what users actually decide.
  const PRESETS: { id: ComputePreset; title: string; subtitle: string }[] = [
    {
      id: "light",
      title: "Light",
      subtitle:
        "Pauses when you're using your machine. Recommended on laptops or when battery matters.",
    },
    {
      id: "balanced",
      title: "Balanced",
      subtitle:
        "Mostly works in the background; backs off when you need the machine. Default.",
    },
    {
      id: "performance",
      title: "Performance",
      subtitle:
        "Works continuously, even while you're at the keyboard. Best on a desktop with headroom.",
    },
  ];

  let restarting = $state(false);
  let upgrading = $state(false);
  let computeBusy = $state(false);
  let serviceBusy = $state(false);
  let lingerBusy = $state(false);

  const activePreset = $derived(classifyPreset($computeConfig));

  type BringUpResult = {
    outcome: "already_running" | "started" | "failed" | "no_workspace";
    workspace: string | null;
    detail: string;
  };

  async function restartEngine() {
    if (restarting) return;
    restarting = true;
    boostEngineStatusPolling(15_000);
    try {
      const result = await invoke<BringUpResult>("bring_up_engine");
      showToast(
        result.outcome === "started"
          ? "Vaner engine started."
          : result.outcome === "already_running"
            ? "Vaner engine already running."
            : result.detail || "Could not start engine.",
        result.outcome === "failed" ? "attention" : "success",
        4000,
      );
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Could not restart: ${err}`,
        "attention",
        5000,
      );
    } finally {
      restarting = false;
    }
  }

  async function upgradeEngine() {
    if (upgrading) return;
    upgrading = true;
    try {
      const result = await invoke<string>("diagnostics_upgrade_engine");
      showToast(result || "Engine upgrade finished.", "success", 4000);
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Upgrade failed: ${err}`,
        "attention",
        5000,
      );
    } finally {
      upgrading = false;
    }
  }

  async function selectPreset(preset: ComputePreset) {
    if (computeBusy) return;
    computeBusy = true;
    try {
      await applyComputePreset(preset);
      showToast(`Performance set to ${preset}.`, "success", 2500);
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Could not apply preset: ${err}`,
        "attention",
        4000,
      );
    } finally {
      computeBusy = false;
    }
  }

  function describeServiceState(state: ServiceState | undefined): string {
    switch (state) {
      case "active":
        return "Running in the background. Survives desktop close + login restart.";
      case "enabled":
        return "Enabled but not currently running. systemd will start it on next login.";
      case "disabled":
        return "Unit installed but disabled. Toggle on to bring it up.";
      case "missing":
        return "Not installed. Toggle on to install + enable + start the unit.";
      case "unavailable":
        return "systemctl --user is unavailable on this session — the engine will only run while the desktop is open.";
      default:
        return "Checking…";
    }
  }

  async function onServiceToggleClick(target: boolean) {
    if (serviceBusy) return;
    serviceBusy = true;
    try {
      if (target) {
        const status = await installEngineService();
        showToast(
          status.state === "active"
            ? "Background engine service started."
            : "Background engine service installed.",
          "success",
          3500,
        );
      } else {
        await uninstallEngineService();
        showToast("Background engine service stopped + removed.", "success", 3000);
      }
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Service action failed: ${err}`,
        "attention",
        5000,
      );
      await loadEngineServiceStatus();
    } finally {
      serviceBusy = false;
    }
  }

  async function onLingerToggleClick(target: boolean) {
    if (lingerBusy) return;
    lingerBusy = true;
    try {
      const status = await setEngineServiceLinger(target);
      showToast(
        status.linger_enabled
          ? "Linger enabled — the engine will keep running across logout."
          : "Linger disabled — the engine will stop on logout.",
        "success",
        3500,
      );
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Linger toggle failed: ${err}`,
        "attention",
        5000,
      );
      await loadEngineServiceStatus();
    } finally {
      lingerBusy = false;
    }
  }

  // Engine binary status rollup — reduce the engineStatus store to a
  // single label the Install card branches on. Mirrors the macOS
  // EngineHealth.rollup enum.
  type StatusKind = "running" | "stopped" | "error" | "not_installed" | "checking";
  const status = $derived<StatusKind>(
    $engineStatus.cliMissing
      ? "not_installed"
      : $engineStatus.reachable
        ? "running"
        : ($engineStatus.uptimeMinutes === 0 && $engineStatus.filesWatched === 0)
          ? "stopped"
          : "error",
  );

  onMount(() => {
    void loadComputeConfig();
    void loadEngineServiceStatus();
  });
</script>

<header class="hd">
  <div class="kicker-row">
    <V1Kicker text="Engine" />
    <DocsLink path="/architecture" />
  </div>
  <V1Headline text="The local background process" size={22} />
  <V1Body muted text="Runs on your machine. Answers your agents." />
</header>

<!-- Install & updates card -->
<div class="card">
  <div class="card-head">
    <span class="rail" style="background: var(--vd-amber);"></span>
    <span>Install &amp; updates</span>
  </div>

  {#if status === "running"}
    <div class="status running">
      <span class="dot"></span>
      <strong>Running</strong>
      {#if $engineStatus.uptimeMinutes > 0}
        <span class="meta">· uptime {$engineStatus.uptimeMinutes}m</span>
      {/if}
    </div>
    <div class="actions">
      <V1GhostButton title={upgrading ? "Updating…" : "Update engine"} onclick={upgradeEngine} />
      <V1GhostButton title={restarting ? "Restarting…" : "Restart engine"} onclick={restartEngine} />
    </div>
  {:else if status === "stopped"}
    <div class="status stopped">
      <span class="dot"></span>
      <strong>Stopped</strong>
      <span class="meta">· daemon not responding on 127.0.0.1:8473</span>
    </div>
    <div class="actions">
      <V1PrimaryButton
        title={restarting ? "Starting…" : "Start engine"}
        onclick={restartEngine}
      />
    </div>
  {:else if status === "error"}
    <div class="status error">
      <span class="dot"></span>
      <strong>Can't reach Vaner</strong>
    </div>
    <div class="actions">
      <V1PrimaryButton
        title={restarting ? "Restarting…" : "Restart engine"}
        onclick={restartEngine}
      />
    </div>
  {:else if status === "not_installed"}
    <div class="status missing">
      <span class="dot"></span>
      <strong>Engine not installed</strong>
      <span class="meta">· `vaner` CLI not on PATH</span>
    </div>
    <V1Body
      muted
      text="Install Vaner via pipx or the install script at vaner.ai/install.sh."
    />
  {:else}
    <div class="status checking">
      <span class="dot"></span>
      <strong>Checking…</strong>
    </div>
  {/if}

  {#if $engineService}
    {@const svc = $engineService}
    {@const checked = svc.state === "active" || svc.state === "enabled"}
    {@const disabled = serviceBusy || svc.state === "unavailable"}
    {@const installed = svc.state !== "missing" && svc.state !== "unavailable"}
    <hr class="sep" />
    <label class="row" class:dim={disabled}>
      <input
        type="checkbox"
        {checked}
        {disabled}
        onchange={(e) => onServiceToggleClick((e.currentTarget as HTMLInputElement).checked)}
      />
      <span class="row-text">
        <span class="row-title">Run engine in the background (systemd)</span>
        <span class="row-detail">{describeServiceState(svc.state)}</span>
        {#if svc.workspace && installed}
          <span class="row-detail">
            Targeting <code>{svc.workspace}</code> · unit at <code>{svc.unit_path}</code>.
          </span>
        {/if}
      </span>
    </label>

    {#if installed}
      <label class="row" class:dim={lingerBusy}>
        <input
          type="checkbox"
          checked={svc.linger_enabled}
          disabled={lingerBusy}
          onchange={(e) => onLingerToggleClick((e.currentTarget as HTMLInputElement).checked)}
        />
        <span class="row-text">
          <span class="row-title">Keep the engine running after logout (linger)</span>
          <span class="row-detail">
            {#if svc.linger_enabled}
              The user manager survives logout, so Vaner keeps indexing across reboots and lock screens.
            {:else}
              The engine stops as soon as you log out. Toggle on if you want it indexing in the background even when you're away.
            {/if}
          </span>
          <span class="row-detail">
            Toggling triggers a graphical password prompt (polkit) to run
            <code>loginctl {svc.linger_enabled ? "disable-linger" : "enable-linger"}</code>.
          </span>
        </span>
      </label>
    {/if}
  {/if}
</div>

<!-- Runtime settings card -->
<div class="card">
  <div class="card-head">
    <span class="rail" style="background: var(--vd-purple);"></span>
    <span>Runtime settings</span>
  </div>
  <V1Body muted text="How hard Vaner works in the background." />
  {#if $computeConfig}
    <div class="presets">
      {#each PRESETS as preset (preset.id)}
        {@const selected = activePreset === preset.id}
        <button
          type="button"
          class="preset-row"
          class:selected
          disabled={computeBusy}
          onclick={() => selectPreset(preset.id)}
        >
          <span class="preset-title">{preset.title}</span>
          <span class="preset-sub">{preset.subtitle}</span>
          {#if selected}
            <span class="preset-badge">ACTIVE</span>
          {/if}
        </button>
      {/each}
    </div>
  {:else}
    <V1Body muted text="Loading runtime settings…" />
  {/if}
</div>

<style>
  .hd { display: flex; flex-direction: column; gap: 6px; margin-bottom: 24px; }
  .kicker-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
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
  .sep {
    border: none;
    border-top: 0.5px solid var(--vd-hair);
    margin: 6px 0;
  }

  .status {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 13px;
    color: var(--vd-fg-1);
    flex-wrap: wrap;
  }
  .status .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: 0 0 8px;
    align-self: center;
  }
  .status.running .dot { background: var(--vd-st-on); }
  .status.stopped .dot { background: var(--vd-fg-3); }
  .status.error .dot { background: var(--vd-st-attention); }
  .status.missing .dot { background: var(--vd-st-attention); }
  .status.checking .dot {
    background: var(--vd-fg-3);
    animation: pulse 1.4s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }
  .status .meta {
    font-size: 11.5px;
    color: var(--vd-fg-3);
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 4px;
  }

  .row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
  }
  .row.dim {
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

  .presets {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }
  .preset-row {
    display: grid;
    grid-template-columns: max-content 1fr max-content;
    grid-template-rows: auto auto;
    column-gap: 12px;
    row-gap: 2px;
    align-items: baseline;
    padding: 10px 12px;
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-chip);
    text-align: left;
    cursor: pointer;
    color: var(--vd-fg-1);
    font: inherit;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .preset-row:hover:not(:disabled) {
    background: var(--vd-bg-2);
  }
  .preset-row:disabled { opacity: 0.5; cursor: not-allowed; }
  .preset-row.selected {
    border-color: color-mix(in srgb, var(--vd-purple) 60%, transparent);
    background: color-mix(in srgb, var(--vd-purple) 8%, var(--vd-bg-1));
  }
  .preset-title {
    grid-column: 1;
    font-size: 13px;
    font-weight: 500;
  }
  .preset-sub {
    grid-column: 1 / span 3;
    grid-row: 2;
    font-size: 11.5px;
    color: var(--vd-fg-3);
    line-height: 1.4;
  }
  .preset-badge {
    grid-column: 3;
    grid-row: 1;
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.1em;
    color: var(--vd-purple);
    align-self: center;
  }
</style>
