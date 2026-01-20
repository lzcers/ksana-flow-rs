use async_trait::async_trait;
use flow::{
    Node, NodeInputs, ReactiveStream,
    observable::{Observable, Observer, VecSubscription},
};
use futures::StreamExt;
use rig::{
    agent::{Agent, MultiTurnStreamItem},
    client::{CompletionClient, ProviderClient},
    message::{Reasoning, Text},
    providers::deepseek::{self, CompletionModel, DEEPSEEK_CHAT},
    streaming::{StreamedAssistantContent, StreamingPrompt},
};

pub struct LLMStreamObservable<S> {
    stream: S,
}

#[async_trait]
impl<S> Observable<String, String> for LLMStreamObservable<S>
where
    S: futures::Stream<Item = Result<String, String>> + Send + Unpin + 'static,
{
    type Sub = VecSubscription;

    async fn subscribe(mut self, mut observer: impl Observer<String, String>) -> Self::Sub {
        let mut stream = self.stream;

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => observer.on_next(chunk).await,
                Err(e) => {
                    observer.on_error(e).await;
                    return VecSubscription;
                }
            }
        }
        observer.on_completed().await;
        VecSubscription
    }
}

pub struct LLMStreamNode {
    llm: Agent<CompletionModel>,
    #[allow(dead_code)]
    system_prompt: String,
    user_prompt_template: String,
}

impl LLMStreamNode {
    pub fn new(sys_prompt: &str, user_tmpl: &str) -> Self {
        dotenv::dotenv().ok();
        // Initialize the DeepSeek client from environment variables
        let client = deepseek::Client::from_env();
        let mut builder = client.agent(DEEPSEEK_CHAT);

        // Handle system prompt
        if !sys_prompt.is_empty() {
            builder = builder.preamble(&sys_prompt);
        }

        let llm = builder.build();

        Self {
            llm,
            system_prompt: sys_prompt.to_owned(),
            user_prompt_template: user_tmpl.to_owned(),
        }
    }
}

#[async_trait]
impl Node for LLMStreamNode {
    type Out = ReactiveStream<String>;

    async fn run(&mut self, _ctx: &flow::Context, inputs: NodeInputs) -> Self::Out {
        let input = inputs
            .get_any()
            .and_then(|any| any.as_ref().as_any().downcast_ref::<String>())
            .cloned()
            .unwrap_or_default();

        let prompt = if !input.is_empty() {
            if self.user_prompt_template.contains("{input}") {
                self.user_prompt_template.replace("{input}", &input)
            } else {
                input
            }
        } else {
            self.user_prompt_template.clone()
        };

        let stream = self.llm.stream_prompt(&prompt).await.map(|res| match res {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                Text { text },
            ))) => Ok(text),
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(
                Reasoning { reasoning, .. },
            ))) => Ok(reasoning.join("\n")),
            Ok(_) => Ok(String::new()),
            Err(e) => Err(e.to_string()),
        });
        let react_stream = LLMStreamObservable { stream };
        ReactiveStream::from_observable(react_stream)
    }
}

#[cfg(test)]
mod tests {
    use flow::NodeInputs;
    use flow::{Context, TaskEvent};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    use super::*;

    async fn collect_output(stream: ReactiveStream<String>) -> String {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let ctx = Arc::new(Context::new());
        let _sub = (stream.subscribe)(tx, "test".to_string(), ctx);

        let mut output = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Next(_, val) => {
                    if let Some(s) = val.as_any().downcast_ref::<String>() {
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
    fn test_llm_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMStreamNode::new("", "");
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs = HashMap::new();
            inputs.insert(
                "test".to_string(),
                Box::new(input) as Box<dyn flow::SendableAny>,
            );

            let stream = node.run(&ctx, NodeInputs::new(inputs)).await;
            let output = collect_output(stream).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }

    #[test]
    fn test_llm_node_with_template() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            // Template with {input} placeholder
            let mut node = LLMStreamNode::new(
                "You are a helpful translator.",
                "Translate this to English: {input}",
            );
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs = HashMap::new();
            inputs.insert(
                "test".to_string(),
                Box::new(input) as Box<dyn flow::SendableAny>,
            );

            let stream = node.run(&ctx, NodeInputs::new(inputs)).await;
            let output = collect_output(stream).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }

    #[test]
    fn test_llm_node_empty_input() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            // Template without placeholder, used when input is empty
            let mut node = LLMStreamNode::new("", "Tell me a joke");
            let input = "".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs = HashMap::new();
            inputs.insert(
                "test".to_string(),
                Box::new(input) as Box<dyn flow::SendableAny>,
            );

            let stream = node.run(&ctx, NodeInputs::new(inputs)).await;
            let output = collect_output(stream).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }

    #[test]
    fn test_deepseek_direct_stream() {
        use rig::providers::deepseek;
        use tokio::runtime::Runtime;

        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let client = deepseek::Client::from_env();
            let agent = client
                .agent(DEEPSEEK_CHAT)
                .preamble("You are a helpful assistant.")
                .build();

            let prompt = "Hello, deepseek!";
            println!("Sending prompt: {}", prompt);

            let mut stream = agent.stream_prompt(prompt).await;
            let _ = stream_to_stdout(&mut stream).await;

            println!("\nDone.");
        });
    }
}
