<!--
  Prepared — the hero state. Lead moment + Send/Copy/Dismiss + supporting
  cards. Mirrors vaner-desktop-macos/vaner/Popover/States/PreparedView.swift.
-->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import QuietShell from "$lib/components/primitives/QuietShell.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import SourceGlyph from "$lib/components/primitives/SourceGlyph.svelte";
  import PopoverFooter from "$lib/components/PopoverFooter.svelte";
  import PopoverContextBlock from "./PopoverContextBlock.svelte";
  import PopoverQuickActions from "./PopoverQuickActions.svelte";
  import { showToast } from "$lib/stores/toast.js";
  import type { PopoverRuntimeContext, PreparedMoment } from "$lib/state/types.js";

  type Props = {
    lead: PreparedMoment;
    supporting: PreparedMoment[];
    context: PopoverRuntimeContext;
  };
  const { lead, supporting, context }: Props = $props();

  const minutesAgo = $derived(
    Math.max(0, Math.floor((Date.now() - lead.readyAt) / 60_000)),
  );
  const confPct = $derived(Math.round(lead.confidence * 100));

  async function send() {
    try {
      const intent = await invoke<string>("adopt_prediction", { predictionId: lead.id });
      showToast(
        `Prepared moment adopted — ${intent}. Your agent's next prompt will use this package.`,
        "success",
        5000,
      );
    } catch (err) {
      const msg = typeof err === "string" ? err : "Couldn't send that moment.";
      showToast(msg, "attention", 5000);
    }
  }

  async function copyContext() {
    try {
      const text = lead.prediction || lead.title;
      await navigator.clipboard.writeText(text);
      showToast("Copied to clipboard.", "success", 3000);
    } catch {
      showToast("Couldn't access the clipboard.", "attention", 4000);
    }
  }

  function dismiss() {
    // Dismiss endpoint TBD; for now just acknowledge.
    showToast("Dismissed for this session.", "info", 3000);
  }
</script>

<QuietShell markState="prepared" stateLabel={`Prepared · ${minutesAgo}m ago`} stateLabelTint="var(--vd-amber)">
  <V1Kicker text="Prepared for you" color="var(--vd-amber)" />
  <div class="gap-8"></div>
  <V1Headline text={lead.prediction} size={15} />

  <div class="trigger">
    <span class="trigger__label">Triggered by</span>
    <span class="trigger__title">{lead.title}</span>
    <span class="trigger__sep">·</span>
    <span class="trigger__conf">{confPct}% confident</span>
  </div>

  <div class="actions">
    <V1PrimaryButton title="Send to agent" onclick={send} />
    <V1GhostButton title="Copy context" onclick={copyContext} />
    <V1GhostButton title="Dismiss" onclick={dismiss} />
  </div>

  {#if lead.why && lead.why.length > 0}
    <div class="why">
      <V1Kicker text="Why this, now" />
      <ul>
        {#each lead.why as r (r)}
          <li>
            <span class="bullet"></span>
            <span>{r}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if supporting.length > 0}
    <div class="also-prepared">
      <V1Kicker text="Also prepared" />
      <div class="rows">
        {#each supporting as m (m.id)}
          <div class="row">
            <SourceGlyph kind={m.primarySource.kind} size={12} dim />
            <div class="row-body">
              <div class="row-title">{m.title}</div>
              <div class="row-pred">{m.prediction}</div>
            </div>
            <span class="row-conf">{Math.round(m.confidence * 100)}%</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <PopoverContextBlock {context} compact />
  <PopoverQuickActions cockpitPrimary={false} tab="prepared" />

  {#snippet footer()}
    <PopoverFooter health="on" healthLabel={`Last update ${context.lastUpdateLabel}`} detailsTab="prepared" />
  {/snippet}
</QuietShell>

<style>
  .gap-8 { height: 8px; }
  .trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
    font-family: var(--vd-font);
    font-size: 12px;
  }
  .trigger__label, .trigger__sep { color: var(--vd-fg-3); }
  .trigger__title { color: var(--vd-fg-2); font-weight: 500; }
  .trigger__conf  { color: var(--vd-amber); font-weight: 600; }

  .actions {
    display: flex;
    gap: 6px;
    margin-top: 14px;
    flex-wrap: wrap;
  }

  .why {
    margin-top: 18px;
  }
  .why ul {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .why li {
    display: flex;
    gap: 8px;
    font-family: var(--vd-font);
    font-size: 12px;
    color: var(--vd-fg-2);
    line-height: 1.42;
  }
  .why .bullet {
    flex: 0 0 auto;
    width: 5px;
    height: 5px;
    margin-top: 7px;
    border-radius: 50%;
    background: var(--vd-amber);
    opacity: 0.9;
  }

  .also-prepared {
    margin-top: 18px;
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: rgba(255, 255, 255, 0.02);
    border: 0.5px solid var(--vd-hair);
    border-radius: 7px;
  }
  .row-body { flex: 1 1 auto; min-width: 0; }
  .row-title {
    font-family: var(--vd-font);
    font-size: 12px;
    font-weight: 500;
    color: var(--vd-fg-1);
  }
  .row-pred {
    font-family: var(--vd-font);
    font-size: 11px;
    color: var(--vd-fg-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-conf {
    flex: 0 0 auto;
    font-family: var(--vd-font-mono);
    font-size: 10.5px;
    color: var(--vd-fg-4);
    font-variant-numeric: tabular-nums;
  }
</style>
