//! Tauri commands for the MCP Clients pane (0.8.5 WS12-D).
//!
//! Unlike the prediction commands in `commands.rs` (which talk HTTP to
//! the local daemon), the `clients_*` commands shell out to the
//! `vaner` CLI's `clients` subcommand introduced in Vaner 0.8.5 WS12-A.
//! That CLI is the single source of truth for per-client config paths,
//! atomic + backup-rotated writes, idempotent merges, and launcher
//! drift detection. All this Rust side does is parse the JSON output.
//!
//! Errors map to short human strings via `human_io_error` /
//! `human_subprocess_error` so the Svelte layer can toast them.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Wire types matching `vaner clients --format json`
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedClient {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub status: String, // "configured" / "installed" / "missing"
    pub detected: bool,
    pub configured: bool,
    pub config_path: Option<String>,
    pub detail: String,
}

#[derive(Debug, Deserialize)]
struct DetectResponse {
    clients: Vec<DetectedClient>,
}

/// One layer's outcome inside a single-client install result. Matches
/// the shape `vaner clients install --format json` emits as of the
/// multi-layer (Phase C) rollout: each client produces a results
/// entry with its own per-layer breakdown (`mcp` / `primer` / `skill`
/// / `hook`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteLayerResult {
    pub layer: String,
    #[serde(default)]
    pub applicable: bool,
    /// "added" / "updated" / "skipped" / "failed" / "not-applicable"
    /// — broader than the pre-multi-layer set so we just carry it
    /// through as a string.
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub client_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub detected: bool,
    /// "ready" / "wired-mcp-only" / "partial" / "missing" /
    /// "not-detected" — the overall rollup the user sees.
    #[serde(default)]
    pub overall: String,
    #[serde(default)]
    pub layers: Vec<WriteLayerResult>,
}

