/**
 * FlowEvent Command 类型定义
 * 所有 FlowEvent 相关操作都定义为 Command，通过 CommandBus 分发
 */

import type { WorkflowStatus } from '../../store/types';
import type { FlowEvent, WebSocketFlowMessage } from './types';

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
