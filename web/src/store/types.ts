import type { NodeMetadata } from '../api';
import type { Node, Edge, NodeChange, EdgeChange, Connection } from '../model/types';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

export interface WorkflowSlice {
  workflows: { id: number; name: string }[];
  currentWorkflowId: number | null;
  nodeTypes: NodeMetadata[];
  loadMetadata: () => Promise<void>;
  loadWorkflow: (id: number) => Promise<void>;
  saveWorkflow: (name?: string) => Promise<void>;
  renameWorkflow: (id: number, newName: string) => Promise<void>;
  deleteWorkflow: (id: number) => Promise<void>;
  createNewWorkflow: () => Promise<void>;
  setWorkflows: (workflows: { id: number; name: string }[]) => void;
  setCurrentWorkflowId: (id: number | null) => void;
  setNodeTypes: (types: NodeMetadata[]) => void;
}

export interface CanvasSlice {
  nodes: Node[];
  edges: Edge[];
  selectedNodeId: string | null;
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (type: string, position?: { x: number; y: number }) => void;
  deleteNode: (id: string) => void;
  updateNodeData: (id: string, data: Record<string, any>) => void;
  updateNodeDimensions: (id: string, width: number, height: number) => void;
  selectNode: (id: string | null) => void;
  setNodes: (nodes: Node[]) => void;
  setEdges: (edges: Edge[]) => void;
}

export interface ExecutionSlice {
  workflowStatus: WorkflowStatus;
  workflowStatuses: Record<number, WorkflowStatus>;
  currentRunId: string | null;
  runWorkflow: () => Promise<void>;
  pauseWorkflow: () => Promise<void>;
  resumeWorkflow: () => Promise<void>;
  stopWorkflow: () => Promise<void>;
  runNode: (nodeId: string) => Promise<void>;
  setWorkflowStatus: (status: WorkflowStatus) => void;
  setWorkflowStatuses: (statuses: Record<number, WorkflowStatus>) => void;
  setCurrentRunId: (runId: string | null) => void;
  initializeWebSocket: () => () => void;
  handleWebSocketMessage: (message: any) => void;
}

export interface NotificationSlice {
  notify: (type: 'success' | 'error' | 'info', message: string) => void;
  setNotificationHandler: (handler: (type: 'success' | 'error' | 'info', message: string) => void) => void;
}

export type StoreState = WorkflowSlice & CanvasSlice & ExecutionSlice & NotificationSlice;
