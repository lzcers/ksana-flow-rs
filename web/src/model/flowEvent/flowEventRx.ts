/**
 * RxFlowEvent (Reactive Layer)
 * 包装 Core FlowEventModel，提供 RxJS 响应式接口。
 * 
 * 流派生结构:
 * connectWebSocket (WebSocketFlowMessage) - 根流
 * ├── workflowStatusForRunId$ (按 runId 过滤的状态流)
 * └── flowEventForRunId$ (按 runId 过滤的事件流)
 *     └── flowEventForNodeId$ (按 nodeId 过滤的事件流)
 *         └── nodeStatus$ (节点状态流)
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
} from './flowEventModel';
import type { FlowEventCommand, NodeExecutionData } from './commands';
import type { FlowEvent, WebSocketFlowMessage, FlowControlEvent } from './types';
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

  // ===== Stream Derivation API =====

  /**
   * 根流：连接 WebSocket，返回 WebSocketFlowMessage 流
   * 这是所有派生流的源头
   */
  connectWebSocket(spaceId: string): Observable<WebSocketFlowMessage> {
    return createFlowSocketObservable(spaceId).pipe(
      retry({ delay: 2000 }),
      tap(message => {
        // 将原始事件也发送到 events$ Subject
        this._events$.next(message.event);
      })
    );
  }

  /**
   * 派生流 1：按 runId 过滤的工作流状态流
   * 从 WebSocketFlowMessage 中派生，只关注指定 runId 的状态变化
   */
  workflowStatusForRunId$(
    source$: Observable<WebSocketFlowMessage>,
    runId: string
  ): Observable<WorkflowStatus> {
    return source$.pipe(
      filter(msg => msg.runId === runId),
      map(msg => msg.event),
      filter((event): event is FlowControlEvent => 
        'runId' in event && 
        ['FlowPaused', 'FlowResumed', 'FlowStopped', 'FlowFinished'].includes(event.type)
      ),
      tap(event => {
        // 分发 Command 更新工作流状态
        this.dispatch({
          type: 'PROCESS_CONTROL_EVENT',
          payload: { event }
        });
      }),
  map(event => {
          // 根据控制事件类型映射到 WorkflowStatus
          switch (event.type) {
            case 'FlowPaused': return 'paused' as WorkflowStatus;
            case 'FlowResumed': return 'running' as WorkflowStatus;
            case 'FlowStopped': return 'stopped' as WorkflowStatus;
            case 'FlowFinished': return 'completed' as WorkflowStatus;
            default: return 'idle' as WorkflowStatus;
          }
          ,
            inctUntilChanged()
            
          
        
      *
     * 派生流 2：按 runId 过滤的 FlowEvent 流
   * 从 WebSocketFlowMessage 中派生，只关注指定 runId 的事件
 */
 lowEventForRunId$(
  source$: Observable<WebSocketFlowMessage>,
  runId: string
   Observable<FlowEvent> {
    turn source$.pipe(
    filter(msg => msg.runId === runId || !msg.runId),
    map(msg => msg.event),
    tap(event => {
        // 分发 Command 处理 FlowEvent
      if ('nodeId' in event) {
        // 节点相关事件
        if (['NodeError', 'NodeInMessage', 'NodeOutMessage', 'NodeStreamNextMessage'].includes(event.type)) {
          this.dispatch({
            type: 'PROCESS_NODE_MSG_EVENT',
            payload: { event: event as any }
          });
        } else if (['NodeStarted', 'NodeStreamStarted', 'NodeCompleted'].includes(event.type)) {
          this.dispatch({
              type: 'PROCESS_NODE_STATUS_EVENT',
            payload: { event: event as any }
          });
        }
      } else if ('runId' in event) { 
        // 控制事件
        this.dispatch({
          type: 'PROCESS_CONTROL_EVENT',
          payload: { event: event as any }
        });
        }
    })
  );
 

  *
    生流 3：按 nodeId 过滤的 FlowEvent 流
     FlowEventForRunId 流中进一步派生
  /
flowEventForNodeId$(
    source$: Observable<FlowEvent>,
  nodeId: string
 : Observable<FlowEvent> {
  return source$.pipe(
    filter((event): event is FlowEvent => 
      'nodeId' in event && event.nodeId === nodeId
    )
    );
}
          
 **
 * 派生流 4：节点状态流
   从 FlowEventForNodeId 流中派生，提取节点状态
  /
  deStatus$(
  source$: Observable<FlowEvent>,
  nodeId: string
  ): Observable<{ status: 'idle' | 'running' | 'completed' | 'error'; message?: any }> {
    return source$.pipe(
      filter((event): event is FlowEvent => 
        'nodeId' in event && ev e nt.nodeId ===  nodeId
    ),
    tap(event => {
        // 分发 Command 更新节点执行数据
        if (['NodeStarted', 'NodeStreamStarted', 'NodeCompleted', 'NodeError'].includes(event.type)) {
          const status = event.type === 'NodeStarted' || event.type === 'NodeStreamStarted' ? 'running' :
                         event.type === 'NodeCompleted' ? 'completed' : 'error';
          
          this.dispatch({
            type: 'UPDATE_NODE_EXECUTION_DATA',
            payload: {
              nodeId,
              data: {
                status,
                ...(event.type === 'NodeError' && { errorMessage: (event as any).msg })
              }
            }
          });
        } else if (['NodeInMessage', 'NodeOutMessage', 'NodeStreamNextMessage'].includes(event.type)) {
          this.dispatch({
            type: 'UPDATE_NODE_EXECUTION_DATA',
            payload: {
              nodeId,
              data: {
                lastMessage: (event as any).msg,
                isOutputStream: event.type === 'NodeStreamNextMessage' || event.type === 'NodeOutMessage'
              }
            }
          });
        }
      }),
      map(event => {
        const status = event.type === 'NodeStarted' || event.type === 'NodeStreamStarted' ? 'running' :
                       event.type === 'NodeCompleted' ? 'completed' :
                       event.type === 'NodeError' ? 'error' : 'idle';
        return {
          status,
          message: 'msg' in event ? (event as any).msg : undefined
        };
      }),
      distinctUntilChanged((a, b) => a.status === b.status)
    );
  }

  // ===== Legacy Public API =====

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
