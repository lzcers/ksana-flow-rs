import { useEffect, useState } from 'react';
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
  const flowEventForNodeId$ = useStore((s) => s.flowEventForNodeId$);
  const currentRunId = useStore((s) => s.currentRunId);
  const [state, setState] = useState<MapNodeStreamState>(INITIAL);

  useEffect(() => {
    const subscription = flowEventForNodeId$(nodeId).subscribe((event: FlowEvent) => {
      switch (event.type) {
        case 'NodeStarted':
          setState(INITIAL);
          return;

        case 'NodeError':
          setState(INITIAL);
          return;

        case 'NodeStreamNextMessage': {
          const value = event.msg;
          if (value == null || typeof value !== 'object') return;

          const kind = (value).kind;
          if (kind === 'item') {
            const index = typeof (value).index === 'number' ? (value).index : null;
            const output = (value).output;
            setState((prev) => ({
              ...prev,
              receivedItems: prev.receivedItems + 1,
              lastItemIndex: index,
              lastItemOutput: output,
            }));
          } else if (kind === 'done') {
            const count = typeof (value).count === 'number' ? (value).count : null;
            setState((prev) => ({
              ...prev,
              doneCount: count,
            }));
          }
          return;
        }

        case 'NodeOutMessage': {
          const outValue = event.msg;
          if (Array.isArray(outValue)) {
            setState((prev) => ({
              ...prev,
              finalCount: outValue.length,
            }));
          }
          return;
        }
      }
    });

    return () => subscription.unsubscribe();
  }, [flowEventForNodeId$, currentRunId, nodeId]);

  return state;
}
