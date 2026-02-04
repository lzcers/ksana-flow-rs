import type { StateCreator } from 'zustand';
import type { StoreState, Canvas } from './types';
import type { Node, NodeChange, EdgeChange, Connection } from '../model/workflow/types';
import { sortNodesByParent } from '../model/workflow/utils';
import { rxWorkflowModel } from '.';

export const createCanvas: StateCreator<StoreState, [], [], Canvas> = (set, get) => {
  return {
    nodes: [],
    edges: [],
    selectedNodeId: null,
    isConnecting: false,
    connectionSourceId: null,

    undo: () => {
      rxWorkflowModel.undo();
    },

    redo: () => {
      rxWorkflowModel.redo();
    },

    canUndo: false,
    canRedo: false,

    setNodes: (nodes) => rxWorkflowModel.dispatchers.setNodes(nodes as any),
    setEdges: (edges) => rxWorkflowModel.dispatchers.setEdges(edges as any),

    pasteNodes: (nodes, edges) => {
      rxWorkflowModel.dispatchers.pasteNodes(nodes as any, edges as any);
    },

    onNodesChange: (changes: NodeChange[]) => {
      // Snapshot on remove (e.g. Backspace key)
      rxWorkflowModel.dispatch({
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

      removeOriginalEdgeIds.forEach((id) => rxWorkflowModel.dispatchers.removeEdge(id));

      if (coreChanges.length > 0) {
        rxWorkflowModel.dispatch({
          type: 'APPLY_EDGE_CHANGES',
          payload: { changes: coreChanges },
        });
      }
    },

    onNodeDragStop: (_event: any, draggedNode: any) => {
      const nodeId = typeof draggedNode?.id === 'string' ? draggedNode.id : null;
      if (!nodeId) return;
      rxWorkflowModel.dispatchers.handleNodeDragStop(nodeId);
    },

    onConnect: (connection: Connection) => {
      rxWorkflowModel.dispatchers.onConnect(connection as any);
    },

    addNode: (type: string, position: { x: number; y: number } = { x: 300, y: 200 }) => {
      const { nodeTypes } = get();
      const { nodes } = rxWorkflowModel.getSnapshot();
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
      rxWorkflowModel.dispatchers.addNode(type, position as any, {
        id,
        data: {
          label: type,
          description: meta?.description || '',
          config: meta?.config || {},
          status: 'idle',
        },
      });
      rxWorkflowModel.dispatchers.selectNode(id);
    },

    deleteNode: (id: string) => {
      rxWorkflowModel.dispatchers.deleteNode(id);
    },

    updateNodeData: (id: string, data: Record<string, any>) => {
      rxWorkflowModel.dispatchers.updateNodeData(id, data);
    },

    updateNodeDimensions: (id: string, width: number, height: number) => {
      rxWorkflowModel.dispatchers.updateNodeDimensions(id, width, height);
    },

    selectNode: (id: string | null) => {
      rxWorkflowModel.dispatchers.selectNode(id);
    },

    setConnectionState: (connecting, sourceId = null) => set({ isConnecting: connecting, connectionSourceId: sourceId }),

    groupNodes: (nodeIds: string[]) => {
      if (nodeIds.length < 1) return;
      const { nodes } = rxWorkflowModel.getSnapshot();
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

      rxWorkflowModel.dispatchers.setNodes(
        sortNodesByParent([...updatedNodes, groupNode]) as any
      );
      rxWorkflowModel.dispatchers.selectNode(groupId);
    },

    toggleSubgraph: (nodeId: string) => {
      const snapshot = rxWorkflowModel.getSnapshot();
      const node = snapshot.nodes.find((n) => n.id === nodeId);
      if (!node || (node.type !== 'SubgraphNode' && node.type !== 'MapNode')) return;
      rxWorkflowModel.dispatchers.toggleSubgraph(nodeId);
    },
  }
}
