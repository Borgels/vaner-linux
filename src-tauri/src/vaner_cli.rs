//! Shared resolution of the `vaner` CLI binary.
//!
//! Prefers an explicit `VANER_BIN` env override (the AppImage and Windows
//! NSIS bundle can ship the CLI alongside) and otherwise falls back to a
//! `$PATH` lookup via the cross-platform [`which`] crate. On Windows that
//! handles `.exe`/`.cmd`/`.bat` extensions transparently, matching Python's
//! `shutil.which` semantics.

use std::path::PathBuf;

pub fn resolve_vaner_bin() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("VANER_BIN") {
        if !explicit.is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let local = cwd.join("scripts").join("vaner-local-cli.sh");
        if local.is_file() {
            return Ok(local);
        }
    }
    if let Ok(p) = which::which("vaner") {
        return Ok(p);
    }
    // GUI processes (AppImage launched from a .desktop file, GNOME's
    // tray-app autostart, etc.) run under a sanitised PATH that
    // typically excludes `~/.local/bin` — where `pipx install vaner`
    // lands by default. Probe the canonical install locations so the
    // desktop doesn't tell the user "Vaner not found" when it's right
    // there. Order mirrors the macOS app's `EngineDetector`.
    let candidates: Vec<PathBuf> = std::env::var_os("HOME")
        .map(|h| {
            vec![
                PathBuf::from(&h).join(".local/bin/vaner"),
                PathBuf::from(&h).join(".cargo/bin/vaner"),
            ]
        })
        .unwrap_or_default()
        .into_iter()
        .chain([
            PathBuf::from("/usr/local/bin/vaner"),
            PathBuf::from("/opt/homebrew/bin/vaner"),
            PathBuf::from("/home/linuxbrew/.linuxbrew/bin/vaner"),
        ])
        .collect();
    for p in candidates {
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(
        "Vaner binary not found on PATH or in ~/.local/bin / /usr/local/bin. \
         Install Vaner via vaner.ai/install.sh or set $VANER_BIN."
            .to_string(),
    )
}
