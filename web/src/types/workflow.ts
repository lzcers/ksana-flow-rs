import type { Node, Edge } from '@xyflow/react';

export type NodeType =
  | 'LLMNode'
  | 'TextNode'
  | 'EmailNotifyNode'
  | 'TimerNode'
  | 'Backtester'
  | 'ReactiveSourceNode'
  | 'VOLMFINode'
  | string;

export interface WorkflowNodeData extends Record<string, unknown> {
  label: string;
  type: string;
  description?: string;
  config?: Record<string, any>;
  status?: 'idle' | 'running' | 'completed' | 'error';
  errorMessage?: string;
  lastMessage?: any;
}

export type WorkflowNode = Node<WorkflowNodeData>;
export type WorkflowEdge = Edge;

export interface WorkflowState {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  selectedNodeId: string | null;
}
