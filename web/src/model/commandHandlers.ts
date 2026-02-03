/**
 * Command Handlers 注册
 * 将处理器函数注册到 CommandBus
 */

import type { RxCommandBus } from './rx';
import * as nodeProcessors from './processors/nodeProcessors';
import * as edgeProcessors from './processors/edgeProcessors';
import * as graphProcessors from './processors/graphProcessors';

/**
 * 注册所有 Command 处理器
 */
export function registerAllHandlers(commandBus: RxCommandBus): void {
  // ===== Node Handlers =====
  commandBus.registerHandler('ADD_NODE', nodeProcessors.processAddNode);
  commandBus.registerHandler('REMOVE_NODE', nodeProcessors.processRemoveNode);
  commandBus.registerHandler('UPDATE_NODE_DATA', nodeProcessors.processUpdateNodeData);
  commandBus.registerHandler('UPDATE_NODE_POSITION', nodeProcessors.processUpdateNodePosition);
  commandBus.registerHandler('UPDATE_NODE_DIMENSIONS', nodeProcessors.processUpdateNodeDimensions);
  commandBus.registerHandler('SELECT_NODE', nodeProcessors.processSelectNode);
  commandBus.registerHandler('APPLY_NODE_CHANGES', nodeProcessors.processApplyNodeChanges);

  // ===== Edge Handlers =====
  commandBus.registerHandler('ADD_EDGE', edgeProcessors.processAddEdge);
  commandBus.registerHandler('REMOVE_EDGE', edgeProcessors.processRemoveEdge);
  commandBus.registerHandler('ON_CONNECT', edgeProcessors.processOnConnect);
  commandBus.registerHandler('UPDATE_EDGE', edgeProcessors.processUpdateEdge);
  commandBus.registerHandler('SET_EDGES', edgeProcessors.processSetEdges);
  commandBus.registerHandler('APPLY_EDGE_CHANGES', edgeProcessors.processApplyEdgeChanges);

  // ===== Graph Handlers =====
  commandBus.registerHandler('PASTE_NODES', graphProcessors.processPasteNodes);
  commandBus.registerHandler('GROUP_NODES', graphProcessors.processGroupNodes);
  commandBus.registerHandler('TOGGLE_SUBGRAPH', graphProcessors.processToggleSubgraph);
  commandBus.registerHandler('SET_NODES', graphProcessors.processSetNodes);

  console.log('[CommandHandlers] All handlers registered');
}
