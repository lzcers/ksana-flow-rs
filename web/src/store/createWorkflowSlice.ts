import { type StateCreator } from 'zustand';
import type { StoreState, WorkflowSlice } from './types';
import type { Node, Edge } from '../model/types';
import * as api from '../api';

export const createWorkflowSlice: StateCreator<StoreState, [], [], WorkflowSlice> = (set, get) => ({
  workflows: [],
  currentWorkflowId: null,
  nodeTypes: [],

  setWorkflows: (workflows) => set({ workflows }),
  setCurrentWorkflowId: (currentWorkflowId) => set({ currentWorkflowId }),
  setNodeTypes: (nodeTypes) => set({ nodeTypes }),

  loadMetadata: async () => {
    try {
      const types = await api.fetchNodes();
      const wfList = await api.fetchWorkflows();
      set({ nodeTypes: types, workflows: wfList });
    } catch (e) {
      console.error("Failed to load metadata", e);
    }
  },

  loadWorkflow: async (id: number) => {
    const { nodeTypes, error, setNodes, setEdges, selectNode, setWorkflowStatus, setWorkflowStatuses, setCurrentRunId } = get();
    try {
      const wf = await api.fetchWorkflow(id);
      set({ currentWorkflowId: id });

      // Transform backend nodes to ReactFlow nodes
      const nodes: Node[] = wf.blueprint.nodes.map((n: any) => ({
        id: n.id,
        type: 'workflow',
        position: n.position || { x: 0, y: 0 },
        width: n.width,
        height: n.height,
        style: n.width && n.height ? { width: n.width, height: n.height } : undefined,
        data: {
          label: n.type,
          type: n.type,
          description: nodeTypes.find(t => t.name === n.type)?.description || '',
          config: n.data,
          status: 'idle'
        }
      }));

      // Transform backend edges to ReactFlow edges
      const edges: Edge[] = wf.blueprint.edges.map((e: any) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle,
        type: 'smoothstep'
      }));

      setNodes(nodes);
      setEdges(edges);
      selectNode(null);

      // Fetch and replay execution status
      try {
        const statusRes = await api.getWorkflowStatus(id);
        if (statusRes) {
          if (statusRes.run_id) {
            setCurrentRunId(statusRes.run_id);
            // We might need a way to map runId to workflowId in execution slice if needed globally
            // For now, assuming current workflow context
          }
          if (statusRes.status) {
            let status = statusRes.status.toLowerCase();
            if (status === 'completed' || status === 'stopped' || status === 'failed') {
              status = 'idle';
            }
            setWorkflowStatus(status as any);
            setWorkflowStatuses({ ...get().workflowStatuses, [id]: status as any });
          }

          if (statusRes.events && Array.isArray(statusRes.events)) {
            // Replay events
            // We need to access state to replay events.
            // Since handleWebSocketMessage updates state based on event, we can reuse it?
            // But handleWebSocketMessage expects a wrapper.
            // Let's manually apply events or expose applyEventToState.
            // For simplicity, let's just use handleWebSocketMessage with a fake wrapper if possible,
            // or better, extract applyEvent logic.
            // Since we are inside the store, we can just dispatch updates.
            // Actually, `useWorkflow` had `applyEventToState`. We should probably expose that or just iterate here.

            // To avoid duplication, I will implement event application in execution slice and call it here if exposed,
            // or just rely on the fact that `handleWebSocketMessage` does it. 
            // `handleWebSocketMessage` takes `{ runId, event }`.

            statusRes.events.forEach((event: any) => {
              // We need to update nodes based on these events.
              // This logic is currently in `applyEventToState` in `useWorkflow.ts`.
              // I should move that logic to `executionSlice`'s `handleWebSocketMessage` or a helper.
              // Let's assume `handleWebSocketMessage` can handle raw event if we pass a special flag or just call the internal helper.
              // But `handleWebSocketMessage` is an action.
              // Let's just defer this implementation detail to `executionSlice` and assume we can call an action there.
              // Ideally `executionSlice` should expose `processEvent(event)`.
              // For now, I'll access the store state directly via `get()` in `executionSlice`.

              // Since I cannot call `applyEventToState` easily if it's not exported, I will implement it in `executionSlice` as `applyEvent`.
              // And I'll call it here.
              // But `applyEvent` is not in `WorkflowSlice` interface.
              // I will cast or extend the interface later.

              // Let's try to reuse `handleWebSocketMessage` by constructing a fake message?
              // No, that's hacky.
              // I'll add `applyExecutionEvent` to ExecutionSlice interface.
              const { applyExecutionEvent } = get() as any; // Type assertion for now
              if (applyExecutionEvent) {
                applyExecutionEvent(event);
              }
            });
          }
        }
      } catch (e) {
        console.warn("Failed to fetch workflow status", e);
      }

    } catch (e) {
      console.error("Failed to load workflow", e);
      error('Failed to load workflow');
    }
  },

  saveWorkflow: async (name?: string) => {
    const { nodes, edges, currentWorkflowId, workflows, success, error, setWorkflows, setCurrentWorkflowId } = get();

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
      if (currentWorkflowId) {
        const currentWf = workflows.find(w => w.id === currentWorkflowId);
        const nameToUse = name || currentWf?.name || 'Untitled';

        await api.updateWorkflow(currentWorkflowId, nameToUse, blueprint);

        if (name && name !== currentWf?.name) {
          setWorkflows(workflows.map(w => w.id === currentWorkflowId ? { ...w, name } : w));
        }
      } else {
        const newWf = await api.createWorkflow(name || 'Untitled Workflow', blueprint);
        setCurrentWorkflowId(newWf.id);
        setWorkflows([...workflows, { id: newWf.id, name: name || 'Untitled Workflow' }]);
      }
      success('Workflow saved');
    } catch (e) {
      console.error("Failed to save workflow", e);
      error('Failed to save workflow');
    }
  },

  renameWorkflow: async (id: number, newName: string) => {
    const { nodes, edges, currentWorkflowId, workflows, success, error, setWorkflows } = get();
    try {
      let blueprint;
      if (id === currentWorkflowId) {
        blueprint = {
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
      } else {
        const wf = await api.fetchWorkflow(id);
        blueprint = wf.blueprint;
      }

      await api.updateWorkflow(id, newName, blueprint);
      setWorkflows(workflows.map(w => w.id === id ? { ...w, name: newName } : w));
      success('Workflow renamed');
    } catch (e) {
      console.error("Failed to rename workflow", e);
      error('Failed to rename workflow');
    }
  },

  deleteWorkflow: async (id: number) => {
    const { currentWorkflowId, workflows, success, error, setWorkflows, setCurrentWorkflowId, setNodes, setEdges, selectNode } = get();
    try {
      await api.deleteWorkflow(id);
      setWorkflows(workflows.filter(w => w.id !== id));
      if (currentWorkflowId === id) {
        setCurrentWorkflowId(null);
        setNodes([]);
        setEdges([]);
        selectNode(null);
      }
      success('Workflow deleted');
    } catch (e) {
      console.error("Failed to delete workflow", e);
      error('Failed to delete workflow');
    }
  },

  createNewWorkflow: async () => {
    const { setNodes, setEdges, selectNode, setCurrentWorkflowId, setWorkflowStatus, setCurrentRunId } = get();
    setNodes([]);
    setEdges([]);
    selectNode(null);
    setCurrentWorkflowId(null);
    setWorkflowStatus('idle');
    setCurrentRunId(null);
  }
});
