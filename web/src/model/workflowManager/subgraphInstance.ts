import type {
    FlowControlEvent,
    FlowEvent,
    FlowNodeMsgEvent,
    FlowNodeStatusEvent,
    FlowNodeStatusEventType,
    WebSocketFlowMessage,
    SubgraphFrame,
} from "../flowEvent";
import { isFlowControlEvent, isFlowNodeMsgEvent, isFlowNodeStatusEvent } from "../flowEvent/RxFlowEvent";
import type { CommandMeta } from "../workflow/commands";
import type { WorkflowModelInterface } from "../workflow/interface";
import type { NodeStatus, WorkflowStatus } from "../workflow/types";

export interface SubgraphInstance {
    runnerId: number;
    parentNodeId: string;
    parentRunnerId: number;
    status: WorkflowStatus;
    nodeStatuses: Map<string, NodeStatus>;
    messages: Map<string, any>;
    subgraphPath: SubgraphFrame[];
    isActive: boolean;
    threadIndex?: number;
}

export interface SubgraphManagerDeps {
    getModel: () => WorkflowModelInterface;
    getRunId: () => string | null;
}

export class SubgraphManager {
    private subgraphInstances: Map<number, SubgraphInstance> = new Map();
    private pendingSubgraphActivation: Map<string, { threadIndex: number; parentNodeId: string }> = new Map();
    private deps: SubgraphManagerDeps;

    constructor(deps: SubgraphManagerDeps) {
        this.deps = deps;
    }

    getSubgraphInstances(): Map<number, SubgraphInstance> {
        return this.subgraphInstances;
    }

    shouldProcessEvent(e: WebSocketFlowMessage): boolean {
        const runId = this.deps.getRunId();
        if (e.runId === runId && e.runnerKind === "Root") {
            return true;
        }

        if (e.parentRunnerId !== undefined && e.runId === runId) {
            if (e.parentRunnerId === 1 || this.subgraphInstances.has(e.parentRunnerId)) {
                return true;
            }
        }
        return false;
    }

    applySubgraphEvent(msg: WebSocketFlowMessage) {
        const runnerId = msg.runnerId;
        if (runnerId === undefined) return;

        let subgraph = this.subgraphInstances.get(runnerId);
        if (!subgraph) {
            subgraph = this.createSubgraphInstance(msg, runnerId);
            if (!subgraph) return;
        }

        this.processSubgraphEvent(subgraph, msg.event);
    }

    clearSubgraphInstances() {
        this.subgraphInstances.clear();
        this.pendingSubgraphActivation.clear();

        const meta: CommandMeta = { skipHistory: true };
        const model = this.deps.getModel();
        const snapshot = model.getSnapshot();

        snapshot.nodes.forEach(node => {
            if (node.data?.subgraphStatuses || node.data?.hasSubgraph) {
                const newData = { ...node.data };
                delete newData.subgraphStatuses;
                delete newData.subgraphAggregateStatus;
                delete newData.subgraphProgress;
                delete newData.hasSubgraph;
                delete newData.activeThreadIndex;
                delete newData.activeRunnerId;
                delete newData.subgraphStatus;
                model.action.updateNodeData(node.id, newData, meta);
            }
        });
    }

    getSubgraphThreads(parentNodeId: string): Array<{
        runnerId: number;
        threadIndex: number;
        status: WorkflowStatus;
    }> {
        const threads: Array<{ runnerId: number; threadIndex: number; status: WorkflowStatus }> = [];

        this.subgraphInstances.forEach((subgraph, runnerId) => {
            if (subgraph.parentNodeId === parentNodeId) {
                threads.push({
                    runnerId,
                    threadIndex: subgraph.threadIndex ?? 0,
                    status: subgraph.status,
                });
            }
        });

        return threads.sort((a, b) => a.threadIndex - b.threadIndex);
    }

    activateSubgraph(parentNodeId: string, threadIndex: number = 0): boolean {
        const threads = this.getSubgraphThreads(parentNodeId);
        const targetThread = threads.find(t => t.threadIndex === threadIndex);

        if (!targetThread) {
            this.pendingSubgraphActivation.set(`${parentNodeId}:${threadIndex}`, {
                threadIndex,
                parentNodeId,
            });

            const meta: CommandMeta = { skipHistory: true };
            const model = this.deps.getModel();
            const currentData = model.getNodeData(parentNodeId) || {};
            model.action.updateNodeData(
                parentNodeId,
                {
                    ...currentData,
                    activeThreadIndex: threadIndex,
                    subgraphStatus: "waiting",
                },
                meta,
            );

            return true;
        }

        const subgraph = this.subgraphInstances.get(targetThread.runnerId);
        if (!subgraph) return false;

        this.pendingSubgraphActivation.delete(`${parentNodeId}:${threadIndex}`);

        return this.activateSubgraphInstance(subgraph, {
            updateParentState: true,
            parentNodeId,
            threadIndex,
        });
    }

