/**
 * Command 类型定义
 * 所有 Graph 操作都定义为 Command，通过 RxCommandBus 分发
 */

import type { XYPosition, Connection } from '@xyflow/react';
import type { Node, NodeData, Edge, NodeChange, EdgeChange } from '../types';

export interface BaseCommand {
  meta?: {
    skipHistory?: boolean;
  };
}

// ===== Node Commands =====

export interface AddNodeCommand extends BaseCommand {
  type: 'ADD_NODE';
  payload: {
    id?: string;
    nodeType: string;
    position: XYPosition;
    data?: Partial<NodeData>;
  };
}

export interface RemoveNodeCommand extends BaseCommand {
  type: 'REMOVE_NODE';
  payload: {
    id: string;
  };
}

export interface UpdateNodeDataCommand extends BaseCommand {
  type: 'UPDATE_NODE_DATA';
  payload: {
    id: string;
    data: Partial<NodeData>;
  };
}

export interface UpdateNodePositionCommand extends BaseCommand {
  type: 'UPDATE_NODE_POSITION';
  payload: {
    id: string;
    position: XYPosition;
  };
}

export interface UpdateNodeDimensionsCommand extends BaseCommand {
  type: 'UPDATE_NODE_DIMENSIONS';
  payload: {
    id: string;
    width: number;
    height: number;
  };
}

export interface SelectNodeCommand extends BaseCommand {
  type: 'SELECT_NODE';
  payload: {
    id: string | null;
  };
}

export interface ApplyNodeChangesCommand extends BaseCommand {
  type: 'APPLY_NODE_CHANGES';
  payload: {
    changes: NodeChange[];
  };
}

export interface UpdateNodeStatusCommand extends BaseCommand {
  type: 'UPDATE_NODE_STATUS';
  payload: {
    id: string;
    status: 'idle' | 'running' | 'completed' | 'error';
    errorMessage?: string;
  };
}

export interface UpdateNodeInputCommand extends BaseCommand {
  type: 'UPDATE_NODE_INPUT';
  payload: {
    id: string;
    key: string;
    value: any;
  };
}

export interface UpdateNodeInputsCommand extends BaseCommand {
  type: 'UPDATE_NODE_INPUTS';
  payload: {
    id: string;
    inputs: Record<string, any>;
  };
}

export interface UpdateNodeOutputCommand extends BaseCommand {
  type: 'UPDATE_NODE_OUTPUT';
  payload: {
    id: string;
    key: string;
    value: any;
  };
}

// ===== Edge Commands =====

export interface AddEdgeCommand extends BaseCommand {
  type: 'ADD_EDGE';
  payload: {
    edge: Edge;
  };
}

export interface RemoveEdgeCommand extends BaseCommand {
  type: 'REMOVE_EDGE';
  payload: {
    id: string;
  };
}

export interface OnConnectCommand extends BaseCommand {
  type: 'ON_CONNECT';
  payload: Connection;
}

export interface UpdateEdgeCommand extends BaseCommand {
  type: 'UPDATE_EDGE';
  payload: {
    id: string;
    updates: Partial<Edge>;
  };
}

export interface ApplyEdgeChangesCommand extends BaseCommand {
  type: 'APPLY_EDGE_CHANGES';
  payload: {
    changes: EdgeChange[];
  };
}

// ===== Graph Commands =====

export interface SetNodesCommand extends BaseCommand {
  type: 'SET_NODES';
  payload: {
    nodes: Node[];
  };
}

export interface SetEdgesCommand extends BaseCommand {
  type: 'SET_EDGES';
  payload: {
    edges: Edge[];
  };
}

export interface PasteNodesCommand extends BaseCommand {
  type: 'PASTE_NODES';
  payload: {
    nodes: Node[];
    edges: Edge[];
  };
}

export interface GroupNodesCommand extends BaseCommand {
  type: 'GROUP_NODES';
  payload: {
    nodeIds: string[];
  };
}

export interface ToggleSubgraphCommand extends BaseCommand {
  type: 'TOGGLE_SUBGRAPH';
  payload: {
    nodeId: string;
  };
}

export interface HandleNodeDragStopCommand extends BaseCommand {
  type: 'HANDLE_NODE_DRAG_STOP';
  payload: {
    nodeId: string;
  };
}

export interface ResetExecutionStateCommand extends BaseCommand {
  type: 'RESET_EXECUTION_STATE';
  payload: Record<string, never>; // Empty payload
}

// ===== Batch Commands =====

export interface BatchCommand extends BaseCommand {
  type: 'BATCH';
  payload: {
    commands: GraphCommand[];
  };
}

// ===== History Commands =====

export interface UndoCommand extends BaseCommand {
  type: 'UNDO';
  payload: Record<string, never>;
}

export interface RedoCommand extends BaseCommand {
  type: 'REDO';
  payload: Record<string, never>;
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
  | HandleNodeDragStopCommand
  | ResetExecutionStateCommand
  // Batch
  | BatchCommand
  // History
  | UndoCommand
  | RedoCommand;
