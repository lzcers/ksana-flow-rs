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
    useWorkflowStatuses,
    useCurrentRunId,
    useActiveGraphKey,
    useIsLoadingWorkflow,
    canvasActions,
    historyActions,
    workflowActions,
    executionActions,
} from "./useStoreSelectors";

export type { WorkflowStatus } from "../store/types";

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
    const workflowStatuses = useWorkflowStatuses();
    const currentRunId = useCurrentRunId();
    const activeGraphKey = useActiveGraphKey();
    const isLoadingWorkflow = useIsLoadingWorkflow();
    // Actions 直接从常量对象引用，不通过 hook（避免无限循环）

    // 派生状态
    const state = {
        activeGraphKey,
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
        workflowStatuses,
        currentRunId,
        isLoadingWorkflow,

        // Canvas actions
        onNodesChange: canvasActions.onNodesChange,
        onEdgesChange: canvasActions.onEdgesChange,
        onNodeDrag: canvasActions.onNodeDrag,
        onNodeDragStop: canvasActions.onNodeDragStop,
        onConnect: canvasActions.onConnect,
        addNode: canvasActions.addNode,
        deleteNode: canvasActions.deleteNode,
        updateNodeData: canvasActions.updateNodeData,
        updateNodeDimensions: canvasActions.updateNodeDimensions,
        groupNodes: canvasActions.groupNodes,
        onPaste: canvasActions.pasteNodes,

        // History actions
        undo: historyActions.undo,
        redo: historyActions.redo,

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
        getWorkflowBlueprint: workflowActions.getWorkflowBlueprint,
    };
}
