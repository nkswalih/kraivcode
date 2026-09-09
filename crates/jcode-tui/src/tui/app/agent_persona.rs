//! Agent personas: selection persistence, per-turn directives, and the
//! Plan-persona tool gate (read-only enforcement + secret-path classifier).
//!
//! Persona selection persists to `agent_persona.json` inside the shared app
//! config directory so local runs and the daemon observe the same active
//! persona without protocol changes.
//!
//! Tool availability is enforced at the **definition level**: each persona
//! declares an allowlist that filters `Registry::definitions()` so the model
//! never sees tools it may not use.  This is zero-token-cost enforcement
//! (OpenCode-style) — no system-prompt injection needed for tool gating.
//! The optional `persona_directive()` strings provide *behavioral guidance*
//! only and may be removed in a later pass.

use crate::tui::AgentMode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const AGENT_PERSONA_FILE: &str = "agent_persona.json";
const AGENT_PERSONA_VERSION: u8 = 1;

/// An agent question awaiting an answer from the user (Plan-mode popup).
///
/// Rendered as a modal overlay; while present, the app's normal prompt input
/// is suppressed and keys control this dialog until the user answers or
/// cancels.
pub struct PendingAskUser {
    pub request_id: String,
    pub question: String,
    /// (label, value) pairs shown in the picker.
    pub options: Vec<(String, String)>,
    /// Whether the user may type a free-text answer.
    pub free_text: bool,
    /// Currently highlighted option index.
    pub selected: usize,
    /// In-progress free-text answer.
    pub free_text_buffer: String,
    /// Cursor position within `free_text_buffer`.
    pub cursor: usize,
    /// Synthesized post-turn Plan-followup popup (a prose question the daemon
    /// caught in the finished Plan text). A free-text answer is re-submitted to
    /// the session as the next user turn.
    pub plan_followup: bool,
}

/// What a key press did to a pending agent question popup.
pub(in crate::tui::app) enum AskUserAction {
    /// Key was consumed without answering (e.g. moving the selection).
    None,
    /// The user answered; carries the submitted value (None = empty answer).
    Submitted(Option<String>),
    /// The user dismissed the question.
    Cancelled,
}

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
// Permission rulesets (zero-token enforcement)
// ---------------------------------------------------------------------------

/// Tool-policy outcome for a given persona.
///
/// Mirrors OpenCode's permission-based approach: the model never sees tools
/// it may not use, so no system-prompt instruction is needed to block them.
#[derive(Debug, Clone)]
pub struct PersonaToolPolicy {
    /// `None` = all registered tools are visible to the model.
    /// `Some(set)` = only tools in the set are visible.
    pub allowed_tools: Option<HashSet<String>>,
    /// Tools blocked even if present in the allowlist (defense-in-depth).
    /// Currently unused — reserved for per-tool overrides (e.g. blocking
    /// specific MCP tools while allowing others).
    #[allow(dead_code)]
    pub disabled_tools: HashSet<String>,
}

/// Return the tool-policy ruleset for the given persona.
///
/// - **Build** (default): full tool access (`None` allowlist).
/// - **Plan**: read-only tools only; write/edit/bash/browser/… are hidden.
/// - All other personas: full access (future work may restrict these).
///
/// Delegates to the shared `jcode_app_core::agent::persona` module so the TUI
/// and the daemon always agree on the same allowlist.
pub fn persona_tool_policy(mode: AgentMode) -> PersonaToolPolicy {
    let persona =
        jcode_app_core::agent::persona::AgentPersona::from_key(mode.key()).unwrap_or_default();
    let shared = jcode_app_core::agent::persona::persona_tool_policy(persona);
    PersonaToolPolicy {
        allowed_tools: shared.allowed_tools,
        disabled_tools: HashSet::new(),
    }
}

// ---------------------------------------------------------------------------
// Per-turn directives (reserved — behavioral guidance only)
// ---------------------------------------------------------------------------

/// Behavioral guidance appended to the system reminder for each persona.
///
/// This is **not** used for tool gating — tool availability is enforced at
/// the definition level via [`persona_tool_policy`].  The directives here
/// provide *how* the agent should work, not *what* it may do.
///
/// Currently unused (zero-token approach).  Kept for a potential future
/// pass where a short behavioral hint is desirable alongside the hard
/// permission rules.
#[allow(dead_code)]
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
///
/// The persona directive (from the shared `jcode_app_core` module) is appended
/// so local turns and daemon turns inject the same behavioral guidance.
pub(in crate::tui::app) fn merge_turn_reminder(
    base: Option<String>,
    mode: AgentMode,
) -> Option<String> {
    let persona =
        jcode_app_core::agent::persona::AgentPersona::from_key(mode.key()).unwrap_or_default();
    let directive = jcode_app_core::agent::persona::persona_directive(persona);
    match (base, directive) {
        (Some(base), Some(directive)) => Some(format!("{base}\n\n{directive}")),
        (Some(base), None) => Some(base),
        (None, Some(directive)) => Some(directive),
        (None, None) => None,
    }
}

