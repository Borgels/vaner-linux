import { writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { showToast } from "./toast.js";

export type InstallKind = "deb" | "appimage" | "other";

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes?: string | null;
  /** What format the running binary was installed as. Kept for
   *  diagnostics/copy; the update action itself always goes through
   *  Tauri's signed updater. */
  installKind: InstallKind;
}

/** `null` until a newer release is found. */
export const availableUpdate = writable<UpdateInfo | null>(null);
/** 0–1 when an install is in progress, `null` otherwise. */
export const updateProgress = writable<number | null>(null);

let unlisteners: UnlistenFn[] = [];
let booted = false;

export async function bootstrapUpdaterListeners(): Promise<void> {
  if (booted) return;
  booted = true;

  unlisteners.push(
    await listen<{
      version: string;
      current_version: string;
      release_notes: string | null;
      install_kind: InstallKind;
    }>("update:available", ({ payload }) => {
      availableUpdate.set({
        version: payload.version,
        currentVersion: payload.current_version,
        notes: payload.release_notes,
        installKind: payload.install_kind,
      });
    }),
  );

  unlisteners.push(
    await listen<number>("update:progress", ({ payload }) => {
      updateProgress.set(payload);
    }),
  );

  unlisteners.push(
    await listen<void>("update:ready-to-restart", () => {
      updateProgress.set(1);
      showToast(
        "Update installed. Restarting Vaner.",
        "success",
        8000,
      );
    }),
  );
}

export async function installUpdate(): Promise<void> {
  updateProgress.set(0);
  try {
    await invoke("install_update");
  } catch (err) {
    updateProgress.set(null);
    const msg = typeof err === "string" ? err : "Update install failed.";
    showToast(msg, "attention", 5000);
  }
}

/** Open the vaner.ai download page for `version`. Error fallback only;
 *  normal updates use the signed in-app updater. */
export async function openReleasePage(version: string): Promise<void> {
  try {
    await invoke("update_open_release", { version });
  } catch (err) {
    const msg = typeof err === "string" ? err : "Could not open the download page.";
    showToast(msg, "attention", 4000);
  }
}
