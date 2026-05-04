<!--
  Companion center pane router. Picks the right pane based on ?tab=.
-->
<script lang="ts">
  import { page } from "$app/stores";
  import PreparedPane from "./panes/PreparedPane.svelte";
  import AgentsPane from "./panes/AgentsPane.svelte";
  import ModelsPane from "./panes/ModelsPane.svelte";
  import EnginePane from "./panes/EnginePane.svelte";
  import FocusPane from "./panes/FocusPane.svelte";
  import PreferencesPane from "./panes/PreferencesPane.svelte";
  import DiagnosticsPane from "./panes/DiagnosticsPane.svelte";

  const active = $derived(($page.url.searchParams.get("tab") ?? "prepared").toLowerCase());
</script>

{#if active === "prepared"}
  <PreparedPane />
{:else if active === "agents" || active === "sources"}
  <!-- 'sources' aliased to 'agents' so any old deep-links keep working
       and the popover's old InstalledNotConnected CTA still resolves. -->
  <AgentsPane />
{:else if active === "models"}
  <ModelsPane />
{:else if active === "engine"}
  <EnginePane />
{:else if active === "focus"}
  <FocusPane />
{:else if active === "preferences"}
  <PreferencesPane />
{:else if active === "diagnostics"}
  <DiagnosticsPane />
{:else}
  <PreparedPane />
{/if}
