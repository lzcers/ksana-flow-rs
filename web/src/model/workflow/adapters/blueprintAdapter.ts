import type { Node, Edge } from '../types';

export interface BackendNode {
    id: string;
    type: string;
    position?: { x: number; y: number };
    width?: number;
    height?: number;
    parentId?: string;
    extent?: string;
    hidden?: boolean;
    data?: Record<string, any>;
}

export interface BackendEdge {
    id: string;
    source: string;
    target: string;
    sourceHandle?: string | null;
    targetHandle?: string | null;
    type?: string;
    data?: Record<string, any>;
}

export interface WorkflowBlueprint {
    nodes: BackendNode[];
    edges: BackendEdge[];
}

export const toBlueprint = (nodes: Node[], edges: Edge[]): WorkflowBlueprint => {
    return {
        nodes: nodes.map((n) => {
            const { type: _, ...cleanData } = (n.data as any) || {};
            return {
                id: n.id,
                type: n.type || 'default', // Ensure type is a string
                data: cleanData,
                position: n.position,
                width: typeof n.style?.width === 'number' ? n.style.width : (typeof n.style?.width === 'string' ? parseFloat(n.style.width) : (n.width ?? n.measured?.width)),
                height: typeof n.style?.height === 'number' ? n.style.height : (typeof n.style?.height === 'string' ? parseFloat(n.style.height) : (n.height ?? n.measured?.height)),
                parentId: n.parentId,
                extent: n.extent ? (typeof n.extent === 'string' ? n.extent : undefined) : undefined, // Backend expects string or undefined
                hidden: n.hidden,
            };
        }),
        edges: edges.filter((e: any) => !e?.data?.__uiSubgraphEdge).map((e) => ({
            id: e.id,
            source: e.source,
            target: e.target,
            sourceHandle: e.sourceHandle,
            targetHandle: e.targetHandle,
            type: e.type,
        })),
    };
};

export const fromBlueprint = (blueprint: WorkflowBlueprint): { nodes: Node[]; edges: Edge[] } => {
    const nodes: Node[] = (blueprint.nodes || []).map((n: any) => {
        const { type: _, ...cleanData } = n.data || {};
        const expanded = cleanData.expanded !== false;
        const preferredSize = expanded ? cleanData.expandedSize : cleanData.collapsedSize;
        let width = n.width;
        let height = n.height;
        if (n.type === 'SubgraphNode' || n.type === 'MapNode') {
            if (preferredSize && typeof preferredSize.width === 'number' && typeof preferredSize.height === 'number') {
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
            extent: n.extent,
            hidden: n.hidden,
            data: cleanData,
        };
    });

    const edges: Edge[] = (blueprint.edges || []).map((e: any) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle,
        type: e.type || 'default',
    }));

    return { nodes, edges };
};
