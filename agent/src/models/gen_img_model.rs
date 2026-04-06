use std::{collections::HashMap, sync::Arc};

use crate::{
    core::Message,
    models::{ChatError, GenImgCapability, GenImgResponse},
    providers::{Provider, Request, Response},
};
use async_trait::async_trait;
use serde_json::json;

pub struct GenImgModel {
    model_providers: HashMap<String, Arc<dyn Provider>>,
    active_model: Option<String>,
    aspect_ratio: String,
    image_size: String,
}

impl Default for GenImgModel {
    fn default() -> Self {
        Self::new()
    }
}

impl GenImgModel {
    pub fn new() -> Self {
        Self {
            model_providers: HashMap::new(),
            active_model: None,
            aspect_ratio: "1:1".to_string(),
            image_size: "1K".to_string(),
        }
    }

    pub fn add_model_provider(&mut self, model_name: &str, provider: Arc<dyn Provider>) {
        self.model_providers
            .entry(model_name.to_owned())
            .or_insert(provider);
    }

    pub fn add_models_for_provider(&mut self, model_names: &[&str], provider: Arc<dyn Provider>) {
        for model_name in model_names {
            self.add_model_provider(model_name, provider.clone());
        }
    }

    pub fn set_active_model(&mut self, model_name: &str) -> Result<(), ChatError> {
        if !self.model_providers.contains_key(model_name) {
            return Err(ChatError::ModelNotFound(model_name.to_owned()));
        }
        self.active_model = Some(model_name.to_owned());
        Ok(())
    }

    pub fn with_aspect_ratio(mut self, aspect_ratio: String) -> Self {
        self.aspect_ratio = aspect_ratio;
        self
    }

    pub fn with_image_size(mut self, image_size: String) -> Self {
        self.image_size = image_size;
        self
    }

    fn get_provider(&self, model_name: &str) -> Result<&Arc<dyn Provider>, ChatError> {
        self.model_providers
            .get(model_name)
            .ok_or_else(|| ChatError::ModelNotFound(model_name.to_owned()))
    }

    pub fn active_model(&self) -> Option<&str> {
        self.active_model.as_deref()
    }
}

#[async_trait]
impl GenImgCapability for GenImgModel {
    async fn gen_img(&self, msgs: Vec<Message>) -> Result<GenImgResponse, ChatError> {
        let model_name = self
            .active_model
            .as_ref()
            .ok_or_else(|| ChatError::ModelNotFound("No active model set".to_string()))?;

        let provider = self.get_provider(model_name)?;

        let mut extra = std::collections::HashMap::new();
        extra.insert("modalities".to_string(), json!(["image"]));
        extra.insert(
            "image_config".to_string(),
            json!({
                "aspect_ratio": self.aspect_ratio,
                "image_size": self.image_size
            }),
        );

        let mut request = Request::new(model_name, msgs);
        request.extra = extra;

        let response: Response = provider.chat(request).await?;

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
    use crate::core::Message;
    use crate::providers::{openrouter_provider, openrouter_provider_from_env};

    #[tokio::test]
    async fn test_gen_img_with_openrouter() {
        dotenv::dotenv().ok();

        let provider = match openrouter_provider_from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                eprintln!("OPENROUTER_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = GenImgModel::new()
            .with_aspect_ratio("1:1".to_string())
            .with_image_size("1K".to_string());

        model.add_model_provider("black-forest-labs/flux.2-klein-4b", provider);

        if let Err(e) = model.set_active_model("black-forest-labs/flux.2-klein-4b") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let msg = Message::user("Generate a beautiful sunset over mountains");

        let result = model.gen_img(vec![msg]).await;
        if let Err(e) = result {
            eprintln!("Failed to generate image: {}", e);
            return;
        }

        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.image_urls.is_empty());
    }

    #[test]
    fn test_model_provider_mapping() {
        let or_provider = Arc::new(openrouter_provider("dummy_key"));

        let mut model = GenImgModel::new();

        model.add_models_for_provider(
            &[
                "black-forest-labs/flux.2-klein-4b",
                "black-forest-labs/flux.1-pro",
            ],
            or_provider,
        );

        assert!(
            model
                .model_providers
                .contains_key("black-forest-labs/flux.2-klein-4b")
        );
        assert!(
            model
                .model_providers
                .contains_key("black-forest-labs/flux.1-pro")
        );
        assert_eq!(model.model_providers.len(), 2);
    }

    #[test]
    fn test_set_active_model() {
        let provider = Arc::new(openrouter_provider("dummy_key"));
        let mut model = GenImgModel::new();
        model.add_model_provider("black-forest-labs/flux.2-klein-4b", provider);

        let result = model.set_active_model("black-forest-labs/flux.2-klein-4b");
        assert!(result.is_ok());
        assert_eq!(
            model.active_model,
            Some("black-forest-labs/flux.2-klein-4b".to_string())
        );

        let result = model.set_active_model("non-existent-model");
        assert!(result.is_err());
        assert!(matches!(result, Err(ChatError::ModelNotFound(_))));
    }

    #[test]
    fn test_builder_methods() {
        let model = GenImgModel::new()
            .with_aspect_ratio("16:9".to_string())
            .with_image_size("2K".to_string());

        assert_eq!(model.aspect_ratio, "16:9");
        assert_eq!(model.image_size, "2K");
    }
}
