import type { StateCreator } from 'zustand';
import type { StoreState, CanvasSlice } from './types';
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
  setEdges
} from '../model';

export const createCanvasSlice: StateCreator<StoreState, [], [], CanvasSlice> = (set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,

  setNodes: (nodes) => set(state => ({ ...state, ...setNodes(state, nodes) })),
  setEdges: (edges) => set(state => ({ ...state, ...setEdges(state, edges) })),

  onNodesChange: (changes: NodeChange[]) => set(state => ({ ...state, ...applyNodeChanges(state, changes) })),
  onEdgesChange: (changes: EdgeChange[]) => set(state => ({ ...state, ...applyEdgeChanges(state, changes) })),

  onConnect: (connection: Connection) => set(state => ({ ...state, ...onConnect(state, connection) })),

  addNode: (type: string, position: { x: number; y: number } = { x: 300, y: 200 }) => {
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
      type: 'workflow',
      position,
      data: {
        label: type,
        type: type,
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

  deleteNode: (id: string) => set(state => ({ ...state, ...removeNode(state, id) })),

  updateNodeData: (id: string, data: Record<string, any>) => set(state => ({ ...state, ...updateNodeData(state, id, data) })),

  updateNodeDimensions: (id: string, width: number, height: number) => set(state => ({ ...state, ...updateNodeDimensions(state, id, width, height) })),

  selectNode: (id: string | null) => set(state => ({ ...state, ...selectNode(state, id) }))
});
