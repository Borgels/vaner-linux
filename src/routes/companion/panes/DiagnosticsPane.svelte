<!--
  Diagnostics pane — bundle version, daemon health probe, log tail, and
  a one-shot Send-incident button (writes a redacted bundle to disk).
  v0.2.2: surface the version + probe; the log tail and incident bundle
  flow once the daemon ships POST /diagnostics/incident.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import { engineStatus } from "$lib/stores/engine-status.js";
  import { showToast } from "$lib/stores/toast.js";

  let appVersion = $state<string>("…");
  let actionDetail = $state<string | null>(null);

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = "unknown";
    }
  });

  async function runAction(name: "diagnostics_status" | "diagnostics_doctor" | "diagnostics_restart_engine" | "diagnostics_upgrade_engine") {
    actionDetail = "Working…";
    try {
      const result = await invoke<unknown>(name);
      actionDetail = typeof result === "string" ? result : JSON.stringify(result, null, 2);
      showToast("Diagnostics updated", "success", 2500);
    } catch (err) {
      actionDetail = err instanceof Error ? err.message : String(err);
      showToast("Diagnostics action failed", "attention", 3500);
    }
  }
</script>

<header class="hd">
  <div class="kicker-row">
    <V1Kicker text="Diagnostics" />
    <DocsLink path="/troubleshooting" />
  </div>
  <V1Headline text="Help me help you" size={22} />
  <V1Body
    muted
    text="Snapshot of your local install. Use this view if something feels off — paste it into a bug report and I can usually triage in one round."
  />
</header>

<section class="block">
  <VSectionLabel text="App" />
  <div class="kv">
    <span>Desktop app</span><span>{appVersion}</span>
  </div>
</section>

<section class="block">
  <VSectionLabel text="Engine" />
  <div class="kv">
    <span>Reachable</span><span>{$engineStatus.reachable ? "yes" : "no"}</span>
    <span>Files watched</span><span>{$engineStatus.filesWatched}</span>
    <span>Sources</span><span>{$engineStatus.sourcesCount}</span>
    <span>Uptime</span><span>{$engineStatus.uptimeMinutes}m</span>
  </div>
</section>

<section class="block">
  <VSectionLabel text="Actions" />
  <div class="actions">
    <V1PrimaryButton title="Run doctor" onclick={() => runAction("diagnostics_doctor")} />
    <V1GhostButton title="Check status" onclick={() => runAction("diagnostics_status")} />
    <V1GhostButton title="Restart engine" onclick={() => runAction("diagnostics_restart_engine")} />
    <V1GhostButton title="Update engine" onclick={() => runAction("diagnostics_upgrade_engine")} />
  </div>
  {#if actionDetail}
    <pre>{actionDetail}</pre>
  {/if}
</section>

<style>
  .hd { display: flex; flex-direction: column; gap: 6px; margin-bottom: 24px; }
  .kicker-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .block { margin-bottom: 22px; }
  .kv {
    margin-top: 10px;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    font-family: var(--vd-font);
    font-size: 12px;
  }
  .kv > span:nth-child(odd) { color: var(--vd-fg-3); text-transform: uppercase; letter-spacing: 0.05em; font-size: 10.5px; padding-top: 2px; }
  .kv > span:nth-child(even) { color: var(--vd-fg-1); font-family: var(--vd-font-mono); }
  .actions { display: flex; gap: 6px; margin-top: 10px; }
  pre {
    margin: 12px 0 0;
    padding: 10px;
    max-height: 260px;
    overflow: auto;
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: 8px;
    color: var(--vd-fg-2);
    font-size: 11px;
    white-space: pre-wrap;
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
</style>
