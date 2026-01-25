use async_trait::async_trait;
use flow::{Context, Node, NodeInputs};
use serde_json::json;
use uuid::Uuid;

mod utils;

pub struct ImgGenNode {
    api_key: Option<String>,
    base_url: String,
    http: reqwest::Client,
    model: String,
    system_prompt: String,
    user_prompt_template: String,
    aspect_ratio: String,
    image_size: String,
    input_image_file_id: Option<String>,
    space_id: Option<String>,
}

impl ImgGenNode {
    pub fn new(
        system_prompt: &str,
        user_prompt_template: &str,
        model: &str,
        aspect_ratio: &str,
        image_size: &str,
        input_image_file_id: Option<String>,
        space_id: Option<String>,
    ) -> Self {
        dotenv::dotenv().ok();

        let api_key = std::env::var("OPENROUTER_API_KEY").ok();
        let base_url =
            std::env::var("OPENROUTER_BASE_URL").unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

        Self {
            api_key,
            base_url,
            http: reqwest::Client::new(),
            model: model.to_owned(),
            system_prompt: system_prompt.to_owned(),
            user_prompt_template: user_prompt_template.to_owned(),
            aspect_ratio: aspect_ratio.to_owned(),
            image_size: image_size.to_owned(),
            input_image_file_id,
            space_id,
        }
    }
}

#[async_trait]
impl Node for ImgGenNode {
    type Out = String;

    async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> Self::Out {
        let Some(api_key) = &self.api_key else {
            return "Missing OPENROUTER_API_KEY".to_string();
        };

        let space_id = inputs
            .get::<String>("space_id")
            .cloned()
            .or_else(|| self.space_id.clone())
            .unwrap_or_else(|| "ksana".to_string());

        let input = utils::extract_string_input(&inputs);
        let prompt = if !input.is_empty() {
            if self.user_prompt_template.contains("{input}") {
                self.user_prompt_template.replace("{input}", &input)
            } else {
                input
            }
        } else {
            self.user_prompt_template.clone()
        };

        let image_data_url = match self.input_image_file_id.as_deref() {
            Some(file_id) if !file_id.is_empty() => match utils::load_uploaded_image_data_url(file_id, &space_id) {
                Ok(v) => v,
                Err(e) => return e,
            },
            _ => None,
        };

        let payload = utils::build_payload(
            &self.model,
            &self.system_prompt,
            &prompt,
            image_data_url,
            &self.aspect_ratio,
            &self.image_size,
        );

        let resp_json = match utils::send_openrouter_chat_completions(&self.http, &self.base_url, api_key, &payload).await
        {
            Ok(v) => v,
            Err(e) => return e,
        };

        let Some(data_url) = utils::extract_first_image_data_url(&resp_json) else {
            return "No image returned from model".to_string();
        };

        let (mime_type, bytes) = match utils::parse_data_url_to_bytes(&data_url) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let media_id = Uuid::new_v4().to_string();
        if let Err(e) =
            utils::save_ai_media_to_db(&media_id, &mime_type, &bytes, &prompt, &self.model, &space_id)
        {
            return e;
        }

        json!({
            "media_id": media_id,
            "data_url": data_url,
            "mime_type": mime_type,
        })
        .to_string()
    }
}

