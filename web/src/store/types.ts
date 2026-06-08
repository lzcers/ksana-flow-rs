import type { NodeMetadata, UploadedFile } from "../api";
import type { Node, Edge, NodeChange, EdgeChange, Connection, WorkflowStatus, NodeData } from "../model/workflow/types";
import type { WorkflowBlueprint } from "@/model/workflow/adapters/blueprintAdapter";
import type { GraphKey } from "../model/workflowManager";
import type { OnNodeDrag } from "@xyflow/react";

// WorkflowBlueprint type is now imported from adapter

// 处理工作流相关的状态和操作
export interface Workflow {
    nodeTypes: NodeMetadata[];
    workflows: { id: number; name: string }[];
    activeGraphKey: GraphKey | null;
    currentWorkflowId: number | null;
    currentSpaceId: string | null;
    currentRunId: string | null;
    currentWorkflowStatus: WorkflowStatus;
    workflowStatuses: Record<number, WorkflowStatus>;
    isLoadingWorkflow: boolean;
    setActiveGraphKey: (graphKey: GraphKey | null) => void;
    setSpaceId: (id: string) => void;
    loadMetadata: () => Promise<void>;
    loadWorkflow: (id: number) => Promise<void>;
    saveWorkflow: (name?: string) => Promise<void>;
    renameWorkflow: (id: number, newName: string) => Promise<void>;
    deleteWorkflow: (id: number) => Promise<void>;
    createNewWorkflow: () => Promise<void>;
    importWorkflow: (blueprint: WorkflowBlueprint) => void;
    getWorkflowBlueprint: () => WorkflowBlueprint;
    uploadFile: (file: File) => Promise<UploadedFile>;
    setWorkflows: (workflows: { id: number; name: string }[]) => void;
    setCurrentWorkflowId: (id: number | null) => void;
    setNodeTypes: (types: NodeMetadata[]) => void;
    runWorkflow: () => Promise<void>;
    pauseWorkflow: () => Promise<void>;
    resumeWorkflow: () => Promise<void>;
    stopWorkflow: () => Promise<void>;
    runNode: (nodeIds: string[]) => Promise<void>;
    initializeWebSocket: () => () => void;
    startAutoSave: () => void;
    stopAutoSave: () => void;
}

// 处理画布相关的状态和操作
export interface Canvas {
    nodes: Node[];
    edges: Edge[];
    selectedNodeId: string[];
    isConnecting: boolean;
    connectionSourceId: string | null;
    dragOverNodeId: string | null;
    switchCanvas: (graphKey: GraphKey) => void;
    onNodesChange: (changes: NodeChange[]) => void;
    onEdgesChange: (changes: EdgeChange[]) => void;
    onNodeDrag: OnNodeDrag<Node>;
    onNodeDragStop: OnNodeDrag<Node>;
    onConnect: (connection: Connection) => void;
    addNode: (type: string, position?: { x: number; y: number }) => void;
    deleteNode: (id: string) => void;
    updateNodeData: (id: string, data: Partial<NodeData>) => void;
    updateNodeDimensions: (id: string, width: number, height: number) => void;
    selectNode: (id: string[]) => void;
    setNodes: (nodes: Node[]) => void;
    setEdges: (edges: Edge[]) => void;
    pasteNodes: (nodes: Node[], edges: Edge[]) => void;
    setConnectionState: (connecting: boolean, sourceId?: string | null) => void;
    groupNodes: (nodeIds: string[]) => void;
    toggleSubgraph: (nodeId: string) => void;
    // History
    undo: () => void;
    redo: () => void;
}

export interface ToastItem {
    id: string;
    message: string;
    type: "success" | "error" | "info";
    duration?: number;
}

export interface Toast {
    toasts: ToastItem[];
    showToast: (message: string, type: "success" | "error" | "info", duration?: number) => void;
    removeToast: (id: string) => void;
    success: (message: string, duration?: number) => void;
    error: (message: string, duration?: number) => void;
    info: (message: string, duration?: number) => void;
}

export type StoreState = Workflow & Canvas & Toast;

export type { WorkflowStatus } from "../model/workflow/types";
