<!--
  SetupWizard — the multi-slide setup flow used by both the dedicated
  onboarding window (/onboarding) and the in-app re-runnable wizard
  route (/setup).

  Two flows on shared slide indices:

    Default (fast):
      0  Welcome
      1  Work styles                       (multi-select chips)
      4  Recommended model + optional installed-model override
      5  Done

  First-run does not ask normal users to choose runtimes, models,
  quantization, backends, or compute devices. Core Vaner recommends a
  local model from the hardware profile and this optional work-type
  hint; users who already have an Ollama model pulled can select it.

  `onComplete` is called only when the user clicks a final action.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    setup,
    loadQuestions,
    loadStatus,
    loadHardware,
    loadModelRecommendation,
    recommend,
    apply,
    type ModelsRecommendedPayload,
  } from "$lib/stores/setup.js";
  import { showToast } from "$lib/stores/toast.js";
  import { invoke } from "@tauri-apps/api/core";
  import VMark from "$lib/components/primitives/VMark.svelte";
  import V1Kicker from "$lib/components/primitives/V1Kicker.svelte";
  import V1PrimaryButton from "$lib/components/primitives/V1PrimaryButton.svelte";
  import V1GhostButton from "$lib/components/primitives/V1GhostButton.svelte";
  import Spinner from "$lib/components/primitives/Spinner.svelte";
  import RecommendedPresetCard from "$lib/components/RecommendedPresetCard.svelte";
  import WizardVerificationPanel from "$lib/components/WizardVerificationPanel.svelte";
  import type {
    BackgroundPosture,
    CloudPosture,
    ComputePosture,
    Priority,
    SelectionResult,
    SetupAnswers,
    SetupQuestion,
    WorkStyle,
  } from "$lib/contract/setup-types.js";

  type Props = {
    /** Called after a successful setup_apply (post cloud-widening
     *  confirm if applicable). The parent dispatches the right
     *  exit behavior — close the onboarding window, or goto('/'). */
    onComplete: () => void | Promise<void>;
    /** Called when the user dismisses the wizard. */
    onSkip: () => void | Promise<void>;
  };
  const { onComplete, onSkip }: Props = $props();

  // Slide indices (first-run skips 2 + 3; those legacy slides are kept
  // unreachable until the old custom-question UI is fully deleted):
  //   0 Welcome · 1 Work styles · 2 Priority · 3 Energy ·
  //   4 Model recommendation · 5 Apply / Done
  const TOTAL_SLIDES = 6;
  let slide = $state(0);

  let questions = $state<SetupQuestion[]>([]);
  let workStyles = $state<WorkStyle[]>(["mixed"]);
  let priority = $state<Priority>("balanced");
  // Energy collapses (compute_posture, background_posture) into one
  // user-facing question. Each value maps deterministically to a
  // (compute, background) pair. The daemon enums never see "energy";
  // we resolve this at submit time.
  type Energy = "light" | "balanced" | "burst" | "use_machine";
  let energy = $state<Energy>("balanced");
  const ENERGY_TO_POSTURES: Record<
    Energy,
    { compute: ComputePosture; background: BackgroundPosture }
  > = {
    light: { compute: "light", background: "minimal" },
    balanced: { compute: "balanced", background: "normal" },
    burst: { compute: "balanced", background: "idle_more" },
    use_machine: { compute: "available_power", background: "deep_run_aggressive" },
  };
  // Cloud posture is hidden from the wizard — see header comment.
  // Hardcoded to `local_only`; cloud opt-in lives in Preferences.
  const CLOUD_POSTURE_DEFAULT: CloudPosture = "local_only";
  let recommendation = $state<SelectionResult | null>(null);
  let recommending = $state(false);
  let modelRecommendation = $state<ModelsRecommendedPayload | null>(null);
  let modelRecommending = $state(false);
  let applying = $state(false);
  let widening = $state<{ id: string; reasons: string[] } | null>(null);
  let appliedBundleId = $state<string | null>(null);
  let applyError = $state<string | null>(null);
  // Override id from the recommendation review. When non-null and
  // different from the registry's automatic pick, we persist it via
  // `vaner config set backend.model` after the policy bundle is written.
  let selectedModelId = $state<string | null>(null);

  const hardware = $derived($setup.hardware);

  // Trim the daemon's option lists to the wizard's intended set.
  // The full enums stay valid on the daemon side — the in-app
  // /setup re-runnable surface and Preferences both still expose
  // every choice. The wizard hides the redundant ones.
  const WORK_STYLE_KEEP = new Set<WorkStyle>([
    "coding",
    "writing",
    "research",
    "planning",
    "support",
    "learning",
    "mixed",
  ]);
  const PRIORITY_KEEP = new Set<Priority>([
    "balanced",
    "speed",
    "quality",
    "privacy",
    "cost",
  ]);

  onMount(async () => {
    questions = await loadQuestions();
    await loadStatus();
    await loadHardware();
  });

  function getQuestion(id: string): SetupQuestion | undefined {
    return questions.find((q) => q.id === id);
  }

  function answers(): SetupAnswers {
    const postures = ENERGY_TO_POSTURES[energy];
    return {
      work_styles: workStyles.length === 0 ? ["mixed"] : workStyles,
      priority,
      compute_posture: postures.compute,
      cloud_posture: CLOUD_POSTURE_DEFAULT,
      background_posture: postures.background,
    };
  }

  function toggleWorkStyle(v: WorkStyle) {
    workStyles = workStyles.includes(v)
      ? workStyles.filter((x) => x !== v)
      : [...workStyles, v];
  }

  async function loadRecommendations() {
    recommending = true;
    modelRecommending = true;
    try {
      const [sel, modelRec] = await Promise.all([
        recommend(answers()),
        loadModelRecommendation(workStyles),
      ]);
      recommendation = sel;
      modelRecommendation = modelRec;
      if (sel == null) {
        // recommend() swallows errors and returns null; the wizard
        // would otherwise sit silently on the click. Surface the
        // last error from the setup store so the user knows what's
        // wrong (typically: "vaner setup exited with code N: …").
        const detail = $setup.lastError ?? "Could not generate a recommendation.";
        showToast(detail, "attention", 6000);
      }
      return sel != null;
    } finally {
      recommending = false;
      modelRecommending = false;
    }
  }

  async function nextSlide() {
    // Slide 1 (Work styles): load policy + model recommendations, then
    // let the user accept the hardware pick or choose an installed model.
    if (slide === 1) {
      const ok = await loadRecommendations();
      if (ok) {
        slide = 4;
      }
      return;
    }
    if (slide === 4) {
      slide = 5;
      void doApply();
      return;
    }
    slide = Math.min(slide + 1, TOTAL_SLIDES - 1);
  }

  function prevSlide() {
    if (slide === 0) return;
    if (slide === 4) {
      slide = 1;
      return;
    }
    slide -= 1;
  }

  function backToRecommendation() {
    applyError = null;
    applyErrorStep = null;
    slide = recommendation || modelRecommendation ? 4 : 1;
  }

  // Sequentialized apply with per-step status. Each step renders its own
  // pending/running/done/error chip in the checklist. `applyError` /
  // `applyErrorStep` track which step failed so Retry resumes from there
  // instead of re-running the whole flow (e.g. don't re-write the bundle
  // if it already succeeded and only the engine probe failed).
  type StepId = "save_config" | "set_model" | "engine_ready";
  type StepStatus = "pending" | "running" | "done" | "error" | "skipped";
  type StepState = { id: StepId; label: string; status: StepStatus };

  const DEFAULT_STEPS: StepState[] = [
    { id: "save_config", label: "Save Vaner settings", status: "pending" },
    { id: "set_model", label: "Apply model preference", status: "pending" },
    { id: "engine_ready", label: "Confirm Vaner is ready", status: "pending" },
  ];
  let steps = $state<StepState[]>(DEFAULT_STEPS.map((s) => ({ ...s })));
  let applyErrorStep = $state<StepId | null>(null);

  function setStepStatus(id: StepId, status: StepStatus) {
    steps = steps.map((s) => (s.id === id ? { ...s, status } : s));
  }

  function autoModelId(): string | null {
    const auto = modelRecommendation?.user?.selected_model ?? modelRecommendation?.selected;
    return auto?.model_id ?? auto?.id ?? null;
  }

  type ExistingModelOption = {
    runtime: string;
    modelId: string;
    size: string;
  };

  const installedModels = $derived.by<ExistingModelOption[]>(() => {
    const seen = new Set<string>();
    const rows: ExistingModelOption[] = [];
    for (const row of hardware?.detected_models ?? []) {
      const [runtime, modelId, size] = row;
      if (!modelId || runtime !== "ollama" || seen.has(modelId)) continue;
      seen.add(modelId);
      rows.push({ runtime, modelId, size });
    }
    return rows;
  });

  function selectedModelDisplay(): string {
    const auto = autoModelId();
    if (!selectedModelId || selectedModelId === auto) return "Vaner recommendation";
    return selectedModelId;
  }

  async function probeEngineReady(timeoutMs = 12_000): Promise<boolean> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      try {
        const result = await invoke<unknown>("diagnostics_status");
        if (typeof result === "object" && result !== null) {
          // The CLI's `vaner status --json` wraps health under .health or
          // .ok depending on version. Treat any non-error object as
          // reachable; the user will see the cockpit's own state if
          // something is off.
          const probe = result as Record<string, unknown>;
          if (probe.health || probe.ok || probe.status) return true;
          if (Object.keys(probe).length > 0) return true;
        }
      } catch {
        // Engine not yet up — keep polling.
      }
      await new Promise((r) => setTimeout(r, 700));
    }
    return false;
  }

  async function doApply(confirmWidening = false) {
    applying = true;
    applyError = null;
    applyErrorStep = null;

    // Reset any step that wasn't already done — Retry comes through here
    // and resumes from the failed step, but we also surface a fresh run.
    steps = steps.map((s) => (s.status === "done" ? s : { ...s, status: "pending" }));

    try {
      // Step 1 — write the policy bundle.
      if (steps.find((s) => s.id === "save_config")?.status !== "done") {
        setStepStatus("save_config", "running");
        const result = await apply({
          answers: answers(),
          confirm_cloud_widening: confirmWidening,
        });
        if (!result) {
          setStepStatus("save_config", "error");
          applyErrorStep = "save_config";
          applyError = "Setup did not return a result. Check diagnostics, then retry.";
          return;
        }
        if (result.widens_cloud_posture && !result.written) {
          // Park the bundle behind the widening confirm UI; not an error.
          setStepStatus("save_config", "pending");
          widening = { id: result.selected_bundle_id, reasons: result.reasons };
          return;
        }
        widening = null;
        appliedBundleId = result.selected_bundle_id;
        setStepStatus("save_config", "done");
      }

      // Step 2 — apply the user's model override (if it differs).
      const auto = autoModelId();
      if (selectedModelId && auto && selectedModelId !== auto) {
        setStepStatus("set_model", "running");
        try {
          await invoke<string>("set_local_model", { modelId: selectedModelId });
          setStepStatus("set_model", "done");
        } catch (err) {
          setStepStatus("set_model", "error");
          applyErrorStep = "set_model";
          applyError = err instanceof Error ? err.message : String(err);
          return;
        }
      } else {
        setStepStatus("set_model", "skipped");
      }

      // Step 3 — wait for the engine to actually answer.
      setStepStatus("engine_ready", "running");
      const ready = await probeEngineReady();
      if (!ready) {
        setStepStatus("engine_ready", "error");
        applyErrorStep = "engine_ready";
        applyError =
          "Vaner saved your settings but the engine is not answering yet. " +
          "Open Diagnostics and run 'Restart engine'.";
        return;
      }
      setStepStatus("engine_ready", "done");

      showToast("Vaner is ready", "success", 3500);
    } catch (err) {
      // Whichever step was running marks itself failed via setStepStatus
      // already; this is the unexpected branch.
      applyError = err instanceof Error ? err.message : String(err);
    } finally {
      applying = false;
    }
  }

  function resetStepsForRetry() {
    // Retry from scratch, except for steps that already succeeded — those
    // stay green so we don't re-write the bundle unnecessarily.
    applyError = null;
    applyErrorStep = null;
    steps = steps.map((s) => (s.status === "error" ? { ...s, status: "pending" } : s));
  }

  // ---------------------------------------------------------------------
  // Verification panel helpers
  // ---------------------------------------------------------------------

  /** Empty string lets the Tauri side resolve to its cwd default — same
   *  convention used by ``clients_install`` / ``clients_detect`` etc.
   *  See ``src/lib/stores/clients.ts::defaultRepoRoot``. */
  function repoRootForVerification(): string {
    return "";
  }

  async function repairClient(clientId: string): Promise<void> {
    try {
      await invoke<unknown>("clients_install", {
        repoRoot: "",
        clientId,
        force: true,
      });
      showToast(`Re-installed Vaner into ${clientId}`, "success", 2500);
    } catch (err) {
      showToast(
        err instanceof Error ? err.message : `Could not re-install ${clientId}`,
        "attention",
        3500,
      );
    }
  }

  // Per-slide button labels.
  const nextLabel = $derived(
    slide === 0
      ? "Get started"
      : slide === 1
        ? recommending || applying
          ? "Checking…"
          : "Continue"
        : slide === 4
          ? applying
            ? "Setting up…"
            : "Use this setup"
        : "Continue",
  );
  const nextDisabled = $derived(
    (slide === 1 && workStyles.length === 0) ||
      recommending ||
      applying ||
      (slide === 4 && modelRecommending),
  );

  // Build each question's choice list lazily so the chips can read
  // `findChoiceLabel("work_styles", "coding")` style. The wizard
  // filters the daemon's full option list down to the trimmed UI set
  // (see WORK_STYLE_KEEP / PRIORITY_KEEP at the top).
  function choices(qid: string) {
    return getQuestion(qid)?.choices ?? [];
  }
  function workStyleChoices() {
    return choices("work_styles").filter((c) =>
      WORK_STYLE_KEEP.has(c.value as WorkStyle),
    );
  }
  function priorityChoices() {
    return choices("priority").filter((c) => PRIORITY_KEEP.has(c.value as Priority));
  }

  // Energy choices are wizard-local — daemon does not have an "energy"
  // enum. Hard-coded labels here stay close to the macOS sibling.
  const ENERGY_CHOICES: Array<{ value: Energy; label: string; hint: string }> = [
    { value: "light", label: "Light", hint: "Barely use the CPU/GPU." },
    { value: "balanced", label: "Balanced", hint: "Work with what's idle (recommended)." },
    { value: "burst", label: "Burst when idle", hint: "Run broadly while the box is idle." },
    { value: "use_machine", label: "Use this machine", hint: "Cranked — happy to ponder overnight." },
  ];

  const dotCount = 4;
  // Visible-position-of-current-slide for the dots header.
  // Flow is 0 → 1 → 4 → 5.
  const dotIndex = $derived.by(() => {
    if (slide <= 1) return slide;
    if (slide === 4) return 2;
    return 3;
  });
