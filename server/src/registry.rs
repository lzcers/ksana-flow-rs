use chrono::{Local, NaiveDateTime};
use flow::{AnyNode, OutputPayload};
use nodes::{
    EmailNotifyNode, ImgGenNode, ShortVideoScriptNode, TextFileNode, TextMergeNode, TextNode,
    TimerNode, create_llm_any_node,
    trade::{Backtester, ReactiveSourceNode, VOLMFINode},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub type NodeCreator = Box<dyn Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InputType {
    String,
    Number,
    Boolean,
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
    pub description: String,
    pub category: String,
    pub config: Value,
    pub inputs: Vec<InputType>,
    pub outputs: Vec<InputType>,
}

pub struct NodeRegistry {
    creators: HashMap<String, NodeCreator>,
    metadata: HashMap<String, NodeMetadata>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            creators: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, meta: NodeMetadata, creator: F)
    where
        F: Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync + 'static,
    {
        self.creators.insert(meta.name.clone(), Box::new(creator));
        self.metadata.insert(meta.name.clone(), meta);
    }

    pub fn create_node(
        &self,
        name: &str,
        config: Value,
    ) -> Result<Arc<RwLock<dyn AnyNode>>, String> {
        if let Some(creator) = self.creators.get(name) {
            creator(config)
        } else {
            Err(format!("Node type '{}' not found", name))
        }
    }

    pub fn get_metadata(&self) -> Vec<NodeMetadata> {
        self.metadata.values().cloned().collect()
    }

    pub fn get_node_metadata(&self, name: &str) -> Option<&NodeMetadata> {
        self.metadata.get(name)
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_default_value(inputs: &[InputType]) -> OutputPayload {
    if inputs.is_empty() {
        return OutputPayload::cloned(());
    }
    match inputs[0] {
        InputType::String => OutputPayload::cloned("".to_string()),
        InputType::None => OutputPayload::cloned(()),
        InputType::Number => OutputPayload::cloned(0.0),
        InputType::Boolean => OutputPayload::cloned(false),
    }
}

pub fn create_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    registry.register(
        NodeMetadata {
            name: "TimerNode".to_string(),
            description: "Timer node based on Cron expression".to_string(),
            category: "Trigger".to_string(),
            config: json!({
                "cron_expr": "* * * * * * *"
            }),
            inputs: vec![InputType::None],
            outputs: vec![],
        },
        |config: Value| {
            let cron_expr = config["cron_expr"].as_str().unwrap_or("* * * * * * *");
            let node = TimerNode::new(cron_expr).map_err(|e| e.to_string())?;
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "EmailNotifyNode".to_string(),
            description: "Email notification node".to_string(),
            category: "Notification".to_string(),
            config: json!({
                "subject": "Notification",
                "body": ""
            }),
            inputs: vec![InputType::None],
            outputs: vec![],
        },
        |config: Value| {
            let subject = config["subject"]
                .as_str()
                .unwrap_or("Notification")
                .to_string();
            let body = config["body"].as_str().unwrap_or("").to_string();
            let node = EmailNotifyNode::new(subject, body);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "ReactiveSourceNode".to_string(),
            description: "Source node providing K-line data".to_string(),
            category: "Source".to_string(),
            config: json!({
                "code": "510300.SH",
                "start_time": "2023-01-01T00:00:00",
                "end_time": null
            }),
            inputs: vec![InputType::None],
            outputs: vec![],
        },
        |config: Value| {
            let code = config["code"].as_str().unwrap_or("510300.SH");
            let start_str = config["start_time"]
                .as_str()
                .unwrap_or("2023-01-01T00:00:00");
            let start_time = NaiveDateTime::parse_from_str(start_str, "%Y-%m-%dT%H:%M:%S")
                .map_err(|e| e.to_string())?
                .and_local_timezone(Local)
                .single()
                .ok_or("Invalid local time")?;

            let end_time = if let Some(s) = config["end_time"].as_str() {
                Some(
                    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                        .map_err(|e| e.to_string())?
                        .and_local_timezone(Local)
                        .single()
                        .ok_or("Invalid local time")?,
                )
            } else {
                None
            };

            let node =
                ReactiveSourceNode::new(code, start_time, end_time).map_err(|e| e.to_string())?;
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "VOLMFINode".to_string(),
            description: "Volume Money Flow Index Strategy".to_string(),
            category: "Strategy".to_string(),
            config: json!({
                "ema_period": 8,
                "mfi_period": 8
            }),
            inputs: vec![],
            outputs: vec![],
        },
        |config: Value| {
            let ema = config["ema_period"].as_u64().unwrap_or(8) as usize;
            let mfi = config["mfi_period"].as_u64().unwrap_or(8) as usize;
            let node = VOLMFINode::new(ema, mfi);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "Backtester".to_string(),
            description: "Backtesting Engine".to_string(),
            category: "Sink".to_string(),
            config: json!({
                "initial_capital": 500000.0,
                "transaction_cost": 0.0002354
            }),
            inputs: vec![],
            outputs: vec![],
        },
        |config: Value| {
            let capital = config["initial_capital"].as_f64().unwrap_or(500000.0);
            let cost = config["transaction_cost"].as_f64().unwrap_or(0.0002354);
            let node = Backtester::new(capital, cost);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "LLMNode".to_string(),
            description: "Large Language Model Node".to_string(),
            category: "AI".to_string(),
            config: json!({
                "system_prompt": "",
                "user_prompt_template": "",
                "model": "deepseek-chat",
                "stream": true
            }),
            inputs: vec![InputType::String],
            outputs: vec![],
        },
        |config: Value| {
            let system_prompt = config["system_prompt"].as_str();
            let user_prompt_template = config["user_prompt_template"].as_str();
            let model = config["model"].as_str().unwrap_or("deepseek-chat");
            let stream = config["stream"].as_bool().unwrap_or(false);
            Ok(create_llm_any_node(
                system_prompt.unwrap_or(""),
                user_prompt_template.unwrap_or(""),
                model,
                stream,
            ))
        },
    );
    registry.register(
        NodeMetadata {
            name: "StreamLLMNode".to_string(),
            description: "Large Language Model Node".to_string(),
            category: "AI".to_string(),
            config: json!({
                "system_prompt": "",
                "user_prompt_template": "",
                "model": "deepseek-chat",
                "stream": true
            }),
            inputs: vec![InputType::String],
            outputs: vec![],
        },
        |config: Value| {
            let system_prompt = config["system_prompt"].as_str();
            let user_prompt_template = config["user_prompt_template"].as_str();
            let model = config["model"].as_str().unwrap_or("deepseek-chat");
            Ok(create_llm_any_node(
                system_prompt.unwrap_or(""),
                user_prompt_template.unwrap_or(""),
                model,
                true,
            ))
        },
    );

    registry.register(
        NodeMetadata {
            name: "ImgGenNode".to_string(),
            description: "Generate image with OpenRouter".to_string(),
            category: "AI".to_string(),
            config: json!({
                "system_prompt": "",
                "user_prompt_template": "",
                "model": "black-forest-labs/flux.2-klein-4b",
                "aspect_ratio": "1:1",
                "image_size": "1K",
                "input_image_file_id": ""
            }),
            inputs: vec![InputType::String],
            outputs: vec![InputType::String],
        },
        |config: Value| {
            let system_prompt = config["system_prompt"].as_str().unwrap_or("");
            let user_prompt_template = config["user_prompt_template"].as_str().unwrap_or("");
            let model = config["model"]
                .as_str()
                .unwrap_or("black-forest-labs/flux.2-klein-4b");
            let aspect_ratio = config["aspect_ratio"].as_str().unwrap_or("1:1");
            let image_size = config["image_size"].as_str().unwrap_or("1K");
            let input_image_file_id = config["input_image_file_id"]
                .as_str()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            let node = ImgGenNode::new(
                system_prompt,
                user_prompt_template,
                model,
                aspect_ratio,
                image_size,
                input_image_file_id,
                None,
            );
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "TextNode".to_string(),
            description: "Text input node".to_string(),
            category: "Input".to_string(),
            config: json!({
                "text": ""
            }),
            inputs: vec![InputType::String],
            outputs: vec![],
        },
        |config: Value| {
            let text = config["text"].as_str().unwrap_or("").to_string();
            let id = config["id"].as_str().unwrap_or("unknown").to_string();
            let node = TextNode::new(id, text);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "TextFileNode".to_string(),
            description: "Reads content from an uploaded text file".to_string(),
            category: "Input".to_string(),
            config: json!({
                "file_id": "",
                "filename": ""
            }),
            inputs: vec![InputType::None],
            outputs: vec![InputType::String],
        },
        |config: Value| {
            let file_id = config["file_id"].as_str().unwrap_or("").to_string();
            let node = TextFileNode::new(file_id);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "TextMergeNode".to_string(),
            description: "Merges multiple text inputs".to_string(),
            category: "Logic".to_string(),
            config: json!({
                "separator": "\n"
            }),
            inputs: vec![InputType::String],
            outputs: vec![InputType::String],
        },
        |config: Value| {
            let separator = config["separator"].as_str().map(|s| s.to_string());
            let node = TextMergeNode::new(separator);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "ShortVideoScriptNode".to_string(),
            description: "Generates short video scripts in JSON format".to_string(),
            category: "AI".to_string(),
            config: json!({
                "model": "deepseek-chat"
            }),
            inputs: vec![InputType::String],
            outputs: vec![InputType::String],
        },
        |config: Value| {
            let model = config["model"].as_str().unwrap_or("deepseek-chat");
            let node = ShortVideoScriptNode::new(model);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry
}
