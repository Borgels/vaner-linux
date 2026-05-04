<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import {
    focusRoute,
    refreshFocusRoute,
    startFocusPolling,
    stopFocusPolling,
    updateFocusRoute,
    type FocusRouteState,
  } from "$lib/stores/focus.js";

  onMount(() => {
    startFocusPolling();
    void refreshFocusRoute();
  });
  onDestroy(() => stopFocusPolling());

  const route = $derived($focusRoute.effective_route);
  const workspace = $derived(route.workspace);
  const client = $derived(route.client);
  const backend = $derived(route.backend);
  const isManualWorkspace = $derived(route.workspace_policy === "pinned" || route.workspace_policy === "work_here");
  const isPreferredWorkspace = $derived(route.workspace_policy === "pinned" || workspace?.pinned === true);
  const workspacePath = $derived(workspace?.canonical_path ?? "");
  const selectedResource = $derived(route.resource_mode ?? "balanced");
  const selectedDevice = $derived(route.device ?? "auto");
  const selectedBackend = $derived(backend?.name || $focusRoute.hardware_options.runtimes[0]?.id || "");
  const selectedModel = $derived(backend?.model || $focusRoute.hardware_options.models[0]?.id || "");
  const workspaceName = $derived(workspace?.display_name ?? "No workspace");
  const clientName = $derived(client?.display_name ?? "No client yet");
  const hasWorkspace = $derived(Boolean(workspace));
  const heading = $derived(
    isManualWorkspace
      ? "Vaner is using a manual workspace"
      : hasWorkspace
        ? "Vaner follows your active workspace automatically"
        : "Vaner is waiting for an active workspace",
  );
  const subtext = $derived(
    isManualWorkspace
      ? `Pinned to ${workspaceName}. Return to Auto when you want Vaner to follow your active workspace again.`
      : hasWorkspace
        ? "No setup needed. Vaner uses supported AI clients to detect where you are working."
        : "Open a supported AI client, or choose a workspace manually.",
  );
  const debugInfo = $derived(JSON.stringify($focusRoute, null, 2));

  const fallbackResourceModes = [
    { id: "low_power", label: "Light" },
    { id: "balanced", label: "Balanced" },
    { id: "performance", label: "Fast" },
  ];

  const resourceModes = $derived(
    ($focusRoute.hardware_options.resource_modes.length
      ? $focusRoute.hardware_options.resource_modes
      : fallbackResourceModes
    ).map((mode) => ({ ...mode, label: performanceLabel(mode.id, mode.label) })),
  );

  const deviceOptions = $derived([
    { id: "auto", name: "Auto" },
    ...$focusRoute.hardware_options.devices.map((device) => ({
      id: device.id,
      name: device.name || device.id,
    })),
  ]);

  const backendOptions = $derived(
    $focusRoute.hardware_options.runtimes.length
      ? $focusRoute.hardware_options.runtimes
      : selectedBackend
        ? [{ id: selectedBackend, kind: selectedBackend }]
        : [],
  );

  const modelOptions = $derived(
    $focusRoute.hardware_options.models.length
      ? $focusRoute.hardware_options.models
      : selectedModel
        ? [{ id: selectedModel, name: selectedModel, runtime_id: selectedBackend }]
        : [],
  );

  function performanceLabel(id: string, fallback: string): string {
    switch (id) {
      case "low_power":
        return "Light";
      case "balanced":
        return "Balanced";
      case "performance":
        return "Fast";
      default:
        return fallback;
    }
  }

  function clientStatus(option: FocusRouteState["client_options"][number]): string {
    return option.running || option.integration_state === "ready" ? "Connected" : "Not configured";
  }

  async function chooseWorkspace(policy: "work_here" | "pinned") {
    const path = await invoke<string | null>("workspace_pick");
    if (path) await updateFocusRoute({ workspace_policy: policy, workspace_path: path });
  }

  async function copyDebugInfo() {
    try {
      await navigator.clipboard.writeText(debugInfo);
    } catch {
      // Clipboard permission can be unavailable inside some Tauri contexts.
    }
  }
</script>

