//! The `ask_user` tool: an interactive clarifying question rendered as a popup
//! in the TUI. The running turn blocks until the user answers, so the question
//! genuinely pauses the agent — the key mechanism for Plan mode's clarifying
//! questions.
//!
//! The tool only works when the turn is executing inside an interactive client
//! session that has wired `ToolContext.ask_user_request_tx`. In headless or
//! non-interactive contexts it returns a clear error instead of hanging.

use super::{AskUserAnswer, AskUserOption, AskUserQuestion, Tool, ToolContext, ToolOutput};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct AskUserTool;

impl AskUserTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct AskUserInput {
    question: String,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default = "default_free_text")]
    free_text: bool,
}

fn default_free_text() -> bool {
    true
}

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a clarifying question. The user sees a popup and the turn \
         pauses until they answer. Prefer concrete multiple-choice options; set \
         free_text=true to also allow a typed answer. Use this whenever a \
         requirement, constraint, or expected behavior is ambiguous."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "intent": super::intent_schema_property(),
                "question": {
                    "type": "string",
                    "description": "The question to ask the user, shown verbatim in the popup."
                },
                "options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional short multiple-choice answers. The model receives the picked option string verbatim."
                },
                "free_text": {
                    "type": "boolean",
                    "description": "Whether the user may also type a free-text answer. Defaults to true."
                }
            },
            "required": ["question", "intent"]
        })
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let params: AskUserInput = serde_json::from_value(input)?;
        let question = params.question.trim().to_string();
        if question.is_empty() {
            return Err(anyhow!("ask_user requires a non-empty question"));
        }

        let Some(tx) = ctx.ask_user_request_tx.as_ref() else {
            return Err(anyhow!(
                "ask_user requires an interactive session; no connected client to ask."
            ));
        };

        let request_id = crate::id::new_id("ask_user");
        let options = params
            .options
            .into_iter()
            .map(|label| AskUserOption {
                label: label.clone(),
                value: label,
            })
            .collect::<Vec<_>>();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel::<AskUserAnswer>();
        tx.send(AskUserQuestion {
            request_id,
            question: question.clone(),
            options,
            free_text: params.free_text,
            response_tx,
        })?;

        let answer = response_rx.await?;
        let output = if answer.cancelled {
            format!("The user cancelled the question: {question}")
        } else {
            match answer.value {
                Some(value) => format!("User answer to {question:?}: {value}"),
                None => format!("The user cancelled the question: {question}"),
            }
        };
        Ok(ToolOutput::new(output).with_title("ask_user"))
    }
}