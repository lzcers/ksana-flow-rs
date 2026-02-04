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

export interface NodeData extends Record<string, unknown> {
  label?: string;
  description?: string;
  expanded?: boolean;
  expandedSize?: { width: number; height: number };
  collapsedSize?: { width: number; height: number };
  inputs?: Record<string, any>;
  outputs?: Record<string, any>;
  config?: Record<string, any>;
  status?: 'idle' | 'running' | 'completed' | 'error';
  errorMessage?: string;
  lastMessage?: any;
  lastMessageRunId?: string;
  isOutputStream?: boolean;
  upstreamIsStreaming?: boolean;
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
  selectedNodeId: string | null;
}

export type { EdgeChange, Connection };
