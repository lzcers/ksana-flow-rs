import { type Subscription, ReplaySubject } from "rxjs";

import { map, filter, retry } from "rxjs/operators";
import type { FlowEvent, WebSocketFlowMessage, FlowControlEvent, FlowNodeStatusEvent, FlowNodeMsgEvent, FlowNodeMsgEventType } from "./types";
import { createFlowSocketObservable } from "./socket";
import type { WorkflowStatus } from "../workflow/types";

export class RxFlowEvent {
    private _source$ = new ReplaySubject<WebSocketFlowMessage>(20);
    private _socketSubscription: Subscription | null = null;
    private _currentSpaceId: string | null = null;

    constructor() {}
    /**
     * 根流：连接 WebSocket，将消息发送给 _source$
     * 这是所有派生流的源头
     * 注意：此方法将 WebSocket 消息推送到 _source$，所有派生流都基于 _source$
     */
    connectWebSocket(spaceId: string): void {
        if (this._currentSpaceId === spaceId && this._socketSubscription) return;
        this.disconnectWebSocket();
        this._currentSpaceId = spaceId;

        const socket$ = createFlowSocketObservable(spaceId).pipe(retry({ delay: 2000 }));

        this._socketSubscription = socket$.subscribe({
            next: message => this._source$.next(message),
            error: err => console.error("[RxFlowEvent] WebSocket error:", err),
        });
    }

    disconnectWebSocket(): void {
        this._socketSubscription?.unsubscribe();
        this._socketSubscription = null;
        this._currentSpaceId = null;
    }

    // ===== Stream Derivation API =====
    getSource$ = () => this._source$;

    workflowStatus$ = () =>
        this._source$.pipe(
            filter(msg => isFlowControlEvent(msg.event)),
            map(msg => {
                // 根据控制事件类型映射到 WorkflowStatus
                let status = "idle" as WorkflowStatus;
                switch (msg.event.type) {
                    case "FlowPaused":
                        status = "paused";
                        break;
                    case "FlowResumed":
                        status = "running";
                        break;
                    case "FlowStopped":
                        status = "idle";
                        break;
                    case "FlowFinished":
                        status = "idle";
                        break;
                    default:
                        status = "idle";
                        break;
                }
                return { runId: msg.runId, status };
            }),
        );

    workflowNodeStatus$ = () => {
        return this._source$.pipe(
            filter(msg => isFlowNodeStatusEvent(msg.event)),
            map(msg => {
                const { ...rest } = msg;
                const evt = msg.event as FlowNodeStatusEvent;
                return { nodeId: evt.nodeId, status: evt.type, ...rest };
            }),
        );
    };

    workflowNodeMessage$ = () => {
        return this._source$.pipe(
            filter(msg => isFlowNodeMsgEvent(msg.event)),
            map(msg => {
                const { ...rest } = msg;
                const evt = msg.event as FlowNodeMsgEvent;
                const type = msg.event.type as FlowNodeMsgEventType;
                return { nodeId: evt.nodeId, type, msg: evt.msg, ...rest };
            }),
        );
    };
    /**
     * 销毁：完成 _source$ 流，释放资源
     */
    destroy(): void {
        this.disconnectWebSocket();
        this._source$.complete();
    }
}

export function isFlowControlEvent(event: FlowEvent): event is FlowControlEvent {
    return (
        event.type === "FlowStarted" ||
        event.type === "FlowPaused" ||
        event.type === "FlowResumed" ||
        event.type === "FlowStopped" ||
        event.type === "FlowFinished"
    );
}

export function isFlowNodeStatusEvent(event: FlowEvent): event is FlowNodeStatusEvent {
    return ("nodeId" in event && event.type === "NodeStarted") || event.type === "NodeStreamStarted" || event.type === "NodeCompleted";
}

export function isFlowNodeMsgEvent(event: FlowEvent): event is FlowNodeMsgEvent {
    return (
        ("nodeId" in event && event.type === "NodeError") ||
        event.type === "NodeInMessage" ||
        event.type === "NodeOutMessage" ||
        event.type === "NodeStreamNextMessage"
    );
}
