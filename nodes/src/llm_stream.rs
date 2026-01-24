use async_trait::async_trait;
use flow::{
    Node, NodeInputs, ReactiveStream,
    observable::{Observable, Observer, VecSubscription},
};
use futures::stream::{BoxStream, StreamExt};
use rig::{
    agent::{Agent, MultiTurnStreamItem},
    client::{CompletionClient, ProviderClient},
    message::{Reasoning, Text},
    providers::{
        deepseek::{self, CompletionModel},
        openrouter::{self, CompletionModel as OpenRouterCompletionModel},
    },
    streaming::{StreamedAssistantContent, StreamingPrompt},
};
use tracing::info;

enum LLMStreamAgent {
    DeepSeek(Agent<CompletionModel>),
    OpenRouter(Agent<OpenRouterCompletionModel>),
}

impl LLMStreamAgent {
    async fn stream_prompt(&self, prompt: &str) -> BoxStream<'static, Result<String, String>> {
        match self {
            LLMStreamAgent::DeepSeek(agent) => agent
                .stream_prompt(prompt)
                .await
                .map(|res| process_stream_result(res))
                .boxed(),
            LLMStreamAgent::OpenRouter(agent) => agent
                .stream_prompt(prompt)
                .await
                .map(|res| process_stream_result(res))
                .boxed(),
        }
    }
}

fn process_stream_result<T, E: ToString>(
    res: Result<MultiTurnStreamItem<T>, E>,
) -> Result<String, String> {
    match res {
        Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(Text {
            text,
        }))) => Ok(text),
        Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(
            Reasoning { reasoning, .. },
        ))) => Ok(reasoning.join("\n")),
        Ok(_) => Ok(String::new()),
        Err(e) => Err(e.to_string()),
    }
}

pub struct LLMStreamObservable<S> {
    pub stream: S,
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
    llm: LLMStreamAgent,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    system_prompt: String,
    user_prompt_template: String,
}

impl LLMStreamNode {
    pub fn new(sys_prompt: &str, user_tmpl: &str, model: &str) -> Self {
        dotenv::dotenv().ok();

        let use_openrouter = model.contains('/');

        let llm = if use_openrouter {
            let client = openrouter::Client::from_env();
            let mut builder = client.agent(model);
            if !sys_prompt.is_empty() {
                builder = builder.preamble(sys_prompt);
            }
            LLMStreamAgent::OpenRouter(builder.build())
        } else {
            let client = deepseek::Client::from_env();
            let mut builder = client.agent(model);
            if !sys_prompt.is_empty() {
                builder = builder.preamble(sys_prompt);
            }
            LLMStreamAgent::DeepSeek(builder.build())
        };

        Self {
            llm,
            model: model.to_owned(),
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
            .get::<String>("input")
            .or_else(|| inputs.get::<String>("external_start"))
            .or_else(|| inputs.get::<String>("output"))
            // If get failed, try to get from any input and cast to String using new helper
            .or_else(|| {
                inputs.get_any().and_then(|any| {
                    let inner: &dyn flow::SendableAny = &**any;
                    if let Some(v) = inner.as_any().downcast_ref::<String>() {
                        Some(v)
                    } else if let Some(inner_box) =
                        inner.as_any().downcast_ref::<Box<dyn flow::SendableAny>>()
                    {
                        let inner_inner: &dyn flow::SendableAny = &**inner_box;
                        inner_inner.as_any().downcast_ref::<String>()
                    } else {
                        None
                    }
                })
            })
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
        let stream = self.llm.stream_prompt(&prompt).await;
        let react_stream = LLMStreamObservable { stream };
        ReactiveStream::from_observable_with_accumulator(react_stream, |chunks: Vec<String>| {
            let full_text = chunks.join("");
            Some(Box::new(full_text))
        })
    }
}

#[cfg(test)]
mod tests {
    use flow::{Context, TaskEvent};
    use flow::{NodeInputs, TaskGuard};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    use super::*;
    use rig::providers::deepseek::DEEPSEEK_CHAT;

    async fn collect_output(stream: ReactiveStream<String>) -> String {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let ctx = Arc::new(Context::new());
        let guard = TaskGuard::default();
        let _sub = (stream.subscribe)(guard, tx, "test".to_string(), ctx);

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
            let mut node = LLMStreamNode::new("", "", DEEPSEEK_CHAT);
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
                DEEPSEEK_CHAT,
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
            let mut node = LLMStreamNode::new("", "Tell me a joke", DEEPSEEK_CHAT);
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
    fn test_open_router_stream_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMStreamNode::new("", "", "google/gemini-3-pro-preview");
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
            let _ = rig::agent::stream_to_stdout(&mut stream).await;

            println!("\nDone.");
        });
    }

    #[test]
    fn test_llm_node_completed_payload() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMStreamNode::new("", "Say hello", DEEPSEEK_CHAT);

            let inputs = HashMap::new();
            let stream = node.run(&ctx, NodeInputs::new(inputs)).await;

            let (tx, mut rx) = tokio::sync::mpsc::channel(100);
            let ctx = Arc::new(Context::new());
            let _sub = (stream.subscribe)(TaskGuard::default(), tx, "test".to_string(), ctx);

            let mut full_output = String::new();
            let mut completed_payload = None;

            while let Some(event) = rx.recv().await {
                match event {
                    TaskEvent::Next(_, val) => {
                        if let Some(s) = val.as_any().downcast_ref::<String>() {
                            full_output.push_str(s);
                        }
                    }
                    TaskEvent::Completed(_, output) => {
                        completed_payload = output;
                        break;
                    }
                    TaskEvent::Error(_, e) => panic!("Stream error: {}", e),
                    _ => {}
                }
            }

            // Verify that we got a payload in Completed event
            assert!(
                completed_payload.is_some(),
                "Completed event should have a payload"
            );

            // Verify the payload matches the accumulated stream
            let payload_str = completed_payload
                .unwrap()
                .as_any()
                .downcast_ref::<String>()
                .expect("Payload should be string")
                .clone();

            assert_eq!(
                payload_str, full_output,
                "Completed payload should match accumulated stream"
            );
            assert!(!payload_str.is_empty(), "Output should not be empty");
        });
    }
}
