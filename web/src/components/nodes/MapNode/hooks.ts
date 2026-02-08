import { useMemo } from 'react';
import type { NodeData } from '@/model/workflow/types';

export type MapNodeStreamState = {
  receivedItems: number;
  lastItemIndex: number | null;
  lastItemOutput: any;
  doneCount: number | null;
  finalCount: number | null;
};

const INITIAL: MapNodeStreamState = {
  receivedItems: 0,
  lastItemIndex: null,
  lastItemOutput: null,
  doneCount: null,
  finalCount: null,
};

export function useMapNodeStream(_nodeId: string, data?: NodeData): MapNodeStreamState {
  return useMemo(() => {
    const value = (data as any)?.mapStream;
    if (!value || typeof value !== 'object') return INITIAL;
    return {
      receivedItems: typeof value.receivedItems === 'number' ? value.receivedItems : 0,
      lastItemIndex: typeof value.lastItemIndex === 'number' ? value.lastItemIndex : null,
      lastItemOutput: value.lastItemOutput ?? null,
      doneCount: typeof value.doneCount === 'number' ? value.doneCount : null,
      finalCount: typeof value.finalCount === 'number' ? value.finalCount : null,
    };
  }, [data]);
}
