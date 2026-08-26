//! Agent personas: selection persistence, per-turn directives, and the
//! Plan-persona tool gate (read-only enforcement + secret-path classifier).
//!
//! Persona selection persists to `agent_persona.json` inside the shared app
//! config directory so local runs and the daemon observe the same active
//! persona without protocol changes.

use crate::tui::AgentMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const AGENT_PERSONA_FILE: &str = "agent_persona.json";
const AGENT_PERSONA_VERSION: u8 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AgentPersonaStore {
    version: u8,
    #[serde(default)]
    persona: Option<String>,
    /// Path substrings the user explicitly allowed the Plan agent to read
    /// via the "Always allow" pill. Matched case-insensitively against the
    /// whole path string.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    plan_read_allowlist: Vec<String>,
}

fn agent_persona_path() -> Option<PathBuf> {
    crate::storage::app_config_dir()
        .ok()
        .map(|dir| dir.join(AGENT_PERSONA_FILE))
}

pub(in crate::tui::app) fn load_persisted_persona() -> Option<AgentMode> {
    let path = agent_persona_path()?;
    let bytes = std::fs::read(path).ok()?;
    let store: AgentPersonaStore = serde_json::from_slice(&bytes).ok()?;
    if store.version != AGENT_PERSONA_VERSION {
        return None;
    }
    let key = store.persona?;
    AgentMode::from_key(&key)
}

pub(in crate::tui::app) fn persist_persona(mode: AgentMode) {
    let Some(path) = agent_persona_path() else {
        return;
    };
    let store = AgentPersonaStore {
        version: AGENT_PERSONA_VERSION,
        persona: Some(mode.key().to_string()),
        // Preserve any existing allowlist when rewriting the file.
        plan_read_allowlist: read_store()
            .map(|store| store.plan_read_allowlist)
            .unwrap_or_default(),
    };
    if let Ok(json) = serde_json::to_vec_pretty(&store) {
        let _ = std::fs::write(path, json);
    }
}

fn read_store() -> Option<AgentPersonaStore> {
    let path = agent_persona_path()?;
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

// ---------------------------------------------------------------------------
// Per-turn directives
// ---------------------------------------------------------------------------

/// System-reminder directive appended to every turn while a persona is
/// active. `Build` is the default and injects nothing extra beyond its
/// structured-workflow guidance; other personas get their own guidance.
pub(in crate::tui::app) fn persona_directive(mode: AgentMode) -> &'static str {
    match mode {
        AgentMode::Build => {
            "# Persona: Build\n\n\
             You are in Build mode. Work like a senior engineer:\n\
             1. Make sure you fully understand the request. If anything is ambiguous, \
             risky, or underspecified, ask concise clarifying questions BEFORE writing code.\n\
             2. Once clear, produce a short todo/phased plan and follow it.\n\
             3. Implement phase by phase. Keep changes minimal and focused.\n\
             4. Verify each phase (build/tests) before moving on.\n\
             5. Summarize what changed at the end."
        }
        AgentMode::Plan => {
            "# Persona: Plan (READ-ONLY)\n\n\
             You are in Plan mode. Your job is to research and produce an \
             implementation plan — never to modify anything.\n\
             1. Investigate thoroughly with read-only tools.\n\
             2. Ask clarifying questions until you understand the goal, how it \
             should work, and the constraints.\n\
             3. Deliver a structured plan: Goal, Approach, Phases (steps + files \
             + risks), Open questions.\n\
             Write/edit/mutating tools are BLOCKED in this mode. Reads of secret \
             files require explicit user approval."
        }
        _ => "",
    }
}

/// Merge an optional base reminder with the active persona directive.
pub(in crate::tui::app) fn merge_turn_reminder(
    base: Option<String>,
    mode: AgentMode,
) -> Option<String> {
    let directive = persona_directive(mode);
    match (base, directive.is_empty()) {
        (Some(base), true) => Some(base),
        (None, true) => None,
        (base, false) => {
            let mut combined = String::new();
            if let Some(base) = base.filter(|value| !value.is_empty()) {
                combined.push_str(&base);
                combined.push_str("\n\n");
            }
            combined.push_str(directive);
            Some(combined)
        }
    }
}

// ---------------------------------------------------------------------------
// Plan persona tool gate
// ---------------------------------------------------------------------------

