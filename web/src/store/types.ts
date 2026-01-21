import type { NodeMetadata } from '../api';
import type { Node, Edge, NodeChange, EdgeChange, Connection } from '../model/types';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

export interface WorkflowSlice {
  workflows: { id: number; name: string }[];
  currentWorkflowId: number | null;
  currentSpaceId: string | null;
  nodeTypes: NodeMetadata[];
  setSpaceId: (id: string) => void;
  loadMetadata: () => Promise<void>;
  loadWorkflow: (id: number) => Promise<void>;
  saveWorkflow: (name?: string) => Promise<void>;
  renameWorkflow: (id: number, newName: string) => Promise<void>;
  deleteWorkflow: (id: number) => Promise<void>;
  createNewWorkflow: () => Promise<void>;
  importWorkflow: (blueprint: any) => void;
  getWorkflowBlueprint: () => any;
  uploadFile: (file: File) => Promise<any>;
  setWorkflows: (workflows: { id: number; name: string }[]) => void;
  setCurrentWorkflowId: (id: number | null) => void;
  setNodeTypes: (types: NodeMetadata[]) => void;
  applyExecutionEvent: (event: any) => void;
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
  isConnecting: boolean;
  connectionSourceId: string | null;
  setConnectionState: (connecting: boolean, sourceId?: string | null) => void;
}

export interface ExecutionSlice {
  workflowStatus: WorkflowStatus;
  workflowStatuses: Record<number, WorkflowStatus>;
  runIdToWorkflowId: Record<string, number>;
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

export interface ToastItem {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info';
  duration?: number;
}

export interface ToastSlice {
  toasts: ToastItem[];
  showToast: (message: string, type: 'success' | 'error' | 'info', duration?: number) => void;
  removeToast: (id: string) => void;
  success: (message: string, duration?: number) => void;
  error: (message: string, duration?: number) => void;
  info: (message: string, duration?: number) => void;
}

export type StoreState = WorkflowSlice & CanvasSlice & ExecutionSlice & ToastSlice;
