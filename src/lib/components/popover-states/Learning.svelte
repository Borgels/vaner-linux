<!--
  Learning — Vaner is indexing, nothing prepared yet. The mark satellite
  breathes. Mirrors LearningView.swift + handoff QuietPopoverLearning.
-->
<script lang="ts">
  import QuietShell from "$lib/components/primitives/QuietShell.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import SourceGlyph from "$lib/components/primitives/SourceGlyph.svelte";
  import PopoverFooter from "$lib/components/PopoverFooter.svelte";
  import PopoverContextBlock from "./PopoverContextBlock.svelte";
  import PopoverQuickActions from "./PopoverQuickActions.svelte";
  import PredictionWorkSwitcher from "./PredictionWorkSwitcher.svelte";
  import type { LearningProgress, PopoverRuntimeContext } from "$lib/state/types.js";

  type Props = { progress: LearningProgress; context: PopoverRuntimeContext };
  const { progress, context }: Props = $props();
</script>

<QuietShell markState="learning" breathingMark stateLabel={`Learning · ${progress.uptimeMinutes}m`}>
  <V1Kicker text="Learning" />
  <div class="gap-6"></div>
  <V1Headline text="Vaner is learning your current context" />
  <div class="gap-8"></div>
  <V1Body
    muted
    text={`Current client: ${context.clientLabel}. Workspace: ${context.workspaceLabel}. Strong predictions usually appear in ${progress.etaMinutes != null ? `~${progress.etaMinutes}m` : "a few minutes"}.`}
  />

  {#if progress.currentlyReading.length > 0}
    <div class="reading">
      {#each progress.currentlyReading.slice(0, 4) as item (item.title)}
        <div class="reading-row">
          <SourceGlyph kind={item.source} size={14} dim />
          <div class="reading-body">
            <div class="reading-title">{item.title}</div>
            <div class="reading-since">since {item.since}</div>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <PopoverContextBlock {context} />
  <PredictionWorkSwitcher />
  <PopoverQuickActions cockpitPrimary />

  {#snippet footer()}
    <PopoverFooter health="learning" healthLabel={`Last update ${context.lastUpdateLabel}`} detailsTab="prepared" />
  {/snippet}
</QuietShell>

<style>
  .gap-6 { height: 6px; }
  .gap-8 { height: 8px; }
  .reading {
    margin-top: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .reading-row {
    display: flex;
    gap: 10px;
    padding: 8px 10px;
    background: rgba(255, 255, 255, 0.02);
    border: 0.5px solid var(--vd-hair);
    border-radius: 7px;
  }
  .reading-body { flex: 1 1 auto; min-width: 0; }
  .reading-title {
    font-family: var(--vd-font);
    font-size: 12px;
    font-weight: 500;
    color: var(--vd-fg-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .reading-since {
    font-family: var(--vd-font-mono);
    font-size: 10.5px;
    color: var(--vd-fg-3);
  }
</style>
