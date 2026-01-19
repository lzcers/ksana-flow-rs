import { produce } from 'immer';
import type { WorkflowState, Node, NodeData, NodeChange } from './types';
import { applyNodeChangesXyflow } from './utils';


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


export const applyNodeChanges = (
  state: WorkflowState,
  changes: NodeChange[]
): WorkflowState => {
  return produce(state, (draft) => {
    const updatedNodes = applyNodeChangesXyflow(changes, draft.nodes);
    draft.nodes = updatedNodes as any[];
  });
};


export const selectNode = (state: WorkflowState, nodeId: string | null): WorkflowState => {
  return produce(state, (draft) => {
    draft.selectedNodeId = nodeId;

    draft.nodes.forEach(node => {
      node.selected = node.id === nodeId;
    });
  });
};


export const setNodes = (state: WorkflowState, nodes: Node[]): WorkflowState => {
  return produce(state, (draft) => {
    draft.nodes = nodes as any[];
  });
};
