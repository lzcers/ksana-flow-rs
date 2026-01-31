use super::agent::LlmAgent;
use crate::prompt::build_user_prompt;
use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// 单个 Map 项的结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapItem {
    /// 用于标识该项的唯一 ID
    pub id: String,
    /// 实际的输入内容
    pub content: String,
    /// 可选的额外元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// MapNode 配置
#[derive(Debug, Clone)]
pub struct MapNodeConfig {
    /// 系统提示词
    pub system_prompt: String,
    /// 用户提示词模板，支持 {input} 占位符
    pub user_prompt_template: String,
    /// LLM 模型名称
    pub model: String,
    /// 最大并行处理数量
    pub max_parallel: usize,
}

impl Default for MapNodeConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            user_prompt_template: String::new(),
            model: String::new(),
            max_parallel: 3,
        }
    }
}

/// MapNode 用于并行处理上游节点的输入数组
/// 每个输入项会被发送到 LLM 进行处理
/// 处理结果会作为数组输出到下游节点（通常是 Reduce 节点）
pub struct MapNode {
    llm: Arc<LlmAgent>,
    config: Arc<MapNodeConfig>,
}

impl MapNode {
    /// 创建新的 MapNode
    pub fn new(config: MapNodeConfig) -> Self {
        let llm = Arc::new(LlmAgent::new(&config.system_prompt, &config.model));
        let config = Arc::new(config);
        Self { llm, config }
    }

    /// 从输入中提取 MapItem 数组
    fn extract_map_items(input: &Input) -> Vec<MapItem> {
        // 尝试从不同的 key 获取输入
        if let Some(items) = input.get_str_as::<Vec<MapItem>>("items") {
            return items;
        }
        if let Some(items) = input.get_str_as::<Vec<MapItem>>("input") {
            return items;
        }
        if let Some(items) = input.get_any_as::<Vec<MapItem>>() {
            return items;
        }
        // 尝试解析为字符串数组
        if let Some(str_items) = input.get_str_as::<Vec<String>>("items") {
            return str_items
                .into_iter()
                .enumerate()
                .map(|(idx, content)| MapItem {
                    id: format!("item_{}", idx),
                    content,
                    metadata: None,
                })
                .collect();
        }
        if let Some(str_items) = input.get_any_as::<Vec<String>>() {
            return str_items
                .into_iter()
                .enumerate()
                .map(|(idx, content)| MapItem {
                    id: format!("item_{}", idx),
                    content,
                    metadata: None,
                })
                .collect();
        }
        Vec::new()
    }

    /// 处理单个 MapItem
    async fn process_item(
        llm: Arc<LlmAgent>,
        config: Arc<MapNodeConfig>,
        item: MapItem,
    ) -> MapResult {
        let prompt = build_user_prompt(&config.user_prompt_template, &item.content);

        match llm.prompt(&prompt).await {
            Ok(output) => MapResult {
                id: item.id,
                input: item.content,
                output,
                success: true,
                error: None,
            },
            Err(e) => MapResult {
                id: item.id,
                input: item.content,
                output: String::new(),
                success: false,
                error: Some(format!("LLM processing failed: {}", e)),
            },
        }
    }
}

/// Map 处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapResult {
    /// 对应输入项的 ID
    pub id: String,
    /// 原始输入内容
    pub input: String,
    /// LLM 处理后的输出
    pub output: String,
    /// 处理是否成功
    pub success: bool,
    /// 如果失败，错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[async_trait]
impl Node for MapNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let items = Self::extract_map_items(input);

        if items.is_empty() {
            return Ok(Value::Array(vec![]).into());
        }

        // 使用信号量限制并发数
        let semaphore = Arc::new(Semaphore::new(self.config.max_parallel));
        let mut handles = Vec::with_capacity(items.len());

        for item in items {
            let llm = Arc::clone(&self.llm);
            let config = Arc::clone(&self.config);
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = semaphore
                    .acquire()
                    .await
                    .map_err(|e| format!("Failed to acquire semaphore permit: {}", e))?;

                let result = Self::process_item(llm, config, item).await;
                Ok::<MapResult, String>(result)
            });

            handles.push(handle);
        }

        // 并行执行所有任务
        let results: Vec<MapResult> = join_all(handles)
            .await
            .into_iter()
            .filter_map(|h| h.ok())
            .filter_map(|r| r.ok())
            .collect();

        // 将结果序列化为 JSON 数组
        let results_json: Vec<Value> = results
            .into_iter()
            .map(|r| serde_json::to_value(r).unwrap_or(Value::Null))
            .collect();

        Ok(Value::Array(results_json).into())
    }
}

/// 创建一个可用于 Graph 的 AnyNode 类型的 MapNode
pub fn create_map_any_node(
    config: MapNodeConfig,
) -> std::sync::Arc<tokio::sync::RwLock<dyn flow::AnyNode>> {
    std::sync::Arc::new(tokio::sync::RwLock::new(MapNode::new(config)))
        as std::sync::Arc<tokio::sync::RwLock<dyn flow::AnyNode>>
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_extract_map_items() {
        let items = vec![
            MapItem {
                id: "1".to_string(),
                content: "Test content 1".to_string(),
                metadata: None,
            },
            MapItem {
                id: "2".to_string(),
                content: "Test content 2".to_string(),
                metadata: None,
            },
        ];

        let mut values = HashMap::new();
        values.insert("items".to_string(), serde_json::to_value(&items).unwrap());

        let input = Input::new(values);
        let extracted = MapNode::extract_map_items(&input);

        assert_eq!(extracted.len(), 2);
        assert_eq!(extracted[0].id, "1");
        assert_eq!(extracted[0].content, "Test content 1");
    }

    #[test]
    fn test_extract_string_array() {
        let str_items = vec!["item1".to_string(), "item2".to_string()];

        let mut values = HashMap::new();
        values.insert(
            "input".to_string(),
            serde_json::to_value(&str_items).unwrap(),
        );

        let input = Input::new(values);
        let extracted = MapNode::extract_map_items(&input);

        assert_eq!(extracted.len(), 2);
        assert_eq!(extracted[0].id, "item_0");
        assert_eq!(extracted[0].content, "item1");
    }

    #[test]
    fn test_empty_input() {
        let values = HashMap::new();
        let input = Input::new(values);
        let extracted = MapNode::extract_map_items(&input);

        assert!(extracted.is_empty());
    }
}
