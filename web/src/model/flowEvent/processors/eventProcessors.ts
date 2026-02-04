/**
 * FlowEvent 处理器函数
 * 处理各种 FlowEvent 类型，更新 FlowEventState
 */

import { produce } from 'immer';
import type { Immutable } from 'immer';
import type { FlowEventState } from '../flowEventModel';
import type {
  ProcessFlowEventCommand,
  ProcessNodeMsgEventCommand,
  ProcessNodeStatusEventCommand,
  ProcessControlEventCommand,
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
  ProcessWebSocketMessageCommand,
} from '../commands';
import type {
  FlowControlEvent,
  FlowNodeMsgEvent,
  FlowNodeStatusEvent,
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

// ===== 事件处理器 =====

/**
 * 处理节点消息事件 (FlowNodeMsgEvent)
 * 包括: NodeError, NodeInMessage, NodeOutMessage, NodeStreamNextMessage
 */
export const processNodeMsgEvent = (
  state: Immutable<FlowEventState>,
  command: ProcessNodeMsgEventCommand
): Immutable<FlowEventState> => {
  const { event, runId } = command.payload;

  // 检查是否是当前 run 的事件
  if (!isCurrentRunEvent(state as FlowEventState, runId, state.activeRunContext)) {
    return state;
  }

  return produce(state, (draft) => {
    const nodeData = getOrCreateNodeData(draft as FlowEventState, event.nodeId);

    switch (event.type) {
      case 'NodeError':
        nodeData.status = 'error';
        nodeData.errorMessage = event.msg;
        nodeData.isOutputStream = false;
        break;

      case 'NodeInMessage':
        nodeData.inputs = typeof event.msg === 'object' && event.msg !== null
          ? event.msg
          : { value: event.msg };
        break;

      case 'NodeOutMessage':
        nodeData.outputs = { output: event.msg };
        nodeData.lastMessage = event.msg;
        nodeData.isOutputStream = false;
        break;

      case 'NodeStreamNextMessage':
        // 流式消息，追加到 lastMessage
        if (!nodeData.lastMessage) {
          nodeData.lastMessage = event.msg;
        } else if (typeof nodeData.lastMessage === 'string' && typeof event.msg === 'string') {
          nodeData.lastMessage += event.msg;
        } else {
          // 如果不是字符串，用数组存储
          if (!Array.isArray(nodeData.lastMessage)) {
            nodeData.lastMessage = [nodeData.lastMessage];
          }
          (nodeData.lastMessage as any[]).push(event.msg);
        }
        break;
    }

    // 更新 pendingNodeUpdates
    if (Object.keys(nodeData).length > 0) {
      draft.pendingNodeUpdates.set(event.nodeId, nodeData);
    }
  });
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

/**
 * 处理流程控制事件 (FlowControlEvent)
 * 包括: FlowPaused, FlowResumed, FlowStopped, FlowFinished
 */
export const processControlEvent = (
  state: Immutable<FlowEventState>,
  command: ProcessControlEventCommand
): Immutable<FlowEventState> => {
  const { event } = command.payload;

  return produce(state, (draft) => {
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
  });
};

// ===== 向后兼容的通用处理器 =====

/**
 * 通用 FlowEvent 处理器（向后兼容）
 * 根据事件类型分发到对应的处理器
 */
export const processFlowEvent = (
  state: Immutable<FlowEventState>,
  command: ProcessFlowEventCommand
): Immutable<FlowEventState> => {
  const { event, runId } = command.payload;

  // 检查是否是当前 run 的事件
  if (!isCurrentRunEvent(state as FlowEventState, runId, state.activeRunContext)) {
    return state;
  }

  // 根据事件类型分发到对应的处理器
  if ('nodeId' in event) {
    // 节点相关事件
    const eventType = event.type;

    // FlowNodeMsgEvent: NodeError, NodeInMessage, NodeOutMessage, NodeStreamNextMessage
    if (
      eventType === 'NodeError' ||
      eventType === 'NodeInMessage' ||
      eventType === 'NodeOutMessage' ||
      eventType === 'NodeStreamNextMessage'
    ) {
      return processNodeMsgEvent(state, {
        type: 'PROCESS_NODE_MSG_EVENT',
        payload: { event: event as FlowNodeMsgEvent, runId },
      });
    }

    // FlowNodeStatusEvent: NodeStarted, NodeStreamStarted, NodeCompleted
    if (
      eventType === 'NodeStarted' ||
      eventType === 'NodeStreamStarted' ||
      eventType === 'NodeCompleted'
    ) {
      return processNodeStatusEvent(state, {
        type: 'PROCESS_NODE_STATUS_EVENT',
        payload: { event: event as FlowNodeStatusEvent, runId },
      });
    }
  }

  // FlowControlEvent: FlowPaused, FlowResumed, FlowStopped, FlowFinished
  if ('runId' in event && !('nodeId' in event)) {
    return processControlEvent(state, {
      type: 'PROCESS_CONTROL_EVENT',
      payload: { event: event as FlowControlEvent },
    });
  }

  return state;
};

/**
 * WebSocket 消息处理器
 */
export const processWebSocketMessage = (
  state: Immutable<FlowEventState>,
  command: ProcessWebSocketMessageCommand
): Immutable<FlowEventState> => {
  const { message } = command.payload;
  const flowEventCommand: ProcessFlowEventCommand = {
    type: 'PROCESS_FLOW_EVENT',
    payload: {
      event: message.event,
      runId: message.runId,
    },
  };
  return processFlowEvent(state, flowEventCommand);
};

// ===== 辅助函数 =====

/**
 * 处理 RunNode 完成
 */
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