import type { Observable } from 'rxjs';
import { map, shareReplay } from 'rxjs/operators';
import { RxCommandBus } from './rx';
import { registerAllHandlers } from './commandHandlers';
import { applyCollapsedSubgraphUi } from './utils';
import type { WorkflowState } from './types';
import type { GraphCommand } from './commands';
import type { XYPosition, Connection } from '@xyflow/react';
import type { Edge } from './types';

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
}

export interface WorkflowModel {
  commandBus: RxCommandBus;
  state$: Observable<WorkflowState>;
  viewState$: Observable<WorkflowState>;
  canUndo$: Observable<boolean>;
  canRedo$: Observable<boolean>;
  dispatch: (command: GraphCommand) => void;
  undo: () => void;
  redo: () => void;
  dispatchers: WorkflowModelDispatchers;
  getSnapshot: () => WorkflowState;
  destroy: () => void;
}

export interface CreateWorkflowModelOptions {
  initialState?: WorkflowState;
  enableLogging?: boolean;
}

const defaultWorkflowState: WorkflowState = {
  nodes: [],
  edges: [],
  selectedNodeId: null,
};

export function createWorkflowModel(
  options: CreateWorkflowModelOptions = {}
): WorkflowModel {
  const commandBus = new RxCommandBus({
    initialState: options.initialState ?? defaultWorkflowState,
    enableLogging: options.enableLogging,
  });
  registerAllHandlers(commandBus);

  const state$ = commandBus.state$.pipe(
    shareReplay({ bufferSize: 1, refCount: true })
  );

  const viewState$ = state$.pipe(
    map((state) => {
      const hasCollapsed = state.nodes.some(
        (n) =>
          (n.type === 'SubgraphNode' || n.type === 'MapNode') &&
          (n.data as any)?.expanded === false
      );
      if (!hasCollapsed) return state;
      const { nodes, edges } = applyCollapsedSubgraphUi(state.nodes, state.edges);
      return { ...state, nodes, edges };
    }),
    shareReplay({ bufferSize: 1, refCount: true })
  );

  const canUndo$ = commandBus.canUndo$.pipe(shareReplay({ bufferSize: 1, refCount: true }));
  const canRedo$ = commandBus.canRedo$.pipe(shareReplay({ bufferSize: 1, refCount: true }));

  const dispatch = (command: GraphCommand) => commandBus.dispatch(command);
  const undo = () => commandBus.undo();
  const redo = () => commandBus.redo();

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
      dispatch({ type: 'UPDATE_NODE_DATA', payload: { id, data } }),
    updateNodePosition: (id, position) =>
      dispatch({ type: 'UPDATE_NODE_POSITION', payload: { id, position } }),
    updateNodeDimensions: (id, width, height) =>
      dispatch({
        type: 'UPDATE_NODE_DIMENSIONS',
        payload: { id, width, height },
      }),
    selectNode: (id) => dispatch({ type: 'SELECT_NODE', payload: { id } }),
    onConnect: (connection) =>
      dispatch({ type: 'ON_CONNECT', payload: connection }),
    addEdge: (edge) => dispatch({ type: 'ADD_EDGE', payload: { edge } }),
    removeEdge: (id) => dispatch({ type: 'REMOVE_EDGE', payload: { id } }),
    setNodes: (nodes) => dispatch({ type: 'SET_NODES', payload: { nodes } }),
    setEdges: (edges) => dispatch({ type: 'SET_EDGES', payload: { edges } }),
    pasteNodes: (nodes, edges) =>
      dispatch({ type: 'PASTE_NODES', payload: { nodes, edges } }),
    groupNodes: (nodeIds) =>
      dispatch({ type: 'GROUP_NODES', payload: { nodeIds } }),
    toggleSubgraph: (nodeId) =>
      dispatch({ type: 'TOGGLE_SUBGRAPH', payload: { nodeId } }),
  };

  return {
    commandBus,
    state$,
    viewState$,
    canUndo$,
    canRedo$,
    dispatch,
    undo,
    redo,
    dispatchers,
    getSnapshot: () => commandBus.currentState,
    destroy: () => commandBus.destroy(),
  };
}
