# 子图（Subgraph）与代理节点

本目录实现“子图”运行时能力：把一张 `Graph` 当成一个可执行节点嵌入到另一张图里，同时保留蓝图层面的分组（group）与跨边（外部节点连到子图内部节点）语义。

## 为什么需要代理节点

子图在运行时有两个关键约束：

- 子图 Runner 只能从一个统一入口节点启动，并接收“一份入口数据”（`start_id` + `INPUT_EXTERNAL_START`）。
- 蓝图层允许多条“外部 -> 子图内部”的跨边：不同外部 `source` 可能连到子图内部不同 `target`，语义上这些输入是彼此独立的。

为同时满足这两点，编译期会引入代理节点，把跨图边改写成子图内部的普通边，并在内部把统一入口数据拆分成多路。

## 运行时组成

### 1) 外层代理：`SubgraphNode`

外层图无法直接“连一条边到一张 Graph”，因为运行时执行单位是 `graph::Node`。因此子图容器节点会被编译为 `SubgraphNode`：

- 在父图里表现为普通节点（实现 `Node::run`）。
- `run` 会把父图的 `Input` 打包成一个 `serde_json::Value`，调用 `SubgraphExecutor::execute` 执行子图 Runner。
- 子图完成后，从子图 `exit_node` 的输出取结果作为该节点输出。

对应实现：

- `SubgraphNode::run`：`node.rs`
- `SubgraphExecutor::execute`：`executor.rs`

### 2) 内层代理：`SubgraphInNode`

对每个“外部 source -> 子图内部 target”的跨边，编译器会在子图内部生成一个 `SubgraphInNode { key: source_id }` 作为代理输入节点，并改写连线：

`start -> proxy_in(source_id) -> internal_target`

`SubgraphInNode` 的行为是：

- 若入口值是内部 tagged envelope，则从 `values` 中按 `source_id` 取出对应值；
- 若入口值没有内部标签，则视为直接调用的单值输入并原样透传，包括 JSON Object。

这样子图仍然只需要“一个统一入口值”，但内部能按外部 `source` 把输入拆分成多路，保持跨边语义不丢失。

对应实现：

- `SubgraphInNode`：`node.rs`
- 代理节点生成与边改写：`compiler.rs`

## 输入打包/拆包约定

父图传入子图时会做一次“归一化”：

- 若存在 `INPUT_EXTERNAL_START`，直接使用其值作为未标记的单值入口；
- 否则，单路和多路父图输入都编码为内部信封：

```json
{
  "__ksana_flow_subgraph_input": {
    "version": 1,
    "kind": "routed",
    "values": {
      "source-node-id": "value"
    }
  }
}
```

单路也保留来源 ID，因此业务 Object 不会再与路由表混淆。保留的特殊键只用于
flow 内部协议，业务数据不应主动构造同形信封。

## 编译期边重写概览

子图折叠发生在蓝图编译阶段（`compiler.rs`）：

- 子图内部会生成一对统一的 `start_id/end_id` 节点。
- 内部普通边保持不变。
- “外部 -> 内部”跨边会被改写为：`source -> group`（运输）以及
  `start -> proxy_in(source) -> target`（逐边条件判断）。
- “内部 -> 外部”跨边用于推导出口候选节点，并统一连到 `end_id`，保证子图输出收敛。
- 静态 `false` 分支不会进入运行时图，由此不可达的成员节点会在编译期剪枝，避免
  它们阻塞 `AllUpstreamReady` 出口。

当前运行时边只支持节点级控制流。`EdgeKind::Data`、port/data type 元数据、无法
保真的多来源 outbound 和并行同端点边会在编译期明确报错，而不是静默降级。

## 生命周期与出口约定

- 子 Runner 注册由 RAII guard 持有；成功、错误、超时或 future 被取消都会注销。
- entry/exit 节点在创建 Runner 前校验。
- exit 必须达到 `Completed` 且产生输出；未执行与“合法输出为 null”是不同状态。
- 每次调用仍创建独立 Runner 和节点实例。这是有状态节点隔离的语义保证；高 fan-out
  应由上层限制并发，不能通过复用节点实例换取速度。
