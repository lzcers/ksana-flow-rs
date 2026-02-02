import type { WorkflowState } from './types';

export * from './nodeOperator';
export * from './edgeOperator';
export * from './utils';

export const initialWorkflowState: WorkflowState = {
    nodes: [],
    edges: [],
    selectedNodeId: null,
};
