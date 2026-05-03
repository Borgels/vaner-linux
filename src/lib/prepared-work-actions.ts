import type { PreparedWorkAction, PreparedWorkCard } from "$lib/contract/types.js";

type InvokeFn = <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>;

export type PreparedWorkDispatchResult =
  | { kind: "adopt"; message: string }
  | { kind: "prepared_work_action"; value: unknown }
  | { kind: "unsupported"; message: string };

function predictionId(card: PreparedWorkCard, action: PreparedWorkAction): string {
  const fromArgs = action.arguments?.prediction_id;
  return typeof fromArgs === "string" && fromArgs.trim() ? fromArgs : card.source_id;
}

export function canRunPreparedWorkAction(card: PreparedWorkCard, action: PreparedWorkAction | null): action is PreparedWorkAction {
  if (!action?.endpoint) return false;
  if (card.source_type === "prediction") return action.kind === "adopt";
  return action.endpoint.startsWith("/work-products/");
}

export async function dispatchPreparedWorkAction(
  invoke: InvokeFn,
  card: PreparedWorkCard,
  action: PreparedWorkAction,
): Promise<PreparedWorkDispatchResult> {
  if (card.source_type === "prediction") {
    if (action.kind !== "adopt") {
      return { kind: "unsupported", message: `${action.label} is not available for prediction cards yet.` };
    }
    const intent = await invoke<string>("adopt_prediction", { predictionId: predictionId(card, action) });
    return { kind: "adopt", message: intent };
  }

  if (!action.endpoint?.startsWith("/work-products/")) {
    return { kind: "unsupported", message: `${action.label} is not available for this card.` };
  }

  const value = await invoke("prepared_work_action", {
    endpoint: action.endpoint,
    kind: action.kind,
    arguments: action.arguments ?? {},
  });
  return { kind: "prepared_work_action", value };
}
