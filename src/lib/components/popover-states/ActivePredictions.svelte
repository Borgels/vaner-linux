<!--
  ActivePredictions — prediction-centric pondering with a compact switcher
  for predictions and prepared work.
-->
<script lang="ts">
  import QuietShell from "$lib/components/primitives/QuietShell.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import PopoverFooter from "$lib/components/PopoverFooter.svelte";
  import PopoverContextBlock from "./PopoverContextBlock.svelte";
  import PopoverQuickActions from "./PopoverQuickActions.svelte";
  import PredictionWorkSwitcher from "./PredictionWorkSwitcher.svelte";
  import type { PredictedPrompt } from "$lib/contract/types.js";
  import type { PopoverRuntimeContext } from "$lib/state/types.js";

  type Props = { predictions: PredictedPrompt[]; context: PopoverRuntimeContext };
  const { predictions, context }: Props = $props();
</script>

<QuietShell markState="active" stateLabel={`Pondering · ${predictions.length} active`} stateLabelTint="var(--vd-st-active)">
  <V1Kicker text="Likely next steps" color="var(--vd-st-active)" />
  <div class="gap-6"></div>
  <V1Headline text={predictions[0]?.spec.label ?? "Vaner is preparing options"} />

  <PredictionWorkSwitcher initialTab="predictions" />

  <PopoverContextBlock {context} compact />
  <PopoverQuickActions cockpitPrimary={false} tab="prepared" />

  {#snippet footer()}
    <PopoverFooter health="active" healthLabel={`Last update ${context.lastUpdateLabel}`} detailsTab="prepared" />
  {/snippet}
</QuietShell>

<style>
  .gap-6 { height: 6px; }
</style>
