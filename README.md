尝试用 Rust 实现一个基于 Graph 的 LLM Agent Builder 框架

# Graph module 说明

Graph 模块提供一个基于图的节点执行框架，用于描述任何可以用图表示的计算流程。

总有由几类对象：

-   Node
-   Edge
-   Context

Context 是一个 trait，用于在节点执行过程中共享、传递、保存数据，以及提供节点运行时所需要的上下文信息。
它可以在多个节点共享数据，不同节点依赖不同类型的 Context，因此 Context 是一个泛型 trait。

Runner 是一个动态图的执行器，它根据图的结构，以及节点的运行时上下文，来执行图中的节点。
