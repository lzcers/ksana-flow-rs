import type { Observable } from 'rxjs';
import { RxWorkflow, type RxWorkflowOptions } from './rx/RxWorkflow';
import { registerAllHandlers } from './commandHandlers';
import type { WorkflowState } from './types';
import type { GraphCommand } from './commands';
import type { XYPosition, Connection } from '@xyflow/react';
import type { Edge } from './types';

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

export function createWorkflowModel(
  options: RxWorkflowOptions = {}
): WorkflowModelInterface {
  const rxWorkflow = new RxWorkflow(options);

  // 注册所有处理器到 RxWorkflow (实际上是注册到 Core WorkflowModel)
  // 注意：registerAllHandlers 需要适配新的接口
  // 我们需要一个适配器，因为 registerAllHandlers 期望的是 RxCommandBus
  // 但我们可以直接修改 commandHandlers.ts 或者在这里适配

  // 更好的方式是让 registerAllHandlers 接受一个通用接口
  // 暂时我们这里做一个简单的鸭子类型适配，或者直接让 commandHandlers 导出处理器映射

  // 让我们修改 registerAllHandlers 签名可能会更好，但为了最小改动：
  // 我们可以在 RxWorkflow 中实现 registerHandler 方法
  registerAllHandlers(rxWorkflow as any); // 需要确保 RxWorkflow 有 registerHandler 方法

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
