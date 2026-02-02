/**
 * Graph 处理器函数
 * 处理复杂的图操作，如分组、粘贴、子图切换等
 */

import { produce } from 'immer';
import type { WorkflowState, Node, Edge, NodeData } from '../types';
import type {
  PasteNodesCommand,
  GroupNodesCommand,
  ToggleSubgraphCommand,
  SetNodesCommand,
  BatchCommand,
} from '../commands';
import { getNextNodeId } from './nodeProcessors';

// ===== 辅助函数 =====

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

export const processPasteNodes = (
  state: WorkflowState,
  command: PasteNodesCommand
): WorkflowState => {
  const { nodes: newNodes, edges: newEdges } = command.payload;

  return produce(state, (draft) => {
    // 取消所有现有选择
    draft.nodes.forEach((n) => (n.selected = false));
    draft.edges.forEach((e) => ((e as any).selected = false));
    draft.selectedNodeId = null;

    const idMap = new Map<string, string>();

    // 处理节点
    newNodes.forEach((node) => {
      const type =
        node.type && typeof node.type === 'string' ? node.type : 'node';
      const newId = getNextNodeId(draft.nodes, type);
      idMap.set(node.id, newId);

      const newNode: Node = {
        ...node,
        id: newId,
        selected: true,
        dragging: false,
        data: {
          ...node.data,
          status: 'idle',
        },
      };

      draft.nodes.push(newNode as any);
      draft.selectedNodeId = newId;
    });

    // 处理边
    newEdges.forEach((edge) => {
      const newSource = idMap.get(edge.source);
      const newTarget = idMap.get(edge.target);

      if (newSource && newTarget) {
        const newEdge: Edge = {
          ...edge,
          id: `e${newSource}-${newTarget}-${Date.now()}-${Math.random()
            .toString(36)
            .substr(2, 5)}`,
          source: newSource,
          target: newTarget,
          selected: true,
        };
        draft.edges.push(newEdge);
      }
    });
  });
};

export const processGroupNodes = (
  state: WorkflowState,
  command: GroupNodesCommand
): WorkflowState => {
  const { nodeIds } = command.payload;

  return produce(state, (draft) => {
    if (nodeIds.length === 0) return;

    // 计算边界框
    const nodesToGroup = draft.nodes.filter((n) => nodeIds.includes(n.id));
    if (nodesToGroup.length === 0) return;

    const minX = Math.min(...nodesToGroup.map((n) => n.position.x)) - 20;
    const minY = Math.min(...nodesToGroup.map((n) => n.position.y)) - 40;
    const maxX = Math.max(
      ...nodesToGroup.map((n) => n.position.x + (n.width || 200))
    );
    const maxY = Math.max(
      ...nodesToGroup.map((n) => n.position.y + (n.height || 50))
    );

    // 创建 Subgraph 节点
    const subgraphId = getNextNodeId(draft.nodes, 'Subgraph');
    const subgraphNode: Node = {
      id: subgraphId,
      type: 'SubgraphNode',
      position: { x: minX, y: minY },
      width: maxX - minX + 40,
      height: maxY - minY + 40,
      data: {
        label: `Subgraph ${subgraphId.split('-')[1]}`,
        expanded: true,
        expandedSize: { width: maxX - minX + 40, height: maxY - minY + 40 },
        collapsedSize: { width: 200, height: 50 },
      },
    };

    draft.nodes.push(subgraphNode as any);

    // 标记子节点
    nodesToGroup.forEach((node) => {
      if (!node.data) node.data = {};
      (node.data as any).parentId = subgraphId;
    });
  });
};

export const processToggleSubgraph = (
  state: WorkflowState,
  command: ToggleSubgraphCommand
): WorkflowState => {
  const { nodeId } = command.payload;

  return produce(state, (draft) => {
    const subgraphNode = draft.nodes.find((n) => n.id === nodeId);
    if (
      !subgraphNode ||
      (subgraphNode.type !== 'SubgraphNode' &&
        subgraphNode.type !== 'MapNode')
    )
      return;

    const isExpanded = subgraphNode.data?.expanded !== false;
    const expandedSize = (subgraphNode.data?.expandedSize || {
      width: 400,
      height: 300,
    }) as { width: number; height: number };
    const collapsedSize = (subgraphNode.data?.collapsedSize || {
      width: 200,
      height: 50,
    }) as { width: number; height: number };

    // 更新节点尺寸
    if (isExpanded) {
      // 折叠
      (subgraphNode as any).width = collapsedSize.width;
      (subgraphNode as any).height = collapsedSize.height;
    } else {
      // 展开
      (subgraphNode as any).width = expandedSize.width;
      (subgraphNode as any).height = expandedSize.height;
    }

    // 更新 expanded 状态
    subgraphNode.data = {
      ...subgraphNode.data,
      expanded: !isExpanded,
    };

    // 处理子节点可见性
    const childNodes = draft.nodes.filter(
      (n) => (n.data as any)?.parentId === nodeId
    );

    if (!isExpanded) {
      // 展开：显示子节点
      childNodes.forEach((child) => {
        (child as any).hidden = false;
      });

      // 显示子节点之间的边
      const childNodeIds = new Set(childNodes.map((n) => n.id));
      draft.edges.forEach((edge) => {
        if (childNodeIds.has(edge.source) && childNodeIds.has(edge.target)) {
          (edge as any).hidden = false;
        }
      });
    } else {
      // 折叠：隐藏子节点
      childNodes.forEach((child) => {
        (child as any).hidden = true;
      });

      // 隐藏子节点之间的边
      const childNodeIds = new Set(childNodes.map((n) => n.id));
      draft.edges.forEach((edge) => {
        if (childNodeIds.has(edge.source) && childNodeIds.has(edge.target)) {
          (edge as any).hidden = true;
        }
      });
    }
  });
};

export const processSetNodes = (
  state: WorkflowState,
  command: SetNodesCommand
): WorkflowState => {
  const { nodes } = command.payload;

  return produce(state, (draft) => {
    draft.nodes = nodes as any[];
  });
};

export const processBatch = (
  state: WorkflowState,
  command: BatchCommand
): WorkflowState => {
  const { commands } = command.payload;

  // 注意：这里需要递归处理批量命令
  // 为避免循环依赖，batch 处理器应该由外部调用时传入处理器映射
  // 或者使用 CommandBus 来分发

  // 当前实现：直接返回原状态，实际处理由 CommandBus 完成
  return state;
};
