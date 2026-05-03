//! System-tray icon + menu. Both left-click and right-click surface
//! the menu — the user's preference on Linux (and KDE users expect
//! it too). macOS's "left-click toggles popover, right-click shows
//! menu" split is not the Linux convention.
//!
//! Menu structure:
//!
//! ```text
//! ┌──────────────────┐
//! │  Open Vaner      │  ← popover::show
//! │  Pin window      │  ← keeps the small Vaner window open
//! ├──────────────────┤
//! │  Preferences…    │  ← opens companion window on Preferences pane
//! │  Pause Vaner     │  ← emits menu:toggle-pause; Svelte flips
//! │  / Resume Vaner  │    the isPaused store. Label is dynamic —
//! │                  │    this row reads "Resume Vaner" when paused
//! │                  │    via an `app:pause-changed` event listener.
//! ├──────────────────┤
//! │  Quit            │  ← app.exit(0)
//! └──────────────────┘
//! ```
//!
//! `on_tray_icon_event` forwards to `tauri_plugin_positioner::on_tray_event`
//! so the positioner plugin's tray-bounds cache stays populated. Without
//! that, `popover::anchor` would always have to fall through to its
//! TopRight fallback.

use serde::Deserialize;
use tauri::{
    AppHandle, Emitter, Listener, Runtime,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

use crate::{companion, popover};

pub const TRAY_ID: &str = "main";

/// Menu item IDs — stringly-typed per Tauri's API.
const ID_OPEN: &str = "open";
const ID_PIN: &str = "pin";
const ID_PREFERENCES: &str = "preferences";
const ID_COCKPIT: &str = "cockpit";
const ID_PAUSE: &str = "pause";
const ID_QUIT: &str = "quit";

/// Cockpit URL the engine binds to. Hardcoded for now — the cockpit
/// host/port live in `[cockpit]` of `.vaner/config.toml` and the
/// engine respects `--port`, but the desktop assumes the default
/// (matches `vaner up`'s default and what the bring-up flow targets).
const COCKPIT_URL: &str = "http://127.0.0.1:8473";

const LABEL_PAUSE: &str = "Pause Vaner";
const LABEL_RESUME: &str = "Resume Vaner";

/// Payload for `app:pause-changed`, emitted by the Svelte side every
/// time the `isPaused` store flips. Single-field struct so adding more
/// state (e.g. queued-count for the menu hint) is non-breaking.
#[derive(Debug, Deserialize)]
struct PauseChangedPayload {
    paused: bool,
}

/// Install the tray icon. Call from the Tauri `setup` closure.
pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let (menu, pause_item) = build_menu(app)?;

    // Keep the pause menu item label in sync with the Svelte
    // `isPaused` store. The store is the source of truth (it owns
    // localStorage persistence + the popover's Resume button); when
    // it flips, app-state.ts emits `app:pause-changed { paused }`
    // and we relabel here. MenuItem is Arc-wrapped under the hood, so
    // cloning into the listener closure is cheap and the same handle
    // the menu was built from.
    let pause_for_listener = pause_item.clone();
    app.listen("app:pause-changed", move |event| {
        let payload: PauseChangedPayload = match serde_json::from_str(event.payload()) {
            Ok(p) => p,
            Err(_) => return,
        };
        let label = if payload.paused {
            LABEL_RESUME
        } else {
            LABEL_PAUSE
        };
        let _ = pause_for_listener.set_text(label);
    });

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound(
                "default window icon must be present to build the tray icon".into(),
            )
        })?)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Vaner")
        .on_menu_event(|app, event| match event.id.as_ref() {
            ID_OPEN => {
                let _ = popover::show(app);
            }
            ID_PIN => {
                let _ = popover::toggle_pinned(app);
            }
            ID_PREFERENCES => {
                let _ = companion::open_window(app, Some("preferences".into()));
            }
            ID_COCKPIT => {
                let _ = std::process::Command::new("xdg-open")
                    .arg(COCKPIT_URL)
                    .spawn();
            }
            ID_PAUSE => {
                // Forward to the Svelte side. The popover's
                // app-state store listens for `menu:toggle-pause`
                // and flips the isPaused flag, which the reducer
                // turns into the .paused popover state.
                let _ = app.emit("menu:toggle-pause", ());
            }
            ID_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            #[cfg(not(target_family = "unix"))]
            {
                // The positioner plugin caches the tray icon's bounds so
                // `Position::TrayCenter` knows where to anchor on platforms
                // where the tray geometry API is reliable.
                tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            }
            #[cfg(target_family = "unix")]
            {
                let _ = (tray, event);
            }
        })
        .build(app)?;
    Ok(())
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<(Menu<R>, MenuItem<R>)> {
    let open = MenuItem::with_id(app, ID_OPEN, "Open Vaner", true, None::<&str>)?;
    let pin = MenuItem::with_id(app, ID_PIN, "Pin / Unpin window", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let prefs = MenuItem::with_id(app, ID_PREFERENCES, "Preferences…", true, None::<&str>)?;
    // The cockpit is the engine's web UI (FastAPI app served by
    // `vaner up` on 127.0.0.1:8473). For users who want the full
    // dashboard rather than the popover summary, route the existing
    // browser session there with one click.
    let cockpit = MenuItem::with_id(app, ID_COCKPIT, "Open Cockpit (web UI)", true, None::<&str>)?;
    // UI-level mute toggle. Daemon-side POST /engine/pause is still
    // Tier B; today this just flips an isPaused flag the popover
    // reducer reads to enter the .paused state. Re-wire to the
    // engine endpoint when CONTRACT.md ships it.
    //
    // Initial label is "Pause Vaner" because the Svelte side hasn't
    // hydrated `isPaused` yet — it'll emit `app:pause-changed` on
    // bootstrap and the listener installed above will flip the
    // label to "Resume Vaner" if the user was previously paused.
    let pause = MenuItem::with_id(app, ID_PAUSE, LABEL_PAUSE, true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, ID_QUIT, "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&open, &pin, &sep1, &prefs, &cockpit, &pause, &sep2, &quit],
    )?;
    Ok((menu, pause))
}
