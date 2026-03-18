/**
 * Node 处理器函数 - 简化版
 * 纯函数，接收 state 和 command，返回新的 state
 */

import { produce, type Draft, type Immutable } from "immer";
import type { WorkflowState, Node, NodeData, NodeChange } from "../types";
import type { AddNodeCommand, RemoveNodeCommand, UpdateNodeCommand, ApplyNodeChangesCommand, ResetAllNodeStatusCommand } from "../commands";
import { applyNodeChangesXyflow, getNextNodeId } from "../utils";

export { getNextNodeId };

// 默认节点数据
const getDefaultNodeData = (type: string): Partial<NodeData> => {
    const defaults: Record<string, Partial<NodeData>> = {
        LLMNode: {
            label: "LLM",
            config: { model: "gpt-4", temperature: 0.7 },
        },
        TextNode: {
            label: "Text",
            config: { content: "" },
        },
        SubgraphNode: {
            label: "Subgraph",
            expanded: true,
            expandedSize: { width: 400, height: 300 },
            collapsedSize: { width: 200, height: 50 },
        },
        MapNode: {
            label: "Map",
            expanded: true,
            expandedSize: { width: 400, height: 300 },
            collapsedSize: { width: 200, height: 50 },
        },
    };
    return defaults[type] || { label: type };
};

// ===== 处理器函数 =====

export const processAddNode = (state: Immutable<WorkflowState>, command: AddNodeCommand): Immutable<WorkflowState> => {
    const { id: requestedId, nodeType, position, data } = command.payload;

    return produce(state, draft => {
        const id = requestedId ?? getNextNodeId(draft.nodes, nodeType);
        const newNode: Node = {
            id,
            type: nodeType,
            position,
            data: {
                ...getDefaultNodeData(nodeType),
                ...data,
                label: data?.label ?? id,
                status: "idle",
            },
        };
        draft.nodes.push(newNode);
    });
};

export const processRemoveNode = (state: Immutable<WorkflowState>, command: RemoveNodeCommand): Immutable<WorkflowState> => {
    const { id } = command.payload;

    return produce(state, draft => {
        draft.nodes = draft.nodes.filter(n => n.id !== id);
        draft.edges = draft.edges.filter(e => e.source !== id && e.target !== id);
    });
};

/**
 * 统一的节点更新处理器 - 合并所有 update 操作
 */
export const processUpdateNode = (state: Immutable<WorkflowState>, command: UpdateNodeCommand): Immutable<WorkflowState> => {
    const { id, updates } = command.payload;

    return produce(state, draft => {
        const node = draft.nodes.find(n => n.id === id);
        if (!node) return;

        // 更新 data
        if (updates.data) {
            node.data = { ...node.data, ...updates.data };
        }

        // 更新 position
        if (updates.position) {
            node.position = updates.position;
        }

        // 更新 dimensions (同时更新 style 和 width/height)
        if (updates.dimensions) {
            const { width, height } = updates.dimensions;
            node.style = { ...node.style, width, height };
            node.width = width;
            node.height = height;

            // 处理子图/Map 节点的尺寸
            if (node.type === "SubgraphNode" || node.type === "MapNode") {
                const expanded = node.data?.expanded !== false;
                const size = { width, height };
                node.data = {
                    ...node.data,
                    expandedSize: expanded ? size : node.data?.expandedSize,
                    collapsedSize: expanded ? node.data?.collapsedSize : size,
                };
            }
        }

        // 更新 status
        if (updates.status !== undefined) {
            if (!node.data) node.data = {};
            node.data.status = updates.status;
            if (updates.errorMessage !== undefined) {
                node.data.errorMessage = updates.errorMessage;
            }
        }

        // 更新 inputs
        if (updates.inputs) {
            if (!node.data) node.data = {};
            node.data.inputs = { ...node.data.inputs, ...updates.inputs };
        }

        // 更新 outputs
        if (updates.outputs) {
            if (!node.data) node.data = {};
            node.data.outputs = { ...node.data.outputs, ...updates.outputs };
        }

        // 更新其他字段 (如 isOutputStream, lastMessage 等)
        if (updates.isOutputStream !== undefined) {
            if (!node.data) node.data = {};
            node.data.isOutputStream = updates.isOutputStream;
        }
        if (updates.lastMessage !== undefined) {
            if (!node.data) node.data = {};
            node.data.lastMessage = updates.lastMessage;
        }

        syncEdgeHighlighting(draft);
    });
};

export const processApplyNodeChanges = (state: Immutable<WorkflowState>, command: ApplyNodeChangesCommand): Immutable<WorkflowState> => {
    const { changes } = command.payload;

    return produce(state, draft => {
        const updatedNodes = applyNodeChangesXyflow(changes as NodeChange[], draft.nodes);
        draft.nodes = updatedNodes;
    });
};

export const processResetAllNodeStatus = (state: Immutable<WorkflowState>, _command: ResetAllNodeStatusCommand): Immutable<WorkflowState> => {
    return produce(state, draft => {
        draft.nodes.forEach(node => {
            if (node.data.status === "running") {
                node.data.status = "idle";
                node.data.errorMessage = undefined;
            }
        });
        syncEdgeHighlighting(draft);
    });
};

// ===== 内部工具函数 =====

function syncEdgeHighlighting(draft: Draft<WorkflowState>): void {
    // 根据节点状态同步边的样式
    const selectedNodeIds = new Set(draft.nodes.filter(n => n.selected || n.data?.status === "running").map(n => n.id));

    draft.edges.forEach(edge => {
        if (selectedNodeIds.has(edge.source)) {
            edge.animated = true;
            edge.style = { ...edge.style, stroke: "#3b82f6", strokeWidth: 3 };
        } else {
            edge.animated = false;
            if (edge.style) {
                delete edge.style.stroke;
                delete edge.style.strokeWidth;
                if (Object.keys(edge.style).length === 0) {
                    edge.style = undefined;
                }
            }
        }
    });
}
