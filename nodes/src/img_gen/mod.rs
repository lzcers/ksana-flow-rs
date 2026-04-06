use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use agent::{
    core::Message,
    models::{ChatError, GenImgCapability, GenImgModel, GenImgResponse},
    providers::openrouter_provider_from_env,
};

mod utils;

pub struct ImgGenNode {
    gen_img_model: GenImgModel,
    system_prompt: String,
    user_prompt_template: String,
    input_image_file_id: Option<String>,
    space_id: Option<String>,
    active_model_name: String,
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

        let mut gen_img_model = GenImgModel::new()
            .with_aspect_ratio(aspect_ratio.to_string())
            .with_image_size(image_size.to_string());

        if let Ok(provider) = openrouter_provider_from_env() {
            gen_img_model.add_model_provider(model, Arc::new(provider));
            let _ = gen_img_model.set_active_model(model);
        }

        Self {
            gen_img_model,
            system_prompt: system_prompt.to_owned(),
            user_prompt_template: user_prompt_template.to_owned(),
            input_image_file_id,
            space_id,
            active_model_name: model.to_string(),
        }
    }
}

#[async_trait]
impl Node for ImgGenNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        info!("ImgGenNode: using {:?}", self.active_model_name);
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
                    Err(e) => return Err(e),
                }
            }
            _ => None,
        };

        let mut msgs = Vec::new();
        if !system_prompt.is_empty() {
            msgs.push(Message::system(system_prompt));
        }

        if let Some(data_url) = image_data_url {
            msgs.push(Message::user(format!(
                "{}\n\n[Image: {}]",
                prompt, data_url
            )));
        } else {
            msgs.push(Message::user(prompt.clone()));
        }

        let img_resp: GenImgResponse =
            self.gen_img_model
                .gen_img(msgs)
                .await
                .map_err(|e| match e {
                    ChatError::Provider(e) => format!("Provider error: {}", e),
                    ChatError::NoResponse => "No response from model".to_string(),
                    ChatError::StreamError(e) => format!("Stream error: {}", e),
                    ChatError::ModelNotFound(e) => format!("Model not found: {}", e),
                })?;

        let image_url = match img_resp.image_urls.first() {
            Some(url) => url,
            None => {
                let err_msg = "No image returned from GenImgModel";
                warn!("{}", err_msg);
                return Err(err_msg.to_string());
            }
        };

        let (mime_type, bytes) = match utils::parse_data_url_to_bytes(&image_url) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let media_id = Uuid::new_v4().to_string();
        if let Err(e) = utils::save_ai_media_to_db(
            &media_id,
            &mime_type,
            &bytes,
            &prompt,
            self.gen_img_model.active_model().unwrap_or_default(),
            &space_id,
        ) {
            return Err(e);
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
