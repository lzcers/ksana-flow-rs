import {
  type EdgeChange,
  type Connection,
  type Node as XNode,
  type NodeChange as XNodeChange,
  type Edge
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
  | string;

export interface NodeData extends Record<string, unknown> {
  label?: string;
  description?: string;
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

export type Node = XNode<NodeData>;
export type NodeChange = XNodeChange<Node>;
export interface WorkflowState {
  nodes: Node[];
  edges: Edge[];
  selectedNodeId: string | null;
}

export type { EdgeChange, Connection, Edge };
