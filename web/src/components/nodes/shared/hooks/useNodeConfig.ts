import { useCallback, useEffect, useRef } from 'react';
import { workflowModel } from '../../../../store/workflowModel';

export function useNodeConfig<T extends Record<string, unknown>>(id: string, config: T | undefined) {
  const configRef = useRef<T | undefined>(config);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  const updateConfig = useCallback(
    (patch: Partial<T>) => {
      workflowModel.dispatchers.updateNodeData(id, { config: { ...(configRef.current ?? ({} as T)), ...patch } });
    },
    [id],
  );

  return { configRef, updateConfig };
}
