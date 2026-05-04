//! Tauri commands exposed to the Svelte frontend.
//!
//! Each command is a thin wrapper over a [`vaner_contract::EngineClient`]
//! method. Errors are converted to strings for the `invoke` boundary
//! (Tauri serializes `Err` values as JSON).

use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use vaner_contract::{EngineClient, EngineClientError, PredictedPrompt, stash_adopt};

use crate::AppState;
use crate::prepared_work_endpoint::validate_prepared_work_endpoint;

fn loopback_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("could not build HTTP client: {e}"))
}

/// `Quit` from the companion sidebar (and any other "exit Vaner"
/// affordance the UI surfaces). The companion's `window.close()`
/// only hides the companion webview — it does not exit the app —
/// which made the existing button look broken.
#[tauri::command]
pub fn app_quit(app: AppHandle) -> tauri::Result<()> {
    app.exit(0);
    Ok(())
}

/// Hide just the current window (the companion). Wired separately
/// from `app_quit` so the sidebar can offer both behaviours: a
/// "Close" affordance that hides the window and a "Quit" that
/// terminates the app.
#[tauri::command]
pub fn window_hide(app: AppHandle, label: String) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window(&label) {
        win.hide()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn active_predictions(
    state: State<'_, AppState>,
) -> Result<Vec<PredictedPrompt>, String> {
    state.engine.active_predictions().await.map_err(human)
}

#[tauri::command]
pub async fn prediction_overview(limit: Option<u16>) -> Result<Vec<serde_json::Value>, String> {
    let capped = limit.unwrap_or(24).clamp(1, 50) as usize;
    let body: serde_json::Value = loopback_client()?
        .get("http://127.0.0.1:8473/predictions/active?include_all=true")
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("prediction overview request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("prediction overview decode failed: {e}"))?;

    let mut seen = std::collections::HashSet::<String>::new();
    let mut rows = Vec::<serde_json::Value>::new();
    let mut push_row = |value: &serde_json::Value| {
        let Some(id) = value.get("id").and_then(|id| id.as_str()) else {
            return;
        };
        if seen.insert(id.to_string()) {
            rows.push(value.clone());
        }
    };

    if let Some(predictions) = body.get("predictions").and_then(|value| value.as_array()) {
        for prediction in predictions {
            push_row(prediction);
        }
    }
    if let Some(by_state) = body.get("by_state").and_then(|value| value.as_object()) {
        for readiness in [
            "ready",
            "drafting",
            "evidence_gathering",
            "grounding",
            "queued",
            "stale",
        ] {
            if let Some(group) = by_state.get(readiness).and_then(|value| value.as_array()) {
                for prediction in group {
                    push_row(prediction);
                }
            }
        }
    }

    fn readiness_rank(value: &serde_json::Value) -> u8 {
        match value
            .get("run")
            .and_then(|run| run.get("readiness"))
            .and_then(|readiness| readiness.as_str())
            .unwrap_or("queued")
        {
            "ready" => 0,
            "drafting" => 1,
            "evidence_gathering" => 2,
            "grounding" => 3,
            "queued" => 4,
            "stale" => 5,
            _ => 6,
        }
    }
    fn confidence(value: &serde_json::Value) -> f64 {
        value
            .get("spec")
            .and_then(|spec| spec.get("confidence"))
            .and_then(|confidence| confidence.as_f64())
            .unwrap_or(0.0)
    }
    fn updated_at(value: &serde_json::Value) -> f64 {
        value
            .get("run")
            .and_then(|run| run.get("updated_at"))
            .and_then(|updated_at| updated_at.as_f64())
            .unwrap_or(0.0)
    }

    rows.sort_by(|lhs, rhs| {
        readiness_rank(lhs)
            .cmp(&readiness_rank(rhs))
            .then_with(|| {
                confidence(rhs)
                    .partial_cmp(&confidence(lhs))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                updated_at(rhs)
                    .partial_cmp(&updated_at(lhs))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    rows.truncate(capped);
    Ok(rows)
}

#[tauri::command]
pub async fn prepared_work(limit: Option<u16>) -> Result<Vec<serde_json::Value>, String> {
    let capped = limit.unwrap_or(3).clamp(1, 12);
    let url = format!("http://127.0.0.1:8473/prepared-work?surface=desktop&limit={capped}");
    let body: serde_json::Value = loopback_client()?
        .get(url)
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("prepared-work request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("prepared-work decode failed: {e}"))?;
    Ok(body
        .get("prepared_work")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub async fn prepared_work_action(
    endpoint: String,
    kind: String,
    arguments: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    validate_prepared_work_endpoint(&endpoint).map_err(str::to_string)?;
    let url = format!("http://127.0.0.1:8473{endpoint}");
    let client = loopback_client()?;
    let request = if kind == "inspect" {
        client.get(url)
    } else if kind == "feedback" {
        let feedback_state = arguments
            .as_ref()
            .and_then(|value| value.get("feedback_state"))
            .and_then(|value| value.as_str())
            .unwrap_or("useful");
        client
            .post(url)
            .json(&serde_json::json!({ "feedback_state": feedback_state }))
    } else {
        client.post(url)
    };
    request
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("prepared work action failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("prepared work response decode failed: {e}"))
}

#[tauri::command]
pub async fn focus_status() -> Result<serde_json::Value, String> {
    loopback_client()?
        .get("http://127.0.0.1:8473/focus")
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("focus request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("focus decode failed: {e}"))
}

#[tauri::command]
pub async fn focus_route_status() -> Result<serde_json::Value, String> {
    loopback_client()?
        .get("http://127.0.0.1:8473/focus/route")
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("focus route request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("focus route decode failed: {e}"))
}

#[tauri::command]
pub async fn focus_route_update(payload: serde_json::Value) -> Result<serde_json::Value, String> {
    let body = payload.as_object().cloned().unwrap_or_default();
    loopback_client()?
        .post("http://127.0.0.1:8473/focus/route")
        .json(&body)
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("focus route update failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("focus route decode failed: {e}"))
}

#[tauri::command]
pub async fn focus_action(
    action: String,
    path: Option<String>,
    mode: Option<String>,
) -> Result<serde_json::Value, String> {
    let endpoint = match action.as_str() {
        "work_here" => "/focus/workspaces/current/work-here",
        "pin" => "/focus/workspaces/current/pin",
        "unpin" => "/focus/workspaces/current/unpin",
        "pause" => "/focus/workspaces/current/pause",
        "resume" => "/focus/workspaces/current/resume",
        "pause_all" => "/focus/pause-all",
        "mode" => "/focus/mode",
        _ => return Err("unknown focus action".to_string()),
    };
    let mut payload = serde_json::Map::new();
    if let Some(path) = path {
        payload.insert("path".to_string(), serde_json::Value::String(path));
    }
    if let Some(mode) = mode {
        payload.insert("mode".to_string(), serde_json::Value::String(mode));
    }
    loopback_client()?
        .post(format!("http://127.0.0.1:8473{endpoint}"))
        .json(&payload)
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("focus action failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("focus decode failed: {e}"))
}

#[tauri::command]
pub async fn resources_status() -> Result<serde_json::Value, String> {
    loopback_client()?
        .get("http://127.0.0.1:8473/resources")
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("resources request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("resources decode failed: {e}"))
}

#[tauri::command]
pub async fn jobs_status() -> Result<serde_json::Value, String> {
    loopback_client()?
        .get("http://127.0.0.1:8473/jobs")
        .send()
        .await
        .map_err(|_| "Vaner is unreachable. Is the daemon running?".to_string())?
        .error_for_status()
        .map_err(|e| format!("jobs request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("jobs decode failed: {e}"))
}

/// Adopt flow:
///  1. POST `/predictions/{id}/adopt` to the daemon.
///  2. Stash the full Resolution (+ raw bytes for unknown server keys)
///     at `$XDG_STATE_HOME/vaner/pending-adopt.json` via
///     `vaner_contract::stash_adopt`.
///  3. Copy a paste-fallback payload to the clipboard
///     (`predicted_response ?? prepared_briefing ?? intent`).
///  4. Return the short intent string so the frontend can toast it.
#[tauri::command]
pub async fn adopt_prediction(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    prediction_id: String,
) -> Result<String, String> {
    let (resolution, raw) = state.engine.adopt(&prediction_id).await.map_err(human)?;

    // File-drop on a blocking thread — JSONSerialization + fs::rename
    // shouldn't block the main task.
    let raw_bytes = raw.to_vec();
    let resolution_for_stash = resolution.clone();
    let stash_result =
        tokio::task::spawn_blocking(move || stash_adopt(&resolution_for_stash, &raw_bytes))
            .await
            .map_err(|e| format!("handoff task join error: {e}"))?;
    stash_result.map_err(|e| format!("handoff stash failed: {e}"))?;

    let clipboard_body = resolution
        .predicted_response
        .clone()
        .or_else(|| resolution.prepared_briefing.clone())
        .unwrap_or_else(|| resolution.intent.clone());
    // Writing the clipboard on the main actor is cheap; no detach.
    app.clipboard()
        .write_text(clipboard_body)
        .map_err(|e| format!("clipboard write failed: {e}"))?;

    Ok(resolution.intent)
}

fn human(err: EngineClientError) -> String {
    match err {
        EngineClientError::NotFound => "That prediction is no longer active.".into(),
        EngineClientError::EngineUnavailable => {
            "Vaner can't reach the prediction engine right now.".into()
        }
        EngineClientError::InvalidInput => "Invalid prediction.".into(),
        EngineClientError::Transport(_) => "Vaner is unreachable. Is the daemon running?".into(),
        other => format!("{other}"),
    }
}

/// Open an arbitrary URL in the user’s default browser via `xdg-open`.
/// Used by popover states and Preferences cards that link to external
/// docs (`docs.vaner.ai/integrations/connect-your-client`, the
/// release page when the apt mirror is unreachable, etc.). The
/// updater banner has its own narrower wrapper around this; this is
/// the general one for non-version-specific URLs.
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        // Tight gate to keep this from being repurposed as a generic
        // shell-out. We only ever want to open https / http URLs.
        return Err("only http(s) URLs are allowed".to_string());
    }
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open URL: {e}"))
}
