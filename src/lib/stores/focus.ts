import { invoke } from "@tauri-apps/api/core";
import { writable, type Readable } from "svelte/store";

export type FocusState = {
  mode: string;
  status: string;
  resource_mode: string;
  focus_epoch: number;
  active_workspace_id?: string | null;
  selected_by: string;
  explanation: string;
  why_not: Array<{ reason_code?: string; message?: string; action?: string }>;
  workspaces: Array<{
    id: string;
    display_name: string;
    tier: string;
    pinned: boolean;
    paused: boolean;
    focus_confidence: number;
  }>;
  detected_clients: Array<{
    id: string;
    display_name: string;
    running: boolean;
    integration_state: string;
  }>;
};

export type FocusRouteState = {
  effective_route: {
    workspace?: {
      id: string;
      display_name: string;
      canonical_path?: string | null;
      selected?: boolean;
      pinned?: boolean;
      paused?: boolean;
    } | null;
    client?: {
      id: string;
      display_name: string;
      running?: boolean;
      selected?: boolean;
      preferred?: boolean;
    } | null;
    workspace_policy: "auto" | "work_here" | "pinned" | string;
    automatic: boolean;
    resource_mode: string;
    device: string;
    backend: {
      name: string;
      base_url: string;
      model: string;
      api_key_env: string;
    };
    selected_by: string;
    expires_at?: number | null;
  };
  workspace_options: Array<{
    id: string;
    display_name: string;
    canonical_path?: string | null;
    selected: boolean;
    pinned: boolean;
    paused: boolean;
    eligible: boolean;
  }>;
  client_options: Array<{
    id: string;
    display_name: string;
    running: boolean;
    selected: boolean;
    preferred: boolean;
    eligible: boolean;
    integration_state: string;
  }>;
  hardware_options: {
    resource_modes: Array<{ id: string; label: string }>;
    devices: Array<{ id: string; name: string; kind?: string; explanation?: string }>;
    runtimes: Array<{ id: string; kind: string; endpoint?: string | null; healthy?: boolean }>;
    models: Array<{ id: string; name: string; runtime_id: string; size_label?: string }>;
    current: {
      resource_mode: string;
      device: string;
      backend: {
        name: string;
        base_url: string;
        model: string;
        api_key_env: string;
      };
    };
  };
  diagnostics: {
    why_not?: Array<{ reason_code?: string; message?: string; action?: string }>;
    raw_detected_clients?: FocusState["detected_clients"];
    warnings?: Array<{ code?: string; message?: string }>;
  };
  explanation: string;
};

export type FocusRouteUpdate = {
  workspace_policy?: "auto" | "work_here" | "pinned";
  workspace_path?: string;
  client_id?: string | null;
  resource_mode?: string;
  compute_device?: string;
  backend?: Partial<FocusRouteState["effective_route"]["backend"]>;
  ttl_seconds?: number;
};

const fallback: FocusState = {
  mode: "auto",
  status: "idle",
  resource_mode: "balanced",
  focus_epoch: 0,
  selected_by: "unavailable",
  explanation: "Vaner focus is unavailable because the daemon is not reachable.",
  why_not: [{ reason_code: "daemon_unavailable", message: "Start Vaner to see Auto Focus state." }],
  workspaces: [],
  detected_clients: [],
};

const routeFallback: FocusRouteState = {
  effective_route: {
    workspace: null,
    client: null,
    workspace_policy: "auto",
    automatic: true,
    resource_mode: "balanced",
    device: "auto",
    backend: { name: "", base_url: "", model: "", api_key_env: "" },
    selected_by: "unavailable",
    expires_at: null,
  },
  workspace_options: [],
  client_options: [],
  hardware_options: {
    resource_modes: [
      { id: "low_power", label: "Light" },
      { id: "balanced", label: "Balanced" },
      { id: "performance", label: "Fast" },
    ],
    devices: [],
    runtimes: [],
    models: [],
    current: { resource_mode: "balanced", device: "auto", backend: { name: "", base_url: "", model: "", api_key_env: "" } },
  },
  diagnostics: { why_not: fallback.why_not, raw_detected_clients: [], warnings: [] },
  explanation: "Workspace settings are unavailable because Vaner is not reachable.",
};

const { subscribe, set }: { subscribe: Readable<FocusState>["subscribe"]; set: (v: FocusState) => void } = writable(fallback);
const routeStore: { subscribe: Readable<FocusRouteState>["subscribe"]; set: (v: FocusRouteState) => void } = writable(routeFallback);

let timer: ReturnType<typeof setInterval> | null = null;
let focusInFlight: Promise<void> | null = null;
let routeInFlight: Promise<void> | null = null;

export async function refreshFocus(): Promise<void> {
  if (focusInFlight) return focusInFlight;
  focusInFlight = doRefreshFocus().finally(() => {
    focusInFlight = null;
  });
  return focusInFlight;
}

async function doRefreshFocus(): Promise<void> {
  try {
    set(await invoke<FocusState>("focus_status"));
  } catch (err) {
    console.warn("[vaner] focus_status failed:", err);
    set(fallback);
  }
}

export async function refreshFocusRoute(): Promise<void> {
  if (routeInFlight) return routeInFlight;
  routeInFlight = doRefreshFocusRoute().finally(() => {
    routeInFlight = null;
  });
  return routeInFlight;
}

async function doRefreshFocusRoute(): Promise<void> {
  try {
    routeStore.set(await invoke<FocusRouteState>("focus_route_status"));
  } catch (err) {
    console.warn("[vaner] focus_route_status failed:", err);
    routeStore.set(routeFallback);
  }
}

export async function updateFocusRoute(payload: FocusRouteUpdate): Promise<void> {
  try {
    routeStore.set(await invoke<FocusRouteState>("focus_route_update", { payload }));
    await refreshFocus();
  } catch (err) {
    console.warn("[vaner] focus_route_update failed:", err);
    await refreshFocusRoute();
  }
}

export async function focusAction(action: string, mode?: string): Promise<void> {
  try {
    set(await invoke<FocusState>("focus_action", { action, mode }));
  } catch (err) {
    console.warn("[vaner] focus_action failed:", err);
    await refreshFocus();
  }
}

export function startFocusPolling(): void {
  if (timer) return;
  void refreshFocus();
  void refreshFocusRoute();
  timer = setInterval(() => {
    void refreshFocus();
    void refreshFocusRoute();
  }, 5000);
}

export function stopFocusPolling(): void {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
}

export const focus = { subscribe };
export const focusRoute = { subscribe: routeStore.subscribe };
