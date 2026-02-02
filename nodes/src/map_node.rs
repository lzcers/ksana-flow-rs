use std::sync::Arc;
use tokio::sync::Semaphore;

use async_trait::async_trait;
use flow::observable::Subscription;
use flow::{
    Context, ControllerHandle, Graph, Input, Node, Output, RunnerId, SubgraphConfig,
    SubgraphExecutor,
};
use flow::{ReactiveStream, TaskEvent};
use serde_json::Value;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task::{JoinHandle, JoinSet};

/// MapNode - 并发映射节点
///
/// 对输入数组中的每个元素并行执行子图，收集所有结果
///
/// # 示例
/// ```
/// use flow::Graph;
/// use nodes::map_node::MapNode;
/// let map_node = MapNode::new(
///     Graph::new(),  // 要执行的子图
///     10,        // 最大并发数
/// );
/// ```
pub struct MapNode {
    /// 子图执行器
    executor: SubgraphExecutor,
    /// 最大并发数
    max_concurrency: usize,
    /// 是否以流式方式输出每个 item 的结果，并在末尾发出 done 信号
    streaming: bool,
}

impl MapNode {
    /// 创建新的 MapNode
    ///
    /// # 参数
    /// - `subgraph`: 子图定义，用于处理每个输入元素
    /// - `max_concurrency`: 最大并发数，0 表示无限制
    pub fn new(subgraph: Graph, max_concurrency: usize) -> Self {
        let executor = SubgraphExecutor::with_defaults(subgraph);

        Self {
            executor,
            max_concurrency,
            streaming: false,
        }
    }

    pub fn new_streaming(subgraph: Graph, max_concurrency: usize) -> Self {
        let executor = SubgraphExecutor::with_defaults(subgraph);
        Self {
            executor,
            max_concurrency,
            streaming: true,
        }
    }

    /// 创建 MapNode，指定入口和出口节点
    pub fn with_entry_exit(
        subgraph: Graph,
        max_concurrency: usize,
        entry_node: impl Into<String>,
        exit_node: impl Into<String>,
    ) -> Self {
        let mut config = SubgraphConfig::default();
        config.entry_node = entry_node.into();
        config.exit_node = exit_node.into();

        let executor = SubgraphExecutor::new(subgraph, config);

        Self {
            executor,
            max_concurrency,
            streaming: false,
        }
    }

    pub fn with_entry_exit_streaming(
        subgraph: Graph,
        max_concurrency: usize,
        entry_node: impl Into<String>,
        exit_node: impl Into<String>,
    ) -> Self {
        let mut config = SubgraphConfig::default();
        config.entry_node = entry_node.into();
        config.exit_node = exit_node.into();
        let executor = SubgraphExecutor::new(subgraph, config);
        Self {
            executor,
            max_concurrency,
            streaming: true,
        }
    }

    pub fn with_executor(
        executor: SubgraphExecutor,
        max_concurrency: usize,
        streaming: bool,
    ) -> Self {
        Self {
            executor,
            max_concurrency,
            streaming,
        }
    }

