<!--
  Watching — connected, on, alive, nothing strong yet. Calm idle voice.
  Mirrors WatchingView.swift + handoff V1Watching.
-->
<script lang="ts">
  import QuietShell from "$lib/components/primitives/QuietShell.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import PopoverFooter from "$lib/components/PopoverFooter.svelte";
  import PopoverContextBlock from "./PopoverContextBlock.svelte";
  import PopoverQuickActions from "./PopoverQuickActions.svelte";
  import PredictionWorkSwitcher from "./PredictionWorkSwitcher.svelte";
  import type { PopoverRuntimeContext, WatchingSummary } from "$lib/state/types.js";

  type Props = { summary: WatchingSummary; silentHours: boolean; context: PopoverRuntimeContext };
  const { silentHours, context }: Props = $props();
</script>

<QuietShell markState="on" stateLabel={silentHours ? "Silent hours" : context.statusLabel}>
  {#if silentHours}
    <div class="silent">
      <V1Kicker text="Silent hours" color="var(--vd-purple)" />
      <div class="gap-6"></div>
      <V1Body muted text="Holding new prepared moments and surfacing them when silent hours end." />
    </div>
  {:else}
    <V1Kicker text="On" />
    <div class="gap-6"></div>
    <V1Headline text="Vaner is learning your current context" />
    <div class="gap-8"></div>
    <V1Body
      muted
      text={`Current client: ${context.clientLabel}. Workspace: ${context.workspaceLabel}. No strong next-step prediction yet.`}
    />
    <PopoverContextBlock {context} />
    <PredictionWorkSwitcher />
    <PopoverQuickActions cockpitPrimary />
  {/if}

  {#snippet footer()}
    <PopoverFooter health="on" healthLabel={`Last update ${context.lastUpdateLabel}`} detailsTab="prepared" />
  {/snippet}
</QuietShell>

<style>
  .gap-6 { height: 6px; }
  .gap-8 { height: 8px; }
  .silent {
    padding: 8px 12px;
    background: color-mix(in srgb, var(--vd-purple) 12%, transparent);
    border-radius: 8px;
    border: 0.5px solid color-mix(in srgb, var(--vd-purple) 30%, transparent);
  }
</style>
