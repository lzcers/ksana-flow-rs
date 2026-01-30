use futures::stream::{BoxStream, StreamExt};
use rig::{
    agent::{Agent, MultiTurnStreamItem},
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    message::{Reasoning, Text},
    providers::{
        deepseek::{self, CompletionModel},
        openrouter::{self, CompletionModel as OpenRouterCompletionModel},
    },
    streaming::{StreamedAssistantContent, StreamingPrompt},
};

pub(crate) enum LlmAgent {
    DeepSeek(Agent<CompletionModel>),
    OpenRouter(Agent<OpenRouterCompletionModel>),
}

impl LlmAgent {
    pub(crate) fn new(sys_prompt: &str, model: &str) -> Self {
        dotenv::dotenv().ok();

        let use_openrouter = model.contains('/');

        if use_openrouter {
            let client = openrouter::Client::from_env();
            let mut builder = client.agent(model);
            if !sys_prompt.is_empty() {
                builder = builder.preamble(sys_prompt);
            }
            Self::OpenRouter(builder.build())
        } else {
            let client = deepseek::Client::from_env();
            let mut builder = client.agent(model);
            if !sys_prompt.is_empty() {
                builder = builder.preamble(sys_prompt);
            }
            Self::DeepSeek(builder.build())
        }
    }

    pub(crate) async fn prompt(&self, prompt: &str) -> Result<String, String> {
        match self {
            Self::DeepSeek(agent) => agent.prompt(prompt).await.map_err(|e| e.to_string()),
            Self::OpenRouter(agent) => agent.prompt(prompt).await.map_err(|e| e.to_string()),
        }
    }

    pub(crate) async fn stream_prompt(
        &self,
        prompt: &str,
    ) -> BoxStream<'static, Result<String, String>> {
        match self {
            Self::DeepSeek(agent) => agent
                .stream_prompt(prompt)
                .await
                .map(process_stream_result)
                .boxed(),
            Self::OpenRouter(agent) => agent
                .stream_prompt(prompt)
                .await
                .map(process_stream_result)
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
