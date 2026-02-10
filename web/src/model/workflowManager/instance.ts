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
    isActive: boolean;           // 是否是当前激活的线程（用于画布持续更新）
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

    /**
     * 预激活状态存储：用于子图尚未创建但用户已请求激活的场景
     * Key: `${parentNodeId}:${threadIndex}`
     */
    private pendingSubgraphActivation: Map<string, { threadIndex: number; parentNodeId: string }> = new Map();

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

    // ========== 子图事件处理（重构后）==========

    /**
     * 清理子图实例（当工作流停止时）
     */
    clearSubgraphInstances() {
        this.subgraphInstances.clear();
        this.pendingSubgraphActivation.clear(); // 清理预激活状态

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
                delete newData.activeThreadIndex;
                delete newData.activeRunnerId;
                delete newData.subgraphStatus;
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
     * 处理子图事件（统一入口）
     */
    private applySubgraphEvent(msg: WebSocketFlowMessage) {
        const runnerId = msg.runnerId;
        if (runnerId === undefined) return;

        // 获取或创建子图实例
        let subgraph = this.subgraphInstances.get(runnerId);
        if (!subgraph) {
            subgraph = this.createSubgraphInstance(msg, runnerId);
            if (!subgraph) return;
        }

        // 统一处理事件：存储 + 条件同步
        this.processSubgraphEvent(subgraph, msg.event);
    }

    /**
     * 创建子图实例
     */
    private createSubgraphInstance(msg: WebSocketFlowMessage, runnerId: number): SubgraphInstance | undefined {
        const parentNodeId = msg.parentNodeId;
        const parentRunnerId = msg.parentRunnerId;

        if (parentNodeId === undefined || parentRunnerId === undefined) {
            console.warn('Subgraph event missing parent info:', msg);
            return undefined;
        }

        // 推断 threadIndex（对于 Map 节点）
        let threadIndex = 0;
        const existingThreads = this.getMapThreadIndices(parentNodeId);
        if (existingThreads.length > 0) {
            threadIndex = Math.max(...existingThreads) + 1;
        }

        const subgraph: SubgraphInstance = {
            runnerId,
            parentNodeId,
            parentRunnerId,
            threadIndex,
            status: 'idle',
            nodeStatuses: new Map(),
            messages: new Map(),
            subgraphPath: msg.subgraphPath || [],
            isActive: false,
        };

        this.subgraphInstances.set(runnerId, subgraph);

        // 检查是否有预激活请求，如果有则自动激活
        this.checkAndApplyPendingActivation(subgraph, parentNodeId, threadIndex);

        return subgraph;
    }

    /**
     * 检查并应用预激活请求
     */
    private checkAndApplyPendingActivation(
        subgraph: SubgraphInstance,
        parentNodeId: string,
        threadIndex: number
    ) {
        const pendingKey = `${parentNodeId}:${threadIndex}`;
        const pendingActivation = this.pendingSubgraphActivation.get(pendingKey);

        if (pendingActivation && pendingActivation.threadIndex === threadIndex) {
            // 自动激活该子图
            this.activateSubgraphInstance(subgraph, {
                updateParentState: true,
                parentNodeId,
                threadIndex,
            });

            // 清除预激活记录
            this.pendingSubgraphActivation.delete(pendingKey);

            // 更新节点状态为运行中
            const meta: CommandMeta = { skipHistory: true };
            const currentData = this.model.getNodeData(parentNodeId) || {};
            this.model.action.updateNodeData(parentNodeId, {
                ...currentData,
                subgraphStatus: 'active',
            }, meta);
        }
    }

    /**
     * 处理子图事件核心逻辑：存储 + 条件同步
     */
    private processSubgraphEvent(subgraph: SubgraphInstance, event: FlowEvent) {
        // 1. 存储事件状态
        this.storeSubgraphEvent(subgraph, event);

        // 2. 如果子图处于激活状态，同步到画布
        if (subgraph.isActive) {
            this.syncSubgraphToCanvas(subgraph, event);
        }
    }

    /**
     * 存储子图事件到实例
     */
    private storeSubgraphEvent(subgraph: SubgraphInstance, event: FlowEvent) {
        if (isFlowNodeMsgEvent(event)) {
            this.storeNodeMsgEvent(subgraph, event);
        } else if (isFlowNodeStatusEvent(event)) {
            this.storeNodeStatusEvent(subgraph, event);
        } else if (isFlowControlEvent(event)) {
            this.storeControlEvent(subgraph, event);
        }
    }

    /**
     * 存储节点消息事件
     */
    private storeNodeMsgEvent(subgraph: SubgraphInstance, event: FlowNodeMsgEvent) {
        const { nodeId, type, msg: message } = event;

        switch (type) {
            case 'NodeInMessage':
                subgraph.messages.set(`${nodeId}:input`, message);
                break;
            case 'NodeOutMessage':
                subgraph.messages.set(`${nodeId}:output`, message);
                break;
            case 'NodeStreamNextMessage': {
                const prev = subgraph.messages.get(`${nodeId}:stream`) || '';
                subgraph.messages.set(`${nodeId}:stream`,
                    typeof message === 'string' ? prev + message : message
                );
                break;
            }
            case 'NodeError':
                subgraph.messages.set(`${nodeId}:error`, message);
                break;
        }
    }

    /**
     * 存储节点状态事件
     */
    private storeNodeStatusEvent(subgraph: SubgraphInstance, event: FlowNodeStatusEvent) {
        const { nodeId, type } = event;
        const eventTypeToNodeStatus: Record<FlowNodeStatusEventType, NodeStatus> = {
            "NodeStarted": "running",
            "NodeStreamStarted": "running",
            "NodeCompleted": "completed",
        };

        const status = eventTypeToNodeStatus[type];
        subgraph.nodeStatuses.set(nodeId, status);
    }

    /**
     * 存储控制事件
     */
    private storeControlEvent(subgraph: SubgraphInstance, event: FlowControlEvent) {
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
    }

    /**
     * 将子图状态同步到画布节点（仅同步，不存储）
     */
    private syncSubgraphToCanvas(subgraph: SubgraphInstance, event: FlowEvent) {
        // 获取子图节点的 ID 列表
        const subgraphNodeIds = this.getSubgraphNodeIds(subgraph.parentNodeId);
        if (subgraphNodeIds.length === 0) return;

        const meta: CommandMeta = { skipHistory: true };

        // 根据事件类型同步对应节点
        if (isFlowNodeMsgEvent(event) || isFlowNodeStatusEvent(event)) {
            this.syncNodeStateToCanvas(subgraph, event.nodeId, meta);
        } else {
            // 控制事件：同步所有节点状态
            subgraphNodeIds.forEach(nodeId => {
                this.syncNodeStateToCanvas(subgraph, nodeId, meta);
            });
        }
    }

    /**
     * 同步单个节点状态到画布
     */
    private syncNodeStateToCanvas(
        subgraph: SubgraphInstance,
        nodeId: string,
        meta: CommandMeta
    ) {
        const nodeStatus = subgraph.nodeStatuses.get(nodeId);
        const nodeMessage = subgraph.messages.get(`${nodeId}:output`);
        const nodeStream = subgraph.messages.get(`${nodeId}:stream`);
        const nodeError = subgraph.messages.get(`${nodeId}:error`);
        const nodeInput = subgraph.messages.get(`${nodeId}:input`);

        const currentData = this.model.getNodeData(nodeId) || {};
        const updates: Record<string, any> = {};

        // 更新状态
        if (nodeStatus) {
            updates.status = nodeStatus;
        }

        // 更新消息（优先使用流式消息）
        if (nodeStream !== undefined) {
            updates.lastMessage = nodeStream;
            updates.outputs = { output: nodeStream };
            updates.isOutputStream = true;
        } else if (nodeMessage !== undefined) {
            updates.lastMessage = nodeMessage;
            updates.outputs = { output: nodeMessage };
        }

        // 更新输入
        if (nodeInput !== undefined) {
            updates.inputs = nodeInput;
        }

        // 更新错误
        if (nodeError !== undefined) {
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

    /**
     * 获取子图包含的节点 ID 列表
     */
    private getSubgraphNodeIds(parentNodeId: string): string[] {
        const snapshot = this.model.getSnapshot();
        return snapshot.nodes
            .filter(n => n.parentId === parentNodeId)
            .map(n => n.id);
    }

    // ========== Map 节点线程管理 ==========

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
     * 设置当前观测的线程（Map 节点）
     * 支持预激活：如果子图尚未创建，记录激活意图，待子图创建时自动激活
     */
    setActiveThread(mapNodeId: string, threadIndex: number): boolean {
        const threads = this.getMapThreads(mapNodeId);
        const targetThread = threads.find(t => t.threadIndex === threadIndex);

        if (!targetThread) {
            // 子图实例不存在，记录预激活意图
            this.pendingSubgraphActivation.set(`${mapNodeId}:${threadIndex}`, {
                threadIndex,
                parentNodeId: mapNodeId,
            });

            // 更新节点状态显示"等待中"
            const meta: CommandMeta = { skipHistory: true };
            const currentData = this.model.getNodeData(mapNodeId) || {};
            this.model.action.updateNodeData(mapNodeId, {
                ...currentData,
                activeThreadIndex: threadIndex,
                subgraphStatus: 'waiting', // 等待子图创建
            }, meta);

            return true; // 返回 true 表示已接受请求
        }

        // 子图存在，正常激活
        const subgraph = this.subgraphInstances.get(targetThread.runnerId);
        if (!subgraph) return false;

        // 清除可能存在的预激活记录
        this.pendingSubgraphActivation.delete(`${mapNodeId}:${threadIndex}`);

        return this.activateSubgraphInstance(subgraph, {
            updateParentState: true,
            parentNodeId: mapNodeId,
            threadIndex,
        });
    }

    // ========== SubgraphNode 激活处理 ==========

    /**
     * 激活 SubgraphNode 对应的子图
     * 当用户展开 SubgraphNode 时调用，用于持续接收该子图的事件
     */
    activateSubgraphNode(nodeId: string): boolean {
        // 查找该 SubgraphNode 对应的子图实例
        let targetSubgraph: SubgraphInstance | undefined;

        for (const subgraph of this.subgraphInstances.values()) {
            if (subgraph.parentNodeId === nodeId) {
                targetSubgraph = subgraph;
                break;
            }
        }

        if (!targetSubgraph) {
            console.warn(`No subgraph instance found for SubgraphNode: ${nodeId}`);
            return false;
        }

        // 使用统一的激活方法
        return this.activateSubgraphInstance(targetSubgraph, {
            updateParentState: false,
        });
    }

    /**
     * 激活子图实例的统一方法
     */
    private activateSubgraphInstance(
        targetSubgraph: SubgraphInstance,
        options: {
            updateParentState: boolean;
            parentNodeId?: string;
            threadIndex?: number;
        }
    ): boolean {
        // 1. 取消其他所有子图的活跃状态
        this.subgraphInstances.forEach((subgraph) => {
            if (subgraph.runnerId !== targetSubgraph.runnerId) {
                subgraph.isActive = false;
            }
        });

        // 2. 激活目标子图
        targetSubgraph.isActive = true;

        // 3. 如果需要，更新父节点状态
        if (options.updateParentState && options.parentNodeId !== undefined) {
            const meta: CommandMeta = { skipHistory: true };
            const currentData = this.model.getNodeData(options.parentNodeId) || {};
            this.model.action.updateNodeData(options.parentNodeId, {
                ...currentData,
                activeThreadIndex: options.threadIndex,
                activeRunnerId: targetSubgraph.runnerId,
            }, meta);
        }

        // 4. 同步当前状态到画布（全量同步所有节点）
        this.syncAllSubgraphNodesToCanvas(targetSubgraph);

        return true;
    }


    /**
     * 将子图所有节点状态同步到画布（全量同步）
     */
    private syncAllSubgraphNodesToCanvas(subgraph: SubgraphInstance) {
        const meta: CommandMeta = { skipHistory: true };
        const subgraphNodeIds = this.getSubgraphNodeIds(subgraph.parentNodeId);

        if (subgraphNodeIds.length === 0) {
            console.warn('Subgraph has no child nodes:', subgraph.parentNodeId);
            return;
        }

        // 同步所有子节点
        subgraphNodeIds.forEach(nodeId => {
            this.syncNodeStateToCanvas(subgraph, nodeId, meta);
        });
    }


}