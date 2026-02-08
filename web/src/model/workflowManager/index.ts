import {
  createWorkflowModel,
} from '@/model/workflow';
import type { WorkflowStatus } from '@/model/workflow/types';
import { RxFlowEvent } from '@/model/flowEvent';
import { makeGraphKey, ModelInstance, type GraphKey } from './instance';

export { type GraphKey, makeGraphKey };

export type WorkflowManagerEvent =
  | { type: 'WorkflowStatusChanged'; graphKey: GraphKey; workflowId: number | null; runId: string | null; status: WorkflowStatus }
  | { type: 'ModelDestroyed'; graphKey: GraphKey; workflowId: number | null };

export type WorkflowManagerListener = (event: WorkflowManagerEvent) => void;

// 管理多个 WorkflowModelInterface 实例
export class WorkflowManager {
  private models = new Map<GraphKey, ModelInstance>();
  private rxFlowEvent$: RxFlowEvent = new RxFlowEvent();
  private listeners = new Set<WorkflowManagerListener>();

  getOrCreate(graphKey: GraphKey): ModelInstance {
    const existing = this.models.get(graphKey);
    if (existing) return existing;
    const model = createWorkflowModel();
    const [spaceId, workflowIdRaw] = graphKey.split(':');
    const workflowId = Number(workflowIdRaw);
    const modelInstance = new ModelInstance(
      graphKey,
      model,
      this.rxFlowEvent$,
      spaceId,
      workflowId,
      null,
      (changedGraphKey, changedWorkflowId, runId, status) => {
        this.emit({
          type: 'WorkflowStatusChanged',
          graphKey: changedGraphKey,
          workflowId: changedWorkflowId,
          runId,
          status,
        });
      },
    );
    this.models.set(graphKey, modelInstance);
    return modelInstance;
  }

  getModelInstance(graphKey: GraphKey) {
    return this.models.get(graphKey)
  }

  destroy(graphKey: GraphKey): void {
    const entry = this.models.get(graphKey);
    if (!entry) return;
    entry.destroy();
    const workflowId = Number.isFinite(entry.workflowId) ? entry.workflowId : null;
    this.models.delete(graphKey);
    this.emit({ type: 'ModelDestroyed', graphKey, workflowId });
  }

  connectWebSocket(spaceId: string): void {
    this.rxFlowEvent$.connectWebSocket(spaceId);
  }

  disconnectWebSocket(): void {
    this.rxFlowEvent$.disconnectWebSocket();
  }

  subscribe(listener: WorkflowManagerListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private emit(event: WorkflowManagerEvent): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }
}

declare global {
  var __ksanaWorkflowModelManager: WorkflowManager | undefined;
}

export const workflowManager =
  globalThis.__ksanaWorkflowModelManager ?? (globalThis.__ksanaWorkflowModelManager = new WorkflowManager());
