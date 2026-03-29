use std::collections::BTreeSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::agents::memory::{MemoryError, MemoryStore};
use crate::agents::{Context, Layer, LayerKind};
use crate::core::Message;
use crate::models::{ChatCapability, ChatError};

const DEFAULT_SUMMARY_LAYER_NAME: &str = "conversation_summary";
const DEFAULT_SUMMARY_LAYER_TAG: &str = "compressed_summary";
const DEFAULT_SUMMARY_PRIORITY: i32 = 25;
const DEFAULT_SUMMARIZE_INSTRUCTION: &str = "Preserve key decisions, constraints, pending work, and unresolved questions. Discard repetitive exploration and raw tool output unless it changes the state of the task.";
const DEFAULT_SUMMARY_SYSTEM_PROMPT: &str = "You compress layered agent context. Return only a concise summary that will help continue the task later.";
const TOOL_RESULT_CLEARED: &str = "[Tool result cleared]";
const REASONING_CLEARED: &str = "[Reasoning cleared]";

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("conversation layer `{0}` must contain a JSON array of messages")]
    InvalidConversationLayer(String),
    #[error("layer action `{action}` is incompatible with layer `{layer}`")]
    IncompatibleLayerAction { layer: String, action: String },
    #[error("summary model returned an empty summary")]
    EmptySummary,
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("memory error: {0}")]
    Memory(#[from] MemoryError),
    #[error("chat error: {0}")]
    Chat(#[from] ChatError),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleCompression {
    #[serde(default)]
    pub layer_rules: Vec<LayerRule>,
    #[serde(default)]
    pub conversation: Option<ConversationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerRule {
    pub selector: LayerSelector,
    pub action: LayerAction,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerSelector {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub kind: Option<LayerKind>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub include_readonly: bool,
}

impl LayerSelector {
    fn matches(&self, layer: &Layer) -> bool {
        if layer.meta.readonly && !self.include_readonly {
            return false;
        }

        if let Some(name) = &self.name
            && layer.name != *name
        {
            return false;
        }

        if let Some(kind) = &self.kind
            && layer.kind != *kind
        {
            return false;
        }

        self.tags
            .iter()
            .all(|tag| layer.meta.tags.iter().any(|candidate| candidate == tag))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayerAction {
    Drop,
    Clear,
    TrimArray { keep_head: usize, keep_tail: usize },
    Replace { value: Value },
}

impl LayerAction {
    fn label(&self) -> &'static str {
        match self {
            Self::Drop => "drop",
            Self::Clear => "clear",
            Self::TrimArray { .. } => "trim_array",
            Self::Replace { .. } => "replace",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRule {
    #[serde(default = "default_keep_recent_messages")]
    pub keep_recent_messages: usize,
    #[serde(default = "default_keep_recent_messages")]
    pub keep_recent_tool_results: usize,
    #[serde(default)]
    pub clear_reasoning: bool,
    #[serde(default = "default_keep_recent_reasoning_turns")]
    pub keep_recent_reasoning_turns: usize,
}

impl Default for ConversationRule {
    fn default() -> Self {
        Self {
            keep_recent_messages: default_keep_recent_messages(),
            keep_recent_tool_results: default_keep_recent_messages(),
            clear_reasoning: false,
            keep_recent_reasoning_turns: default_keep_recent_reasoning_turns(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCompression {
    #[serde(default = "default_keep_recent_turns")]
    pub keep_recent_turns: usize,
    #[serde(default = "default_summary_layer_name")]
    pub summary_layer_name: String,
    #[serde(default = "default_summary_priority")]
    pub summary_priority: i32,
    #[serde(default)]
    pub instruction: Option<String>,
    #[serde(default = "default_include_existing_summary")]
    pub include_existing_summary: bool,
}

impl Default for ModelCompression {
    fn default() -> Self {
        Self {
            keep_recent_turns: default_keep_recent_turns(),
            summary_layer_name: default_summary_layer_name(),
            summary_priority: default_summary_priority(),
            instruction: None,
            include_existing_summary: default_include_existing_summary(),
        }
    }
}

#[async_trait]
pub trait SummaryModel: Send + Sync {
    async fn summarize(&self, prompt: &str) -> Result<String, ChatError>;
}

pub struct ChatSummaryModel<'a, C> {
    chat: &'a C,
    system_prompt: String,
}

impl<'a, C> ChatSummaryModel<'a, C> {
    pub fn new(chat: &'a C) -> Self {
        Self {
            chat,
            system_prompt: DEFAULT_SUMMARY_SYSTEM_PROMPT.to_string(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }
}

#[async_trait]
impl<C> SummaryModel for ChatSummaryModel<'_, C>
where
    C: ChatCapability + Send + Sync,
{
    async fn summarize(&self, prompt: &str) -> Result<String, ChatError> {
        let response = self
            .chat
            .chat(
                vec![
                    Message::system(self.system_prompt.clone()),
                    Message::user(prompt),
                ],
                None,
            )
            .await?;

        match response {
            Message::Assistant { content, .. } if !content.trim().is_empty() => Ok(content),
            _ => Err(ChatError::NoResponse),
        }
    }
}

impl Context {
    pub fn compress_by_rule(&self, rule: &RuleCompression) -> Result<Self, CompressionError> {
        compress_by_rule_internal(self, rule, None)
    }

    pub fn compress_by_rule_with_archive(
        &self,
        rule: &RuleCompression,
        memory: &mut dyn MemoryStore,
    ) -> Result<Self, CompressionError> {
        compress_by_rule_internal(self, rule, Some(memory))
    }

    pub async fn compress_by_model(
        &self,
        model: &dyn SummaryModel,
        options: &ModelCompression,
    ) -> Result<Self, CompressionError> {
        let Some(conversation_index) = primary_conversation_layer_index(self) else {
            return Ok(self.clone());
        };

        let messages = conversation_messages(&self.layers[conversation_index])?;
        if messages.is_empty() {
            return Ok(self.clone());
        }

        let (system_prefix, turns) = split_by_user_turns(&messages);
        if turns.len() <= options.keep_recent_turns {
            return Ok(self.clone());
        }

        let split_index = turns.len().saturating_sub(options.keep_recent_turns);
        let old_messages: Vec<Message> = turns[..split_index]
            .iter()
            .flat_map(|turn| turn.clone())
            .collect();
        if old_messages.is_empty() {
            return Ok(self.clone());
        }

        let recent_messages: Vec<Message> = turns[split_index..]
            .iter()
            .flat_map(|turn| turn.clone())
            .collect();
        let prompt = build_summary_prompt(self, &old_messages, options);
        let summary = model.summarize(&prompt).await?;
        if summary.trim().is_empty() {
            return Err(CompressionError::EmptySummary);
        }

        let mut next = self.clone();
        next.layers[conversation_index].data = serde_json::to_value(
            system_prefix
                .into_iter()
                .chain(recent_messages.into_iter())
                .collect::<Vec<_>>(),
        )?;
        upsert_summary_layer(&mut next, &summary, options);
        Ok(next)
    }
}

fn compress_by_rule_internal(
    context: &Context,
    rule: &RuleCompression,
    memory: Option<&mut dyn MemoryStore>,
) -> Result<Context, CompressionError> {
    let mut layers = Vec::with_capacity(context.layers.len());
    'layer: for layer in &context.layers {
        let mut current = layer.clone();
        for layer_rule in &rule.layer_rules {
            if !layer_rule.selector.matches(&current) {
                continue;
            }

            match apply_layer_action(&mut current, &layer_rule.action)? {
                LayerDisposition::Keep => {}
                LayerDisposition::Drop => continue 'layer,
            }
        }
        layers.push(current);
    }

    let mut next = Context { layers };
    if let Some(conversation_rule) = &rule.conversation
        && let Some(index) = primary_conversation_layer_index(&next)
    {
        let messages = conversation_messages(&next.layers[index])?;
        let compressed = compress_conversation_messages(messages, conversation_rule, memory)?;
        next.layers[index].data = serde_json::to_value(compressed)?;
    }

    Ok(next)
}

enum LayerDisposition {
    Keep,
    Drop,
}

fn apply_layer_action(
    layer: &mut Layer,
    action: &LayerAction,
) -> Result<LayerDisposition, CompressionError> {
    match action {
        LayerAction::Drop => Ok(LayerDisposition::Drop),
        LayerAction::Clear => {
            layer.data = clear_value(&layer.data);
            Ok(LayerDisposition::Keep)
        }
        LayerAction::TrimArray {
            keep_head,
            keep_tail,
        } => {
            let Value::Array(items) = &layer.data else {
                return Err(CompressionError::IncompatibleLayerAction {
                    layer: layer.name.clone(),
                    action: action.label().to_string(),
                });
            };

            if keep_head.saturating_add(*keep_tail) >= items.len() {
                return Ok(LayerDisposition::Keep);
            }

            let mut trimmed = items.iter().take(*keep_head).cloned().collect::<Vec<_>>();
            let tail_start = items.len().saturating_sub(*keep_tail);
            trimmed.extend(items.iter().skip(tail_start).cloned());
            layer.data = Value::Array(trimmed);
            Ok(LayerDisposition::Keep)
        }
        LayerAction::Replace { value } => {
            layer.data = value.clone();
            Ok(LayerDisposition::Keep)
        }
    }
}

fn clear_value(value: &Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(_) => Value::Null,
        Value::Number(_) => Value::Null,
        Value::String(_) => Value::String(String::new()),
        Value::Array(_) => Value::Array(Vec::new()),
        Value::Object(_) => Value::Object(Default::default()),
    }
}

fn primary_conversation_layer_index(context: &Context) -> Option<usize> {
    context
        .layers
        .iter()
        .position(|layer| layer.kind == LayerKind::Conversation)
}

fn conversation_messages(layer: &Layer) -> Result<Vec<Message>, CompressionError> {
    let Value::Array(items) = &layer.data else {
        return Err(CompressionError::InvalidConversationLayer(
            layer.name.clone(),
        ));
    };

    Ok(items
        .iter()
        .filter_map(|item| serde_json::from_value::<Message>(item.clone()).ok())
        .collect())
}

fn compress_conversation_messages(
    messages: Vec<Message>,
    rule: &ConversationRule,
    mut memory: Option<&mut dyn MemoryStore>,
) -> Result<Vec<Message>, CompressionError> {
    let (system_prefix, non_system_messages) = split_leading_system_messages(&messages);
    let retained_messages = trim_recent_messages(non_system_messages, rule.keep_recent_messages);
    let keep_tool_indices = recent_matching_indices(
        &retained_messages,
        rule.keep_recent_tool_results,
        is_tool_message,
    );
    let keep_reasoning_indices = recent_matching_indices(
        &retained_messages,
        rule.keep_recent_reasoning_turns,
        has_reasoning_content,
    );

    let mut compressed = system_prefix;
    let mut cleared_tool_count = 0usize;

    for (index, message) in retained_messages.into_iter().enumerate() {
        match message {
            Message::Tool {
                tool_call_id,
                content,
            } => {
                if keep_tool_indices.contains(&index) {
                    compressed.push(Message::Tool {
                        tool_call_id,
                        content,
                    });
                    continue;
                }

                cleared_tool_count += 1;
                let placeholder = if let Some(store) = memory.as_deref_mut() {
                    let archive_path =
                        archive_tool_result(store, cleared_tool_count, &tool_call_id, &content)?;
                    format!("[Tool result cleared. Use memory_read('{archive_path}') to retrieve.]")
                } else {
                    TOOL_RESULT_CLEARED.to_string()
                };

                compressed.push(Message::Tool {
                    tool_call_id,
                    content: placeholder,
                });
            }
            Message::Assistant {
                content,
                reasoning_content,
                tool_calls,
            } => {
                if rule.clear_reasoning
                    && reasoning_content
                        .as_deref()
                        .is_some_and(|text| !text.trim().is_empty())
                    && !keep_reasoning_indices.contains(&index)
                {
                    compressed.push(Message::Assistant {
                        content: if content.trim().is_empty() {
                            REASONING_CLEARED.to_string()
                        } else {
                            content
                        },
                        reasoning_content: None,
                        tool_calls,
                    });
                } else {
                    compressed.push(Message::Assistant {
                        content,
                        reasoning_content,
                        tool_calls,
                    });
                }
            }
            other => compressed.push(other),
        }
    }

    Ok(compressed)
}

fn split_leading_system_messages(messages: &[Message]) -> (Vec<Message>, Vec<Message>) {
    let prefix_len = messages
        .iter()
        .take_while(|message| is_system_message(message))
        .count();
    (
        messages[..prefix_len].to_vec(),
        messages[prefix_len..].to_vec(),
    )
}

fn trim_recent_messages(messages: Vec<Message>, limit: usize) -> Vec<Message> {
    if limit == usize::MAX || messages.len() <= limit {
        return messages;
    }

    let skip = messages.len() - limit;
    messages.into_iter().skip(skip).collect()
}

fn recent_matching_indices<F>(messages: &[Message], keep: usize, predicate: F) -> BTreeSet<usize>
where
    F: Fn(&Message) -> bool,
{
    if keep == usize::MAX {
        return messages
            .iter()
            .enumerate()
            .filter_map(|(index, message)| predicate(message).then_some(index))
            .collect();
    }

    messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| predicate(message).then_some(index))
        .rev()
        .take(keep)
        .collect()
}

fn archive_tool_result(
    memory: &mut dyn MemoryStore,
    cleared_count: usize,
    tool_call_id: &str,
    content: &str,
) -> Result<String, CompressionError> {
    let stem = if tool_call_id.trim().is_empty() {
        format!("tool_{cleared_count:03}")
    } else {
        format!(
            "tool_{cleared_count:03}_{}",
            sanitize_identifier(tool_call_id)
        )
    };

    let mut attempt = 0usize;
    loop {
        let path = if attempt == 0 {
            format!("/memories/compression/{stem}.md")
        } else {
            format!("/memories/compression/{stem}_{attempt}.md")
        };

        match memory.create(&path, content) {
            Ok(()) => return Ok(path),
            Err(MemoryError::FileExists(_)) => attempt += 1,
            Err(err) => return Err(err.into()),
        }
    }
}

fn sanitize_identifier(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitized.is_empty() {
        "tool".to_string()
    } else {
        sanitized
    }
}

fn build_summary_prompt(
    context: &Context,
    old_messages: &[Message],
    options: &ModelCompression,
) -> String {
    let mut sections = Vec::new();
    if options.include_existing_summary
        && let Some(existing_summary) = existing_summary_text(context, &options.summary_layer_name)
        && !existing_summary.trim().is_empty()
    {
        sections.push(format!("Existing summary:\n{existing_summary}"));
    }

    let transcript = old_messages
        .iter()
        .flat_map(transcript_lines)
        .collect::<Vec<_>>()
        .join("\n");

    if !transcript.trim().is_empty() {
        sections.push(format!("Conversation:\n{transcript}"));
    }

    let instruction = options
        .instruction
        .as_deref()
        .unwrap_or(DEFAULT_SUMMARIZE_INSTRUCTION);

    format!(
        "Summarize the compressed portion of the conversation.\n{instruction}\n\n{}\n\nSummary:",
        sections.join("\n\n")
    )
}

fn transcript_lines(message: &Message) -> Vec<String> {
    match message {
        Message::System { content } => {
            if content.trim().is_empty() {
                Vec::new()
            } else {
                vec![format!("system: {content}")]
            }
        }
        Message::User { content } => {
            if content.trim().is_empty() {
                Vec::new()
            } else {
                vec![format!("user: {content}")]
            }
        }
        Message::Assistant {
            content,
            tool_calls,
            ..
        } => {
            let mut lines = Vec::new();
            if !content.trim().is_empty() {
                lines.push(format!("assistant: {content}"));
            }
            if let Some(tool_calls) = tool_calls {
                for call in tool_calls {
                    let name = call.get_name();
                    if name.is_empty() {
                        continue;
                    }
                    lines.push(format!(
                        "assistant_tool_call: {} {}",
                        name,
                        call.get_arguments()
                    ));
                }
            }
            lines
        }
        Message::Tool {
            tool_call_id,
            content,
        } => {
            if content.trim().is_empty() {
                Vec::new()
            } else if tool_call_id.trim().is_empty() {
                vec![format!("tool: {content}")]
            } else {
                vec![format!("tool[{tool_call_id}]: {content}")]
            }
        }
    }
}

fn existing_summary_text(context: &Context, layer_name: &str) -> Option<String> {
    let layer = context.get(layer_name)?;
    memory_layer_text(layer)
}

fn memory_layer_text(layer: &Layer) -> Option<String> {
    match &layer.data {
        Value::String(text) if !text.trim().is_empty() => Some(text.clone()),
        Value::Array(items) => {
            let lines = items
                .iter()
                .filter_map(|item| {
                    item.as_object()
                        .and_then(|map| map.get("content"))
                        .and_then(Value::as_str)
                        .map(|text| text.trim().to_string())
                        .filter(|text| !text.is_empty())
                })
                .collect::<Vec<_>>();
            (!lines.is_empty()).then(|| lines.join("\n"))
        }
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn upsert_summary_layer(context: &mut Context, summary: &str, options: &ModelCompression) {
    context
        .layers
        .retain(|layer| layer.name != options.summary_layer_name);
    context.layers.push(build_summary_layer(summary, options));
}

fn build_summary_layer(summary: &str, options: &ModelCompression) -> Layer {
    let mut entries = vec![json!({ "content": "[Previous conversation summary]" })];
    entries.extend(
        summary
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| json!({ "content": line })),
    );

    let mut layer = Layer::new(
        options.summary_layer_name.clone(),
        LayerKind::Memory,
        Value::Array(entries),
    );
    layer.meta.priority = options.summary_priority;
    layer.meta.tags.push(DEFAULT_SUMMARY_LAYER_TAG.to_string());
    layer
}

fn split_by_user_turns(messages: &[Message]) -> (Vec<Message>, Vec<Vec<Message>>) {
    let (system_prefix, rest) = split_leading_system_messages(messages);
    let mut turns = Vec::new();
    let mut current_turn = Vec::new();

    for message in rest {
        if is_user_message(&message) && !current_turn.is_empty() {
            turns.push(current_turn);
            current_turn = Vec::new();
        }

        current_turn.push(message);
    }

    if !current_turn.is_empty() {
        turns.push(current_turn);
    }

    (system_prefix, turns)
}

fn is_system_message(message: &Message) -> bool {
    matches!(message, Message::System { .. })
}

fn is_user_message(message: &Message) -> bool {
    matches!(message, Message::User { .. })
}

fn is_tool_message(message: &Message) -> bool {
    matches!(message, Message::Tool { .. })
}

fn has_reasoning_content(message: &Message) -> bool {
    matches!(
        message,
        Message::Assistant {
            reasoning_content: Some(reasoning_content),
            ..
        } if !reasoning_content.trim().is_empty()
    )
}

fn default_keep_recent_messages() -> usize {
    usize::MAX
}

fn default_keep_recent_reasoning_turns() -> usize {
    1
}

fn default_keep_recent_turns() -> usize {
    2
}

fn default_summary_layer_name() -> String {
    DEFAULT_SUMMARY_LAYER_NAME.to_string()
}

fn default_summary_priority() -> i32 {
    DEFAULT_SUMMARY_PRIORITY
}

fn default_include_existing_summary() -> bool {
    true
}

#[cfg(test)]
mod tests;
