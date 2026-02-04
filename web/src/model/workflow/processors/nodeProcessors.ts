/**
 * Node 处理器函数
 * 纯函数，接收 state 和 command，返回新的 state
 */

import { produce, type Draft, type Immutable } from 'immer';
import type { WorkflowState, Node, NodeData, NodeChange } from '../types';
import type {
  AddNodeCommand,
  RemoveNodeCommand,
  UpdateNodeDataCommand,
  UpdateNodePositionCommand,
  UpdateNodeDimensionsCommand,
  SelectNodeCommand,
  ApplyNodeChangesCommand,
  UpdateNodeStatusCommand,
  UpdateNodeInputCommand,
  UpdateNodeInputsCommand,
  UpdateNodeOutputCommand,
} from '../commands';
import { applyNodeChangesXyflow, getNextNodeId } from '../utils';

export { getNextNodeId }; // Re-export for compatibility if needed within processors module

// 默认节点数据
const getDefaultNodeData = (type: string): Partial<NodeData> => {
  const defaults: Record<string, Partial<NodeData>> = {
    LLMNode: {
      label: 'LLM',
      config: { model: 'gpt-4', temperature: 0.7 },
    },
    TextNode: {
      label: 'Text',
      config: { content: '' },
    },
    SubgraphNode: {
      label: 'Subgraph',
      expanded: true,
      expandedSize: { width: 400, height: 300 },
      collapsedSize: { width: 200, height: 50 },
    },
    MapNode: {
      label: 'Map',
      expanded: true,
      expandedSize: { width: 400, height: 300 },
      collapsedSize: { width: 200, height: 50 },
    },
  };
  return defaults[type] || { label: type };
};

// ===== 处理器函数 =====

export const processAddNode = (
  state: Immutable<WorkflowState>,
  command: AddNodeCommand
): Immutable<WorkflowState> => {
  const { id: requestedId, nodeType, position, data } = command.payload;

  return produce(state, (draft) => {
    const id = requestedId ?? getNextNodeId(draft.nodes, nodeType);
    const newNode: Node = {
      id,
      type: nodeType,
      position,
      data: {
        ...getDefaultNodeData(nodeType),
        ...data,
        label: (data as any)?.label ?? id,
        status: 'idle',
      },
    };
    draft.nodes.push(newNode as any);
  });
};

export const processRemoveNode = (
  state: Immutable<WorkflowState>,
  command: RemoveNodeCommand
): Immutable<WorkflowState> => {
  const { id } = command.payload;

  return produce(state, (draft) => {
    draft.nodes = draft.nodes.filter((n) => n.id !== id);
    draft.edges = draft.edges.filter(
      (e) => e.source !== id && e.target !== id
    );
    if (draft.selectedNodeId === id) {
      draft.selectedNodeId = null;
    }
  });
};

export const processUpdateNodeData = (
  state: Immutable<WorkflowState>,
  command: UpdateNodeDataCommand
): Immutable<WorkflowState> => {
  const { id, data } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      node.data = { ...node.data, ...data };
    }
    syncEdgeHighlighting(draft);
  });
};

export const processUpdateNodePosition = (
  state: Immutable<WorkflowState>,
  command: UpdateNodePositionCommand
): Immutable<WorkflowState> => {
  const { id, position } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      node.position = position;
    }
  });
};

export const processUpdateNodeDimensions = (
  state: Immutable<WorkflowState>,
  command: UpdateNodeDimensionsCommand
): Immutable<WorkflowState> => {
  const { id, width, height } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      if (!node.style) node.style = {};
      (node.style as any).width = width;
      (node.style as any).height = height;
      node.width = width;
      node.height = height;

      // 处理子图/Map 节点的尺寸
      if (node.type === 'SubgraphNode' || node.type === 'MapNode') {
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

export const processUpdateNodeStatus = (
  state: Immutable<WorkflowState>,
  command: UpdateNodeStatusCommand
): Immutable<WorkflowState> => {
  const { id, status, errorMessage } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      if (!node.data) node.data = {};
      node.data.status = status;
      if (errorMessage !== undefined) {
        node.data.errorMessage = errorMessage;
      }
    }
    syncEdgeHighlighting(draft);
  });
};

export const processUpdateNodeInput = (
  state: Immutable<WorkflowState>,
  command: UpdateNodeInputCommand
): Immutable<WorkflowState> => {
  const { id, key, value } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      if (!node.data) node.data = {};
      if (!node.data.inputs) node.data.inputs = {};
      node.data.inputs[key] = value;
    }
  });
};

export const processUpdateNodeInputs = (
  state: Immutable<WorkflowState>,
  command: UpdateNodeInputsCommand
): Immutable<WorkflowState> => {
  const { id, inputs } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      if (!node.data) node.data = {};
      node.data.inputs = { ...node.data.inputs, ...inputs };
    }
  });
};

export const processUpdateNodeOutput = (
  state: Immutable<WorkflowState>,
  command: UpdateNodeOutputCommand
): Immutable<WorkflowState> => {
  const { id, key, value } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      if (!node.data) node.data = {};
      if (!node.data.outputs) node.data.outputs = {};
      node.data.outputs[key] = value;
    }
  });
};

export const processSelectNode = (
  state: Immutable<WorkflowState>,
  command: SelectNodeCommand
): Immutable<WorkflowState> => {
  const { id } = command.payload;

  return produce(state, (draft) => {
    draft.selectedNodeId = id;

    draft.nodes.forEach((node) => {
      node.selected = node.id === id;
    });

    syncEdgeHighlighting(draft);
  });
};

export const processApplyNodeChanges = (
  state: Immutable<WorkflowState>,
  command: ApplyNodeChangesCommand
): Immutable<WorkflowState> => {
  const { changes } = command.payload;

  return produce(state, (draft) => {
    const updatedNodes = applyNodeChangesXyflow(changes as NodeChange[], draft.nodes);
    draft.nodes = updatedNodes as any[];

    changes.forEach((change) => {
      if (change.type === 'select') {
        if ((change as any).selected) {
          draft.selectedNodeId = change.id;
        } else if (draft.selectedNodeId === change.id) {
          draft.selectedNodeId = null;
        }
      }
    });

    syncEdgeHighlighting(draft);
  });
};

// ===== 辅助函数 =====

const syncEdgeHighlighting = (draft: Draft<WorkflowState>) => {
  const selectedNodeIds = new Set(
    draft.nodes
      .filter((n) => n.selected || n.data?.status === 'running')
      .map((n) => n.id)
  );

  draft.edges.forEach((edge) => {
    if (selectedNodeIds.has(edge.source)) {
      (edge).animated = true;
      edge.style = { ...edge.style, stroke: '#3b82f6', strokeWidth: 3 };
    } else {
      (edge as any).animated = false;
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
