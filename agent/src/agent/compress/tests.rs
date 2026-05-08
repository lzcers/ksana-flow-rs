use async_trait::async_trait;
use serde_json::{Value, json};
use tempfile::tempdir;

use crate::agent::compress::{
    ChatSummaryModel, ConversationRule, LayerAction, LayerRule, LayerSelector, ModelCompression,
    RuleCompression, SummaryModel,
};
use crate::agent::{
    Context, FsMemoryStore, Layer, LayerKind, MemoryConfig, MemoryStore, ToolCall, ToolCallFunction,
};
use crate::core::Message;
use crate::models::{ChatCapability, ChatChunk, ChatError};
use futures::stream::{self, BoxStream};

fn conversation_with_tools_and_reasoning() -> Vec<Message> {
    vec![
        Message::system("Conversation system"),
        Message::user("Open the config"),
        Message::Assistant {
            content: String::new(),
            reasoning_content: Some("Need to inspect the config first".to_string()),
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                call_type: Some("function".to_string()),
                index: None,
                function: Some(ToolCallFunction {
                    name: "read_file".to_string(),
                    arguments: "{\"path\":\"config.json\"}".to_string(),
                }),
                name: None,
                arguments: None,
            }]),
        },
        Message::Tool {
            tool_call_id: "call_1".to_string(),
            content: "{\"debug\":true}".to_string(),
        },
        Message::Assistant {
            content: "Config loaded".to_string(),
            reasoning_content: Some("Debug mode is enabled".to_string()),
            tool_calls: None,
        },
        Message::user("Search handlers"),
        Message::Assistant {
            content: String::new(),
            reasoning_content: Some("Need grep across the repository".to_string()),
            tool_calls: Some(vec![ToolCall {
                id: "call_2".to_string(),
                call_type: Some("function".to_string()),
                index: None,
                function: Some(ToolCallFunction {
                    name: "file_search".to_string(),
                    arguments: "{\"pattern\":\"handler\"}".to_string(),
                }),
                name: None,
                arguments: None,
            }]),
        },
        Message::Tool {
            tool_call_id: "call_2".to_string(),
            content: "{\"matches\":[\"src/main.rs:12\"]}".to_string(),
        },
        Message::assistant("Found one handler"),
    ]
}

fn context_with_layers() -> Context {
    let mut readonly_memory = Layer::new(
        "readonly_memory",
        LayerKind::Memory,
        json!([{ "content": "do not edit" }]),
    );
    readonly_memory.meta.readonly = true;

    let mut tagged_notes = Layer::new(
        "notes",
        LayerKind::Custom("notes".to_string()),
        json!(["a", "b", "c", "d"]),
    );
    tagged_notes.meta.tags = vec!["trim".to_string(), "temp".to_string()];

    Context::new()
        .layer(Layer::new(
            "system",
            LayerKind::System,
            Value::String("Be concise".to_string()),
        ))
        .layer(readonly_memory)
        .layer(tagged_notes)
        .layer(Layer::new(
            "conversation",
            LayerKind::Conversation,
            serde_json::to_value(conversation_with_tools_and_reasoning())
                .expect("messages should serialize"),
        ))
}

struct StubSummaryModel;

#[async_trait]
impl SummaryModel for StubSummaryModel {
    async fn summarize(&self, prompt: &str) -> Result<String, ChatError> {
        assert!(prompt.contains("Open the config"));
        assert!(prompt.contains("assistant_tool_call: read_file"));
        Ok("Decided to inspect config first.\nNeed to revisit handlers.".to_string())
    }
}

struct StubChatModel {
    response: Message,
}

#[async_trait]
impl ChatCapability for StubChatModel {
    async fn chat(
        &self,
        msgs: Vec<Message>,
        _tools: Option<Vec<crate::agent::ToolDef>>,
    ) -> Result<Message, ChatError> {
        assert_eq!(msgs.len(), 2);
        assert!(matches!(msgs[0], Message::System { .. }));
        assert!(matches!(msgs[1], Message::User { .. }));
        Ok(self.response.clone())
    }

    async fn chat_stream(
        &self,
        _msgs: Vec<Message>,
        _tools: Option<Vec<crate::agent::ToolDef>>,
    ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
        Ok(Box::pin(stream::empty()))
    }
}

#[test]
fn rule_compression_applies_layer_rules_and_skips_readonly_by_default() {
    let context = context_with_layers();
    let rule = RuleCompression {
        layer_rules: vec![
            LayerRule {
                selector: LayerSelector {
                    tags: vec!["trim".to_string()],
                    ..LayerSelector::default()
                },
                action: LayerAction::TrimArray {
                    keep_head: 1,
                    keep_tail: 1,
                },
            },
            LayerRule {
                selector: LayerSelector {
                    kind: Some(LayerKind::Memory),
                    ..LayerSelector::default()
                },
                action: LayerAction::Clear,
            },
        ],
        conversation: None,
    };

    let compressed = context
        .compress_by_rule(&rule)
        .expect("rule compression should work");

    let notes = compressed.get("notes").expect("notes layer should remain");
    assert_eq!(notes.data, json!(["a", "d"]));

    let readonly = compressed
        .get("readonly_memory")
        .expect("readonly layer should remain untouched");
    assert_eq!(readonly.data, json!([{ "content": "do not edit" }]));
}

