import type { StateCreator } from 'zustand';
import type { StoreState, Canvas } from './types';
import type { Node, NodeChange, EdgeChange, Connection } from '../model/types';

import { workflowModel } from './workflowModel';

const sortNodesByParent = (nodes: Node[]): Node[] => {
  const idSet = new Set(nodes.map((n) => n.id));
  const childrenByParent = new Map<string, Node[]>();

  nodes.forEach((n) => {
    if (!n.parentId || !idSet.has(n.parentId)) return;
    const children = childrenByParent.get(n.parentId);
    if (children) {
      children.push(n);
    } else {
      childrenByParent.set(n.parentId, [n]);
    }
  });

  const result: Node[] = [];
  const visited = new Set<string>();

  const visit = (node: Node) => {
    if (visited.has(node.id)) return;
    visited.add(node.id);
    result.push(node);
    const children = childrenByParent.get(node.id);
    if (children) {
      children.forEach(visit);
    }
  };

  nodes.forEach((n) => {
    if (!n.parentId || !idSet.has(n.parentId)) {
      visit(n);
    }
  });

  nodes.forEach((n) => visit(n));

  return result;
};

export const createCanvas: StateCreator<StoreState, [], [], Canvas> = (set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,
  isConnecting: false,
  connectionSourceId: null,

  history: { past: [], future: [] },

  pushHistory: () => {
    // Deprecated: History is now managed by WorkflowModel
  },

  undo: () => {
    workflowModel.undo();
  },

  redo: () => {
    workflowModel.redo();
  },

  canUndo: false,
  canRedo: false,

  setNodes: (nodes) => workflowModel.dispatchers.setNodes(nodes as any),
  setEdges: (edges) => workflowModel.dispatchers.setEdges(edges as any),

  pasteNodes: (nodes, edges) => {
    workflowModel.dispatchers.pasteNodes(nodes as any, edges as any);
  },

  onNodesChange: (changes: NodeChange[]) => {
    // Snapshot on remove (e.g. Backspace key)
    workflowModel.dispatch({
      type: 'APPLY_NODE_CHANGES',
      payload: { changes },
    });
  },

  onEdgesChange: (changes: EdgeChange[]) => {
    // Snapshot on remove (e.g. Backspace key or removing edge)
    const { edges } = get();
    const edgeById = new Map(edges.map((e) => [e.id, e] as const));

    const removeOriginalEdgeIds: string[] = [];
    const coreChanges: EdgeChange[] = [];

    for (const change of changes) {
      if (!('id' in change)) {
        coreChanges.push(change);
        continue;
      }

      const edge = edgeById.get(change.id) as any;
      const proxy = edge?.data?.__uiSubgraphEdge;
      if (proxy) {
        if (change.type === 'remove' && typeof proxy.originalEdgeId === 'string') {
          removeOriginalEdgeIds.push(proxy.originalEdgeId);
        }
        continue;
      }
      coreChanges.push(change);
    }

    removeOriginalEdgeIds.forEach((id) => workflowModel.dispatchers.removeEdge(id));

    if (coreChanges.length > 0) {
      workflowModel.dispatch({
        type: 'APPLY_EDGE_CHANGES',
        payload: { changes: coreChanges },
      });
    }
  },

  onNodeDragStop: (_event: any, draggedNode: any) => {
    const { nodes } = workflowModel.getSnapshot();
    const nodeId = typeof draggedNode?.id === 'string' ? draggedNode.id : null;
    if (!nodeId) return;

    const nodeById = new Map(nodes.map((n) => [n.id, n] as const));
    const node = nodeById.get(nodeId);
    if (!node) return;

    const isGroup = (n: Node) => n.type === 'SubgraphNode' || n.type === 'MapNode';
    const isDropTargetGroup = (n: Node) =>
      isGroup(n) && n.id !== nodeId && n.hidden !== true && (n.data as any)?.expanded !== false;

    const toNumber = (v: unknown): number | undefined => {
      if (typeof v === 'number' && Number.isFinite(v)) return v;
      if (typeof v === 'string') {
        const n = parseFloat(v);
        if (Number.isFinite(n)) return n;
      }
      return undefined;
    };

    const getSize = (n: Node): { width: number; height: number } => {
      const styleW = toNumber((n.style as any)?.width);
      const styleH = toNumber((n.style as any)?.height);
      const width = (n.measured?.width ?? styleW ?? n.width ?? (isGroup(n) ? 300 : 150)) as number;
      const height = (n.measured?.height ?? styleH ?? n.height ?? (isGroup(n) ? 200 : 50)) as number;
      return { width, height };
    };

    const getAbsPos = (n: Node): { x: number; y: number } => {
      let x = n.position.x;
      let y = n.position.y;
      let cur: Node | undefined = n;
      const visited = new Set<string>();
      while (cur?.parentId) {
        if (!visited.add(cur.parentId)) break;
        const p = nodeById.get(cur.parentId);
        if (!p) break;
        x += p.position.x;
        y += p.position.y;
        cur = p;
      }
      return { x, y };
    };

    const depthOf = (n: Node): number => {
      let depth = 0;
      let cur: Node | undefined = n;
      const visited = new Set<string>();
      while (cur?.parentId) {
        if (!visited.add(cur.parentId)) break;
        const p = nodeById.get(cur.parentId);
        if (!p) break;
        depth += 1;
        cur = p;
      }
      return depth;
    };

    const isAncestor = (ancestorId: string, descendantId: string): boolean => {
      let cur = nodeById.get(descendantId);
      const visited = new Set<string>();
      while (cur?.parentId) {
        if (cur.parentId === ancestorId) return true;
        if (!visited.add(cur.parentId)) break;
        cur = nodeById.get(cur.parentId);
      }
      return false;
    };

    const nodeSize = getSize(node);
    const nodeAbs = getAbsPos(node);
    const center = { x: nodeAbs.x + nodeSize.width / 2, y: nodeAbs.y + nodeSize.height / 2 };

    let targetGroup: Node | null = null;
    let bestDepth = -1;
    for (const g of nodes) {
      if (!isDropTargetGroup(g)) continue;
      if (isGroup(node) && isAncestor(node.id, g.id)) continue;

      const gAbs = getAbsPos(g);
      const gSize = getSize(g);
      const inside =
        center.x >= gAbs.x &&
        center.x <= gAbs.x + gSize.width &&
        center.y >= gAbs.y &&
        center.y <= gAbs.y + gSize.height;
      if (!inside) continue;
      const d = depthOf(g);
      if (d > bestDepth) {
        bestDepth = d;
        targetGroup = g;
      }
    }

    const currentParent = node.parentId ? nodeById.get(node.parentId) : undefined;
    const currentParentIsGroup = currentParent ? isGroup(currentParent) : false;

    let nextParentId: string | undefined | null = node.parentId ?? undefined;
    if (targetGroup) {
      if (targetGroup.id !== node.parentId) nextParentId = targetGroup.id;
    } else if (currentParentIsGroup && currentParent) {
      const pAbs = getAbsPos(currentParent);
      const pSize = getSize(currentParent);
      const insideParent =
        center.x >= pAbs.x &&
        center.x <= pAbs.x + pSize.width &&
        center.y >= pAbs.y &&
        center.y <= pAbs.y + pSize.height;
      if (!insideParent) nextParentId = currentParent.parentId ?? undefined;
    }

    if (nextParentId === (node.parentId ?? undefined)) return;

    const nextParent = nextParentId ? nodeById.get(nextParentId) : undefined;
    const nextParentAbs = nextParent ? getAbsPos(nextParent) : { x: 0, y: 0 };
    const nextPos = nextParentId
      ? { x: nodeAbs.x - nextParentAbs.x, y: nodeAbs.y - nextParentAbs.y }
      : { x: nodeAbs.x, y: nodeAbs.y };

    const nextNodes = nodes.map((n) => {
      if (n.id !== nodeId) return n;
      return {
        ...n,
        parentId: nextParentId || undefined,
        extent: nextParentId ? ('parent' as const) : undefined,
        position: nextPos,
        expandParent: undefined,
      };
    });

    workflowModel.dispatchers.setNodes(sortNodesByParent(nextNodes) as any);
  },

  onConnect: (connection: Connection) => {
    workflowModel.dispatchers.onConnect(connection as any);
  },

  addNode: (type: string, position: { x: number; y: number } = { x: 300, y: 200 }) => {
    const { nodeTypes } = get();
    const { nodes } = workflowModel.getSnapshot();
    // Find all nodes of the same type and extract their numbers
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
    const id = `${type}-${nextNum}`;
    const meta = nodeTypes.find(t => t.name === type);
    workflowModel.dispatchers.addNode(type, position as any, {
      id,
      data: {
        label: type,
        description: meta?.description || '',
        config: meta?.config || {},
        status: 'idle',
      },
    });
    workflowModel.dispatchers.selectNode(id);
  },

  deleteNode: (id: string) => {
    workflowModel.dispatchers.deleteNode(id);
  },

  updateNodeData: (id: string, data: Record<string, any>) => {
    workflowModel.dispatchers.updateNodeData(id, data);
  },

  updateNodeDimensions: (id: string, width: number, height: number) => {
    workflowModel.dispatchers.updateNodeDimensions(id, width, height);
  },

  selectNode: (id: string | null) => {
    workflowModel.dispatchers.selectNode(id);
  },

  setConnectionState: (connecting, sourceId = null) => set({ isConnecting: connecting, connectionSourceId: sourceId }),

  groupNodes: (nodeIds: string[]) => {
    if (nodeIds.length < 1) return;
    const { nodes } = workflowModel.getSnapshot();
    const selectedNodes = nodes.filter(n => nodeIds.includes(n.id));
    if (selectedNodes.length === 0) return;

    // Check if all selected nodes share the same parent
    const firstParentId = selectedNodes[0].parentId;
    const sameParent = selectedNodes.every(n => n.parentId === firstParentId);
    if (!sameParent) {
      console.warn("Cannot group nodes from different parents");
      return;
    }

    // Calculate bounds
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    selectedNodes.forEach(n => {
      minX = Math.min(minX, n.position.x);
      minY = Math.min(minY, n.position.y);
      const w = (n.measured?.width ?? (typeof n.style?.width === 'number' ? n.style.width : 150)) as number;
      const h = (n.measured?.height ?? (typeof n.style?.height === 'number' ? n.style.height : 50)) as number;
      maxX = Math.max(maxX, n.position.x + w);
      maxY = Math.max(maxY, n.position.y + h);
    });

    const padding = 40;
    const groupX = minX - padding;
    const groupY = minY - padding - 40; // Extra space for header
    const groupWidth = maxX - minX + padding * 2;
    const groupHeight = maxY - minY + padding * 2 + 40;

    // Generate ID
    const type = 'SubgraphNode';
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
    const groupId = `${type}-${nextNum}`;
    
    const groupNode: Node = {
      id: groupId,
      type,
      position: { x: groupX, y: groupY },
      style: { width: groupWidth, height: groupHeight },
      data: {
        label: 'Group',
        expanded: true,
        expandedSize: { width: groupWidth, height: groupHeight },
        collapsedSize: { width: 180, height: 80 },
      },
      parentId: firstParentId,
    };

    const updatedNodes = nodes.map(n => {
      if (nodeIds.includes(n.id)) {
        // Calculate position relative to parent (groupNode)
        const relativeX = n.position.x - groupX;
        const relativeY = n.position.y - groupY;
        return {
          ...n,
          parentId: groupId,
          extent: 'parent' as const,
          position: {
            x: relativeX,
            y: relativeY,
          },
          // Remove expandParent to prevent auto-resize behavior
          expandParent: undefined,
        };
      }
      return n;
    });

    workflowModel.dispatchers.setNodes(
      sortNodesByParent([...updatedNodes, groupNode]) as any
    );
    workflowModel.dispatchers.selectNode(groupId);
  },

  toggleSubgraph: (nodeId: string) => {
    const snapshot = workflowModel.getSnapshot();
    const node = snapshot.nodes.find((n) => n.id === nodeId);
    if (!node || (node.type !== 'SubgraphNode' && node.type !== 'MapNode')) return;
    workflowModel.dispatchers.toggleSubgraph(nodeId);
  },
});
