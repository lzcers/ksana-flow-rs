import { castDraft } from 'immer';
import { type StateCreator } from 'zustand';
import type { StoreState, Workflow } from './types';
import * as api from '../api';
import { applyCollapsedSubgraphUi } from '../model/workflow/utils';
import { fromBlueprint, toBlueprint } from '@/model/workflow/adapters';
import { NODE_TYPES } from '../components/WorkflowEditor/nodeTypes';
import { makeGraphKey, workflowManager } from './workflowManager';

export const createWorkflow: StateCreator<StoreState, [], [], Workflow> = (set, get) => {
  const getActiveWorkflowInstance = () => {
    const activeModel = workflowManager.activeModel;
    if (!activeModel) {
      throw new Error('No active Model');
    }
    return activeModel;
  };

  return ({
    workflows: [],
    nodeTypes: [],
    setSpaceId: (id) => {
      set({ currentSpaceId: id });
    },

    setWorkflows: (workflows) => set({ workflows }),

    setNodeTypes: (nodeTypes) => set({ nodeTypes }),

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
      try {
        // 1. 获取或创建工作实例
        const graphKey = makeGraphKey(currentSpaceId, id);
        const rxWorkflowInstance = workflowManager.getOrCreate(graphKey);
        workflowManager.activate(graphKey);
        // 2. 从 API 获取工作流定义并初始化
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
            if (statusRes.events && Array.isArray(statusRes.events)) {
              statusRes.events.forEach((message: any) => {
                rxWorkflowInstance.applyFlowEvent(message);
              });
            }
          }

          // 3.切换到新的画布实例
          switchCanvas(graphKey);

        } catch (e) {
          console.warn("Failed to fetch workflow status", e);
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
        if (graphKey) workflowModelManager.destroy(graphKey);
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

    },

    importWorkflow: (blueprint: any) => {
      const { setNodes, setEdges, selectNode, setCurrentWorkflowId } = get();
      const { nodes, edges } = fromBlueprint(blueprint);

      const preprocessed = applyCollapsedSubgraphUi(nodes, edges);
      setNodes(castDraft(preprocessed.nodes));
      setEdges(castDraft(preprocessed.edges));

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

    // ===== Workflow Actions =====
    runWorkflow: async () => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error } = get();
      if (!currentSpaceId) return;

      const blueprint = toBlueprint(nodes, edges);
      try {
        const res = await api.runWorkflow(currentSpaceId, blueprint as never, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          success('Workflow started');
        }
      } catch (e) {
        console.error("Failed to run workflow", e);
        error('Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
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
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error } = get();
      if (!currentSpaceId) return;
      const blueprint = toBlueprint(nodes, edges);

      try {
        const res = await api.runNode(currentSpaceId, blueprint as never, nodeId, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
        }
        success(`Node ${nodeId} execution started`);
      } catch (e) {
        console.error(`Failed to run node ${nodeId}`, e);
        error(`Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
      }
    },
  })
};