#[test]
fn rule_compression_trims_conversation_and_clears_old_tool_results() {
    let context = context_with_layers();
    let rule = RuleCompression {
        layer_rules: Vec::new(),
        conversation: Some(ConversationRule {
            keep_recent_messages: 6,
            keep_recent_tool_results: 1,
            clear_reasoning: true,
            keep_recent_reasoning_turns: 1,
        }),
    };

    let compressed = context
        .compress_by_rule(&rule)
        .expect("rule compression should work");
    let conversation = compressed.conversation();

    assert_eq!(conversation.len(), 7);
    assert!(matches!(conversation.first(), Some(Message::System { .. })));

    let old_tool = conversation
        .iter()
        .find(|message| matches!(message, Message::Tool { tool_call_id, .. } if tool_call_id == "call_1"))
        .expect("old tool result should still exist as placeholder");
    assert_eq!(old_tool.content(), "[Tool result cleared]");

    let recent_tool = conversation
        .iter()
        .find(|message| matches!(message, Message::Tool { tool_call_id, .. } if tool_call_id == "call_2"))
        .expect("recent tool result should remain");
    assert_eq!(recent_tool.content(), "{\"matches\":[\"src/main.rs:12\"]}");

    let cleared_reasoning = conversation
        .iter()
        .find(|message| matches!(message, Message::Assistant { content, .. } if content == "Config loaded"))
        .expect("older assistant message should remain");
    assert_eq!(cleared_reasoning.reasoning_content(), None);

    let recent_reasoning = conversation
        .iter()
        .find(|message| {
            matches!(
                message,
                Message::Assistant {
                    reasoning_content: Some(reasoning),
                    ..
                } if reasoning == "Need grep across the repository"
            )
        })
        .expect("most recent reasoning should remain");
    assert_eq!(
        recent_reasoning.reasoning_content(),
        Some("Need grep across the repository")
    );
}

#[test]
fn rule_compression_can_archive_cleared_tool_results() {
    let root = tempdir().expect("temp dir should exist");
    let mut memory =
        FsMemoryStore::new(MemoryConfig::new(root.path())).expect("memory store should exist");
    let context = context_with_layers();
    let rule = RuleCompression {
        layer_rules: Vec::new(),
        conversation: Some(ConversationRule {
            keep_recent_messages: usize::MAX,
            keep_recent_tool_results: 1,
            clear_reasoning: false,
            keep_recent_reasoning_turns: 1,
        }),
    };

    let compressed = context
        .compress_by_rule_with_archive(&rule, &mut memory)
        .expect("rule compression should work");
    let tool_message = compressed
        .conversation()
        .into_iter()
        .find(|message| matches!(message, Message::Tool { tool_call_id, .. } if tool_call_id == "call_1"))
        .expect("older tool message should remain");

    assert!(
        tool_message
            .content()
            .contains("memory_read('/memories/compression/tool_001_call_1.md')")
    );

    let listing = memory
        .view("/memories/compression", None)
        .expect("archive listing should work")
        .to_string();
    assert!(listing.contains("/memories/compression/tool_001_call_1.md"));
}

#[tokio::test(flavor = "current_thread")]
async fn model_compression_writes_summary_layer_and_keeps_recent_turns() {
    let context = context_with_layers();
    let options = ModelCompression {
        keep_recent_turns: 1,
        ..ModelCompression::default()
    };

    let compressed = context
        .compress_by_model(&StubSummaryModel, &options)
        .await
        .expect("model compression should work");

    let summary_layer = compressed
        .get("conversation_summary")
        .expect("summary layer should exist");
    assert_eq!(summary_layer.kind, LayerKind::Memory);
    assert!(
        summary_layer
            .meta
            .tags
            .iter()
            .any(|tag| tag == "compressed_summary")
    );

    let conversation = compressed.conversation();
    assert_eq!(conversation.len(), 5);
    assert!(matches!(conversation[0], Message::System { .. }));
    assert!(matches!(conversation[1], Message::User { .. }));
    assert_eq!(conversation[1].content(), "Search handlers");

    let output_messages = compressed.to_messages();
    assert!(matches!(output_messages[0], Message::System { .. }));
    assert!(matches!(output_messages[1], Message::User { .. }));
    let memory_content = output_messages[1].content();
    assert!(memory_content.contains("[Previous conversation summary]"));
    assert!(memory_content.contains("Decided to inspect config first."));
}

#[tokio::test(flavor = "current_thread")]
async fn model_compression_reuses_existing_summary_layer() {
    let mut context = context_with_layers();
    let mut existing_summary = Layer::new(
        "conversation_summary",
        LayerKind::Memory,
        json!([{ "content": "[Previous conversation summary]" }, { "content": "Earlier summary" }]),
    );
    existing_summary.meta.tags = vec!["compressed_summary".to_string()];
    context.layers.push(existing_summary);

    let options = ModelCompression {
        keep_recent_turns: 1,
        include_existing_summary: true,
        ..ModelCompression::default()
    };

    let compressed = context
        .compress_by_model(&StubSummaryModel, &options)
        .await
        .expect("model compression should work");

    let summary_layers = compressed
        .layers
        .iter()
        .filter(|layer| layer.name == "conversation_summary")
        .count();
    assert_eq!(summary_layers, 1);
}

#[tokio::test(flavor = "current_thread")]
async fn chat_summary_model_adapts_chat_capability() {
    let model = StubChatModel {
        response: Message::assistant("summary from chat model"),
    };
    let adapter = ChatSummaryModel::new(&model);

    let summary = adapter
        .summarize("Summarize this conversation.")
        .await
        .expect("chat adapter should summarize");

    assert_eq!(summary, "summary from chat model");
}
