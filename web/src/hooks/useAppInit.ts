import { useEffect } from 'react';
import { useStore } from '../store';

export function useAppInit(spaceId: string) {
  const loadMetadata = useStore((state) => state.loadMetadata);
  const initializeWebSocket = useStore((state) => state.initializeWebSocket);
  const setSpaceId = useStore((state) => state.setSpaceId);

  useEffect(() => {
    setSpaceId(spaceId);
  }, [spaceId, setSpaceId]);

  useEffect(() => {
    loadMetadata();
  }, [loadMetadata, spaceId]);

  useEffect(() => {
    const cleanup = initializeWebSocket();
    return cleanup;
  }, [initializeWebSocket, spaceId]);
}
