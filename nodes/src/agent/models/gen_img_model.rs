use crate::agent::{
    core::Message,
    models::{ChatError, GenImgCapability, GenImgResponse},
    providers::{Provider, Request, Response},
};
use serde_json::json;

pub struct GenImgModel<T: Provider> {
    provider: T,
    model: String,
    aspect_ratio: String,
    image_size: String,
}

impl<T: Provider> GenImgModel<T> {
    pub fn new(provider: T, model: String, aspect_ratio: String, image_size: String) -> Self {
        Self {
            provider,
            model,
            aspect_ratio,
            image_size,
        }
    }

    pub fn with_aspect_ratio(mut self, aspect_ratio: String) -> Self {
        self.aspect_ratio = aspect_ratio;
        self
    }

    pub fn with_image_size(mut self, image_size: String) -> Self {
        self.image_size = image_size;
        self
    }
}

impl<T: Provider> GenImgCapability for GenImgModel<T> {
    async fn gen_img(&self, msg: &Message) -> Result<GenImgResponse, ChatError> {
        let mut extra = std::collections::HashMap::new();
        extra.insert("modalities".to_string(), json!(["image"]));
        extra.insert(
            "image_config".to_string(),
            json!({
                "aspect_ratio": self.aspect_ratio,
                "image_size": self.image_size
            }),
        );

        let messages = vec![msg.clone()];
        let mut request = Request::new(&self.model, messages);
        request.extra = extra;

        let response: Response = self
            .provider
            .send_request("/chat/completions", &request, &self.model)
            .await?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(ChatError::NoResponse)?;

        let mut image_urls = Vec::new();
        if let Some(images) = choice.message.images {
            for img in images {
                image_urls.push(img.image_url.url);
            }
        }

        if image_urls.is_empty() {
            return Err(ChatError::NoResponse);
        }

        Ok(GenImgResponse { image_urls })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::core::Message;
    use crate::agent::providers::OpenRouterProvider;

    #[tokio::test]
    async fn test_gen_img_with_openrouter() {
        dotenv::dotenv().ok();

        let provider = match OpenRouterProvider::from_env() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("OPENROUTER_API_KEY not set, skipping test");
                return;
            }
        };

        let model = GenImgModel::new(
            provider,
            "black-forest-labs/flux.2-klein-4b".to_string(),
            "1:1".to_string(),
            "1K".to_string(),
        );

        let msg = Message::user("Generate a beautiful sunset over mountains");

        let result = model.gen_img(&msg).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.image_urls.is_empty());
    }
}
