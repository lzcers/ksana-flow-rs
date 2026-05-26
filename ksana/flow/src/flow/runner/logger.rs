//! 统一的 Runner 日志记录模块
//!
//! 提供结构化的日志记录函数，用于追踪节点状态变化、Runner 状态变化
//! 以及执行生命周期事件。

use super::exec_context::NodeState;
use super::runner::RunnerState;
use crate::RunnerId;
use crate::flow::graph::NodeId;
use std::time::Duration;
use tracing::{Span, debug, info, info_span, trace, warn};

/// 记录节点状态变化
///
/// # 字段
/// - `node.id`: 节点 ID
/// - `node.state.old`: 旧状态
/// - `node.state.new`: 新状态
/// - `runner.id`: Runner ID
pub fn log_node_state_change(
    node_id: &NodeId,
    old_state: Option<NodeState>,
    new_state: NodeState,
    runner_id: RunnerId,
) {
    match old_state {
        Some(old) => {
            info!(
                node.id = %node_id,
                node.state.old = ?old,
                node.state.new = ?new_state,
                runner.id = %runner_id,
                "Node state changed"
            );
        }
        None => {
            info!(
                node.id = %node_id,
                node.state.new = ?new_state,
                runner.id = %runner_id,
                "Node state initialized"
            );
        }
    }
}

/// 记录 Runner 状态变化
///
/// # 字段
/// - `runner.id`: Runner ID
/// - `runner.state.old`: 旧状态
/// - `runner.state.new`: 新状态
pub fn log_runner_state_change(
    runner_id: RunnerId,
    old_state: RunnerState,
    new_state: RunnerState,
) {
    info!(
        runner.id = %runner_id,
        runner.state.old = ?old_state,
        runner.state.new = ?new_state,
        "Runner state changed"
    );
}

/// 记录节点执行开始
///
/// # 字段
/// - `node.id`: 节点 ID
/// - `runner.id`: Runner ID
/// - `task.active_count`: 当前活跃任务数
pub fn log_node_execution_start(node_id: &NodeId, runner_id: RunnerId, active_task_count: usize) {
    info!(
        node.id = %node_id,
        runner.id = %runner_id,
        task.active_count = active_task_count,
        "Node execution started"
    );
}

/// 记录节点执行完成
///
/// # 字段
/// - `node.id`: 节点 ID
/// - `runner.id`: Runner ID
/// - `node.duration_ms`: 执行耗时（毫秒）
/// - `output.type`: 输出类型（可选）
pub fn log_node_execution_completed(
    node_id: &NodeId,
    runner_id: RunnerId,
    duration: Duration,
    has_output: bool,
) {
    info!(
        node.id = %node_id,
        runner.id = %runner_id,
        node.duration_ms = duration.as_millis() as u64,
        output.has_value = has_output,
        "Node execution completed"
    );
}

/// 记录节点执行错误
///
/// # 字段
/// - `node.id`: 节点 ID
/// - `runner.id`: Runner ID
/// - `error`: 错误信息
pub fn log_node_execution_error(node_id: &NodeId, runner_id: RunnerId, error: &str) {
    // 使用 warn 级别，因为错误已被捕获并继续执行
    warn!(
        node.id = %node_id,
        runner.id = %runner_id,
        error = error,
        "Node execution failed"
    );
}

/// 记录 Runner 启动
///
/// # 字段
/// - `runner.id`: Runner ID
/// - `graph.node_count`: 图中节点数量
pub fn log_runner_started(runner_id: RunnerId, node_ids: &[NodeId], node_count: usize) {
    info!(
        runner.id = %runner_id,
        node_ids = ?node_ids,
        graph.node_count = node_count,
        "Runner started"
    );
}

/// 记录 Runner 完成
///
/// # 字段
/// - `runner.id`: Runner ID
/// - `runner.duration_ms`: 总执行耗时（毫秒）
pub fn log_runner_completed(runner_id: RunnerId, duration: Duration) {
    info!(
        runner.id = %runner_id,
        runner.duration_ms = duration.as_millis() as u64,
        "Runner completed"
    );
}

/// 记录 Runner 被终止
///
/// # 字段
/// - `runner.id`: Runner ID
/// - `reason`: 终止原因
pub fn log_runner_terminated(runner_id: RunnerId, reason: &str) {
    info!(
        runner.id = %runner_id,
        reason = reason,
        "Runner terminated"
    );
}

/// 记录调度事件
///
/// # 字段
/// - `from.node_id`: 上游节点 ID
/// - `to.node_id`: 下游节点 ID
/// - `runner.id`: Runner ID
pub fn log_node_scheduled(from_node_id: &NodeId, to_node_id: &NodeId, runner_id: RunnerId) {
    debug!(
        from.node_id = %from_node_id,
        to.node_id = %to_node_id,
        runner.id = %runner_id,
        "Node scheduled"
    );
}

/// 创建一个用于追踪 Runner 执行的 Span
///
/// 使用时需要手动记录进入和退出
pub fn create_runner_span(runner_id: RunnerId, node_count: usize) -> Span {
    info_span!(
        "runner",
        runner.id = %runner_id,
        graph.node_count = node_count,
    )
}

/// 创建一个用于追踪节点执行的 Span
///
/// 作为 runner span 的子 span
pub fn create_node_span(node_id: &NodeId, runner_id: RunnerId, parent: &Span) -> Span {
    info_span!(
        parent: parent,  // 显式指定父 span
        "node",
        node.id = %node_id,
        runner.id = %runner_id,
    )
}

/// 记录流式输出事件
///
/// # 字段
/// - `node.id`: 节点 ID
/// - `runner.id`: Runner ID
/// - `chunk.size`: 数据块大小
pub fn log_stream_chunk(node_id: &NodeId, runner_id: RunnerId, chunk_size: usize) {
    trace!(
        node.id = %node_id,
        runner.id = %runner_id,
        chunk.size = chunk_size,
        "Stream chunk received"
    );
}

/// 记录流式输出开始
///
/// # 字段
/// - `node.id`: 节点 ID
/// - `runner.id`: Runner ID
pub fn log_stream_started(node_id: &NodeId, runner_id: RunnerId) {
    info!(
        node.id = %node_id,
        runner.id = %runner_id,
        "Stream started"
    );
}

/// 记录流式输出结束
///
/// # 字段
/// - `node.id`: 节点 ID
/// - `runner.id`: Runner ID
/// - `total_chunks`: 总数据块数
pub fn log_stream_ended(node_id: &NodeId, runner_id: RunnerId, total_chunks: usize) {
    info!(
        node.id = %node_id,
        runner.id = %runner_id,
        stream.total_chunks = total_chunks,
        "Stream ended"
    );
}
