/**
 * FlowEventModel (Core)
 * 纯粹的、同步的、基于 Immer 的状态管理核心。
 * 负责：
 * 1. 持有 FlowEvent 执行状态
 * 2. 管理节点执行状态的批量更新
 * 3. 执行 Command (调用 Processor)
 */

import type { Immutable } from 'immer';
import type { WorkflowStatus } from '../../store/types';
import type { FlowEvent, FlowEventCommand } from './commands';
import type { NodeExecutionData } from './types';
import {
  processNodeMsgEvent,
  processNodeStatusEvent,
  processControlEvent,
  processSetCurrentRun,
  processUpdateWorkflowStatus,
  processMapRunToWorkflow,
  processUnmapRun,
  processUpdateNodeExecutionData,
  processBatchUpdateNodeData,
  processClearPendingUpdates,
  processSetActiveRunContext,
  processClearActiveRunContext,
  processResetFlowEventState,
} from './processors';

// Command Handler 类型定义
export type FlowEventProcessor<T extends FlowEventCommand = FlowEventCommand> = (
  state: Immutable<FlowEventState>,
  command: T
) => Immutable<FlowEventState>;

export interface FlowEventState {
  currentRunId: string | null;
  currentWorkflowId: number | null;
  workflowStatus: WorkflowStatus;
  workflowStatuses: Record<number, WorkflowStatus>;
  runIdToWorkflowId: Record<string, number>;
  // 节点执行数据缓存（用于批量更新）
  pendingNodeUpdates: Map<string, NodeExecutionData>;
  // 当前活动执行上下文
  activeRunContext: {
    runId: string;
    startNodeId: string;
    workflowId: number | null;
  } | null;
}

export interface FlowEventModelOptions {
  initialState?: FlowEventState;
  onStateChange?: (state: Immutable<FlowEventState>) => void;
  onNodeUpdates?: (updates: Map<string, NodeExecutionData>) => void;
  onError?: (error: unknown, command: FlowEventCommand) => void;
}

const defaultState: FlowEventState = {
  currentRunId: null,
  currentWorkflowId: null,
  workflowStatus: 'idle',
  workflowStatuses: {},
  runIdToWorkflowId: {},
  pendingNodeUpdates: new Map(),
  activeRunContext: null,
};

export class FlowEventModel {
  private _state: Immutable<FlowEventState>;
  private _handlers = new Map<string, FlowEventProcessor>();
  private _options: FlowEventModelOptions;

  constructor(options: FlowEventModelOptions = {}) {
    this._options = options;
    this._state = options.initialState ?? defaultState;

    // 注册所有处理器
    this._registerHandlers();
  }

  // ===== Getters =====

  get state(): Immutable<FlowEventState> {
    return this._state;
  }

  get pendingNodeUpdates(): Map<string, NodeExecutionData> {
    return new Map(this._state.pendingNodeUpdates);
  }

  // ===== Public API =====

  registerHandler(type: string, handler: FlowEventProcessor): void {
    this._handlers.set(type, handler);
  }

  registerHandlers(handlers: Record<string, FlowEventProcessor>): void {
    Object.entries(handlers).forEach(([type, handler]) => {
      this._handlers.set(type, handler);
    });
  }

  execute(command: FlowEventCommand): void {
    const handler = this._handlers.get(command.type);
    if (!handler) {
      console.warn(`[FlowEventModel] No handler for command: ${command.type}`);
      return;
    }

    try {
      const nextState = handler(this._state, command);

      if (nextState === this._state) {
        return;
      }

      const previousPendingUpdates = this._state.pendingNodeUpdates;
      const nextPendingUpdates = nextState.pendingNodeUpdates;
      if (
        nextPendingUpdates !== previousPendingUpdates &&
        nextPendingUpdates.size > 0
      ) {
        const updatesSnapshot = new Map<string, NodeExecutionData>();
        nextPendingUpdates.forEach((data, nodeId) => {
          updatesSnapshot.set(nodeId, { ...data });
        });
        this._options.onNodeUpdates?.(updatesSnapshot);
      }

      this._updateState(nextState);
    } catch (error) {
      console.error(`[FlowEventModel] Error executing command ${command.type}:`, error);
      this._options.onError?.(error, command);
    }
  }

  // 快捷方法
  setCurrentRun(runId: string | null, workflowId: number | null): void {
    this.execute({
      type: 'SET_CURRENT_RUN',
      payload: { runId, workflowId },
    });
  }

  updateWorkflowStatus(workflowId: number, status: WorkflowStatus): void {
    this.execute({
      type: 'UPDATE_WORKFLOW_STATUS',
      payload: { workflowId, status },
    });
  }

  processFlowEvent(event: FlowEvent, runId?: string): void {
    this.execute({
      type: 'PROCESS_FLOW_EVENT',
      payload: { event, runId },
    });
  }

  clearPendingNodeUpdates(): void {
    this.execute({
      type: 'CLEAR_PENDING_UPDATES',
      payload: {},
    });
  }

  reset(): void {
    this._updateState(defaultState);
  }

  // ===== Private =====

  private _registerHandlers(): void {
    this.registerHandlers({
      // 事件处理器
      'PROCESS_NODE_MSG_EVENT': processNodeMsgEvent as FlowEventProcessor,
      'PROCESS_NODE_STATUS_EVENT': processNodeStatusEvent as FlowEventProcessor,
      'PROCESS_CONTROL_EVENT': processControlEvent as FlowEventProcessor,
      // Run 管理
      'SET_CURRENT_RUN': processSetCurrentRun as FlowEventProcessor,
      'UPDATE_WORKFLOW_STATUS': processUpdateWorkflowStatus as FlowEventProcessor,
      'MAP_RUN_TO_WORKFLOW': processMapRunToWorkflow as FlowEventProcessor,
      'UNMAP_RUN': processUnmapRun as FlowEventProcessor,
      // 节点更新
      'UPDATE_NODE_EXECUTION_DATA': processUpdateNodeExecutionData as FlowEventProcessor,
      'BATCH_UPDATE_NODE_DATA': processBatchUpdateNodeData as FlowEventProcessor,
      'CLEAR_PENDING_UPDATES': processClearPendingUpdates as FlowEventProcessor,
      // Run Node 执行
      'SET_ACTIVE_RUN_CONTEXT': processSetActiveRunContext as FlowEventProcessor,
      'CLEAR_ACTIVE_RUN_CONTEXT': processClearActiveRunContext as FlowEventProcessor,
      // Meta
      'RESET_FLOW_EVENT_STATE': processResetFlowEventState as FlowEventProcessor,
    });
  }

  private _updateState(nextState: Immutable<FlowEventState>): void {
    this._state = nextState;
    this._options.onStateChange?.(nextState);
  }
}
