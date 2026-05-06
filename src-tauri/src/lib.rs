//! Tauri v2 app entry point.
//!
//! Wires the shared [`vaner_contract`] HTTP/SSE client into a
//! Tauri-native runtime. Responsibilities are split across modules:
//!
//! - [`commands`] — `#[tauri::command]` handlers exposed to Svelte via
//!   `invoke()` (active predictions, adopt flow).
//! - [`sse_task`] — background tokio task that subscribes to the
//!   daemon's SSE stream and emits `predictions:snapshot` events to
//!   the WebView.
//! - [`session`] — XDG session / DE detection for first-run guidance
//!   on GNOME/Wayland without the AppIndicator extension.
//! - [`tray`] — system-tray icon + menu.
//! - [`popover`] — show / hide / toggle the borderless popover window.
//! - [`updater`] — background update check via tauri-plugin-updater.
//!
//! The public entry is [`run`], called from `main.rs`.

use std::sync::Arc;
use tauri::{Emitter, Manager, WindowEvent};
use tokio::sync::Mutex;

use vaner_contract::HttpEngineClient;

pub mod agent_detector;
pub mod bring_up;
pub mod clients;
pub mod commands;
pub mod companion;
pub mod daemon_audit;
pub mod diagnostics;
pub mod engine;
pub mod engine_config;
pub mod engine_service;
pub mod engine_status_task;
pub mod ollama;
pub mod ollama_health_task;
pub mod onboarding;
pub mod popover;
pub mod prepared_work_endpoint;
pub mod session;
pub mod setup;
pub mod sse_task;
pub mod tray;
pub mod updater;
pub mod vaner_cli;
pub mod workspace;

/// Process-wide state. A single reqwest-backed HTTP client is shared
/// across every `#[tauri::command]` so connection pooling works.
pub struct AppState {
    pub engine: Arc<HttpEngineClient>,
    /// Handle to the SSE background task; kept so the app can abort
    /// on shutdown / reconnection.
    pub sse_handle: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            engine: Arc::new(HttpEngineClient::localhost()),
            sse_handle: Mutex::new(None),
        }
    }
}

/// Strip AppImage-injected env vars so they don't leak to child
/// processes. The AppImage runtime sets `PYTHONHOME` / `PYTHONPATH` /
/// `LD_LIBRARY_PATH` pointing into the AppImage's `/tmp/.mount_*`
/// directory — useful for Python/libs *inside* the AppImage, lethal
/// for anything we shell out to (notably `vaner`, which is itself a
/// Python CLI installed at the user's `~/.local/bin/vaner`). Without
/// this strip, every `vaner` invocation crashes with
/// "ModuleNotFoundError: No module named 'encodings'" because the
/// child's Python tries to load stdlib from the AppImage mount.
///
/// Done at the top of `run()` before any worker is spawned so the
/// edition-2024 `set_var`/`remove_var` race is moot.
fn strip_appimage_env() {
    // SAFETY: called once, single-threaded, before any task spawns.
    unsafe {
        for var in [
            "PYTHONHOME",
            "PYTHONPATH",
            "LD_LIBRARY_PATH",
            "LD_PRELOAD",
            "APPDIR",
            "ARGV0",
            "GIO_MODULE_DIR",
            "GTK_PATH",
            "PERL5LIB",
        ] {
            std::env::remove_var(var);
        }
    }
}

/// Silence the `tauri-plugin-positioner` zbus-executor panic.
///
/// Background: `tauri-plugin-positioner` keeps a long-lived task on
/// its zbus connection that handles tray-position updates. On Linux
/// hosts where the SNI/AppIndicator panel never reports the icon's
/// geometry (most of them, today), the task hits an
/// `expect("Tray position not set")` at `ext.rs:301:17` and the
/// worker thread dies. Rust's default panic handler prints a stack
/// trace to stderr. The main process keeps running — it's noise, not
/// a crash — but the noise lands in the user's terminal whenever they
/// launch `vaner-desktop` from a shell, which is alarming.
///
/// We swallow the specific panic ("Tray position not set") at the
/// hook level. Every other panic falls through to the prior handler
/// so real bugs still surface.
fn install_positioner_panic_silencer() {
    let prior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info
            .payload()
            .downcast_ref::<&'static str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("");
        if msg.contains("Tray position not set") {
            return;
        }
        prior(info);
    }));
}