</script>

<div class="wizard">
  <!-- Decorationless onboarding window: drag-handle strip lets the user
       move it. Wayland may still place initial position; this fixes the
       static-top-left feel after that. -->
  <div class="drag-handle" data-tauri-drag-region aria-hidden="true"></div>
  <!-- Step indicator (mode-aware: default = 4 dots, custom = 6 dots). -->
  <header class="dots">
    {#each Array.from({ length: dotCount }) as _, i (i)}
      <span class="dot" class:active={dotIndex >= i} class:current={dotIndex === i}></span>
    {/each}
  </header>

  <main class="slide-stage">
    {#if slide === 0}
      <!-- 0 · Welcome -->
      <section class="slide welcome">
        <VMark size={48} satelliteState="prepared" breathing />
        <V1Kicker text="Welcome" />
        <h1>Vaner sets itself up for this computer.</h1>
        <p class="lead">
          Pick what you will use Vaner for, then Vaner checks your machine
          and chooses the local setup for you.
        </p>
      </section>
    {:else if slide === 1}
      <!-- 1 · Work styles -->
      <section class="slide">
        <V1Kicker text="Optional" />
        <h1>{getQuestion("work_styles")?.prompt ?? "What kinds of work?"}</h1>
        <p class="lead">Pick all that apply, or keep Mixed.</p>
        <div class="chips multi">
          {#each workStyleChoices() as c (c.value)}
            <button
              type="button"
              class="chip"
              class:on={workStyles.includes(c.value as WorkStyle)}
              onclick={() => toggleWorkStyle(c.value as WorkStyle)}
            >
              <span>{c.label}</span>
              {#if c.hint}
                <span class="hint">{c.hint}</span>
              {/if}
            </button>
          {/each}
        </div>
      </section>
    {:else if slide === 2}
      <!-- 2 · Priority (custom-mode only) -->
      <section class="slide">
        <V1Kicker text="Question 2 of 3" />
        <h1>{getQuestion("priority")?.prompt ?? "What matters most?"}</h1>
        <div class="chips single">
          {#each priorityChoices() as c (c.value)}
            <button
              type="button"
              class="chip"
              class:on={priority === c.value}
              onclick={() => (priority = c.value as Priority)}
            >
              <span>{c.label}</span>
              {#if c.hint}<span class="hint">{c.hint}</span>{/if}
            </button>
          {/each}
        </div>
      </section>
    {:else if slide === 3}
      <!-- 3 · Energy (custom-mode only; merges compute + background) -->
      <section class="slide">
        <V1Kicker text="Question 3 of 3" />
        <h1>How hard should Vaner work?</h1>
        <p class="lead">
          One knob covering both foreground compute and idle-time
          pondering. Pick a wider setting in Preferences if you want
          them split apart.
        </p>
        <div class="chips single">
          {#each ENERGY_CHOICES as c (c.value)}
            <button
              type="button"
              class="chip"
              class:on={energy === c.value}
              onclick={() => (energy = c.value)}
            >
              <span>{c.label}</span>
              <span class="hint">{c.hint}</span>
            </button>
          {/each}
        </div>
      </section>
    {:else if slide === 4}
      <!-- 4 · Recommended model + existing-model override -->
      <section class="slide recommendation">
        <V1Kicker text="Recommended model" />
        <h1>Use Vaner’s hardware pick, or choose one you already have.</h1>
        <p class="lead">
          Vaner will configure Ollama for the recommended local model. If you
          already pulled another Ollama model, select it here.
        </p>

        <RecommendedPresetCard
          payload={modelRecommendation}
          loading={modelRecommending}
          hardware={hardware}
          bind:selectedModelId
        />

        {#if installedModels.length > 0}
          <div class="installed-models" role="radiogroup" aria-label="Installed Ollama models">
            <div class="installed-head">
              <span>Installed models</span>
              <span>{selectedModelDisplay()}</span>
            </div>
            {#each installedModels as model (model.modelId)}
              <button
                type="button"
                class="installed-row"
                class:on={selectedModelId === model.modelId}
                role="radio"
                aria-checked={selectedModelId === model.modelId}
                onclick={() => (selectedModelId = model.modelId)}
              >
                <code>{model.modelId}</code>
                {#if model.size}<span>{model.size}</span>{/if}
              </button>
            {/each}
            <button
              type="button"
              class="installed-row recommended-row"
              class:on={!selectedModelId || selectedModelId === autoModelId()}
              role="radio"
              aria-checked={!selectedModelId || selectedModelId === autoModelId()}
              onclick={() => (selectedModelId = autoModelId())}
            >
              <span>Use Vaner recommendation</span>
              {#if autoModelId()}<code>{autoModelId()}</code>{/if}
            </button>
          </div>
        {:else}
          <p class="hardware-line">
            No installed Ollama models were detected. Vaner will use the
            recommended model and guide the download if needed.
          </p>
        {/if}
      </section>
    {:else}
      <!-- 5 · Apply / Done. Widening branch is mostly dead code now
           that the onboarding wizard always submits local_only — but
           kept as a safety net in case the recommended bundle itself
           has a wider local_cloud_posture than the previous policy. -->
      <section class="slide done">
        {#if widening}
          <V1Kicker text="One more thing" color="var(--vd-amber)" />
          <h1>The recommended bundle would widen cloud access.</h1>
          <p class="lead">
            <strong>{widening.id}</strong> proposes a wider cloud posture
            than your current policy. The wizard normally submits
            <em>local_only</em> — pick how to proceed.
          </p>
          <div class="actions inline">
            <V1PrimaryButton title="Allow widening" tint="var(--vd-amber)" onclick={() => doApply(true)} />
            <V1GhostButton title="Keep local-only" onclick={() => { widening = null; slide = 1; }} />
          </div>
        {:else if applying || applyError}
          {@const isError = Boolean(applyError)}
          <V1Kicker
            text={isError ? "Setup needs attention" : "Setting up"}
            color="var(--vd-amber)"
          />
          <h1>{isError ? "Vaner could not finish setup." : "Vaner is getting ready."}</h1>
          <ol class="checklist">
            {#each steps as step (step.id)}
              <li class="step" data-status={step.status}>
                {#if step.status === "running"}
                  <Spinner size={14} />
                {:else if step.status === "done"}
                  <span class="check" aria-hidden="true">✓</span>
                {:else if step.status === "error"}
                  <span class="cross" aria-hidden="true">!</span>
                {:else if step.status === "skipped"}
                  <span class="dot" aria-hidden="true">·</span>
                {:else}
                  <span class="dot" aria-hidden="true">○</span>
                {/if}
                <span>{step.label}</span>
                {#if step.status === "skipped"}
                  <span class="muted">(no change needed)</span>
                {/if}
              </li>
            {/each}
          </ol>
          {#if applyError}
            <p class="lead">{applyError}</p>
            <div class="actions inline">
              <V1PrimaryButton
                title="Retry"
                tint="var(--vd-amber)"
                onclick={() => { resetStepsForRetry(); void doApply(false); }}
              />
              <V1GhostButton
                title="Open diagnostics"
                onclick={() => onSkip()}
              />
              <V1GhostButton
                title="Back"
                onclick={backToRecommendation}
              />
            </div>
          {/if}
        {:else if appliedBundleId}
          <VMark size={48} satelliteState="prepared" />
          <V1Kicker text="Ready" color="var(--vd-st-on)" />
          <h1>Vaner is ready.</h1>
          <p class="lead">
            Vaner has saved the recommended setup. Open the small Vaner
            window from the tray to see prepared work, switch agents, or
            pin the window while you work.
          </p>

          <WizardVerificationPanel
            repoRoot={repoRootForVerification()}
            mode="select"
            onRepair={(clientId) => repairClient(clientId)}
          />

          <div class="actions inline">
            <V1PrimaryButton
              title="Open Vaner"
              tint="var(--vd-amber)"
              onclick={async () => {
                // Open the companion immediately, then close the
                // onboarding window. Pre-fix the button only fired
                // `onComplete()` (which closes onboarding) and left
                // the user staring at an empty desktop wondering
                // where Vaner went. The companion is the natural
                // landing surface — Prepared + Agents + Engine
                // panes are exactly what someone who just finished
                // setup wants to see.
                try {
                  await invoke("open_companion", { tab: "prepared" });
                } catch {
                  // Best-effort: fall through to the close handler
                  // even if the companion couldn't open, so the user
                  // isn't trapped in onboarding.
                }
                onComplete();
              }}
            />
          </div>
        {:else}
          <div class="loading"><Spinner size={20} /><span>Preparing setup…</span></div>
        {/if}
      </section>
    {/if}
  </main>

  <!-- Footer controls -->
  <footer class="ctl">
    {#if slide < TOTAL_SLIDES - 1}
      <V1GhostButton title="Skip for now" onclick={() => onSkip()} />
    {/if}
    <span class="spacer"></span>
    {#if slide > 0 && slide < TOTAL_SLIDES - 1}
      <V1GhostButton title="Back" onclick={prevSlide} disabled={recommending || applying} />
    {/if}
    {#if slide < TOTAL_SLIDES - 1}
      <V1PrimaryButton
        title={nextLabel}
        tint={slide === 1 ? "var(--vd-amber)" : undefined}
        disabled={nextDisabled}
        onclick={nextSlide}
      />
    {/if}
  </footer>
</div>

<style>
  .wizard {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--vd-bg-0);
    color: var(--vd-fg-1);
    font-family: var(--vd-font);
    padding: 22px 36px 22px;
    overflow: hidden;
    position: relative;
  }
  .wizard > .drag-handle {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 14px;
    cursor: grab;
    -webkit-app-region: drag;
    z-index: 5;
  }
  .wizard > .drag-handle:active { cursor: grabbing; }
  .dots {
    display: flex;
    gap: 6px;
    align-items: center;
    flex: 0 0 auto;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--vd-line);
    transition: background 0.18s ease-out, transform 0.18s ease-out;
  }
  .dot.active {
    background: color-mix(in srgb, var(--vd-amber) 60%, var(--vd-fg-3));
  }
  .dot.current {
    background: var(--vd-amber);
    transform: scale(1.4);
  }

  .slide-stage {
    flex: 1 1 auto;
    display: flex;
    /* Top-align so longer slides (e.g. 8-option work_styles) don't
       push their h1 above the dots row. flex-start prevents the
       overlap; overflow-y: auto lets the slide scroll if the content
       still doesn't fit (rare). */
    align-items: flex-start;
    justify-content: center;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 16px 0 12px;
    min-height: 0;
    scrollbar-width: thin;
    scrollbar-color: var(--vd-line) transparent;
  }
  .slide-stage::-webkit-scrollbar {
    width: 6px;
  }
  .slide-stage::-webkit-scrollbar-thumb {
    background: var(--vd-line);
    border-radius: 3px;
  }
  .slide {
    max-width: 540px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .slide.welcome,
  .slide.done {
    gap: 14px;
    margin-top: 8px;
  }
  .slide h1 {
    margin: 2px 0 0;
    font-family: var(--vd-font);
    font-size: 22px;
    font-weight: 500;
    line-height: 1.22;
    letter-spacing: -0.014em;
    color: var(--vd-fg-1);
  }
  .slide .lead {
    margin: 0;
    font-size: 13px;
    color: var(--vd-fg-2);
    line-height: 1.55;
  }
  .slide .lead strong { font-weight: 500; color: var(--vd-fg-1); }
  .slide .lead em { font-style: italic; color: var(--vd-amber); }
  .slide .hardware-line {
    margin: 6px 0 0;
    font-family: var(--vd-font-mono, monospace);
    font-size: 11.5px;
    color: var(--vd-fg-3);
    line-height: 1.4;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }
  /* Multi-select (work_styles, 8 options) — pill-style chips that
     pack horizontally. The hint line is hidden on multi to keep the
     chip compact; macOS shows just the label on this question too. */
  .chips.multi {
    flex-direction: row;
  }
  .chips.multi .chip {
    flex: 0 0 auto;
    padding: 7px 12px;
    font-size: 12px;
    line-height: 1.25;
  }
  .chips.multi .chip .hint {
    display: none;
  }
  /* Single-select (priority, postures — 3-4 options each) — full-width
     stacked rows with the descriptive hint visible. */
  .chips.single {
    flex-direction: column;
    align-items: stretch;
  }
  .chip {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 9px 12px;
    background: var(--vd-bg-1);
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-chip);
    color: var(--vd-fg-1);
    font-family: var(--vd-font);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    transition: background 0.12s, border-color 0.12s;
  }
  .chip:hover {
    background: var(--vd-bg-2);
  }
  .chip.on {
    background: color-mix(in srgb, var(--vd-amber) 14%, transparent);
    border-color: color-mix(in srgb, var(--vd-amber) 50%, transparent);
  }
  .chip .hint {
    font-size: 11px;
    color: var(--vd-fg-3);
  }
  .chip.on .hint {
    color: var(--vd-fg-2);
  }

  .recommendation {
    max-width: 620px;
  }
  .installed-models {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
    padding-top: 10px;
    border-top: 0.5px solid var(--vd-hair);
  }
  .installed-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--vd-fg-3);
  }
  .installed-head span:last-child {
    text-transform: none;
    letter-spacing: 0;
    color: var(--vd-fg-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .installed-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 34px;
    padding: 8px 10px;
    border: 0.5px solid var(--vd-line);
    border-radius: var(--vd-r-chip);
    background: var(--vd-bg-1);
    color: var(--vd-fg-1);
    font-family: var(--vd-font);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .installed-row:hover {
    background: var(--vd-bg-2);
  }
  .installed-row.on {
    background: color-mix(in srgb, var(--vd-amber) 14%, transparent);
    border-color: color-mix(in srgb, var(--vd-amber) 50%, transparent);
  }
  .installed-row code {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--vd-font-mono, monospace);
    font-size: 11.5px;
    color: var(--vd-fg-1);
  }
  .installed-row span {
    color: var(--vd-fg-3);
    font-size: 11px;
  }
  .recommended-row span:first-child {
    color: var(--vd-fg-1);
    font-size: 12px;
  }

  .loading {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--vd-fg-2);
  }
  .actions.inline {
    display: flex;
    gap: 8px;
    margin-top: 6px;
    flex-wrap: wrap;
  }
  .checklist {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .checklist li {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--vd-fg-2);
    font-size: 13px;
  }
  .step[data-status="done"] { color: var(--vd-st-on, #6cc76c); }
  .step[data-status="error"] { color: var(--vd-st-attention, #e6b656); }
  .step[data-status="skipped"] { color: var(--vd-fg-3); }
  .step .check {
    display: inline-flex;
    width: 14px;
    height: 14px;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: var(--vd-st-on, #6cc76c);
    color: var(--vd-bg-0, #0e0e12);
    font-size: 10px;
    font-weight: 700;
  }
  .step .cross {
    display: inline-flex;
    width: 14px;
    height: 14px;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: var(--vd-st-attention, #e6b656);
    color: var(--vd-bg-0, #0e0e12);
    font-size: 10px;
    font-weight: 700;
  }
  .step .dot {
    display: inline-flex;
    width: 14px;
    height: 14px;
    align-items: center;
    justify-content: center;
    color: var(--vd-fg-4);
    font-size: 12px;
  }
  .step .muted { color: var(--vd-fg-4); font-size: 11px; }

  .ctl {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 12px;
    border-top: 0.5px solid var(--vd-hair);
  }
  .ctl .spacer {
    flex: 1 1 auto;
  }
</style>
