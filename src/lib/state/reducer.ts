// Pure reducer — ported line-by-line from
// vaner-desktop-macos/vaner/State/StateReducer.swift (lines 39–114).
// Same precedence chain, same fallbacks, same input shape. Tests at
// ./reducer.test.ts cover every branch with the macOS fixtures.
//
// Do NOT optimize. Do NOT short-circuit. The chain is the contract;
// any deviation between Linux and macOS shows up as a different popover
// state for the same daemon condition, which is exactly the bug a single
// pure reducer is supposed to prevent.

import { isAdoptable, type PredictedPrompt, type PreparedWorkCard } from "$lib/contract/types.js";
import type {
  AgentSuggestion,
  ClientDetectStatus,
  EngineStatus,
  LearningProgress,
  PopoverRuntimeContext,
  PreparedList,
  SourceStatus,
  VanerState,
  WatchingSummary,
} from "./types.js";

export interface ReducerInputs {
  status: EngineStatus;
  prepared: PreparedList;
  blockedSources: SourceStatus[];
  anyAgentRunning: boolean;
  silentHours: boolean;
  hasAnySource: boolean;
  /** Snapshot of which AI clients have Vaner registered. When zero
   *  clients are wired in, the desktop has no consumer for the
   *  daemon — surfacing engine state ahead of "go integrate Vaner
   *  somewhere" is putting the cart before the horse. The popover
   *  routes to `.notWiredToAnyClient` until the detector confirms
   *  ≥1 wired client. Defaults are tolerant: if `detected.total = 0`
   *  the detector hasn't run yet (or failed), and we don't gate on
   *  it — falling through to whichever engine state is real. */
  clientDetect: ClientDetectStatus;
  /** Ollama presence + reachability. Vaner's local-first default
   *  backend is Ollama on `127.0.0.1:11434`; if it isn't installed
   *  the model loop 502s on every MCP call. Routed before the
   *  engine-error branch so the user sees the *cause* ("install
   *  Ollama") rather than the *symptom* ("engine isn't responding"). */
  ollamaHealth: { installed: boolean; running: boolean; detail: string };
  /** 0.8.0 prediction-centric pondering. Defaults to [] for callers
   *  that haven't been updated to the new shape. */
  activePredictions: PredictedPrompt[];
  preparedWork: PreparedWorkCard[];
  activity: {
    clientLabel: string;
    workspaceLabel: string;
    signalLabels: string[];
  };
  /** Suggested agents to launch when noActiveAgent fires. Equivalent
   *  to the macOS `PreviewData.noAgentSuggestions` constant; injected
   *  here so the reducer stays pure (no static-data import). */
  noAgentSuggestions: AgentSuggestion[];
  /** Tray-menu Pause toggle. When true, the popover renders a calm
   *  .paused state with a Resume button. Urgent states (error,
   *  permissionNeeded, attention, engineMissing) still show through
   *  so the user isn't silenced into a broken engine. */
  paused: boolean;
}

function normalizeEpochMs(value: number | null | undefined): number | null {
  if (value == null || !Number.isFinite(value) || value <= 0) return null;
  return value > 1_000_000_000_000 ? value : value * 1000;
}

