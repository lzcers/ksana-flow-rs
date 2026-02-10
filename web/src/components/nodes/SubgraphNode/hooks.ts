import { useCallback, useEffect } from 'react';
import { useStore } from '@/store';
import type { NodeData } from '@/model/workflow/types';
import { workflowManager } from '@/model/workflowManager';

export function useSubgraphController(id: string, data: NodeData) {
  const toggleSubgraph = useStore((state) => state.toggleSubgraph);
  const activeGraphKey = useStore((state) => state.activeGraphKey);

  const onToggle = useCallback(() => {
    toggleSubgraph(id);
  }, [id, toggleSubgraph]);

  const expanded = data.expanded !== false; // Default to true

  // 当 SubgraphNode 展开时，激活对应的子图实例
  useEffect(() => {
    if (expanded && activeGraphKey) {
      const instance = workflowManager.getModelInstance(activeGraphKey);
      if (instance) {
        // 激活该 SubgraphNode 对应的子图
        instance.activateSubgraphNode(id);
      }
    }
  }, [expanded, id, activeGraphKey]);

  return {
    expanded,
    onToggle,
  };
}
