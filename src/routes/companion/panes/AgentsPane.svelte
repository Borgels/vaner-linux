<!--
  Agents pane — one leverage-status list with install/finish/remove actions.
-->
<script lang="ts">
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import WizardVerificationPanel from "$lib/components/WizardVerificationPanel.svelte";
  import { install, uninstall } from "$lib/stores/clients.js";

  async function repairClient(clientId: string): Promise<void> {
    await install(clientId, "", true);
  }

  async function removeClient(clientId: string): Promise<void> {
    await uninstall(clientId, "");
  }
</script>

<header class="hd">
  <div class="kicker-row">
    <V1Kicker text="Agents" />
    <DocsLink path="/integrations/connect-your-client" />
  </div>
  <V1Headline text="Who can Vaner talk to" size={22} />
</header>

<WizardVerificationPanel repoRoot="" onRepair={repairClient} onRemove={removeClient} />

<style>
  .hd { display: flex; flex-direction: column; gap: 6px; margin-bottom: 24px; }
  .kicker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
</style>
