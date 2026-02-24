/**
 * 节点注册表模块入口
 *
 * 导出所有类型和 API
 */

// 类型导出
export type {
  PortKind,
  DataType,
  PortDef,
  NodePorts,
  NodeMetadata,
} from './types';

// 常量和工具函数导出
export {
  CONTROL_HANDLE_ID,
  dataHandleId,
  parseHandleId,
  isControlHandle,
  isDataHandle,
} from './types';

// 注册表 API 导出
export {
  registerNode,
  getNodeMetadata,
  getNodePorts,
  getRegisteredNodeTypes,
  getAllNodeMetadata,
  getNodesByCategory,
  getPortDef,
  getPortByHandleId,
  isDataTypeCompatible,
} from './registry';