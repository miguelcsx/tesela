//! Context window budget tracking and priority-based message retention.
//!
//! When the estimated token count exceeds the budget, messages are pruned
//! according to priority: system (never dropped) > recent user / assistant
//! > tool results > older messages (summarised or dropped).

use crate::constants::*;
use crate::ports::{Compactor, ModelProvider};
use crate::query::Message;
use tesela_core::Error;
use std::sync::Arc;

/// Default token budget when none is configured on the agent.
pub const DEFAULT_TOKEN_BUDGET: u32 = 16_000;

/// Priority rank for retention decisions.
/// Lower values are kept longer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    System = 0,
    RecentUser = 1,
    RecentAssistant = 2,
    ToolResult = 3,
    Older = 4,
}

impl Message {
    fn priority(&self, idx: usize, total: usize, _recent_window: usize) -> Priority {
        if self.role == ROLE_SYSTEM {
            return Priority::System;
        }
        let is_recent = idx + _recent_window >= total;
        if self.role == ROLE_USER && is_recent {
            return Priority::RecentUser;
        }
        if self.role == ROLE_ASSISTANT && is_recent {
            return Priority::RecentAssistant;
        }
        if self.role == ROLE_TOOL {
            return Priority::ToolResult;
        }
        Priority::Older
    }

    fn estimate_tokens(&self) -> usize {
        self.content.len() / CHARS_PER_TOKEN + 1
    }
}

/// Tracks token budget and compacts message history when exceeded.
pub struct ContextEngineer {
    budget: u32,
    summarizer: Arc<dyn ModelProvider>,
    recent_window: usize,
}

impl ContextEngineer {
    /// Create a new engineer with the given token budget.
    pub fn new(budget: u32, summarizer: Arc<dyn ModelProvider>) -> Self {
        Self {
            budget,
            summarizer,
            recent_window: 6,
        }
    }

    /// Estimate total tokens for a message slice.
    pub fn estimate_tokens(messages: &[Message]) -> usize {
        messages.iter().map(|m| m.estimate_tokens()).sum()
    }

    /// Whether the messages exceed the budget.
    pub fn is_over_budget(&self, messages: &[Message]) -> bool {
        Self::estimate_tokens(messages) > self.budget as usize
    }

    /// Summarise a batch of older messages into a single compact message.
    fn summarise(&self, window: &[Message]) -> Result<Message, Error> {
        let combined: String = window
            .iter()
            .map(|m| format!("[{}]: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let req = crate::query::ModelRequest {
            system: COMPACTION_SYSTEM_PROMPT.to_string(),
            messages: vec![{
                let mut map = std::collections::BTreeMap::new();
                map.insert("role".to_string(), tesela_core::Value::from(ROLE_USER));
                map.insert(
                    "content".to_string(),
                    tesela_core::Value::from(combined.as_str()),
                );
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

impl Compactor for ContextEngineer {
    fn compact(&self, messages: &[Message]) -> Result<Vec<Message>, Error> {
        if messages.is_empty() {
            return Ok(Vec::new());
        }

        let total = messages.len();
        let mut scored: Vec<(usize, Priority, usize)> = messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                (
                    i,
                    m.priority(i, total, self.recent_window),
                    m.estimate_tokens(),
                )
            })
            .collect();

        scored.sort_by_key(|&(_, p, _)| p);

        let mut keep = vec![false; total];
        let mut kept_tokens: usize = 0;

        // Keep system messages first, then recent, etc.
        for (idx, _, tokens) in &scored {
            if kept_tokens + *tokens <= self.budget as usize {
                keep[*idx] = true;
                kept_tokens += *tokens;
            }
        }

        let mut result = Vec::new();
        let mut dropped_window: Vec<&Message> = Vec::new();

        for (i, m) in messages.iter().enumerate() {
            if keep[i] {
                if !dropped_window.is_empty() {
                    // Summarise the dropped window before inserting the kept message
                    let to_summarise: Vec<Message> =
                        dropped_window.iter().map(|m| (*m).clone()).collect();
                    let summary = self.summarise(&to_summarise)?;
                    result.push(summary);
                    dropped_window.clear();
                }
                result.push(m.clone());
            } else {
                dropped_window.push(m);
            }
        }

        if !dropped_window.is_empty() {
            let to_summarise: Vec<Message> = dropped_window.iter().map(|m| (*m).clone()).collect();
            let summary = self.summarise(&to_summarise)?;
            result.push(summary);
        }

        Ok(result)
    }
}
