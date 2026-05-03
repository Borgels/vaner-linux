<script lang="ts">
  import QuietShell from "$lib/components/primitives/QuietShell.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import PopoverFooter from "$lib/components/PopoverFooter.svelte";
  import PopoverContextBlock from "./PopoverContextBlock.svelte";
  import PopoverQuickActions from "./PopoverQuickActions.svelte";
  import PredictionWorkSwitcher from "./PredictionWorkSwitcher.svelte";
  import type { PreparedWorkCard } from "$lib/contract/types.js";
  import type { PopoverRuntimeContext } from "$lib/state/types.js";

  type Props = { cards: PreparedWorkCard[]; context: PopoverRuntimeContext };
  const { cards, context }: Props = $props();
  const readyTitle = $derived(
    context.preparedReady === 1
      ? "1 prepared item ready"
      : `${Math.max(context.preparedReady, cards.length)} prepared items ready`,
  );
</script>

<QuietShell markState="active" stateLabel={`Prepared work · ${cards.length}`} stateLabelTint="var(--vd-st-active)">
  <V1Kicker text="Prepared work" color="var(--vd-st-active)" />
  <div class="gap-6"></div>
  <V1Headline text={readyTitle} />

  <PredictionWorkSwitcher initialTab="prepared" />

  <PopoverContextBlock {context} compact />
  <PopoverQuickActions cockpitPrimary={false} tab="prepared" />

  {#snippet footer()}
    <PopoverFooter health="active" healthLabel={`Last update ${context.lastUpdateLabel}`} detailsTab="prepared" />
  {/snippet}
</QuietShell>

<style>
  .gap-6 { height: 6px; }
</style>
