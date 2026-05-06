//! Background updater — checks the vaner.ai release mirror via
//! `tauri-plugin-updater`, emits events the Svelte layer can surface
//! as a calm "update available" toast.
//!
//! The plugin handles signature verification against the minisign
//! pubkey baked into `tauri.conf.json`; a tampered `latest.json`
//! fails verification and the check silently returns no update.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_updater::UpdaterExt as _;

/// Event payload for `update:available`.
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePayload {
    pub version: String,
    pub current_version: String,
    pub release_notes: Option<String>,
    /// What format the running binary was installed as. The Svelte layer
    /// can surface this in diagnostics, but update installation itself is
    /// always driven by Tauri's signed updater.
    pub install_kind: InstallKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallKind {
    /// `.deb` install at `/usr/bin/vaner-desktop` (root-owned). Tauri's
    /// updater installs a new signed package through the platform package
    /// flow when the manifest exposes `linux-x86_64-deb`.
    Deb,
    /// AppImage launch — `$APPIMAGE` is set to the AppImage path.
    /// Tauri's updater self-replaces it and the in-app flow Just
    /// Works.
    AppImage,
    /// Anything else (manual build, snap, flatpak, Windows NSIS, …).
    /// The install command still tries Tauri's updater; this is telemetry
    /// for UI copy and diagnostics, not a hard gate.
    Other,
}

/// Detect how the running binary was installed.
///
/// Order:
/// 1. `$APPIMAGE` set → AppImage. The AppImage runtime exports this
///    env var to the binary it launches; nothing else does.
/// 2. `current_exe()` resolves under `/usr/`, `/opt/`, or any other
///    system prefix → assume `.deb` (the only system-package format
///    we ship). The user *could* have installed a manual rpm into
///    `/usr/bin/`, but the bring-up flow is the same: deb-style.
/// 3. Anything else → Other (dev build, manual install in `~`).
pub fn detect_install_kind() -> InstallKind {
    if std::env::var("APPIMAGE")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return InstallKind::AppImage;
    }
    if let Ok(exe) = std::env::current_exe() {
        let path = exe.to_string_lossy();
        if path.starts_with("/usr/") || path.starts_with("/opt/") {
            return InstallKind::Deb;
        }
    }
    InstallKind::Other
}

pub fn updater_disabled() -> bool {
    fn enabled_env(name: &str) -> bool {
        matches!(
            std::env::var(name).ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
        )
    }
    enabled_env("VANER_DISABLE_UPDATER") || enabled_env("VANER_DESKTOP_LOCAL_BUILD")
}

#[tauri::command]
pub fn update_install_kind() -> InstallKind {
    detect_install_kind()
}

/// Open the vaner.ai download page for `version` in the user's default
/// browser. This is an error fallback only; the primary update path is
/// always the signed in-app updater.
#[tauri::command]
pub fn update_open_release(version: String) -> Result<(), String> {
    let url = format!("https://vaner.ai/download?desktopVersion={version}");
    open_url(&url)
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open URL: {e}"))
}

/// Kick off a best-effort background update check. Errors are
/// swallowed — the app works fine without the updater, and there's
/// no useful user-facing message for a transient network failure
/// that the user didn't ask about.
pub fn spawn_check<R: Runtime>(app: AppHandle<R>) {
    if updater_disabled() {
        eprintln!("[vaner-desktop] updater disabled for local build");
        return;
    }
    tauri::async_runtime::spawn(async move {
        if let Err(e) = check(app).await {
            // Log at stderr; operators grepping daemon logs will see
            // this, end users won't be bothered.
            eprintln!("[vaner-desktop] updater check failed: {e}");
        }
    });
}

async fn check<R: Runtime>(app: AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let updater = app.updater_builder().build()?;
    let Some(update) = updater.check().await? else {
        return Ok(());
    };

    let payload = UpdatePayload {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        release_notes: update.body.clone(),
        install_kind: detect_install_kind(),
    };

    app.emit("update:available", &payload)?;
    Ok(())
}

/// `#[tauri::command]` — invoked from Svelte when the user clicks
/// "Update now" on the banner. Downloads + installs + emits
/// progress events on `update:progress` for a future UI progress
/// bar; once finished the app is in a restart-required state.
///
/// The release manifest must expose installer-specific targets
/// (`linux-x86_64-deb`, `linux-x86_64-appimage`, `windows-x86_64-nsis`)
/// so Tauri downloads the package format matching the running install.
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if updater_disabled() {
        return Err("updater is disabled for this local build".to_string());
    }
    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| format!("updater init failed: {e}"))?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|e| format!("updater check failed: {e}"))?
    else {
        return Ok(());
    };

    let app_handle = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                let fraction = total
                    .map(|t| (downloaded as f64) / (t as f64))
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);
                let _ = app_handle.emit("update:progress", fraction);
            },
            || {},
        )
        .await
        .map_err(|e| format!("download-and-install failed: {e}"))?;
    let _ = app.emit("update:progress", 1.0);
    let _ = app.emit("update:ready-to-restart", ());
    app.request_restart();
    Ok(())
}
