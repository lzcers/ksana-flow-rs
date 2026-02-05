import { type StateCreator } from 'zustand';
import type { StoreState, Workflow } from './types';
import * as api from '../api';
import { applyCollapsedSubgraphUi } from '../model/workflow/utils';
import { fromBlueprint, toBlueprint } from '@/model/workflow/adapters';
import { NODE_TYPES } from '../components/WorkflowEditor/nodeTypes';
import type { FlowEvent } from '@/model/flowEvent/types';
import { rxWorkflowModel } from '.';
import { castDraft } from 'immer';

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
      const filteredTypes = types.filter(t => allowedTypes.has(t.name));

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
      setNodes(castDraft(preprocessed.nodes));
      setEdges(castDraft(preprocessed.edges));
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
            setWorkflowStatus(status);
            setWorkflowStatuses({ ...get().workflowStatuses, [id]: status });
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
    setNodes(castDraft(preprocessed.nodes));
    setEdges(castDraft(preprocessed.edges));
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

  applyExecutionEvent: (event: FlowEvent) => {
    const runtimeMeta = { meta: { skipHistory: true } } as const;
    if ('nodeId' in event) {
      // 节点相关事件 (FlowNodeMsgEvent | FlowNodeStatusEvent)
      const { nodeId: id } = event;
      switch (event.type) {
        case 'NodeStarted':
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_STATUS',
            payload: { id, status: 'running' },
            ...runtimeMeta
          });
          break;
        case 'NodeStreamStarted':
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_DATA',
            payload: { id, data: { isOutputStream: true } },
            ...runtimeMeta
          });
          break;
        case 'NodeStreamNextMessage': {
          const { msg: chunk } = event;
          const prev = rxWorkflowModel.getSnapshot().nodes.find(n => n.id === id)?.data?.lastMessage;
          const lastMessage =
            typeof chunk === 'string'
              ? `${typeof prev === 'string' ? prev : ''}${chunk}`
              : chunk;
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_DATA',
            payload: { id, data: { lastMessage } },
            ...runtimeMeta
          });
          break;
        }
        case 'NodeInMessage': {
          const { msg: value } = event;
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_DATA',
            payload: { id, data: { lastMessage: value } },
            ...runtimeMeta
          });
          if (typeof value === 'object' && value !== null) {
            rxWorkflowModel.dispatch({
              type: 'UPDATE_NODE_INPUTS',
              payload: { id, inputs: value },
              ...runtimeMeta
            });
          }
          break;
        }
        case 'NodeOutMessage': {
          const { msg: value } = event;
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_DATA',
            payload: { id, data: { lastMessage: value, isOutputStream: false } },
            ...runtimeMeta
          });
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_OUTPUT',
            payload: { id, key: 'output', value },
            ...runtimeMeta
          });

          // Propagate to downstream nodes
          const { edges } = rxWorkflowModel.getSnapshot();
          const outEdges = edges.filter(e => e.source === id);
          outEdges.forEach(edge => {
            rxWorkflowModel.dispatch({
              type: 'UPDATE_NODE_DATA',
              payload: { id: edge.target, data: { lastMessage: value } },
              ...runtimeMeta
            });
            rxWorkflowModel.dispatch({
              type: 'UPDATE_NODE_INPUT',
              payload: { id: edge.target, key: edge.targetHandle || 'default', value },
              ...runtimeMeta
            });
          });
          break;
        }
        case 'NodeCompleted':
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_STATUS',
            payload: { id, status: 'completed' },
            ...runtimeMeta
          });
          break;
        case 'NodeError': {
          const { msg: error } = event;
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_STATUS',
            payload: { id, status: 'error', errorMessage: error },
            ...runtimeMeta
          });
          rxWorkflowModel.dispatch({
            type: 'UPDATE_NODE_DATA',
            payload: { id, data: { isOutputStream: false } },
            ...runtimeMeta
          });
          break;
        }
      }
    } else {
      // 控制事件 (FlowControlEvent)
      // 目前 applyExecutionEvent 主要处理节点事件
      // 控制事件通常由 FlowEventModel 处理
      switch (event.type) {
        case 'FlowFinished':
        case 'FlowStopped':
        case 'FlowPaused':
        case 'FlowResumed':
          // 这些事件由 FlowEventModel 处理
          break;
      }
    }
  }
});
