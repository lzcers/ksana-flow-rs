import { BehaviorSubject, filter, map, type Observable, type Subscription } from 'rxjs';
import {
  createWorkflowModel,
  type CommandMeta,
  type WorkflowModelInterface,
} from '@/model/workflow';
import type { NodeStatus, WorkflowStatus } from '@/model/workflow/types';
import { RxFlowEvent, type FlowControlEvent, type FlowEvent, type FlowNodeMsgEvent, type FlowNodeStatusEvent, type FlowNodeStatusEventType } from '@/model/flowEvent';
import { isFlowNodeStatusEvent, isFlowControlEvent, isFlowNodeMsgEvent } from '@/model/flowEvent/RxFlowEvent';

// spaceId:workflowId or spaceId:draft
export type GraphKey = string;

export function makeGraphKey(spaceId: string, workflowId: number): GraphKey {
  if (workflowId == null) return `${spaceId}:draft`;
  return `${spaceId}:${workflowId}`;
}

export type WorkflowRuntimeState = {
  activeGraphKey: GraphKey | null;
  activeWorkflowId: number | null;
  activeRunId: string | null;
  activeWorkflowStatus: WorkflowStatus;
  workflowStatuses: Record<number, WorkflowStatus>;
};


class ModelInstance {
  model: WorkflowModelInterface;
  spaceId: string;
  workflowId: number;
  workflowStatus: WorkflowStatus;
  runId: string | null;
  rxFlowEvent$: RxFlowEvent;
  private notifyChange: () => void;

  constructor(
    model: WorkflowModelInterface,
    rxFlowEvent$: RxFlowEvent,
    spaceId: string,
    workflowId: number,
    workflowStatus: WorkflowStatus,
    runId: string | null,
    notifyChange: () => void,
  ) {
    this.model = model;
    this.spaceId = spaceId;
    this.workflowId = workflowId;
    this.workflowStatus = workflowStatus;
    this.runId = runId;
    this.rxFlowEvent$ = rxFlowEvent$;
    this.notifyChange = notifyChange;

    this.rxFlowEvent$.getSource$()
      .pipe(
        filter((e) => e.runId === this.runId && e.runnerKind === "Root"),
      )
      .subscribe((e) => {
        this.applyFlowEvent(e.event);
      });
  }

  setRunId(runId: string | null) {
    this.runId = runId;
    this.notifyChange();
  }
  setWorkflowStatus(status: WorkflowStatus) {
    this.workflowStatus = status;
    this.notifyChange();
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
          isOutputStream: false,
          outputs: { output: msg },
        }, meta);
        break;
      case "NodeStreamNextMessage":
        const prev = this.model.getSnapshot().nodes.find(n => n.id === nodeId)?.data?.lastMessage;
        const lastMessage =
          typeof msg === 'string'
            ? `${typeof prev === 'string' ? prev : ''}${msg}`
            : msg;
        this.model.action.updateNodeData(nodeId, {
          lastMessage
        }, meta);
        break;
      case "NodeError":
        this.model.action.updateNodeData(nodeId, {
          errorMessage: msg,
          status: 'error'
        }, meta);
    }
  }

  applyFlowNodeStatusEvent(event: FlowNodeStatusEvent) {
    const meta: CommandMeta = { skipHistory: true }
    // 根据控制事件类型映射到 WorkflowStatus
    const eventTypeToNodeStatus: Record<FlowNodeStatusEventType, NodeStatus> = {
      "NodeStarted": "running",
      "NodeStreamStarted": "running",
      "NodeCompleted": "completed",
    }
    const { nodeId, type } = event;
    this.model.action.updateNodeData(nodeId, {
      status: eventTypeToNodeStatus[type],
      isOuputStream: type === "NodeStreamStarted",
    }, meta)
  }

  applyFlowControlEvent(event: FlowControlEvent) {
    // 根据控制事件类型映射到 WorkflowStatus
    let status = "idle" as WorkflowStatus;
    switch (event.type) {
      case 'FlowPaused': status = 'paused'; break;
      case 'FlowResumed': status = 'running'; break;
      case 'FlowStopped': status = 'idle'; break;
      case 'FlowFinished': status = 'idle'; break;
      default: status = 'idle'; break;
    }
    this.workflowStatus = status;
    this.notifyChange();
  }
}

const defaultRuntimeState: WorkflowRuntimeState = {
  activeGraphKey: null,
  activeWorkflowId: null,
  activeRunId: null,
  activeWorkflowStatus: 'idle',
  workflowStatuses: {},
};

