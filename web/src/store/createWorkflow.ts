import { type StateCreator } from 'zustand';
import type { StoreState, Workflow } from './types';
import * as api from '../api';
import { applyCollapsedSubgraphUi } from '../model/workflow/utils';
import { NODE_TYPES } from '../components/WorkflowEditor/nodeTypes';
import { workflowModel } from '.';
import { fromBlueprint, toBlueprint } from '@/model/workflow/adapters';

export const createWorkflow: StateCreator<StoreState, [], [], Workflow> = (set, get) => ({
  workflows: [],
  currentWorkflowId: null,
  currentSpaceId: null,
  nodeTypes: [],

  setSpaceId: (id) => set({ currentSpaceId: id }),
  setWorkflows: (workflows) => set({ workflows }),
  setCurrentWorkflowId: (currentWorkflowId) => set({ currentWorkflowId }),
  setNodeTypes: (nodeTypes) => set({ nodeTypes }),

  loadMetadata: async () => {
    const { currentSpaceId } = get();
    if (!currentSpaceId) return;
    try {
      const types = await api.fetchNodes(currentSpaceId);
      const wfList = await api.fetchWorkflows(currentSpaceId);

      const allowedTypes = new Set(NODE_TYPES.map(nt => nt.type));
      const filteredTypes = types.filter(t => allowedTypes.has(t.name as any));

      // Inject SubgraphNode manually if not present (frontend-only node)
      if (!filteredTypes.find(t => t.name === 'SubgraphNode')) {
        filteredTypes.push({
          name: 'SubgraphNode',
          description: 'A group of nodes (Subgraph)',
          category: 'Logic',
          inputs: [],
          outputs: [],
          config: {},
        });
      }

      set({ nodeTypes: filteredTypes, workflows: wfList });
    } catch (e) {
      console.error("Failed to load metadata", e);
    }
  },

  loadWorkflow: async (id: number) => {
    const { currentSpaceId, error, setNodes, setEdges, selectNode, setWorkflowStatus, setWorkflowStatuses, setCurrentRunId } = get();
    if (!currentSpaceId) return;
    try {
      const wf = await api.fetchWorkflow(currentSpaceId, id);
      set({ currentWorkflowId: id });

      // Ensure blueprint matches BackendNode type structure or cast appropriately if safe
      const { nodes, edges } = fromBlueprint(wf.blueprint as any);

      const preprocessed = applyCollapsedSubgraphUi(nodes, edges);
      setNodes(preprocessed.nodes);
      setEdges(preprocessed.edges);
      selectNode(null);

      try {
        const statusRes = await api.getWorkflowStatus(currentSpaceId, id);
        if (statusRes) {
          if (statusRes.run_id) {
            setCurrentRunId(statusRes.run_id);
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

            statusRes.events.forEach((event: any) => {
              const { applyExecutionEvent } = get();
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
    const { currentSpaceId, nodes, edges, currentWorkflowId, workflows, success, error, setWorkflows, setCurrentWorkflowId } = get();
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
        const newWf = await api.createWorkflow(currentSpaceId, name || 'Untitled Workflow', blueprint as any);
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
  },

  importWorkflow: (blueprint: any) => {
    const { setNodes, setEdges, selectNode, setCurrentWorkflowId, setWorkflowStatus, setCurrentRunId } = get();
    // Transform backend nodes to ReactFlow nodes
    const { nodes, edges } = fromBlueprint(blueprint);

    const preprocessed = applyCollapsedSubgraphUi(nodes, edges);
    setNodes(preprocessed.nodes);
    setEdges(preprocessed.edges);
    selectNode(null);
    setCurrentWorkflowId(null);
    setWorkflowStatus('idle');
    setCurrentRunId(null);
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

  applyExecutionEvent: (event: any) => {
    // 使用 CommandBus 派发事件，而不是直接修改 state
    // 注意：这里不再需要 set()，因为 CommandBus 会更新 RxState，RxState 会同步回 Zustand
    if (event.NodeStarted) {
      const id = event.NodeStarted;
      workflowModel.dispatch({
        type: 'UPDATE_NODE_STATUS',
        payload: { id, status: 'running' }
      });
    } else if (event.NodeStreamStarted) {
      const id = event.NodeStreamStarted;
      workflowModel.dispatch({
        type: 'UPDATE_NODE_DATA',
        payload: { id, data: { isOutputStream: true } }
      });
    }
    else if (event.NodeInMessage) {
      const [id, value] = event.NodeInMessage;
      workflowModel.dispatch({
        type: 'UPDATE_NODE_DATA',
        payload: { id, data: { lastMessage: value } }
      });
      if (typeof value === 'object' && value !== null) {
        workflowModel.dispatch({
          type: 'UPDATE_NODE_INPUTS',
          payload: { id, inputs: value }
        });
      }
    } else if (event.NodeOutMessage) {
      const [id, value] = event.NodeOutMessage;
      workflowModel.dispatch({
        type: 'UPDATE_NODE_DATA',
        payload: { id, data: { lastMessage: value, isOutputStream: false } }
      });
      workflowModel.dispatch({
        type: 'UPDATE_NODE_OUTPUT',
        payload: { id, key: 'output', value }
      });

      // Propagate to downstream nodes
      const { edges } = workflowModel.getSnapshot();
      const outEdges = edges.filter(e => e.source === id);
      outEdges.forEach(edge => {
        workflowModel.dispatch({
          type: 'UPDATE_NODE_DATA',
          payload: { id: edge.target, data: { lastMessage: value } }
        });
        workflowModel.dispatch({
          type: 'UPDATE_NODE_INPUT',
          payload: { id: edge.target, key: edge.targetHandle || 'default', value }
        });
      });
    } else if (event.NodeCompleted) {
      const id = event.NodeCompleted;
      workflowModel.dispatch({
        type: 'UPDATE_NODE_STATUS',
        payload: { id, status: 'completed' }
      });
    } else if (event.NodeError) {
      const [id, error] = event.NodeError;
      workflowModel.dispatch({
        type: 'UPDATE_NODE_STATUS',
        payload: { id, status: 'error', errorMessage: error }
      });
      workflowModel.dispatch({
        type: 'UPDATE_NODE_DATA',
        payload: { id, data: { isOutputStream: false } }
      });
    }
  }
});
