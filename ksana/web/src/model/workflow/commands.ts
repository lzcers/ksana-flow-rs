import type { XYPosition, Connection } from '@xyflow/react';
import type { Node, NodeData, NodeUpdate, Edge, NodeChange, EdgeChange } from './types';

export type CommandMeta = {
  skipHistory?: boolean;
}
export interface BaseCommand {
  meta?: CommandMeta;
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


export interface UpdateNodeCommand extends BaseCommand {
  type: 'UPDATE_NODE';
  payload: {
    id: string;
    updates: NodeUpdate;
  };
}

export interface ApplyNodeChangesCommand extends BaseCommand {
  type: 'APPLY_NODE_CHANGES';
  payload: {
    changes: NodeChange[];
  };
}

export interface ResetAllNodeStatusCommand extends BaseCommand {
  type: 'RESET_ALL_NODE_STATUS';
}

// ===== Edge Commands =====

export interface UpdateEdgesCommand extends BaseCommand {
  type: 'UPDATE_EDGES';
  payload: {
    add?: Edge[];
    remove?: string[];
    update?: Array<{ id: string; updates: Partial<Edge> }>;
    changes?: EdgeChange[];
    connect?: Connection;
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
  payload: Record<string, never>;
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
  | UpdateNodeCommand
  | ApplyNodeChangesCommand
  | ResetAllNodeStatusCommand
  // Edge commands
  | UpdateEdgesCommand
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