    private createSubgraphInstance(msg: WebSocketFlowMessage, runnerId: number): SubgraphInstance | undefined {
        const parentNodeId = msg.parentNodeId;
        const parentRunnerId = msg.parentRunnerId;

        if (parentNodeId === undefined || parentRunnerId === undefined) {
            console.warn("Subgraph event missing parent info:", msg);
            return undefined;
        }

        let threadIndex = 0;
        const existingThreads = this.getSubgraphThreadIndices(parentNodeId);
        if (existingThreads.length > 0) {
            threadIndex = Math.max(...existingThreads) + 1;
        }

        const subgraph: SubgraphInstance = {
            runnerId,
            parentNodeId,
            parentRunnerId,
            threadIndex,
            status: "idle",
            nodeStatuses: new Map(),
            messages: new Map(),
            subgraphPath: msg.subgraphPath || [],
            isActive: false,
        };

        this.subgraphInstances.set(runnerId, subgraph);

        this.checkAndApplyPendingActivation(subgraph, parentNodeId, threadIndex);

        return subgraph;
    }

    private checkAndApplyPendingActivation(subgraph: SubgraphInstance, parentNodeId: string, threadIndex: number) {
        const pendingKey = `${parentNodeId}:${threadIndex}`;
        const pendingActivation = this.pendingSubgraphActivation.get(pendingKey);

        if (pendingActivation && pendingActivation.threadIndex === threadIndex) {
            this.activateSubgraphInstance(subgraph, {
                updateParentState: true,
                parentNodeId,
                threadIndex,
            });

            this.pendingSubgraphActivation.delete(pendingKey);

            const meta: CommandMeta = { skipHistory: true };
            const model = this.deps.getModel();
            const currentData = model.getNodeData(parentNodeId) || {};
            model.action.updateNodeData(
                parentNodeId,
                {
                    ...currentData,
                    subgraphStatus: "active",
                },
                meta,
            );
        }
    }

    private processSubgraphEvent(subgraph: SubgraphInstance, event: FlowEvent) {
        this.storeSubgraphEvent(subgraph, event);

        if (subgraph.isActive) {
            this.syncSubgraphToCanvas(subgraph, event);
        }
    }

    private storeSubgraphEvent(subgraph: SubgraphInstance, event: FlowEvent) {
        if (isFlowNodeMsgEvent(event)) {
            this.storeNodeMsgEvent(subgraph, event);
        } else if (isFlowNodeStatusEvent(event)) {
            this.storeNodeStatusEvent(subgraph, event);
        } else if (isFlowControlEvent(event)) {
            this.storeControlEvent(subgraph, event);
        }
    }

    private storeNodeMsgEvent(subgraph: SubgraphInstance, event: FlowNodeMsgEvent) {
        const { nodeId, type, msg: message } = event;

        switch (type) {
            case "NodeInMessage":
                subgraph.messages.set(`${nodeId}:input`, message);
                break;
            case "NodeOutMessage":
                subgraph.messages.set(`${nodeId}:output`, message);
                break;
            case "NodeStreamNextMessage": {
                const prev = subgraph.messages.get(`${nodeId}:stream`) || "";
                subgraph.messages.set(`${nodeId}:stream`, typeof message === "string" ? prev + message : message);
                break;
            }
            case "NodeError":
                subgraph.messages.set(`${nodeId}:error`, message);
                break;
        }
    }

    private storeNodeStatusEvent(subgraph: SubgraphInstance, event: FlowNodeStatusEvent) {
        const { nodeId, type } = event;
        const eventTypeToNodeStatus: Record<FlowNodeStatusEventType, NodeStatus> = {
            NodeStarted: "running",
            NodeStreamStarted: "running",
            NodeCompleted: "completed",
        };
        // 清空流式消息
        if (type === "NodeStreamStarted") {
            subgraph.messages.set(`${nodeId}:stream`, "");
        }
        const status = eventTypeToNodeStatus[type];
        subgraph.nodeStatuses.set(nodeId, status);
    }

