import { produce } from 'immer';
import type { WorkflowState, Node } from '../types';
import type { HandleNodeDragStopCommand } from '../commands';
import { sortNodesByParent } from '../utils';

// Helper functions
export const processHandleNodeDragStop = (
    state: WorkflowState,
    command: HandleNodeDragStopCommand
): WorkflowState => {
    const { nodeId } = command.payload;
    if (!nodeId) return state;

    const nodeById = new Map(state.nodes.map((n) => [n.id, n] as const));
    const node = nodeById.get(nodeId);
    if (!node) return state;

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
    for (const g of state.nodes) {
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

    if (nextParentId === (node.parentId ?? undefined)) return state;

    const nextParent = nextParentId ? nodeById.get(nextParentId) : undefined;
    const nextParentAbs = nextParent ? getAbsPos(nextParent) : { x: 0, y: 0 };
    const nextPos = nextParentId
        ? { x: nodeAbs.x - nextParentAbs.x, y: nodeAbs.y - nextParentAbs.y }
        : { x: nodeAbs.x, y: nodeAbs.y };

    const nextNodes = state.nodes.map((n) => {
        if (n.id !== nodeId) return n;
        return {
            ...n,
            parentId: nextParentId || undefined,
            extent: nextParentId ? ('parent' as const) : undefined,
            position: nextPos,
            expandParent: undefined,
        };
    });

    return produce(state, (draft) => {
        draft.nodes = sortNodesByParent(nextNodes) as any[];
    });
};
