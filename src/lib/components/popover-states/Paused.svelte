<!--
  Paused — explicit "I asked Vaner to be quiet" state. Tray menu's
  Pause item flipped the isPaused store, the reducer routed us here.
  Resume button flips it back; the popover then re-resolves to
  whatever the underlying state would have been.
-->
<script lang="ts">
  import QuietShell from "$lib/components/primitives/QuietShell.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import PopoverFooter from "$lib/components/PopoverFooter.svelte";
  import PopoverContextBlock from "./PopoverContextBlock.svelte";
  import { setVanerPaused } from "$lib/stores/app-state.js";
  import type { PopoverRuntimeContext } from "$lib/state/types.js";

  type Props = { queued: number; context: PopoverRuntimeContext };
  const { queued, context }: Props = $props();

  function resume() {
    void setVanerPaused(false);
  }
</script>

<QuietShell markState="idle" stateLabel="Paused" stateLabelTint="var(--vd-fg-3)">
  <V1Kicker text="Paused" />
  <div class="gap-6"></div>
  <V1Headline text="Vaner is holding off." />
  <div class="gap-8"></div>
  {#if queued > 0}
    <V1Body
      muted
      text={`${queued} ${queued === 1 ? "moment is" : "moments are"} queued. They'll surface as soon as you resume.`}
    />
  {:else}
    <V1Body muted text="Nothing pressing was queued while paused. Resume any time." />
  {/if}

  <div class="actions">
    <V1PrimaryButton title="Resume" onclick={resume} />
  </div>

  <PopoverContextBlock {context} compact />

  {#snippet footer()}
    <PopoverFooter health="idle" healthLabel={queued > 0 ? `${queued} queued` : "Paused"} detailsTab="prepared" />
  {/snippet}
</QuietShell>

<style>
  .gap-6 { height: 6px; }
  .gap-8 { height: 8px; }
  .actions {
    display: flex;
    gap: 6px;
    margin-top: 14px;
  }
</style>
