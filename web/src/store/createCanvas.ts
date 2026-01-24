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

  setConnectionState: (connecting, sourceId = null) => set({ isConnecting: connecting, connectionSourceId: sourceId })
});
