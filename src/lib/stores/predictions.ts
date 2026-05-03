import { writable, type Readable } from "svelte/store";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { PredictedPrompt } from "$lib/contract/types";

// The Rust backend emits `predictions:snapshot` whenever the SSE
// stream delivers a new frame. This store mirrors the latest
// snapshot to every subscribed Svelte component.
const { subscribe, set }: { subscribe: Readable<PredictedPrompt[]>["subscribe"]; set: (v: PredictedPrompt[]) => void } =
  writable<PredictedPrompt[]>([]);

let unlisten: UnlistenFn | null = null;
let started = false;
let timer: ReturnType<typeof setInterval> | null = null;
let inFlight: Promise<void> | null = null;

async function refreshPredictions(limit = 24): Promise<void> {
  if (inFlight) return inFlight;
  inFlight = doRefreshPredictions(limit).finally(() => {
    inFlight = null;
  });
  return inFlight;
}

async function doRefreshPredictions(limit = 24): Promise<void> {
  try {
    const overview = await invoke<PredictedPrompt[]>("prediction_overview", { limit });
    set(overview.map(sanitizePrediction));
  } catch (overviewErr) {
    try {
      const initial = await invoke<PredictedPrompt[]>("active_predictions");
      set(initial.map(sanitizePrediction));
    } catch (activeErr) {
      console.warn("[vaner] prediction refresh failed:", overviewErr, activeErr);
    }
  }
}

/** Subscribe to live prediction snapshots. Safe to call multiple times. */
export async function startPredictionStream(): Promise<void> {
  if (started) return;
  started = true;

  // Pull a first overview so the UI can show queued/grounding predictions
  // from the worker snapshot before any adoptable SSE frame exists.
  void refreshPredictions();
  timer = setInterval(() => void refreshPredictions(), 5000);

  unlisten = await listen<PredictedPrompt[]>("predictions:snapshot", (event) => {
    if (event.payload.length > 0) {
      set(event.payload.map(sanitizePrediction));
    }
    void refreshPredictions();
  });
}

/** Tear the subscription down — tests / teardown only. */
export async function stopPredictionStream(): Promise<void> {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
  started = false;
}

export const predictions = { subscribe };

function sanitizePrediction(prediction: PredictedPrompt): PredictedPrompt {
  const sourceLabel = prediction.source_label ?? prediction.spec.source ?? "Vaner";
  const fallback = `${sourceLabel} prediction`;
  return {
    ...prediction,
    spec: {
      ...prediction.spec,
      label: safeText(prediction.spec.label, fallback),
      description: prediction.spec.description ? safeText(prediction.spec.description, `${sourceLabel} prediction is ready.`) : null,
      anchor: prediction.spec.anchor ? safeText(prediction.spec.anchor, "current flow") : null,
    },
    ui_summary: prediction.ui_summary ? safeText(prediction.ui_summary, `${sourceLabel} prediction is ready.`) : prediction.ui_summary,
  };
}

function safeText(value: string, fallback: string): string {
  const text = value.trim();
  if (!text) return fallback;
  if (looksLikeRawHistory(text)) return fallback;
  const collapsed = text.replace(/\s+/g, " ");
  return collapsed.length > 160 ? `${collapsed.slice(0, 159).trim()}…` : collapsed;
}

function looksLikeRawHistory(text: string): boolean {
  if (text.length > 260) return true;
  if (text.split(/\r?\n/).filter((line) => line.trim().length > 0).length > 1) return true;
  if ((text.match(/\?/g) ?? []).length >= 2) return true;
  const lower = ` ${text.toLowerCase()} `;
  if (lower.includes("recent queries clustered") || lower.includes("chat history")) return true;
  return [
    " i am ",
    " i'm ",
    " i'd ",
    " i've ",
    " we ",
    " we're ",
    " we've ",
    " you ",
    " your ",
    " why ",
    " shouldn't ",
    " couldn't ",
  ].some((marker) => lower.includes(marker));
}
