<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import QuietShell from "$lib/components/primitives/QuietShell.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import PopoverFooter from "$lib/components/PopoverFooter.svelte";
  import PopoverContextBlock from "./PopoverContextBlock.svelte";
  import PopoverQuickActions from "./PopoverQuickActions.svelte";
  import { showToast } from "$lib/stores/toast.js";
  import { canRunPreparedWorkAction, dispatchPreparedWorkAction } from "$lib/prepared-work-actions.js";
  import type { PreparedWorkAction, PreparedWorkCard } from "$lib/contract/types.js";
  import type { PopoverRuntimeContext } from "$lib/state/types.js";

  type Props = { cards: PreparedWorkCard[]; context: PopoverRuntimeContext };
  const { cards, context }: Props = $props();
  const visibleCards = $derived(cards.slice(0, 3));
  const readyTitle = $derived(
    context.preparedReady === 1
      ? "1 prepared item ready"
      : `${Math.max(context.preparedReady, cards.length)} prepared items ready`,
  );

  function typeLabel(card: PreparedWorkCard): string {
    switch (card.kind) {
      case "review": return "review note";
      case "bug": return "bug hypothesis";
      case "docs": return "docs drift";
      case "diff": return "virtual diff";
      case "brief": return "research brief";
      case "draft": return "suggested change";
      default: return card.badge || "prepared work";
    }
  }

  function statusLabel(card: PreparedWorkCard): string {
    const freshness = (card.freshness_state ?? "").toLowerCase();
    if (freshness === "stale" || freshness === "possibly_stale") return "needs review";
    if (card.confidence_label.toLowerCase().includes("low")) return "partial";
    return "ready";
  }

  async function run(card: PreparedWorkCard, action: PreparedWorkAction) {
    if (!canRunPreparedWorkAction(card, action)) return;
    try {
      const result = await dispatchPreparedWorkAction(invoke, card, action);
      if (result.kind === "adopt") {
        showToast(`Prediction adopted — ${result.message}.`, "success", 4000);
      } else if (result.kind === "unsupported") {
        showToast(result.message, "attention", 4000);
      } else {
        showToast(`${action.label} complete.`, "success", 3000);
      }
    } catch (err) {
      const msg = typeof err === "string" ? err : `Couldn't ${action.label.toLowerCase()}.`;
      showToast(msg, "attention", 5000);
    }
  }
</script>

<QuietShell markState="active" stateLabel={`Prepared work · ${cards.length}`} stateLabelTint="var(--vd-st-active)">
  <V1Kicker text="Prepared work" color="var(--vd-st-active)" />
  <div class="gap-6"></div>
  <V1Headline text={readyTitle} />

  <div class="rows">
    {#each visibleCards as card, i (card.id)}
      <article class="row">
        <div class="top">
          <span class="badge">{typeLabel(card)}</span>
          <div class="title">{card.title}</div>
        </div>
        <div class="summary">{card.summary}</div>
        {#if card.why_prepared}
          <div class="why">{card.why_prepared}</div>
        {/if}
        <div class="meta">
          <span>{statusLabel(card)}</span>
          <span>{card.confidence_label}</span>
          <span>{card.freshness_label}</span>
          <span>{card.target_label}</span>
          {#if card.evidence_count > 0}
            <span>{card.evidence_count} evidence</span>
          {/if}
        </div>
        {#if card.action_note}
          <div class:warn={card.freshness_state === "possibly_stale" || card.freshness_state === "stale"} class="note">
            {card.action_note}
          </div>
        {/if}
        <div class="actions">
          {#if card.primary_action && canRunPreparedWorkAction(card, card.primary_action)}
            {#if i === 0}
              <V1PrimaryButton title={card.primary_action.label} tint="var(--vd-st-active)" onclick={() => run(card, card.primary_action!)} />
            {:else}
              <V1GhostButton title={card.primary_action.label} onclick={() => run(card, card.primary_action!)} />
            {/if}
          {/if}
          {#each card.secondary_actions.filter((a) => canRunPreparedWorkAction(card, a)).slice(0, 3) as action (`${card.id}-${action.kind}-${action.label}`)}
            <V1GhostButton title={action.label} onclick={() => run(card, action)} />
          {/each}
        </div>
      </article>
    {/each}
  </div>

  <PopoverContextBlock {context} compact />
  <PopoverQuickActions cockpitPrimary={false} tab="prepared" />

  {#snippet footer()}
    <PopoverFooter health="active" healthLabel={`Last update ${context.lastUpdateLabel}`} detailsTab="prepared" />
  {/snippet}
</QuietShell>

<style>
  .gap-6 { height: 6px; }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 12px;
  }
  .row {
    padding: 10px 12px;
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-card);
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .badge {
    flex: 0 0 auto;
    padding: 2px 7px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--vd-st-active) 18%, transparent);
    color: var(--vd-st-active);
    font-family: var(--vd-font);
    font-size: 10px;
    font-weight: 600;
  }
  .title {
    min-width: 0;
    font-family: var(--vd-font);
    font-size: 13px;
    font-weight: 500;
    color: var(--vd-fg-1);
    line-height: 1.3;
  }
  .summary {
    margin-top: 6px;
    font-family: var(--vd-font);
    font-size: 11.5px;
    line-height: 1.38;
    color: var(--vd-fg-3);
  }
  .why,
  .note {
    margin-top: 6px;
    font-family: var(--vd-font);
    font-size: 10.8px;
    line-height: 1.35;
    color: var(--vd-fg-4);
  }
  .note.warn {
    color: var(--vd-st-warn);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    margin-top: 8px;
    color: var(--vd-fg-4);
    font-family: var(--vd-font-mono);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
</style>
