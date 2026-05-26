import type { Edge, EdgeData, Node, NodeData } from '../types';
import { inferEdgeDataFromHandles } from '../utils/connection';

export interface BackendNode {
    id: string;
    type: string;
    position?: { x: number; y: number };
    width?: number;
    height?: number;
    parentId?: string;
    extent?: string;
    hidden?: boolean;
    data?: Record<string, unknown>;
}

export interface BackendEdge {
    id: string;
    source: string;
    target: string;
    sourceHandle?: string | null;
    targetHandle?: string | null;
    type?: string;
    data?: Record<string, unknown>;
}

export interface WorkflowBlueprint {
    nodes: BackendNode[];
    edges: BackendEdge[];
}

function isNodeSize(value: unknown): value is { width?: number; height?: number } {
    return typeof value === 'object' && value !== null;
}

function toNodeExtent(extent?: string): 'parent' | undefined {
    return extent === 'parent' ? 'parent' : undefined;
}

function sanitizeNodeData(data: NodeData | undefined): Record<string, unknown> {
    const { type: _type, ...cleanData } = data ?? {};
    return cleanData;
}

function sanitizeEdgeData(data: EdgeData | undefined): Record<string, unknown> | undefined {
    if (!data) {
        return undefined;
    }

    const { __uiSubgraphEdge, ...cleanData } = data;
    return Object.keys(cleanData).length > 0 ? cleanData : undefined;
}

export const toBlueprint = (nodes: Node[], edges: Edge[]): WorkflowBlueprint => {
    return {
        nodes: nodes.map((n) => {
            return {
                id: n.id,
                type: n.type || 'default', // Ensure type is a string
                data: sanitizeNodeData(n.data),
                position: n.position,
                width: typeof n.style?.width === 'number' ? n.style.width : (typeof n.style?.width === 'string' ? parseFloat(n.style.width) : (n.width ?? n.measured?.width)),
                height: typeof n.style?.height === 'number' ? n.style.height : (typeof n.style?.height === 'string' ? parseFloat(n.style.height) : (n.height ?? n.measured?.height)),
                parentId: n.parentId,
                extent: n.extent ? (typeof n.extent === 'string' ? n.extent : undefined) : undefined, // Backend expects string or undefined
                hidden: n.hidden,
            };
        }),
        edges: edges.filter(e => !e.data?.__uiSubgraphEdge).map((e) => ({
            id: e.id,
            source: e.source,
            target: e.target,
            sourceHandle: e.sourceHandle,
            targetHandle: e.targetHandle,
            type: e.type,
            data: sanitizeEdgeData(e.data),
        })),
    };
};

export const fromBlueprint = (blueprint: WorkflowBlueprint): { nodes: Node[]; edges: Edge[] } => {
    const nodes: Node[] = (blueprint.nodes || []).map((n) => {
        const { type: _type, ...cleanData } = n.data ?? {};
        const expanded = cleanData.expanded !== false;
        const preferredSize = expanded ? cleanData.expandedSize : cleanData.collapsedSize;
        let width = n.width;
        let height = n.height;
        if (n.type === 'SubgraphNode' || n.type === 'MapNode') {
            if (
                isNodeSize(preferredSize) &&
                typeof preferredSize.width === 'number' &&
                typeof preferredSize.height === 'number'
            ) {
                width = preferredSize.width;
                height = preferredSize.height;
            } else if (!expanded) {
                width = width ?? 180;
                height = height ?? 80;
            }
        }
        return {
            id: n.id,
            type: n.type,
            position: n.position || { x: 0, y: 0 },
            width,
            height,
            style: width && height ? { width, height } : undefined,
            parentId: n.parentId,
            extent: toNodeExtent(n.extent),
            hidden: n.hidden,
            data: cleanData as NodeData,
        };
    });

    const edges: Edge[] = (blueprint.edges || []).map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle,
        type: e.type || 'default',
        data: {
            ...(e.data ?? {}),
            ...inferEdgeDataFromHandles(e.sourceHandle, e.targetHandle),
        },
    }));

    return { nodes, edges };
};
