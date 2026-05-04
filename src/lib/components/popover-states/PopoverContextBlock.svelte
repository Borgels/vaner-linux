<script lang="ts">
  import type { PopoverRuntimeContext } from "$lib/state/types.js";

  type Props = {
    context: PopoverRuntimeContext;
    compact?: boolean;
  };
  const { context, compact = false }: Props = $props();

  const predictionLabel = $derived(
    context.predictionsReady > 0
      ? `${context.predictionsReady} ready${context.predictionsWarming ? `, ${context.predictionsWarming} warming` : ""}`
      : context.predictionsWarming > 0
        ? "warming"
        : "none yet",
  );
  const preparedLabel = $derived(
    context.preparedReady > 0
      ? `${context.preparedReady} ready${context.preparedPartial ? `, ${context.preparedPartial} partial` : ""}`
      : context.preparedPartial > 0
        ? `${context.preparedPartial} partial`
        : "none yet",
  );
</script>

<section class="context" class:compact aria-label="Current Vaner context">
  <div class="row">
    <span>Client</span>
    <strong>{context.clientLabel}</strong>
  </div>
  <div class="row">
    <span>Workspace</span>
    <strong>{context.workspaceLabel}</strong>
  </div>
  <div class="row">
    <span>Signals</span>
    <strong>{context.signalLabels.join(", ")}</strong>
  </div>
  <div class="row">
    <span>Predictions</span>
    <strong>{predictionLabel}</strong>
  </div>
  <div class="row">
    <span>Prepared work</span>
    <strong>{preparedLabel}</strong>
  </div>
  <div class="row">
    <span>Last update</span>
    <strong>{context.lastUpdateLabel}</strong>
  </div>
</section>

<style>
  .context {
    display: grid;
    gap: 7px;
    padding: 10px 12px;
    margin-top: 12px;
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-card);
  }
  .context.compact {
    margin-top: 10px;
    gap: 6px;
  }
  .row {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr);
    gap: 10px;
    align-items: baseline;
    min-width: 0;
    font-family: var(--vd-font);
    font-size: 11.5px;
    line-height: 1.3;
  }
  .row span {
    color: var(--vd-fg-4);
  }
  .row strong {
    min-width: 0;
    color: var(--vd-fg-2);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