#[derive(Debug, Deserialize)]
struct WriteResponse {
    results: Vec<WriteResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDrift {
    pub client_id: String,
    pub label: String,
    pub config_path: Option<String>,
    pub drift: bool,
    pub current_in_config: Option<String>,
    pub expected: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub drift: Vec<ClientDrift>,
    pub drift_count: u32,
    pub fix_command: String,
}

// ---------------------------------------------------------------------------
// Verify — multi-layer leverage stack
// ---------------------------------------------------------------------------
//
// Mirrors the JSON shape of `vaner clients verify --format json` (added
// in this conversation's vaner PR). Four layers per client:
//   1. mcp     — universal floor; technical wiring
//   2. primer  — system-instruction-like rules file
//   3. skill   — agent-callable subroutine (Vaner ships vaner-feedback)
//   4. plugin  — atomic install bundle with hooks (Vaner ships for
//                Claude Code today)
//
// See docs.vaner.ai/integrations/client-capabilities for the full
// per-client capability matrix.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatus {
    pub applicable: bool,
    pub wired: bool,
    pub path: Option<String>,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientLayers {
    pub mcp: LayerStatus,
    pub primer: LayerStatus,
    pub skill: LayerStatus,
    pub plugin: LayerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientVerification {
    pub client_id: String,
    pub label: String,
    pub detected: bool,
    /// One of: "ready" | "wired-mcp-only" | "partial" | "missing" | "not-detected"
    pub overall: String,
    pub layers: ClientLayers,
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    results: Vec<ClientVerification>,
}

// ---------------------------------------------------------------------------
// Subprocess helpers
// ---------------------------------------------------------------------------

async fn run_vaner_clients_json(
    repo_root: &Path,
    extra_args: &[&str],
    allow_nonzero: bool,
) -> Result<String, String> {
    let bin = crate::vaner_cli::resolve_vaner_bin()?;
    let repo_root = resolve_clients_repo_root(repo_root)?;
    let mut cmd = Command::new(&bin);
    cmd.arg("clients")
        .args(extra_args)
        .arg("--repo-root")
        .arg(&repo_root)
        .arg("--format")
        .arg("json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("failed to run `vaner clients`: {e}"))?;

    if !output.status.success() && !allow_nonzero {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "vaner clients exited with code {}: {}",
            output.status.code().unwrap_or(-1),
            if stderr.is_empty() {
                "no stderr".into()
            } else {
                stderr
            }
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn resolve_clients_repo_root(repo_root: &Path) -> Result<PathBuf, String> {
    if !repo_root.as_os_str().is_empty() {
        return Ok(repo_root.to_path_buf());
    }
    crate::workspace::resolve().ok_or_else(|| {
        "No Vaner workspace selected. Pick a workspace before installing or verifying agents."
            .to_string()
    })
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// `vaner clients detect --format json` — list every supported MCP
/// client + its install/configured status on this machine.
#[tauri::command]
pub async fn clients_detect(repo_root: String) -> Result<Vec<DetectedClient>, String> {
    let stdout = run_vaner_clients_json(Path::new(&repo_root), &["detect"], false).await?;
    let resp: DetectResponse = serde_json::from_str(&stdout)
        .map_err(|e| format!("could not parse clients detect output: {e}"))?;
    Ok(resp.clients)
}

#[tauri::command]
pub async fn clients_install(
    repo_root: String,
    client_id: String,
    force: bool,
) -> Result<Vec<WriteResult>, String> {
    let mut args: Vec<&str> = vec!["install", &client_id];
    if force {
        args.push("--force");
    }
    // Install can exit non-zero on partial failure but still emits a per-
    // client breakdown — pass `allow_nonzero=true` so we surface the rows.
    let stdout = run_vaner_clients_json(Path::new(&repo_root), &args, true).await?;
    let resp: WriteResponse = serde_json::from_str(&stdout)
        .map_err(|e| format!("could not parse clients install output: {e}"))?;
    Ok(resp.results)
}

#[tauri::command]
pub async fn clients_install_all(
    repo_root: String,
    force: bool,
) -> Result<Vec<WriteResult>, String> {
    let mut args: Vec<&str> = vec!["install", "--all"];
    if force {
        args.push("--force");
    }
    let stdout = run_vaner_clients_json(Path::new(&repo_root), &args, true).await?;
    let resp: WriteResponse = serde_json::from_str(&stdout)
        .map_err(|e| format!("could not parse clients install --all output: {e}"))?;
    Ok(resp.results)
}

#[tauri::command]
pub async fn clients_uninstall(
    repo_root: String,
    client_id: String,
) -> Result<Vec<WriteResult>, String> {
    let stdout =
        run_vaner_clients_json(Path::new(&repo_root), &["uninstall", &client_id], true).await?;
    let resp: WriteResponse = serde_json::from_str(&stdout)
        .map_err(|e| format!("could not parse clients uninstall output: {e}"))?;
    Ok(resp.results)
}

/// `vaner clients doctor --format json` — exits non-zero on drift,
/// but the JSON payload is still valid; we tolerate non-zero here.
#[tauri::command]
pub async fn clients_doctor(repo_root: String) -> Result<DoctorReport, String> {
    let stdout = run_vaner_clients_json(Path::new(&repo_root), &["doctor"], true).await?;
    let report: DoctorReport = serde_json::from_str(&stdout)
        .map_err(|e| format!("could not parse clients doctor output: {e}"))?;
    Ok(report)
}

/// `vaner clients verify --format json` — per-layer status (MCP /
/// primer / skill / plugin) for every supported client. Drives the
/// final-slide verification panel in the desktop wizard so the user
/// sees which clients are wired and at what depth, not just whether
/// MCP is wired.
#[tauri::command]
pub async fn clients_verify(repo_root: String) -> Result<Vec<ClientVerification>, String> {
    let stdout = run_vaner_clients_json(Path::new(&repo_root), &["verify"], true).await?;
    let resp: VerifyResponse = serde_json::from_str(&stdout)
        .map_err(|e| format!("could not parse clients verify output: {e}"))?;
    Ok(resp.results)
}
