import { filter } from 'rxjs';
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


class ModelInstance {
  model: WorkflowModelInterface;
  spaceId: string;
  workflowId: number;
  workflowStatus: WorkflowStatus;
  runId: string | null;
  rxFlowEvent$: RxFlowEvent;

  constructor(model: WorkflowModelInterface, rxFlowEvent$: RxFlowEvent, spaceId: string, workflowId: number, workflowStatus: WorkflowStatus, runId: string | null) {
    this.model = model;
    this.spaceId = spaceId;
    this.workflowId = workflowId;
    this.workflowStatus = workflowStatus;
    this.runId = runId;
    this.rxFlowEvent$ = rxFlowEvent$;

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
  }
  setWorkflowStatus(status: WorkflowStatus) {
    this.workflowStatus = status;
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
      meta
    })
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
  }
}
// 管理多个 WorkflowModelInterface 实例
export class WorkflowManager {
  private models = new Map<GraphKey, ModelInstance>();
  private rxFlowEvent$: RxFlowEvent = new RxFlowEvent();
  private activeGraphKey: GraphKey | null = null;

  get active(): WorkflowModelInterface | undefined {
    return this.activeGraphKey ? this.getModelInstance(this.activeGraphKey)?.model : undefined;
  }

  getActiveGraphKey() {
    return this.activeGraphKey;
  }

  getOrCreate(graphKey: GraphKey): ModelInstance {
    const existing = this.models.get(graphKey);
    if (existing) return existing;
    const model = createWorkflowModel();
    const modelInstance = new ModelInstance(model, this.rxFlowEvent$, graphKey.split(':')[0], Number(graphKey.split(':')[1]), "idle", null);
    this.models.set(graphKey, modelInstance);
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
  }


  destroy(graphKey: GraphKey): void {
    const entry = this.models.get(graphKey);
    if (!entry) return;
    entry.model.destroy();
    this.models.delete(graphKey);
  }
}

declare global {
  var __ksanaWorkflowModelManager: WorkflowManager | undefined;
}

export const workflowManager =
  globalThis.__ksanaWorkflowModelManager ?? (globalThis.__ksanaWorkflowModelManager = new WorkflowManager());
