<!--
  DocsLink — small "docs ↗" link that opens a docs.vaner.ai page in
  the user's browser. Slotted into pane headers so users can jump
  from the Agents / Models / Engine / Preferences screens to the
  matching doc without hunting for it.

  Visual cue: the trailing ↗ arrow makes "this leaves the app" obvious
  at a glance, per the user's "should be easy to see that it would
  open in a browser" ask.
-->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { showToast } from "$lib/stores/toast.js";

  type Props = {
    /** Path under https://docs.vaner.ai (must start with `/`). Pages
     *  that don't exist 404 — keep this list in sync with the routes
     *  shipped in the vaner-docs repo. */
    path: string;
    /** Visible label. Defaults to "Docs" — short and recognisable. */
    label?: string;
  };

  const { path, label = "Docs" }: Props = $props();
  const docsPath = $derived(path.replace(/^\/docs(?=\/|$)/, ""));

  async function open() {
    const base = "https://docs.vaner.ai";
    const url = path.startsWith("http") ? path : `${base}${docsPath || "/"}`;
    try {
      await invoke("open_external_url", { url });
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err), "attention", 3500);
    }
  }
</script>

<button type="button" class="docs-link" onclick={open}>
  <span class="label">{label}</span>
  <span class="arrow" aria-hidden="true">↗</span>
</button>

<style>
  .docs-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: 0.5px solid var(--vd-line);
    border-radius: 999px;
    padding: 3px 9px;
    color: var(--vd-fg-3);
    font-family: var(--vd-font);
    font-size: 11px;
    cursor: pointer;
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .docs-link:hover {
    color: var(--vd-fg-1);
    border-color: var(--vd-fg-3);
    background: var(--vd-bg-2);
  }
  .label {
    letter-spacing: 0.02em;
  }
  .arrow {
    font-size: 10px;
    line-height: 1;
    transform: translateY(-0.5px);
    color: var(--vd-amber);
  }
</style>
