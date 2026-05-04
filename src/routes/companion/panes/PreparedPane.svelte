<!--
  Prepared pane — companion-scale view of the daemon's unified
  `/prepared-work` surface. The older reactive `prepared` store is still
  present for legacy popover states, but this pane should show what Vaner
  can actually hand to an agent now.
-->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import Spinner from "$lib/components/primitives/Spinner.svelte";
  import { canRunPreparedWorkAction, dispatchPreparedWorkAction } from "$lib/prepared-work-actions.js";
  import type { PreparedWorkAction, PreparedWorkCard } from "$lib/contract/types.js";
  import { preparedWorkState, refreshPreparedWork } from "$lib/stores/prepared-work.js";
  import { showToast } from "$lib/stores/toast.js";

  const cards = $derived($preparedWorkState.cards);
  const lead = $derived(cards[0] ?? null);
  const supporting = $derived(cards.slice(1));

  function typeLabel(card: PreparedWorkCard): string {
    switch (card.kind) {
      case "review": return "review note";
      case "bug": return "bug hypothesis";
      case "docs": return "docs drift";
      case "diff": return "virtual diff";
      case "brief": return "research brief";
      case "draft": return "suggested change";
      case "prediction": return "prediction";
      default: return card.badge || "prepared work";
    }
  }

  function actions(card: PreparedWorkCard): PreparedWorkAction[] {
    const out: PreparedWorkAction[] = [];
    if (canRunPreparedWorkAction(card, card.primary_action)) out.push(card.primary_action);
    for (const action of card.secondary_actions) {
      if (canRunPreparedWorkAction(card, action)) out.push(action);
    }
    return out;
  }

  function shouldRefreshAfter(action: PreparedWorkAction): boolean {
    return action.kind === "adopt" || action.kind === "dismiss" || action.kind === "feedback" || action.kind === "export";
  }

  async function run(card: PreparedWorkCard, action: PreparedWorkAction) {
    try {
      const result = await dispatchPreparedWorkAction(invoke, card, action);
      if (result.kind === "adopt") {
        showToast(`Prediction adopted — ${result.message}. Your agent's next prompt will use this package.`, "success", 5000);
      } else if (result.kind === "unsupported") {
        showToast(result.message, "attention", 4000);
      } else {
        showToast(`${action.label} complete.`, "success", 3000);
      }
      if (shouldRefreshAfter(action)) void refreshPreparedWork(8);
    } catch (err) {
      showToast(typeof err === "string" ? err : `Couldn't ${action.label.toLowerCase()}.`, "attention", 5000);
    }
  }
</script>

<header class="hd">
  <div class="kicker-row">
    <V1Kicker text="Prepared" />
    <DocsLink path="/prepared-work" />
  </div>
  <V1Headline text="What Vaner has ready for you" size={22} />
  <V1Body
    muted
    text="The lead card is ranked first by the daemon. Supporting cards are ready or nearly ready, with confidence and freshness shown inline."
  />
</header>

{#if $preparedWorkState.loading && cards.length === 0}
  <section class="status-card">
    <Spinner size={16} />
    <span>Loading prepared work…</span>
  </section>
{:else if $preparedWorkState.error}
  <section class="status-card error" role="alert">
    <span>{$preparedWorkState.error}</span>
    <V1GhostButton title="Retry" onclick={() => void refreshPreparedWork(8)} />
  </section>
{:else if lead}
  <section class="lead-section">
    <VSectionLabel text="Lead" />
    {@render PreparedCard(lead, true, run)}
  </section>

  {#if supporting.length > 0}
    <section class="sup">
      <VSectionLabel text={`Also prepared · ${supporting.length}`} />
      <div class="grid">
        {#each supporting as card (card.id)}
          {@render PreparedCard(card, false, run)}
        {/each}
      </div>
    </section>
  {/if}
{:else}
  <section class="empty">
    <V1Body
      muted
      text="No prepared work is ready right now. Vaner will populate this pane as predictions or concrete work products mature."
    />
  </section>
{/if}

{#snippet PreparedCard(card: PreparedWorkCard, lead = false, onRun: (card: PreparedWorkCard, action: PreparedWorkAction) => Promise<void>)}
  <article class:lead class="card">
    <div class="card-top">
      <span class="badge">{typeLabel(card)}</span>
      <span class="freshness">{card.freshness_label}</span>
    </div>
    <h3>{card.title}</h3>
    <p class="summary">{card.summary}</p>
    {#if card.why_prepared}
      <p class="why">{card.why_prepared}</p>
    {/if}
    <div class="meta">
      <span>{card.badge}</span>
      <span>{card.confidence_label}</span>
      <span>{card.target_label}</span>
      {#if card.evidence_count > 0}
        <span>{card.evidence_count} evidence</span>
      {/if}
    </div>
    {#if card.action_note}
      <p class:warn={card.freshness_state === "possibly_stale" || card.freshness_state === "stale"} class="note">
        {card.action_note}
      </p>
    {/if}
    {#if actions(card).length > 0}
      <div class="actions">
        {#each actions(card).slice(0, lead ? 4 : 2) as action, i (`${card.id}-${action.kind}-${action.label}`)}
          {#if lead && i === 0}
            <V1PrimaryButton title={action.label} tint="var(--vd-st-active)" onclick={() => void onRun(card, action)} />
          {:else}
            <V1GhostButton title={action.label} onclick={() => void onRun(card, action)} />
          {/if}
        {/each}
      </div>
    {/if}
  </article>
{/snippet}

<style>
  .hd {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 24px;
  }
  .kicker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .lead-section {
    margin-bottom: 28px;
  }
  .sup {
    margin-top: 14px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 10px;
    margin-top: 10px;
  }
  .card,
  .status-card,
  .empty {
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-card);
    background: var(--vd-bg-1);
  }
  .card {
    padding: 12px;
  }
  .card.lead {
    background: var(--vd-bg-2);
    margin-top: 10px;
  }
  .card-top,
  .meta,
  .actions,
  .status-card {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .card-top {
    justify-content: space-between;
  }
  .badge {
    padding: 2px 7px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--vd-st-active) 18%, transparent);
    color: var(--vd-st-active);
    font-size: 10px;
    font-weight: 600;
  }
  .freshness,
  .meta {
    font-family: var(--vd-font-mono);
    font-size: 10.5px;
    color: var(--vd-fg-3);
  }
  h3 {
    margin: 8px 0 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--vd-fg-1);
    line-height: 1.25;
  }
  .summary,
  .why,
  .note {
    margin: 7px 0 0;
    font-size: 12px;
    line-height: 1.42;
    color: var(--vd-fg-2);
  }
  .why,
  .note {
    color: var(--vd-fg-3);
  }
  .note.warn {
    color: var(--vd-amber);
  }
  .meta {
    flex-wrap: wrap;
    margin-top: 10px;
  }
  .actions {
    flex-wrap: wrap;
    margin-top: 12px;
  }
  .status-card,
  .empty {
    margin-top: 24px;
    padding: 14px;
    color: var(--vd-fg-3);
    font-size: 13px;
  }
  .status-card.error {
    justify-content: space-between;
    color: var(--vd-st-attention);
  }
</style>
