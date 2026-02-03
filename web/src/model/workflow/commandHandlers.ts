/**
 * Command Handlers 注册
 * 将处理器函数注册到 CommandBus
 */

import type { RxWorkflow } from './workflowRx';
import * as graphProcessors from './processors/graphProcessors';
import * as nodeProcessors from './processors/nodeProcessors';
import * as edgeProcessors from './processors/edgeProcessors';
import { processHandleNodeDragStop } from './processors/layoutProcessors';

/**
 * 注册所有 Command 处理器
 */
export function registerAllHandlers(rxWorkflow: RxWorkflow): void {
  // ===== Node Handlers =====
  rxWorkflow.registerHandler('ADD_NODE', nodeProcessors.processAddNode as any);
  rxWorkflow.registerHandler('REMOVE_NODE', nodeProcessors.processRemoveNode as any);
  rxWorkflow.registerHandler('UPDATE_NODE_DATA', nodeProcessors.processUpdateNodeData as any);
  rxWorkflow.registerHandler('UPDATE_NODE_POSITION', nodeProcessors.processUpdateNodePosition as any);
  rxWorkflow.registerHandler('UPDATE_NODE_DIMENSIONS', nodeProcessors.processUpdateNodeDimensions as any);
  rxWorkflow.registerHandler('UPDATE_NODE_STATUS', nodeProcessors.processUpdateNodeStatus as any);
  rxWorkflow.registerHandler('UPDATE_NODE_INPUT', nodeProcessors.processUpdateNodeInput as any);
  rxWorkflow.registerHandler('UPDATE_NODE_INPUTS', nodeProcessors.processUpdateNodeInputs as any);
  rxWorkflow.registerHandler('UPDATE_NODE_OUTPUT', nodeProcessors.processUpdateNodeOutput as any);
  rxWorkflow.registerHandler('SELECT_NODE', nodeProcessors.processSelectNode as any);
  rxWorkflow.registerHandler('APPLY_NODE_CHANGES', nodeProcessors.processApplyNodeChanges as any);

  // ===== Edge Handlers =====
  rxWorkflow.registerHandler('ADD_EDGE', edgeProcessors.processAddEdge as any);
  rxWorkflow.registerHandler('REMOVE_EDGE', edgeProcessors.processRemoveEdge as any);
  rxWorkflow.registerHandler('ON_CONNECT', edgeProcessors.processOnConnect as any);
  rxWorkflow.registerHandler('UPDATE_EDGE', edgeProcessors.processUpdateEdge as any);
  rxWorkflow.registerHandler('SET_EDGES', edgeProcessors.processSetEdges as any);
  rxWorkflow.registerHandler('APPLY_EDGE_CHANGES', edgeProcessors.processApplyEdgeChanges as any);

  // ===== Graph Handlers =====
  rxWorkflow.registerHandler('PASTE_NODES', graphProcessors.processPasteNodes as any);
  rxWorkflow.registerHandler('GROUP_NODES', graphProcessors.processGroupNodes as any);
  rxWorkflow.registerHandler('TOGGLE_SUBGRAPH', graphProcessors.processToggleSubgraph as any);
  rxWorkflow.registerHandler('SET_NODES', graphProcessors.processSetNodes as any);
  rxWorkflow.registerHandler('RESET_EXECUTION_STATE', graphProcessors.processResetExecutionState as any);
  rxWorkflow.registerHandler('HANDLE_NODE_DRAG_STOP', processHandleNodeDragStop as any);

  console.log('[CommandHandlers] All handlers registered');
}
