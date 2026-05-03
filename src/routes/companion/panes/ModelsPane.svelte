<!--
  Models pane — the full backend picker. Linux equivalent of
  vaner-desktop-macos `Companion/ModelsPane.swift`.

  Responsibilities:
    1. Default to Ollama (local) — Vaner is local-first and Ollama is
       a hard prerequisite for the local path. A fresh `.vaner/config
       .toml` (empty `backend.base_url`) gets pinned to Ollama on
       first open of this pane, no toast, no click.
    2. Let the user switch between Ollama and a Custom OpenAI-
       compatible endpoint. Power users who want a hosted provider
       (OpenAI / Anthropic / etc.) point a Custom endpoint at it via
       the Advanced disclosure — we don't surface them as preset
       cards because the local-first story is the supported path.
    3. When Ollama is the active provider, surface the installed-
       model list + suggested pulls (the ModelsCard component).
    4. Advanced disclosure exposes the underlying TOML keys (name,
       base_url, model, api_key_env) for self-hosted endpoints.
    5. The cost-banner still fires when classifyBackend recognises a
       known cloud URL the user has wired through Custom — kept as a
       guard rail so a copy-paste of an OpenAI URL isn't silent.
    6. Hardware summary moved to a footer section.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import DocsLink from "$lib/components/primitives/DocsLink.svelte";
  import V1Headline from "$lib/components/primitives/V1Headline.svelte";
  import V1Body from "$lib/components/primitives/V1Body.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import VSectionLabel from "$lib/components/primitives/VSectionLabel.svelte";
  import { setup, loadHardware } from "$lib/stores/setup.js";
  import {
    applyBackendPreset,
    backendConfig,
    classifyBackend,
    loadBackendConfig,
    setBackendKey,
    type BackendPreset,
  } from "$lib/stores/backend-config.js";
  import { showToast } from "$lib/stores/toast.js";
  import ModelsCard from "$lib/components/ModelsCard.svelte";

  // Local-first: only Ollama and Custom are surfaced as picker
  // entries. Cloud providers (OpenAI / Anthropic / hosted services)
  // are still reachable — power users point a Custom endpoint at
  // their provider's OpenAI-compatible URL — but they're not the
  // headline path. The default is always Ollama; new installs land
  // on it without a click (see onMount below).
  const PRESETS: {
    id: BackendPreset;
    title: string;
    subtitle: string;
    accent: string;
  }[] = [
    {
      id: "ollama",
      title: "Ollama (local)",
      subtitle: "Local model on your machine. Default.",
      accent: "var(--vd-st-active)",
    },
    {
      id: "custom",
      title: "Custom endpoint",
      subtitle: "Any OpenAI-compatible URL. Configure under Advanced.",
      accent: "var(--vd-fg-3)",
    },
  ];

  // Track whichever preset is mid-flight so the row can show a
  // subtle pending indicator without disabling the rest of the
  // picker. Switching providers takes ~4 sequential `vaner config
  // set` calls; greying out every button during that window felt
  // like a UI freeze. Letting clicks queue + the store update
  // optimistically (active = matches new preset before the writes
  // finish) is closer to what the user expects.
  let pending = $state<BackendPreset | null>(null);
  let savingAdv = $state(false);
  let showAdvanced = $state(false);
  // Suppress the picker until the first config load resolves.
  // Without this the pane briefly renders with nothing selected
  // (because $backendConfig is null on first paint) and only flips
  // to the right preset after the async invoke returns.
  let hydrated = $state(false);

  // Per-field edit state for the Custom preset / Advanced rows.
  let advName = $state("");
  let advBaseUrl = $state("");
  let advModel = $state("");
  let advApiKeyEnv = $state("");
  let advDirty = $state(false);

  const active = $derived(classifyBackend($backendConfig));

  $effect(() => {
    // When the live config changes, refresh the advanced editors so
    // they show the current persisted values whenever the disclosure
    // is opened.
    const cfg = $backendConfig;
    if (cfg && !advDirty) {
      advName = cfg.name;
      advBaseUrl = cfg.base_url;
      advModel = cfg.model;
      advApiKeyEnv = cfg.api_key_env;
    }
  });

  async function selectPreset(preset: BackendPreset) {
    if (preset === active) return;
    pending = preset;
    try {
      await applyBackendPreset(preset);
      const label = PRESETS.find((p) => p.id === preset)?.title ?? preset;
      showToast(`Switched to ${label}.`, "success", 2500);
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Could not switch backend: ${err}`,
        "attention",
        5000,
      );
    } finally {
      // Clear only if this is still the most-recent click. A second
      // click while the first was in flight overrides `pending`, so
      // we don't want to flip back to null while the user's latest
      // choice is still mid-write.
      if (pending === preset) pending = null;
    }
  }

  async function saveAdvanced() {
    if (savingAdv || !advDirty) return;
    savingAdv = true;
    try {
      await setBackendKey("name", advName);
      await setBackendKey("base_url", advBaseUrl);
      await setBackendKey("model", advModel);
      await setBackendKey("api_key_env", advApiKeyEnv);
      advDirty = false;
      showToast("Backend updated.", "success", 2500);
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Could not save backend: ${err}`,
        "attention",
        5000,
      );
    } finally {
      savingAdv = false;
    }
  }

  onMount(async () => {
    loadHardware();
    const cfg = await loadBackendConfig();
    // Local-first default: a fresh `.vaner/config.toml` ships an
    // empty `backend.base_url` (the CLI's default scaffolding leaves
    // it blank). When that's the case, silently pin Ollama as the
    // backend so the picker shows it as the active provider rather
    // than landing on Custom-by-default. No toast — this is part of
    // "Vaner is local-first by default", not a notable user action.
    if (cfg && cfg.base_url.trim() === "") {
      try {
        await applyBackendPreset("ollama");
      } catch {
        // Best-effort. If the daemon's config write fails the picker
        // still renders correctly off whatever the CLI returned —
        // the user just won't see Ollama highlighted until they
        // click it.
      }
    }
    hydrated = true;
  });

  const hw = $derived($setup.hardware);
  const detectedModels = $derived(hw?.detected_models ?? []);
