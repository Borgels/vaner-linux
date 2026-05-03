//! Background updater — checks GitHub Releases via
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
    /// What format the running binary was installed as. The Tauri-v2
    /// updater on Linux can only self-replace AppImages — `.deb`
    /// installs go to root-owned `/usr/bin/vaner-desktop` and the
    /// updater throws "invalid updater binary format" on the swap.
    /// The Svelte banner reads this and routes the user to the
    /// release page instead of pretending the in-app install will
    /// work.
    pub install_kind: InstallKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallKind {
    /// `.deb` install at `/usr/bin/vaner-desktop` (root-owned). Tauri's
    /// updater can't replace this; surface a "Download .deb" CTA.
    Deb,
    /// AppImage launch — `$APPIMAGE` is set to the AppImage path.
    /// Tauri's updater self-replaces it and the in-app flow Just
    /// Works.
    AppImage,
    /// Anything else (manual build, snap, flatpak, …). Surface a
    /// "View release" CTA — we don't know how to swap the binary.
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

/// Open the GitHub release page for `version` in the user's default
/// browser. Used by the banner's "Download .deb" / "View release"
/// CTA when the in-app updater can't self-replace.
#[tauri::command]
pub fn update_open_release(version: String) -> Result<(), String> {
    let url = format!(
        "https://github.com/Borgels/vaner-desktop/releases/tag/v{}",
        version
    );
    open_url(&url)
}

fn open_url(url: &str) -> Result<(), String> {
    // xdg-open on Linux, no-op on platforms we don't run on. Spawning
    // is enough — we don't care about the exit code; the user sees the
    // browser open or it doesn't, in which case they'll tell us.
    std::process::Command::new("xdg-open")
        .arg(url)
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
/// "Install update" on the toast. Downloads + installs + emits
/// progress events on `update:progress` for a future UI progress
/// bar; once finished the app is in a restart-required state.
///
/// Refuses to run on `.deb` installs: Tauri-v2's updater on Linux can
/// only self-replace AppImages, so the swap throws "invalid updater
/// binary format" and the user is told the update succeeded when it
/// silently didn't. The banner branches on `install_kind` and routes
/// `.deb` users to `update_open_release` instead.
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if updater_disabled() {
        return Err("updater is disabled for this local build".to_string());
    }
    if detect_install_kind() == InstallKind::Deb {
        return Err(
            "in-app update is not supported on .deb installs; download the new .deb from the release page"
                .to_string(),
        );
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
    update
        .download_and_install(
            |chunk, total| {
                let fraction = total
                    .map(|t| (chunk as f64) / (t as f64))
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);
                let _ = app_handle.emit("update:progress", fraction);
            },
            || {
                let _ = app_handle.emit("update:ready-to-restart", ());
            },
        )
        .await
        .map_err(|e| format!("download-and-install failed: {e}"))?;
    Ok(())
}
