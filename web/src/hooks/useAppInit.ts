import { useEffect, useRef } from "react";
import { useStore } from "../store";

export function useAppInit(spaceId: string, workflowId?: number) {
    const loadMetadata = useStore(state => state.loadMetadata);
    const initializeWebSocket = useStore(state => state.initializeWebSocket);
    const setSpaceId = useStore(state => state.setSpaceId);
    const startAutoSave = useStore(state => state.startAutoSave);
    const stopAutoSave = useStore(state => state.stopAutoSave);
    const loadWorkflow = useStore(state => state.loadWorkflow);
    const workflowLoadedRef = useRef(false);

    useEffect(() => {
        setSpaceId(spaceId);
        workflowLoadedRef.current = false;
    }, [spaceId, setSpaceId]);

    useEffect(() => {
        loadMetadata();
    }, [loadMetadata, spaceId]);

    useEffect(() => {
        startAutoSave();
        return () => stopAutoSave();
    }, [startAutoSave, stopAutoSave]);

    useEffect(() => {
        const cleanup = initializeWebSocket();
        return cleanup;
    }, [initializeWebSocket, spaceId]);

    useEffect(() => {
        if (workflowId !== undefined && !workflowLoadedRef.current) {
            workflowLoadedRef.current = true;
            loadWorkflow(workflowId);
        }
    }, [workflowId, loadWorkflow]);
}
