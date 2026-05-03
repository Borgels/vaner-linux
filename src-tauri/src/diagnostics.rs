use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

async fn run_vaner(args: &[&str], allow_nonzero: bool) -> Result<String, String> {
    let bin = crate::vaner_cli::resolve_vaner_bin()?;
    let output = Command::new(&bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to run vaner: {e}"))?;
    if !output.status.success() && !allow_nonzero {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!(
                "vaner exited with code {}",
                output.status.code().unwrap_or(-1)
            )
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn selected_workspace() -> Result<String, String> {
    crate::workspace::resolve()
        .map(|path| path.to_string_lossy().into_owned())
        .ok_or_else(|| "No Vaner workspace selected.".to_string())
}

#[tauri::command]
pub async fn diagnostics_status() -> Result<Value, String> {
    let path = selected_workspace()?;
    let stdout = run_vaner(&["status", "--json", "--path", &path], true).await?;
    serde_json::from_str::<Value>(&stdout).map_err(|e| format!("could not parse status JSON: {e}"))
}

#[tauri::command]
pub async fn diagnostics_runtime() -> Result<Value, String> {
    let cli_path = crate::vaner_cli::resolve_vaner_bin().ok();
    let cli_version = match run_vaner(&["--version"], true).await {
        Ok(text) => text.trim().to_string(),
        Err(err) => err,
    };
    Ok(serde_json::json!({
        "desktop_version": env!("CARGO_PKG_VERSION"),
        "current_exe": std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned()),
        "appimage": std::env::var("APPIMAGE").ok(),
        "local_build": crate::updater::updater_disabled(),
        "updater_disabled": crate::updater::updater_disabled(),
        "vaner_bin": cli_path.map(|p| p.to_string_lossy().into_owned()),
        "vaner_path_env": std::env::var("VANER_PATH").ok(),
        "workspace": crate::workspace::resolve().map(|p| p.to_string_lossy().into_owned()),
        "cli_version": cli_version,
        "cockpit_url": "http://127.0.0.1:8473",
    }))
}

#[tauri::command]
pub async fn diagnostics_doctor() -> Result<Value, String> {
    let path = selected_workspace()?;
    let stdout = run_vaner(&["doctor", "--json", "--path", &path], true).await?;
    serde_json::from_str::<Value>(&stdout).map_err(|e| format!("could not parse doctor JSON: {e}"))
}

#[tauri::command]
pub async fn diagnostics_restart_engine() -> Result<String, String> {
    let Some(path) = crate::workspace::resolve() else {
        return Err(
            "No Vaner workspace selected. Pick a workspace before restarting the engine."
                .to_string(),
        );
    };
    let path = path.to_string_lossy().into_owned();
    let _ = run_vaner(&["down", "--path", &path], true).await;
    let result = crate::bring_up::ensure_engine_running().await;
    serde_json::to_string_pretty(&result)
        .map_err(|e| format!("could not encode restart result: {e}"))
}

#[tauri::command]
pub async fn diagnostics_upgrade_engine() -> Result<String, String> {
    // `vaner upgrade` uses pipx/pip and doesn't take --path; it's a
    // package-level operation, not a workspace one.
    run_vaner(&["upgrade"], true)
        .await
        .map(|_| "Vaner engine update finished.".to_string())
}

/// Persist a local-model override via `vaner config set backend.model <id>`.
/// Called by the Light/Medium/Heavy switcher in the recommended-setup card
/// after `setup_apply` has written the policy bundle. The CLI handles
/// loading/persisting `.vaner/config.toml` so we don't touch it from Rust.
#[tauri::command]
pub async fn set_local_model(model_id: String) -> Result<String, String> {
    if model_id.trim().is_empty() {
        return Err("model_id is required".to_string());
    }
    let path = selected_workspace()?;
    run_vaner(
        &["config", "set", "--path", &path, "backend.model", &model_id],
        true,
    )
    .await
    .map(|_| format!("backend.model set to {model_id}"))
}
