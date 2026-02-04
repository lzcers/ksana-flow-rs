/**
 * 节点状态事件处理器
 * 处理: NodeStarted, NodeStreamStarted, NodeCompleted
 */

import { produce } from 'immer';
import type { Immutable } from 'immer';
import type { FlowEventState } from '../flowEventModel';
import type { ProcessNodeStatusEventCommand } from '../commands';

// ===== 辅助函数 =====

const isCurrentRunEvent = (
  state: FlowEventState,
  runId: string | undefined,
  activeContext: typeof state.activeRunContext
): boolean => {
  if (!runId) return true;
  if (runId === state.currentRunId) return true;
  if (activeContext && runId === activeContext.runId) return true;
  return false;
};

const getOrCreateNodeData = (
  state: FlowEventState,
  nodeId: string
): FlowEventState['pendingNodeUpdates'] extends Map<string, infer V> ? V : never => {
  const existing = state.pendingNodeUpdates.get(nodeId);
  if (existing) return existing as never;
  return {} as never;
};

// ===== RunNode 完成处理 =====

const handleRunNodeCompletion = (draft: FlowEventState, runId: string): void => {
  // 清理 activeRunContext
  draft.activeRunContext = null;

  // 更新 workflowStatus
  draft.workflowStatus = 'idle';

  // 清理 runId 映射
  const workflowId = draft.runIdToWorkflowId[runId];
  if (workflowId != null) {
    draft.workflowStatuses[workflowId] = 'idle';
    delete draft.runIdToWorkflowId[runId];
  }

  // 重置 currentRunId
  if (draft.currentRunId === runId) {
    draft.currentRunId = null;
  }
};

/**
 * 处理节点状态事件 (FlowNodeStatusEvent)
 * 包括: NodeStarted, NodeStreamStarted, NodeCompleted
 */
export const processNodeStatusEvent = (
  state: Immutable<FlowEventState>,
  command: ProcessNodeStatusEventCommand
): Immutable<FlowEventState> => {
  const { event, runId } = command.payload;

  // 检查是否是当前 run 的事件
  if (!isCurrentRunEvent(state as FlowEventState, runId, state.activeRunContext)) {
    return state;
  }

  return produce(state, (draft) => {
    const nodeData = getOrCreateNodeData(draft as FlowEventState, event.nodeId);

    switch (event.type) {
      case 'NodeStarted':
        nodeData.status = 'running';
        break;

      case 'NodeStreamStarted':
        nodeData.isOutputStream = true;
        break;

      case 'NodeCompleted':
        nodeData.status = 'completed';
        nodeData.isOutputStream = false;

        // 检查是否是 RunNode 执行完成
        if (
          draft.activeRunContext &&
          draft.activeRunContext.runId === runId &&
          draft.activeRunContext.startNodeId === event.nodeId
        ) {
          // 处理 RunNode 完成逻辑
          handleRunNodeCompletion(draft, runId!);
        }
        break;
    }

    // 更新 pendingNodeUpdates
    if (Object.keys(nodeData).length > 0) {
      draft.pendingNodeUpdates.set(event.nodeId, nodeData);
    }
  });
};
