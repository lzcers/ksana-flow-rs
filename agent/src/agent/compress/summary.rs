use serde_json::{Value, json};

use super::conversation::{
    conversation_messages, primary_conversation_layer_index, split_by_user_turns,
};
use super::{CompressionError, ModelCompression, SummaryModel};
use crate::agent::{Context, Layer, LayerKind};
use crate::core::Message;

const DEFAULT_SUMMARIZE_INSTRUCTION: &str = "Preserve key decisions, constraints, pending work, and unresolved questions. Discard repetitive exploration and raw tool output unless it changes the state of the task.";
const DEFAULT_SUMMARY_LAYER_TAG: &str = "compressed_summary";

pub(super) async fn compress_by_model(
    context: &Context,
    model: &dyn SummaryModel,
    options: &ModelCompression,
) -> Result<Context, CompressionError> {
    let Some(conversation_index) = primary_conversation_layer_index(context) else {
        return Ok(context.clone());
    };

    let messages = conversation_messages(&context.layers[conversation_index])?;
    if messages.is_empty() {
        return Ok(context.clone());
    }

    let (system_prefix, turns) = split_by_user_turns(&messages);
    if turns.len() <= options.keep_recent_turns {
        return Ok(context.clone());
    }

    let split_index = turns.len().saturating_sub(options.keep_recent_turns);
    let old_messages: Vec<Message> = turns[..split_index]
        .iter()
        .flat_map(|turn| turn.clone())
        .collect();
    if old_messages.is_empty() {
        return Ok(context.clone());
    }

    let recent_messages: Vec<Message> = turns[split_index..]
        .iter()
        .flat_map(|turn| turn.clone())
        .collect();
    let prompt = build_summary_prompt(context, &old_messages, options);
    let summary = model.summarize(&prompt).await?;
    if summary.trim().is_empty() {
        return Err(CompressionError::EmptySummary);
    }

    let mut next = context.clone();
    next.layers[conversation_index].data = serde_json::to_value(
        system_prefix
            .into_iter()
            .chain(recent_messages.into_iter())
            .collect::<Vec<_>>(),
    )?;
    upsert_summary_layer(&mut next, &summary, options);
    Ok(next)
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
