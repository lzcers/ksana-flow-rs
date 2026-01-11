import { useCallback, useEffect, useState } from 'react';
import { useImmer } from 'use-immer';
import {
  addEdge as flowAddEdge,
  applyNodeChanges,
  applyEdgeChanges,
  type NodeChange,
  type EdgeChange,
  type Connection,
  type Edge,
  type Node
} from '@xyflow/react';
import type { WorkflowState, WorkflowNodeData } from '../types/workflow';
import * as api from '../api';

const INITIAL_STATE: WorkflowState = {
  nodes: [],
  edges: [],
  selectedNodeId: null,
};

export function useWorkflow() {
  const [state, updateState] = useImmer<WorkflowState>(INITIAL_STATE);
  const [nodeTypes, setNodeTypes] = useState<api.NodeMetadata[]>([]);

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
    // Load initial data
    const load = async () => {
      const types = await api.fetchNodes();
      setNodeTypes(types);

      const graph = await api.fetchGraph();
      updateState(draft => {
        draft.nodes = graph.nodes.map((n: any) => ({
          id: n.id,
          type: 'workflow',
          position: n.position || { x: 0, y: 0 },
          data: {
            label: n.type_name,
            type: n.type_name,
            description: types.find((t: any) => t.name === n.type_name)?.description || '',
            config: n.config
          }
        }));
        draft.edges = graph.edges.map((e: any) => ({
          id: e.id,
          source: e.source,
          target: e.target,
          type: 'smoothstep'
        }));
      });
    };
    load();
  }, [updateState]);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    // Handle removals
    changes.forEach(change => {
      if (change.type === 'remove') {
        api.removeNode(change.id);
      }
    });

    updateState(draft => {
      draft.nodes = applyNodeChanges(changes, draft.nodes) as any;

      // Update selectedNodeId based on changes
      const selectChange = changes.find(c => c.type === 'select');
      if (selectChange && 'selected' in selectChange) {
        if (selectChange.selected) {
          draft.selectedNodeId = selectChange.id;
        } else if (draft.selectedNodeId === selectChange.id) {
          draft.selectedNodeId = null;
        }
      }

      // TODO: Sync position changes to server if needed
    });
  }, [updateState]);

  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    // Handle removals
    changes.forEach(change => {
      if (change.type === 'remove') {
        api.removeEdge(change.id);
      }
    });

    updateState(draft => {
      draft.edges = applyEdgeChanges(changes, draft.edges) as any;
    });
  }, [updateState]);

  const onNodeDragStop = useCallback((_event: any, node: Node) => {
    api.updateNodePosition(node.id, node.position);
  }, []);

  const onConnect = useCallback(async (connection: Connection) => {
    const edgeId = `${connection.source}-${connection.target}`;
    // Optimistic update
    updateState(draft => {
      draft.edges = flowAddEdge({ ...connection, id: edgeId, type: 'smoothstep' }, draft.edges);
    });

    try {
      await api.addEdge({
        id: edgeId,
        source: connection.source,
        target: connection.target
      });
    } catch (e) {
      console.error("Failed to add edge", e);
      // Revert if failed (omitted for brevity)
    }
  }, [updateState]);

  const addNode = useCallback(async (type: string, position: { x: number; y: number } = { x: 300, y: 200 }) => {
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

    // Optimistic update
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

    try {
      await api.addNode({
        id,
        type_name: type,
        config: meta?.config || {},
        position
      });
    } catch (e) {
      console.error("Failed to add node", e);
    }
  }, [updateState, nodeTypes, state.nodes]);

  const deleteNode = useCallback((id: string) => {
    updateState(draft => {
      draft.nodes = draft.nodes.filter(n => n.id !== id);
      draft.edges = draft.edges.filter(e => e.source !== id && e.target !== id);
      if (draft.selectedNodeId === id) draft.selectedNodeId = null;
    });
    api.removeNode(id);
  }, [updateState]);

  const updateNodeData = useCallback((id: string, data: Partial<WorkflowNodeData>) => {
    updateState(draft => {
      const node = draft.nodes.find(n => n.id === id);
      if (node) {
        node.data = { ...node.data, ...data };
        // TODO: Sync config update to server
      }
    });
  }, [updateState]);

  const runWorkflow = useCallback(async () => {
    updateState(draft => {
      draft.nodes.forEach(node => {
        node.data.status = 'idle';
        node.data.errorMessage = undefined;
      });
    });
    await api.runFlow();
  }, [updateState]);

  return {
    state,
    nodeTypes,
    onNodesChange,
    onEdgesChange,
    onNodeDragStop,
    onConnect,
    addNode,
    deleteNode,
    updateNodeData,
    runWorkflow
  };
}
