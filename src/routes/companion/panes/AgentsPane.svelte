<!--
  Agents pane — wraps the existing MCPClientsPanel from /preferences,
  re-skinned at its container level. Underlying logic (clients store +
  install/uninstall/doctor invokes) is unchanged.
-->
<script lang="ts">
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import MCPClientsPanel from "../../preferences/MCPClientsPanel.svelte";
  import WizardVerificationPanel from "$lib/components/WizardVerificationPanel.svelte";
  import { install } from "$lib/stores/clients.js";

  // Per-layer status mirrors what the onboarding wizard's last slide
  // shows — same panel, same data, just slotted into the companion.
  // The user asked for parity: "On the Agents screen I'd like to see
  // the same status for each client (which layer is installed)".
  // The MCPClientsPanel below stays for install / reinstall / remove
  // actions; this one is the at-a-glance leverage view.
  async function repairClient(clientId: string): Promise<void> {
    await install(clientId, "", true);
  }
</script>

<header class="hd">
  <div class="kicker-row">
    <V1Kicker text="Agents" />
    <DocsLink path="/integrations/connect-your-client" />
  </div>
  <V1Headline text="Who can Vaner talk to" size={22} />
</header>

<WizardVerificationPanel repoRoot="" onRepair={repairClient} />

<section class="panel">
  <MCPClientsPanel />
</section>

<style>
  .hd { display: flex; flex-direction: column; gap: 6px; margin-bottom: 24px; }
  .kicker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .panel {
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-card);
    background: var(--vd-bg-1);
    padding: 16px;
  }
</style>
