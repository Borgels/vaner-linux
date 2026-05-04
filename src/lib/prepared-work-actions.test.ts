import { describe, expect, it, vi } from "vitest";
import type { PreparedWorkAction, PreparedWorkCard } from "$lib/contract/types.js";
import { canRunPreparedWorkAction, dispatchPreparedWorkAction } from "./prepared-work-actions.js";

const action = (overrides: Partial<PreparedWorkAction>): PreparedWorkAction => ({
  kind: "inspect",
  label: "Inspect",
  tool: null,
  endpoint: "/work-products/wp/inspect",
  arguments: {},
  ...overrides,
});

const card = (overrides: Partial<PreparedWorkCard>): PreparedWorkCard => ({
  id: "work_product:wp",
  source_id: "wp",
  source_type: "work_product",
  kind: "brief",
  title: "Prepared item",
  summary: "Summary",
  badge: "Brief",
  confidence_label: "High",
  freshness_label: "Fresh",
  target_label: "target",
  evidence_count: 1,
  created_at: 1,
  updated_at: 1,
  primary_action: null,
  secondary_actions: [],
  ...overrides,
});

describe("prepared-work action dispatch", () => {
  it("adopts prediction cards through adopt_prediction", async () => {
    const invoke = vi.fn().mockResolvedValue("Adopted intent");
    const prediction = card({
      id: "prediction:p-1",
      source_id: "p-1",
      source_type: "prediction",
      kind: "prediction",
    });
    const adopt = action({
      kind: "adopt",
      label: "Adopt",
      tool: "vaner.predictions.adopt",
      endpoint: "/predictions/p-1/adopt",
      arguments: { prediction_id: "p-1" },
    });

    const result = await dispatchPreparedWorkAction(invoke, prediction, adopt);

    expect(invoke).toHaveBeenCalledWith("adopt_prediction", { predictionId: "p-1" });
    expect(result).toEqual({ kind: "adopt", message: "Adopted intent" });
  });

  it("runs work-product actions through prepared_work_action", async () => {
    const invoke = vi.fn().mockResolvedValue({ ok: true });
    const work = card({});
    const inspect = action({ kind: "inspect", endpoint: "/work-products/wp/inspect" });

    const result = await dispatchPreparedWorkAction(invoke, work, inspect);

    expect(invoke).toHaveBeenCalledWith("prepared_work_action", {
      endpoint: "/work-products/wp/inspect",
      kind: "inspect",
      arguments: {},
    });
    expect(result).toEqual({ kind: "prepared_work_action", value: { ok: true } });
  });

  it("does not route prediction inspect through the work-product action command", async () => {
    const invoke = vi.fn();
    const prediction = card({
      id: "prediction:p-1",
      source_id: "p-1",
      source_type: "prediction",
      kind: "prediction",
    });
    const inspect = action({
      kind: "inspect",
      endpoint: "/predictions/p-1",
    });

    expect(canRunPreparedWorkAction(prediction, inspect)).toBe(false);
    const result = await dispatchPreparedWorkAction(invoke, prediction, inspect);

    expect(invoke).not.toHaveBeenCalled();
    expect(result.kind).toBe("unsupported");
  });
});
