import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useStore } from '@/store';
import type { FlowEvent } from '@/model/flowEvent/types';

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

export function useMapNodeStream(nodeId: string): MapNodeStreamState {
  const eventsForNode$ = useStore((s) => s.eventsForNode$);
  const [state, setState] = useState<MapNodeStreamState>(INITIAL);

  useEffect(() => {
    const stream$ = eventsForNode$?.(nodeId);
    if (!stream$) return;

    const subscription = stream$.subscribe((event: FlowEvent) => {
      if (!('nodeId' in event)) return;
      if (event.nodeId !== nodeId) return;

      switch (event.type) {
        case 'NodeStarted':
          setState(INITIAL);
          return;

        case 'NodeError':
          setState(INITIAL);
          return;

        case 'NodeStreamNextMessage':
          const value = event.msg;
          if (value == null || typeof value !== 'object') return;

          const kind = (value as any).kind;
          if (kind === 'item') {
            const index = typeof (value as any).index === 'number' ? (value as any).index : null;
            const output = (value as any).output;
            setState((prev) => ({
              ...prev,
              receivedItems: prev.receivedItems + 1,
              lastItemIndex: index,
              lastItemOutput: output,
            }));
          } else if (kind === 'done') {
            const count = typeof (value as any).count === 'number' ? (value as any).count : null;
            setState((prev) => ({
              ...prev,
              doneCount: count,
            }));
          }
          return;

        case 'NodeOutMessage':
          const outValue = event.msg;
          if (Array.isArray(outValue)) {
            setState((prev) => ({
              ...prev,
              finalCount: outValue.length,
            }));
          }
          return;
      }
    });

    return () => subscription.unsubscribe();
  }, [eventsForNode$, nodeId]);

  return state;
}
