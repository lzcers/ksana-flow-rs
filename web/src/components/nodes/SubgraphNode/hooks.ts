import { useCallback } from 'react';
import { useStore } from '@/store';
import type { NodeData } from '@/model/types';

export function useSubgraphController(id: string, data: NodeData) {
  // @ts-ignore - toggleSubgraph will be added to store later
  const toggleSubgraph = useStore((state) => state.toggleSubgraph);

  const onToggle = useCallback(() => {
    if (toggleSubgraph) {
      toggleSubgraph(id);
    }
  }, [id, toggleSubgraph]);

  return {
    expanded: data.expanded !== false, // Default to true
    onToggle,
  };
}
