import { produce } from 'immer';
import type { WorkflowState, Node, NodeData, NodeChange, Edge } from './types';
import { applyNodeChangesXyflow } from './utils';


export const getNextNodeId = (nodes: Node[], type: string): string => {
  const sameTypeNodes = nodes.filter(n => n.id.startsWith(`${type}-`));
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
  return `${type}-${nextNum}`;
};

export const addNode = (state: WorkflowState, node: Node): WorkflowState => {
  return produce(state, (draft) => {
    draft.nodes.push(node as any);
  });
};


export const removeNode = (state: WorkflowState, nodeId: string): WorkflowState => {
  return produce(state, (draft) => {
    draft.nodes = draft.nodes.filter((n) => n.id !== nodeId);
    draft.edges = draft.edges.filter(
      (e) => e.source !== nodeId && e.target !== nodeId
    );
    if (draft.selectedNodeId === nodeId) {
      draft.selectedNodeId = null;
    }
  });
};


export const updateNodeData = (
  state: WorkflowState,
  nodeId: string,
  data: Partial<NodeData>
): WorkflowState => {
  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === nodeId);
    if (node) {
      node.data = { ...node.data, ...data };
    }
    syncEdgeHighlighting(draft);
  });
};

export const updateNodeDimensions = (
  state: WorkflowState,
  nodeId: string,
  width: number,
  height: number
): WorkflowState => {
  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === nodeId);
    if (node) {
      if (!node.style) node.style = {};
      node.style.width = width;
      node.style.height = height;
      node.width = width;
      node.height = height;

      if (node.type === 'SubgraphNode') {
        const expanded = node.data?.expanded !== false;
        const size = { width, height };
        node.data = {
          ...node.data,
          expandedSize: expanded ? size : (node.data?.expandedSize as any),
          collapsedSize: expanded ? (node.data?.collapsedSize as any) : size,
        };
      }
    }
  });
};

export const updateNodeStatus = (
  state: WorkflowState,
  nodeId: string,
  status: 'idle' | 'running' | 'completed' | 'error',
  errorMessage?: string
): WorkflowState => {
  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === nodeId);
    if (node) {
      if (node.data) {
        node.data.status = status;
        if (errorMessage !== undefined) {
          node.data.errorMessage = errorMessage;
        }
      }
    }
    syncEdgeHighlighting(draft);
  });
};

export const updateNodeInput = (
  state: WorkflowState,
  nodeId: string,
  key: string,
  value: any
): WorkflowState => {
  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === nodeId);
    if (node) {
      if (!node.data.inputs) node.data.inputs = {};
      node.data.inputs[key] = value;
    }
  });
};

export const updateNodeInputs = (
  state: WorkflowState,
  nodeId: string,
  inputs: Record<string, any>
): WorkflowState => {
  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === nodeId);
    if (node) {
      node.data.inputs = { ...node.data.inputs, ...inputs };
    }
  });
};

export const updateNodeOutput = (
  state: WorkflowState,
  nodeId: string,
  key: string,
  value: any
): WorkflowState => {
  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === nodeId);
    if (node) {
      if (!node.data.outputs) node.data.outputs = {};
      node.data.outputs[key] = value;
    }
  });
};


export const resetWorkflowExecutionState = (state: WorkflowState): WorkflowState => {
  return produce(state, (draft) => {
    draft.nodes.forEach((node) => {
      if (node.data) {
        node.data.status = 'idle';
        node.data.errorMessage = undefined;
        node.data.isOutputStream = undefined;
      }
    });
  });
};

const syncEdgeHighlighting = (draft: WorkflowState) => {
  const selectedNodeIds = new Set(draft.nodes.filter(n => n.selected || n.data.status === 'running').map(n => n.id));

  draft.edges.forEach(edge => {
    if (selectedNodeIds.has(edge.source)) {
      edge.animated = true;
      edge.style = { ...edge.style, stroke: '#3b82f6', strokeWidth: 3 };
    } else {
      edge.animated = false;
      if (edge.style) {
        delete edge.style.stroke;
        delete edge.style.strokeWidth;
        if (Object.keys(edge.style).length === 0) {
          edge.style = undefined;
        }
      }
    }
  });
};


export const applyNodeChanges = (
  state: WorkflowState,
  changes: NodeChange[]
): WorkflowState => {
  return produce(state, (draft) => {
    const updatedNodes = applyNodeChangesXyflow(changes, draft.nodes);
    draft.nodes = updatedNodes as any[];

    changes.forEach((change) => {
      if (change.type === 'select') {
        if (change.selected) {
          draft.selectedNodeId = change.id;
        } else if (draft.selectedNodeId === change.id) {
          draft.selectedNodeId = null;
        }
      }
    });
    syncEdgeHighlighting(draft);
  });
};


export const selectNode = (state: WorkflowState, nodeId: string | null): WorkflowState => {
  return produce(state, (draft) => {
    draft.selectedNodeId = nodeId;

    draft.nodes.forEach(node => {
      node.selected = node.id === nodeId;
    });

    syncEdgeHighlighting(draft);
  });
};


export const setNodes = (state: WorkflowState, nodes: Node[]): WorkflowState => {
  return produce(state, (draft) => {
    draft.nodes = nodes as any[];
  });
};


export const pasteNodes = (state: WorkflowState, newNodes: Node[], newEdges: Edge[]): WorkflowState => {
  return produce(state, (draft) => {
    // Deselect all existing nodes and edges
    draft.nodes.forEach(n => n.selected = false);
    draft.edges.forEach(e => e.selected = false);
    draft.selectedNodeId = null;

    const idMap = new Map<string, string>();

    // Process Nodes
    newNodes.forEach(node => {
      // Ensure we have a valid type for ID generation
      const type = (node.type && typeof node.type === 'string') ? node.type : 'node';
      const newId = getNextNodeId(draft.nodes, type);
      idMap.set(node.id, newId);

      const newNode = {
        ...node,
        id: newId,
        selected: true,
        dragging: false,
        data: {
          ...node.data,
          status: 'idle', // Reset status
        }
      };
      draft.nodes.push(newNode as any);
      draft.selectedNodeId = newId; // Set last pasted as selected ID
    });

    // Process Edges
    newEdges.forEach(edge => {
      const newSource = idMap.get(edge.source);
      const newTarget = idMap.get(edge.target);

      if (newSource && newTarget) {
        const newEdge = {
          ...edge,
          id: `e${newSource}-${newTarget}-${Date.now()}-${Math.random().toString(36).substr(2, 5)}`,
          source: newSource,
          target: newTarget,
          selected: true
        };
        draft.edges.push(newEdge);
      }
    });

    syncEdgeHighlighting(draft);
  });
};
