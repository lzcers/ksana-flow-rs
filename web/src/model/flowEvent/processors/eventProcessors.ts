/**
 * FlowEvent 处理器入口
 * 从各个子模块重新导出所有处理器
 */

// ===== 事件处理器 =====
export {
  processNodeMsgEvent,
} from './nodeMsgEventProcessor';

export {
  processNodeStatusEvent,
} from './nodeStatusEventProcessor';

export {
  processControlEvent,
} from './controlEventProcessor';

// ===== 向后兼容的通用处理器 =====

import type { Immutable } from 'immer';
import type { FlowEventState } from '../flowEventModel';
import type {
  ProcessFlowEventCommand,
  ProcessWebSocketMessageCommand,
} from '../commands';
import type {
  FlowControlEvent,
  FlowNodeMsgEvent,
  FlowNodeStatusEvent,
} from '../types';
import { processNodeMsgEvent } from './nodeMsgEventProcessor';
import { processNodeStatusEvent } from './nodeStatusEventProcessor';
import { processControlEvent } from './controlEventProcessor';

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

const isCurrentRunEvent = (
  state: FlowEventState,
  runId: string | undefined,
  activeContext: typeof state.activeRunContext
 : boolean => {
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

// ===== Run Management Processors =====

import type {
  SetCurrentRunCommand,
  UpdateWorkflowStatusCommand,
  MapRunToWorkflowCommand,
  UnmapRunCommand,
} from '../commands';

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

import type {
  UpdateNodeExecutionDataCommand,
  BatchUpdateNodeDataCommand,
  ClearPendingUpdatesCommand,
} from '../commands';

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

import type {
  SetActiveRunContextCommand,
  ClearActiveRunContextCommand,
} from '../commands';

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
 
 mport type { ResetFlowEventStateCommand } from '../commands';

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
