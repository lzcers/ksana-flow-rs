/**
 * RxFlowEvent (Reactive Layer)
 * connectWebSocket (WebSocketFlowMessage) - 根流
 * ├── workflowStatusForRunId$ (按 runId 过滤的状态流)
 * └── flowEventForRunId$ (按 runId 过滤的事件流)
 *     └── flowEventForNodeId$ (按 nodeId 过滤的事件流)
 */
import {
  Observable,
  type Subscription,
  ReplaySubject,
} from 'rxjs';

import {
  map,
  distinctUntilChanged,
  filter,
  retry,
} from 'rxjs/operators';
import type { FlowEvent, WebSocketFlowMessage, FlowControlEvent, FlowNodeStatusEvent, FlowNodeMsgEvent } from './types';
import { createFlowSocketObservable } from './socket';
import type { WorkflowStatus } from '@/store/types';


export class RxFlowEvent {
  private _source$ = new ReplaySubject<WebSocketFlowMessage>(20);
  private _socketSubscription: Subscription | null = null;
  private _currentSpaceId: string | null = null;


  constructor() {

  }
  /**
   * 根流：连接 WebSocket，将消息发送给 _source$
   * 这是所有派生流的源头
   * 注意：此方法将 WebSocket 消息推送到 _source$，所有派生流都基于 _source$
   */
  connectWebSocket(spaceId: string): void {
    if (this._currentSpaceId === spaceId && this._socketSubscription) return;
    this.disconnectWebSocket();
    this._currentSpaceId = spaceId;

    const socket$ = createFlowSocketObservable(spaceId).pipe(
      retry({ delay: 2000 })
    );

    this._socketSubscription = socket$.subscribe({
      next: (message) => this._source$.next(message),
      error: (err) => console.error('[RxFlowEvent] WebSocket error:', err),
    });
  }

  disconnectWebSocket(): void {
    this._socketSubscription?.unsubscribe();
    this._socketSubscription = null;
    this._currentSpaceId = null;
  }

  // ===== Stream Derivation API =====

  /**
   * 派生流 1：按 runId 过滤的工作流状态流
   * 从 WebSocketFlowMessage 中派生，只关注指定 runId 的状态变化
   * 注意：此流只输出纯数据，不做任何 command 分发
   */
  workflowStatusForRunId$(
    runId: string
  ): Observable<WorkflowStatus> {
    return this._source$.pipe(
      filter(msg => msg.runId === runId),
      map(msg => msg.event),
      filter((event): event is FlowControlEvent =>
        ['FlowPaused', 'FlowResumed', 'FlowStopped', 'FlowFinished'].includes(event.type)
      ),
      map(event => {
        // 根据控制事件类型映射到 WorkflowStatus
        switch (event.type) {
          case 'FlowPaused': return 'paused' as WorkflowStatus;
          case 'FlowResumed': return 'running' as WorkflowStatus;
          case 'FlowStopped': return 'idle' as WorkflowStatus;
          case 'FlowFinished': return 'idle' as WorkflowStatus;
          default: return 'idle' as WorkflowStatus;
        }
      }),
      distinctUntilChanged()
    );
  }

  flowMessageForRunId$(
    runId: string
  ): Observable<WebSocketFlowMessage> {
    return this._source$.pipe(
      filter(msg => msg.runId === runId)
    );
  }

  /**
   * 派生流 2：按 runId 过滤的 FlowEvent 流
   * 从 WebSocketFlowMessage 中派生，只关注指定 runId 的事件
   * 注意：此流只输出纯数据，不做任何 command 分发
   */
  flowEventForRunId$(
    runId: string
  ): Observable<FlowEvent> {
    return this._source$.pipe(
      filter(msg => msg.runId === runId),
      filter(msg => msg.runnerKind === 'Root' || !msg.runnerKind),
      map(msg => msg.event)
    );
  }

  /**
   * 派生流 3：按 nodeId 过滤的 FlowEvent 流
   * 从 flowEventForRunId$ 流中进一步派生
   */
  flowEventForNodeId$(
    nodeId: string
  ) {
    return (flowEventObservable: Observable<FlowEvent>) =>
      flowEventObservable.pipe(
        filter((event): event is FlowEvent =>
          'nodeId' in event && event.nodeId === nodeId
        )
      );
  }

  flowMessageForRunnerId$(
    runnerId: number
  ) {
    return (flowMessageObservable: Observable<WebSocketFlowMessage>) =>
      flowMessageObservable.pipe(
        filter((msg) => msg.runnerId === runnerId)
      );
  }

  flowMessageForSubgraphNodeId$(
    parentNodeId: string
  ) {
    return (flowMessageObservable: Observable<WebSocketFlowMessage>) =>
      flowMessageObservable.pipe(
        filter((msg) => msg.parentNodeId === parentNodeId)
      );
  }

  /**
   * 派生流 4：节点状态流
   * 从 flowEventForNodeId$ 流中派生，提取节点状态
   * 注意：此流只输出纯数据，不做任何 command 分发
   */
  nodeStatus$(
    nodeId: string
  ) {
    const isFlowNodeStatusEvent = (event: FlowEvent): event is FlowNodeStatusEvent =>
      'nodeId' in event &&
      event.nodeId === nodeId &&
      (event.type === 'NodeStarted' ||
        event.type === 'NodeStreamStarted' ||
        event.type === 'NodeCompleted');

    return (flowEventObservable: Observable<FlowEvent>): Observable<FlowNodeStatusEvent> =>
      flowEventObservable.pipe(
        filter((event): event is FlowEvent =>
          'nodeId' in event && event.nodeId === nodeId
        ),
        filter((event): event is FlowNodeStatusEvent =>
          isFlowNodeStatusEvent(event)
        ),
        distinctUntilChanged((a, b) => a.type === b.type)
      );
  }

  /**
   * 派生流 5：节点数据更新流
   * 从 flowEventForNodeId$ 流中派生，提取节点数据更新事件
   * 注意：此流只输出纯数据，不做任何 command 分发
   */
  nodeDataUpdate$(
    nodeId: string
  ) {
    const isFlowNodeMsgEvent = (event: FlowEvent): event is FlowNodeMsgEvent =>
      'nodeId' in event && event.nodeId === nodeId && 'msg' in event;

    return (flowEventObservable: Observable<FlowEvent>): Observable<FlowNodeMsgEvent> =>
      flowEventObservable.pipe(
        filter((event): event is FlowEvent =>
          'nodeId' in event && event.nodeId === nodeId
        ),
        filter(event => isFlowNodeMsgEvent(event))
      )
  }
  /**
   * 销毁：完成 _source$ 流，释放资源
   */
  destroy(): void {
    this.disconnectWebSocket();
    this._source$.complete();
  }
}
