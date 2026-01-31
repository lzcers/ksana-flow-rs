import type { StateCreator } from 'zustand';
import type { StoreState, Canvas } from './types';
import type { Node, NodeChange, EdgeChange, Connection } from '../model/types';

import {
  addNode,
  removeNode,
  updateNodeData,
  updateNodeDimensions,
  applyNodeChanges,
  applyEdgeChanges,
  onConnect,
  selectNode,
  setNodes,
  setEdges,
  pasteNodes
} from '../model';

export const createCanvas: StateCreator<StoreState, [], [], Canvas> = (set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,
  isConnecting: false,
  connectionSourceId: null,

  history: { past: [], future: [] },

  pushHistory: () => {
    const { nodes, edges, history } = get();
    // Limit history to 50 steps to prevent memory issues
    const newPast = [...history.past, { nodes, edges }].slice(-50);
    set({ history: { past: newPast, future: [] } });
  },

  undo: () => {
    const { history } = get();
    if (history.past.length === 0) return;

    const previous = history.past[history.past.length - 1];
    const newPast = history.past.slice(0, -1);

    const { nodes, edges } = get();

    set({
      nodes: previous.nodes,
      edges: previous.edges,
      history: {
        past: newPast,
        future: [{ nodes, edges }, ...history.future]
      }
    });
  },

  redo: () => {
    const { history } = get();
    if (history.future.length === 0) return;

    const next = history.future[0];
    const newFuture = history.future.slice(1);

    const { nodes, edges } = get();

    set({
      nodes: next.nodes,
      edges: next.edges,
      history: {
        past: [...history.past, { nodes, edges }],
        future: newFuture
      }
    });
  },

  canUndo: () => get().history.past.length > 0,
  canRedo: () => get().history.future.length > 0,

  setNodes: (nodes) => set(state => ({ ...state, ...setNodes(state, nodes) })),
  setEdges: (edges) => set(state => ({ ...state, ...setEdges(state, edges) })),

  pasteNodes: (nodes, edges) => {
    get().pushHistory();
    set(state => ({ ...state, ...pasteNodes(state, nodes, edges) }));
  },

  onNodesChange: (changes: NodeChange[]) => {
    // Snapshot on remove (e.g. Backspace key)
    if (changes.some(c => c.type === 'remove')) {
      get().pushHistory();
    }
    set(state => ({ ...state, ...applyNodeChanges(state, changes) }));
  },

  onEdgesChange: (changes: EdgeChange[]) => {
    // Snapshot on remove (e.g. Backspace key or removing edge)
    if (changes.some(c => c.type === 'remove')) {
      get().pushHistory();
    }
    set(state => ({ ...state, ...applyEdgeChanges(state, changes) }));
  },

  onConnect: (connection: Connection) => {
    get().pushHistory();
    set(state => ({ ...state, ...onConnect(state, connection) }));
  },

  addNode: (type: string, position: { x: number; y: number } = { x: 300, y: 200 }) => {
    get().pushHistory();

    const { nodes, nodeTypes } = get();
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
    const newNode: Node = {
      id,
      type: type as any,
      position,
      data: {
        label: type,
        description: meta?.description || '',
        config: meta?.config || {},
        status: 'idle'
      },
    };

    set(state => {
      let next = addNode(state, newNode);
      next = selectNode(next, id);
      return { ...state, ...next };
    });
  },

  deleteNode: (id: string) => {
    get().pushHistory();
    set(state => ({ ...state, ...removeNode(state, id) }));
  },

  updateNodeData: (id: string, data: Record<string, any>) => set(state => ({ ...state, ...updateNodeData(state, id, data) })),

  updateNodeDimensions: (id: string, width: number, height: number) => set(state => ({ ...state, ...updateNodeDimensions(state, id, width, height) })),

  selectNode: (id: string | null) => set(state => ({ ...state, ...selectNode(state, id) })),

  setConnectionState: (connecting, sourceId = null) => set({ isConnecting: connecting, connectionSourceId: sourceId }),

  groupNodes: (nodeIds: string[]) => {
    if (nodeIds.length < 1) return;
    const { nodes } = get();
    const selectedNodes = nodes.filter(n => nodeIds.includes(n.id));
    if (selectedNodes.length === 0) return;

    // Check if all selected nodes share the same parent
    const firstParentId = selectedNodes[0].parentId;
    const sameParent = selectedNodes.every(n => n.parentId === firstParentId);
    if (!sameParent) {
      console.warn("Cannot group nodes from different parents");
      return;
    }

    get().pushHistory();

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
      data: { label: 'Group', expanded: true },
      parentId: firstParentId,
    };

    const updatedNodes = nodes.map(n => {
      if (nodeIds.includes(n.id)) {
        return {
          ...n,
          parentId: groupId,
          position: {
            x: n.position.x - groupX,
            y: n.position.y - groupY,
          },
          expandParent: true,
        };
      }
      return n;
    });

    set({ nodes: [...updatedNodes, groupNode], selectedNodeId: groupId });
  },

  toggleSubgraph: (nodeId: string) => {
    const { nodes } = get();
    const node = nodes.find(n => n.id === nodeId);
    if (!node || node.type !== 'SubgraphNode') return;

    const isExpanded = node.data.expanded !== false;
    const nextExpanded = !isExpanded;

    const updatedNodes = nodes.map(n => {
      if (n.id === nodeId) {
        if (nextExpanded) {
           const savedSize = node.data.expandedSize as { width: number, height: number } | undefined;
           return {
             ...n,
             style: { 
                ...n.style, 
                width: savedSize?.width ?? 300, 
                height: savedSize?.height ?? 200 
             },
             data: { ...n.data, expanded: true }
           };
        } else {
           const currentWidth = node.measured?.width ?? (typeof node.style?.width === 'number' ? node.style.width : 300);
           const currentHeight = node.measured?.height ?? (typeof node.style?.height === 'number' ? node.style.height : 200);
           return {
             ...n,
             style: { ...n.style, width: 200, height: 60 },
             data: { 
               ...n.data, 
               expanded: false,
               expandedSize: { width: currentWidth, height: currentHeight }
             }
           };
        }
      }
      
      if (n.parentId === nodeId) {
        return {
          ...n,
          hidden: !nextExpanded
        };
      }
      return n;
    });

    set({ nodes: updatedNodes });
  },
});
