import { useEffect } from 'react';
import { useStore } from '../store';

export function useAppInit() {
  const loadMetadata = useStore((state) => state.loadMetadata);
  const initializeWebSocket = useStore((state) => state.initializeWebSocket);

  useEffect(() => {
    loadMetadata();
  }, [loadMetadata]);

  useEffect(() => {
    const cleanup = initializeWebSocket();
    return cleanup;
  }, [initializeWebSocket]);
}
