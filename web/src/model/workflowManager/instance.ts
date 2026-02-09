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
    threadIndex?: number;        // Map 节点中的线程索引
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

            // 推断 threadIndex（对于 Map 节点）
            // 根据当前 Map 节点已有的子图数量来确定 threadIndex
            let threadIndex = 0;
            const existingThreads = this.getMapThreadIndices(parentNodeId);
            if (existingThreads.length > 0) {
                // 找到最大的 threadIndex 并 +1
                threadIndex = Math.max(...existingThreads) + 1;
            }

            subgraph = {
                runnerId,
                parentNodeId,
                parentRunnerId,
                threadIndex,
                status: 'idle',
                nodeStatuses: new Map(),
                messages: new Map(),
                subgraphPath: msg.subgraphPath || [],
            };
            this.subgraphInstances.set(runnerId, subgraph);
            // 初始化父节点的子图状态存储
            this.initParentNodeSubgraphState(parentNodeId, runnerId, threadIndex);
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
    private initParentNodeSubgraphState(parentNodeId: string, runnerId: number, threadIndex: number = 0) {
        // const meta: CommandMeta = { skipHistory: true };
        // const currentData = this.model.getNodeData(parentNodeId) || {};

        // // 确保有 subgraphStatuses 字段存储子图状态
        // const subgraphStatuses = currentData.subgraphStatuses || {};
        // subgraphStatuses[runnerId] = {
        //     runnerId,
        //     threadIndex,
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

    // ========== Map 节点线程管理新方法 ==========

    /**
     * 获取 Map 节点的所有线程索引列表
     */
    private getMapThreadIndices(mapNodeId: string): number[] {
        const indices: number[] = [];
        this.subgraphInstances.forEach((subgraph) => {
            if (subgraph.parentNodeId === mapNodeId && subgraph.threadIndex !== undefined) {
                indices.push(subgraph.threadIndex);
            }
        });
        return indices;
    }

    /**
     * 获取 Map 节点的所有子图线程
     */
    getMapThreads(mapNodeId: string): Array<{
        runnerId: number;
        threadIndex: number;
        status: WorkflowStatus;
    }> {
        const threads: Array<{ runnerId: number; threadIndex: number; status: WorkflowStatus }> = [];

        this.subgraphInstances.forEach((subgraph, runnerId) => {
            if (subgraph.parentNodeId === mapNodeId) {
                threads.push({
                    runnerId,
                    threadIndex: subgraph.threadIndex ?? 0,
                    status: subgraph.status,
                });
            }
        });

        return threads.sort((a, b) => a.threadIndex - b.threadIndex);
    }

    /**
     * 设置当前观测的线程
     */
    setActiveThread(mapNodeId: string, threadIndex: number): boolean {
        const threads = this.getMapThreads(mapNodeId);
        const targetThread = threads.find(t => t.threadIndex === threadIndex);

        if (!targetThread) return false;

        const subgraph = this.subgraphInstances.get(targetThread.runnerId);
        if (!subgraph) return false;

        // 更新节点的 activeThreadIndex
        const meta: CommandMeta = { skipHistory: true };
        const currentData = this.model.getNodeData(mapNodeId) || {};
        this.model.action.updateNodeData(mapNodeId, {
            ...currentData,
            activeThreadIndex: threadIndex,
            activeRunnerId: targetThread.runnerId,
        }, meta);

        // 同步该线程的状态到画布
        this.syncSubgraphToCanvas(subgraph);

        return true;
    }

    /**
     * 将子图状态同步到画布节点
     */
    private syncSubgraphToCanvas(subgraph: SubgraphInstance) {
        const meta: CommandMeta = { skipHistory: true };
        const mapNodeId = subgraph.parentNodeId;

        // 获取子图节点的 ID 列表
        // 从 Map 节点的配置中获取 subgraph_node_ids
        const mapNodeData = this.model.getNodeData(mapNodeId) || {};
        const subgraphNodeIds: string[] = mapNodeData.config?.subgraph_node_ids || [];

        if (subgraphNodeIds.length === 0) {
            console.warn('Map node has no subgraph_node_ids configured:', mapNodeId);
            return;
        }

        // 将子图状态同步到每个子图节点
        subgraphNodeIds.forEach(nodeId => {
            const nodeStatus = subgraph.nodeStatuses.get(nodeId);
            const nodeMessage = subgraph.messages.get(`${nodeId}:output`);
            const nodeError = subgraph.messages.get(`${nodeId}:error`);
            const nodeInput = subgraph.messages.get(`${nodeId}:input`);

            // 只有当有状态或消息时才更新
            if (nodeStatus || nodeMessage || nodeError || nodeInput) {
                const currentData = this.model.getNodeData(nodeId) || {};
                const updates: Record<string, any> = {};

                // 更新状态
                if (nodeStatus) {
                    updates.status = nodeStatus;
                }

                // 更新消息
                if (nodeMessage) {
                    updates.lastMessage = nodeMessage;
                    updates.outputs = { output: nodeMessage };
                }

                // 更新输入
                if (nodeInput) {
                    updates.inputs = nodeInput;
                }

                // 更新错误
                if (nodeError) {
                    updates.errorMessage = nodeError;
                    updates.status = 'error';
                }

                // 添加同步标记
                updates._syncedFromSubgraph = true;
                updates._syncedRunnerId = subgraph.runnerId;
                updates._syncedAt = Date.now();

                this.model.action.updateNodeData(nodeId, {
                    ...currentData,
                    ...updates,
                }, meta);
            }
        });

        // 更新 Map 节点的同步状态
        this.model.action.updateNodeData(mapNodeId, {
            ...mapNodeData,
            isSyncedToCanvas: true,
            lastSyncedAt: Date.now(),
            syncedRunnerId: subgraph.runnerId,
        }, meta);
    }
}