<section class="workspace-pane">
  <header class="pane-head">
    <VSectionLabel text="Workspace" />
    <h1>{heading}</h1>
    <p>{subtext}</p>
  </header>

  <section class="current-card" aria-label="Current setting">
    <div class="setting-row">
      <span class="label">Current setting</span>
      <strong>{isManualWorkspace ? "Manual workspace" : "Auto"}</strong>
      <small>
        {isManualWorkspace
          ? "Vaner stays with the workspace you chose."
          : "Vaner follows your active AI client and workspace."}
      </small>
    </div>

    <div class="fact-grid">
      <div class="fact">
        <span>Current workspace</span>
        <strong>{workspaceName}</strong>
        {#if workspacePath}<small>{workspacePath}</small>{/if}
      </div>
      <div class="fact">
        <span>Current client</span>
        <strong>{clientName}</strong>
        <small>{client ? (client.running ? "Connected" : "Not configured") : "Waiting for a supported client"}</small>
      </div>
    </div>
  </section>

  <section class="actions" aria-label="Workspace actions">
    {#if isManualWorkspace}
      <button class="primary" onclick={() => updateFocusRoute({ workspace_policy: "auto", client_id: null })}>Return to Auto</button>
      <button onclick={() => chooseWorkspace("work_here")}>Choose another workspace</button>
      {#if workspacePath && !isPreferredWorkspace}
        <button class="quiet" onclick={() => updateFocusRoute({ workspace_policy: "pinned", workspace_path: workspacePath })}>
          Use this as preferred workspace
        </button>
      {/if}
    {:else}
      <button onclick={() => chooseWorkspace("work_here")}>Choose workspace manually</button>
      {#if !hasWorkspace}
        <button class="quiet" onclick={() => updateFocusRoute({ workspace_policy: "auto", client_id: null })}>Keep Auto</button>
      {/if}
    {/if}
  </section>

  <details class="section-details">
    <summary>Advanced settings</summary>
    <div class="advanced-grid">
      <section class="advanced-panel">
        <h2>Preferred workspace</h2>
        <p>Auto remains the normal setting. A preferred workspace is used only when you choose one.</p>
        <div class="button-list">
          <button class:active={route.workspace_policy === "auto"} onclick={() => updateFocusRoute({ workspace_policy: "auto" })}>Auto</button>
          {#each $focusRoute.workspace_options as option (option.id)}
            <button
              class:active={option.selected}
              disabled={!option.canonical_path}
              onclick={() => updateFocusRoute({ workspace_policy: "pinned", workspace_path: option.canonical_path ?? undefined })}
            >
              <span>{option.display_name}</span>
              {#if option.pinned}<small>Preferred workspace</small>{/if}
            </button>
          {/each}
        </div>
      </section>

      <section class="advanced-panel">
        <h2>AI client preference</h2>
        <p>Leave this on Auto unless you want Vaner to prefer one connected AI client.</p>
        <div class="button-list">
          <button class:active={!client?.preferred} onclick={() => updateFocusRoute({ client_id: null })}>Auto</button>
          {#if $focusRoute.client_options.length}
            {#each $focusRoute.client_options as option (option.id)}
              <button class:active={option.selected} onclick={() => updateFocusRoute({ client_id: option.id })}>
                <span>{option.display_name}</span>
                <small>{clientStatus(option)}</small>
              </button>
            {/each}
          {:else}
            <div class="empty">No connected AI clients yet.</div>
          {/if}
        </div>
      </section>

      <section class="advanced-panel form-panel">
        <h2>Performance and model</h2>
        <label>
          <span>Mode</span>
          <select value={selectedResource} onchange={(event) => updateFocusRoute({ resource_mode: event.currentTarget.value })}>
            {#each resourceModes as mode (mode.id)}
              <option value={mode.id}>{mode.label}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Device</span>
          <select value={selectedDevice} onchange={(event) => updateFocusRoute({ compute_device: event.currentTarget.value })}>
            {#each deviceOptions as device (device.id)}
              <option value={device.id}>{device.name}</option>
            {/each}
          </select>
          <small>Usually best. Vaner chooses the best available local device.</small>
        </label>
        <label>
          <span>Backend</span>
          <select value={selectedBackend} onchange={(event) => updateFocusRoute({ backend: { name: event.currentTarget.value } })}>
            {#each backendOptions as runtime (runtime.id)}
              <option value={runtime.id}>{runtime.kind || runtime.id}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Model</span>
          <select value={selectedModel} onchange={(event) => updateFocusRoute({ backend: { model: event.currentTarget.value } })}>
            {#each modelOptions as model (model.id)}
              <option value={model.id}>{model.name}</option>
            {/each}
          </select>
        </label>
      </section>
    </div>
  </details>

  <details class="section-details">
    <summary>Details for troubleshooting</summary>
    <div class="trouble-grid">
      <section>
        <h2>Detected clients</h2>
        <div class="detail-list">
          {#if $focusRoute.diagnostics.raw_detected_clients?.length}
            {#each $focusRoute.diagnostics.raw_detected_clients as detected}
              <div><span>{detected.display_name}</span><small>{detected.running ? "Connected" : "Not configured"}</small></div>
            {/each}
          {:else}
            <div><span>No clients detected</span><small>Not configured</small></div>
          {/if}
        </div>
      </section>
      <section>
        <h2>Selection reason</h2>
        <div class="detail-list">
          <div><span>{$focusRoute.explanation || route.selected_by}</span></div>
          {#each $focusRoute.diagnostics.why_not ?? [] as reason}
            <div><span>{reason.message ?? reason.reason_code}</span></div>
          {/each}
        </div>
        <button class="quiet copy" onclick={copyDebugInfo}>Copy debug info</button>
      </section>
    </div>
  </details>
</section>

<style>
  .workspace-pane {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 920px;
  }
  .pane-head {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  h1 {
    margin: 0;
    font-size: 26px;
    letter-spacing: 0;
    line-height: 1.15;
  }
  h2 {
    margin: 0;
    color: var(--vd-fg-1);
    font-size: 13px;
    letter-spacing: 0;
  }
  p {
    margin: 0;
    color: var(--vd-fg-2);
    line-height: 1.5;
    max-width: 700px;
  }
  .current-card,
  .advanced-panel,
  .section-details {
    border: 0.5px solid var(--vd-hair);
    border-radius: 8px;
    background: var(--vd-bg-1);
  }
  .current-card {
    padding: 18px;
    display: grid;
    gap: 18px;
  }
  .setting-row,
  .fact {
    min-width: 0;
    display: grid;
    gap: 5px;
  }
  .label,
  .fact span,
  small,
  summary,
  .empty {
    color: var(--vd-fg-3);
    font-size: 12px;
  }
  strong {
    color: var(--vd-fg-1);
    font-size: 16px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fact-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(0, 0.8fr);
    gap: 14px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button,
  select {
    min-height: 34px;
    border: 0.5px solid var(--vd-hair);
    border-radius: 6px;
    background: var(--vd-bg-1);
    color: var(--vd-fg-1);
    padding: 0 12px;
    font: inherit;
    font-size: 12px;
  }
  button {
    cursor: pointer;
  }
  button.primary {
    border-color: var(--vd-accent);
    background: var(--vd-bg-2);
  }
  button.quiet {
    color: var(--vd-fg-2);
  }
  button.active {
    border-color: var(--vd-accent);
    background: var(--vd-bg-2);
  }
  button:hover:not(:disabled),
  select:hover {
    background: var(--vd-bg-2);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .section-details {
    padding: 0;
    overflow: hidden;
  }
  .section-details > summary {
    cursor: pointer;
    list-style: none;
    padding: 14px 16px;
    color: var(--vd-fg-2);
  }
  .section-details > summary::-webkit-details-marker {
    display: none;
  }
  .section-details > summary::after {
    content: "Show";
    float: right;
    color: var(--vd-fg-3);
  }
  .section-details[open] > summary {
    border-bottom: 0.5px solid var(--vd-hair);
  }
  .section-details[open] > summary::after {
    content: "Hide";
  }
  .advanced-grid,
  .trouble-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
    padding: 16px;
  }
  .trouble-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .advanced-panel {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .button-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .button-list button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 100%;
  }
  .button-list button span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .form-panel label {
    display: grid;
    gap: 7px;
    color: var(--vd-fg-2);
    font-size: 12px;
  }
  .form-panel select {
    width: 100%;
    min-width: 0;
  }
  .detail-list {
    display: grid;
    gap: 8px;
    margin-top: 10px;
    color: var(--vd-fg-2);
    font-size: 12px;
  }
  .detail-list div {
    min-width: 0;
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .detail-list span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .copy {
    margin-top: 12px;
  }
  @media (max-width: 900px) {
    .fact-grid,
    .advanced-grid,
    .trouble-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
