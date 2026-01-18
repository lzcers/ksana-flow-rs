import { useStore } from '../store';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

export function useWorkflow() {
  const store = useStore();

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
