<script lang="ts">
  import { predictions } from "$lib/stores/predictions.js";
  import { preparedWork } from "$lib/stores/prepared-work.js";
  import { invoke } from "@tauri-apps/api/core";
  import { canRunPreparedWorkAction, dispatchPreparedWorkAction } from "$lib/prepared-work-actions.js";
  import { isAdoptable, type PredictedPrompt, type PreparedWorkCard } from "$lib/contract/types.js";
  import { showToast } from "$lib/stores/toast.js";

  type Props = { initialTab?: "predictions" | "prepared" };
  const { initialTab = "predictions" }: Props = $props();
  let tab = $state<"predictions" | "prepared">("predictions");
  $effect(() => {
    tab = initialTab;
  });

  const predictionList = $derived(($predictions ?? []).slice(0, 6));
  const workList = $derived(($preparedWork ?? []).slice(0, 4));
  const readyCount = $derived(($predictions ?? []).filter((p) => p.run.readiness === "ready").length);
  const warmingCount = $derived(($predictions ?? []).filter((p) => !isAdoptable(p.run.readiness)).length);

  function predictionLabel(p: PredictedPrompt): string {
    return p.readiness_label ?? p.run.readiness.replaceAll("_", " ");
  }

  function predictionDetail(p: PredictedPrompt): string {
    const parts = [
      p.eta_bucket_label,
      p.source_label,
      `${Math.round(p.spec.confidence * 100)}%`,
    ].filter(Boolean);
    return parts.join(" · ");
  }

  function workDetail(card: PreparedWorkCard): string {
    return [card.badge, card.confidence_label, card.freshness_label].filter(Boolean).join(" · ");
  }

  async function adopt(id: string) {
    try {
      const intent = await invoke<string>("adopt_prediction", { predictionId: id });
      showToast(`Prediction adopted — ${intent}.`, "success", 4000);
    } catch (err) {
      showToast(typeof err === "string" ? err : "Couldn't adopt that prediction.", "attention", 5000);
    }
  }

  async function runWork(card: PreparedWorkCard) {
    if (!canRunPreparedWorkAction(card, card.primary_action)) return;
    try {
      const result = await dispatchPreparedWorkAction(invoke, card, card.primary_action);
      showToast(result.kind === "adopt" ? `Prediction adopted — ${result.message}.` : `${card.primary_action.label} complete.`, "success", 3500);
    } catch (err) {
      showToast(typeof err === "string" ? err : `Couldn't run ${card.primary_action.label}.`, "attention", 5000);
    }
  }
</script>

<section class="switcher" aria-label="Vaner predictions and prepared work">
  <div class="tabs" role="tablist" aria-label="Prediction surfaces">
    <button
      type="button"
      role="tab"
      aria-selected={tab === "predictions"}
      class:active={tab === "predictions"}
      onclick={() => (tab = "predictions")}
    >
      Predictions
      <span>{($predictions ?? []).length}</span>
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={tab === "prepared"}
      class:active={tab === "prepared"}
      onclick={() => (tab = "prepared")}
    >
      Prepared work
      <span>{($preparedWork ?? []).length}</span>
    </button>
  </div>

  {#if tab === "predictions"}
    {#if predictionList.length > 0}
      <div class="summary">
        <span>{readyCount} ready</span>
        <span>{warmingCount} warming</span>
      </div>
      <div class="rows">
        {#each predictionList as p (p.id)}
          <article class="row">
            <div class="row-main">
              <div class="title">{p.spec.label}</div>
              {#if p.ui_summary || p.spec.description}
                <div class="body">{p.ui_summary ?? p.spec.description}</div>
              {/if}
              <div class="meta">{predictionDetail(p)}</div>
            </div>
            {#if isAdoptable(p.run.readiness)}
              <button class="row-action" type="button" onclick={() => adopt(p.id)}>Adopt</button>
            {:else}
              <span class={`state state-${p.run.readiness}`}>{predictionLabel(p)}</span>
            {/if}
          </article>
        {/each}
      </div>
    {:else}
      <div class="empty">No prediction candidates from the worker yet.</div>
    {/if}
  {:else if workList.length > 0}
    <div class="rows">
      {#each workList as card (card.id)}
        <article class="row">
          <div class="row-main">
            <div class="title">{card.title}</div>
            <div class="body">{card.summary}</div>
            <div class="meta">{workDetail(card)}</div>
          </div>
          {#if canRunPreparedWorkAction(card, card.primary_action)}
            <button class="row-action" type="button" onclick={() => runWork(card)}>{card.primary_action.label}</button>
          {:else}
            <span class="state state-ready">{card.kind}</span>
          {/if}
        </article>
      {/each}
    </div>
  {:else}
    <div class="empty">No prepared work is ready yet.</div>
  {/if}
</section>

<style>
  .switcher {
    margin-top: 12px;
    display: grid;
    gap: 8px;
  }
  .tabs {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 4px;
    padding: 3px;
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: 7px;
  }
  button {
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: 0;
    border-radius: 5px;
    padding: 6px 8px;
    background: transparent;
    color: var(--vd-fg-3);
    font-family: var(--vd-font);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
  }
  button.active {
    background: var(--vd-bg-2);
    color: var(--vd-fg-1);
  }
  button span {
    color: var(--vd-fg-4);
    font-family: var(--vd-font-mono);
    font-size: 10px;
    font-weight: 500;
  }
  .summary {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    color: var(--vd-fg-4);
    font-family: var(--vd-font-mono);
    font-size: 10.5px;
  }
  .rows {
    display: grid;
    gap: 6px;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content;
    gap: 10px;
    align-items: start;
    padding: 9px 10px;
    background: rgba(255, 255, 255, 0.025);
    border: 0.5px solid var(--vd-line);
    border-radius: 7px;
  }
  .row-main {
    min-width: 0;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--vd-fg-1);
    font-family: var(--vd-font);
    font-size: 12px;
    font-weight: 600;
  }
  .body {
    margin-top: 3px;
    display: -webkit-box;
    overflow: hidden;
    color: var(--vd-fg-4);
    font-family: var(--vd-font);
    font-size: 10.8px;
    line-height: 1.35;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  .meta {
    margin-top: 5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--vd-fg-4);
    font-family: var(--vd-font-mono);
    font-size: 10px;
  }
  .state {
    max-width: 112px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--vd-fg-3);
    font-family: var(--vd-font-mono);
    font-size: 9.5px;
    text-transform: uppercase;
  }
  .row-action {
    flex: 0 0 auto;
    align-self: start;
    border: 0.5px solid color-mix(in srgb, var(--vd-st-active) 45%, transparent);
    border-radius: 5px;
    padding: 5px 8px;
    background: color-mix(in srgb, var(--vd-st-active) 16%, transparent);
    color: var(--vd-fg-1);
    font-family: var(--vd-font);
    font-size: 10.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .state-ready {
    background: color-mix(in srgb, var(--vd-st-on) 18%, transparent);
    color: var(--vd-st-on);
  }
  .state-drafting {
    background: color-mix(in srgb, var(--vd-st-active) 18%, transparent);
    color: var(--vd-st-active);
  }
  .state-grounding,
  .state-evidence_gathering {
    background: color-mix(in srgb, var(--vd-st-learning) 18%, transparent);
    color: var(--vd-st-learning);
  }
  .empty {
    padding: 12px 10px;
    border: 0.5px solid var(--vd-line);
    border-radius: 7px;
    color: var(--vd-fg-4);
    background: rgba(255, 255, 255, 0.02);
    font-family: var(--vd-font);
    font-size: 11.5px;
  }
</style>
