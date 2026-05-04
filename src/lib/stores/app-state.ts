import { get, writable } from "svelte/store";
import { emit, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { showToast } from "./toast.js";

/**
 * Global app-level flags driven by Tauri events fired from the tray
 * menu (`menu:toggle-pause`, `menu:open-preferences`) and startup
 * session detection (`setup:appindicator-missing`).
 *
 * The Svelte UI subscribes to these stores and reacts — no direct
 * access to Tauri events from components.
 */
const PAUSE_KEY = "vaner.pref.paused";

function loadPause(): boolean {
  try {
    return localStorage.getItem(PAUSE_KEY) === "true";
  } catch {
    return false;
  }
}

export const isPaused = writable<boolean>(loadPause());
isPaused.subscribe((p) => {
  try {
    localStorage.setItem(PAUSE_KEY, String(p));
  } catch {
    /* localStorage unavailable */
  }
  // Notify the Rust side so the tray menu can flip its row label
  // ("Pause Vaner" ↔ "Resume Vaner"). The popover's Resume button and
  // the tray's toggle item both feed into this same store, so a
  // single subscriber covers every input. emit returns a Promise
  // we deliberately discard — the tray label update is best-effort
  // (worst case the row stays stale until the next change).
  void emit("app:pause-changed", { paused: p });
});

export const needsAppIndicator = writable<boolean>(false);

let bootstrapped = false;

export async function setVanerPaused(paused: boolean): Promise<void> {
  isPaused.set(paused);
  try {
    await invoke("focus_action", { action: paused ? "pause_all" : "resume" });
  } catch (err) {
    console.warn("[vaner] pause/resume daemon action failed:", err);
    showToast(
      paused ? "Pause state saved, but daemon did not respond" : "Resume state saved, but daemon did not respond",
      "attention",
    );
  }
}

export async function bootstrapAppStateListeners(): Promise<void> {
  if (bootstrapped) return;
  bootstrapped = true;

  await listen<void>("menu:toggle-pause", () => {
    const next = !get(isPaused);
    showToast(next ? "Vaner paused" : "Vaner resumed", "info");
    void setVanerPaused(next);
  });

  await listen<void>("menu:open-preferences", async () => {
    // 0.8.5 WS12: navigate to /preferences (lands on the MCP Clients
    // tab). Loaded lazily so this store has no SvelteKit `goto`
    // dependency at module-load time.
    const { goto } = await import("$app/navigation");
    void goto("/preferences");
  });

  await listen<void>("setup:appindicator-missing", () => {
    needsAppIndicator.set(true);
  });
}
