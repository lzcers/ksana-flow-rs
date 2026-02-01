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

- `SubgraphNode::run`：[node.rs](file:///d:/codes/ksana-flow-rs/flow/src/flow/graph/subgraph/node.rs)
- `SubgraphExecutor::execute`：[executor.rs](file:///d:/codes/ksana-flow-rs/flow/src/flow/graph/subgraph/executor.rs)

### 2) 内层代理：`SubgraphInNode`

对每个“外部 source -> 子图内部 target”的跨边，编译器会在子图内部生成一个 `SubgraphInNode { key: source_id }` 作为代理输入节点，并改写连线：

`start -> proxy_in(source_id) -> internal_target`

`SubgraphInNode` 的行为是：

- 若入口值是对象，则按 `key` 取出对应字段作为输出；
- 若入口值不是对象，则直接透传该值。

这样子图仍然只需要“一个统一入口值”，但内部能按外部 `source` 把输入拆分成多路，保持跨边语义不丢失。

对应实现：

- `SubgraphInNode`：[node.rs](file:///d:/codes/ksana-flow-rs/flow/src/flow/graph/subgraph/node.rs)
- 代理节点生成与边改写：[compiler.rs](file:///d:/codes/ksana-flow-rs/flow/src/flow/graph/compiler.rs)

## 输入打包/拆包约定

父图传入子图时会做一次“归一化”：

- 若存在 `INPUT_EXTERNAL_START`，直接使用其值作为入口；
- 否则：
    - 0 个输入：`null`
    - 1 个输入：该唯一值
    - 多个输入：打包成 `{handle: value, ...}` 对象

子图内部的 `SubgraphInNode` 会把对象按 `source_id` 拆成对应的那一路输入；若不是对象则透传。

## 编译期边重写概览

子图折叠发生在蓝图编译阶段（`compiler.rs`）：

- 子图内部会生成一对统一的 `start_id/end_id` 节点。
- 内部普通边保持不变。
- “外部 -> 内部”跨边会被改写为：`start -> proxy_in(source) -> target`。
- “内部 -> 外部”跨边用于推导出口候选节点，并统一连到 `end_id`，保证子图输出收敛。
