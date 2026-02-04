/**
 * FlowEvent Command 类型定义
 * 所有 FlowEvent 相关操作都定义为 Command，通过 CommandBus 分发
 */

import type { WorkflowStatus } from '../../store/types';
import type { FlowEvent, WebSocketFlowMessage, FlowNodeMsgEvent, FlowNodeStatusEvent, FlowControlEvent } from './types';

export interface BaseCommand {
  meta?: {
    skipHistory?: boolean;
  };
}

// ===== Event Processing Commands =====

export interface ProcessFlowEventCommand extends BaseCommand {
  type: 'PROCESS_FLOW_EVENT';
  payload: {
    event: FlowEvent;
    runId?: string;
  };
}

export interface ProcessNodeMsgEventCommand extends BaseCommand {
  type: 'PROCESS_NODE_MSG_EVENT';
  payload: {
    event: FlowNodeMsgEvent;
    runId?: string;
  };
}

export interface ProcessNodeStatusEventCommand extends BaseCommand {
  type: 'PROCESS_NODE_STATUS_EVENT';
  payload: {
    event: FlowNodeStatusEvent;
    runId?: string;
  };
}

export interface ProcessControlEventCommand extends BaseCommand {
  type: 'PROCESS_CONTROL_EVENT';
  payload: {
    event: FlowControlEvent;
  };
}

export interface ProcessWebSocketMessageCommand extends BaseCommand {
  type: 'PROCESS_WS_MESSAGE';
  payload: {
    message: WebSocketFlowMessage;
  };
}

// ===== Run Management Commands =====

export interface SetCurrentRunCommand extends BaseCommand {
  type: 'SET_CURRENT_RUN';
  payload: {
    runId: string | null;
    workflowId: number | null;
  };
}

export interface UpdateWorkflowStatusCommand extends BaseCommand {
  type: 'UPDATE_WORKFLOW_STATUS';
  payload: {
    workflowId: number;
    status: WorkflowStatus;
  };
}

export interface MapRunToWorkflowCommand extends BaseCommand {
  type: 'MAP_RUN_TO_WORKFLOW';
  payload: {
    runId: string;
    workflowId: number;
  };
}

export interface UnmapRunCommand extends BaseCommand {
  type: 'UNMAP_RUN';
  payload: {
    runId: string;
  };
}

// ===== Node Update Commands =====

export interface UpdateNodeExecutionDataCommand extends BaseCommand {
  type: 'UPDATE_NODE_EXECUTION_DATA';
  payload: {
    nodeId: string;
    data: Partial<NodeExecutionData>;
  };
}

export interface BatchUpdateNodeDataCommand extends BaseCommand {
  type: 'BATCH_UPDATE_NODE_DATA';
  payload: {
    updates: Array<{
      nodeId: string;
      data: Partial<NodeExecutionData>;
    }>;
  };
}

export interface ClearPendingUpdatesCommand extends BaseCommand {
  type: 'CLEAR_PENDING_UPDATES';
  payload: Record<string, never>;
}

// ===== Run Node Execution Commands =====

export interface SetActiveRunContextCommand extends BaseCommand {
  type: 'SET_ACTIVE_RUN_CONTEXT';
  payload: {
    runId: string;
    startNodeId: string;
    workflowId: number | null;
  } | null;
}

export interface ClearActiveRunContextCommand extends BaseCommand {
  type: 'CLEAR_ACTIVE_RUN_CONTEXT';
  payload: Record<string, never>;
}

// ===== Meta Commands =====

export interface ResetFlowEventStateCommand extends BaseCommand {
  type: 'RESET_FLOW_EVENT_STATE';
  payload: Record<string, never>;
}

// ===== Node Execution Data Type =====

export interface NodeExecutionData {
  status?: 'idle' | 'running' | 'completed' | 'error';
  lastMessage?: any;
  lastMessageRunId?: string;
  isOutputStream?: boolean;
  upstreamIsStreaming?: boolean;
  errorMessage?: string;
  inputs?: Record<string, any>;
  outputs?: Record<string, any>;
}

// ===== Union Type =====

export type FlowEventCommand =
  // Event Processing
  | ProcessFlowEventCommand
  | ProcessNodeMsgEventCommand
  | ProcessNodeStatusEventCommand
  | ProcessControlEventCommand
  | ProcessWebSocketMessageCommand
  // Run Management
  | SetCurrentRunCommand
  | UpdateWorkflowStatusCommand
  | MapRunToWorkflowCommand
  | UnmapRunCommand
  // Node Updates
  | UpdateNodeExecutionDataCommand
  | BatchUpdateNodeDataCommand
  | ClearPendingUpdatesCommand
  // Run Node Execution
  | SetActiveRunContextCommand
  | ClearActiveRunContextCommand
  // Meta
  | ResetFlowEventStateCommand;

// Re-export types from types.ts for convenience
export type {
  FlowNodeMsgEvent,
  FlowNodeStatusEvent,
  FlowControlEvent,
  FlowEvent,
  WebSocketFlowMessage,
} from './types';

// ===== Command Factory Functions =====

/**
 * 将 FlowEvent 转换为对应的 FlowEventCommand
 * 纯函数，无副作用
 */
export function flowEventToCommand(event: FlowEvent): FlowEventCommand | null {
  // 节点相关事件
  if ('nodeId' in event) {
    // 节点消息事件
    if (['NodeError', 'NodeInMessage', 'NodeOutMessage', 'NodeStreamNextMessage'].includes(event.type)) {
      return {
        type: 'PROCESS_NODE_MSG_EVENT',
        payload: { event: event as FlowNodeMsgEvent }
      };
    }
    // 节点状态事件
    if (['NodeStarted', 'NodeStreamStarted', 'NodeCompleted'].includes(event.type)) {
      return {
        type: 'PROCESS_NODE_STATUS_EVENT',
        payload: { event: event as FlowNodeStatusEvent }
      };
    }
  }

  // 控制事件
  if ('runId' in event) {
    return {
      type: 'PROCESS_CONTROL_EVENT',
      payload: { event: event as FlowControlEvent }
    };
  }

  return null;
}

/**
 * 将 WebSocketFlowMessage 转换为 Command
 * 纯函数，无副作用
 */
export function wsMessageToCommand(message: WebSocketFlowMessage): FlowEventCommand | null {
  return flowEventToCommand(message.event);
}
