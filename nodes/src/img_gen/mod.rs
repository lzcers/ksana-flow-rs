use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;
use serde_json::json;
use tracing::{info, warn};
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
        let base_url = std::env::var("OPENROUTER_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

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
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let Some(api_key) = &self.api_key else {
            return Ok(Value::String("Missing OPENROUTER_API_KEY".to_string()).into());
        };
        info!("ImgGenNode: model={}", self.model);
        let space_id = input
            .get_str_as::<String>("space_id")
            .or_else(|| self.space_id.clone())
            .unwrap_or_else(|| "ksana".to_string());

        let input = utils::extract_string_input(input);
        let prompt = if !input.is_empty() {
            if self.user_prompt_template.contains("{input}") {
                self.user_prompt_template.replace("{input}", &input)
            } else {
                input
            }
        } else {
            self.user_prompt_template.clone()
        };

        let system_prompt = if self.system_prompt.is_empty() {
            "You are an image generation model. Always generate at least one image. Do not reply with scripts, tables, or long text. If the prompt is not a valid image request, rewrite it into a concise image prompt and generate the image."
        } else {
            self.system_prompt.as_str()
        };

        let prompt = format!(
            "Generate a single image.\nStyle: cinematic, high quality.\nReturn an image.\n\nPrompt:\n{}",
            prompt
        );

        let image_data_url = match self.input_image_file_id.as_deref() {
            Some(file_id) if !file_id.is_empty() => {
                match utils::load_uploaded_image_data_url(file_id, &space_id) {
                    Ok(v) => v,
                    Err(e) => return Ok(Value::String(e).into()),
                }
            }
            _ => None,
        };

        let payload = utils::build_payload(
            &self.model,
            system_prompt,
            &prompt,
            image_data_url,
            &self.aspect_ratio,
            &self.image_size,
        );

        let resp_json = match utils::send_openrouter_chat_completions(
            &self.http,
            &self.base_url,
            api_key,
            &payload,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => return Ok(Value::String(e).into()),
        };

        let Some(data_url) = utils::extract_first_image_data_url(&resp_json) else {
            if let Some(err) = utils::extract_model_error(&resp_json) {
                let err_msg = format!("No image returned from model. Error: {}", err);
                warn!("{}", err_msg);
                panic!("{}", err_msg);
            }
            let err_msg = format!(
                "No image returned from model. Response excerpt: {}",
                utils::response_debug_excerpt(&resp_json)
            );
            warn!("{}", err_msg);
            panic!("{}", err_msg);
        };

        let (mime_type, bytes) = match utils::parse_data_url_to_bytes(&data_url) {
            Ok(v) => v,
            Err(e) => return Ok(Value::String(e).into()),
        };

        let media_id = Uuid::new_v4().to_string();
        if let Err(e) = utils::save_ai_media_to_db(
            &media_id,
            &mime_type,
            &bytes,
            &prompt,
            &self.model,
            &space_id,
        ) {
            return Ok(Value::String(e).into());
        }

        let out = json!({
            "media_id": media_id,
            "mime_type": mime_type,
            "space_id": space_id,
        })
        .to_string();
        Ok(Value::String(out).into())
    }
}
