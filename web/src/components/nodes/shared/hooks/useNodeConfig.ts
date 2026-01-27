import { useCallback, useEffect, useRef } from 'react';
import { useStore } from '../../../../store';

export function useNodeConfig<T extends Record<string, unknown>>(id: string, config: T | undefined) {
  const { updateNodeData } = useStore();
  const configRef = useRef<T | undefined>(config);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  const updateConfig = useCallback(
    (patch: Partial<T>) => {
      updateNodeData(id, { config: { ...(configRef.current ?? ({} as T)), ...patch } });
    },
    [id, updateNodeData],
  );

  return { configRef, updateConfig };
}
