import type { StateCreator } from 'zustand';
import type { StoreState, Execution } from './types';
import * as api from '@/api';
import { toBlueprint } from '@/model/workflow/adapters';
import { rxWorkflowModel } from '.';
import { createFlowEventModel } from '@/model/flowEvent';
import { EMPTY, type Observable, type Subscription } from 'rxjs';
import type { FlowEvent } from '@/model/flowEvent/types';

export const createExecution: StateCreator<StoreState, [], [], Execution> = (set, get) => {
  const rxFlowEventModel = createFlowEventModel();
  let workflowStatusSubscription: Subscription | null = null;
  let flowEventSubscription: Subscription | null = null;

  const syncWorkflowStatusForRun = (runId: string | null) => {
    workflowStatusSubscription?.unsubscribe();
    workflowStatusSubscription = null;
    if (!runId) return;
    workflowStatusSubscription = rxFlowEventModel.workflowStatusForRunId$(runId).subscribe((status) => {
      get().setWorkflowStatus(status);
      // 工作流完成或者终止时重置所有节点状态
      if (status === 'idle') {
        rxWorkflowModel.dispatch({
          type: 'RESET_ALL_NODE_STATUS',
        });
      }
    });
  };

  const syncFlowEventsForRun = (runId: string | null) => {
    flowEventSubscription?.unsubscribe();
    flowEventSubscription = null;
    if (!runId) return;
    flowEventSubscription = rxFlowEventModel.flowEventForRunId$(runId).subscribe((event) => {
      get().applyExecutionEvent(event);
    });
  };

  return {
    // ===== State =====
    workflowStatus: 'idle',
    workflowStatuses: {},
    runIdToWorkflowId: {},
    currentRunId: null,

    flowEventForRunId$: (runId: string) => {
      return rxFlowEventModel.flowEventForRunId$(runId);
    },
    flowEventForNodeId$: (nodeId: string) => {
      const runId = get().currentRunId;
      if (!runId) return EMPTY as Observable<FlowEvent>;
      return rxFlowEventModel.flowEventForNodeId$(nodeId)(
        rxFlowEventModel.flowEventForRunId$(runId)
      );
    },

    // ===== State Setters =====
    setWorkflowStatus: (status) => {
      set((state) => {
        const currentWorkflowId = state.currentWorkflowId;
        if (currentWorkflowId == null) {
          return { workflowStatus: status } as Partial<StoreState>;
        }
        return {
          workflowStatus: status,
          workflowStatuses: { ...state.workflowStatuses, [currentWorkflowId]: status },
        } as Partial<StoreState>;
      });
    },

    setWorkflowStatuses: (statuses) => set({ workflowStatuses: statuses }),

    setCurrentRunId: (runId) => {
      set((state) => {
        const currentWorkflowId = state.currentWorkflowId;
        if (!runId || currentWorkflowId == null) {
          return { currentRunId: runId } as Partial<StoreState>;
        }
        return {
          currentRunId: runId,
          runIdToWorkflowId: { ...state.runIdToWorkflowId, [runId]: currentWorkflowId },
        } as Partial<StoreState>;
      });
      syncWorkflowStatusForRun(runId);
      syncFlowEventsForRun(runId);
    },

    // ===== WebSocket =====
    initializeWebSocket: () => {
      const { currentSpaceId } = get();
      if (!currentSpaceId) return () => { };

      rxFlowEventModel.connectWebSocket(currentSpaceId);

      return () => {
        workflowStatusSubscription?.unsubscribe();
        workflowStatusSubscription = null;
        flowEventSubscription?.unsubscribe();
        flowEventSubscription = null;
        rxFlowEventModel.disconnectWebSocket();
      };
    },

    // ===== Workflow Actions =====
    runWorkflow: async () => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error, setWorkflowStatus, setCurrentRunId } = get();
      if (!currentSpaceId) return;

      const blueprint = toBlueprint(nodes, edges);

      try {
        rxWorkflowModel.dispatch({ type: 'RESET_EXECUTION_STATE', payload: {} });
        setWorkflowStatus('running');

        const res = await api.runWorkflow(currentSpaceId, blueprint as never, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          setCurrentRunId(res.run_id);
        }
        success('Workflow started');
      } catch (e) {
        console.error("Failed to run workflow", e);
        error('Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
        setWorkflowStatus('idle');
        setCurrentRunId(null);
      }
    },

    pauseWorkflow: async () => {
      const { currentSpaceId, currentRunId, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.pauseWorkflow(currentSpaceId, currentRunId);
      } catch (e) {
        console.error("Failed to pause workflow", e);
        error("Failed to pause workflow");
      }
    },

    resumeWorkflow: async () => {
      const { currentSpaceId, currentRunId, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.resumeWorkflow(currentSpaceId, currentRunId);
      } catch (e) {
        console.error("Failed to resume workflow", e);
        error("Failed to resume workflow");
      }
    },

    stopWorkflow: async () => {
      const { currentSpaceId, currentRunId, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.stopWorkflow(currentSpaceId, currentRunId);
      } catch (e) {
        console.error("Failed to stop workflow", e);
        error("Failed to stop workflow");
      }
    },

    runNode: async (nodeId: string) => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error, setWorkflowStatus, setCurrentRunId } = get();
      if (!currentSpaceId) return;
      const blueprint = toBlueprint(nodes, edges);

      try {
        const res = await api.runNode(currentSpaceId, blueprint as never, nodeId, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          setCurrentRunId(res.run_id);
          setWorkflowStatus('running');
        }
        success(`Node ${nodeId} execution started`);
      } catch (e) {
        console.error(`Failed to run node ${nodeId}`, e);
        error(`Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
        setWorkflowStatus('idle');
      }
    },
  };
};
