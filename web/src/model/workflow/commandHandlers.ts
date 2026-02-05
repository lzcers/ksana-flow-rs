/**
 * Command Handlers 注册
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
  workflow.registerHandler('UPDATE_NODE_DATA', nodeProcessors.processUpdateNodeData);
  workflow.registerHandler('UPDATE_NODE_POSITION', nodeProcessors.processUpdateNodePosition);
  workflow.registerHandler('UPDATE_NODE_DIMENSIONS', nodeProcessors.processUpdateNodeDimensions);
  workflow.registerHandler('UPDATE_NODE_STATUS', nodeProcessors.processUpdateNodeStatus);
  workflow.registerHandler('UPDATE_NODE_INPUT', nodeProcessors.processUpdateNodeInput);
  workflow.registerHandler('UPDATE_NODE_INPUTS', nodeProcessors.processUpdateNodeInputs);
  workflow.registerHandler('UPDATE_NODE_OUTPUT', nodeProcessors.processUpdateNodeOutput);
  workflow.registerHandler('SELECT_NODE', nodeProcessors.processSelectNode);
  workflow.registerHandler('APPLY_NODE_CHANGES', nodeProcessors.processApplyNodeChanges);
  workflow.registerHandler('RESET_ALL_NODE_STATUS', nodeProcessors.processResetAllNodeStatus);

  // ===== Edge Handlers =====
  workflow.registerHandler('ADD_EDGE', edgeProcessors.processAddEdge);
  workflow.registerHandler('REMOVE_EDGE', edgeProcessors.processRemoveEdge);
  workflow.registerHandler('ON_CONNECT', edgeProcessors.processOnConnect);
  workflow.registerHandler('UPDATE_EDGE', edgeProcessors.processUpdateEdge);
  workflow.registerHandler('SET_EDGES', edgeProcessors.processSetEdges);
  workflow.registerHandler('APPLY_EDGE_CHANGES', edgeProcessors.processApplyEdgeChanges);

  // ===== Graph Handlers =====
  workflow.registerHandler('PASTE_NODES', graphProcessors.processPasteNodes);
  workflow.registerHandler('GROUP_NODES', graphProcessors.processGroupNodes);
  workflow.registerHandler('TOGGLE_SUBGRAPH', graphProcessors.processToggleSubgraph);
  workflow.registerHandler('SET_NODES', graphProcessors.processSetNodes);
  workflow.registerHandler('RESET_EXECUTION_STATE', graphProcessors.processResetExecutionState);
  workflow.registerHandler('HANDLE_NODE_DRAG_STOP', processHandleNodeDragStop);

}
