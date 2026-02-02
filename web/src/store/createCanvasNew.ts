/**
 * createCanvasNew - 重构后的 Canvas Store
 * 使用 RxCommandBus 处理 Graph 操作，Zustand 仅维护 UI 状态
 */

import type { StateCreator } from 'zustand';
import type { NodeChange, EdgeChange, Connection } from '@xyflow/react';
import type { StoreState, Canvas } from './types';
import type { Node, Edge } from '@/model/types';
import type { RxCommandBus } from '@/model/rx';
import { createCommandDispatchers, type CommandDispatchers } from './rxConnector';

// 历史记录类型
interface HistoryState {
  nodes: Node[];
  edges: Edge[];
}

export interface CanvasNew extends Canvas {
  // RxJS 相关
  _commandBus?: RxCommandBus;
  _dispatchers?: CommandDispatchers;
  _unsubscribeRx?: () => void;

  // 初始化方法
  initializeCommandBus: (commandBus: RxCommandBus) => void;
  destroyCommandBus: () => void;
}

export const createCanvasNew: StateCreator<StoreState, [], [], CanvasNew> = (set, get) => ({
  // ===== 从 RxCommandBus 同步的状态 =====
  nodes: [],
  edges: [],
  selectedNodeId: null,

  // ===== UI 状态（仅 Canvas 层维护） =====
  isConnecting: false,
  connectionSourceId: null,
  history: { past: [], future: [] },

  // ===== RxJS 引用 =====
  _commandBus: undefined,
  _dispatchers: undefined,
  _unsubscribeRx: undefined,

  // ===== 初始化方法 =====
  initializeCommandBus: (commandBus: RxCommandBus) => {
    const { _unsubscribeRx } = get();

    // 清理旧的订阅
    if (_unsubscribeRx) {
      _unsubscribeRx();
    }

    // 创建 dispatchers
    const dispatchers = createCommandDispatchers(commandBus);

    // 订阅 RxJS State 变化，同步到 Zustand
    const unsubscribe = commandBus.state$.subscribe((state) => {
      set({
        nodes: state.nodes,
        edges: state.edges,
        selectedNodeId: state.selectedNodeId,
      });
    });

    set({
      _commandBus: commandBus,
      _dispatchers: dispatchers,
      _unsubscribeRx: unsubscribe,
    });

    console.log('[createCanvasNew] CommandBus initialized');
  },

  destroyCommandBus: () => {
    const { _unsubscribeRx, _commandBus } = get();

    if (_unsubscribeRx) {
      _unsubscribeRx();
    }

    if (_commandBus) {
      _commandBus.destroy();
    }

    set({
      _commandBus: undefined,
      _dispatchers: undefined,
      _unsubscribeRx: undefined,
    });

    console.log('[createCanvasNew] CommandBus destroyed');
  },

  // ===== 委托给 RxCommandBus 的方法 =====
  addNode: (type, position) => {
    get().pushHistory();
    get()._dispatchers?.addNode(type, position);
  },

  removeNode: (id) => {
    get().pushHistory();
    get()._dispatchers?.removeNode(id);
  },

  updateNodeData: (id, data) => {
    get()._dispatchers?.updateNodeData(id, data);
  },

  updateNodeDimensions: (id, width, height) => {
    get()._dispatchers?.updateNodeDimensions(id, width, height);
  },

  selectNode: (id) => {
    get()._dispatchers?.selectNode(id);
  },

  onConnect: (connection) => {
    get().pushHistory();
    get()._dispatchers?.onConnect(connection);
  },

  setNodes: (nodes) => {
    get()._dispatchers?.setNodes(nodes);
  },

  setEdges: (edges) => {
    get()._dispatchers?.setEdges(edges);
  },

  pasteNodes: (nodes, edges) => {
    get().pushHistory();
    get()._dispatchers?.pasteNodes(nodes, edges);
  },

  groupNodes: (nodeIds) => {
    get().pushHistory();
    get()._dispatchers?.groupNodes(nodeIds);
  },

  toggleSubgraph: (nodeId) => {
    get()._dispatchers?.toggleSubgraph(nodeId);
  },

  // ===== ReactFlow 事件处理 =====
  onNodesChange: (changes) => {
    // 这里可以处理 ReactFlow 的节点变更事件
    // 例如：位置更新、选中状态变化等
    // 暂时保持原有行为
  },

  onEdgesChange: (changes) => {
    // 处理 ReactFlow 的边变更事件
  },

  onNodeDragStop: (_, node) => {
    // 处理节点拖拽停止事件
    get()._dispatchers?.updateNodePosition(node.id, node.position);
  },

  // ===== UI 状态方法 =====
  setConnectionState: (isConnecting, sourceId) => {
    set({
      isConnecting,
      connectionSourceId: sourceId,
    });
  },

  // ===== 历史记录方法 =====
  pushHistory: () => {
    const { nodes, edges, history } = get();

    // 限制历史记录为 50 步
    const newPast = [...history.past, { nodes, edges }].slice(-50);
    set({ history: { past: newPast, future: [] } });
  },

  undo: () => {
    const { history } = get();
    if (history.past.length === 0) return;

    const previous = history.past[history.past.length - 1];
    const newPast = history.past.slice(0, -1);
    const { nodes, edges } = get();

    // 使用 setNodes 和 setEdges 更新状态
    get().setNodes(previous.nodes);
    get().setEdges(previous.edges);

    set({
      history: {
        past: newPast,
        future: [{ nodes, edges }, ...history.future],
      },
    });
  },

  redo: () => {
    const { history } = get();
    if (history.future.length === 0) return;

    const next = history.future[0];
    const newFuture = history.future.slice(1);
    const { nodes, edges } = get();

    // 使用 setNodes 和 setEdges 更新状态
    get().setNodes(next.nodes);
    get().setEdges(next.edges);

    set({
      history: {
        past: [...history.past, { nodes, edges }],
        future: newFuture,
      },
    });
  },

  canUndo: () => {
    return get().history.past.length > 0;
  },

  canRedo: () => {
    return get().history.future.length > 0;
  },
});
