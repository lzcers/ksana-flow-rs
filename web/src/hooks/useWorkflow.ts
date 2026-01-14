import { useCallback, useEffect, useState } from 'react';
import { useImmer } from 'use-immer';
import {
  addEdge as flowAddEdge,
  applyNodeChanges,
  applyEdgeChanges,
  type NodeChange,
  type EdgeChange,
  type Connection,
  type Node
} from '@xyflow/react';
import type { WorkflowState, WorkflowNodeData, WorkflowNode, WorkflowEdge } from '../types/workflow';
import * as api from '../api';
import { useToast } from '../contexts/ToastContext';

const INITIAL_STATE: WorkflowState = {
  nodes: [],
  edges: [],
  selectedNodeId: null,
};

export function useWorkflow() {
  const [state, updateState] = useImmer<WorkflowState>(INITIAL_STATE);
  const [nodeTypes, setNodeTypes] = useState<api.NodeMetadata[]>([]);
  const [currentWorkflowId, setCurrentWorkflowId] = useState<number | null>(null);
  const [workflows, setWorkflows] = useState<{ id: number; name: string }[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const toast = useToast();

  useEffect(() => {
    let ws: WebSocket | null = null;
    let reconnectTimeout: number | null = null;

    const connect = () => {
      ws = new WebSocket('ws://localhost:3000/ws');

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          updateState(draft => {
            if (msg.NodeStarted) {
              const id = msg.NodeStarted;
              const node = draft.nodes.find(n => n.id === id);
              if (node) node.data.status = 'running';
            } else if (msg.NodeMessage) {
              const [id, value] = msg.NodeMessage;
              const node = draft.nodes.find(n => n.id === id);
              if (node) node.data.lastMessage = value;
            } else if (msg.NodeCompleted) {
              const id = msg.NodeCompleted;
              const node = draft.nodes.find(n => n.id === id);
              if (node) node.data.status = 'completed';
            } else if (msg.NodeError) {
              const [id, error] = msg.NodeError;
              const node = draft.nodes.find(n => n.id === id);
              if (node) {
                node.data.status = 'error';
                node.data.errorMessage = error;
              }
            } else if (msg === 'Finished') {
              // Optionally mark all running nodes as completed or idle
              draft.nodes.forEach(node => {
                if (node.data.status === 'running') {
                  node.data.status = 'completed';
                }
              });
              setIsRunning(false);
            }
          });
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
  }, [updateState]);

  useEffect(() => {
    const loadMetadata = async () => {
      const types = await api.fetchNodes();
      setNodeTypes(types);
      const wfList = await api.fetchWorkflows();
      setWorkflows(wfList);
    };
    loadMetadata();
  }, []);

  const loadWorkflow = useCallback(async (id: number) => {
    try {
      const wf = await api.fetchWorkflow(id);
      setCurrentWorkflowId(id);

      // Transform backend nodes to ReactFlow nodes
      const nodes: WorkflowNode[] = wf.blueprint.nodes.map((n: any) => ({
        id: n.id,
        type: 'workflow',
        position: n.position || { x: 0, y: 0 },
        data: {
          label: n.type,
          type: n.type,
          description: nodeTypes.find(t => t.name === n.type)?.description || '',
          config: n.data,
          status: 'idle'
        }
      }));

      // Transform backend edges to ReactFlow edges
      const edges: WorkflowEdge[] = wf.blueprint.edges.map((e: any) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle,
        type: 'smoothstep'
      }));

      updateState(draft => {
        draft.nodes = nodes;
        draft.edges = edges;
        draft.selectedNodeId = null;
      });
    } catch (e) {
      console.error("Failed to load workflow", e);
      toast.error('Failed to load workflow');
    }
  }, [updateState, nodeTypes, toast]);

  const saveWorkflow = useCallback(async (name?: string) => {
    // Transform ReactFlow state to backend blueprint format
    const blueprint = {
      nodes: state.nodes.map(n => ({
        id: n.id,
        type: n.data.type,
        data: n.data.config,
        position: n.position
      })),
      edges: state.edges.map(e => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle
      }))
    };

    try {
      if (currentWorkflowId) {
        // If name is not provided, keep the existing name
        const currentWf = workflows.find(w => w.id === currentWorkflowId);
        const nameToUse = name || currentWf?.name;

        await api.updateWorkflow(currentWorkflowId, nameToUse, blueprint);

        // Update local list if name changed
        if (name && name !== currentWf?.name) {
          setWorkflows(prev => prev.map(w => w.id === currentWorkflowId ? { ...w, name } : w));
        }
      } else {
        const newWf = await api.createWorkflow(name || 'Untitled Workflow', blueprint);
        setCurrentWorkflowId(newWf.id);
        setWorkflows(prev => [...prev, { id: newWf.id, name: name || 'Untitled Workflow' }]);
      }
      toast.success('Workflow saved');
    } catch (e) {
      console.error("Failed to save workflow", e);
      toast.error('Failed to save workflow');
    }
  }, [state.nodes, state.edges, currentWorkflowId, workflows, toast]);

  const renameWorkflow = useCallback(async (id: number, newName: string) => {
    try {
      let blueprint;
      // If we are renaming the current workflow, use the current state to avoid data loss
      if (id === currentWorkflowId) {
        blueprint = {
          nodes: state.nodes.map(n => ({
            id: n.id,
            type: n.data.type,
            data: n.data.config,
            position: n.position
          })),
          edges: state.edges.map(e => ({
            id: e.id,
            source: e.source,
            target: e.target,
            sourceHandle: e.sourceHandle,
            targetHandle: e.targetHandle
          }))
        };
      } else {
        // Otherwise fetch the latest blueprint from server
        const wf = await api.fetchWorkflow(id);
        blueprint = wf.blueprint;
      }

      await api.updateWorkflow(id, newName, blueprint);
      setWorkflows(prev => prev.map(w => w.id === id ? { ...w, name: newName } : w));
      toast.success('Workflow renamed');
    } catch (e) {
      console.error("Failed to rename workflow", e);
      toast.error('Failed to rename workflow');
    }
  }, [currentWorkflowId, state.nodes, state.edges, toast]);

  const deleteWorkflow = useCallback(async (id: number) => {
    try {
      await api.deleteWorkflow(id);
      setWorkflows(prev => prev.filter(w => w.id !== id));
      if (currentWorkflowId === id) {
        setCurrentWorkflowId(null);
        updateState(draft => {
          draft.nodes = [];
          draft.edges = [];
          draft.selectedNodeId = null;
        });
      }
      toast.success('Workflow deleted');
    } catch (e) {
      console.error("Failed to delete workflow", e);
      toast.error('Failed to delete workflow');
    }
  }, [currentWorkflowId, updateState, toast]);

  const createNewWorkflow = useCallback(async () => {
    updateState(draft => {
      draft.nodes = [];
      draft.edges = [];
      draft.selectedNodeId = null;
    });
    setCurrentWorkflowId(null);
  }, [updateState]);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    updateState(draft => {
      draft.nodes = applyNodeChanges(changes, draft.nodes as unknown as WorkflowNode[]) as WorkflowNode[];

      // Update selectedNodeId based on changes
      const selectChange = changes.find(c => c.type === 'select');
      if (selectChange && 'selected' in selectChange) {
        if (selectChange.selected) {
          draft.selectedNodeId = selectChange.id;
        } else if (draft.selectedNodeId === selectChange.id) {
          draft.selectedNodeId = null;
        }
      }
    });
  }, [updateState]);

  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    updateState(draft => {
      draft.edges = applyEdgeChanges(changes, draft.edges as unknown as WorkflowEdge[]) as WorkflowEdge[];
    });
  }, [updateState]);

  const onNodeDragStop = useCallback((_event: any, _node: Node) => {
    // No explicit action needed as position is updated in onNodesChange
  }, []);

  const onConnect = useCallback((connection: Connection) => {
    const edgeId = `${connection.source}-${connection.target}`;
    updateState(draft => {
      draft.edges = flowAddEdge({ ...connection, id: edgeId, type: 'smoothstep' }, draft.edges as unknown as WorkflowEdge[]) as WorkflowEdge[];
    });
  }, [updateState]);

  const addNode = useCallback((type: string, position: { x: number; y: number } = { x: 300, y: 200 }) => {
    // Find all nodes of the same type and extract their numbers
    const sameTypeNodes = state.nodes.filter(n => n.id.startsWith(`${type}-`));
    let nextNum = 1;
    if (sameTypeNodes.length > 0) {
      const nums = sameTypeNodes.map(n => {
        const parts = n.id.split('-');
        const lastPart = parts[parts.length - 1];
        const num = parseInt(lastPart, 10);
        return isNaN(num) ? 0 : num;
      });
      nextNum = Math.max(...nums) + 1;
    }
    const id = `${type}-${nextNum}`;
    const meta = nodeTypes.find(t => t.name === type);

    updateState(draft => {
      draft.nodes.push({
        id,
        type: 'workflow',
        position,
        data: {
          label: type,
          type: type,
          description: meta?.description || '',
          config: meta?.config || {}
        },
      });
      draft.selectedNodeId = id;
    });
  }, [updateState, nodeTypes, state.nodes]);

  const deleteNode = useCallback((id: string) => {
    updateState(draft => {
      draft.nodes = draft.nodes.filter(n => n.id !== id);
      draft.edges = draft.edges.filter(e => e.source !== id && e.target !== id);
      if (draft.selectedNodeId === id) draft.selectedNodeId = null;
    });
  }, [updateState]);

  const updateNodeData = useCallback((id: string, data: Partial<WorkflowNodeData>) => {
    updateState(draft => {
      const node = draft.nodes.find(n => n.id === id);
      if (node) {
        node.data = { ...node.data, ...data };
      }
    });
  }, [updateState]);

  const runWorkflow = useCallback(async () => {
    // Transform ReactFlow state to backend blueprint format
    const blueprint = {
      nodes: state.nodes.map(n => ({
        id: n.id,
        type: n.data.type,
        data: n.data.config,
        position: n.position
      })),
      edges: state.edges.map(e => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle
      }))
    };

    try {
      setIsRunning(true);
      updateState(draft => {
        draft.nodes.forEach(node => {
          node.data.status = 'idle';
          node.data.errorMessage = undefined;
        });
      });

      const res = await api.runWorkflow(blueprint);
      if (res && res.error) {
        throw new Error(res.error);
      }
      toast.success('Workflow started');
    } catch (e) {
      console.error("Failed to run workflow", e);
      toast.error('Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
      setIsRunning(false);
    }
  }, [state.nodes, state.edges, updateState, toast]);

  const runNode = useCallback(async (nodeId: string) => {
    const blueprint = {
      nodes: state.nodes.map(n => ({
        id: n.id,
        type: n.data.type,
        data: n.data.config,
        position: n.position
      })),
      edges: state.edges.map(e => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle
      }))
    };

    try {
      setIsRunning(true);
      updateState(draft => {
        draft.nodes.forEach(node => {
          node.data.status = 'idle';
          node.data.errorMessage = undefined;
        });
      });

      const res = await api.runNode(blueprint, nodeId);
      if (res && res.error) {
        throw new Error(res.error);
      }
      toast.success(`Node ${nodeId} execution started`);
    } catch (e) {
      console.error(`Failed to run node ${nodeId}`, e);
      toast.error(`Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
      setIsRunning(false);
    }
  }, [state.nodes, state.edges, updateState, toast]);

  return {
    state,
    nodeTypes,
    workflows,
    currentWorkflowId,
    isRunning,
    onNodesChange,
    onEdgesChange,
    onNodeDragStop,
    onConnect,
    addNode,
    deleteNode,
    updateNodeData,
    runWorkflow,
    runNode,
    saveWorkflow,
    loadWorkflow,
    deleteWorkflow,
    renameWorkflow,
    createNewWorkflow
  };
}
