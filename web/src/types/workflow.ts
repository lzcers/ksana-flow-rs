import type { Node, Edge } from '@xyflow/react';

export type NodeType = 'start' | 'task' | 'condition' | 'end';

export interface WorkflowNodeData extends Record<string, unknown> {
  label: string;
  type: string;
  description?: string;
  config?: Record<string, any>;
  status?: 'idle' | 'running' | 'completed' | 'error';
  errorMessage?: string;
}

export type WorkflowNode = Node<WorkflowNodeData>;
export type WorkflowEdge = Edge;

export interface WorkflowState {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  selectedNodeId: string | null;
}
