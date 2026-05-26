import { useCallback, useMemo, useState } from 'react';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '@/model/workflow/types';

export type ReduceReducer =
  | 'sum'
  | 'count'
  | 'max'
  | 'min'
  | 'concat'
  | 'merge_array'
  | 'merge_object_deep';

export function useReduceNodeController(id: string, data: NodeData) {
  const { updateConfig } = useNodeConfig(id, data.config);

  const reducer = useMemo<ReduceReducer>(() => {
    const r = data.config?.reducer;
    if (typeof r === 'string') return r as ReduceReducer;
    return 'sum';
  }, [data.config?.reducer]);

  const [separator, setSeparator] = useState<string>(() => {
    const s = data.config?.separator;
    return typeof s === 'string' ? s : '\n';
  });

  const onReducerChange = useCallback(
    (next: ReduceReducer) => {
      updateConfig({ reducer: next });
    },
    [updateConfig],
  );

  const onSeparatorChange = useCallback((next: string) => {
    setSeparator(next);
  }, []);

  const onSeparatorBlur = useCallback(() => {
    updateConfig({ separator });
  }, [separator, updateConfig]);

  const outputPreview = useMemo(() => {
    const v = data.lastMessage ?? data.outputs?.output ?? data.outputs;
    if (v == null) return '';
    if (typeof v === 'string') return v;
    try {
      return JSON.stringify(v);
    } catch {
      return String(v);
    }
  }, [data.lastMessage, data.outputs]);

  return {
    reducer,
    separator,
    onReducerChange,
    onSeparatorChange,
    onSeparatorBlur,
    outputPreview,
  };
}

