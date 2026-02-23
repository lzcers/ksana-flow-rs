use chrono::{Local, NaiveDateTime};
use flow::AnyNode;
use nodes::reduce_node::ReduceNode;
use nodes::{
    EmailNotifyNode, ImgGenNode, LLMNode, TextFileNode, TextMergeNode, TextNode, TextSplitConfig,
    TextSplitNode, TimerNode,
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
    Json,
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
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

pub fn create_default_value(inputs: &[InputType]) -> Value {
    if inputs.is_empty() {
        return Value::Null;
    }
    match inputs[0] {
        InputType::String => Value::String("".to_string()),
        InputType::None => Value::Null,
        InputType::Number => json!(0.0),
        InputType::Boolean => json!(false),
        InputType::Json => Value::Null,
    }
}

pub fn create_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    registry.register(
        NodeMetadata {
            name: "TimerNode".to_string(),
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
            Ok(Arc::new(RwLock::new(LLMNode::new(
                system_prompt.unwrap_or(""),
                user_prompt_template.unwrap_or(""),
                model,
                stream,
            ))))
        },
    );

    registry.register(
        NodeMetadata {
            name: "ImgGenNode".to_string(),
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
            name: "TextSplitNode".to_string(),
            config: serde_json::to_value(TextSplitConfig::default()).unwrap_or(Value::Null),
            inputs: vec![InputType::String],
            outputs: vec![InputType::Json],
        },
        |config: Value| {
            let mut split_config = TextSplitConfig::default();

            // Parse mode configuration
            if let Some(mode_obj) = config.get("mode") {
                if let Some(by_line_count) = mode_obj.get("by_line_count") {
                    if let Some(max_lines) = by_line_count
                        .get("max_lines_per_part")
                        .and_then(|v| v.as_u64())
                    {
                        split_config.mode = nodes::text::TextSplitMode::ByLineCount {
                            max_lines_per_part: max_lines as usize,
                        };
                    }
                } else if let Some(by_rule) = mode_obj.get("by_rule") {
                    if let Some(rule_obj) = by_rule.get("rule") {
                        if let Some(heading) = rule_obj.get("heading_keywords") {
                            let keywords: Vec<String> = heading
                                .get("keywords")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let require_prefix = heading
                                .get("require_prefix")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            split_config.mode = nodes::text::TextSplitMode::ByRule {
                                rule: nodes::text::TextSplitRule::HeadingKeywords {
                                    keywords,
                                    require_prefix,
                                },
                            };
                        }
                    }
                }
            }

            // Parse other config options
            if let Some(v) = config.get("remove_empty_lines").and_then(|v| v.as_bool()) {
                split_config.remove_empty_lines = v;
            }
            if let Some(v) = config
                .get("rule_only_keep_matched_ranges")
                .and_then(|v| v.as_bool())
            {
                split_config.rule_only_keep_matched_ranges = v;
            }
            if let Some(line_nums) = config.get("line_numbers") {
                if let Some(v) = line_nums.get("enabled").and_then(|v| v.as_bool()) {
                    split_config.line_numbers.enabled = v;
                }
                if let Some(v) = line_nums.get("template").and_then(|v| v.as_str()) {
                    split_config.line_numbers.template = v.to_string();
                }
                if let Some(v) = line_nums.get("pad_width").and_then(|v| v.as_u64()) {
                    split_config.line_numbers.pad_width = Some(v as usize);
                }
                if let Some(v) = line_nums.get("pad_char").and_then(|v| v.as_str()) {
                    if let Some(c) = v.chars().next() {
                        split_config.line_numbers.pad_char = c;
                    }
                }
            }

            let node = TextSplitNode::new(split_config);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry.register(
        NodeMetadata {
            name: "MapNode".to_string(),
            config: json!({
                "max_concurrency": 2,
                "streaming": false,
                "inherit_context": false,
                "timeout_ms": null
            }),
            inputs: vec![InputType::None],
            outputs: vec![InputType::None],
        },
        |_config: Value| {
            Err("MapNode is a group node and must be compiled from its child nodes".to_string())
        },
    );

    registry.register(
        NodeMetadata {
            name: "ReduceNode".to_string(),
            config: json!({
                "reducer": "sum",
                "separator": "\n"
            }),
            inputs: vec![InputType::Json],
            outputs: vec![InputType::Json],
        },
        |config: Value| {
            let reducer = config
                .get("reducer")
                .and_then(|v| v.as_str())
                .unwrap_or("sum");
            let node = match reducer {
                "sum" => ReduceNode::sum(),
                "count" => ReduceNode::count(),
                "max" => ReduceNode::max(),
                "min" => ReduceNode::min(),
                "concat" => {
                    let separator = config
                        .get("separator")
                        .and_then(|v| v.as_str())
                        .unwrap_or("\n");
                    ReduceNode::concat(separator)
                }
                "merge_array" => ReduceNode::merge(false),
                "merge_object_deep" => ReduceNode::merge(true),
                other => return Err(format!("Unknown reducer: {}", other)),
            };
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        },
    );

    registry
}
