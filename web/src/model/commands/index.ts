/**
 * Command 类型定义
 * 所有 Graph 操作都定义为 Command，通过 RxCommandBus 分发
 */

import type { XYPosition, Connection } from '@xyflow/react';
import type { Node, NodeData, Edge, NodeChange, EdgeChange } from '../types';

// ===== Node Commands =====

export interface AddNodeCommand {
  type: 'ADD_NODE';
  payload: {
    id?: string;
    nodeType: string;
    position: XYPosition;
    data?: Partial<NodeData>;
  };
}

export interface RemoveNodeCommand {
  type: 'REMOVE_NODE';
  payload: {
    id: string;
  };
}

export interface UpdateNodeDataCommand {
  type: 'UPDATE_NODE_DATA';
  payload: {
    id: string;
    data: Partial<NodeData>;
  };
}

export interface UpdateNodePositionCommand {
  type: 'UPDATE_NODE_POSITION';
  payload: {
    id: string;
    position: XYPosition;
  };
}

export interface UpdateNodeDimensionsCommand {
  type: 'UPDATE_NODE_DIMENSIONS';
  payload: {
    id: string;
    width: number;
    height: number;
  };
}

export interface SelectNodeCommand {
  type: 'SELECT_NODE';
  payload: {
    id: string | null;
  };
}

export interface ApplyNodeChangesCommand {
  type: 'APPLY_NODE_CHANGES';
  payload: {
    changes: NodeChange[];
  };
}

export interface UpdateNodeStatusCommand {
  type: 'UPDATE_NODE_STATUS';
  payload: {
    id: string;
    status: 'idle' | 'running' | 'completed' | 'error';
    errorMessage?: string;
  };
}

export interface UpdateNodeInputCommand {
  type: 'UPDATE_NODE_INPUT';
  payload: {
    id: string;
    key: string;
    value: any;
  };
}

export interface UpdateNodeInputsCommand {
  type: 'UPDATE_NODE_INPUTS';
  payload: {
    id: string;
    inputs: Record<string, any>;
  };
}

export interface UpdateNodeOutputCommand {
  type: 'UPDATE_NODE_OUTPUT';
  payload: {
    id: string;
    key: string;
    value: any;
  };
}

// ===== Edge Commands =====

export interface AddEdgeCommand {
  type: 'ADD_EDGE';
  payload: {
    edge: Edge;
  };
}

export interface RemoveEdgeCommand {
  type: 'REMOVE_EDGE';
  payload: {
    id: string;
  };
}

export interface OnConnectCommand {
  type: 'ON_CONNECT';
  payload: Connection;
}

export interface UpdateEdgeCommand {
  type: 'UPDATE_EDGE';
  payload: {
    id: string;
    updates: Partial<Edge>;
  };
}

export interface ApplyEdgeChangesCommand {
  type: 'APPLY_EDGE_CHANGES';
  payload: {
    changes: EdgeChange[];
  };
}

// ===== Graph Commands =====

export interface SetNodesCommand {
  type: 'SET_NODES';
  payload: {
    nodes: Node[];
  };
}

export interface SetEdgesCommand {
  type: 'SET_EDGES';
  payload: {
    edges: Edge[];
  };
}

export interface PasteNodesCommand {
  type: 'PASTE_NODES';
  payload: {
    nodes: Node[];
    edges: Edge[];
  };
}

export interface GroupNodesCommand {
  type: 'GROUP_NODES';
  payload: {
    nodeIds: string[];
  };
}

export interface ToggleSubgraphCommand {
  type: 'TOGGLE_SUBGRAPH';
  payload: {
    nodeId: string;
  };
}

export interface ResetExecutionStateCommand {
  type: 'RESET_EXECUTION_STATE';
  payload: Record<string, never>; // Empty payload
}

// ===== Batch Commands =====

export interface BatchCommand {
  type: 'BATCH';
  payload: {
    commands: GraphCommand[];
  };
}

// ===== Union Type =====

export type GraphCommand =
  // Node commands
  | AddNodeCommand
  | RemoveNodeCommand
  | UpdateNodeDataCommand
  | UpdateNodePositionCommand
  | UpdateNodeDimensionsCommand
  | SelectNodeCommand
  | ApplyNodeChangesCommand
  | UpdateNodeStatusCommand
  | UpdateNodeInputCommand
  | UpdateNodeInputsCommand
  | UpdateNodeOutputCommand
  // Edge commands
  | AddEdgeCommand
  | RemoveEdgeCommand
  | OnConnectCommand
  | UpdateEdgeCommand
  | ApplyEdgeChangesCommand
  // Graph commands
  | SetNodesCommand
  | SetEdgesCommand
  | PasteNodesCommand
  | GroupNodesCommand
  | ToggleSubgraphCommand
  | ResetExecutionStateCommand
  // Batch
  | BatchCommand;