// ---------------------------------------------------------------------------
// Plan persona tool gate
// ---------------------------------------------------------------------------

/// Outcome of gating one tool call under the active persona.
pub(in crate::tui::app) enum ToolGateDecision {
    /// Run the tool normally.
    Allow,
    /// Block until the user answers the in-chat permission panel.
    NeedsPermission { reason: String },
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
/// Under the permission-based approach, non-read tools are already hidden
/// from the model via [`persona_tool_policy`].  This function only handles
/// **secret-path reads**: files that look like secrets (.env, .pem, .ssh/…)
/// require explicit user approval even if the `read` tool is allowed.
///
/// For non-Plan personas, this always returns `Allow`.
pub(in crate::tui::app) fn gate_plan_tool(
    tool_name: &str,
    input_json: &serde_json::Value,
) -> ToolGateDecision {
    // Only the Plan persona has restricted access — Build and all others
    // pass everything through.  (We don't check self.agent_mode here
    // because the caller doesn't pass it; the caller is always the TUI
    // turn loop, which only calls this for the active persona.)
    //
    // For Plan: secret-path reads of the `read` tool raise the panel.
    let name_lower = tool_name.to_ascii_lowercase();
    if !name_lower.contains("read") {
        // Non-read tool: already blocked at definition level for Plan;
        // allowed for Build.  Nothing to gate here.
        return ToolGateDecision::Allow;
    }

    let path = ["file_path", "path", "absolute_path"]
        .iter()
        .find_map(|key| input_json.get(*key).and_then(|value| value.as_str()));
    if let Some(path) = path {
        let lowered = path.to_ascii_lowercase();
        if is_secret_path(&lowered) && !allowlisted(&lowered) {
            return ToolGateDecision::NeedsPermission {
                reason: format!("Plan agent wants to READ {path}"),
            };
        }
    }

    ToolGateDecision::Allow
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
    fn secret_path_gate_requires_permission_for_plan_reads() {
        use serde_json::json;

        // Non-read tools: always Allow (filtered at definition level)
        assert!(matches!(
            gate_plan_tool("write", &json!({ "file_path": "src/main.rs" })),
            ToolGateDecision::Allow
        ));
        assert!(matches!(
            gate_plan_tool("bash", &json!({ "command": "ls" })),
            ToolGateDecision::Allow
        ));

        // Read of normal file: Allow
        assert!(matches!(
            gate_plan_tool("read", &json!({ "file_path": "src/main.rs" })),
            ToolGateDecision::Allow
        ));

        // Read of secret file: NeedsPermission
        assert!(matches!(
            gate_plan_tool("read", &json!({ "file_path": ".env" })),
            ToolGateDecision::NeedsPermission { .. }
        ));
        assert!(matches!(
            gate_plan_tool("read", &json!({ "file_path": "/home/u/.ssh/id_rsa" })),
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
    fn persona_tool_policy_build_has_no_restrictions() {
        let policy = persona_tool_policy(AgentMode::Build);
        assert!(policy.allowed_tools.is_none());
        assert!(policy.disabled_tools.is_empty());
    }

    #[test]
    fn persona_tool_policy_plan_restricts_to_read_only() {
        let policy = persona_tool_policy(AgentMode::Plan);
        let allowed = policy.allowed_tools.unwrap();
        assert!(allowed.contains("read"));
        assert!(allowed.contains("agentgrep"));
        assert!(allowed.contains("ls"));
        assert!(allowed.contains("glob"));
        assert!(allowed.contains("memory"));
        assert!(!allowed.contains("write"));
        assert!(!allowed.contains("edit"));
        assert!(!allowed.contains("bash"));
        assert!(!allowed.contains("multiedit"));
        assert!(!allowed.contains("patch"));
        assert!(!allowed.contains("apply_patch"));
        assert!(!allowed.contains("browser"));
        assert!(!allowed.contains("open"));
    }

    #[test]
    fn persona_tool_policy_others_have_no_restrictions() {
        let policy = persona_tool_policy(AgentMode::Swarm);
        assert!(policy.allowed_tools.is_none());
    }
}
