<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { setVanerPaused } from "$lib/stores/app-state.js";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";

  type Props = {
    cockpitPrimary?: boolean;
    tab?: string;
  };
  const { cockpitPrimary = true, tab = "prepared" }: Props = $props();

  function openCockpit() {
    invoke("open_external_url", { url: "http://127.0.0.1:8473" }).catch((e) => {
      console.warn("open cockpit failed", e);
    });
  }

  function openSettings() {
    invoke("open_companion", { tab: "preferences" }).catch((e) => {
      console.warn("open preferences failed", e);
    });
  }

  function openCompanion() {
    invoke("open_companion", { tab }).catch((e) => {
      console.warn("open companion failed", e);
    });
  }

  function pause() {
    void setVanerPaused(true);
  }
</script>

<div class="actions">
  {#if cockpitPrimary}
    <V1PrimaryButton title="Open Cockpit" onclick={openCockpit} />
    <V1GhostButton title="Settings" onclick={openSettings} />
  {:else}
    <V1PrimaryButton title="Inspect" tint="var(--vd-st-active)" onclick={openCompanion} />
    <V1GhostButton title="Open Cockpit" onclick={openCockpit} />
    <V1GhostButton title="Settings" onclick={openSettings} />
  {/if}
  <V1GhostButton title="Pause" onclick={pause} />
</div>

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 14px;
  }
</style>
