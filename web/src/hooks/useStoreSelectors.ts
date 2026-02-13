import { useCallback } from 'react';
import { useStore } from '../store';
import type { StoreState } from '../store/types';
import type { Node, Edge } from '../model/workflow/types';
import type { NodeMetadata } from '../api';

// ==========================================
// Canvas 状态选择器
// ==========================================

/** 只订阅节点数组（最常用） */
export function useNodes(): Node[] {
  return useStore(
    useCallback((state: StoreState) => {
      return state.nodes;
    }, [])
  );
}

/** 只订阅边数组 */
export function useEdges(): Edge[] {
  return useStore(
    useCallback((state: StoreState) => {
      return state.edges;
    }, [])
  );
}

/** 只订阅选中节点ID */
export function useSelectedNodeId(): string[] {
  return useStore(
    useCallback((state: StoreState) => {
      return state.selectedNodeId;
    }, [])
  );
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

/** 画布操作方法 - 直接从 store 获取，不通过选择器订阅
 *  这些 actions 引用稳定，不会导致重渲染
 */
export const canvasActions = {
  get setNodes() { return useStore.getState().setNodes; },
  get setEdges() { return useStore.getState().setEdges; },
  get addNode() { return useStore.getState().addNode; },
  get deleteNode() { return useStore.getState().deleteNode; },
  get updateNodeData() { return useStore.getState().updateNodeData; },
  get updateNodeDimensions() { return useStore.getState().updateNodeDimensions; },
  get selectNode() { return useStore.getState().selectNode; },
  get onNodesChange() { return useStore.getState().onNodesChange; },
  get onEdgesChange() { return useStore.getState().onEdgesChange; },
  get onNodeDrag() { return useStore.getState().onNodeDrag; },
  get onNodeDragStop() { return useStore.getState().onNodeDragStop; },
  get onConnect() { return useStore.getState().onConnect; },
  get pasteNodes() { return useStore.getState().pasteNodes; },
  get groupNodes() { return useStore.getState().groupNodes; },
  get toggleSubgraph() { return useStore.getState().toggleSubgraph; },
  get setConnectionState() { return useStore.getState().setConnectionState; },
};

/** 历史操作方法 - 直接从 store 获取 */
export const historyActions = {
  get undo() { return useStore.getState().undo; },
  get redo() { return useStore.getState().redo; },
};


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

export function useActiveGraphKey(): string | null {
  return useStore(useCallback((state: StoreState) => state.activeGraphKey, []));
}

/** 订阅当前空间ID */
export function useCurrentSpaceId(): string | null {
  return useStore(useCallback((state: StoreState) => state.currentSpaceId, []));
}

/** 订阅节点类型（元数据） */
export function useNodeTypes(): NodeMetadata[] {
  return useStore(useCallback((state: StoreState) => state.nodeTypes, []));
}

/** 工作流操作方法 - 直接从 store 获取 */
export const workflowActions = {
  get loadWorkflow() { return useStore.getState().loadWorkflow; },
  get saveWorkflow() { return useStore.getState().saveWorkflow; },
  get renameWorkflow() { return useStore.getState().renameWorkflow; },
  get deleteWorkflow() { return useStore.getState().deleteWorkflow; },
  get createNewWorkflow() { return useStore.getState().createNewWorkflow; },
  get importWorkflow() { return useStore.getState().importWorkflow; },
  get getWorkflowBlueprint() { return useStore.getState().getWorkflowBlueprint; },
  get loadMetadata() { return useStore.getState().loadMetadata; },
  get setSpaceId() { return useStore.getState().setSpaceId; },
};

// ==========================================
// Execution 状态选择器
// ==========================================

/** 订阅工作流执行状态 */
export function useWorkflowStatus(): import('../store/types').WorkflowStatus {
  return useStore(useCallback((state: StoreState) => state.currentWorkflowStatus, []));
}

export function useWorkflowStatuses(): Record<number, import('../store/types').WorkflowStatus> {
  return useStore(useCallback((state: StoreState) => state.workflowStatuses, []));
}

/** 订阅当前运行ID */
export function useCurrentRunId(): string | null {
  return useStore(useCallback((state: StoreState) => state.currentRunId, []));
}

/** 执行操作方法 - 直接从 store 获取 */
export const executionActions = {
  get runWorkflow() { return useStore.getState().runWorkflow; },
  get pauseWorkflow() { return useStore.getState().pauseWorkflow; },
  get resumeWorkflow() { return useStore.getState().resumeWorkflow; },
  get stopWorkflow() { return useStore.getState().stopWorkflow; },
  get runNode() { return useStore.getState().runNode; },
};

// ==========================================
// Toast 状态选择器
// ==========================================

/** Toast 操作方法 - 直接从 store 获取 */
export const toastActions = {
  get success() { return useStore.getState().success; },
  get error() { return useStore.getState().error; },
  get info() { return useStore.getState().info; },
  get removeToast() { return useStore.getState().removeToast; },
};

// 导出 useStore 以便直接使用
export { useStore };
