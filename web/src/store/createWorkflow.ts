import { castDraft } from 'immer';
import { EMPTY } from 'rxjs';
import { type StateCreator } from 'zustand';
import type { StoreState, Workflow } from './types';
import * as api from '../api';
import { applyCollapsedSubgraphUi } from '../model/workflow/utils';
import { fromBlueprint, toBlueprint } from '@/model/workflow/adapters';
import { NODE_TYPES } from '../components/WorkflowEditor/nodeTypes';
import { makeGraphKey, workflowManager, type GraphKey } from '@/model/workflowManager';
import type { WorkflowStatus } from '@/model/workflow/types';

export const createWorkflow: StateCreator<StoreState, [], [], Workflow> = (set, get) => {
  const runIdByGraphKey = new Map<GraphKey, string | null>();
  const statusByGraphKey = new Map<GraphKey, WorkflowStatus>();

  const workflowIdFromGraphKey = (graphKey: GraphKey): number | null => {
    const [, workflowIdRaw] = graphKey.split(':');
    const workflowId = Number(workflowIdRaw);
    return Number.isFinite(workflowId) ? workflowId : null;
  };

  const setGraphStatus = (graphKey: GraphKey, status: WorkflowStatus) => {
    statusByGraphKey.set(graphKey, status);
    const workflowId = workflowIdFromGraphKey(graphKey);
    set((state) => ({
      currentWorkflowStatus: state.activeGraphKey === graphKey ? status : state.currentWorkflowStatus,
      workflowStatuses:
        workflowId == null
          ? state.workflowStatuses
          : {
            ...state.workflowStatuses,
            [workflowId]: status,
          },
    }));
  };

  workflowManager.subscribe((event) => {
    switch (event.type) {
      case 'ActiveChanged': {
        const activeGraphKey = event.activeGraphKey;
        const currentRunId = activeGraphKey ? runIdByGraphKey.get(activeGraphKey) ?? null : null;
        const currentWorkflowStatus = activeGraphKey ? statusByGraphKey.get(activeGraphKey) ?? 'idle' : 'idle';
        set({
          activeGraphKey,
          currentRunId,
          currentWorkflowStatus,
        });
        break;
      }
      case 'RunIdChanged': {
        runIdByGraphKey.set(event.graphKey, event.runId);
        set((state) =>
          state.activeGraphKey === event.graphKey ? { currentRunId: event.runId } : {},
        );
        break;
      }
      case 'WorkflowStatusChanged': {
        setGraphStatus(event.graphKey, event.status);
        break;
      }
      case 'ModelDestroyed': {
        runIdByGraphKey.delete(event.graphKey);
        statusByGraphKey.delete(event.graphKey);
        set((state) => {
          const next: Partial<Workflow> = {};
          if (state.activeGraphKey === event.graphKey) {
            next.activeGraphKey = null;
            next.currentRunId = null;
            next.currentWorkflowStatus = 'idle';
          }
          if (event.workflowId != null && Object.prototype.hasOwnProperty.call(state.workflowStatuses, event.workflowId)) {
            const { [event.workflowId]: _removed, ...rest } = state.workflowStatuses;
            next.workflowStatuses = rest;
          }
          return next;
        });
        break;
      }
    }
  });

  return ({
    workflows: [],
    nodeTypes: [],
    currentSpaceId: null,
    currentWorkflowId: null,
    currentRunId: null,
    currentWorkflowStatus: 'idle',
    activeGraphKey: null,
    workflowStatuses: {},

    setSpaceId: (id) => {
      set({ currentSpaceId: id });
    },

    setWorkflows: (workflows) => set({ workflows }),

    setNodeTypes: (nodeTypes) => set({ nodeTypes }),

    setCurrentWorkflowId: (id) => set({ currentWorkflowId: id }),

    loadMetadata: async () => {
      const { currentSpaceId } = get();
      if (!currentSpaceId) return;
      try {
        const types = await api.fetchNodes(currentSpaceId);
        const wfList = await api.fetchWorkflows(currentSpaceId);

        const allowedTypes = new Set(NODE_TYPES.map(nt => nt.type));
        const filteredTypes = types.filter(t => allowedTypes.has(t.name));

        set({ nodeTypes: filteredTypes, workflows: wfList });
      } catch (e) {
        console.error("Failed to load metadata", e);
      }
    },

    loadWorkflow: async (id: number) => {
      const { currentSpaceId, switchCanvas, error, setNodes, setEdges, selectNode } = get();
      if (!currentSpaceId) return;
      const graphKey = makeGraphKey(currentSpaceId, id);
      try {
        const existing = workflowManager.getModelInstance(graphKey);
        if (existing) {
          runIdByGraphKey.set(graphKey, existing.runId ?? null);
          statusByGraphKey.set(graphKey, existing.status ?? 'idle');
          setGraphStatus(graphKey, existing.status ?? 'idle');

          selectNode([]);
          set({ currentWorkflowId: id });

          const snapshot = existing.model.getSnapshot();
          const hasAnyGraphData = snapshot.nodes.length > 0 || snapshot.edges.length > 0;
          if (hasAnyGraphData) {
            switchCanvas(graphKey);
            return;
          }
        }

        if (!runIdByGraphKey.has(graphKey)) runIdByGraphKey.set(graphKey, null);
        if (!statusByGraphKey.has(graphKey)) statusByGraphKey.set(graphKey, 'idle');

        const rxWorkflowInstance = workflowManager.getOrCreate(graphKey);
        workflowManager.activate(graphKey);

        const wf = await api.fetchWorkflow(currentSpaceId, id);
        const { nodes, edges } = fromBlueprint(wf.blueprint as any);
        const preprocessed = applyCollapsedSubgraphUi(nodes, edges);
        setNodes(castDraft(preprocessed.nodes));
        setEdges(castDraft(preprocessed.edges));
        selectNode([]);
        set({ currentWorkflowId: id });

        try {
          const statusRes = await api.getWorkflowStatus(currentSpaceId, id);
          if (statusRes) {
            if (typeof statusRes.run_id === 'string') {
              workflowManager.setRunId(graphKey, statusRes.run_id);
            }
            if (statusRes.events && Array.isArray(statusRes.events)) {
              statusRes.events.forEach((message: any) => {
                rxWorkflowInstance.applyFlowEvent(message);
              });
            }
          }
        } catch (e) {
          console.warn("Failed to fetch workflow status", e);
        } finally {
          switchCanvas(graphKey);
        }

      } catch (e) {
        console.error("Failed to load workflow", e);
        error('Failed to load workflow');
      }
    },

    saveWorkflow: async (name?: string) => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, workflows, success, error, setWorkflows } = get();
      if (!currentSpaceId) return;
      const blueprint = toBlueprint(nodes, edges);
      try {
        if (currentWorkflowId) {
          const currentWf = workflows.find(w => w.id === currentWorkflowId);
          const nameToUse = name || currentWf?.name || 'Untitled';

          await api.updateWorkflow(currentSpaceId, currentWorkflowId, nameToUse, blueprint as any);

          if (name && name !== currentWf?.name) {
            setWorkflows(workflows.map(w => w.id === currentWorkflowId ? { ...w, name } : w));
          }
        } else {
          await api.createWorkflow(currentSpaceId, name || 'Untitled Workflow', blueprint as any);
        }
        success('Workflow saved');
      } catch (e) {
        console.error("Failed to save workflow", e);
        error('Failed to save workflow');
      }
    },

    renameWorkflow: async (id: number, newName: string) => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, workflows, success, error, setWorkflows } = get();
      if (!currentSpaceId) return;
      try {
        let blueprint;
        if (id === currentWorkflowId) {
          blueprint = toBlueprint(nodes, edges);
        } else {
          const wf = await api.fetchWorkflow(currentSpaceId, id);
          blueprint = wf.blueprint;
        }

        await api.updateWorkflow(currentSpaceId, id, newName, blueprint as any);
        setWorkflows(workflows.map(w => w.id === id ? { ...w, name: newName } : w));
        success('Workflow renamed');
      } catch (e) {
        console.error("Failed to rename workflow", e);
        error('Failed to rename workflow');
      }
    },

    deleteWorkflow: async (id: number) => {
      const { currentSpaceId, currentWorkflowId, workflows, success, error, setWorkflows, setCurrentWorkflowId, setNodes, setEdges, selectNode } = get();
      if (!currentSpaceId) return;
      try {
        await api.deleteWorkflow(currentSpaceId, id);
        const graphKey = makeGraphKey(currentSpaceId, id);
        if (graphKey) workflowManager.destroy(graphKey);
        setWorkflows(workflows.filter(w => w.id !== id));
        if (currentWorkflowId === id) {
          setCurrentWorkflowId(null);
          setNodes([]);
          setEdges([]);
          selectNode([]);
        }
        success('Workflow deleted');
      } catch (e) {
        console.error("Failed to delete workflow", e);
        error('Failed to delete workflow');
      }
    },

    createNewWorkflow: async () => {
      const { currentSpaceId, switchCanvas, setNodes, setEdges, selectNode } = get();
      if (!currentSpaceId) return;
      const graphKey = `${currentSpaceId}:draft`;
      if (!runIdByGraphKey.has(graphKey)) runIdByGraphKey.set(graphKey, null);
      if (!statusByGraphKey.has(graphKey)) statusByGraphKey.set(graphKey, 'idle');
      workflowManager.getOrCreate(graphKey);
      workflowManager.activate(graphKey);
      set({ currentWorkflowId: null });
      setNodes([]);
      setEdges([]);
      selectNode([]);
      switchCanvas(graphKey);
    },

    importWorkflow: (blueprint: any) => {
      const { setNodes, setEdges, selectNode, setCurrentWorkflowId } = get();
      const { nodes, edges } = fromBlueprint(blueprint);

      const preprocessed = applyCollapsedSubgraphUi(nodes, edges);
      setNodes(castDraft(preprocessed.nodes));
      setEdges(castDraft(preprocessed.edges));
      selectNode([]);
      setCurrentWorkflowId(null);

    },

    getWorkflowBlueprint: () => {
      const { nodes, edges } = get();
      return toBlueprint(nodes, edges);
    },

    uploadFile: async (file: File) => {
      const { currentSpaceId } = get();
      if (!currentSpaceId) throw new Error("No active workspace");
      return api.uploadFile(currentSpaceId, file);
    },

    initializeWebSocket: () => {
      const { currentSpaceId } = get();
      if (currentSpaceId) {
        workflowManager.connectWebSocket(currentSpaceId);
      }
      return () => {
        workflowManager.disconnectWebSocket();
      };
    },

    flowEventForRunId$: (runId: string) => {
      if (!runId) return EMPTY;
      return workflowManager.flowEventForRunId$(runId);
    },

    flowEventForNodeId$: (nodeId: string) => {
      const { currentRunId } = get();
      if (!currentRunId) return EMPTY;
      return workflowManager.flowEventForNodeId$(currentRunId, nodeId);
    },

    // ===== Workflow Actions =====
    runWorkflow: async () => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, activeGraphKey, success, error } = get();
      if (!currentSpaceId) return;

      const blueprint = toBlueprint(nodes, edges);
      try {
        const res = await api.runWorkflow(currentSpaceId, blueprint as never, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          const graphKey = activeGraphKey ?? (currentWorkflowId != null ? makeGraphKey(currentSpaceId, currentWorkflowId) : null);
          if (graphKey) {
            workflowManager.setRunId(graphKey, res.run_id);
            setGraphStatus(graphKey, 'running');
          }
          success('Workflow started');
        }
      } catch (e) {
        console.error("Failed to run workflow", e);
        error('Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
      }
    },

    pauseWorkflow: async () => {
      const { currentSpaceId, currentRunId, activeGraphKey, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.pauseWorkflow(currentSpaceId, currentRunId);
        if (activeGraphKey) setGraphStatus(activeGraphKey, 'paused');
      } catch (e) {
        console.error("Failed to pause workflow", e);
        error("Failed to pause workflow");
      }
    },

    resumeWorkflow: async () => {
      const { currentSpaceId, currentRunId, activeGraphKey, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.resumeWorkflow(currentSpaceId, currentRunId);
        if (activeGraphKey) setGraphStatus(activeGraphKey, 'running');
      } catch (e) {
        console.error("Failed to resume workflow", e);
        error("Failed to resume workflow");
      }
    },

    stopWorkflow: async () => {
      const { currentSpaceId, currentRunId, activeGraphKey, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.stopWorkflow(currentSpaceId, currentRunId);
        if (activeGraphKey) {
          workflowManager.setRunId(activeGraphKey, null);
          setGraphStatus(activeGraphKey, 'idle');
        }
      } catch (e) {
        console.error("Failed to stop workflow", e);
        error("Failed to stop workflow");
      }
    },

    runNode: async (nodeId: string) => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, activeGraphKey, success, error } = get();
      if (!currentSpaceId) return;
      const blueprint = toBlueprint(nodes, edges);

      try {
        const res = await api.runNode(currentSpaceId, blueprint as never, nodeId, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          const graphKey = activeGraphKey ?? (currentWorkflowId != null ? makeGraphKey(currentSpaceId, currentWorkflowId) : null);
          if (graphKey) {
            workflowManager.setRunId(graphKey, res.run_id);
            setGraphStatus(graphKey, 'running');
          }
        }
        success(`Node ${nodeId} execution started`);
      } catch (e) {
        console.error(`Failed to run node ${nodeId}`, e);
        error(`Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
      }
    },
  })
};
