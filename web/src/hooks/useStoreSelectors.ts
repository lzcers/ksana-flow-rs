import { useCallback } from 'react';
import { useStore } from '../store';
import type { StoreState } from '../store/types';
import type { Node, Edge } from '@xyflow/react';
import type { NodeMetadata } from '../api';

// ==========================================
// Canvas 状态选择器
// ==========================================

/** 只订阅节点数组（最常用） */
export function useNodes(): Node[] {
  return useStore(useCallback((state: StoreState) => state.nodes, []));
}

/** 只订阅边数组 */
export function useEdges(): Edge[] {
  return useStore(useCallback((state: StoreState) => state.edges, []));
}

/** 只订阅选中节点ID */
export function useSelectedNodeId(): string | null {
  return useStore(useCallback((state: StoreState) => state.selectedNodeId, []));
}

/** 订阅连接状态 */
export function useConnectionState(): {
  isConnecting: boolean;
  connectionSourceId: string | null;
} {
  return useStore(
    useCallback(
      (state: StoreState) => ({
        isConnecting: state.isConnecting,
        connectionSourceId: state.connectionSourceId,
      }),
      []
    )
  );
}

/** 订阅画布操作方法（稳定引用） */
export function useCanvasActions() {
  return useStore(
    useCallback(
      (state: StoreState) => ({
        setNodes: state.setNodes,
        setEdges: state.setEdges,
        addNode: state.addNode,
        deleteNode: state.deleteNode,
        updateNodeData: state.updateNodeData,
        updateNodeDimensions: state.updateNodeDimensions,
        selectNode: state.selectNode,
        onNodesChange: state.onNodesChange,
        onEdgesChange: state.onEdgesChange,
        onConnect: state.onConnect,
        pasteNodes: state.pasteNodes,
        groupNodes: state.groupNodes,
        toggleSubgraph: state.toggleSubgraph,
        setConnectionState: state.setConnectionState,
      }),
      []
    )
  );
}

/** 订阅历史操作方法 */
export function useHistoryActions() {
  return useStore(
    useCallback(
      (state: StoreState) => ({
        pushHistory: state.pushHistory,
        undo: state.undo,
        redo: state.redo,
        canUndo: state.canUndo,
        canRedo: state.canRedo,
      }),
      []
    )
  );
}

/** 订阅历史状态 - 使用独立选择器避免对象比较问题 */
export function useCanUndo(): boolean {
  return useStore(useCallback((state: StoreState) => state.history.past.length > 0, []));
}

export function useCanRedo(): boolean {
  return useStore(useCallback((state: StoreState) => state.history.future.length > 0, []));
}

// ==========================================
// Workflow 状态选择器
// ==========================================

/** 订阅工作流列表 */
export function useWorkflows(): { id: number; name: string }[] {
  return useStore(useCallback((state: StoreState) => state.workflows, []));
}

/** 订阅当前工作流ID */
export function useCurrentWorkflowId(): number | null {
  return useStore(useCallback((state: StoreState) => state.currentWorkflowId, []));
}

/** 订阅当前空间ID */
export function useCurrentSpaceId(): string | null {
  return useStore(useCallback((state: StoreState) => state.currentSpaceId, []));
}

/** 订阅节点类型（元数据） */
export function useNodeTypes(): NodeMetadata[] {
  return useStore(useCallback((state: StoreState) => state.nodeTypes, []));
}

/** 订阅工作流操作方法 */
export function useWorkflowActions() {
  return useStore(
    useCallback(
      (state: StoreState) => ({
        loadWorkflow: state.loadWorkflow,
        saveWorkflow: state.saveWorkflow,
        renameWorkflow: state.renameWorkflow,
        deleteWorkflow: state.deleteWorkflow,
        createNewWorkflow: state.createNewWorkflow,
        importWorkflow: state.importWorkflow,
        loadMetadata: state.loadMetadata,
        setSpaceId: state.setSpaceId,
      }),
      []
    )
  );
}

// ==========================================
// Execution 状态选择器
// ==========================================

/** 订阅工作流执行状态 */
export function useWorkflowStatus(): import('../store/types').WorkflowStatus {
  return useStore(useCallback((state: StoreState) => state.workflowStatus, []));
}

/** 订阅当前运行ID */
export function useCurrentRunId(): string | null {
  return useStore(useCallback((state: StoreState) => state.currentRunId, []));
}

/** 订阅执行操作方法 */
export function useExecutionActions() {
  return useStore(
    useCallback(
      (state: StoreState) => ({
        runWorkflow: state.runWorkflow,
        pauseWorkflow: state.pauseWorkflow,
        resumeWorkflow: state.resumeWorkflow,
        stopWorkflow: state.stopWorkflow,
        runNode: state.runNode,
      }),
      []
    )
  );
}

// ==========================================
// Toast 状态选择器
// ==========================================

/** 订阅 Toast 操作方法 */
export function useToastActions() {
  return useStore(
    useCallback(
      (state: StoreState) => ({
        success: state.success,
        error: state.error,
        info: state.info,
        removeToast: state.removeToast,
      }),
      []
    )
  );
}
