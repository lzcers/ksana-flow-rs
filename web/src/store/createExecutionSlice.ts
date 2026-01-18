import type { StateCreator } from 'zustand';
import type { StoreState, ExecutionSlice } from './types';
import * as api from '../api';
import { updateNodeStatus, updateNodeData, resetWorkflowExecutionState } from '../model';

export const createExecutionSlice: StateCreator<StoreState, [], [], ExecutionSlice> = (set, get) => ({
  workflowStatus: 'idle',
  workflowStatuses: {},
  currentRunId: null,

  setWorkflowStatus: (status) => set({ workflowStatus: status }),
  setWorkflowStatuses: (statuses) => set({ workflowStatuses: statuses }),
  setCurrentRunId: (currentRunId) => set({ currentRunId }),

  initializeWebSocket: () => {
    let ws: WebSocket | null = null;
    let reconnectTimeout: number | null = null;

    const connect = () => {
      ws = new WebSocket('ws://localhost:3000/ws');

      ws.onmessage = (event) => {
        try {
          const wrapper = JSON.parse(event.data);
          const { handleWebSocketMessage } = get();
          handleWebSocketMessage(wrapper);
        } catch (e) {
          console.error("WS parse error", e);
        }
      };

      ws.onclose = () => {
        console.log('WS closed, reconnecting...');
        reconnectTimeout = window.setTimeout(connect, 2000);
      };

      ws.onerror = (err) => {
        console.error('WS error', err);
        ws?.close();
      };
    };

    connect();

    return () => {
      if (ws) ws.close();
      if (reconnectTimeout) clearTimeout(reconnectTimeout);
    };
  },

  handleWebSocketMessage: (wrapper: any) => {
    const { runId, event: msg } = wrapper;
    const { currentRunId, setWorkflowStatus, setCurrentRunId } = get();

    if (runId && currentRunId && runId !== currentRunId) {
      // Ignore messages for other runs for now in terms of state updates
    }

    set(state => {
      let nextState = state;

      // Helper to apply operator and merge
      const apply = (opResult: any) => {
        nextState = { ...nextState, ...opResult };
      };

      // Apply event to state (canvas nodes)
      if (!runId || (runId === state.currentRunId)) {
        if (msg.NodeStarted) {
          const id = msg.NodeStarted;
          apply(updateNodeStatus(nextState, id, 'running'));
        } else if (msg.NodeInMessage) {
          const [id, value] = msg.NodeInMessage;
          apply(updateNodeData(nextState, id, { lastMessage: value }));
        } else if (msg.NodeCompleted) {
          const id = msg.NodeCompleted;
          apply(updateNodeStatus(nextState, id, 'completed'));
        } else if (msg.NodeError) {
          const [id, error] = msg.NodeError;
          apply(updateNodeStatus(nextState, id, 'error', error));
        } else if (msg === 'FlowFinished') {
          // We need to iterate over nodes.
          // nextState.nodes might be updated by previous ops if we had chained them, 
          // but here we are in a single block.
          // We need to map over nodes and update status.
          // Since we don't have a bulk update op, we can do it manually or call updateNodeStatus in loop.
          // Manual update is cleaner here since we are inside set callback and we know structure.
          // BUT we should respect the pattern.
          // Let's assume we can mutate nextState.nodes if we deep clone it first?
          // No, we should use the operators.
          // But iterating and calling operator repeatedly is inefficient (creates many intermediate states).
          // However, for FlowFinished, we just need to set running -> completed.

          // Since we don't have a bulk operator, let's create a temporary list of updates.
          const nodesToUpdate = nextState.nodes.filter(n => n.data.status === 'running');
          nodesToUpdate.forEach(node => {
            apply(updateNodeStatus(nextState, node.id, 'completed'));
          });
        }
      }

      return nextState;
    });

    // Handle side effects (React state updates equivalent)
    if (msg === 'FlowFinished') {
      if (!runId || runId === get().currentRunId) {
        setWorkflowStatus('idle');
        setCurrentRunId(null);
      }
    } else if (msg === 'FlowPaused') {
      if (!runId || runId === get().currentRunId) setWorkflowStatus('paused');
    } else if (msg === 'FlowResumed') {
      if (!runId || runId === get().currentRunId) setWorkflowStatus('running');
    } else if (msg === 'FlowStopped') {
      if (!runId || runId === get().currentRunId) {
        setWorkflowStatus('idle');
        setCurrentRunId(null);
      }
    }
  },

  runWorkflow: async () => {
    const { nodes, edges, currentWorkflowId, notify, setWorkflowStatus, setCurrentRunId, setWorkflowStatuses } = get();

    const blueprint = {
      nodes: nodes.map(n => ({
        id: n.id,
        type: n.data.type,
        data: n.data.config,
        position: n.position,
        width: typeof n.style?.width === 'number' ? n.style.width : (typeof n.style?.width === 'string' ? parseFloat(n.style.width) : (n.width ?? n.measured?.width)),
        height: typeof n.style?.height === 'number' ? n.style.height : (typeof n.style?.height === 'string' ? parseFloat(n.style.height) : (n.height ?? n.measured?.height))
      })),
      edges: edges.map(e => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle
      }))
    };

    try {
      setWorkflowStatus('running');
      set(state => ({ ...state, ...resetWorkflowExecutionState(state) }));

      const res = await api.runWorkflow(blueprint, currentWorkflowId || -1);
      if (res && res.error) {
        throw new Error(res.error);
      }
      if (res && res.run_id) {
        setCurrentRunId(res.run_id);
        if (currentWorkflowId) {
          setWorkflowStatuses({ ...get().workflowStatuses, [currentWorkflowId]: 'running' });
        }
      }
      notify('success', 'Workflow started');
    } catch (e) {
      console.error("Failed to run workflow", e);
      notify('error', 'Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
      setWorkflowStatus('idle');
      setCurrentRunId(null);
      if (currentWorkflowId) {
        setWorkflowStatuses({ ...get().workflowStatuses, [currentWorkflowId]: 'idle' });
      }
    }
  },

  pauseWorkflow: async () => {
    const { currentRunId, notify } = get();
    if (!currentRunId) return;
    try {
      await api.pauseWorkflow(currentRunId);
    } catch (e) {
      console.error("Failed to pause workflow", e);
      notify('error', "Failed to pause workflow");
    }
  },

  resumeWorkflow: async () => {
    const { currentRunId, notify } = get();
    if (!currentRunId) return;
    try {
      await api.resumeWorkflow(currentRunId);
    } catch (e) {
      console.error("Failed to resume workflow", e);
      notify('error', "Failed to resume workflow");
    }
  },

  stopWorkflow: async () => {
    const { currentRunId, notify } = get();
    if (!currentRunId) return;
    try {
      await api.stopWorkflow(currentRunId);
    } catch (e) {
      console.error("Failed to stop workflow", e);
      notify('error', "Failed to stop workflow");
    }
  },

  runNode: async (nodeId: string) => {
    const { nodes, edges, notify, setWorkflowStatus, setCurrentRunId } = get();
    const blueprint = {
      nodes: nodes.map(n => ({
        id: n.id,
        type: n.data.type,
        data: n.data.config,
        position: n.position,
        width: typeof n.style?.width === 'number' ? n.style.width : (typeof n.style?.width === 'string' ? parseFloat(n.style.width) : (n.width ?? n.measured?.width)),
        height: typeof n.style?.height === 'number' ? n.style.height : (typeof n.style?.height === 'string' ? parseFloat(n.style.height) : (n.height ?? n.measured?.height))
      })),
      edges: edges.map(e => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle
      }))
    };

    try {
      set(state => ({ ...state, ...resetWorkflowExecutionState(state) }));

      const res = await api.runNode(blueprint, nodeId);
      if (res && res.error) {
        throw new Error(res.error);
      }
      if (res && res.run_id) {
        setCurrentRunId(res.run_id);
        setWorkflowStatus('running');
      }
      notify('success', `Node ${nodeId} execution started`);
    } catch (e) {
      console.error(`Failed to run node ${nodeId}`, e);
      notify('error', `Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
      setWorkflowStatus('idle');
    }
  }
});
