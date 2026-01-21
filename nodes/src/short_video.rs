use crate::llm_stream::LLMStreamObservable;
use async_trait::async_trait;
use flow::{Node, NodeInputs, ReactiveStream};
use futures::StreamExt;
use rig::{
    agent::{Agent, MultiTurnStreamItem},
    client::{CompletionClient, ProviderClient},
    message::{Reasoning, Text},
    providers::deepseek::{self, CompletionModel},
    streaming::{StreamedAssistantContent, StreamingPrompt},
};

const SYSTEM_PROMPT: &str = r#"
You are a professional short video script writer.
Your task is to generate a detailed script, character list, and storyboard based on the user's input.
You must output STRICTLY valid JSON content that matches the following TypeScript interface.

interface ScriptData {
  content: string;
}

interface CharacterData {
  id: string;
  name: string;
  avatar?: string;
  description: string;
  tags: string[];
}

interface StoryboardShot {
  id: string;
  shotNo: number;
  image?: string; // Leave empty or provide placeholder
  description: {
    background: string;
    relation: string;
    composition: string;
  };
  lines: {
    narration?: string;
    dialogue?: string;
  };
  mainCharacter: string;
  shotSize: string; // e.g., "特写", "近景", "中景"
  cameraAngle: string; // e.g., "视平", "俯平"
  lensType: string; // e.g., "单人镜头"
  duration: number; // seconds
}

interface ProjectData {
  script: ScriptData;
  characters: CharacterData[];
  storyboard: StoryboardShot[];
}
Do not include any markdown formatting (like ```json), just the raw JSON string.
"#;

pub struct ShortVideoScriptNode {
    llm: Agent<CompletionModel>,
    model: String,
}

impl ShortVideoScriptNode {
    pub fn new(model: &str) -> Self {
        dotenv::dotenv().ok();
        // Initialize the DeepSeek client from environment variables
        let client = deepseek::Client::from_env();
        let llm = client.agent(model).preamble(SYSTEM_PROMPT).build();

        Self {
            llm,
            model: model.to_owned(),
        }
    }
}

#[async_trait]
impl Node for ShortVideoScriptNode {
    type Out = ReactiveStream<String>;

    async fn run(&mut self, _ctx: &flow::Context, inputs: NodeInputs) -> Self::Out {
        let mut all_inputs = String::new();
        // Collect all inputs
        // NodeInputs wraps a HashMap, access via .inputs (if public) or we need another way.
        // Based on compiler suggestion, .inputs seems accessible.
        for (key, value) in inputs.inputs.iter() {
            // value is Box<dyn SendableAny>
            if let Some(s) = value.as_ref().as_any().downcast_ref::<String>() {
                if !all_inputs.is_empty() {
                    all_inputs.push_str("\n\n");
                }
                all_inputs.push_str(&format!("{}", s));
            }
        }

        if all_inputs.is_empty() {
            all_inputs = "Please generate a random interesting short video script.".to_string();
        }

        let prompt = format!(
            "Generate a short video script based on the following inputs:\n{}",
            all_inputs
        );

        let stream = self.llm.stream_prompt(&prompt).await.map(|res| match res {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                Text { text },
            ))) => Ok(text),
            // We ignore reasoning for this node to ensure valid JSON output
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(
                Reasoning { .. },
            ))) => Ok(String::new()),
            Ok(_) => Ok(String::new()),
            Err(e) => Err(e.to_string()),
        });

        let react_stream = LLMStreamObservable { stream };
        ReactiveStream::from_observable_with_accumulator(react_stream, |chunks: Vec<String>| {
            let full_text = chunks.join("");
            Some(Box::new(full_text))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::NodeInputs;
    use flow::{Context, TaskEvent};
    use rig::providers::deepseek::DEEPSEEK_CHAT;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    async fn collect_output(stream: ReactiveStream<String>) -> String {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let ctx = Arc::new(Context::new());
        let _sub = (stream.subscribe)(tx, "test".to_string(), ctx);

        let mut output = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Next(_, val) => {
                    if let Some(s) = val.as_any().downcast_ref::<String>() {
                        print!("{}", &s);
                        output.push_str(s);
                    }
                }
                TaskEvent::Completed(_, _) => break,
                TaskEvent::Error(_, e) => panic!("Stream error: {}", e),
                _ => {}
            }
        }
        output
    }

    #[test]
    fn test_short_video_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = ShortVideoScriptNode::new(DEEPSEEK_CHAT);
            let input = "Theme: Cyberpunk detective story".to_owned();

            let mut inputs = HashMap::new();
            inputs.insert(
                "theme".to_string(),
                Box::new(input) as Box<dyn flow::SendableAny>,
            );

            let stream = node.run(&ctx, NodeInputs::new(inputs)).await;
            let output = collect_output(stream).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
            // Basic JSON check
            assert!(output.trim().starts_with("{") || output.trim().starts_with("["));
        });
    }
}
