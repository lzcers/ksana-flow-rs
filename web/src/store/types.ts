import type { NodeMetadata } from '../api';
import type { Node, Edge, NodeChange, EdgeChange, Connection } from '../model/types';
import type { Observable } from 'rxjs';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

export type FlowEvent =
  | { NodeStarted: string }
  | { NodeCompleted: string }
  | { NodeError: [string, string] }
  | { NodeInMessage: [string, any] }
  | { NodeOutMessage: [string, any] }
  | { NodeStreamStarted: string }
  | { NodeStreamNextMessage: [string, any] }
  | 'FlowPaused'
  | 'FlowResumed'
  | 'FlowStopped'
  | 'FlowFinished';

export interface WebSocketFlowMessage {
  runId?: string;
  event: FlowEvent;
}

export interface WorkflowBlueprint {
  nodes: Partial<Node>[];
  edges: Partial<Edge>[];
}
// 处理工作流相关的状态和操作
export interface Workflow {
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
  getWorkflowBlueprint: () => WorkflowBlueprint;
  uploadFile: (file: File) => Promise<any>;
  setWorkflows: (workflows: { id: number; name: string }[]) => void;
  setCurrentWorkflowId: (id: number | null) => void;
  setNodeTypes: (types: NodeMetadata[]) => void;
  applyExecutionEvent: (event: any) => void;
}

// 处理画布相关的状态和操作
export interface Canvas {
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
  pasteNodes: (nodes: Node[], edges: Edge[]) => void;
  isConnecting: boolean;
  connectionSourceId: string | null;
  setConnectionState: (connecting: boolean, sourceId?: string | null) => void;
}

// 处理执行相关的状态和操作
export interface Execution {
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
  handleWebSocketMessage: (message: WebSocketFlowMessage) => void;
  events$: Observable<WebSocketFlowMessage>;
}

export interface ToastItem {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info';
  duration?: number;
}

export interface Toast {
  toasts: ToastItem[];
  showToast: (message: string, type: 'success' | 'error' | 'info', duration?: number) => void;
  removeToast: (id: string) => void;
  success: (message: string, duration?: number) => void;
  error: (message: string, duration?: number) => void;
  info: (message: string, duration?: number) => void;
}

export type StoreState = Workflow & Canvas & Execution & Toast;
