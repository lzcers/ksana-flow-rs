import type { WorkflowState } from './types';

// 导出原有 API（向后兼容）
export * from './nodeOperator';
export * from './edgeOperator';
export * from './utils';

// 导出新的 RxJS API
export * from './rx';
export * from './commands';
export * from './processors';
export { registerAllHandlers } from './commandHandlers';
export * from './workflowModel';

// 初始状态
export const initialWorkflowState: WorkflowState = {
    nodes: [],
    edges: [],
    selectedNodeId: null,
};
