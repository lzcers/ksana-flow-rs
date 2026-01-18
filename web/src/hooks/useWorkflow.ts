import { useEffect } from 'react';
import { useStore } from '../store';
import { useToast } from '../contexts/ToastContext';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

export function useWorkflow() {
  const store = useStore();
  const toast = useToast();

  // Setup notification handler
  useEffect(() => {
    store.setNotificationHandler((type, message) => {
      if (type === 'success') toast.success(message);
      else if (type === 'error') toast.error(message);
      else toast.info(message);
    });
  }, [toast, store.setNotificationHandler]);

  // Setup WebSocket
  useEffect(() => {
    const cleanup = store.initializeWebSocket();
    return cleanup;
  }, [store.initializeWebSocket]);

  // Load metadata
  useEffect(() => {
    store.loadMetadata();
  }, [store.loadMetadata]);

  // Construct the return object to match previous API
  // We reconstruct the 'state' object to match the expected interface of components
  const state = {
    nodes: store.nodes,
    edges: store.edges,
    selectedNodeId: store.selectedNodeId
  };

  return {
    state,
    nodeTypes: store.nodeTypes,
    workflows: store.workflows,
    currentWorkflowId: store.currentWorkflowId,
    workflowStatus: store.workflowStatus,
    workflowStatuses: store.workflowStatuses,
    currentRunId: store.currentRunId,
    onNodesChange: store.onNodesChange,
    onEdgesChange: store.onEdgesChange,
    onNodeDragStop: (_event: any, _node: any) => { }, // No-op to match interface
    onConnect: store.onConnect,
    addNode: store.addNode,
    deleteNode: store.deleteNode,
    updateNodeData: store.updateNodeData,
    updateNodeDimensions: store.updateNodeDimensions,
    runWorkflow: store.runWorkflow,
    pauseWorkflow: store.pauseWorkflow,
    resumeWorkflow: store.resumeWorkflow,
    stopWorkflow: store.stopWorkflow,
    runNode: store.runNode,
    saveWorkflow: store.saveWorkflow,
    loadWorkflow: store.loadWorkflow,
    deleteWorkflow: store.deleteWorkflow,
    renameWorkflow: store.renameWorkflow,
    createNewWorkflow: store.createNewWorkflow
  };
}
