use std::collections::BTreeSet;

use serde_json::Value;

use super::conversation::{
    conversation_messages, has_reasoning_content, is_tool_message,
    primary_conversation_layer_index, split_leading_system_messages,
};
use super::{CompressionError, ConversationRule, LayerAction, RuleCompression};
use crate::agent::{Context, Layer, MemoryError, MemoryStore};
use crate::core::Message;

const TOOL_RESULT_CLEARED: &str = "[Tool result cleared]";
const REASONING_CLEARED: &str = "[Reasoning cleared]";

pub(super) fn compress_by_rule(
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
