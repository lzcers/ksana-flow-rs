import type { Observable } from 'rxjs';
import type { Immutable } from 'immer';
import { RxWorkflow, type RxWorkflowOptions } from './RxWorkflow';
import type { WorkflowState, Node, Edge, NodeStatus, NodeData } from './types';
import type { CommandMeta, GraphCommand } from './commands';
import type { XYPosition, Connection, EdgeChange, NodeChange } from '@xyflow/react';

// Re-export RxWorkflow types
export type { RxWorkflow, RxWorkflowOptions };

export interface WorkflowModelDispatchers {
  // node dispatchers
  addNode: (
    type: string,
    position: XYPosition,
    options?: { id?: string; data?: Record<string, any> },
    meta?: CommandMeta,
  ) => void;
  setNodes: (nodes: WorkflowState['nodes'], meta?: CommandMeta) => void;
  deleteNode: (id: string, meta?: CommandMeta) => void;
  updateNodeData: (id: string, data: Partial<NodeData> & Record<string, any>, meta?: CommandMeta) => void;
  updateNodePosition: (id: string, position: XYPosition, meta?: CommandMeta) => void;
  updateNodeDimensions: (id: string, width: number, height: number, meta?: CommandMeta) => void;
  updateNodeStatus: (id: string, status: NodeStatus, meta?: CommandMeta) => void;
  groupNodes: (nodeIds: string[], meta?: CommandMeta) => void;
  toggleSubgraph: (nodeId: string, meta?: CommandMeta) => void;
  handleNodeDragStop: (nodeId: string, meta?: CommandMeta) => void;
  applyNodeChanges: (changes: NodeChange[], meta?: CommandMeta) => void;
  resetAllNodeStatus: (meta?: CommandMeta) => void;
  // edge dispatchers
  addEdge: (edge: Edge, meta?: CommandMeta) => void;
  removeEdge: (id: string, meta?: CommandMeta) => void;
  setEdges: (edges: WorkflowState['edges'], meta?: CommandMeta) => void;
  onConnect: (connection: Connection, meta?: CommandMeta) => void;
  updateEdges: (changes: EdgeChange[], meta?: CommandMeta) => void;
  pasteNodes: (nodes: Node[], edges: Edge[], meta?: CommandMeta) => void;

}

export interface WorkflowModelInterface {
  rxWorkflow: RxWorkflow;
  state$: Observable<Immutable<WorkflowState>>;
  viewState$: Observable<Immutable<WorkflowState>>;
  canUndo$: Observable<boolean>;
  canRedo$: Observable<boolean>;
  action: WorkflowModelDispatchers;
  getSnapshot: () => Immutable<WorkflowState>;
  getNodeData: (nodeId: string) => Immutable<Node['data']> | undefined;
  undo: () => void;
  redo: () => void;
  destroy: () => void;
}

export function createWorkflowModel(
  options: RxWorkflowOptions = {}
): WorkflowModelInterface {
  const rxWorkflow = new RxWorkflow(options);

  const dispatch = (command: GraphCommand) => rxWorkflow.dispatch(command);

  // 用户操作，会进 History
  const action: WorkflowModelDispatchers = {
    // node dispatchers
    setNodes: (nodes, meta) => dispatch({ type: 'SET_NODES', payload: { nodes }, meta }),
    addNode: (type, position, options, meta) =>
      dispatch({
        type: 'ADD_NODE',
        payload: {
          id: options?.id,
          nodeType: type,
          position,
          data: options?.data,
        },
        meta
      }),
    deleteNode: (id, meta) => dispatch({ type: 'REMOVE_NODE', payload: { id }, meta }),
    updateNodeStatus: (id, status, meta) => dispatch({
      type: "UPDATE_NODE", payload: {
        id,
        updates: { status }
      },
      meta
    }),
    updateNodeData: (id, data, meta) =>
      dispatch({ type: 'UPDATE_NODE', payload: { id, updates: { data } }, meta }),
    updateNodePosition: (id, position, meta) =>
      dispatch({ type: 'UPDATE_NODE', payload: { id, updates: { position } }, meta }),
    updateNodeDimensions: (id, width, height, meta) =>
      dispatch({
        type: 'UPDATE_NODE',
        payload: { id, updates: { dimensions: { width, height } } },
        meta
      }),
    groupNodes: (nodeIds, meta) =>
      dispatch({ type: 'GROUP_NODES', payload: { nodeIds }, meta }),
    toggleSubgraph: (nodeId, meta) =>
      dispatch({ type: 'TOGGLE_SUBGRAPH', payload: { nodeId }, meta }),
    handleNodeDragStop: (nodeId, meta) =>
      dispatch({ type: 'HANDLE_NODE_DRAG_STOP', payload: { nodeId }, meta }),
    applyNodeChanges: (changes, meta) =>
      dispatch({ type: 'APPLY_NODE_CHANGES', payload: { changes }, meta }),
    resetAllNodeStatus: (meta) =>
      dispatch({ type: 'RESET_ALL_NODE_STATUS', meta }),
    // edge dispatchers
    onConnect: (connection: Connection, meta) =>
      dispatch({ type: 'UPDATE_EDGES', payload: { connect: connection }, meta }),
    addEdge: (edge, meta) => dispatch({ type: 'UPDATE_EDGES', payload: { add: [edge] }, meta }),
    removeEdge: (id, meta) => dispatch({ type: 'UPDATE_EDGES', payload: { remove: [id] }, meta }),
    setEdges: (edges, meta) => dispatch({ type: 'SET_EDGES', payload: { edges }, meta }),
    updateEdges: (changes, meta) =>
      dispatch({ type: 'UPDATE_EDGES', payload: { changes: changes }, meta }),
    pasteNodes: (nodes, edges, meta) =>
      dispatch({ type: 'PASTE_NODES', payload: { nodes, edges }, meta }),
  };

  return {
    rxWorkflow,
    state$: rxWorkflow.state$,
    viewState$: rxWorkflow.viewState$,
    canUndo$: rxWorkflow.canUndo$,
    canRedo$: rxWorkflow.canRedo$,
    action,
    undo: () => rxWorkflow.undo(),
    redo: () => rxWorkflow.redo(),
    getSnapshot: () => rxWorkflow.currentState,
    getNodeData: (nodeId: string) => rxWorkflow.getNodeData(nodeId),
    destroy: () => rxWorkflow.destroy(),
  };
}
