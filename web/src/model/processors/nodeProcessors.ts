/**
 * Node 处理器函数
 * 纯函数，接收 state 和 command，返回新的 state
 */

import { produce } from 'immer';
import type { WorkflowState, Node, NodeData, NodeChange } from '../types';
import type {
  AddNodeCommand,
  RemoveNodeCommand,
  UpdateNodeDataCommand,
  UpdateNodePositionCommand,
  UpdateNodeDimensionsCommand,
  SelectNodeCommand,
  ApplyNodeChangesCommand,
} from '../commands';
import { applyNodeChangesXyflow } from '../utils';

// ===== ID 生成 =====

export const getNextNodeId = (nodes: Node[], type: string): string => {
  const sameTypeNodes = nodes.filter((n) => n.id.startsWith(`${type}-`));
  let nextNum = 1;
  if (sameTypeNodes.length > 0) {
    const nums = sameTypeNodes.map((n) => {
      const parts = n.id.split('-');
      const lastPart = parts[parts.length - 1];
      const num = parseInt(lastPart, 10);
      return isNaN(num) ? 0 : num;
    });
    nextNum = Math.max(...nums) + 1;
  }
  return `${type}-${nextNum}`;
};

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
  state: WorkflowState,
  command: AddNodeCommand
): WorkflowState => {
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
  state: WorkflowState,
  command: RemoveNodeCommand
): WorkflowState => {
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
  state: WorkflowState,
  command: UpdateNodeDataCommand
): WorkflowState => {
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
  state: WorkflowState,
  command: UpdateNodePositionCommand
): WorkflowState => {
  const { id, position } = command.payload;

  return produce(state, (draft) => {
    const node = draft.nodes.find((n) => n.id === id);
    if (node) {
      node.position = position;
    }
  });
};

export const processUpdateNodeDimensions = (
  state: WorkflowState,
  command: UpdateNodeDimensionsCommand
): WorkflowState => {
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

export const processSelectNode = (
  state: WorkflowState,
  command: SelectNodeCommand
): WorkflowState => {
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
  state: WorkflowState,
  command: ApplyNodeChangesCommand
): WorkflowState => {
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

const syncEdgeHighlighting = (draft: WorkflowState) => {
  const selectedNodeIds = new Set(
    draft.nodes
      .filter((n) => n.selected || n.data?.status === 'running')
      .map((n) => n.id)
  );

  draft.edges.forEach((edge) => {
    if (selectedNodeIds.has(edge.source)) {
      (edge as any).animated = true;
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
