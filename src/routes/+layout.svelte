<script lang="ts">
  import "../app.css";
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { bootstrapAppStateListeners } from "$lib/stores/app-state.js";
  import { bootstrapUpdaterListeners } from "$lib/stores/updater.js";
  import { loadStatus } from "$lib/stores/setup.js";
  import { rescan as rescanClients } from "$lib/stores/clients.js";
  import { bootstrapDaemonAudit, disposeDaemonAudit } from "$lib/stores/daemon-audit.js";
  import {
    boostEngineStatusPolling,
    setSourcesCount,
    startEngineStatusPolling,
    stopEngineStatusPolling,
  } from "$lib/stores/engine-status.js";
  import {
    startAgentDetectorPolling,
    stopAgentDetectorPolling,
  } from "$lib/stores/agent-detector.js";
  import {
    startOllamaHealthListener,
    stopOllamaHealthListener,
  } from "$lib/stores/ollama-health.js";
  import { startFocusPolling, stopFocusPolling } from "$lib/stores/focus.js";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { showToast } from "$lib/stores/toast.js";

  type BringUpOutcome = "already_running" | "started" | "failed" | "no_workspace";
  type BringUpEvent = {
    outcome: BringUpOutcome;
    workspace: string | null;
    detail: string;
  };

  let bringUpUnlisten: UnlistenFn | null = null;

  let { children } = $props();

  onMount(async () => {
    // Only the main popover window owns the cross-window orchestration
    // (event listeners, first-launch onboarding kickoff, polling). The
    // companion + onboarding windows mount this same layout but skip
    // everything here.
    const label = getCurrentWebviewWindow().label;
    if (label !== "main") return;

    bootstrapAppStateListeners();
    bootstrapUpdaterListeners();
    void bootstrapDaemonAudit();

    // Listen for the startup auto-bring-up result. The Rust side
    // shells `vaner up --detach` itself when the cockpit is down; we
    // boost the engine_status poll to 500ms so the popover flips out
    // of .error within half a second of cockpit-up, and surface a
    // toast on failure so the user has something to act on.
    bringUpUnlisten = await listen<BringUpEvent>("engine:bring-up", (event) => {
      const result = event.payload;
      if (result.outcome === "started") {
        boostEngineStatusPolling(15_000);
      } else if (result.outcome === "failed") {
        boostEngineStatusPolling(15_000);
        showToast(
          result.detail || "Vaner could not start the engine.",
          "attention",
          5000,
        );
      }
      // already_running and no_workspace are silent — the popover
      // surfaces the right state on its own.
    });

    // Reducer-input polling. Both are idempotent; the popover survives
    // these returning errors (the stores keep their last value).
    startEngineStatusPolling();
    startAgentDetectorPolling();
    startFocusPolling();
    void startOllamaHealthListener();

    // Probe MCP-client integrations. The reducer routes the popover
    // to .notWiredToAnyClient when zero clients have Vaner registered,
    // so this needs to run on bootstrap (without it, `total = 0` and
    // the reducer falls through to engine state — wrong UX for fresh
    // installs).
    void rescanClients();
    // Re-probe whenever the popover regains focus. Common flow: user
    // clicks "Connect a client", lands on the Agents pane, installs
    // Vaner into Cursor, returns to the popover — without this, the
    // wired-count stays at 0 and the .notWiredToAnyClient panel
    // sticks around even though the install succeeded.
    const win = getCurrentWebviewWindow();
    void win.onFocusChanged(({ payload: focused }) => {
      if (focused) void rescanClients();
    });

    // First-run check: if no setup has completed, open the dedicated
    // onboarding window. (Re-run setup is also reachable from
    // Preferences, so this is a nudge, not a gate.)
    try {
      const status = await loadStatus();
      const completedAt = status?.setup?.completed_at;
      // Also overlay sourcesCount onto the engine-status store so the
      // reducer can tell .installedNotConnected from .watching. The
      // SetupStatus shape doesn't yet structurally enumerate sources,
      // so we use completed_at as a proxy: completed setup → ≥1 source.
      setSourcesCount(completedAt ? 1 : 0);

      if (!completedAt) {
        await invoke("open_onboarding").catch(() => {});
      }
    } catch {
      // Daemon / CLI unreachable: leave the popover on its default
      // state. The user can re-run setup later from the companion
      // window's Engine pane.
    }
  });

  onDestroy(() => {
    stopEngineStatusPolling();
    stopAgentDetectorPolling();
    stopFocusPolling();
    stopOllamaHealthListener();
    bringUpUnlisten?.();
    void disposeDaemonAudit();
  });
</script>

{@render children?.()}
