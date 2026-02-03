/**
 * Processors 入口
 * 导出所有处理器函数
 */

// Node 处理器
export {
  processAddNode,
  processRemoveNode,
  processUpdateNodeData,
  processUpdateNodePosition,
  processUpdateNodeDimensions,
  processSelectNode,
  processApplyNodeChanges,
} from './nodeProcessors';

// Edge 处理器
export {
  processAddEdge,
  processRemoveEdge,
  processOnConnect,
  processUpdateEdge,
  processSetEdges,
  processApplyEdgeChanges,
} from './edgeProcessors';

// Graph 处理器
export {
  processPasteNodes,
  processGroupNodes,
  processToggleSubgraph,
  processSetNodes,
  processBatch,
} from './graphProcessors';