function formatLastUpdate(status: EngineStatus, predictions: PredictedPrompt[], work: PreparedWorkCard[]): string {
  if (status.lastCycleSecondsAgo != null) {
    if (status.lastCycleSecondsAgo <= 5) return "just now";
    if (status.lastCycleSecondsAgo < 60) return `${status.lastCycleSecondsAgo}s ago`;
    return `${Math.round(status.lastCycleSecondsAgo / 60)}m ago`;
  }

  const newest = [
    ...predictions.map((p) => normalizeEpochMs(p.run.updated_at ?? p.spec.created_at)),
    ...work.map((card) => normalizeEpochMs(card.updated_at || card.created_at)),
  ].filter((v): v is number => v != null).sort((a, b) => b - a)[0];

  if (!newest) return "just now";
  const seconds = Math.max(0, Math.round((Date.now() - newest) / 1000));
  if (seconds <= 5) return "just now";
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m ago`;
  return `${Math.round(seconds / 3600)}h ago`;
}

function isPreparedCardReady(card: PreparedWorkCard): boolean {
  const freshness = (card.freshness_state ?? "").toLowerCase();
  const confidence = card.confidence_label.toLowerCase();
  if (freshness === "stale" || freshness === "possibly_stale") return false;
  if (confidence.includes("low")) return false;
  return true;
}

function runtimeContext(i: ReducerInputs, statusLabel: string): PopoverRuntimeContext {
  const predictionsReady = i.activePredictions.filter((p) => p.run.readiness === "ready").length;
  const predictionsWarming = Math.max(0, i.activePredictions.length - predictionsReady);
  const preparedReady = i.preparedWork.filter(isPreparedCardReady).length + (i.prepared.lead ? 1 : 0);
  const preparedPartial = i.preparedWork.length - i.preparedWork.filter(isPreparedCardReady).length + i.prepared.supporting.length;

  return {
    clientLabel: i.activity.clientLabel || "none detected",
    workspaceLabel: i.activity.workspaceLabel || "unknown",
    signalLabels: i.activity.signalLabels.length ? i.activity.signalLabels : ["recent activity"],
    predictionsReady,
    predictionsWarming,
    preparedReady,
    preparedPartial,
    lastUpdateLabel: formatLastUpdate(i.status, i.activePredictions, i.preparedWork),
    statusLabel,
  };
}

export function reduce(i: ReducerInputs): VanerState {
  // 1a. Vaner CLI itself isn't installed → .notInstalled. This MUST
  //     come before the unreachable branch: a fresh `vaner-desktop`
  //     install on a machine that's never seen the CLI would
  //     otherwise show "Engine error / restart engine", which is
  //     misleading. .notInstalled gives a real install link.
  if (i.status.cliMissing) {
    return { kind: "notInstalled" };
  }

  // 1b. No MCP client has Vaner wired in → .notWiredToAnyClient.
  //     This MUST sit before the engine-reachability check: with
  //     production-mode auto-bring-up disabled, a fresh install
  //     legitimately has no daemon running until a client invokes
  //     `vaner mcp`. Showing .error in that window is wrong — the
  //     engine isn't broken, it's idle because no consumer exists.
  //     `total === 0` (detector hasn't completed yet) is treated
  //     as wiredCount === 0 too, since "show the wire-a-client
  //     panel for half a second on cold start" is a better UX than
  //     "flash a scary engine-error then settle into the right
  //     state."
  if (i.clientDetect.wiredCount === 0) {
    return { kind: "notWiredToAnyClient", detected: i.clientDetect };
  }

  // 1c. Local-first prerequisite: Ollama. Vaner's default backend
  //     points at `127.0.0.1:11434`; if Ollama isn't installed (or
  //     isn't running), the model loop 502s on every MCP call and
  //     the user sees a "engine unreachable" toast that doesn't
  //     name the actual cause. Surface `.ollamaMissing` before the
  //     engine-state branches so the user gets the *cause* rather
  //     than the *symptom*.
  if (!i.ollamaHealth.installed) {
    return {
      kind: "ollamaMissing",
      installed: false,
      detail: i.ollamaHealth.detail,
    };
  }

  // 1d. Clients are wired but the engine isn't reachable. Now it's
  //     a real error — the user expects Vaner to be live somewhere
  //     and the cockpit is silent. Overrides pause; this is
  //     actionable.
  if (!i.status.reachable) {
    return {
      kind: "error",
      engine: {
        message: "The Vaner engine isn't responding on localhost.",
        port: null,
        incidentID: null,
      },
    };
  }

  // 2. Any blocked sources (expired auth) → .permissionNeeded
  //    (also overrides pause — auth needs the user)
  if (i.blockedSources.length > 0) {
    return { kind: "permissionNeeded", sources: i.blockedSources };
  }

  // 3. No sources configured → .installedNotConnected
  //    (overrides pause — the user can't have meant to pause an
  //    engine that hasn't started yet)
  if (!i.hasAnySource) {
    return { kind: "installedNotConnected" };
  }

  // 4. Paused: count anything in flight so the user knows what
  //    Vaner is holding back, then short-circuit to .paused.
  //    Comes after the urgent-3 states so an error during pause
  //    still surfaces.
  if (i.paused) {
    const queued =
      i.preparedWork.length +
      i.activePredictions.filter((p) => isAdoptable(p.run.readiness)).length +
      (i.prepared.lead ? 1 : 0) +
      i.prepared.supporting.length;
    return { kind: "paused", queued, context: runtimeContext(i, "Paused") };
  }

  // 4. Currently learning → .learning
  if (i.status.indexing.kind === "learning") {
    const progress: LearningProgress = {
      filesWatched: i.status.filesWatched,
      uptimeMinutes: i.status.uptimeMinutes,
      currentlyReading: i.status.indexing.currentlyReading,
      etaMinutes: i.status.indexing.etaMinutes,
    };
    return { kind: "learning", progress, context: runtimeContext(i, "Learning") };
  }

  if (i.preparedWork.length > 0) {
    return { kind: "preparedWork", cards: i.preparedWork, context: runtimeContext(i, "Ready") };
  }

  // 5. 0.8.0 — predictions in drafting/ready outrank a reactive
  //    .prepared moment. Symmetric with .prepared: if no agent is
  //    running, redirect to .noActiveAgent so the user launches one
  //    before adopting (the Resolution would land in a pending-adopt
  //    file no one is watching otherwise).
  const surfacable = i.activePredictions.filter((p) => isAdoptable(p.run.readiness));
  if (surfacable.length > 0) {
    if (!i.anyAgentRunning) {
      return {
        kind: "noActiveAgent",
        pendingCount: surfacable.length,
        suggestedLaunch: i.noAgentSuggestions,
      };
    }
    const sorted = [...surfacable].sort((lhs, rhs) => {
      if (lhs.run.readiness !== rhs.run.readiness) {
        // .ready before .drafting
        return lhs.run.readiness === "ready" ? -1 : 1;
      }
      return rhs.spec.confidence - lhs.spec.confidence;
    });
    return { kind: "activePredictions", predictions: sorted, context: runtimeContext(i, "Preparing") };
  }

  // 6. Reactive prepared moment(s) exist → .prepared (or .noActiveAgent
  //    when nothing's running to receive the handoff).
  if (i.prepared.lead) {
    if (!i.anyAgentRunning) {
      return {
        kind: "noActiveAgent",
        pendingCount: 1 + i.prepared.supporting.length,
        suggestedLaunch: i.noAgentSuggestions,
      };
    }
    return {
      kind: "prepared",
      lead: i.prepared.lead,
      supporting: i.prepared.supporting,
      context: runtimeContext(i, "Ready"),
    };
  }

  // 7. Default → .watching (alive and reading, nothing strong yet).
  const summary: WatchingSummary = {
    filesWatched: i.status.filesWatched,
    sourcesCount: i.status.sourcesCount,
    preparedCount: 0,
    currentlyReading: [],
    lastPreparedAgo: null,
  };
  return { kind: "watching", summary, silentHours: i.silentHours, context: runtimeContext(i, "Learning") };
}
