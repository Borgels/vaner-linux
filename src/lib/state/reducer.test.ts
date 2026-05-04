// Reducer tests — port of vaner-desktop-macos/vanerTests/StateReducerTests.swift.
// Covers every branch of the precedence chain in reducer.ts. Each test
// builds a minimal `ReducerInputs` and asserts the resulting kind.

import { describe, expect, it } from "vitest";
import { reduce, type ReducerInputs } from "./reducer.js";
import type {
  AgentSuggestion,
  EngineStatus,
  PreparedList,
  PreparedMoment,
  SourceRef,
  SourceStatus,
} from "./types.js";
import type { PredictedPrompt, PreparedWorkCard } from "$lib/contract/types.js";

// ---------- helpers ----------

const reachableStatus = (
  override: Partial<EngineStatus> = {},
): EngineStatus => ({
  reachable: true,
  cliMissing: false,
  filesWatched: 12,
  sourcesCount: 3,
  uptimeMinutes: 14,
  lastCycleSecondsAgo: 30,
  cycleIntervalSeconds: 60,
  indexing: { kind: "idle" },
  ...override,
});

const emptyPrepared = (): PreparedList => ({
  lead: null,
  supporting: [],
  pendingWhenNoAgent: 0,
});

const githubSrc: SourceRef = {
  id: "github",
  kind: "github",
  label: "vaner-desktop",
  weight: 1,
};

const baseInputs = (override: Partial<ReducerInputs> = {}): ReducerInputs => ({
  status: reachableStatus(),
  prepared: emptyPrepared(),
  blockedSources: [],
  anyAgentRunning: true,
  silentHours: false,
  hasAnySource: true,
  // Detector reports ≥1 wired client by default so the reducer
  // doesn't trip into `.notWiredToAnyClient` for unrelated branches.
  clientDetect: { total: 4, wiredCount: 1, wiredLabels: ["Cursor"] },
  activePredictions: [],
  preparedWork: [],
  activity: {
    clientLabel: "Cursor",
    workspaceLabel: "vaner",
    signalLabels: ["editor", "MCP context", "recent activity"],
  },
  noAgentSuggestions: [],
  paused: false,
  // Ollama is the local-first default backend. Default to "installed
  // and running" so unrelated branches don't trip into
  // `.ollamaMissing`; the dedicated tests below override.
  ollamaHealth: { installed: true, running: true, detail: "" },
  ...override,
});

const aMoment = (id: string, conf = 0.7): PreparedMoment => ({
  id,
  title: `Moment ${id}`,
  prediction: "do the thing",
  why: ["a", "b"],
  primarySource: githubSrc,
  sources: [githubSrc],
  confidence: conf,
  strength: "lead",
  readyAt: 0,
  pinned: false,
});

const aPrediction = (
  readiness: "ready" | "drafting" | "queued",
  confidence = 0.7,
): PredictedPrompt =>
  ({
    id: `${readiness}-${confidence}`,
    spec: {
      label: `Prediction ${readiness}`,
      hypothesis_type: "explicit_intent",
      source: "engine",
      confidence,
      specificity: "exact",
    },
    run: { readiness, started_at: "2026-04-26T20:00:00Z" },
    artifacts: {},
  }) as unknown as PredictedPrompt;

const aPreparedWorkCard = (): PreparedWorkCard => ({
  id: "work_product:wp",
  source_id: "wp",
  source_type: "work_product",
  kind: "diff",
  title: "Prepared fix",
  summary: "Patch-shaped artifact ready for inspection.",
  badge: "Diff",
  confidence_label: "High",
  freshness_label: "Fresh",
  target_label: "src/file.ts",
  evidence_count: 2,
  created_at: 0,
  updated_at: 0,
  primary_action: {
    kind: "export",
    label: "Export",
    tool: "vaner.work_products.export",
    endpoint: "/work-products/wp/export",
    arguments: { work_product_id: "wp" },
  },
  secondary_actions: [],
});

const blockedSrc = (): SourceStatus => ({
  source: githubSrc,
  status: "blocked",
  detail: "Token expired",
});

const fakeAgent = (id: string): AgentSuggestion => ({
  id,
  displayName: id,
  bundleIdentifier: null,
});

// ---------- tests ----------

