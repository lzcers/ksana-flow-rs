import type { StateCreator } from 'zustand';
import type { StoreState, Execution } from './types';
import * as api from '@/api';
import { rxWorkflowModel, rxFlowEventModel } from '.';
import { toBlueprint } from '@/model/workflow/adapters';

export const createExecution: StateCreator<StoreState, [], [], Execution> = (set, get) => {
  // 订阅 FlowEvent 状态变化，同步到 zustand
  rxFlowEventModel.state$.subscribe((state: {
    workflowStatus: import('./types').WorkflowStatus;
    workflowStatuses: Record<number, import('./types').WorkflowStatus>;
    currentRunId: string | null;
    runIdToWorkflowId: Record<string, number>;
  }) => {
    set({
      workflowStatus: state.workflowStatus,
      workflowStatuses: state.workflowStatuses,
      currentRunId: state.currentRunId,
      runIdToWorkflowId: state.runIdToWorkflowId,
    });
  });

  return {
    // ===== State =====
    workflowStatus: 'idle',
    workflowStatuses: {},
    runIdToWorkflowId: {},
    currentRunId: null,

    // ===== Observables (从 RxFlowEvent 代理) =====
    events$: rxFlowEventModel.events$,
    eventsForCurrentRun$: rxFlowEventModel.events$, // 使用相同的事件流
    eventsForNode$: rxFlowEventModel.eventsForNode$.bind(rxFlowEventModel),

    // ===== State Setters =====
    setWorkflowStatus: (status) => {
      const { currentWorkflowId } = get();
      if (currentWorkflowId != null) {
        rxFlowEventModel.updateWorkflowStatus(currentWorkflowId, status);
      }
    },

    setWorkflowStatuses: (statuses) => set({ workflowStatuses: statuses }),

    setCurrentRunId: (runId) => {
      const { currentWorkflowId } = get();
      rxFlowEventModel.setCurrentRun(runId, currentWorkflowId);
    },

    // ===== WebSocket =====
    initializeWebSocket: () => {
      const { currentSpaceId } = get();
      if (!currentSpaceId) return () => { };

      const subscription = rxFlowEventModel.connectWebSocket(currentSpaceId).subscribe({
        error: (err: unknown) => console.error('WS Error', err),
      });

      return () => subscription.unsubscribe();
    },

    handleWebSocketMessage: (message) => {
      // WebSocket 消息现在由 RxFlowEvent 自动处理
      // 这个方法保留用于兼容性
      rxFlowEventModel.emitEvent(message.event);
    },

    // ===== Workflow Actions =====
    runWorkflow: async () => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error, setWorkflowStatus } = get();
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
          rxFlowEventModel.setCurrentRun(res.run_id, currentWorkflowId);
          if (currentWorkflowId != null) {
            rxFlowEventModel.updateWorkflowStatus(currentWorkflowId, 'running');
          }
        }
        success('Workflow started');
      } catch (e) {
        console.error("Failed to run workflow", e);
        error('Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
        setWorkflowStatus('idle');
        rxFlowEventModel.setCurrentRun(null, null);
        if (currentWorkflowId != null) {
          rxFlowEventModel.updateWorkflowStatus(currentWorkflowId, 'idle');
        }
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
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error, setWorkflowStatus } = get();
      if (!currentSpaceId) return;
      const blueprint = toBlueprint(nodes, edges);

      try {
        const res = await api.runNode(currentSpaceId, blueprint as never, nodeId, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          rxFlowEventModel.setCurrentRun(res.run_id, currentWorkflowId);
          setWorkflowStatus('running');

          // 设置 activeRunContext 用于跟踪 RunNode 执行
          if (rxFlowEventModel.currentState) {
            rxFlowEventModel.dispatch({
              type: 'SET_ACTIVE_RUN_CONTEXT',
              payload: {
                runId: res.run_id,
                startNodeId: res.start_node ?? nodeId,
                workflowId: currentWorkflowId,
              },
            });
          }

          if (currentWorkflowId != null) {
            rxFlowEventModel.updateWorkflowStatus(currentWorkflowId, 'running');
          }
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