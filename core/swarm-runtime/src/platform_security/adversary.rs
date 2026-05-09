use anyhow::Result;

use crate::conversation::{ApiClient, ApiRequest, AssistantEvent};
use crate::session::{ConversationMessage, ContentBlock};
use crate::usage::TokenUsage;

const DEFAULT_RULES: &str = r#"BLOCK if the command:
- Exfiltrates data (curl/wget posting to unknown URLs, piping secrets out)
- Is destructive beyond the project scope (rm -rf /, modifying system files)
- Installs malware or runs obfuscated code
- Attempts to escalate privileges unnecessarily
- Downloads and executes untrusted remote scripts

ALLOW if the command is a normal development operation, even if it modifies files,
installs packages, runs tests, uses git, etc. Most commands are fine.
Err on the side of ALLOW — only block truly dangerous things."#;

pub struct AdversaryInspector<'a, C: ApiClient> {
    api_client: &'a mut C,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdversaryDecision {
    Allow { reason: String },
    Block { reason: String },
}

impl<'a, C: ApiClient> AdversaryInspector<'a, C> {
    pub fn new(api_client: &'a mut C) -> Self {
        Self { api_client }
    }

    pub fn inspect(
        &mut self,
        tool_name: &str,
        tool_input: &str,
        history: &[ConversationMessage],
    ) -> Result<AdversaryDecision> {
        let original_task = history
            .iter()
            .find(|m| m.role == crate::session::MessageRole::User)
            .and_then(|m| m.blocks.first())
            .map(|b| match b {
                ContentBlock::Text { text } => text.clone(),
                _ => "(non-text task)".to_string(),
            })
            .unwrap_or_else(|| "(unknown)".to_string());

        let system_prompt = "You are an adversarial security reviewer, protecting the user \
            in case the other agent is rogue. An AI coding agent is about to execute a tool \
            call. Your ONLY job: decide if this tool call is safe given the user's task and \
            rules. Respond with ALLOW or BLOCK on the first line, then a brief reason on \
            the next line.";

        let user_message = format!(
            "The user's original task: {original_task}\n\n\
             The tool call to review:\n\
             Tool: {tool_name}\n\
             Input: {tool_input}\n\n\
             Rules:\n{DEFAULT_RULES}\n\n\
             Respond with a single word on the first line: ALLOW or BLOCK\n\
             Then on the next line, a brief reason."
        );

        let request = ApiRequest {
            system_prompt: vec![system_prompt.to_string()],
            messages: vec![ConversationMessage::user_text(user_message)],
            agent_id: None,
        };

        // Note: SwarmCode's ApiClient is synchronous in the trait definition? 
        // Let's check conversation.rs again: fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
        // It's synchronous.
        let events = self.api_client.stream(request)?;
        
        let mut response_text = String::new();
        for event in events {
            if let AssistantEvent::TextDelta(delta) = event {
                response_text.push_str(&delta);
            }
        }

        let response_text = response_text.trim();
        let first_line = response_text.lines().next().unwrap_or("").to_uppercase();
        let reason = response_text.lines().skip(1).collect::<Vec<_>>().join(" ").trim().to_string();

        if first_line.contains("BLOCK") {
            Ok(AdversaryDecision::Block { 
                reason: if reason.is_empty() { "Blocked by security advisor".to_string() } else { reason } 
            })
        } else {
            Ok(AdversaryDecision::Allow { 
                reason: if reason.is_empty() { "Allowed by security advisor".to_string() } else { reason }
            })
        }
    }
}