describe("StateReducer precedence chain", () => {
  it("CLI missing wins over no-clients-wired (install before integrate)", () => {
    const out = reduce(
      baseInputs({
        status: reachableStatus({ cliMissing: true }),
        clientDetect: { total: 4, wiredCount: 0, wiredLabels: [] },
      }),
    );
    expect(out.kind).toBe("notInstalled");
  });

  it("CLI installed but no client wired → .notWiredToAnyClient", () => {
    const out = reduce(
      baseInputs({
        clientDetect: { total: 4, wiredCount: 0, wiredLabels: [] },
      }),
    );
    expect(out.kind).toBe("notWiredToAnyClient");
  });

  it("detector hasn't completed (total=0) — show .notWiredToAnyClient, not .error", () => {
    // Production-mode flip: with auto-bring-up disabled, a daemon
    // being unreachable is the EXPECTED state until a client invokes
    // `vaner mcp`. Briefly showing the "wire a client" panel during
    // the half-second the probe takes to complete is preferable to
    // flashing a scary engine-error.
    const out = reduce(
      baseInputs({
        clientDetect: { total: 0, wiredCount: 0, wiredLabels: [] },
        status: reachableStatus({ reachable: false }),
      }),
    );
    expect(out.kind).toBe("notWiredToAnyClient");
  });

  it("clients wired but engine unreachable → .error (now actionable)", () => {
    const out = reduce(
      baseInputs({
        clientDetect: { total: 4, wiredCount: 1, wiredLabels: ["Cursor"] },
        status: reachableStatus({ reachable: false }),
      }),
    );
    expect(out.kind).toBe("error");
  });

  it("engine unreachable with default (wired) inputs → .error", () => {
    const out = reduce(baseInputs({ status: reachableStatus({ reachable: false }) }));
    expect(out.kind).toBe("error");
  });

  it("Ollama not installed → .ollamaMissing (overrides engine error)", () => {
    // Wired clients + engine unreachable would normally → .error;
    // .ollamaMissing should win because it names the actual cause.
    const out = reduce(
      baseInputs({
        status: reachableStatus({ reachable: false }),
        ollamaHealth: { installed: false, running: false, detail: "Ollama isn't installed." },
      }),
    );
    expect(out.kind).toBe("ollamaMissing");
    if (out.kind === "ollamaMissing") {
      expect(out.installed).toBe(false);
      expect(out.detail).toContain("Ollama");
    }
  });

  it("Ollama present but cliMissing wins → .notInstalled", () => {
    // The Vaner CLI itself missing is a more fundamental problem
    // than Ollama being absent — keep .notInstalled at the top of
    // the precedence chain.
    const out = reduce(
      baseInputs({
        status: reachableStatus({ cliMissing: true, reachable: false }),
        ollamaHealth: { installed: false, running: false, detail: "" },
      }),
    );
    expect(out.kind).toBe("notInstalled");
  });

  it("Ollama installed but the cockpit is silent → .error (engine, not Ollama)", () => {
    const out = reduce(
      baseInputs({
        status: reachableStatus({ reachable: false }),
        ollamaHealth: { installed: true, running: true, detail: "" },
      }),
    );
    expect(out.kind).toBe("error");
  });

  it("blocked sources → .permissionNeeded (even if otherwise prepared)", () => {
    const out = reduce(
      baseInputs({
        blockedSources: [blockedSrc()],
        prepared: { ...emptyPrepared(), lead: aMoment("a") },
      }),
    );
    expect(out.kind).toBe("permissionNeeded");
  });

  it("no sources configured → .installedNotConnected", () => {
    const out = reduce(baseInputs({ hasAnySource: false }));
    expect(out.kind).toBe("installedNotConnected");
  });

  it("indexing learning → .learning", () => {
    const out = reduce(
      baseInputs({
        status: reachableStatus({
          indexing: { kind: "learning", currentlyReading: [], etaMinutes: 10 },
        }),
      }),
    );
    expect(out.kind).toBe("learning");
  });

  it("ready prediction + agent running → .activePredictions", () => {
    const out = reduce(
      baseInputs({
        activePredictions: [aPrediction("ready", 0.8)],
      }),
    );
    expect(out.kind).toBe("activePredictions");
  });

  it("prepared work is the primary user-facing surface", () => {
    const out = reduce(
      baseInputs({
        preparedWork: [aPreparedWorkCard()],
        activePredictions: [aPrediction("ready", 0.8)],
        prepared: { lead: aMoment("a"), supporting: [], pendingWhenNoAgent: 0 },
        anyAgentRunning: false,
      }),
    );
    expect(out.kind).toBe("preparedWork");
    if (out.kind === "preparedWork") {
      expect(out.cards[0].id).toBe("work_product:wp");
    }
  });

  it("ready prediction + no agent → .noActiveAgent", () => {
    const out = reduce(
      baseInputs({
        anyAgentRunning: false,
        activePredictions: [aPrediction("ready"), aPrediction("drafting")],
        noAgentSuggestions: [fakeAgent("Cursor")],
      }),
    );
    expect(out.kind).toBe("noActiveAgent");
    if (out.kind === "noActiveAgent") {
      expect(out.pendingCount).toBe(2);
    }
  });

  it("queued predictions are filtered out (not surfacable)", () => {
    const out = reduce(
      baseInputs({
        activePredictions: [aPrediction("queued")],
      }),
    );
    expect(out.kind).toBe("watching"); // falls through
  });

  it("activePredictions sorts ready before drafting, then by confidence desc", () => {
    const out = reduce(
      baseInputs({
        activePredictions: [
          aPrediction("drafting", 0.9),
          aPrediction("ready", 0.6),
          aPrediction("ready", 0.8),
          aPrediction("drafting", 0.7),
        ],
      }),
    );
    if (out.kind !== "activePredictions") throw new Error("expected activePredictions");
    const ord = out.predictions.map((p) => `${p.run.readiness}-${p.spec.confidence}`);
    expect(ord).toEqual(["ready-0.8", "ready-0.6", "drafting-0.9", "drafting-0.7"]);
  });

  it("prepared moment + agent → .prepared", () => {
    const out = reduce(
      baseInputs({
        prepared: { lead: aMoment("a"), supporting: [aMoment("b")], pendingWhenNoAgent: 0 },
      }),
    );
    expect(out.kind).toBe("prepared");
    if (out.kind === "prepared") {
      expect(out.lead.id).toBe("a");
      expect(out.supporting).toHaveLength(1);
    }
  });

  it("prepared moment + no agent → .noActiveAgent (pendingCount = lead + supporting)", () => {
    const out = reduce(
      baseInputs({
        anyAgentRunning: false,
        prepared: {
          lead: aMoment("a"),
          supporting: [aMoment("b"), aMoment("c")],
          pendingWhenNoAgent: 0,
        },
        noAgentSuggestions: [fakeAgent("Claude Desktop")],
      }),
    );
    expect(out.kind).toBe("noActiveAgent");
    if (out.kind === "noActiveAgent") {
      expect(out.pendingCount).toBe(3);
    }
  });

  it("no signals → .watching", () => {
    const out = reduce(baseInputs());
    expect(out.kind).toBe("watching");
    if (out.kind === "watching") {
      expect(out.silentHours).toBe(false);
      expect(out.context.clientLabel).toBe("Cursor");
      expect(out.context.workspaceLabel).toBe("vaner");
      expect(out.context.signalLabels).toContain("MCP context");
    }
  });

  it("no signals + silent hours → .watching with flag", () => {
    const out = reduce(baseInputs({ silentHours: true }));
    expect(out.kind).toBe("watching");
    if (out.kind === "watching") {
      expect(out.silentHours).toBe(true);
    }
  });

  it("paused → .paused with queued count = supporting + lead + adoptable predictions", () => {
    const out = reduce(
      baseInputs({
        paused: true,
        prepared: { lead: aMoment("a"), supporting: [aMoment("b"), aMoment("c")], pendingWhenNoAgent: 0 },
        activePredictions: [aPrediction("ready"), aPrediction("queued")],
      }),
    );
    expect(out.kind).toBe("paused");
    if (out.kind === "paused") {
      // 1 ready prediction + 1 lead + 2 supporting; queued prediction filtered out.
      expect(out.queued).toBe(4);
    }
  });

  it("paused does NOT override .error (urgent state wins)", () => {
    const out = reduce(
      baseInputs({
        paused: true,
        status: reachableStatus({ reachable: false }),
      }),
    );
    expect(out.kind).toBe("error");
  });

  it("paused does NOT override .permissionNeeded", () => {
    const out = reduce(
      baseInputs({
        paused: true,
        blockedSources: [blockedSrc()],
      }),
    );
    expect(out.kind).toBe("permissionNeeded");
  });

  it("paused does NOT override .installedNotConnected (engine has nothing to pause yet)", () => {
    const out = reduce(baseInputs({ paused: true, hasAnySource: false }));
    expect(out.kind).toBe("installedNotConnected");
  });
});
