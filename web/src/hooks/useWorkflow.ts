// 状态选择器优化后的 useWorkflow hook
// 使用细粒度的状态选择器，避免不必要的重渲染
import {
  useNodes,
  useEdges,
  useSelectedNodeId,
  useNodeTypes,
  useWorkflows,
  useCurrentWorkflowId,
  useWorkflowStatus,
  useCurrentRunId,
  useCanvasActions,
  useHistoryActions,
  useWorkflowActions,
  useExecutionActions,
  useHistoryState,
} from './useStoreSelectors';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

/**
 * 优化后的 useWorkflow hook
 * 使用细粒度的状态选择器，避免不必要的重渲染
 */
export function useWorkflow() {
  // 使用独立的选择器订阅状态
  const nodes = useNodes();
  const edges = useEdges();
  const selectedNodeId = useSelectedNodeId();
  const nodeTypes = useNodeTypes();
  const workflows = useWorkflows();
  const currentWorkflowId = useCurrentWorkflowId();
  const workflowStatus = useWorkflowStatus();
  const currentRunId = useCurrentRunId();
  const { canUndo, canRedo } = useHistoryState();

  // Actions 有稳定的引用
  const canvasActions = useCanvasActions();
  const historyActions = useHistoryActions();
  const workflowActions = useWorkflowActions();
  const executionActions = useExecutionActions();

  // 派生状态
  const state = {
    nodes,
    edges,
    selectedNodeId,
  };

  // 合并所有 actions，保持与原有接口兼容
  return {
    state,
    nodeTypes,
    workflows,
    currentWorkflowId,
    workflowStatus,
    workflowStatuses: {} as Record<number, WorkflowStatus>, // 保持接口兼容
    currentRunId,

    // Canvas actions
    onNodesChange: canvasActions.onNodesChange,
    onEdgesChange: canvasActions.onEdgesChange,
    onNodeDragStop: (_event: any, _node: any) => { }, // No-op to match interface
    onNodeDragStart: (_event: any, _node: any) => historyActions.pushHistory(),
    onConnect: canvasActions.onConnect,
    addNode: canvasActions.addNode,
    deleteNode: canvasActions.deleteNode,
    updateNodeData: canvasActions.updateNodeData,
    updateNodeDimensions: canvasActions.updateNodeDimensions,
    groupNodes: canvasActions.groupNodes,

    // History actions
    undo: historyActions.undo,
    redo: historyActions.redo,
    canUndo: () => canUndo,
    canRedo: () => canRedo,

    // Execution actions
    runWorkflow: executionActions.runWorkflow,
    pauseWorkflow: executionActions.pauseWorkflow,
    resumeWorkflow: executionActions.resumeWorkflow,
    stopWorkflow: executionActions.stopWorkflow,
    runNode: executionActions.runNode,

    // Workflow actions
    saveWorkflow: workflowActions.saveWorkflow,
    loadWorkflow: workflowActions.loadWorkflow,
    deleteWorkflow: workflowActions.deleteWorkflow,
    renameWorkflow: workflowActions.renameWorkflow,
    createNewWorkflow: workflowActions.createNewWorkflow,
    importWorkflow: workflowActions.importWorkflow,
    getWorkflowBlueprint: () => ({ nodes: [], edges: [] }), // 简化实现
  };
}
