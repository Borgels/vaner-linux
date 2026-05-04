<!--
  /dev/states — render any VanerState variant on demand for visual review.
  Pass ?kind=prepared (or learning, watching, attention, ...) in the query
  string. With no `kind` we show a directory of links.
-->
<script lang="ts">
  import { page } from "$app/stores";
  import EngineMissing from "$lib/components/popover-states/EngineMissing.svelte";
  import NotInstalled from "$lib/components/popover-states/NotInstalled.svelte";
  import InstalledNotConnected from "$lib/components/popover-states/InstalledNotConnected.svelte";
  import Learning from "$lib/components/popover-states/Learning.svelte";
  import Watching from "$lib/components/popover-states/Watching.svelte";
  import Prepared from "$lib/components/popover-states/Prepared.svelte";
  import Attention from "$lib/components/popover-states/Attention.svelte";
  import PermissionNeeded from "$lib/components/popover-states/PermissionNeeded.svelte";
  import NoActiveAgent from "$lib/components/popover-states/NoActiveAgent.svelte";
  import ActivePredictions from "$lib/components/popover-states/ActivePredictions.svelte";
  import PreparedWork from "$lib/components/popover-states/PreparedWork.svelte";
  import VanerError from "$lib/components/popover-states/Error.svelte";
  import Idle from "$lib/components/popover-states/Idle.svelte";
  import Paused from "$lib/components/popover-states/Paused.svelte";
  import NotWiredToAnyClient from "$lib/components/popover-states/NotWiredToAnyClient.svelte";
  import OllamaMissing from "$lib/components/popover-states/OllamaMissing.svelte";
  import type { PopoverRuntimeContext, PreparedMoment, SourceRef } from "$lib/state/types.js";

  const STATES = [
    "notWiredToAnyClient",
    "ollamaMissing",
    "engineMissing",
    "notInstalled",
    "installedNotConnected",
    "learning",
    "watching",
    "prepared",
    "preparedWork",
    "attention",
    "permissionNeeded",
    "noActiveAgent",
    "activePredictions",
    "error",
    "paused",
    "idle",
  ] as const;

  const githubSrc: SourceRef = {
    id: "github",
    kind: "github",
    label: "vaner-desktop",
    weight: 1,
  };
  const filesSrc: SourceRef = {
    id: "files",
    kind: "files",
    label: "Local notes",
    weight: 1,
  };

  const lead: PreparedMoment = {
    id: "lead-1",
    title: "Re-read tray.rs · 9 minutes ago",
    prediction: "When the user clicks the tray icon, anchor the popover near the icon, falling back to top-right when the positioner cache is empty.",
    why: [
      "You opened tray.rs three times today",
      "The macOS sibling already documents this dance",
      "PR #9 attempted but did not validate the fallback path",
    ],
    primarySource: githubSrc,
    sources: [githubSrc, filesSrc],
    confidence: 0.82,
    strength: "lead",
    readyAt: Date.now() - 9 * 60_000,
    pinned: false,
  };
  const supporting: PreparedMoment[] = [
    {
      id: "sup-1",
      title: "Wire on_tray_event in tray.rs",
      prediction: "tauri-plugin-positioner caches tray bounds via this hook; without it TrayCenter panics.",
      why: [],
      primarySource: githubSrc,
      sources: [githubSrc],
      confidence: 0.71,
      strength: "supporting",
      readyAt: Date.now() - 12 * 60_000,
      pinned: false,
    },
    {
      id: "sup-2",
      title: "Update CHANGELOG for v0.2.2",
      prediction: "Summarize the redesign + bug fixes for the release page.",
      why: [],
      primarySource: filesSrc,
      sources: [filesSrc],
      confidence: 0.55,
      strength: "supporting",
      readyAt: Date.now() - 30 * 60_000,
      pinned: false,
    },
  ];

  const context: PopoverRuntimeContext = {
    clientLabel: "Cursor",
    workspaceLabel: "vaner",
    signalLabels: ["editor", "MCP context", "recent activity", "docs"],
    predictionsReady: 2,
    predictionsWarming: 4,
    preparedReady: 1,
    preparedPartial: 2,
    lastUpdateLabel: "10s ago",
    statusLabel: "Learning",
  };

  const kind = $derived($page.url.searchParams.get("kind"));
</script>

<svelte:head>
  <title>vaner-desktop · states</title>
</svelte:head>

