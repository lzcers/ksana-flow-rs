import type { StateCreator } from 'zustand';
import type { StoreState, ExecutionSlice, WorkflowStatus } from './types';
import * as api from '../api';
import { updateNodeStatus, updateNodeData, resetWorkflowExecutionState } from '../model';

export const createExecutionSlice: StateCreator<StoreState, [], [], ExecutionSlice> = (set, get) => ({
  workflowStatus: 'idle',
  workflowStatuses: {},
  runIdToWorkflowId: {},
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
    const { currentRunId, setWorkflowStatus, setCurrentRunId, setWorkflowStatuses } = get();

    if (runId && currentRunId && runId !== currentRunId) {
    }

    set(state => {
      let nextState = state;

      const apply = (opResult: any) => {
        nextState = { ...nextState, ...opResult };
      };

      if (!runId || (runId === state.currentRunId)) {
        if (msg.NodeStarted) {
          const id = msg.NodeStarted;
          apply(updateNodeStatus(nextState, id, 'running'));
        } else if (msg.NodeStreamStarted) {
          const id = msg.NodeStreamStarted;
          apply(updateNodeData(nextState, id, { isOutputStream: true }));
          // Propagate to downstream nodes
          const outEdges = nextState.edges.filter(e => e.source === id);
          outEdges.forEach(edge => {
            apply(updateNodeData(nextState, edge.target, { upstreamIsStreaming: true }));
          });
        } else if (msg.NodeStreamNextMessage) {
          const [id, value] = msg.NodeStreamNextMessage;
          apply(updateNodeData(nextState, id, { lastMessage: value, lastMessageRunId: runId }));
          // Propagate to downstream nodes
          const outEdges = nextState.edges.filter(e => e.source === id);
          outEdges.forEach(edge => {
            apply(updateNodeData(nextState, edge.target, { lastMessage: value, lastMessageRunId: runId }));
          });
        } else if (msg.NodeInMessage) {
          const [id, value] = msg.NodeInMessage;
          apply(updateNodeData(nextState, id, { lastMessage: value, lastMessageRunId: runId }));
        } else if (msg.NodeOutMessage) {
          const [id, value] = msg.NodeOutMessage;
          apply(updateNodeData(nextState, id, { lastMessage: value, lastMessageRunId: runId, isOutputStream: false }));
          // Propagate to downstream nodes
          const outEdges = nextState.edges.filter(e => e.source === id);
          outEdges.forEach(edge => {
            apply(updateNodeData(nextState, edge.target, { lastMessage: value, lastMessageRunId: runId, upstreamIsStreaming: false }));
          });
        } else if (msg.NodeCompleted) {
          const id = msg.NodeCompleted;
          apply(updateNodeStatus(nextState, id, 'completed'));
        } else if (msg.NodeError) {
          const [id, error] = msg.NodeError;
          apply(updateNodeStatus(nextState, id, 'error', error));
          apply(updateNodeData(nextState, id, { isOutputStream: false }));
        } else if (msg === 'FlowFinished') {
          const nodesToUpdate = nextState.nodes.filter(n => n.data.status === 'running');
          nodesToUpdate.forEach(node => {
            apply(updateNodeStatus(nextState, node.id, 'completed'));
          });
        }
      }

      return nextState;
    });

    // Handle side effects (React state updates equivalent)
    // Handle status updates for both current and background workflows
    const workflowId = runId ? get().runIdToWorkflowId[runId] : null;

    if (msg === 'FlowFinished' || msg === 'FlowStopped') {
      if (workflowId) {
        set(state => {
          const newStatuses: Record<number, WorkflowStatus> = { ...state.workflowStatuses, [workflowId]: 'idle' };
          const newMap = { ...state.runIdToWorkflowId };
          delete newMap[runId];
          return { workflowStatuses: newStatuses, runIdToWorkflowId: newMap };
        });
      }

      if (!runId || runId === get().currentRunId) {
        setWorkflowStatus('idle');
        setCurrentRunId(null);
      }
    } else if (msg === 'FlowPaused') {
      if (workflowId) {
        setWorkflowStatuses({ ...get().workflowStatuses, [workflowId]: 'paused' });
      }
      if (!runId || runId === get().currentRunId) setWorkflowStatus('paused');
    } else if (msg === 'FlowResumed') {
      if (workflowId) {
        setWorkflowStatuses({ ...get().workflowStatuses, [workflowId]: 'running' });
      }
      if (!runId || runId === get().currentRunId) setWorkflowStatus('running');
    }
  },

  runWorkflow: async () => {
    const { nodes, edges, currentWorkflowId, success, error, setWorkflowStatus, setCurrentRunId, setWorkflowStatuses } = get();

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
          set(state => ({ runIdToWorkflowId: { ...state.runIdToWorkflowId, [res.run_id]: currentWorkflowId } }));
        }
      }
      success('Workflow started');
    } catch (e) {
      console.error("Failed to run workflow", e);
      error('Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
      setWorkflowStatus('idle');
      setCurrentRunId(null);
      if (currentWorkflowId) {
        setWorkflowStatuses({ ...get().workflowStatuses, [currentWorkflowId]: 'idle' });
      }
    }
  },

  pauseWorkflow: async () => {
    const { currentRunId, error } = get();
    if (!currentRunId) return;
    try {
      await api.pauseWorkflow(currentRunId);
    } catch (e) {
      console.error("Failed to pause workflow", e);
      error("Failed to pause workflow");
    }
  },

  resumeWorkflow: async () => {
    const { currentRunId, error } = get();
    if (!currentRunId) return;
    try {
      await api.resumeWorkflow(currentRunId);
    } catch (e) {
      console.error("Failed to resume workflow", e);
      error("Failed to resume workflow");
    }
  },

  stopWorkflow: async () => {
    const { currentRunId, error } = get();
    if (!currentRunId) return;
    try {
      await api.stopWorkflow(currentRunId);
    } catch (e) {
      console.error("Failed to stop workflow", e);
      error("Failed to stop workflow");
    }
  },

  runNode: async (nodeId: string) => {
    const { nodes, edges, success, error, setWorkflowStatus, setCurrentRunId } = get();
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
      success(`Node ${nodeId} execution started`);
    } catch (e) {
      console.error(`Failed to run node ${nodeId}`, e);
      error(`Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
      setWorkflowStatus('idle');
    }
  }
});
