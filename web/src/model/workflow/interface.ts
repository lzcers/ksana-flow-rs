import type { Observable } from 'rxjs';
import { RxWorkflow, type RxWorkflowOptions } from './workflowRx';
import type { WorkflowState } from './types';
import type { GraphCommand } from './commands';
import type { XYPosition, Connection } from '@xyflow/react';
import type { Edge } from './types';
import type { Immutable } from 'immer';

// Re-export RxWorkflow types
export type { RxWorkflow, RxWorkflowOptions };

export interface WorkflowModelDispatchers {
  addNode: (
    type: string,
    position: XYPosition,
    options?: { id?: string; data?: Record<string, any> }
  ) => void;
  deleteNode: (id: string) => void;
  updateNodeData: (id: string, data: Record<string, any>) => void;
  updateNodePosition: (id: string, position: XYPosition) => void;
  updateNodeDimensions: (id: string, width: number, height: number) => void;
  selectNode: (id: string | null) => void;
  onConnect: (connection: Connection) => void;
  addEdge: (edge: Edge) => void;
  removeEdge: (id: string) => void;
  setNodes: (nodes: WorkflowState['nodes']) => void;
  setEdges: (edges: WorkflowState['edges']) => void;
  pasteNodes: (nodes: WorkflowState['nodes'], edges: WorkflowState['edges']) => void;
  groupNodes: (nodeIds: string[]) => void;
  toggleSubgraph: (nodeId: string) => void;
  handleNodeDragStop: (nodeId: string) => void;
}

// 保持接口兼容性，虽然内部实现换成了 RxWorkflow
export interface WorkflowModelInterface {
  rxWorkflow: RxWorkflow;
  state$: Observable<Immutable<WorkflowState>>;
  viewState$: Observable<Immutable<WorkflowState>>;
  canUndo$: Observable<boolean>;
  canRedo$: Observable<boolean>;
  dispatch: (command: GraphCommand) => void;
  undo: () => void;
  redo: () => void;
  dispatchers: WorkflowModelDispatchers;
  getSnapshot: () => Immutable<WorkflowState>;
  destroy: () => void;
}

export function createWorkflowModel(
  options: RxWorkflowOptions = {}
): WorkflowModelInterface {
  const rxWorkflow = new RxWorkflow(options);

  const dispatch = (command: GraphCommand) => rxWorkflow.dispatch(command);

  const dispatchers: WorkflowModelDispatchers = {
    addNode: (type, position, options) =>
      dispatch({
        type: 'ADD_NODE',
        payload: {
          id: options?.id,
          nodeType: type,
          position,
          data: options?.data,
        },
      }),
    deleteNode: (id) => dispatch({ type: 'REMOVE_NODE', payload: { id } }),
    updateNodeData: (id, data) =>
      dispatch({ type: 'UPDATE_NODE', payload: { id, updates: { data } } }),
    updateNodePosition: (id, position) =>
      dispatch({ type: 'UPDATE_NODE', payload: { id, updates: { position } } }),
    updateNodeDimensions: (id, width, height) =>
      dispatch({
        type: 'UPDATE_NODE',
        payload: { id, updates: { dimensions: { width, height } } },
      }),
    selectNode: (id) => dispatch({ type: 'SELECT_NODE', payload: { id } }),
    onConnect: (connection: Connection) =>
      dispatch({ type: 'UPDATE_EDGES', payload: { connect: connection } }),
    addEdge: (edge) => dispatch({ type: 'UPDATE_EDGES', payload: { add: [edge] } }),
    removeEdge: (id) => dispatch({ type: 'UPDATE_EDGES', payload: { remove: [id] } }),
    setNodes: (nodes) => dispatch({ type: 'SET_NODES', payload: { nodes } }),
    setEdges: (edges) => dispatch({ type: 'SET_EDGES', payload: { edges } }),
    pasteNodes: (nodes, edges) =>
      dispatch({ type: 'PASTE_NODES', payload: { nodes, edges } }),
    groupNodes: (nodeIds) =>
      dispatch({ type: 'GROUP_NODES', payload: { nodeIds } }),
    toggleSubgraph: (nodeId) =>
      dispatch({ type: 'TOGGLE_SUBGRAPH', payload: { nodeId } }),
    handleNodeDragStop: (nodeId) =>
      dispatch({ type: 'HANDLE_NODE_DRAG_STOP', payload: { nodeId } }),
  };

  return {
    rxWorkflow,
    state$: rxWorkflow.state$,
    viewState$: rxWorkflow.viewState$,
    canUndo$: rxWorkflow.canUndo$,
    canRedo$: rxWorkflow.canRedo$,
    dispatch,
    undo: () => rxWorkflow.undo(),
    redo: () => rxWorkflow.redo(),
    dispatchers,
    getSnapshot: () => rxWorkflow.currentState,
    destroy: () => rxWorkflow.destroy(),
  };
}