/// Outcome of gating one tool call under the active persona.
pub(in crate::tui::app) enum ToolGateDecision {
    /// Run the tool normally.
    Allow,
    /// Block the call; the string is returned to the model as the tool error.
    Deny(String),
    /// Block until the user answers the in-chat permission panel.
    NeedsPermission { reason: String },
}

/// Tools the Plan persona may always use (pure reads / non-mutating).
fn plan_allowed_tool(name_lower: &str) -> bool {
    matches!(
        name_lower,
        "read" | "grep" | "glob" | "ls" | "list" | "todo" | "todos"
    )
}

/// Path substrings treated as secrets for the Plan persona.
fn is_secret_path(path_lower: &str) -> bool {
    const SECRET_MARKERS: [&str; 10] = [
        ".env", ".pem", ".key", "id_rsa", "id_ed25519", "/.ssh/", "\\.ssh\\", "credentials",
        "secret", "private_key",
    ];
    SECRET_MARKERS
        .iter()
        .any(|marker| path_lower.contains(marker))
}

fn allowlisted(path_lower: &str) -> bool {
    read_store()
        .map(|store| {
            store
                .plan_read_allowlist
                .iter()
                .any(|rule| path_lower.contains(&rule.to_ascii_lowercase()))
        })
        .unwrap_or(false)
}

/// Gate one tool call under the Plan persona.
///
/// `tool_name`/`input_json` come from the dispatch site; `extract_paths`
/// pulls candidate filesystem paths out of the input for the secret guard.
pub(in crate::tui::app) fn gate_plan_tool(
    tool_name: &str,
    input_json: &serde_json::Value,
) -> ToolGateDecision {
    let name_lower = tool_name.to_ascii_lowercase();
    if plan_allowed_tool(&name_lower) {
        return ToolGateDecision::Allow;
    }

    // Secret guard first on read-ish tools carrying a path argument.
    let path = ["file_path", "path", "absolute_path"]
        .iter()
        .find_map(|key| input_json.get(*key).and_then(|value| value.as_str()));
    if let Some(path) = path {
        let lowered = path.to_ascii_lowercase();
        if !allowlisted(&lowered)
            && (is_secret_path(&lowered) || name_lower.contains("read"))
        {
            return ToolGateDecision::NeedsPermission {
                reason: format!("Plan agent wants to READ {path}"),
            };
        }
    }

    ToolGateDecision::Deny(format!(
        "{tool_name} is blocked in Plan mode. This agent is read-only — \
         switch agents (Tab) to Build to make changes."
    ))
}

/// Persist an "Always allow" rule for `path`.
pub(in crate::tui::app) fn allow_plan_read_path(path: &str) {
    let mut store = read_store().unwrap_or_default();
    let lowered = path.to_ascii_lowercase();
    if store.plan_read_allowlist.iter().any(|rule| lowered.contains(&rule.to_ascii_lowercase())) {
        return;
    }
    store.plan_read_allowlist.push(path.to_string());
    if let Ok(json) = serde_json::to_vec_pretty(&store) {
        if let Some(path_buf) = agent_persona_path() {
            let _ = std::fs::write(path_buf, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_gate_allows_reads_blocks_writes_and_gates_secret_paths() {
        use serde_json::json;

        assert!(matches!(
            gate_plan_tool("read", &json!({ "file_path": "src/main.rs" })),
            ToolGateDecision::Allow
        ));
        assert!(matches!(
            gate_plan_tool(
                "write",
                &json!({ "file_path": "src/main.rs", "content": "x" })
            ),
            ToolGateDecision::Deny(_)
        ));
        assert!(matches!(
            gate_plan_tool("bash", &json!({ "command": "ls" })),
            ToolGateDecision::Deny(_)
        ));
        assert!(matches!(
            gate_plan_tool("read", &json!({ "file_path": ".env" })),
            ToolGateDecision::NeedsPermission { .. }
        ));
    }

    #[test]
    fn secret_classifier_matches_expected_markers() {
        assert!(is_secret_path(".env"));
        assert!(is_secret_path("server.pem"));
        assert!(is_secret_path("/home/u/.ssh/id_rsa"));
        assert!(is_secret_path("C:\\repo\\credentials.json"));
        assert!(!is_secret_path("src/main.rs"));
        assert!(!is_secret_path("docs/readme.md"));
    }

    #[test]
    fn directives_present_for_build_and_plan_only() {
        assert!(!persona_directive(AgentMode::Build).is_empty());
        assert!(!persona_directive(AgentMode::Plan).is_empty());
        assert!(persona_directive(AgentMode::Swarm).is_empty());
    }
}
