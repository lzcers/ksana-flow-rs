import { type StateCreator } from 'zustand';
import type { StoreState, WorkflowSlice } from './types';
import type { Node, Edge } from '../model/types';
import * as api from '../api';
import { updateNodeData, updateNodeStatus } from '../model';

export const createWorkflowSlice: StateCreator<StoreState, [], [], WorkflowSlice> = (set, get) => ({
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
      set({ nodeTypes: types, workflows: wfList });
    } catch (e) {
      console.error("Failed to load metadata", e);
    }
  },

  loadWorkflow: async (id: number) => {
    const { currentSpaceId, nodeTypes, error, setNodes, setEdges, selectNode, setWorkflowStatus, setWorkflowStatuses, setCurrentRunId } = get();
    if (!currentSpaceId) return;
    try {
      const wf = await api.fetchWorkflow(currentSpaceId, id);
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
        type: e.type || 'default'
      }));

      setNodes(nodes);
      setEdges(edges);
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
              const { applyExecutionEvent } = get() as any;
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
        targetHandle: e.targetHandle,
        type: e.type
      }))
    };

    try {
      if (currentWorkflowId) {
        const currentWf = workflows.find(w => w.id === currentWorkflowId);
        const nameToUse = name || currentWf?.name || 'Untitled';

        await api.updateWorkflow(currentSpaceId, currentWorkflowId, nameToUse, blueprint);

        if (name && name !== currentWf?.name) {
          setWorkflows(workflows.map(w => w.id === currentWorkflowId ? { ...w, name } : w));
        }
      } else {
        const newWf = await api.createWorkflow(currentSpaceId, name || 'Untitled Workflow', blueprint);
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
            targetHandle: e.targetHandle,
            type: e.type
          }))
        };
      } else {
        const wf = await api.fetchWorkflow(currentSpaceId, id);
        blueprint = wf.blueprint;
      }

      await api.updateWorkflow(currentSpaceId, id, newName, blueprint);
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
    const { setNodes, setEdges, selectNode, setCurrentWorkflowId, setWorkflowStatus, setCurrentRunId, nodeTypes } = get();
    
    // Transform backend nodes to ReactFlow nodes
    const nodes: Node[] = (blueprint.nodes || []).map((n: any) => ({
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
    const edges: Edge[] = (blueprint.edges || []).map((e: any) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      sourceHandle: e.sourceHandle,
      targetHandle: e.targetHandle,
      type: e.type || 'default'
    }));

    setNodes(nodes);
    setEdges(edges);
    selectNode(null);
    setCurrentWorkflowId(null);
    setWorkflowStatus('idle');
    setCurrentRunId(null);
  },

  getWorkflowBlueprint: () => {
    const { nodes, edges } = get();
    return {
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
        targetHandle: e.targetHandle,
        type: e.type
      }))
    };
  },

  uploadFile: async (file: File) => {
    const { currentSpaceId } = get();
    if (!currentSpaceId) throw new Error("No active workspace");
    return api.uploadFile(currentSpaceId, file);
  },

  applyExecutionEvent: (event: any) => {
    set((state) => {
      let nextState = state;
      const apply = (opResult: any) => {
        nextState = { ...nextState, ...opResult };
      };

      if (event.NodeStarted) {
        const id = event.NodeStarted;
        apply(updateNodeStatus(nextState, id, 'running'));
      } else if (event.NodeStreamStarted) {
        const id = event.NodeStreamStarted;
        apply(updateNodeData(nextState, id, { isOutputStream: true }));
      } else if (event.NodeStreamNextMessage) {
        const [id, value] = event.NodeStreamNextMessage;
        apply(updateNodeData(nextState, id, { lastMessage: value }));
        // Propagate to downstream nodes
        const outEdges = nextState.edges.filter(e => e.source === id);
        outEdges.forEach(edge => {
          apply(updateNodeData(nextState, edge.target, { lastMessage: value }));
        });
      } else if (event.NodeInMessage) {
        const [id, value] = event.NodeInMessage;
        apply(updateNodeData(nextState, id, { lastMessage: value }));
      } else if (event.NodeOutMessage) {
        const [id, value] = event.NodeOutMessage;
        apply(updateNodeData(nextState, id, { lastMessage: value, isOutputStream: false }));
        // Propagate to downstream nodes
        const outEdges = nextState.edges.filter(e => e.source === id);
        outEdges.forEach(edge => {
          apply(updateNodeData(nextState, edge.target, { lastMessage: value }));
        });
      } else if (event.NodeCompleted) {
        const id = event.NodeCompleted;
        apply(updateNodeStatus(nextState, id, 'completed'));
      } else if (event.NodeError) {
        const [id, error] = event.NodeError;
        apply(updateNodeStatus(nextState, id, 'error', error));
        apply(updateNodeData(nextState, id, { isOutputStream: false }));
      } else if (event === 'FlowFinished') {
        const nodesToUpdate = nextState.nodes.filter(n => n.data.status === 'running');
        nodesToUpdate.forEach(node => {
          apply(updateNodeStatus(nextState, node.id, 'completed'));
        });
      }

      return nextState;
    });
  }
});
