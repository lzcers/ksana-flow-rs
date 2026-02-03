import { useCallback, useEffect, useRef } from 'react';
import { rxWorkflowModel } from '@/store';

export function useNodeConfig<T extends Record<string, unknown>>(id: string, config?: T) {
  const configRef = useRef<T | undefined>(config);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  const updateConfig = useCallback(
    (patch: Partial<T>) => {
      rxWorkflowModel.dispatchers.updateNodeData(id, { config: { ...(configRef.current ?? ({} as T)), ...patch } });
    },
    [id],
  );

  return { configRef, updateConfig };
}
