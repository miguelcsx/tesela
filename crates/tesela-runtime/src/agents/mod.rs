//! Agent runtime implementation.

mod default_runtime;
mod tools;

pub use default_runtime::{DefaultAgentRuntime, DefaultAgentRuntimeOptions};
pub use tools::AgentOrchestrator;

use crate::constants::*;
use crate::ports::{Compactor, ModelProvider};
use crate::query::{Message, ModelRequest};
use tesela_core::{Error, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Compacts a message history when the estimated token count exceeds budget.
///
/// Strategy:
/// 1. Count estimated tokens (characters / 4).
/// 2. If under budget, return the slice unchanged.
/// 3. Otherwise, summarise the oldest non-system window via the model provider
///    and replace it with a single "summary" message.
pub struct RollingCompactor {
    max_tokens: u32,
    summarizer: Arc<dyn ModelProvider>,
}

impl RollingCompactor {
    /// Create a new compactor.
    pub fn new(max_tokens: u32, summarizer: Arc<dyn ModelProvider>) -> Self {
        Self {
            max_tokens,
            summarizer,
        }
    }

    fn estimate_tokens(messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| m.content.len() / CHARS_PER_TOKEN + 1)
            .sum()
    }

    fn summarise(&self, window: &[Message]) -> Result<Message, Error> {
        let combined: String = window
            .iter()
            .map(|m| format!("[{}]: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let req = ModelRequest {
            system: COMPACTION_SYSTEM_PROMPT.to_string(),
            messages: vec![{
                let mut map = BTreeMap::new();
                map.insert("role".to_string(), Value::from(ROLE_USER));
                map.insert("content".to_string(), Value::from(combined.as_str()));
                map
            }],
            max_tokens: Some(DEFAULT_COMPACTION_MAX_TOKENS),
            ..Default::default()
        };

        let resp = self.summarizer.call(req)?;
        Ok(Message {
            role: ROLE_SYSTEM.to_string(),
            content: format!("[context summary] {}", resp.content),
            tool_calls: Vec::new(),
            tool_call_id: None,
        })
    }
}

impl Compactor for RollingCompactor {
    fn compact(&self, messages: &[Message]) -> Result<Vec<Message>, Error> {
        let mut result: Vec<Message> = messages.to_vec();

        while Self::estimate_tokens(&result) > self.max_tokens as usize {
            let start = result
                .iter()
                .position(|m| m.role != ROLE_SYSTEM)
                .unwrap_or(1);
            let end = (start + 10).min(result.len());
            if start >= end {
                break;
            }
            let summary = self.summarise(&result[start..end])?;
            result.splice(start..end, std::iter::once(summary));
        }

        Ok(result)
    }
}
