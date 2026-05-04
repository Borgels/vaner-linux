// vaner-state.ts — the popover's single source of truth. A Svelte `derived`
// store that runs the pure reducer (src/lib/state/reducer.ts) over the
// observable inputs and produces a `VanerState` discriminated union. Every
// popover state component subscribes to this; the route at
// src/routes/+page.svelte switches on `kind`.

import { derived, type Readable } from "svelte/store";
import { reduce, type ReducerInputs } from "$lib/state/reducer.js";
import type { VanerState } from "$lib/state/types.js";
import { agentDetector } from "./agent-detector.js";
import { isPaused } from "./app-state.js";
import { blockedSources } from "./blocked-sources.js";
import { clientDetectStatus } from "./clients.js";
import { engineStatus } from "./engine-status.js";
import { focus } from "./focus.js";
import { ollamaHealth } from "./ollama-health.js";
import { predictions } from "./predictions.js";
import { prepared } from "./prepared.js";
import { preparedWork } from "./prepared-work.js";
import { silentHours } from "./silent-hours.js";

type FocusSnapshot = typeof focus extends Readable<infer T> ? T : never;

function activeWorkspaceLabel(f: FocusSnapshot): string {
  const active = f.workspaces.find((workspace) => workspace.id === f.active_workspace_id);
  if (active?.display_name) return active.display_name;
  const pinned = f.workspaces.find((workspace) => workspace.pinned);
  if (pinned?.display_name) return pinned.display_name;
  if (f.workspaces[0]?.display_name) return f.workspaces[0].display_name;
  return "unknown";
}

function activeClientLabel(f: FocusSnapshot, detected: { wiredLabels: string[] }): string {
  const runningIntegrated = f.detected_clients.find(
    (client) => client.running && !["missing", "not_detected", "none"].includes(client.integration_state),
  );
  if (runningIntegrated?.display_name) return runningIntegrated.display_name;
  const running = f.detected_clients.find((client) => client.running);
  if (running?.display_name) return running.display_name;
  if (detected.wiredLabels[0]) return detected.wiredLabels[0];
  return "none detected";
}

function clientSignal(label: string): string | null {
  const l = label.toLowerCase();
  if (!label || l === "none detected") return null;
  if (l.includes("cursor") || l.includes("zed") || l.includes("code") || l.includes("copilot")) return "editor";
  if (l.includes("terminal") || l.includes("claude code") || l.includes("codex")) return "terminal";
  if (l.includes("browser") || l.includes("chatgpt")) return "browser";
  return "client activity";
}

function inferSignals(args: {
  clientLabel: string;
  statusSources: number;
  filesWatched: number;
  wiredCount: number;
  predictions: Array<{ spec?: { label?: string | null }; ui_summary?: string | null }>;
  work: Array<{ target_label?: string | null; summary?: string | null; title?: string | null }>;
}): string[] {
  const labels = new Set<string>();
  const c = clientSignal(args.clientLabel);
  if (c) labels.add(c);
  if (args.wiredCount > 0) labels.add("MCP context");
  if (args.filesWatched > 0) labels.add("files");
  if (args.statusSources > 0) labels.add("recent activity");

  const text = [
    ...args.predictions.map((p) => `${p.spec?.label ?? ""} ${p.ui_summary ?? ""}`),
    ...args.work.map((card) => `${card.title ?? ""} ${card.summary ?? ""} ${card.target_label ?? ""}`),
  ].join(" ").toLowerCase();
  if (/\b(test|spec|coverage)\b/.test(text)) labels.add("tests");
  if (/\b(doc|docs|readme|changelog|markdown)\b/.test(text)) labels.add("docs");
  if (/\b(prompt|mcp|agent)\b/.test(text)) labels.add("prompts");

  return [...labels].slice(0, 5);
}

export const vanerState: Readable<VanerState> = derived(
  [
    predictions,
    preparedWork,
    engineStatus,
    prepared,
    blockedSources,
    agentDetector,
    silentHours,
    isPaused,
    clientDetectStatus,
    ollamaHealth,
    focus,
  ],
  ([$preds, $work, $status, $prep, $blocked, $agents, $silent, $paused, $clientDetect, $ollama, $focus]) => {
    const hasAnySource = $status.sourcesCount > 0;
    const clientLabel = activeClientLabel($focus, $clientDetect);
    const inputs: ReducerInputs = {
      status: $status,
      prepared: $prep,
      blockedSources: $blocked,
      anyAgentRunning: $agents.runningCount > 0,
      silentHours: $silent,
      hasAnySource,
      clientDetect: $clientDetect,
      activePredictions: $preds,
      preparedWork: $work,
      noAgentSuggestions: $agents.suggestions,
      paused: $paused,
      ollamaHealth: $ollama,
      activity: {
        clientLabel,
        workspaceLabel: activeWorkspaceLabel($focus),
        signalLabels: inferSignals({
          clientLabel,
          statusSources: $status.sourcesCount,
          filesWatched: $status.filesWatched,
          wiredCount: $clientDetect.wiredCount,
          predictions: $preds,
          work: $work,
        }),
      },
    };
    return reduce(inputs);
  },
);
