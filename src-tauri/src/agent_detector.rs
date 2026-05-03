//! Linux equivalent of the macOS AgentDetector. Scans `/proc/*/comm`
//! for known AI-agent process names (Cursor, Claude Desktop, VS Code
//! with Continue, Zed, etc.) and returns a snapshot the reducer
//! consumes via the `anyAgentRunning` flag.
//!
//! `/proc` is the canonical Linux way to enumerate processes without
//! shelling out; it's cheap (one readdir + one read per pid) and
//! requires no special privileges.

use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct AgentSuggestionOut {
    pub id: String,
    pub display_name: String,
    pub bundle_identifier: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentDetectorOut {
    pub running_count: u32,
    pub suggestions: Vec<AgentSuggestionOut>,
}

fn process_name(value: &str) -> String {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return String::new();
    }
    Path::new(&trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&trimmed)
        .trim_end_matches(".exe")
        .to_string()
}

fn command_basenames(cmdline: &[String]) -> BTreeSet<String> {
    cmdline.iter().map(|part| process_name(part)).collect()
}

fn claude_management_command(cmdline: &[String]) -> bool {
    let basenames: Vec<String> = cmdline.iter().map(|part| process_name(part)).collect();
    basenames
        .windows(2)
        .any(|pair| pair[0] == "claude" && matches!(pair[1].as_str(), "mcp" | "plugin" | "config"))
}

/// Map a process snapshot to a friendly agent id. Matching is deliberately
/// exact: `codex` must not count as VS Code just because it contains `code`,
/// and `claude mcp list` must not count as an active Claude session.
fn classify_process(comm: &str, cmdline: &[String]) -> Option<&'static str> {
    let name = process_name(comm);
    let basenames = command_basenames(cmdline);

    if name == "codex"
        || basenames.contains("codex")
        || cmdline
            .iter()
            .any(|part| part.to_ascii_lowercase().contains("@openai/codex"))
    {
        return Some("codex-cli");
    }
    if matches!(name.as_str(), "code" | "code-insiders") {
        return Some("vscode");
    }
    if name == "claude-desktop" || name == "claude desktop" {
        return Some("claude-desktop");
    }
    if (name == "claude" || basenames.contains("claude")) && !claude_management_command(cmdline) {
        return Some("claude-code");
    }
    match name.as_str() {
        "cursor" => Some("cursor"),
        "zed" | "zeditor" => Some("zed"),
        "continue" => Some("continue"),
        _ => None,
    }
}

#[tauri::command]
pub fn detect_agents() -> Result<AgentDetectorOut, String> {
    #[cfg(not(target_os = "linux"))]
    {
        return Ok(AgentDetectorOut {
            running_count: 0,
            suggestions: known_suggestions(),
        });
    }

    #[cfg(target_os = "linux")]
    {
        let mut hits: BTreeSet<&'static str> = Default::default();
        let entries = fs::read_dir("/proc").map_err(|e| format!("read /proc: {e}"))?;
        for entry in entries.flatten() {
            let path = entry.path();
            // Only numeric pid dirs.
            let pid_ok = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);
            if !pid_ok {
                continue;
            }
            if let Ok(comm) = fs::read_to_string(path.join("comm")) {
                let cmdline = fs::read(path.join("cmdline"))
                    .ok()
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .map(|raw| {
                        raw.split('\0')
                            .filter(|part| !part.is_empty())
                            .map(ToOwned::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if let Some(id) = classify_process(comm.trim(), &cmdline) {
                    hits.insert(id);
                }
            }
        }

        // Always return the static suggestion list so the
        // .noActiveAgent state has something to render.
        Ok(AgentDetectorOut {
            running_count: hits.len() as u32,
            suggestions: known_suggestions(),
        })
    }
}

fn known_suggestions() -> Vec<AgentSuggestionOut> {
    [
        ("cursor", "Cursor"),
        ("codex-cli", "Codex CLI"),
        ("claude-code", "Claude Code"),
        ("claude-desktop", "Claude Desktop"),
        ("vscode", "VS Code"),
        ("zed", "Zed"),
    ]
    .iter()
    .map(|(id, label)| AgentSuggestionOut {
        id: id.to_string(),
        display_name: label.to_string(),
        bundle_identifier: None,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::classify_process;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

    #[test]
    fn classifies_codex_without_marking_vs_code() {
        assert_eq!(
            classify_process("codex", &args(&["/usr/bin/codex"])),
            Some("codex-cli")
        );
        assert_eq!(
            classify_process("node", &args(&["node", "/home/abo/.npm-global/bin/codex"])),
            Some("codex-cli")
        );
        assert_eq!(
            classify_process(
                "node",
                &args(&[
                    "node",
                    "/home/abo/.npm-global/lib/node_modules/@openai/codex/bin/codex.js"
                ])
            ),
            Some("codex-cli")
        );
        assert_eq!(
            classify_process("code", &args(&["/usr/bin/code"])),
            Some("vscode")
        );
    }

    #[test]
    fn ignores_transient_claude_management_commands() {
        assert_eq!(
            classify_process("claude", &args(&["claude", "mcp", "list"])),
            None
        );
        assert_eq!(
            classify_process("claude", &args(&["claude"])),
            Some("claude-code")
        );
        assert_eq!(
            classify_process("claude-desktop", &args(&["claude-desktop"])),
            Some("claude-desktop")
        );
    }
}
