import { filter, type Subscription } from "rxjs";
import type { FlowControlEvent, FlowEvent, FlowNodeMsgEvent, FlowNodeStatusEvent, FlowNodeStatusEventType, RxFlowEvent, WebSocketFlowMessage, SubgraphFrame } from "../flowEvent";
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
}

export function makeGraphKey(spaceId: string, workflowId: number): GraphKey {
    if (workflowId == null) return `${spaceId}:draft`;
    return `${spaceId}:${workflowId}`;
}

// spaceId:workflowId or spaceId:draft
export type GraphKey = string;

export class ModelInstance {
    graphKey: GraphKey;
    model: WorkflowModelInterface;
    spaceId: string;
    workflowId: number;
    runId: string | null;
    status: WorkflowStatus = 'idle';
    rxFlowEvent$: RxFlowEvent;
    subgraphInstances: Map<number, SubgraphInstance> = new Map();

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
        this.flowEventSubscription = this.rxFlowEvent$.getSource$()
            .pipe(
                filter((e) => this.shouldProcessEvent(e)),
            )
            .subscribe((e) => {
                this.routeEvent(e);
            });
    }

    setRunId(runId: string | null) {
        this.runId = runId;
    }

    destroy() {
        this.flowEventSubscription.unsubscribe();
        // 清理所有子图实例
        this.subgraphInstances.clear();
        this.model.destroy();
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
        const meta: CommandMeta = { skipHistory: true }
        const { nodeId, type, msg } = event;
        switch (type) {
            case "NodeInMessage":
                this.model.action.updateNodeData(nodeId, {
                    lastMessage: msg,
                    inputs: msg
                }, meta);
                break;
            case "NodeOutMessage":
                this.model.action.updateNodeData(nodeId, {
                    lastMessage: msg,
                    outputs: { output: msg },
                }, meta);
                break;
            case "NodeStreamNextMessage":
                const snapshot = this.model.getSnapshot();
                const node = snapshot.nodes.find((n) => n.id === nodeId);
                const prev = node?.data?.lastMessage;
                const lastMessage =
                    typeof msg === 'string'
                        ? `${typeof prev === 'string' ? prev : ''}${msg}`
                        : msg;
                this.model.action.updateNodeData(nodeId, { lastMessage }, meta);
                break;
            case "NodeError":
                this.model.action.updateNodeData(nodeId, {
                    errorMessage: msg,
                    status: 'error',
                }, meta);
                break;
        }
    }

    applyFlowNodeStatusEvent(event: FlowNodeStatusEvent) {
        const meta: CommandMeta = { skipHistory: true }
        const eventTypeToNodeStatus: Record<FlowNodeStatusEventType, NodeStatus> = {
            "NodeStarted": "running",
            "NodeStreamStarted": "running",
            "NodeCompleted": "completed",
        }
        const { nodeId, type } = event;
        const updates: Record<string, any> = {
            status: eventTypeToNodeStatus[type],
        };
        if (type === 'NodeStreamStarted') {
            updates.isOutputStream = true;
            updates.lastMessage = '';
            updates.errorMessage = undefined;
        } else if (type === 'NodeStarted') {
            updates.isOutputStream = false;
            updates.errorMessage = undefined;
        } else if (type === 'NodeCompleted') {
        }
        this.model.action.updateNodeData(nodeId, updates, meta)
    }

    applyFlowControlEvent(event: FlowControlEvent) {
        const meta: CommandMeta = { skipHistory: true }
        // 根据控制事件类型映射到 WorkflowStatus
        let status = "idle" as WorkflowStatus;
        switch (event.type) {
            case 'FlowStarted': status = 'running'; break;
            case 'FlowPaused': status = 'paused'; break;
            case 'FlowResumed': status = 'running'; break;
            case 'FlowStopped': status = 'idle'; break;
            case 'FlowFinished': status = 'idle'; break;
            default: status = 'idle'; break;
        }
        if (event.type === "FlowStopped") {
            this.model.action.resetAllNodeStatus(meta);
        }
        this.status = status;
        this.notifyWorkflowStatusChange(this.graphKey, this.workflowId, this.runId, this.status);
    }

    // ========== 子图事件处理新方法 ==========

    /**
     * 清理子图实例（当工作流停止时）
     */
    clearSubgraphInstances() {
        this.subgraphInstances.clear();

        // 清除所有节点的子图状态
        const meta: CommandMeta = { skipHistory: true };
        const snapshot = this.model.getSnapshot();

        snapshot.nodes.forEach(node => {
            if (node.data?.subgraphStatuses || node.data?.hasSubgraph) {
                const newData = { ...node.data };
                delete newData.subgraphStatuses;
                delete newData.subgraphAggregateStatus;
                delete newData.subgraphProgress;
                delete newData.hasSubgraph;
                this.model.action.updateNodeData(node.id, newData, meta);
            }
        });
    }

    /**
     * 判断是否应该处理该事件
     */
    private shouldProcessEvent(e: WebSocketFlowMessage): boolean {
        // 根 Runner 的事件（当前运行）
        if (e.runId === this.runId && e.runnerKind === "Root") {
            return true;
        }

        // 已知的子图 Runner 事件
        if (e.runnerId !== undefined && this.subgraphInstances.has(e.runnerId)) {
            return true;
        }

        // 检查是否是当前运行的新子图事件
        if (e.parentRunnerId !== undefined && e.runId === this.runId) {
            // 父 Runner 是当前运行的根 Runner 或已知子图
            if (e.parentRunnerId === 1 || this.subgraphInstances.has(e.parentRunnerId)) {
                return true;
            }
        }
        return false;
    }

    /**
     * 路由事件到正确的处理器
     */
    private routeEvent(e: WebSocketFlowMessage) {
        if (e.runnerKind === "Root") {
            // 根工作流事件
            this.applyFlowEvent(e.event);
        } else {
            // 子图事件（Subgraph 或 MapItem）
            this.applySubgraphEvent(e);
        }
    }

    /**
     * 处理子图事件
     */
    private applySubgraphEvent(msg: WebSocketFlowMessage) {
        const runnerId = msg.runnerId;
        if (runnerId === undefined) return;

        // 获取或创建子图实例
        let subgraph = this.subgraphInstances.get(runnerId);
        if (!subgraph) {
            // 新子图，创建实例
            const parentNodeId = msg.parentNodeId;
            const parentRunnerId = msg.parentRunnerId;

            if (parentNodeId === undefined || parentRunnerId === undefined) {
                console.warn('Subgraph event missing parent info:', msg);
                return;
            }

            subgraph = {
                runnerId,
                parentNodeId,
                parentRunnerId,
                status: 'idle',
                nodeStatuses: new Map(),
                messages: new Map(),
                subgraphPath: msg.subgraphPath || [],
            };
            this.subgraphInstances.set(runnerId, subgraph);
            // 初始化父节点的子图状态存储
            this.initParentNodeSubgraphState(parentNodeId, runnerId);
        }

        // 处理事件
        const event = msg.event;

        if (isFlowNodeMsgEvent(event)) {
            this.applySubgraphNodeMsgEvent(subgraph, event);
        } else if (isFlowNodeStatusEvent(event)) {
            this.applySubgraphNodeStatusEvent(subgraph, event);
        } else if (isFlowControlEvent(event)) {
            this.applySubgraphControlEvent(subgraph, event);
        }
    }

    /**
     * 应用子图节点消息事件
     */
    private applySubgraphNodeMsgEvent(
        subgraph: SubgraphInstance,
        event: FlowNodeMsgEvent,
    ) {
        const { nodeId, type, msg: message } = event;

        // 存储到子图实例
        switch (type) {
            case 'NodeInMessage':
                subgraph.messages.set(`${nodeId}:input`, message);
                break;
            case 'NodeOutMessage':
                subgraph.messages.set(`${nodeId}:output`, message);
                break;
            case 'NodeStreamNextMessage':
                const prev = subgraph.messages.get(`${nodeId}:stream`) || '';
                subgraph.messages.set(`${nodeId}:stream`,
                    typeof message === 'string' ? prev + message : message
                );
                break;
            case 'NodeError':
                subgraph.messages.set(`${nodeId}:error`, message);
                break;
        }

        // 同步到父节点状态
        this.syncSubgraphStateToParent(subgraph);
    }

    /**
     * 应用子图节点状态事件
     */
    private applySubgraphNodeStatusEvent(
        subgraph: SubgraphInstance,
        event: FlowNodeStatusEvent,
    ) {
        const { nodeId, type } = event;
        const eventTypeToNodeStatus: Record<FlowNodeStatusEventType, NodeStatus> = {
            "NodeStarted": "running",
            "NodeStreamStarted": "running",
            "NodeCompleted": "completed",
        };

        const status = eventTypeToNodeStatus[type];
        subgraph.nodeStatuses.set(nodeId, status);

        // 同步到父节点状态
        this.syncSubgraphStateToParent(subgraph);
    }

    /**
     * 应用子图控制事件
     */
    private applySubgraphControlEvent(
        subgraph: SubgraphInstance,
        event: FlowControlEvent
    ) {
        let status: WorkflowStatus = 'idle';
        switch (event.type) {
            case 'FlowStarted': status = 'running'; break;
            case 'FlowPaused': status = 'paused'; break;
            case 'FlowResumed': status = 'running'; break;
            case 'FlowStopped': status = 'idle'; break;
            case 'FlowFinished': status = 'idle'; break;
        }
        subgraph.status = status;

        if (event.type === 'FlowStopped') {
            subgraph.nodeStatuses.clear();
        }

        this.syncSubgraphStateToParent(subgraph);
    }


    /**
     * 初始化父节点的子图状态存储
     */
    private initParentNodeSubgraphState(parentNodeId: string, runnerId: number) {
        // 暂时是没必要的
        // const meta: CommandMeta = { skipHistory: true };
        // const currentData = this.model.getNodeData(parentNodeId) || {};

        // // 确保有 subgraphStatuses 字段存储子图状态
        // const subgraphStatuses = currentData.subgraphStatuses || {};
        // subgraphStatuses[runnerId] = {
        //     status: 'idle',
        //     nodeCount: 0,
        //     completedCount: 0,
        //     nodeStatuses: {},
        //     startedAt: Date.now(),
        // };

        // this.model.action.updateNodeData(parentNodeId, {
        //     ...currentData,
        //     subgraphStatuses,
        //     hasSubgraph: true,
        // }, meta);
    }

    /**
     * 同步子图状态到父节点
     */
    private syncSubgraphStateToParent(subgraph: SubgraphInstance) {
        // const meta: CommandMeta = { skipHistory: true };
        // const parentNodeId = subgraph.parentNodeId;
        // const currentData = this.model.getNodeData(parentNodeId) || {};

        // // 统计子图节点状态
        // const nodeStatuses: Record<string, NodeStatus> = {};
        // let completedCount = 0;
        // let errorCount = 0;
        // let runningCount = 0;

        // subgraph.nodeStatuses.forEach((status, nodeId) => {
        //     nodeStatuses[nodeId] = status;
        //     if (status === 'completed') completedCount++;
        //     else if (status === 'error') errorCount++;
        //     else if (status === 'running') runningCount++;
        // });

        // const totalNodes = subgraph.nodeStatuses.size;

        // // 更新父节点的 subgraphStatuses
        // const subgraphStatuses = currentData.subgraphStatuses || {};
        // subgraphStatuses[subgraph.runnerId] = {
        //     status: subgraph.status,
        //     nodeCount: totalNodes,
        //     completedCount,
        //     errorCount,
        //     runningCount,
        //     nodeStatuses,
        //     messages: Object.fromEntries(subgraph.messages),
        //     updatedAt: Date.now(),
        // };

        // // 计算聚合状态（用于在画布上快速展示）
        // let aggregateStatus: NodeStatus = 'idle';
        // if (errorCount > 0) aggregateStatus = 'error';
        // else if (runningCount > 0) aggregateStatus = 'running';
        // else if (completedCount === totalNodes && totalNodes > 0) aggregateStatus = 'completed';

        // this.model.action.updateNodeData(parentNodeId, {
        //     ...currentData,
        //     subgraphStatuses,
        //     subgraphAggregateStatus: aggregateStatus,
        //     subgraphProgress: totalNodes > 0 ? completedCount / totalNodes : 0,
        // }, meta);
    }
}