//! Auto-bring-up of the Vaner engine.
//!
//! Before v0.8.9 the desktop launched, polled `vaner status`, and if
//! the cockpit wasn't reachable showed a permanent "Engine unavailable"
//! panel — leaving the user to click _Restart engine_ manually (which
//! itself was a band-aid, not a fix). The *desktop* is what owns the
//! engine lifecycle now: on launch we probe the cockpit, and if it's
//! down we shell `vaner up --detach --path <workspace>` ourselves and
//! wait until `/healthz` answers.
//!
//! This module exposes:
//!
//! - [`spawn_at_startup`] — fire-and-forget background task the
//!   `tauri::Builder::setup` closure calls. Skips if no workspace has
//!   been picked yet (the popover surfaces the picker).
//! - [`ensure_engine_running`] — async helper the popover's
//!   `Restart engine` flow reuses, so the success path is one
//!   code path and the popover can observe completion via the
//!   returned [`BringUpResult`] instead of guessing.
//! - [`bring_up_engine`] — the matching `#[tauri::command]`.
//!
//! The probe is a 250ms-timeout `GET 127.0.0.1:8473/`. We use the bare
//! root rather than `/healthz` because the cockpit's `GET /` answers
//! 200 on the static index without needing the prediction surface to
//! be ready (`--with-engine` may still be initialising).

use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

const COCKPIT_HOST: &str = "127.0.0.1";
const COCKPIT_PORT: u16 = 8473;
/// Probe timeout per attempt. Short — the cockpit is loopback so a real
/// answer arrives in single-digit ms; anything longer means it's down.
const PROBE_TIMEOUT: Duration = Duration::from_millis(250);
/// Total budget for `ensure_engine_running` to wait after `vaner up`.
/// 10 seconds covers cold model-runtime warmup (Ollama enumeration,
/// scenario DB open) without leaving the popover hanging forever on a
/// truly broken install.
const STARTUP_BUDGET: Duration = Duration::from_secs(10);
/// Poll interval while we wait for the cockpit to answer post-bringup.
const POLL_INTERVAL: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BringUpOutcome {
    /// Cockpit was already up — no `vaner up` needed.
    AlreadyRunning,
    /// We shelled `vaner up --detach` and the cockpit answered before
    /// the budget expired.
    Started,
    /// We shelled `vaner up` but the cockpit still wasn't answering
    /// after `STARTUP_BUDGET`. Caller should surface `detail` to the
    /// user (it's already-redacted CLI stderr).
    Failed,
    /// No workspace picked yet — bringup is the user's call once they
    /// finish the picker. Not a failure.
    NoWorkspace,
}

#[derive(Debug, Clone, Serialize)]
pub struct BringUpResult {
    pub outcome: BringUpOutcome,
    /// Resolved workspace path the bringup targeted, if any.
    pub workspace: Option<String>,
    /// Human-readable explanation. Empty for `AlreadyRunning`.
    pub detail: String,
}

/// HTTP probe of the cockpit. Returns true on any 2xx/3xx — the cockpit
/// answers 200 on `/` even before `--with-engine` is fully online, and
/// that's enough for the popover to stop showing the error panel. The
/// daemon-status JSON poll downstream picks up engine readiness on its
/// own cadence.
async fn probe() -> bool {
    let url = format!("http://{COCKPIT_HOST}:{COCKPIT_PORT}/");
    let client = match reqwest::Client::builder().timeout(PROBE_TIMEOUT).build() {
        Ok(c) => c,
        Err(_) => return false,
    };
    matches!(client.get(&url).send().await, Ok(resp) if resp.status().is_success())
}

