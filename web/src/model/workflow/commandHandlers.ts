/**
 * Command Handlers 注册 - 简化版
 * 将处理器函数注册到 CommandBus
 */

import * as graphProcessors from './processors/graphProcessors';
import * as nodeProcessors from './processors/nodeProcessors';
import * as edgeProcessors from './processors/edgeProcessors';
import { processHandleNodeDragStop } from './processors/layoutProcessors';
import type { WorkflowModel } from './workflowModel';

/**
 * 注册所有 Command 处理器
 */
export function registerAllHandlers(workflow: WorkflowModel): void {
  // ===== Node Handlers =====
  workflow.registerHandler('ADD_NODE', nodeProcessors.processAddNode);
  workflow.registerHandler('REMOVE_NODE', nodeProcessors.processRemoveNode);
  // 新的统一 UPDATE_NODE 处理器
  workflow.registerHandler('UPDATE_NODE', nodeProcessors.processUpdateNode);
  workflow.registerHandler('APPLY_NODE_CHANGES', nodeProcessors.processApplyNodeChanges);
  workflow.registerHandler('RESET_ALL_NODE_STATUS', nodeProcessors.processResetAllNodeStatus);
  // ===== Edge Handlers =====
  // 新的统一 UPDATE_EDGES 处理器
  workflow.registerHandler('UPDATE_EDGES', edgeProcessors.processUpdateEdges);
  workflow.registerHandler('SET_EDGES', edgeProcessors.processSetEdges);
  // ===== Graph Handlers =====
  workflow.registerHandler('PASTE_NODES', graphProcessors.processPasteNodes);
  workflow.registerHandler('GROUP_NODES', graphProcessors.processGroupNodes);
  workflow.registerHandler('TOGGLE_SUBGRAPH', graphProcessors.processToggleSubgraph);
  workflow.registerHandler('SET_NODES', graphProcessors.processSetNodes);
  workflow.registerHandler('RESET_EXECUTION_STATE', graphProcessors.processResetExecutionState);
  workflow.registerHandler('HANDLE_NODE_DRAG_STOP', processHandleNodeDragStop);

}
