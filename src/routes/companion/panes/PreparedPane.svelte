<!--
  Prepared pane — the popover's hero state, expanded for the companion
  window. Lead moment + supporting cards + per-moment "why" details.
  Mirrors vaner-desktop-macos/vaner/Companion/PreparedPane.swift.
-->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import VContextCard from "$lib/components/primitives/VContextCard.svelte";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import { showToast } from "$lib/stores/toast.js";
  import { prepared } from "$lib/stores/prepared.js";

  async function send(id: string) {
    try {
      const intent = await invoke<string>("adopt_prediction", { predictionId: id });
      showToast(`Adopted — ${intent}.`, "success", 4000);
    } catch (err) {
      showToast(typeof err === "string" ? err : "Couldn't send that moment.", "attention", 4000);
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
    text="The lead moment is what I'd send first. Supporting cards have lower confidence or smaller scope; expand them to see why."
  />
</header>

{#if $prepared.lead}
  <section class="lead-section">
    <VSectionLabel text="Lead" />
    <div class="lead-card">
      <VContextCard
        moment={{
          title: $prepared.lead.title,
          prediction: $prepared.lead.prediction,
          sourceKind: $prepared.lead.primarySource.kind,
          sourceLabel: $prepared.lead.primarySource.label,
          confidence: $prepared.lead.confidence,
          reasons: $prepared.lead.why,
          kicker: "Lead",
        }}
        isLead
      />
      <div class="actions">
        <V1PrimaryButton title="Send to agent" onclick={() => send($prepared.lead!.id)} />
        <V1GhostButton title="Copy context" />
        <V1GhostButton title="Pin" />
        <V1GhostButton title="Dismiss" destructive />
      </div>
    </div>
  </section>

  {#if $prepared.supporting.length > 0}
    <section class="sup">
      <VSectionLabel text={`Also prepared · ${$prepared.supporting.length}`} />
      <div class="grid">
        {#each $prepared.supporting as m (m.id)}
          <VContextCard
            moment={{
              title: m.title,
              prediction: m.prediction,
              sourceKind: m.primarySource.kind,
              sourceLabel: m.primarySource.label,
              confidence: m.confidence,
            }}
            onclick={() => send(m.id)}
          />
        {/each}
      </div>
    </section>
  {/if}
{:else}
  <section class="empty">
    <V1Body
      muted
      text="No prepared moments right now. The popover will surface them in real time as soon as Vaner's confidence is high enough."
    />
  </section>
{/if}

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
  .lead-card {
    margin-top: 10px;
  }
  .actions {
    display: flex;
    gap: 6px;
    margin-top: 12px;
    flex-wrap: wrap;
  }
  .sup {
    margin-top: 14px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 10px;
    margin-top: 10px;
  }
  .empty {
    margin-top: 24px;
  }
</style>