/// App entry. Called from both `main.rs` and mobile wrappers.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    strip_appimage_env();
    install_positioner_panic_silencer();

    let state = AppState::default();
    let engine = state.engine.clone();
    let engine_status_cache = Arc::new(engine_status_task::EngineStatusCache::new());
    let ollama_health_cache = Arc::new(ollama_health_task::OllamaHealthCache::new());

    tauri::Builder::default()
        // Single-instance must be registered first so the dbus
        // handshake runs before any other plugin spawns long-lived
        // tasks. When a second `vaner-desktop` is launched (tray
        // double-click, autostart misfire, manual relaunch), this
        // closure runs in the *first* process — we surface the
        // existing popover instead of letting two trays + two
        // bring-up tasks duke it out over the same state.json.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = popover::show(app);
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .manage(engine_status_cache.clone())
        .manage(ollama_health_cache.clone())
        .setup(move |app| {
            // Production-readiness: the desktop does NOT start a
            // daemon, adopt a workspace, or watch any repo unless
            // the user (or a wired MCP client) explicitly causes it
            // to. Two behaviours that previously violated this:
            //
            //   - `workspace::adopt_running_cockpit` silently
            //     persisted whichever workspace happened to have a
            //     cockpit running into the desktop's state.json.
            //     Side effect: if a stale `vaner up` was lingering
            //     from a CLI session, the desktop "inherited" it
            //     and started feeding scenario data the user never
            //     asked for.
            //
            //   - `bring_up::spawn_at_startup` shelled `vaner up`
            //     on every launch, immediately spinning up the
            //     ponder loop with whatever workspace was persisted
            //     — including stale adoptions from the bullet above.
            //
            // Both are removed from the startup path. The Engine
            // pane's "Start engine" button still drives bring-up
            // explicitly, and a wired MCP client spawns its own
            // `vaner mcp --path` invocation; that's how Vaner
            // becomes active on a fresh install. Until then, the
            // desktop is a viewer for whatever the user / their
            // clients started, and nothing more.
            //
            // export_to_env still runs because it only mirrors
            // `state.json` into the process env — no work happens.
            workspace::export_to_env(app.handle());

            // Stray-daemon audit. Runs once at startup and emits the
            // result so the popover/companion can prompt the user
            // about rogue `vaner daemon / up / proxy / mcp`
            // processes the desktop didn't spawn. Empty-result =
            // silent; non-empty = a Diagnostics-style banner with
            // a "Stop these" button. Cheap (read /proc once).
            let audit_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let strays = daemon_audit::find_strays();
                let _ = audit_app.emit("daemon:strays", &strays);
            });

            // Single-source engine status. One Rust-side poll loop
            // shells `vaner status --json`, caches the result, and
            // emits an `engine:status` event whenever anything
            // changes. Every webview reads through this cache —
            // popover and companion can no longer drift.
            engine_status_task::spawn(app.handle().clone(), engine_status_cache.clone());

            // Same single-source pattern for Ollama presence.
            // Vaner's local-first default backend is Ollama on
            // localhost:11434; if it isn't installed (or isn't
            // running) the model loop 502s on every MCP call. The
            // popover routes to `.ollamaMissing` until the daemon
            // probe answers.
            ollama_health_task::spawn(app.handle().clone(), ollama_health_cache.clone());

            // Kick off the SSE snapshot stream; the Svelte store
            // listens on `predictions:snapshot`.
            let handle = sse_task::spawn(app.handle().clone(), engine.clone());
            let app_state = app.state::<AppState>();
            tauri::async_runtime::block_on(async {
                *app_state.sse_handle.lock().await = Some(handle);
            });

            // Install the tray icon + menu ("Open Vaner" /
            // Preferences / Pause / Quit). Menu shows on both left
            // and right click per the documented UX contract.
            tray::install(app.handle())?;

            // First-run guidance: if session+DE can't show tray icons
            // without extra setup, nudge the user now.
            session::first_run_nudge(app.handle());

            // Local AppImage smoke builds are often launched from a
            // terminal on Linux shells where the tray icon is delayed or
            // hidden by the desktop environment. Show the popover once
            // explicitly so the build is testable without hunting for a
            // tray affordance. Production keeps tray-first behavior unless
            // the user/exported launcher requests the same override.
            //
            // Windows is different: Winget and Start Menu validation launch
            // the app and expect a visible UI. Keeping a tray-only first
            // launch there looks like a blank/no-op install, so Windows
            // opens the popover by default unless explicitly suppressed.
            if std::env::var("VANER_DESKTOP_SHOW_ON_START").ok().as_deref() == Some("1")
                || std::env::var("VANER_DESKTOP_LOCAL_BUILD").ok().as_deref() == Some("1")
                || (cfg!(target_os = "windows")
                    && std::env::var("VANER_DESKTOP_START_HIDDEN").ok().as_deref() != Some("1"))
            {
                let show_app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = popover::show(&show_app);
                });
            }

            // Background update check. Emits `update:available` when
            // a new release is on vaner.ai and its minisign signature
            // verifies against the pubkey in tauri.conf.json. Failure
            // modes (no network, no update, bad signature) are all
            // silent by design — the user didn't ask.
            updater::spawn_check(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Menu-bar behaviour: hide the popover when it loses
            // focus, matching NSPopover semantics. The Svelte layer
            // can still re-show via invoke or tray click.
            if window.label() == popover::WINDOW_LABEL
                && matches!(event, WindowEvent::Focused(false))
                && !popover::is_pinned()
            {
                let _ = popover::hide(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::active_predictions,
            commands::prediction_overview,
            commands::prepared_work,
            commands::prepared_work_action,
            commands::focus_status,
            commands::focus_route_status,
            commands::focus_route_update,
            commands::focus_action,
            commands::resources_status,
            commands::jobs_status,
            commands::adopt_prediction,
            commands::app_quit,
            commands::window_hide,
            commands::open_external_url,
            updater::install_update,
            updater::update_install_kind,
            updater::update_open_release,
            popover::popover_toggle_pinned,
            popover::popover_is_pinned,
            diagnostics::diagnostics_status,
            diagnostics::diagnostics_runtime,
            diagnostics::diagnostics_doctor,
            diagnostics::diagnostics_restart_engine,
            diagnostics::diagnostics_upgrade_engine,
            diagnostics::set_local_model,
            clients::clients_detect,
            clients::clients_install,
            clients::clients_install_all,
            clients::clients_uninstall,
            clients::clients_doctor,
            clients::clients_verify,
            setup::setup_questions,
            setup::setup_recommend,
            setup::models_recommended,
            setup::setup_apply,
            setup::setup_status,
            setup::policy_show,
            setup::policy_refresh,
            setup::hardware_profile,
            setup::deep_run_defaults,
            companion::open_companion,
            companion::close_companion,
            onboarding::open_onboarding,
            onboarding::close_onboarding,
            engine::engine_status,
            engine_status_task::engine_status_boost,
            agent_detector::detect_agents,
            ollama::ollama_list,
            ollama_health_task::ollama_health,
            ollama_health_task::install_ollama,
            ollama::ollama_pull,
            ollama::ollama_cancel_pull,
            ollama::ollama_remove,
            workspace::workspace_get,
            workspace::workspace_set,
            workspace::workspace_pick,
            bring_up::bring_up_engine,
            engine_service::engine_service_status,
            engine_service::engine_service_install,
            engine_service::engine_service_uninstall,
            engine_service::engine_service_set_linger,
            engine_config::compute_config_get,
            engine_config::compute_config_set,
            engine_config::compute_apply_preset,
            engine_config::backend_config_get,
            engine_config::backend_apply_preset,
            engine_config::backend_classify,
            daemon_audit::audit_strays,
            daemon_audit::kill_strays,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
