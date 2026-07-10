//! 异步工作流引擎的公共入口。
//!
//! `Graph` 保存可重复实例化的节点工厂和边；每个 `Runner` 在一次执行开始时
//! 物化自己的节点实例，再由 `Scheduler` 生成启动请求、`Executor` 执行节点，
//! 最终通过 `FlowEventEnvelope` 向外报告执行过程。
//!
//! 节点之间统一传递 `serde_json::Value`。需要持续输出时，节点返回
//! `ReactiveStream`，由 Runner 将流事件重新接入同一套调度循环。

mod flow;
mod macros;
pub use flow::*;
#[cfg(test)]
mod tests;
