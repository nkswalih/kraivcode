//! Shared agent personas: the single source of truth for persona keys, the
//! per-persona tool policy (read-only Plan enforcement), and behavioral
//! directives. Both the daemon (jcode-app-core) and the TUI client
//! (jcode-tui) read from here so the two can never drift apart.
//!
//! Tool availability is enforced at the **definition level**: the Plan persona
//! declares an allowlist that filters the tool list sent to the provider, so
//! the model never even sees tools it may not use. The daemon additionally
//! re-checks the allowlist at tool execution time as defense-in-depth.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Operating personas for an agent session.
///
/// Wire keys are lowercase snake-case strings. The client sends the active
/// persona key on every message; the daemon stores it as the session persona
/// and applies its tool policy for that and all subsequent turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentPersona {
    #[default]
    Build,
    Plan,
    Swarm,
    Review,
    Judge,
    Memory,
    Ambient,
}

impl AgentPersona {
    pub fn key(self) -> &'static str {
        match self {
            AgentPersona::Build => "build",
            AgentPersona::Plan => "plan",
            AgentPersona::Swarm => "swarm",
            AgentPersona::Review => "review",
            AgentPersona::Judge => "judge",
            AgentPersona::Memory => "memory",
            AgentPersona::Ambient => "ambient",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_ascii_lowercase().as_str() {
            "build" => Some(AgentPersona::Build),
            "plan" => Some(AgentPersona::Plan),
            "swarm" => Some(AgentPersona::Swarm),
            "review" => Some(AgentPersona::Review),
            "judge" => Some(AgentPersona::Judge),
            "memory" => Some(AgentPersona::Memory),
            "ambient" => Some(AgentPersona::Ambient),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AgentPersona::Build => "Build",
            AgentPersona::Plan => "Plan",
            AgentPersona::Swarm => "Swarm",
            AgentPersona::Review => "Review",
            AgentPersona::Judge => "Judge",
            AgentPersona::Memory => "Memory",
            AgentPersona::Ambient => "Ambient",
        }
    }

    pub fn is_plan(self) -> bool {
        matches!(self, AgentPersona::Plan)
    }

    /// Whether this persona restricts the available tool set at all.
    pub fn restricts_tools(self) -> bool {
        self.is_plan()
    }
}

/// Tool-policy outcome for a persona.
///
/// `allowed_tools: None` = every registered tool the config permits is visible.
/// `Some(set)` = only tools matching the set are visible.
#[derive(Debug, Clone)]
pub struct PersonaToolPolicy {
    pub allowed_tools: Option<HashSet<String>>,
}

/// Return the tool-policy ruleset for the given persona.
///
/// - **Build** (default): the full session tool surface (`None` allowlist).
/// - **Plan**: read-only tools plus `ask_user`, `todo`, `side_panel`, memory,
///   `bg`, `initiative`, `schedule`, and the `mcp` management tool. Write and
///   execution tools are hidden from the model entirely.
/// - All other personas: full access (future work may restrict these).
pub fn persona_tool_policy(persona: AgentPersona) -> PersonaToolPolicy {
    match persona {
        AgentPersona::Build | AgentPersona::Swarm | AgentPersona::Review | AgentPersona::Judge
        | AgentPersona::Memory | AgentPersona::Ambient => PersonaToolPolicy {
            allowed_tools: None,
        },
        AgentPersona::Plan => {
            let mut allowed = HashSet::<String>::new();
            // File / code reading
            allowed.insert("read".into());
            allowed.insert("glob".into());
            allowed.insert("ls".into());
            // Code search
            allowed.insert("agentgrep".into());
            allowed.insert("session_search".into());
            allowed.insert("conversation_search".into());
            // Memory / knowledge
            allowed.insert("memory".into());
            // UI / scratchpad
            allowed.insert("side_panel".into());
            allowed.insert("todo".into());
            // Docs
            allowed.insert("jcode_docs".into());
            // Web (read-only research)
            allowed.insert("webfetch".into());
            allowed.insert("websearch".into());
            // Background / scheduling (non-mutating)
            allowed.insert("bg".into());
            allowed.insert("initiative".into());
            allowed.insert("schedule".into());
            // MCP connection management (individual `mcp__*` tools stay hidden)
            allowed.insert("mcp".into());
            // Interactive clarifying questions (plan-mode popup)
            allowed.insert("ask_user".into());
            PersonaToolPolicy {
                allowed_tools: Some(allowed),
            }
        }
    }
}

/// Marker `tool_call_id` used for the synthesized post-turn Plan-followup popup
/// (prose-question fallback). The TUI treats this specially: a free-text answer
/// is re-submitted to the session as the next user turn.
pub const PLAN_FOLLOWUP_TOOL_CALL_ID: &str = "__plan_followup__";
/// Sentinel option value meaning "nothing to follow up" in that popup.
pub const PLAN_FOLLOWUP_CONTINUE_VALUE: &str = "__continue__";
/// Max characters shown for the extracted trailing question in that popup.
pub const PLAN_FOLLOWUP_MAX_CHARS: usize = 280;

