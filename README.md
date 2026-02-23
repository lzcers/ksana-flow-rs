# ksana-flow-rs

> "What I cannot create, I do not understand"  
> —— Richard Feynman

这是一个玩具项目，用于探索 AI 编程能力的极限，以及理解 AI 应用开发的一些基本范式。

当前项目是一个基于 Graph 的 LLM Agent Builder 框架，用于描述和执行 LLM 应用, 包括一个服务端以及基于 ReactFlow 的前端。
但后面会变成啥样，我也不知道，只是想探索一下。

## flow module

flow 模块提供一个基于图的节点执行框架，用于描述任何可以用图表示的计算流程。

- 定义 LLM 应用的计算图，包括节点和边
- 执行计算图，根据节点的依赖关系，顺序执行节点

## nodes module

nodes 模块提供一些预定义的节点，用于实现 LLM 应用的计算逻辑。

## agent module

agent 模块提供一个 LLM Agent 的实现，包括 Provider，Model, Tool 等组件。
