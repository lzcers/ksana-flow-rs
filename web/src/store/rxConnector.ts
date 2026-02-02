/**
 * RxJS 与 Zustand 的连接器
 * 负责将 RxCommandBus 的状态同步到 Zustand Store
 */

import type { StoreApi } from 'zustand';
import type { RxCommandBus } from '@/model/rx';
import type { WorkflowState } from '@/model/types';
import type { GraphCommand } from '@/model/commands';

export interface RxConnectorConfig {
  commandBus: RxCommandBus;
  onStateChange?: (state: WorkflowState) => void;
  syncThrottleMs?: number;
}

export interface CommandDispatchers {
  addNode: (type: string, position: { x: number; y: number }) => void;
  removeNode: (id: string) => void;
  updateNodeData: (id: string, data: Record<string, any>) => void;
  updateNodePosition: (id: string, position: { x: number; y: number }) => void;
  updateNodeDimensions: (id: string, width: number, height: number) => void;
  selectNode: (id: string | null) => void;
  onConnect: (connection: { source: string; target: string; sourceHandle?: string; targetHandle?: string }) => void;
  addEdge: (edge: any) => void;
  removeEdge: (id: string) => void;
  setNodes: (nodes: any[]) => void;
  setEdges: (edges: any[]) => void;
  pasteNodes: (nodes: any[], edges: any[]) => void;
  groupNodes: (nodeIds: string[]) => void;
  toggleSubgraph: (nodeId: string) => void;
}

/**
 * 连接 RxJS CommandBus 到 Zustand Store
 */
export function connectRxToZustand<T extends { workflow: WorkflowState }>(
  storeApi: StoreApi<T>,
  config: RxConnectorConfig
): () => void {
  const { commandBus, onStateChange } = config;

  // 订阅 RxJS State 变化，同步到 Zustand
  const subscription = commandBus.state$.subscribe((workflowState) => {
    // 同步到 Zustand
    storeApi.setState({ workflow: workflowState } as Partial<T>);

    // 可选的回调
    onStateChange?.(workflowState);
  });

  // 返回取消订阅函数
  return () => {
    subscription.unsubscribe();
  };
}

/**
 * 创建 Command Dispatchers
 * 将用户交互转换为 Command 分发到 RxBus
 */
export function createCommandDispatchers(
  commandBus: RxCommandBus
): CommandDispatchers {
  return {
    // ===== Node Commands =====
    addNode: (type, position) => {
      commandBus.dispatch({
        type: 'ADD_NODE',
        payload: { nodeType: type, position },
      });
    },

    removeNode: (id) => {
      commandBus.dispatch({
        type: 'REMOVE_NODE',
        payload: { id },
      });
    },

    updateNodeData: (id, data) => {
      commandBus.dispatch({
        type: 'UPDATE_NODE_DATA',
        payload: { id, data },
      });
    },

    updateNodePosition: (id, position) => {
      commandBus.dispatch({
        type: 'UPDATE_NODE_POSITION',
        payload: { id, position },
      });
    },

    updateNodeDimensions: (id, width, height) => {
      commandBus.dispatch({
        type: 'UPDATE_NODE_DIMENSIONS',
        payload: { id, width, height },
      });
    },

    selectNode: (id) => {
      commandBus.dispatch({
        type: 'SELECT_NODE',
        payload: { id },
      });
    },

    // ===== Edge Commands =====
    onConnect: (connection) => {
      commandBus.dispatch({
        type: 'ON_CONNECT',
        payload: connection,
      });
    },

    addEdge: (edge) => {
      commandBus.dispatch({
        type: 'ADD_EDGE',
        payload: { edge },
      });
    },

    removeEdge: (id) => {
      commandBus.dispatch({
        type: 'REMOVE_EDGE',
        payload: { id },
      });
    },

    // ===== Graph Commands =====
    setNodes: (nodes) => {
      commandBus.dispatch({
        type: 'SET_NODES',
        payload: { nodes },
      });
    },

    setEdges: (edges) => {
      commandBus.dispatch({
        type: 'SET_EDGES',
        payload: { edges },
      });
    },

    pasteNodes: (nodes, edges) => {
      commandBus.dispatch({
        type: 'PASTE_NODES',
        payload: { nodes, edges },
      });
    },

    groupNodes: (nodeIds) => {
      commandBus.dispatch({
        type: 'GROUP_NODES',
        payload: { nodeIds },
      });
    },

    toggleSubgraph: (nodeId) => {
      commandBus.dispatch({
        type: 'TOGGLE_SUBGRAPH',
        payload: { nodeId },
      });
    },
  };
}