</script>

<header class="hd">
  <div class="kicker-row">
    <V1Kicker text="Models" />
    <DocsLink path="/backends" />
  </div>
  <V1Headline text="Pick which model Vaner calls" size={22} />
</header>

{#if active === "openai" || active === "anthropic"}
  <div class="cost-banner" role="alert">
    <strong>Vaner is using a cloud model.</strong>
    Every ponder cycle may call {active === "openai" ? "OpenAI" : "Anthropic"}.
    Watch your provider's spend limits.
  </div>
{/if}

<section class="block">
  <VSectionLabel text="Provider" />
  {#if !hydrated}
    <div class="presets">
      {#each PRESETS as p (p.id)}
        <div class="preset skeleton" style:--accent={p.accent}>
          <div class="preset-head">
            <span class="preset-title">{p.title}</span>
          </div>
          <span class="preset-sub">{p.subtitle}</span>
        </div>
      {/each}
    </div>
  {:else}
    <div class="presets">
      {#each PRESETS as p (p.id)}
        {@const selected = active === p.id}
        {@const isPending = pending === p.id}
        <button
          type="button"
          class="preset"
          class:selected
          style:--accent={p.accent}
          onclick={() => selectPreset(p.id)}
        >
          <div class="preset-head">
            <span class="preset-title">{p.title}</span>
            {#if isPending}
              <span class="preset-badge">SWITCHING…</span>
            {:else if selected}
              <span class="preset-badge">ACTIVE</span>
            {/if}
          </div>
          <span class="preset-sub">{p.subtitle}</span>
        </button>
      {/each}
    </div>
  {/if}
</section>

{#if active === "ollama"}
  <!-- Ollama installed-models list + pull/remove. The card is the
       same one that used to live in Preferences. -->
  <ModelsCard />
{/if}

<section class="block">
  <button class="adv-toggle" type="button" onclick={() => (showAdvanced = !showAdvanced)}>
    <span aria-hidden="true">{showAdvanced ? "▾" : "▸"}</span>
    <span>Advanced</span>
  </button>
  {#if showAdvanced}
    <div class="adv">
      <label class="field">
        <span>name</span>
        <input
          type="text"
          bind:value={advName}
          oninput={() => (advDirty = true)}
          disabled={savingAdv}
        />
      </label>
      <label class="field">
        <span>base_url</span>
        <input
          type="text"
          bind:value={advBaseUrl}
          oninput={() => (advDirty = true)}
          disabled={savingAdv}
          placeholder="https://…/v1"
        />
      </label>
      <label class="field">
        <span>model</span>
        <input
          type="text"
          bind:value={advModel}
          oninput={() => (advDirty = true)}
          disabled={savingAdv}
          placeholder="model-id"
        />
      </label>
      <label class="field">
        <span>api_key_env</span>
        <input
          type="text"
          bind:value={advApiKeyEnv}
          oninput={() => (advDirty = true)}
          disabled={savingAdv}
          placeholder="MY_API_KEY"
        />
      </label>
      <div class="actions">
        <V1PrimaryButton
          title={savingAdv ? "Saving…" : "Save"}
          onclick={saveAdvanced}
        />
        <V1GhostButton
          title="Revert"
          onclick={() => {
            const cfg = $backendConfig;
            if (cfg) {
              advName = cfg.name;
              advBaseUrl = cfg.base_url;
              advModel = cfg.model;
              advApiKeyEnv = cfg.api_key_env;
            }
            advDirty = false;
          }}
        />
      </div>
    </div>
  {/if}
</section>

<section class="block">
  <VSectionLabel text="Detected hardware" />
  {#if hw}
    <div class="kv">
      <span>Tier</span><span>{hw.tier}</span>
      <span>OS</span><span>{hw.os}</span>
      <span>CPU class</span><span>{hw.cpu_class}</span>
      <span>RAM</span><span>{hw.ram_gb} GB</span>
      <span>GPU</span>
      <span>
        {#if hw.gpu_devices && hw.gpu_devices.length > 0}
          {hw.gpu_devices.map((d) => `${d.name}${d.memory_display_gb ? ` (${d.memory_display_gb} GB)` : ""}`).join(", ")}
        {:else}
          {hw.gpu}{hw.gpu_vram_gb ? ` · ${hw.gpu_vram_gb} GB VRAM` : ""}
        {/if}
      </span>
      <span>Runtimes</span><span>{hw.detected_runtimes.join(", ") || "—"}</span>
    </div>
    {#if detectedModels.length > 0}
      <p class="muted-row">
        {detectedModels.length}
        {detectedModels.length === 1 ? "model" : "models"} detected on disk.
      </p>
    {/if}
  {/if}
</section>

<style>
  .hd { display: flex; flex-direction: column; gap: 6px; margin-bottom: 24px; }
  .kicker-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .block { margin-bottom: 22px; }

  .cost-banner {
    margin-bottom: 18px;
    padding: 12px 14px;
    background: color-mix(in srgb, var(--vd-amber) 8%, var(--vd-bg-1));
    border: 0.5px solid color-mix(in srgb, var(--vd-amber) 40%, transparent);
    border-radius: var(--vd-r-card);
    font-size: 12px;
    color: var(--vd-fg-2);
    line-height: 1.5;
  }
  .cost-banner strong {
    display: block;
    color: var(--vd-fg-1);
    font-weight: 600;
    margin-bottom: 2px;
  }

  .presets {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 10px;
  }
  .preset {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 16px;
    text-align: left;
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-card);
    cursor: pointer;
    color: var(--vd-fg-1);
    font: inherit;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .preset:hover:not(:disabled) { background: var(--vd-bg-2); }
  .preset:disabled { opacity: 0.5; cursor: not-allowed; }
  .preset.selected {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 8%, var(--vd-bg-1));
  }
  .preset.skeleton {
    opacity: 0.45;
    cursor: default;
    pointer-events: none;
  }
  .preset-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .preset-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--vd-fg-1);
  }
  .preset-badge {
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.1em;
    color: var(--accent);
  }
  .preset-sub {
    font-size: 11.5px;
    color: var(--vd-fg-3);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 8px;
  }

  .adv-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    padding: 4px 0;
    color: var(--vd-fg-3);
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .adv-toggle:hover { color: var(--vd-fg-2); }
  .adv {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-top: 8px;
  }
  .field {
    display: grid;
    grid-template-columns: 100px 1fr;
    align-items: center;
    gap: 10px;
  }
  .field span {
    font-size: 11px;
    color: var(--vd-fg-3);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-family: var(--vd-font-mono);
  }
  .field input {
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid var(--vd-hair);
    border-radius: 6px;
    padding: 7px 10px;
    color: var(--vd-fg-1);
    font-family: var(--vd-font-mono);
    font-size: 12px;
  }
  .field input:focus { outline: 1px solid var(--vd-amber); border-color: transparent; }
  .field input:disabled { opacity: 0.6; cursor: not-allowed; }

  .kv {
    margin-top: 10px;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    font-family: var(--vd-font);
    font-size: 12px;
  }
  .kv > span:nth-child(odd) {
    color: var(--vd-fg-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 10.5px;
    padding-top: 2px;
  }
  .kv > span:nth-child(even) {
    color: var(--vd-fg-1);
    font-family: var(--vd-font-mono);
  }
  .muted-row {
    margin: 10px 0 0;
    font-size: 11px;
    color: var(--vd-fg-3);
  }
</style>