    private storeControlEvent(subgraph: SubgraphInstance, event: FlowControlEvent) {
        let status: WorkflowStatus = "idle";
        switch (event.type) {
            case "FlowStarted":
                status = "running";
                break;
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
        }
        subgraph.status = status;

        if (event.type === "FlowStopped") {
            subgraph.nodeStatuses.clear();
        }
    }

    private syncSubgraphToCanvas(subgraph: SubgraphInstance, event: FlowEvent) {
        const subgraphNodeIds = this.getSubgraphNodeIds(subgraph.parentNodeId);
        if (subgraphNodeIds.length === 0) return;

        const meta: CommandMeta = { skipHistory: true };

        if (isFlowNodeMsgEvent(event) || isFlowNodeStatusEvent(event)) {
            this.syncNodeStateToCanvas(subgraph, event.nodeId, meta);
        } else {
            subgraphNodeIds.forEach(nodeId => {
                this.syncNodeStateToCanvas(subgraph, nodeId, meta);
            });
        }
    }

    private syncNodeStateToCanvas(subgraph: SubgraphInstance, nodeId: string, meta: CommandMeta) {
        const nodeStatus = subgraph.nodeStatuses.get(nodeId);
        const nodeMessage = subgraph.messages.get(`${nodeId}:output`);
        const nodeStream = subgraph.messages.get(`${nodeId}:stream`);
        const nodeError = subgraph.messages.get(`${nodeId}:error`);
        const nodeInput = subgraph.messages.get(`${nodeId}:input`);

        const model = this.deps.getModel();
        const currentData = model.getNodeData(nodeId) || {};
        const updates: Record<string, any> = {};

        if (nodeStatus) {
            updates.status = nodeStatus;
        }

        if (nodeStream !== undefined) {
            updates.lastMessage = nodeStream;
            updates.outputs = { output: nodeStream };
            updates.isOutputStream = true;
        } else if (nodeMessage !== undefined) {
            updates.lastMessage = nodeMessage;
            updates.outputs = { output: nodeMessage };
        }

        if (nodeInput !== undefined) {
            updates.inputs = nodeInput;
        }

        if (nodeError !== undefined) {
            updates.errorMessage = nodeError;
            updates.status = "error";
        }

        updates._syncedFromSubgraph = true;
        updates._syncedRunnerId = subgraph.runnerId;
        updates._syncedAt = Date.now();

        model.action.updateNodeData(
            nodeId,
            {
                ...currentData,
                ...updates,
            },
            meta,
        );
    }

    private getSubgraphNodeIds(parentNodeId: string): string[] {
        const model = this.deps.getModel();
        const snapshot = model.getSnapshot();
        return snapshot.nodes.filter(n => n.parentId === parentNodeId).map(n => n.id);
    }

    private getSubgraphThreadIndices(parentNodeId: string): number[] {
        const indices: number[] = [];
        this.subgraphInstances.forEach(subgraph => {
            if (subgraph.parentNodeId === parentNodeId && subgraph.threadIndex !== undefined) {
                indices.push(subgraph.threadIndex);
            }
        });
        return indices;
    }

    private activateSubgraphInstance(
        targetSubgraph: SubgraphInstance,
        options: {
            updateParentState: boolean;
            parentNodeId?: string;
            threadIndex?: number;
        },
    ): boolean {
        this.subgraphInstances.forEach(subgraph => {
            if (subgraph.runnerId !== targetSubgraph.runnerId) {
                subgraph.isActive = false;
            }
        });

        targetSubgraph.isActive = true;

        if (options.updateParentState && options.parentNodeId !== undefined) {
            const meta: CommandMeta = { skipHistory: true };
            const model = this.deps.getModel();
            const currentData = model.getNodeData(options.parentNodeId) || {};
            model.action.updateNodeData(
                options.parentNodeId,
                {
                    ...currentData,
                    activeThreadIndex: options.threadIndex,
                    activeRunnerId: targetSubgraph.runnerId,
                },
                meta,
            );
        }

        this.syncAllSubgraphNodesToCanvas(targetSubgraph);

        return true;
    }

    private syncAllSubgraphNodesToCanvas(subgraph: SubgraphInstance) {
        const meta: CommandMeta = { skipHistory: true };
        const subgraphNodeIds = this.getSubgraphNodeIds(subgraph.parentNodeId);

        if (subgraphNodeIds.length === 0) {
            console.warn("Subgraph has no child nodes:", subgraph.parentNodeId);
            return;
        }

        subgraphNodeIds.forEach(nodeId => {
            this.syncNodeStateToCanvas(subgraph, nodeId, meta);
        });
    }
}