{#if !kind}
  <div class="index">
    <h1>vaner-desktop · popover states</h1>
    <ul>
      {#each STATES as s (s)}
        <li><a href={`/dev/states?kind=${s}`}>{s}</a></li>
      {/each}
    </ul>
    <p><a href="/dev/primitives">→ primitives storyboard</a></p>
  </div>
{:else}
  <div class="frame">
    <div class="popover">
      {#if kind === "notWiredToAnyClient"}
        <NotWiredToAnyClient
          detected={{ total: 4, wiredCount: 0, wiredLabels: [] }}
        />
      {:else if kind === "ollamaMissing"}
        <OllamaMissing installed={false} detail="Ollama isn't installed." />
      {:else if kind === "engineMissing"}
        <EngineMissing install={{ kind: "notDetected" }} />
      {:else if kind === "notInstalled"}
        <NotInstalled />
      {:else if kind === "installedNotConnected"}
        <InstalledNotConnected />
      {:else if kind === "learning"}
        <Learning
          progress={{
            filesWatched: 18,
            uptimeMinutes: 12,
            etaMinutes: 14,
            currentlyReading: [
              { source: "github", title: "src-tauri/src/tray.rs", since: "2m" },
              { source: "files", title: "design-canvas.jsx", since: "5m" },
              { source: "linear", title: "VAN-104 redesign popover states", since: "8m" },
            ],
          }}
          {context}
        />
      {:else if kind === "watching"}
        <Watching
          summary={{
            filesWatched: 42,
            sourcesCount: 3,
            preparedCount: 0,
            currentlyReading: [],
            lastPreparedAgo: "2h ago",
          }}
          silentHours={false}
          {context}
        />
      {:else if kind === "prepared"}
        <Prepared {lead} {supporting} {context} />
      {:else if kind === "preparedWork"}
        <PreparedWork
          {context}
          cards={[
            {
              id: "work-1",
              source_id: "work-1",
              source_type: "work_product",
              kind: "bug",
              title: "Potential setup drift in agent wiring",
              summary: "Claude Code and Codex CLI appear configured, but verification says required layers are missing.",
              badge: "Bug",
              confidence_label: "High",
              freshness_label: "Fresh",
              freshness_state: "fresh",
              target_label: "Agents setup",
              why_prepared: "Recent setup actions and client verification results disagree.",
              evidence_count: 4,
              created_at: Date.now(),
              updated_at: Date.now(),
              primary_action: { kind: "inspect", label: "Inspect", tool: null, endpoint: "/work-products/work-1/inspect", arguments: {} },
              secondary_actions: [],
            },
          ]}
        />
      {:else if kind === "attention"}
        <Attention
          conflict={{
            id: "conflict-1",
            headline: "Two sources disagree about the webhook signing format.",
            sources: [githubSrc, filesSrc],
            evidence: {
              sideALabel: "github · src-tauri/src/updater.rs (today)",
              sideASnippet: "verifyMinisign(payload, signature, base64Pubkey)",
              sideBLabel: "files · CONTRACT.md (last week)",
              sideBSnippet: "verify(payload, signature_hex, pubkey_hex)",
            },
          }}
        />
      {:else if kind === "permissionNeeded"}
        <PermissionNeeded
          sources={[
            { source: githubSrc, status: "blocked", detail: "Token expired 2h ago" },
            { source: filesSrc, status: "paused", detail: "User paused; click resume" },
          ]}
        />
      {:else if kind === "noActiveAgent"}
        <NoActiveAgent
          pendingCount={2}
          suggestedLaunch={[
            { id: "claude", displayName: "Claude Desktop", bundleIdentifier: null },
            { id: "cursor", displayName: "Cursor", bundleIdentifier: null },
          ]}
        />
      {:else if kind === "activePredictions"}
        <ActivePredictions
          {context}
          predictions={[
            {
              id: "p1",
              spec: { label: "Implement WS5 — companion window", confidence: 0.78, hypothesis_type: "explicit_intent", source: "engine", specificity: "exact" },
              run: { readiness: "ready", started_at: "2026-04-26T22:00:00Z" },
              artifacts: {},
            } as any,
            {
              id: "p2",
              spec: { label: "Wire engine_status Tauri command", confidence: 0.62, hypothesis_type: "behavioral", source: "engine", specificity: "approximate" },
              run: { readiness: "drafting", started_at: "2026-04-26T22:01:00Z" },
              artifacts: {},
            } as any,
          ]}
        />
      {:else if kind === "error"}
        <VanerError
          engine={{
            message: "The Vaner engine isn't responding on localhost.",
            port: 8473,
            incidentID: "VNR-7f2a",
          }}
        />
      {:else if kind === "paused"}
        <Paused queued={3} {context} />
      {:else if kind === "idle"}
        <Idle />
      {:else}
        <p style="padding: 20px; color: var(--vd-fg-3); font-family: var(--vd-font);">
          Unknown kind: {kind}
        </p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .index {
    padding: 36px;
    color: var(--vd-fg-1);
    font-family: var(--vd-font);
  }
  .index h1 { font-weight: 500; }
  .index ul {
    list-style: none;
    padding: 0;
    margin: 16px 0;
    display: grid;
    grid-template-columns: repeat(2, max-content);
    gap: 6px 24px;
  }
  .index a { color: var(--vd-amber); text-decoration: none; }
  .frame {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 36px;
    height: 100vh;
  }
  .popover {
    width: 420px;
    height: 560px;
    background: var(--vd-bg-0);
    border-radius: var(--vd-r-pop);
    border: 0.5px solid var(--vd-line);
    box-shadow: var(--vd-shadow-pop);
    overflow: hidden;
    display: flex;
  }
  .popover :global(.quiet-shell) { flex: 1 1 auto; }
</style>
