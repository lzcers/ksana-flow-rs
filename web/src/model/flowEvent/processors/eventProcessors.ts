/**
 * FlowEvent 处理器函数
 * 处理各种 FlowEvent 类型，更新 FlowEventState
 */

import { produce } from 'immer';
import type { Immutable } from 'immer';
import type { FlowEventState } from '../flowEventModel';
import type {
  ProcessFlowEventCommand,
  SetCurrentRunCommand,
  UpdateWorkflowStatusCommand,
  MapRunToWorkflowCommand,
  UnmapRunCommand,
  UpdateNodeExecutionDataCommand,
  BatchUpdateNodeDataCommand,
  ClearPendingUpdatesCommand,
  SetActiveRunContextCommand,
  ClearActiveRunContextCommand,
  ResetFlowEventStateCommand,
} from '../commands';
import type {
  FlowControlEvent,
} from '../types';

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

// ===== 处理器函数 =====

export const processFlowEvent = (
  state: Immutable<FlowEventState>,
  command: ProcessFlowEventCommand
): Immutable<FlowEventState> => {
  const { event, runId } = command.payload;

  // 检查是否是当前 run 的事件
  if (!isCurrentRunEvent(state as FlowEventState, runId, state.activeRunContext)) {
    return state;
  }

  return produce(state, (draft) => {
    // 根据事件类型处理
    if ('nodeId' in event) {
      // 节点相关事件
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
    }

    // 处理控制事件
    if ('runId' in event && !('nodeId' in event)) {
      handleControlEvent(draft, event as FlowControlEvent);
    }
  });
};

// 辅助函数：处理 RunNode 完成
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

// 辅助函数：处理控制事件
const handleControlEvent = (draft: FlowEventState, event: FlowControlEvent): void => {
  const { type, runId } = event;
  const workflowId = draft.runIdToWorkflowId[runId];

  switch (type) {
    case 'FlowFinished':
    case 'FlowStopped':
      if (workflowId != null) {
        draft.workflowStatuses[workflowId] = 'idle';
      }
      if (runId === draft.currentRunId) {
        draft.workflowStatus = 'idle';
        draft.currentRunId = null;
      }
      delete draft.runIdToWorkflowId[runId];
      break;

    case 'FlowPaused':
      if (workflowId != null) {
        draft.workflowStatuses[workflowId] = 'paused';
      }
      if (runId === draft.currentRunId) {
        draft.workflowStatus = 'paused';
      }
      break;

    case 'FlowResumed':
      if (workflowId != null) {
        draft.workflowStatuses[workflowId] = 'running';
      }
      if (runId === draft.currentRunId) {
        draft.workflowStatus = 'running';
      }
      break;
  }
};

// ===== Run Management Processors =====

export const processSetCurrentRun = (
  state: Immutable<FlowEventState>,
  command: SetCurrentRunCommand
): Immutable<FlowEventState> => {
  const { runId, workflowId } = command.payload;

  return produce(state, (draft) => {
    draft.currentRunId = runId;
    draft.currentWorkflowId = workflowId;

    if (runId && workflowId != null) {
      draft.runIdToWorkflowId[runId] = workflowId;
      draft.workflowStatuses[workflowId] = 'running';
      draft.workflowStatus = 'running';
    }
  });
};

export const processUpdateWorkflowStatus = (
  state: Immutable<FlowEventState>,
  command: UpdateWorkflowStatusCommand
): Immutable<FlowEventState> => {
  const { workflowId, status } = command.payload;

  return produce(state, (draft) => {
    draft.workflowStatuses[workflowId] = status;

    // 如果是当前 workflow，同步更新 workflowStatus
    if (workflowId === draft.currentWorkflowId) {
      draft.workflowStatus = status;
    }
  });
};

export const processMapRunToWorkflow = (
  state: Immutable<FlowEventState>,
  command: MapRunToWorkflowCommand
): Immutable<FlowEventState> => {
  const { runId, workflowId } = command.payload;

  return produce(state, (draft) => {
    draft.runIdToWorkflowId[runId] = workflowId;
  });
};

export const processUnmapRun = (
  state: Immutable<FlowEventState>,
  command: UnmapRunCommand
): Immutable<FlowEventState> => {
  const { runId } = command.payload;

  return produce(state, (draft) => {
    delete draft.runIdToWorkflowId[runId];

    // 如果是 currentRunId，清空它
    if (draft.currentRunId === runId) {
      draft.currentRunId = null;
    }
  });
};

// ===== Node Update Processors =====

export const processUpdateNodeExecutionData = (
  state: Immutable<FlowEventState>,
  command: UpdateNodeExecutionDataCommand
): Immutable<FlowEventState> => {
  const { nodeId, data } = command.payload;

  return produce(state, (draft) => {
    const existing = draft.pendingNodeUpdates.get(nodeId) ?? {};
    draft.pendingNodeUpdates.set(nodeId, { ...existing, ...data });
  });
};

export const processBatchUpdateNodeData = (
  state: Immutable<FlowEventState>,
  command: BatchUpdateNodeDataCommand
): Immutable<FlowEventState> => {
  const { updates } = command.payload;

  return produce(state, (draft) => {
    updates.forEach(({ nodeId, data }) => {
      const existing = draft.pendingNodeUpdates.get(nodeId) ?? {};
      draft.pendingNodeUpdates.set(nodeId, { ...existing, ...data });
    });
  });
};

export const processClearPendingUpdates = (
  state: Immutable<FlowEventState>,
  _command: ClearPendingUpdatesCommand
): Immutable<FlowEventState> => {
  return produce(state, (draft) => {
    draft.pendingNodeUpdates.clear();
  });
};

// ===== Run Node Execution Processors =====

export const processSetActiveRunContext = (
  state: Immutable<FlowEventState>,
  command: SetActiveRunContextCommand
): Immutable<FlowEventState> => {
  const payload = command.payload;

  return produce(state, (draft) => {
    draft.activeRunContext = payload;
  });
};

export const processClearActiveRunContext = (
  state: Immutable<FlowEventState>,
  _command: ClearActiveRunContextCommand
): Immutable<FlowEventState> => {
  return produce(state, (draft) => {
    draft.activeRunContext = null;
  });
};

// ===== Meta Processors =====

export const processResetFlowEventState = (
  _state: Immutable<FlowEventState>,
  _command: ResetFlowEventStateCommand
): Immutable<FlowEventState> => {
  return {
    currentRunId: null,
    currentWorkflowId: null,
    workflowStatus: 'idle',
    workflowStatuses: {},
    runIdToWorkflowId: {},
    pendingNodeUpdates: new Map(),
    activeRunContext: null,
  };
};