// 管理多个 WorkflowModelInterface 实例
export class WorkflowManager {
  private models = new Map<GraphKey, ModelInstance>();
  private rxFlowEvent$: RxFlowEvent = new RxFlowEvent();
  private activeGraphKey: GraphKey | null = null;
  private runtimeState$ = new BehaviorSubject<WorkflowRuntimeState>(defaultRuntimeState);

  get active(): WorkflowModelInterface | undefined {
    return this.activeGraphKey ? this.getModelInstance(this.activeGraphKey)?.model : undefined;
  }



  flowEventForRunId$(runId: string): Observable<FlowEvent> {
    return this.rxFlowEvent$.getSource$().pipe(
      filter((msg) => msg.runId === runId && msg.runnerKind === 'Root'),
      map((msg) => msg.event),
    );
  }

  flowEventForNodeId$(runId: string, nodeId: string): Observable<FlowEvent> {
    return this.rxFlowEvent$.getSource$().pipe(
      filter((msg) => msg.runId === runId && msg.runnerKind === 'Root'),
      map((msg) => msg.event),
      filter((evt) => ('nodeId' in evt ? evt.nodeId === nodeId : false)),
    );
  }
  setRunId(graphKey: GraphKey, runId: string | null): void {
    const entry = this.models.get(graphKey);
    if (!entry) return;
    entry.setRunId(runId);
  }

  setWorkflowStatus(graphKey: GraphKey, status: WorkflowStatus): void {
    const entry = this.models.get(graphKey);
    if (!entry) return;
    entry.setWorkflowStatus(status);
  }

  getActiveGraphKey() {
    return this.activeGraphKey;
  }

  getOrCreate(graphKey: GraphKey): ModelInstance {
    const existing = this.models.get(graphKey);
    if (existing) return existing;
    const model = createWorkflowModel();
    const [spaceId, workflowIdRaw] = graphKey.split(':');
    const workflowId = Number(workflowIdRaw);
    const modelInstance = new ModelInstance(
      model,
      this.rxFlowEvent$,
      spaceId,
      workflowId,
      "idle",
      null,
      () => this.emitRuntimeState(),
    );
    this.models.set(graphKey, modelInstance);
    this.emitRuntimeState();
    return modelInstance;
  }

  getModelInstance(graphKey: GraphKey) {
    return this.models.get(graphKey)
  }

  activate(graphKey: GraphKey): void {
    const entry = this.models.get(graphKey);
    const spaceId = graphKey.split(':')[0];
    this.rxFlowEvent$.connectWebSocket(spaceId);
    if (!entry) return;
    this.activeGraphKey = graphKey;
    this.emitRuntimeState();
  }

  destroy(graphKey: GraphKey): void {
    const entry = this.models.get(graphKey);
    if (!entry) return;
    entry.model.destroy();
    this.models.delete(graphKey);
    if (this.activeGraphKey === graphKey) {
      this.activeGraphKey = null;
    }
    this.emitRuntimeState();
  }

  connectWebSocket(spaceId: string): void {
    this.rxFlowEvent$.connectWebSocket(spaceId);
  }

  disconnectWebSocket(): void {
    this.rxFlowEvent$.disconnectWebSocket();
  }

  getRuntimeStateSnapshot(): WorkflowRuntimeState {
    return this.runtimeState$.value;
  }

  subscribeRuntimeState(listener: (state: WorkflowRuntimeState) => void): Subscription {
    return this.runtimeState$.subscribe(listener);
  }

  private emitRuntimeState(): void {
    const workflowStatuses: Record<number, WorkflowStatus> = {};
    for (const instance of this.models.values()) {
      if (Number.isFinite(instance.workflowId)) {
        workflowStatuses[instance.workflowId] = instance.workflowStatus;
      }
    }

    const activeGraphKey = this.activeGraphKey;
    const activeInstance = activeGraphKey ? this.models.get(activeGraphKey) : undefined;
    const activeWorkflowId =
      activeInstance && Number.isFinite(activeInstance.workflowId) ? activeInstance.workflowId : null;

    this.runtimeState$.next({
      activeGraphKey,
      activeWorkflowId,
      activeRunId: activeInstance?.runId ?? null,
      activeWorkflowStatus: activeInstance?.workflowStatus ?? 'idle',
      workflowStatuses,
    });
  }
}

declare global {
  var __ksanaWorkflowModelManager: WorkflowManager | undefined;
}

export const workflowManager =
  globalThis.__ksanaWorkflowModelManager ?? (globalThis.__ksanaWorkflowModelManager = new WorkflowManager());
