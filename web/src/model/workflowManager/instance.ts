import { filter, type Subscription } from "rxjs";
import type {
    FlowControlEvent,
    FlowEvent,
    FlowNodeMsgEvent,
    FlowNodeStatusEvent,
    FlowNodeStatusEventType,
    RxFlowEvent,
    WebSocketFlowMessage,
} from "../flowEvent";
import { isFlowControlEvent, isFlowNodeMsgEvent, isFlowNodeStatusEvent } from "../flowEvent/RxFlowEvent";
import type { CommandMeta } from "../workflow/commands";
import type { WorkflowModelInterface } from "../workflow/interface";
import type { NodeStatus, WorkflowStatus } from "../workflow/types";
import { SubgraphManager } from "./subgraphInstance";

export function makeGraphKey(spaceId: string, workflowId: number): GraphKey {
    if (workflowId == null) return `${spaceId}:draft`;
    return `${spaceId}:${workflowId}`;
}

export type GraphKey = string;

export class ModelInstance {
    graphKey: GraphKey;
    model: WorkflowModelInterface;
    spaceId: string;
    workflowId: number;
    runId: string | null;
    status: WorkflowStatus = "idle";
    rxFlowEvent$: RxFlowEvent;
    private subgraphManager: SubgraphManager;

    private notifyWorkflowStatusChange: (graphKey: GraphKey, workflowId: number | null, runId: string | null, status: WorkflowStatus) => void;
    private flowEventSubscription: Subscription;

    constructor(
        graphKey: GraphKey,
        model: WorkflowModelInterface,
        rxFlowEvent$: RxFlowEvent,
        spaceId: string,
        workflowId: number,
        runId: string | null,
        notifyWorkflowStatusChange: (graphKey: GraphKey, workflowId: number | null, runId: string | null, status: WorkflowStatus) => void,
    ) {
        this.graphKey = graphKey;
        this.model = model;
        this.spaceId = spaceId;
        this.workflowId = workflowId;
        this.runId = runId;
        this.rxFlowEvent$ = rxFlowEvent$;
        this.notifyWorkflowStatusChange = notifyWorkflowStatusChange;

        this.subgraphManager = new SubgraphManager({
            getModel: () => this.model,
            getRunId: () => this.runId,
        });

        this.flowEventSubscription = this.rxFlowEvent$
            .getSource$()
            .pipe(filter(e => this.shouldProcessEvent(e)))
            .subscribe(e => {
                this.routeEvent(e);
            });
    }

    destroy() {
        this.flowEventSubscription.unsubscribe();
        this.subgraphManager.clearSubgraphInstances();
        this.model.destroy();
    }

    setRunId(runId: string | null) {
        this.runId = runId;
    }

    applyFlowEvent(event: FlowEvent) {
        if (isFlowNodeMsgEvent(event)) {
            this.applyFlowNodeMsgEvent(event);
        } else if (isFlowNodeStatusEvent(event)) {
            this.applyFlowNodeStatusEvent(event);
        } else if (isFlowControlEvent(event)) {
            this.applyFlowControlEvent(event);
        }
    }

    applyFlowNodeMsgEvent(event: FlowNodeMsgEvent) {
        const meta: CommandMeta = { skipHistory: true };
        const { nodeId, type, msg } = event;
        switch (type) {
            case "NodeInMessage":
                this.model.action.updateNodeData(
                    nodeId,
                    {
                        lastMessage: msg,
                        inputs: msg,
                    },
                    meta,
                );
                break;
            case "NodeOutMessage":
                this.model.action.updateNodeData(
                    nodeId,
                    {
                        lastMessage: msg,
                        outputs: { output: msg },
                    },
                    meta,
                );
                break;
            case "NodeStreamNextMessage": {
                const snapshot = this.model.getSnapshot();
                const node = snapshot.nodes.find(n => n.id === nodeId);
                const prev = node?.data?.lastMessage;
                const lastMessage = typeof msg === "string" ? `${typeof prev === "string" ? prev : ""}${msg}` : msg;
                this.model.action.updateNodeData(nodeId, { lastMessage }, meta);
                break;
            }
            case "NodeError":
                this.model.action.updateNodeData(
                    nodeId,
                    {
                        errorMessage: msg,
                        status: "error",
                    },
                    meta,
                );
                break;
        }
    }

    applyFlowNodeStatusEvent(event: FlowNodeStatusEvent) {
        const meta: CommandMeta = { skipHistory: true };
        const eventTypeToNodeStatus: Record<FlowNodeStatusEventType, NodeStatus> = {
            NodeStarted: "running",
            NodeStreamStarted: "running",
            NodeCompleted: "completed",
        };
        const { nodeId, type } = event;
        const updates: Record<string, any> = {
            status: eventTypeToNodeStatus[type],
        };
        if (type === "NodeStreamStarted") {
            updates.isOutputStream = true;
            updates.lastMessage = "";
            updates.errorMessage = undefined;
        } else if (type === "NodeStarted") {
            updates.isOutputStream = false;
            updates.lastMessage = "";
            updates.errorMessage = undefined;
        } else if (type === "NodeCompleted") {
            //
        }
        this.model.action.updateNodeData(nodeId, updates, meta);
    }

    applyFlowControlEvent(event: FlowControlEvent) {
        const meta: CommandMeta = { skipHistory: true };
        const status: WorkflowStatus = (() => {
            switch (event.type) {
            case "FlowStarted":
                return "running";
            case "FlowPaused":
                return "paused";
            case "FlowResumed":
                return "running";
            case "FlowStopped":
            case "FlowFinished":
                return "idle";
            default:
                return "idle";
            }
        })();
        if (event.type === "FlowStopped") {
            this.model.action.resetAllNodeStatus(meta);
            this.subgraphManager.clearSubgraphInstances();
        }
        this.status = status;
        this.notifyWorkflowStatusChange(this.graphKey, this.workflowId, this.runId, this.status);
    }

    get subgraphInstances() {
        return this.subgraphManager.getSubgraphInstances();
    }

    clearSubgraphInstances() {
        this.subgraphManager.clearSubgraphInstances();
    }

    getSubgraphThreads(parentNodeId: string) {
        return this.subgraphManager.getSubgraphThreads(parentNodeId);
    }

    activateSubgraph(parentNodeId: string, threadIndex: number = 0): boolean {
        return this.subgraphManager.activateSubgraph(parentNodeId, threadIndex);
    }

    private shouldProcessEvent(e: WebSocketFlowMessage): boolean {
        if (e.runnerKind === "Root") {
            return e.runId === this.runId;
        }
        return this.subgraphManager.shouldProcessEvent(e);
    }

    private routeEvent(e: WebSocketFlowMessage) {
        if (e.runnerKind === "Root") {
            this.applyFlowEvent(e.event);
        } else {
            this.subgraphManager.applySubgraphEvent(e);
        }
    }
}
