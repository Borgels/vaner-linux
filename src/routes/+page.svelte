<!--
  Popover root. The reducer at $lib/stores/vaner-state.js produces the
  current `VanerState`; we switch on its `kind` and render the matching
  state component. The popover ships with a permanent ToastStack +
  UpdateBanner overlay, since both surface across every state.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { startPreparedWorkPolling } from "$lib/stores/prepared-work.js";
  import { startPredictionStream } from "$lib/stores/predictions.js";
  import { vanerState } from "$lib/stores/vaner-state.js";
  import NotWiredToAnyClient from "$lib/components/popover-states/NotWiredToAnyClient.svelte";
  import OllamaMissing from "$lib/components/popover-states/OllamaMissing.svelte";
  import EngineMissing from "$lib/components/popover-states/EngineMissing.svelte";
  import NotInstalled from "$lib/components/popover-states/NotInstalled.svelte";
  import InstalledNotConnected from "$lib/components/popover-states/InstalledNotConnected.svelte";
  import Learning from "$lib/components/popover-states/Learning.svelte";
  import Watching from "$lib/components/popover-states/Watching.svelte";
  import Prepared from "$lib/components/popover-states/Prepared.svelte";
  import PreparedWork from "$lib/components/popover-states/PreparedWork.svelte";
  import Attention from "$lib/components/popover-states/Attention.svelte";
  import PermissionNeeded from "$lib/components/popover-states/PermissionNeeded.svelte";
  import NoActiveAgent from "$lib/components/popover-states/NoActiveAgent.svelte";
  import ActivePredictions from "$lib/components/popover-states/ActivePredictions.svelte";
  import VanerError from "$lib/components/popover-states/Error.svelte";
  import Idle from "$lib/components/popover-states/Idle.svelte";
  import Paused from "$lib/components/popover-states/Paused.svelte";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import StrayDaemonsBanner from "$lib/components/StrayDaemonsBanner.svelte";
  import ToastStack from "$lib/components/ToastStack.svelte";
  import FirstRunGuidance from "$lib/components/FirstRunGuidance.svelte";

  onMount(() => {
    startPredictionStream();
    startPreparedWorkPolling();
  });
</script>

<div class="popover-root">
  <!-- Decorationless windows on Linux can't be moved by the compositor
       without an explicit drag region. The thin strip below is invisible
       but lets the user grab and reposition the popover. -->
  <div class="drag-handle" data-tauri-drag-region aria-hidden="true"></div>

  <UpdateBanner />
  <StrayDaemonsBanner />

  {#if $vanerState.kind === "notWiredToAnyClient"}
    <NotWiredToAnyClient detected={$vanerState.detected} />
  {:else if $vanerState.kind === "ollamaMissing"}
    <OllamaMissing installed={$vanerState.installed} detail={$vanerState.detail} />
  {:else if $vanerState.kind === "engineMissing"}
    <EngineMissing install={$vanerState.install} />
  {:else if $vanerState.kind === "notInstalled"}
    <NotInstalled />
  {:else if $vanerState.kind === "installedNotConnected"}
    <InstalledNotConnected />
  {:else if $vanerState.kind === "learning"}
    <Learning progress={$vanerState.progress} context={$vanerState.context} />
  {:else if $vanerState.kind === "watching"}
    <Watching summary={$vanerState.summary} silentHours={$vanerState.silentHours} context={$vanerState.context} />
  {:else if $vanerState.kind === "prepared"}
    <Prepared lead={$vanerState.lead} supporting={$vanerState.supporting} context={$vanerState.context} />
  {:else if $vanerState.kind === "preparedWork"}
    <PreparedWork cards={$vanerState.cards} context={$vanerState.context} />
  {:else if $vanerState.kind === "attention"}
    <Attention conflict={$vanerState.conflict} />
  {:else if $vanerState.kind === "permissionNeeded"}
    <PermissionNeeded sources={$vanerState.sources} />
  {:else if $vanerState.kind === "noActiveAgent"}
    <NoActiveAgent
      pendingCount={$vanerState.pendingCount}
      suggestedLaunch={$vanerState.suggestedLaunch}
    />
  {:else if $vanerState.kind === "activePredictions"}
    <ActivePredictions predictions={$vanerState.predictions} context={$vanerState.context} />
  {:else if $vanerState.kind === "error"}
    <VanerError engine={$vanerState.engine} />
  {:else if $vanerState.kind === "paused"}
    <Paused queued={$vanerState.queued} context={$vanerState.context} />
  {:else}
    <Idle />
  {/if}

  <FirstRunGuidance />
  <ToastStack />
</div>

<style>
  .popover-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--vd-bg-0);
    color: var(--vd-fg-1);
  }
  .drag-handle {
    flex: 0 0 8px;
    height: 8px;
    width: 100%;
    cursor: grab;
    -webkit-app-region: drag;
  }
  .drag-handle:active { cursor: grabbing; }
  .popover-root > :global(.quiet-shell) {
    flex: 1 1 auto;
    min-height: 0;
  }
</style>
