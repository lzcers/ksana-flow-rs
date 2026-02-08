import type {
  EdgeChange,
  Connection,
  Node as XNode,
  Edge as XEdge,
  NodeChange as XNodeChange,
} from '@xyflow/react';

export type NodeType =
  | 'LLMNode'
  | 'TextNode'
  | 'TextMergeNode'
  | 'TextFileNode'
  | 'EmailNotifyNode'
  | 'TimerNode'
  | 'Backtester'
  | 'ReactiveSourceNode'
  | 'VOLMFINode'
  | 'SubgraphNode'
  | string;

export type NodeStatus = 'idle' | 'running' | 'completed' | 'error';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

export interface NodeData extends Record<string, unknown> {
  label?: string;   // 名称
  inputs?: Record<string, any>;
  outputs?: Record<string, any>;
  config?: Record<string, any>; // 配置
  status?: NodeStatus; // 运行状态
  errorMessage?: string; // 错误消息
  lastMessage?: any;    // 最后的消息
  isOutputStream?: boolean; // 是否输出流
  expanded?: boolean; // 是否展开，Node Group
  expandedSize?: { width: number; height: number }; // 展开大小
  collapsedSize?: { width: number; height: number }; // 收起大小
}

export interface EdgeData extends Record<string, unknown> {
  __uiSubgraphEdge?: { originalEdgeId: string }
}
export type Node = XNode<NodeData>;
export type Edge = XEdge<EdgeData>;
export type NodeChange = XNodeChange<Node>;

export interface WorkflowState {
  nodes: Node[];
  edges: Edge[];
}

export type { EdgeChange, Connection };
