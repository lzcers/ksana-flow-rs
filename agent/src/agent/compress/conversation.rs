use serde_json::Value;

use super::CompressionError;
use crate::agent::{Context, Layer, LayerKind};
use crate::core::Message;

pub(super) fn primary_conversation_layer_index(context: &Context) -> Option<usize> {
    context
        .layers
        .iter()
        .position(|layer| layer.kind == LayerKind::Conversation)
}

pub(super) fn conversation_messages(layer: &Layer) -> Result<Vec<Message>, CompressionError> {
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

pub(super) fn split_leading_system_messages(messages: &[Message]) -> (Vec<Message>, Vec<Message>) {
    let prefix_len = messages
        .iter()
        .take_while(|message| is_system_message(message))
        .count();
    (
        messages[..prefix_len].to_vec(),
        messages[prefix_len..].to_vec(),
    )
}

pub(super) fn split_by_user_turns(messages: &[Message]) -> (Vec<Message>, Vec<Vec<Message>>) {
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

pub(super) fn is_tool_message(message: &Message) -> bool {
    matches!(message, Message::Tool { .. })
}

pub(super) fn has_reasoning_content(message: &Message) -> bool {
    matches!(
        message,
        Message::Assistant {
            reasoning_content: Some(reasoning_content),
            ..
        } if !reasoning_content.trim().is_empty()
    )
}

fn is_system_message(message: &Message) -> bool {
    matches!(message, Message::System { .. })
}

fn is_user_message(message: &Message) -> bool {
    matches!(message, Message::User { .. })
}