    /// 获取最大并发数
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    /// 设置最大并发数
    pub fn set_max_concurrency(&mut self, max_concurrency: usize) {
        self.max_concurrency = max_concurrency;
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    /// 执行 Map 操作
    ///
    /// 对输入数组中的每个元素并行执行子图
    async fn execute_map(&self, ctx: &Context, items: Vec<Value>) -> Result<Vec<Value>, String> {
        let controller = flow::try_controller()
            .ok_or_else(|| "Missing Controller in current task scope".to_string())?;
        let parent_runner_id = flow::try_runner_id();

        // 如果没有输入，直接返回空数组
        if items.is_empty() {
            return Ok(Vec::new());
        }

        // 确定并发数
        let concurrency = if self.max_concurrency == 0 {
            items.len()
        } else {
            self.max_concurrency.min(items.len())
        };

        // 创建信号量控制并发
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::with_capacity(items.len());

        // 为每个输入项创建任务
        for (idx, item) in items.into_iter().enumerate() {
            // 获取信号量许可
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| format!("Failed to acquire permit: {}", e))?;

            let executor = self.executor.clone();
            let parent_ctx = ctx.clone();
            let controller = controller.clone();

            // 创建异步任务
            let handle = tokio::spawn(async move {
                let result = executor
                    .execute_with_controller_and_parent(
                        item,
                        &parent_ctx,
                        controller,
                        parent_runner_id,
                    )
                    .await;
                drop(permit); // 释放许可
                (idx, result)
            });

            handles.push(handle);
        }

        // 收集所有结果
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok((idx, Ok(output))) => {
                    results.push((idx, output));
                }
                Ok((_, Err(e))) => {
                    return Err(format!("Subgraph execution failed: {}", e));
                }
                Err(e) => {
                    return Err(format!("Task panicked: {}", e));
                }
            }
        }

        // 按原始顺序排序结果
        results.sort_by_key(|(idx, _)| *idx);
        let outputs: Vec<Value> = results.into_iter().map(|(_, v)| v).collect();

        Ok(outputs)
    }

    fn build_stream(
        &self,
        items: Vec<Value>,
        controller: ControllerHandle,
        parent_runner_id: Option<RunnerId>,
    ) -> ReactiveStream {
        let executor = self.executor.clone();
        let max_concurrency = self.max_concurrency;
        ReactiveStream {
            subscribe: Box::new(move |guard, tx: mpsc::Sender<TaskEvent>, node_id, ctx| {
                struct TaskSubscription {
                    handle: JoinHandle<()>,
                }

                impl Subscription for TaskSubscription {
                    fn unsubscribe(self: Box<Self>) {
                        self.handle.abort();
                    }
                }

                let total = items.len();
                let controller = controller.clone();
                let handle = tokio::spawn(async move {
                    let _guard = guard;

                    if items.is_empty() {
                        let _ = tx
                            .send(TaskEvent::Next(
                                node_id.clone(),
                                json!({"kind":"done","count":0}),
                            ))
                            .await;
                        let _ = tx
                            .send(TaskEvent::Completed(node_id, Some(Value::Array(vec![]))))
                            .await;
                        return;
                    }

                    let concurrency = if max_concurrency == 0 {
                        total
                    } else {
                        max_concurrency.min(total)
                    };
                    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));

                    let mut join_set = JoinSet::new();
                    for (idx, item) in items.into_iter().enumerate() {
                        let semaphore = semaphore.clone();
                        let executor = executor.clone();
                        let ctx = ctx.clone();
                        let tx = tx.clone();
                        let node_id = node_id.clone();
                            let controller = controller.clone();
                        join_set.spawn(async move {
                            let permit = semaphore
                                .acquire_owned()
                                .await
                                .map_err(|e| format!("Failed to acquire permit: {}", e))?;
                            let result: Result<Value, String> = executor
                                .execute_with_controller_and_parent(
                                    item,
                                    ctx.as_ref(),
                                    controller,
                                    parent_runner_id,
                                )
                                .await
                                .map_err(|e| e.to_string());
                            drop(permit);
                            match result {
                                Ok(output) => {
                                    let _ = tx
                                        .send(TaskEvent::Next(
                                            node_id,
                                            json!({"kind":"item","index":idx,"output":output}),
                                        ))
                                        .await;
                                        Ok::<(usize, Value), String>((idx, output))
                                }
                                Err(e) => Err(e),
                            }
                        });
                    }

                    let mut results: Vec<Option<Value>> = vec![None; total];
                    let mut error: Option<String> = None;

                    while let Some(joined) = join_set.join_next().await {
                        match joined {
                            Ok(Ok((idx, output))) => {
                                if idx < results.len() {
                                    results[idx] = Some(output);
                                }
                            }
                            Ok(Err(e)) => {
                                error = Some(e);
                                join_set.abort_all();
                                break;
                            }
                            Err(e) => {
                                error = Some(format!("Task panicked: {}", e));
                                join_set.abort_all();
                                break;
                            }
                        }
                    }

                    if let Some(e) = error {
                        let _ = tx.send(TaskEvent::Error(node_id, e)).await;
                        return;
                    }

                    let ordered: Vec<Value> = results
                        .into_iter()
                        .map(|v| v.unwrap_or(Value::Null))
                        .collect();
                    let _ = tx
                        .send(TaskEvent::Next(
                            node_id.clone(),
                            json!({"kind":"done","count":ordered.len()}),
                        ))
                        .await;
                    let _ = tx
                        .send(TaskEvent::Completed(node_id, Some(Value::Array(ordered))))
                        .await;
                });

                Box::new(TaskSubscription { handle })
            }),
        }
    }
}

#[async_trait]
impl Node for MapNode {
    const TRIGGER_STRATEGY: flow::TriggerStrategy = flow::TriggerStrategy::AllUpstreamReady;

    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String> {
        // 从输入中提取数组
        let items: Vec<Value> = match input.get_any_as::<Vec<Value>>() {
            Some(items) => items,
            None => {
                // 尝试获取单个值作为单元素数组
                match input.get_any() {
                    Some(value) => vec![value.clone()],
                    None => return Err("Map requires array input".to_string()),
                }
            }
        };

        if self.streaming {
            let controller = flow::try_controller()
                .ok_or_else(|| "Missing Controller in current task scope".to_string())?;
            let parent_runner_id = flow::try_runner_id();
            let stream = self.build_stream(items, controller, parent_runner_id);
            let mut out = Output::new(None);
            out.set_stream(stream);
            Ok(out)
        } else {
            let outputs = self.execute_map(ctx, items).await?;
            Ok(Value::Array(outputs).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_node_creation() {
        let subgraph = Graph::new();
        let map_node = MapNode::new(subgraph, 10);

        assert_eq!(map_node.max_concurrency(), 10);
    }

    #[test]
    fn test_map_node_with_entry_exit() {
        let subgraph = Graph::new();
        let map_node = MapNode::with_entry_exit(subgraph, 5, "input", "output");

        assert_eq!(map_node.max_concurrency(), 5);
    }
}