/// Idempotent. If the cockpit is already up: returns immediately. If
/// no workspace is set: returns `NoWorkspace` (the popover handles it).
/// Otherwise shells `vaner up --detach --path <workspace>` and waits
/// up to `STARTUP_BUDGET` for `/` to answer.
pub async fn ensure_engine_running() -> BringUpResult {
    if probe().await {
        return BringUpResult {
            outcome: BringUpOutcome::AlreadyRunning,
            workspace: crate::workspace::resolve().map(|p| p.to_string_lossy().into_owned()),
            detail: String::new(),
        };
    }

    // No fallback to $HOME: `vaner up --path <home>` rejects the
    // path because a home directory isn't a repo (no .git, no
    // workspace marker). The popover layer is responsible for not
    // surfacing a "Restart engine" CTA in that state — the right
    // CTA is "finish setup", not "try to start a daemon against a
    // non-repo and watch it bounce."
    let Some(workspace) = crate::workspace::resolve() else {
        return BringUpResult {
            outcome: BringUpOutcome::NoWorkspace,
            workspace: None,
            detail: "no workspace selected".to_string(),
        };
    };
    let workspace_str = workspace.to_string_lossy().into_owned();

    let bin = match crate::vaner_cli::resolve_vaner_bin() {
        Ok(p) => p,
        Err(e) => {
            return BringUpResult {
                outcome: BringUpOutcome::Failed,
                workspace: Some(workspace_str),
                detail: e,
            };
        }
    };

    // If the canonical loopback endpoint is silent, clear the workspace's
    // supervised runtime before starting it again. This handles stale
    // same-repo processes that still own 8473 but no longer answer HTTP; a
    // plain `vaner up` would otherwise auto-shift to a fallback port the
    // desktop does not use.
    let _ = down_run(&bin, &workspace_str).await;

    // Fire `vaner up --detach --json` first. New CLIs (≥ 0.8.9) emit
    // a single line of structured stdout we can use for specific
    // failure messages ("repo root looks wrong", "no [setup] section",
    // …). Older CLIs reject the unknown flag with a typer error and
    // we transparently retry without it — `up_run_legacy` matches the
    // pre-0.8.9 behaviour.
    let detail_from_cli = match up_run_json(&bin, &workspace_str).await {
        UpAttempt::Json(payload) => {
            // The CLI reported its result. Even on `started: false`
            // we still poll: some failure modes (port already bound
            // by another vaner) leave the cockpit healthy.
            if !payload.started && !payload.error.is_empty() {
                payload.error
            } else {
                String::new()
            }
        }
        UpAttempt::JsonRejected => up_run_legacy(&bin, &workspace_str).await,
        UpAttempt::SpawnError(e) => format!("could not spawn `vaner up`: {e}"),
    };

    // Poll until the cockpit answers or the budget runs out. We probe
    // even when `vaner up` reported failure, because some failure
    // modes (e.g. cockpit already bound by another instance) still
    // result in a healthy endpoint.
    let deadline = Instant::now() + STARTUP_BUDGET;
    while Instant::now() < deadline {
        if probe().await {
            return BringUpResult {
                outcome: BringUpOutcome::Started,
                workspace: Some(workspace_str),
                detail: String::new(),
            };
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    BringUpResult {
        outcome: BringUpOutcome::Failed,
        workspace: Some(workspace_str),
        detail: if detail_from_cli.is_empty() {
            "cockpit did not come up within 10 seconds".to_string()
        } else {
            detail_from_cli
        },
    }
}

/// Result shape from `vaner up --json` (success or failure).
/// Matches the canonical key set documented in the vaner repo's
/// `cli/commands/app.py::up`. Fields we don't surface to the user
/// (daemon_pid, cockpit_pid, ports, inotify) are still parsed so
/// the `serde_json::from_str` round-trip succeeds even when the CLI
/// adds new keys.
#[derive(Debug, Clone, Default, Deserialize)]
struct UpJsonPayload {
    #[serde(default)]
    started: bool,
    #[serde(default)]
    #[allow(dead_code)]
    reattached: bool,
    #[serde(default)]
    #[allow(dead_code)]
    ready: bool,
    #[serde(default)]
    error: String,
}

enum UpAttempt {
    /// CLI accepted `--json` and emitted parseable output.
    Json(UpJsonPayload),
    /// CLI rejected `--json` (older version). Caller should retry
    /// without the flag.
    JsonRejected,
    /// Spawn failed (CLI binary couldn't be executed). Surfacing the
    /// OS error directly is more useful than a generic "failed".
    SpawnError(std::io::Error),
}

async fn up_run_json(bin: &Path, workspace: &str) -> UpAttempt {
    let output = Command::new(bin)
        .arg("up")
        .arg("--detach")
        .arg("--json")
        .arg("--path")
        .arg(workspace)
        .output()
        .await;
    let output = match output {
        Ok(o) => o,
        Err(e) => return UpAttempt::SpawnError(e),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse stdout first. With `--json` the success path always emits
    // a single JSON line; the failure path emits a JSON error envelope
    // with exit 1. Either way, parseable JSON is the signal that the
    // CLI understood the flag.
    if let Some(line) = stdout.lines().find(|l| l.trim_start().starts_with('{')) {
        if let Ok(payload) = serde_json::from_str::<UpJsonPayload>(line) {
            return UpAttempt::Json(payload);
        }
    }

    // No parseable JSON. Either the CLI is too old (typer rejects
    // `--json` with a UsageError on stderr) or it crashed before
    // emitting anything. Detect the typer rejection via the
    // characteristic "No such option" / "Got unexpected extra
    // argument" / "Usage:" stderr lines so we don't misclassify a
    // real failure as "old CLI".
    let stderr_low = stderr.to_lowercase();
    let looks_like_unknown_flag = stderr_low.contains("no such option")
        || stderr_low.contains("unexpected extra argument")
        || stderr_low.contains("got unexpected")
        || (stderr_low.contains("usage:") && stderr_low.contains("--json"));
    if looks_like_unknown_flag {
        UpAttempt::JsonRejected
    } else {
        // CLI ran but didn't emit JSON. Hand back the stderr as the
        // detail; the synthetic payload stops the caller from
        // double-running `up`.
        UpAttempt::Json(UpJsonPayload {
            error: if stderr.trim().is_empty() {
                format!(
                    "vaner up exited with code {} and no JSON output",
                    output.status.code().unwrap_or(-1)
                )
            } else {
                stderr.trim().to_string()
            },
            ..Default::default()
        })
    }
}

/// Pre-0.8.9 fallback. Returns the stderr (possibly empty) so the
/// caller can use it as the `detail` field if the cockpit poll later
/// times out.
async fn up_run_legacy(bin: &Path, workspace: &str) -> String {
    let output = Command::new(bin)
        .arg("up")
        .arg("--detach")
        .arg("--path")
        .arg(workspace)
        .output()
        .await;
    match output {
        Ok(o) if o.status.success() => String::new(),
        Ok(o) => String::from_utf8_lossy(&o.stderr).trim().to_string(),
        Err(e) => format!("could not spawn `vaner up`: {e}"),
    }
}

async fn down_run(bin: &Path, workspace: &str) -> String {
    let output = Command::new(bin)
        .arg("down")
        .arg("--path")
        .arg(workspace)
        .output()
        .await;
    match output {
        Ok(o) if o.status.success() => String::new(),
        Ok(o) => String::from_utf8_lossy(&o.stderr).trim().to_string(),
        Err(e) => format!("could not spawn `vaner down`: {e}"),
    }
}

/// Background task launched from `tauri::Builder::setup`. Runs once
/// at startup. Emits an `engine:bring-up` event with the result so the
/// popover and Diagnostics pane can react. Failures are logged but
/// non-fatal — the popover surfaces the error panel either way.
pub fn spawn_at_startup(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let result = ensure_engine_running().await;
        if let BringUpOutcome::Failed = result.outcome {
            eprintln!(
                "[vaner-desktop] engine bring-up failed: {} (workspace={})",
                result.detail,
                result.workspace.as_deref().unwrap_or("<none>")
            );
        }
        let _ = app.emit("engine:bring-up", &result);
    });
}

/// `#[tauri::command]` form of [`ensure_engine_running`]. The popover's
/// `Restart engine` flow calls this instead of `diagnostics_restart_engine`
/// when it wants the observed-success path: receive the structured
/// `BringUpResult`, stop the local "Restarting…" spinner, and let the
/// reducer flip out of `.error` on the next status poll.
#[tauri::command]
pub async fn bring_up_engine() -> Result<BringUpResult, String> {
    Ok(ensure_engine_running().await)
}
