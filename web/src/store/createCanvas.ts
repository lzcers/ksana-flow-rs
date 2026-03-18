import { castDraft, type Immutable } from "immer";
import type { StateCreator } from "zustand";
import type { StoreState, Canvas } from "./types";
import type { Node, Edge, NodeChange, EdgeChange, Connection, NodeData } from "../model/workflow/types";
import { sortNodesByParent } from "../model/workflow/utils";
import { workflowManager, type GraphKey } from "../model/workflowManager";
import type { Subscription } from "rxjs";
import type { OnNodeDrag } from "@xyflow/react";
import { getNodeMetadata } from "../model/nodeRegistry";

export const createCanvas: StateCreator<StoreState, [], [], Canvas> = (set, get) => {
    let viewStateSubscription: Subscription | null = null;

    const getActiveModel = () => {
        const graphKey = get().activeGraphKey;
        if (!graphKey) throw new Error("No active graphKey");
        const instance = workflowManager.getModelInstance(graphKey);
        if (!instance) throw new Error(`No Model found for graphKey: ${graphKey}`);
        return instance.model;
    };

    // RAF batching for viewState updates
    let pendingViewState: Immutable<{ nodes: Node[]; edges: Edge[] }> | null = null;
    let rafId: number | null = null;

    const flushViewState = () => {
        if (pendingViewState) {
            set({
                nodes: castDraft(pendingViewState.nodes),
                edges: castDraft(pendingViewState.edges),
            });
            pendingViewState = null;
        }
        rafId = null;
    };

    const switchCanvas = (graphKey: GraphKey) => {
        // Clean up previous RAF batching state
        if (rafId !== null) {
            cancelAnimationFrame(rafId);
            rafId = null;
        }
        pendingViewState = null;

        const rxWorkflowInstance = workflowManager.getModelInstance(graphKey);
        if (!rxWorkflowInstance) {
            throw new Error(`No Model found for graphKey: ${graphKey}`);
        }
        viewStateSubscription?.unsubscribe();
        viewStateSubscription = rxWorkflowInstance.model.viewState$.subscribe(viewState => {
            pendingViewState = viewState;
            if (rafId === null) {
                rafId = requestAnimationFrame(flushViewState);
            }
        });
    };

    const findDropTargetGroup = (nodeId: string, nodes: Immutable<Node[]>): string | null => {
        const nodeById = new Map(nodes.map(n => [n.id, n] as const));
        const node = nodeById.get(nodeId);
        if (!node) return null;

        const isGroup = (n: Immutable<Node>) => n.type === "SubgraphNode" || n.type === "MapNode";
        const isDropTargetGroup = (n: Immutable<Node>) => isGroup(n) && n.id !== nodeId && n.hidden !== true && n.data?.expanded !== false;

        const toNumber = (v: unknown): number | undefined => {
            if (typeof v === "number" && Number.isFinite(v)) return v;
            if (typeof v === "string") {
                const n = parseFloat(v);
                if (Number.isFinite(n)) return n;
            }
            return undefined;
        };

        const getSize = (n: Immutable<Node>): { width: number; height: number } => {
            const styleW = toNumber(n.style?.width);
            const styleH = toNumber(n.style?.height);
            const width = (n.measured?.width ?? styleW ?? n.width ?? (isGroup(n) ? 300 : 150)) as number;
            const height = (n.measured?.height ?? styleH ?? n.height ?? (isGroup(n) ? 200 : 50)) as number;
            return { width, height };
        };

        const getAbsPos = (n: Immutable<Node>): { x: number; y: number } => {
            let x = n.position.x;
            let y = n.position.y;
            let cur: Immutable<Node> | undefined = n;
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

        const depthOf = (n: Immutable<Node>): number => {
            let depth = 0;
            let cur: Immutable<Node> | undefined = n;
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

        let targetGroup: Immutable<Node> | null = null;
        let bestDepth = -1;
        for (const g of nodes) {
            if (!isDropTargetGroup(g)) continue;
            if (isGroup(node) && isAncestor(node.id, g.id)) continue;

            const gAbs = getAbsPos(g);
            const gSize = getSize(g);
            const inside = center.x >= gAbs.x && center.x <= gAbs.x + gSize.width && center.y >= gAbs.y && center.y <= gAbs.y + gSize.height;
            if (!inside) continue;
            const d = depthOf(g);
            if (d > bestDepth) {
                bestDepth = d;
                targetGroup = g;
            }
        }

        return targetGroup?.id ?? null;
    };

    return {
        nodes: [],
        edges: [],
        selectedNodeId: [],
        isConnecting: false,
        connectionSourceId: null,
        dragOverNodeId: null,

        undo: () => {
            getActiveModel().undo();
        },

        redo: () => {
            getActiveModel().redo();
        },

        switchCanvas: (graphKey: GraphKey) => switchCanvas(graphKey),

        setNodes: nodes => getActiveModel().action.setNodes(nodes),
        setEdges: edges => getActiveModel().action.setEdges(edges),

        pasteNodes: (nodes, edges) => {
            getActiveModel().action.pasteNodes(nodes, edges);
        },

        onNodesChange: (changes: NodeChange[]) => {
            const skipHistoryChanges: NodeChange[] = [];
            const otherChanges: NodeChange[] = [];
            changes.forEach(c => {
                // 拖拽不触发
                if (c.type === "position" && c.dragging === true) {
                    skipHistoryChanges.push(c);
                } else if (c.type === "select") {
                    skipHistoryChanges.push(c);
                } else {
                    otherChanges.push(c);
                }
            });
            if (skipHistoryChanges.length > 0) {
                getActiveModel().action.applyNodeChanges(skipHistoryChanges, { skipHistory: true });
            }
            if (otherChanges.length > 0) {
                getActiveModel().action.applyNodeChanges(otherChanges);
            }
        },

        onEdgesChange: (changes: EdgeChange[]) => {
            // Snapshot on remove (e.g. Backspace key or removing edge)
            const { edges } = get();
            const edgeById = new Map(edges.map(e => [e.id, e] as const));

            const removeOriginalEdgeIds: string[] = [];
            const coreChanges: EdgeChange[] = [];

            for (const change of changes) {
                if (!("id" in change)) {
                    coreChanges.push(change);
                    continue;
                }

                const edge = edgeById.get(change.id);
                const proxy = edge?.data?.__uiSubgraphEdge;
                if (proxy) {
                    if (change.type === "remove" && proxy.originalEdgeId) {
                        removeOriginalEdgeIds.push(proxy.originalEdgeId);
                    }
                    continue;
                }
                coreChanges.push(change);
            }

            removeOriginalEdgeIds.forEach(id => getActiveModel().action.removeEdge(id));

            if (coreChanges.length > 0) {
                getActiveModel().action.updateEdges(coreChanges);
            }
        },

        onNodeDrag: ((_event, draggedNode) => {
            const nodeId = draggedNode.id;
            const { nodes } = getActiveModel().getSnapshot();
            const targetId = findDropTargetGroup(nodeId, nodes as Immutable<Node[]>);
            set({ dragOverNodeId: targetId });
        }) satisfies OnNodeDrag<Node>,

        onNodeDragStop: ((_event, draggedNode) => {
            const nodeId = draggedNode.id;
            set({ dragOverNodeId: null });
            getActiveModel().action.handleNodeDragStop(nodeId);
        }) satisfies OnNodeDrag<Node>,

        onConnect: (connection: Connection) => {
            getActiveModel().action.onConnect(connection);
        },

        addNode: (type: string, position: { x: number; y: number } = { x: 300, y: 200 }) => {
            const { nodeTypes } = get();
            const { nodes } = getActiveModel().getSnapshot();
            // Find all nodes of the same type and extract their numbers
            const sameTypeNodes = nodes.filter(n => n.id.startsWith(`${type}-`));
            let nextNum = 1;
            if (sameTypeNodes.length > 0) {
                const nums = sameTypeNodes.map(n => {
                    const parts = n.id.split("-");
                    const lastPart = parts[parts.length - 1];
                    const num = parseInt(lastPart, 10);
                    return isNaN(num) ? 0 : num;
                });
                nextNum = Math.max(...nums) + 1;
            }
            const id = `${type}-${nextNum}`;
            const meta = nodeTypes.find(t => t.name === type);
            const nodeMetadata = getNodeMetadata(type);
            const initialData: Partial<NodeData> = {
                label: nodeMetadata?.displayName ?? type,
                config: meta?.config ?? nodeMetadata?.defaultConfig ?? {},
                status: "idle",
            };
            getActiveModel().action.addNode(type, position, {
                id,
                data: initialData,
            });
            set({ selectedNodeId: [id] });
        },

        deleteNode: (id: string) => {
            getActiveModel().action.deleteNode(id);
        },

        updateNodeData: (id: string, data: Partial<NodeData>) => {
            getActiveModel().action.updateNodeData(id, data);
        },

        updateNodeDimensions: (id: string, width: number, height: number) => {
            getActiveModel().action.updateNodeDimensions(id, width, height);
        },

        selectNode: (id: string[]) => {
            set({ selectedNodeId: id });
        },

        setConnectionState: (connecting, sourceId = null) => set({ isConnecting: connecting, connectionSourceId: sourceId }),

        groupNodes: (nodeIds: string[]) => {
            if (nodeIds.length < 1) return;
            const { nodes } = getActiveModel().getSnapshot();
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
            let minX = Infinity,
                minY = Infinity,
                maxX = -Infinity,
                maxY = -Infinity;
            selectedNodes.forEach(n => {
                minX = Math.min(minX, n.position.x);
                minY = Math.min(minY, n.position.y);
                const w = (n.measured?.width ?? (typeof n.style?.width === "number" ? n.style.width : 150)) as number;
                const h = (n.measured?.height ?? (typeof n.style?.height === "number" ? n.style.height : 50)) as number;
                maxX = Math.max(maxX, n.position.x + w);
                maxY = Math.max(maxY, n.position.y + h);
            });

            const padding = 40;
            const groupX = minX - padding;
            const groupY = minY - padding - 40; // Extra space for header
            const groupWidth = maxX - minX + padding * 2;
            const groupHeight = maxY - minY + padding * 2 + 40;

            // Generate ID
            const type = "SubgraphNode";
            const sameTypeNodes = nodes.filter(n => n.id.startsWith(`${type}-`));
            let nextNum = 1;
            if (sameTypeNodes.length > 0) {
                const nums = sameTypeNodes.map(n => {
                    const parts = n.id.split("-");
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
                    label: "Group",
                    expanded: true,
                    expandedSize: { width: groupWidth, height: groupHeight },
                    collapsedSize: { width: 180, height: 80 },
                },
                parentId: firstParentId,
            };

            const updatedNodes = nodes.map(n => {
                if (nodeIds.includes(n.id)) {
                    const relativeX = n.position.x - groupX;
                    const relativeY = n.position.y - groupY;
                    return {
                        ...n,
                        parentId: groupId,
                        extent: "parent" as const,
                        position: {
                            x: relativeX,
                            y: relativeY,
                        },
                        expandParent: undefined,
                    };
                }
                return n;
            });

            getActiveModel().action.setNodes(sortNodesByParent([...updatedNodes, groupNode]) as Node[]);
            set({ selectedNodeId: [groupId] });
        },

        toggleSubgraph: (nodeId: string) => {
            const snapshot = getActiveModel().getSnapshot();
            const node = snapshot.nodes.find(n => n.id === nodeId);
            if (!node || (node.type !== "SubgraphNode" && node.type !== "MapNode")) return;
            getActiveModel().action.toggleSubgraph(nodeId);
        },
    };
};
