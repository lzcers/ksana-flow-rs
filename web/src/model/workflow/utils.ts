import type { Immutable } from 'immer';
import type { WorkflowState, Node, Edge, Connection } from './types';
export { applyNodeChanges as applyNodeChangesXyflow, applyEdgeChanges as applyEdgeChangesXyflow, addEdge as addEdgeXyflow } from '@xyflow/react';

export const getNextNodeId = (nodes: Immutable<Node[]>, type: string): string => {
  const sameTypeNodes = nodes.filter((n) => n.id.startsWith(`${type}-`));
  let nextNum = 1;
  if (sameTypeNodes.length > 0) {
    const nums = sameTypeNodes.map((n) => {
      const parts = n.id.split('-');
      const lastPart = parts[parts.length - 1];
      const num = parseInt(lastPart, 10);
      return isNaN(num) ? 0 : num;
    });
    nextNum = Math.max(...nums) + 1;
  }
  return `${type}-${nextNum}`;
};

export const applyCollapsedSubgraphUi = (nodes: Immutable<Node[]>, edges: Immutable<Edge[]>) => {
  const nodeById = new Map(nodes.map((n) => [n.id, n] as const));
  const collapsedGroupIds = new Set(
    nodes
      .filter((n) => (n.type === 'SubgraphNode' || n.type === 'MapNode') && (n.data as any)?.expanded === false)
      .map((n) => n.id)
  );

  const normalizeTargetHandle = (handle: any) => {
    const h = typeof handle === 'string' ? handle : '';
    if (/^t-(left|right|top|bottom)$/.test(h)) return h;
    return 't-left';
  };
  const normalizeSourceHandle = (handle: any) => {
    const h = typeof handle === 'string' ? handle : '';
    if (/^s-(left|right|top|bottom)$/.test(h)) return h;
    return 's-right';
  };

  const collapsedAncestor = (nodeId: string): string | null => {
    let cur = nodeById.get(nodeId);
    while (cur?.parentId) {
      if (collapsedGroupIds.has(cur.parentId)) return cur.parentId;
      cur = nodeById.get(cur.parentId);
    }
    return null;
  };

  const nextNodes = nodes.map((n) => {
    if (!n.parentId) return n;
    const hiddenBy = collapsedAncestor(n.id);
    if (!hiddenBy) return n;
    return {
      ...n,
      hidden: true,
      extent: undefined,
    } as any;
  });

  const proxyEdges: Edge[] = [];
  const seenProxy = new Set<string>();

  const nextEdges = edges
    .filter((e: any) => !e?.data?.__uiSubgraphEdge)
    .map((e: any) => {
      const srcCollapse = collapsedAncestor(e.source);
      const tgtCollapse = collapsedAncestor(e.target);

      if (!srcCollapse && !tgtCollapse) {
        if (e?.data?.__uiSubgraphHidden) {
          const { __uiSubgraphHidden, ...restData } = e.data || {};
          return { ...e, hidden: false, data: restData };
        }
        return e;
      }

      const src = srcCollapse ?? e.source;
      const tgt = tgtCollapse ?? e.target;
      if (src === tgt) {
        return { ...e, hidden: true, data: { ...(e.data || {}), __uiSubgraphHidden: true } };
      }

      const proxyId = `ui:subgraph:load:${e.id}:${src}:${tgt}`;
      if (!seenProxy.has(proxyId)) {
        seenProxy.add(proxyId);

        const proxy: any = {
          id: proxyId,
          type: e.type || 'default',
          source: src,
          target: tgt,
          sourceHandle: normalizeSourceHandle(e.sourceHandle),
          targetHandle: normalizeTargetHandle(e.targetHandle),
          data: { __uiSubgraphEdge: { originalEdgeId: e.id } },
        };
        proxyEdges.push(proxy);
      }

      return { ...e, hidden: true, data: { ...(e.data || {}), __uiSubgraphHidden: true } };
    });

  return { nodes: nextNodes, edges: [...nextEdges, ...proxyEdges] };
};

export const getNode = (state: WorkflowState, nodeId: string): Node | undefined => {
  return state.nodes.find((n) => n.id === nodeId);
};


export const getConnectedEdges = (state: WorkflowState, nodeId: string): Edge[] => {
  return state.edges.filter((e) => e.source === nodeId || e.target === nodeId);
};

export const isValidConnection = (_connection: Connection, _state: WorkflowState): boolean => {
  return true;
};

export const sortNodesByParent = (nodes: Immutable<Node[]>): Immutable<Node[]> => {
  const idSet = new Set(nodes.map((n) => n.id));
  const childrenByParent = new Map<string, Immutable<Node>[]>();

  nodes.forEach((n) => {
    if (!n.parentId || !idSet.has(n.parentId)) return;
    const children = childrenByParent.get(n.parentId);
    if (children) {
      children.push(n);
    } else {
      childrenByParent.set(n.parentId, [n]);
    }
  });

  const result: Immutable<Node>[] = [];
  const visited = new Set<string>();

  const visit = (node: Immutable<Node>) => {
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
