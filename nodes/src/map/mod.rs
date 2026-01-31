use async_trait::async_trait;
use flow::{AnyNode, Context, Input, Node, Output};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

/// MapNode 配置
#[derive(Debug, Clone)]
pub struct MapNodeConfig {
    /// 最大并发处理数量
    pub max_parallel: usize,
    /// 子节点类型名称 - 用于在 registry 中查找创建器
    pub sub_node_type: String,
    /// 子节点配置
    pub sub_node_config: Value,
}

impl Default for MapNodeConfig {
    fn default() -> Self {
        Self {
            max_parallel: 3,
            sub_node_type: String::new(),
            sub_node_config: Value::Null,
        }
    }
}

/// 输入项结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapInputItem {
    /// 用于标识该项的唯一 ID
    pub id: String,
    /// 实际的输入数据
    pub data: Value,
}

/// Map 处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapResult {
    /// 对应输入项的 ID
    pub id: String,
    /// 原始输入数据
    pub input: Value,
    /// 处理后的输出
    pub output: Value,
    /// 处理是否成功
    pub success: bool,
    /// 如果失败，错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// MapNode 用于并行处理输入数组
/// 每个输入项会被发送到配置的子节点进行处理
/// 处理结果会作为数组输出到下游节点
pub struct MapNode {
    config: Arc<MapNodeConfig>,
    /// 子节点创建器
    sub_node_creator: Arc<dyn Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync>,
}

impl MapNode {
    /// 创建新的 MapNode
    pub fn new(
        config: MapNodeConfig,
        sub_node_creator: impl Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            config: Arc::new(config),
            sub_node_creator: Arc::new(sub_node_creator),
        }
    }

    /// 从输入中提取 MapInputItem 数组
    fn extract_input_items(input: &Input) -> Vec<MapInputItem> {
        // 尝试从不同的 key 获取输入
        if let Some(items) = input.get_str_as::<Vec<MapInputItem>>("items") {
            return items;
        }
        if let Some(items) = input.get_str_as::<Vec<MapInputItem>>("input") {
            return items;
        }
        if let Some(items) = input.get_any_as::<Vec<MapInputItem>>() {
            return items;
        }
        // 尝试解析为普通 JSON 数组
        if let Some(arr) = input.get_any_as::<Vec<Value>>() {
            return arr
                .into_iter()
                .enumerate()
                .map(|(idx, data)| MapInputItem {
                    id: format!("item_{}", idx),
                    data,
                })
                .collect();
        }
        Vec::new()
    }

    /// 处理单个输入项
    async fn process_item(
        sub_node_creator: &(dyn Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync),
        node_config: Value,
        item: MapInputItem,
    ) -> MapResult {
        // 创建子节点
        let sub_node = match (sub_node_creator)(node_config) {
            Ok(node) => node,
            Err(e) => {
                return MapResult {
                    id: item.id,
                    input: item.data,
                    output: Value::Null,
                    success: false,
                    error: Some(format!("Failed to create sub-node: {}", e)),
                }
            }
        };

        // 构建子节点的输入
        let mut input_map = std::collections::HashMap::new();
        input_map.insert("input".to_string(), item.data.clone());
        let input = Input::new(input_map);

        // 创建空的上下文
        let ctx = Context::new();

        // 执行子节点
        let mut guard = sub_node.write().await;
        match guard.run(&ctx, &input).await {
            Ok(output) => {
                let value = output.get().cloned().unwrap_or(Value::Null);
                MapResult {
                    id: item.id,
                    input: item.data,
                    output: value,
                    success: true,
                    error: None,
                }
            },
            Err(e) => MapResult {
                id: item.id,
                input: item.data,
                output: Value::Null,
                success: false,
                error: Some(format!("Sub-node processing failed: {}", e)),
            },
        }
    }
}

#[async_trait]
impl Node for MapNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let items = Self::extract_input_items(input);

        if items.is_empty() {
            return Ok(Value::Array(vec![]).into());
        }

        // 使用信号量限制并发数
        let semaphore = Arc::new(Semaphore::new(self.config.max_parallel));
        let mut handles = Vec::with_capacity(items.len());

        for item in items {
            let semaphore = Arc::clone(&semaphore);
            let sub_node_creator = Arc::clone(&self.sub_node_creator);
            let node_config = self.config.sub_node_config.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore
                    .acquire()
                    .await
                    .map_err(|e| format!("Failed to acquire semaphore permit: {}", e))?;

                // 将 sub_node_creator 转换为引用并传递
                let result = Self::process_item(
                    sub_node_creator.as_ref(),
                    node_config,
                    item,
                ).await;
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
    sub_node_creator: impl Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String>
        + Send
        + Sync
        + 'static,
) -> Arc<RwLock<dyn AnyNode>> {
    Arc::new(RwLock::new(MapNode::new(config, sub_node_creator)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_input_items() {
        let items = vec![
            MapInputItem {
                id: "1".to_string(),
                data: serde_json::json!("Test content 1"),
            },
            MapInputItem {
                id: "2".to_string(),
                data: serde_json::json!("Test content 2"),
            },
        ];

        let mut values = std::collections::HashMap::new();
        values.insert("items".to_string(), serde_json::to_value(&items).unwrap());

        let input = Input::new(values);
        let extracted = MapNode::extract_input_items(&input);

        assert_eq!(extracted.len(), 2);
        assert_eq!(extracted[0].id, "1");
    }

    #[test]
    fn test_extract_json_array() {
        let arr = vec![
            serde_json::json!("item1"),
            serde_json::json!("item2"),
        ];

        let mut values = std::collections::HashMap::new();
        values.insert("input".to_string(), serde_json::to_value(&arr).unwrap());

        let input = Input::new(values);
        let extracted = MapNode::extract_input_items(&input);

        assert_eq!(extracted.len(), 2);
        assert_eq!(extracted[0].id, "item_0");
    }

    #[test]
    fn test_empty_input() {
        let values = std::collections::HashMap::new();
        let input = Input::new(values);
        let extracted = MapNode::extract_input_items(&input);

        assert!(extracted.is_empty());
    }
}
