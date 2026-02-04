import { useCallback } from 'react';
import { useStore } from '@/store';
import type { NodeData } from '@/model/workflow/types';

export function useSubgraphController(id: string, data: NodeData) {
  const toggleSubgraph = useStore((state) => state.toggleSubgraph);

  const onToggle = useCallback(() => {
    toggleSubgraph(id);
  }, [id, toggleSubgraph]);

  return {
    expanded: data.expanded !== false, // Default to true
    onToggle,
  };
}
