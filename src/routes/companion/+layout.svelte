<!--
  Companion 3-column shell: 200px sidebar · center pane · optional 260px
  timeline (only when Prepared is the active pane). Route hash drives
  pane selection so back/forward navigation feels native.

  Mirrors vaner-desktop-macos/vaner/Companion/CompanionWindow.swift.
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import VMark from "$lib/components/primitives/VMark.svelte";
  import VMenuRow from "$lib/components/primitives/VMenuRow.svelte";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import NavGlyph from "$lib/components/primitives/NavGlyph.svelte";
  import ToastStack from "$lib/components/ToastStack.svelte";
  import {
    startEngineStatusPolling,
    stopEngineStatusPolling,
  } from "$lib/stores/engine-status.js";
  import {
    startOllamaHealthListener,
    stopOllamaHealthListener,
  } from "$lib/stores/ollama-health.js";
  import {
    startPreparedWorkPolling,
    stopPreparedWorkPolling,
  } from "$lib/stores/prepared-work.js";
  import {
    startPredictionStream,
    stopPredictionStream,
  } from "$lib/stores/predictions.js";

  let { children } = $props();

  type Tab = {
    id: string;
    label: string;
    showsTimeline?: boolean;
  };
  // v0.2.3: Sources tab dropped. Vaner is an MCP server — agents call
  // Vaner, not the other way around. The "Connect a source" framing was
  // misleading; "Agents" already exposes the MCP-client install flow,
  // which is the actual integration surface.
  const tabs: Tab[] = [
    { id: "prepared", label: "Prepared", showsTimeline: true },
    { id: "focus", label: "Workspace" },
    { id: "agents", label: "Agents" },
    { id: "models", label: "Models" },
    { id: "engine", label: "Engine" },
    { id: "preferences", label: "Preferences" },
    { id: "diagnostics", label: "Diagnostics" },
  ];

  // Tab driven by ?tab= query string. Default to prepared.
  const active = $derived(($page.url.searchParams.get("tab") ?? "prepared").toLowerCase());

  function navigate(id: string) {
    const url = new URL($page.url);
    url.searchParams.set("tab", id);
    goto(`?${url.searchParams.toString()}`, { keepFocus: true, noScroll: true });
  }

  let unlisten: UnlistenFn | null = null;
  onMount(async () => {
    // Tauri webviews are isolated JS contexts — the popover's poll
    // loop doesn't share state with the companion. Without this the
    // companion's engineStatus store sits at its initial stub
    // (reachable=true) forever, and the Engine pane disagrees with
    // the popover about whether the engine is up. Same store, same
    // command, just one poller per window.
    startEngineStatusPolling();
    void startOllamaHealthListener();
    void startPredictionStream();
    startPreparedWorkPolling(8);
    // The Rust side fires `companion:navigate` when the user reopens
    // the window from the tray with a different requested pane. Sync
    // our query string when that happens.
    unlisten = await listen<string>("companion:navigate", (e) => {
      const tab = (e.payload ?? "prepared").toString().toLowerCase();
      if (tab !== active) navigate(tab);
    });
  });
  onDestroy(() => {
    unlisten?.();
    stopEngineStatusPolling();
    stopOllamaHealthListener();
    stopPreparedWorkPolling();
    void stopPredictionStream();
  });

  const showTimeline = $derived(tabs.find((t) => t.id === active)?.showsTimeline ?? false);
</script>

<svelte:head>
  <title>Vaner</title>
</svelte:head>

<div class="companion">
  <!-- Companion window has decorations:false; this strip lets the user
       drag the whole window from the empty top edge. -->
  <div class="drag-handle" data-tauri-drag-region aria-hidden="true"></div>

  <aside class="sidebar">
    <header class="sidebar__brand">
      <VMark size={22} />
      <span class="sidebar__wordmark">vaner<span class="sidebar__cursor">_</span></span>
    </header>

    <div class="sidebar__nav">
      <VSectionLabel text="Today" />
      {#each tabs as t (t.id)}
        <VMenuRow
          title={t.label}
          selected={active === t.id}
          onclick={() => navigate(t.id)}
        >
          {#snippet icon()}
            <NavGlyph kind={t.id} size={14} dim={active !== t.id} />
          {/snippet}
        </VMenuRow>
      {/each}
    </div>

    <div class="sidebar__foot">
      <VMenuRow
        title="Close window"
        onclick={() => invoke("window_hide", { label: "companion" })}
      />
      <VMenuRow title="Quit Vaner" onclick={() => invoke("app_quit")} />
    </div>
  </aside>

  <div class="hair"></div>

  <main class="center vd-scroll">
    {@render children()}
  </main>

  {#if showTimeline}
    <div class="hair"></div>
    <aside class="timeline vd-scroll">
      <VSectionLabel text="Today's activity" />
      <div class="timeline__placeholder">
        Timeline lands when WS8 wires the daemon's <code>/events/stream</code>
        history feed. For now this column is reserved at 260px.
      </div>
    </aside>
  {/if}
</div>

<ToastStack />

<style>
  .companion {
    display: flex;
    height: 100vh;
    background: var(--vd-bg-0);
    color: var(--vd-fg-1);
    font-family: var(--vd-font);
    position: relative;
  }
  .companion > .drag-handle {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 12px;
    cursor: grab;
    -webkit-app-region: drag;
    z-index: 50;
  }
  .companion > .drag-handle:active { cursor: grabbing; }
  .sidebar {
    flex: 0 0 200px;
    display: flex;
    flex-direction: column;
    background: var(--vd-bg-1);
    padding: 14px 12px 10px;
    gap: 10px;
  }
  .sidebar__brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 4px 8px 8px;
  }
  .sidebar__wordmark {
    font-family: var(--vd-font-term);
    font-size: 14px;
    color: var(--vd-fg-1);
    letter-spacing: 0.02em;
  }
  .sidebar__cursor {
    color: var(--vd-amber);
    animation: vd-blink 1.6s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }
  .sidebar__nav {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sidebar__foot {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-top: 6px;
    border-top: 0.5px solid var(--vd-hair);
  }
  .hair {
    flex: 0 0 0.5px;
    background: var(--vd-hair);
  }
  .center {
    flex: 1 1 auto;
    overflow-y: auto;
    padding: 22px 28px 32px;
  }
  .timeline {
    flex: 0 0 260px;
    background: var(--vd-bg-1);
    padding: 18px 18px 22px;
    overflow-y: auto;
  }
  .timeline__placeholder {
    margin-top: 10px;
    font-size: 12px;
    color: var(--vd-fg-3);
    line-height: 1.55;
  }
  .timeline__placeholder code {
    font-family: var(--vd-font-mono);
    font-size: 11px;
    color: var(--vd-fg-2);
  }
</style>
