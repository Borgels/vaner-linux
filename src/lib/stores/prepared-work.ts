import { derived, writable, type Readable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { PreparedWorkCard } from "$lib/contract/types.js";

type PreparedWorkState = {
  cards: PreparedWorkCard[];
  loading: boolean;
  error: string | null;
};

const state = writable<PreparedWorkState>({
  cards: [],
  loading: false,
  error: null,
});

let started = false;
let timer: ReturnType<typeof setInterval> | null = null;
let inFlight: Promise<void> | null = null;

function message(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
}

export async function refreshPreparedWork(limit = 8): Promise<void> {
  if (inFlight) return inFlight;
  inFlight = doRefreshPreparedWork(limit).finally(() => {
    inFlight = null;
  });
  return inFlight;
}

async function doRefreshPreparedWork(limit = 8): Promise<void> {
  state.update((current) => ({ ...current, loading: current.cards.length === 0 }));
  try {
    const cards = await invoke<PreparedWorkCard[]>("prepared_work", { limit });
    state.set({ cards, loading: false, error: null });
  } catch (err) {
    console.warn("[vaner] prepared_work failed:", err);
    state.set({ cards: [], loading: false, error: message(err) });
  }
}

export function startPreparedWorkPolling(limit = 8, intervalMs = 5000): void {
  if (started) return;
  started = true;
  void refreshPreparedWork(limit);
  timer = setInterval(() => void refreshPreparedWork(limit), intervalMs);
}

export function stopPreparedWorkPolling(): void {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
  started = false;
}

export const preparedWork: Readable<PreparedWorkCard[]> = derived(state, ($state) => $state.cards);
export const preparedWorkState: Readable<PreparedWorkState> = { subscribe: state.subscribe };