/// Options rendered in the synthesized Plan-followup popup.
pub fn plan_followup_options() -> Vec<(String, String)> {
    vec![(
        "Continue".to_string(),
        PLAN_FOLLOWUP_CONTINUE_VALUE.to_string(),
    )]
}

/// Extract the trailing question from a completed Plan turn's text, when the
/// agent left a prose question instead of calling `ask_user`. Returns the
/// question snippet (ending in `?`) if the text ends with an open question.
pub fn extract_trailing_question(text: &str, max_chars: usize) -> Option<String> {
    let trimmed = text.trim_end();
    let last_question = trimmed.rfind('?')?;
    // Sentence start: after the previous sentence-terminating run.
    let mut start = last_question;
    while start > 0 {
        let before = trimmed[..start].chars().next_back().unwrap_or_default();
        if matches!(before, '?' | '!' | '.') || before == '\n' {
            break;
        }
        start -= before.len_utf8();
    }
    let mut question = trimmed[start..last_question + 1].trim().to_string();
    if question.len() > max_chars {
        let tail_start = question.len() - max_chars;
        question = format!("…{}", &question[tail_start..]);
    }
    Some(question)
}

/// Behavioral directive injected into the system content of turns run under a
/// persona. Plan gets a strong, plan-card-oriented directive because the plan
/// output format is part of the requested fix; Build and the rest get a short
/// label only (the hard enforcement is the tool allowlist, not the words here).
pub fn persona_directive(persona: AgentPersona) -> Option<String> {
    match persona {
        AgentPersona::Plan => Some(
            "# Persona: Plan (READ-ONLY)\n\n\
             You are in Plan mode. Your job is to research the request and produce an \
             implementation plan — never to modify anything.\n\n\
             1. Investigate thoroughly using read-only tools (read, ls, agentgrep, \
             session_search, web research).\n\
             2. There is NO text-based Q&A in this interface: the user cannot answer \
             prose questions, so an open question at the end of your reply is dead \
             text. Whenever you need the user to decide, clarify, or confirm — \
             including offers like \"Want me to ...?\" — you MUST call the ask_user \
             tool (concise options + free_text=true for a typed answer) BEFORE \
             finishing your turn, then stop. Never end a Plan reply with a \"?\" in \
             prose; the daemon will re-ask any trailing question it detects through \
             the popup, but planning should use ask_user directly.\n\
             3. Deliver the plan as a structured plan card: Goal, Approach, Phases \
             (each with concrete steps, files, and risks), and Open questions.\n\n\
             Write/edit/mutation and command execution tools are BLOCKED by the daemon \
             in this mode. Do not attempt to call them."
                .to_string(),
        ),
        AgentPersona::Build => Some(
            "# Persona: Build\n\n\
             You are in Build mode. Work like a senior engineer: clarify ambiguity first, \
             then implement in small verified phases and summarize what changed."
                .to_string(),
        ),
        AgentPersona::Swarm | AgentPersona::Review | AgentPersona::Judge | AgentPersona::Memory
        | AgentPersona::Ambient => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persona_keys_roundtrip() {
        for persona in [
            AgentPersona::Build,
            AgentPersona::Plan,
            AgentPersona::Swarm,
            AgentPersona::Review,
            AgentPersona::Judge,
            AgentPersona::Memory,
            AgentPersona::Ambient,
        ] {
            assert_eq!(AgentPersona::from_key(persona.key()), Some(persona));
        }
        assert_eq!(AgentPersona::from_key("PLAN"), Some(AgentPersona::Plan));
        assert_eq!(AgentPersona::from_key("nope"), None);
    }

    #[test]
    fn build_persona_has_no_tool_restrictions() {
        assert!(persona_tool_policy(AgentPersona::Build).allowed_tools.is_none());
    }

    #[test]
    fn plan_persona_is_read_only_plus_ask_user() {
        let allowed = persona_tool_policy(AgentPersona::Plan).allowed_tools.unwrap();
        for write_tool in ["write", "edit", "multiedit", "patch", "apply_patch", "bash", "browser", "open"] {
            assert!(!allowed.contains(write_tool), "{write_tool} must be blocked");
        }
        for read_tool in [
            "read", "ls", "agentgrep", "session_search", "conversation_search", "memory",
            "side_panel", "todo", "jcode_docs", "webfetch", "websearch", "bg", "initiative",
            "schedule", "mcp", "ask_user",
        ] {
            assert!(allowed.contains(read_tool), "{read_tool} must be allowed");
        }
    }
}