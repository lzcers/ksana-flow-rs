/**
 * RxFlowEvent (Reactive Layer)
 * 包装 Core FlowEventModel，提供 RxJS 响应式接口。
 */

import {
  BehaviorSubject,
  Subject,
  Observable,
  interval,
  timer,
  animationFrameScheduler,
  merge,
} from 'rxjs';
import {
  map,
  distinctUntilChanged,
  shareReplay,
  filter,
  bufferWhen,
  tap,
  retry,
} from 'rxjs/operators';
import type { Immutable } from 'immer';
import type { WorkflowStatus } from '../../store/types';
import {
  FlowEventModel,
  type FlowEventState,
  type FlowEventModelOptions,
  type FlowEventProcessor,
} from './flowEventModel';
import {
  processFlowEvent,
  processNodeMsgEvent,
  processNodeStatusEvent,
  processControlEvent,
  processWebSocketMessage,
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
import type { FlowEventCommand, NodeExecutionData } from './commands';
import type { FlowEvent, WebSocketFlowMessage } from './types';
import { createFlowSocketObservable } from './socket';

export interface RxFlowEventOptions extends FlowEventModelOptions {
  enableLogging?: boolean;
}

export class RxFlowEvent {
  private _model: FlowEventModel;

  // Subjects
  private _state$ = new BehaviorSubject<Immutable<FlowEventState>>({
    currentRunId: null,
    currentWorkflowId: null,
    workflowStatus: 'idle' as WorkflowStatus,
    workflowStatuses: {},
    runIdToWorkflowId: {},
    pendingNodeUpdates: new Map(),
    activeRunContext: null,
  });
  private _commands$ = new Subject<FlowEventCommand>();
  private _events$ = new Subject<FlowEvent>();
  private _batchedUpdates$ = new Subject<Map<string, NodeExecutionData>>();

  // Public Observables
  public readonly state$: Observable<Immutable<FlowEventState>>;
  public readonly commands$ = this._commands$.asObservable();
  public readonly events$ = this._events$.asObservable();
  public readonly batchedNodeUpdates$: Observable<Map<string, NodeExecutionData>>;

  // Derived Observables
  public readonly currentRunId$: Observable<string | null>;
  public readonly workflowStatus$: Observable<WorkflowStatus>;
  public readonly currentWorkflowId$: Observable<number | null>;
  public readonly pendingUpdates$: Observable<Map<string, NodeExecutionData>>;

  constructor(options: RxFlowEventOptions = {}) {
    this._model = new FlowEventModel({
      ...options,
      onStateChange: (state) => this._state$.next(state),
      onNodeUpdates: (updates) => this._batchedUpdates$.next(updates),
    });

    // 注册处理器
    this._model.registerHandlers({
      // 通用事件处理（向后兼容）
      'PROCESS_FLOW_EVENT': processFlowEvent as FlowEventProcessor,
      // 分离的事件处理器
      'PROCESS_NODE_MSG_EVENT': processNodeMsgEvent as FlowEventProcessor,
      'PROCESS_NODE_STATUS_EVENT': processNodeStatusEvent as FlowEventProcessor,
      'PROCESS_CONTROL_EVENT': processControlEvent as FlowEventProcessor,
      'PROCESS_WS_MESSAGE': processWebSocketMessage as FlowEventProcessor,
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

    // 初始化 State 流
    this._state$.next(this._model.state);
    this.state$ = this._state$.asObservable();

    // 批量更新流（使用 bufferWhen 实现批量处理）
    this.batchedNodeUpdates$ = this._batchedUpdates$.pipe(
      bufferWhen(() => merge(
        interval(0, animationFrameScheduler),
        timer(16)
      )),
      filter(batch => batch.length > 0),
      map(batch => {
        // 合并多个更新
        const merged = new Map<string, NodeExecutionData>();
        batch.forEach(updates => {
          updates.forEach((data, nodeId) => {
            const existing = merged.get(nodeId) ?? {};
            merged.set(nodeId, { ...existing, ...data });
          });
        });
        return merged;
      }),
      shareReplay({ bufferSize: 1, refCount: true })
    );

    // 派生流
    this.currentRunId$ = this.state$.pipe(
      map(s => s.currentRunId),
      distinctUntilChanged()
    );

    this.workflowStatus$ = this.state$.pipe(
      map(s => s.workflowStatus),
      distinctUntilChanged()
    );

    this.currentWorkflowId$ = this.state$.pipe(
      map(s => s.currentWorkflowId),
      distinctUntilChanged()
    );

    this.pendingUpdates$ = this.state$.pipe(
      map(s => s.pendingNodeUpdates as Map<string, NodeExecutionData>),
      distinctUntilChanged()
    );
  }

  // ===== Public API =====

  /**
   * 分发 Command
   */
  dispatch(command: FlowEventCommand): void {
    this._commands$.next(command);
    this._model.execute(command);
  }

  /**
   * 发送事件到事件流
   */
  emitEvent(event: FlowEvent): void {
    this._events$.next(event);
    this._model.processFlowEvent(event, this._model.state.currentRunId || undefined);
  }

  /**
   * 按 runId 过滤的事件流
   */
  eventsForRun$(runId: string): Observable<FlowEvent> {
    return this.events$.pipe(
      filter(event => {
        // 从事件中提取 runId（如果有）
        // FlowControlEvent 有 runId 字段
        if ('runId' in event) {
          return event.runId === runId;
        }
        return true; // 其他事件（如节点事件）默认匹配当前 run
      })
    );
  }

  /**
   * 按 nodeId 过滤的事件流
   */
  eventsForNode$(nodeId: string): Observable<FlowEvent> {
    return this.events$.pipe(
      filter(event => {
        if (!('nodeId' in event)) return false;
        return event.nodeId === nodeId;
      })
    );
  }

  /**
   * WebSocket 集成
   */
  connectWebSocket(spaceId: string): Observable<WebSocketFlowMessage> {
    return createFlowSocketObservable(spaceId).pipe(
      retry({ delay: 2000 }),
      tap(message => {
        this._events$.next(message.event);
        this.dispatch({
          type: 'PROCESS_FLOW_EVENT',
          payload: { event: message.event },
        });
      })
    );
  }

  /**
   * 快捷方法：设置当前运行
   */
  setCurrentRun(runId: string | null, workflowId: number | null): void {
    this.dispatch({
      type: 'SET_CURRENT_RUN',
      payload: { runId, workflowId },
    });
  }

  /**
   * 快捷方法：更新工作流状态
   */
  updateWorkflowStatus(workflowId: number, status: WorkflowStatus): void {
    this.dispatch({
      type: 'UPDATE_WORKFLOW_STATUS',
      payload: { workflowId, status },
    });
  }

  /**
   * 快捷方法：处理 FlowEvent
   */
  processFlowEvent(event: FlowEvent, runId?: string): void {
    this.dispatch({
      type: 'PROCESS_FLOW_EVENT',
      payload: { event, runId },
    });
  }

  /**
   * 快捷方法：清空待处理更新
   */
  clearPendingUpdates(): void {
    this.dispatch({
      type: 'CLEAR_PENDING_UPDATES',
      payload: {},
    });
  }

  /**
   * 注册处理器
   */
  registerHandler<T extends FlowEventCommand>(type: string, handler: (state: Immutable<FlowEventState>, command: T) => Immutable<FlowEventState>): void {
    this._model.registerHandler(type, handler as never);
  }

  /**
   * 销毁
   */
  destroy(): void {
    this._state$.complete();
    this._commands$.complete();
    this._events$.complete();
    this._batchedUpdates$.complete();
  }

  // ===== Getters =====

  get currentState(): Immutable<FlowEventState> {
    return this._model.state;
  }
}
