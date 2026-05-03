//! Live engine status — shells `vaner status --json` and projects the
//! result into the reducer's `EngineStatus` shape. The `vaner status`
//! output is rich and growing; we extract only the fields the reducer
//! consumes today and ignore the rest. Missing fields fall back to
//! safe defaults so an older `vaner` CLI doesn't crash the popover.
//!
//! The Svelte side polls this command every few seconds via the
//! `engineStatus` store. SSE for status would be nicer; that's a v0.3
//! follow-up once the daemon ships `/events/stream?stages=status`.

use serde::Serialize;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

use crate::vaner_cli::resolve_vaner_bin;

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatusOut {
    pub reachable: bool,
    /// `true` when the `vaner` CLI itself isn't installed (i.e. the
    /// resolution lookup failed). Used by the reducer to distinguish
    /// "engine unreachable" (daemon down, but CLI present) from "fresh
    /// install, no engine on disk yet". Without this, a clean
    /// `vaner-desktop` install on a machine that's never seen the CLI
    /// shows a misleading "Engine error" panel.
    pub cli_missing: bool,
    pub files_watched: u64,
    pub sources_count: u64,
    pub uptime_minutes: u64,
    pub indexing_kind: String, // "idle" | "learning" | "reindexing"
    pub detail: Option<String>,
}

/// Tauri command — returns the cached snapshot the background poller
/// in [`crate::engine_status_task`] keeps fresh. Every window's
/// `engineStatus` store reads through this, so they all see the same
/// boolean at the same time. The `engine:status` event the task
/// emits is the push counterpart for live UIs.
#[tauri::command]
pub async fn engine_status(
    cache: tauri::State<'_, std::sync::Arc<crate::engine_status_task::EngineStatusCache>>,
) -> Result<EngineStatusOut, String> {
    Ok(cache.snapshot().await)
}

/// Bypasses the cache and shells `vaner status` directly. Used by the
/// background poller and by callers that need a freshness guarantee
/// (post-bring-up). Pure function over `vaner status --json`.
pub async fn probe_engine_status() -> EngineStatusOut {
    let bin = match resolve_vaner_bin() {
        Ok(path) => path,
        Err(message) => {
            return EngineStatusOut {
                reachable: false,
                cli_missing: true,
                files_watched: 0,
                sources_count: 0,
                uptime_minutes: 0,
                indexing_kind: "idle".to_string(),
                detail: Some(message),
            };
        }
    };
    let Some(workspace) = crate::workspace::resolve() else {
        return EngineStatusOut {
            reachable: false,
            cli_missing: false,
            files_watched: 0,
            sources_count: 0,
            uptime_minutes: 0,
            indexing_kind: "idle".to_string(),
            detail: Some("No Vaner workspace selected.".to_string()),
        };
    };
    let workspace = workspace.to_string_lossy().into_owned();
    let output = match Command::new(&bin)
        .arg("status")
        .arg("--json")
        .arg("--path")
        .arg(&workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            return EngineStatusOut {
                reachable: false,
                cli_missing: false,
                files_watched: 0,
                sources_count: 0,
                uptime_minutes: 0,
                indexing_kind: "idle".to_string(),
                detail: Some(format!("failed to spawn `vaner status`: {e}")),
            };
        }
    };

    if !output.status.success() {
        // CLI exists but `vaner status` returned non-zero — daemon is
        // down or in a degraded state. The reducer surfaces .error
        // (actionable: restart engine).
        return EngineStatusOut {
            reachable: false,
            cli_missing: false,
            files_watched: 0,
            sources_count: 0,
            uptime_minutes: 0,
            indexing_kind: "idle".to_string(),
            detail: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        };
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            return EngineStatusOut {
                reachable: false,
                cli_missing: false,
                files_watched: 0,
                sources_count: 0,
                uptime_minutes: 0,
                indexing_kind: "idle".to_string(),
                detail: Some(format!("could not parse status JSON: {e}")),
            };
        }
    };

    let reachable = parsed
        .get("cockpit")
        .and_then(|c| c.get("reachable"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let detail = parsed
        .get("cockpit")
        .and_then(|c| c.get("detail"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // `vaner status --json` doesn't expose files/sources directly today;
    // best-effort projection from related fields. The reducer is happy
    // with zeros — it falls through to .installedNotConnected if there
    // are no sources, or .watching once setup populates them.
    let scenarios_ready = parsed
        .get("scenarios_ready")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    EngineStatusOut {
        reachable,
        cli_missing: false,
        files_watched: scenarios_ready, // approximation; honest signal of "engine has read N things"
        sources_count: 0,               // populated by the Svelte side from setup_status
        uptime_minutes: 0,              // not exposed by vaner status today; v0.3 follow-up
        // `vaner status --json` doesn't differentiate idle vs learning
        // today; report idle and let the daemon's future SSE status
        // events upgrade us. The reducer's .learning branch is wired
        // and ready for the day this changes.
        indexing_kind: "idle".into(),
        detail,
    }
}
