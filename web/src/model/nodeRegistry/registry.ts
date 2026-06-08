/**
 * 节点注册表实现
 *
 * 管理所有节点类型的元数据，提供注册、查询、端口获取等功能
 */

import type { NodeMetadata, NodePorts, PortDef, DataType } from './types';
import { CONTROL_HANDLE_ID, isControlHandle, isDataHandle } from './types';
import { BUILTIN_NODE_METADATA } from './builtinNodes';

// ============ 注册表存储 ============

const registry = new Map<string, NodeMetadata>();

function cloneRegistryValue<T>(value: T): T {
  if (typeof structuredClone === 'function') {
    return structuredClone(value);
  }

  return JSON.parse(JSON.stringify(value)) as T;
}

// ============ 注册表 API ============

/**
 * 注册节点元数据
 */
export function registerNode(metadata: NodeMetadata): void {
  if (registry.has(metadata.type)) {
    console.warn(`Node type "${metadata.type}" is already registered. Overwriting.`);
  }
  registry.set(metadata.type, metadata);
}

/**
 * 获取节点元数据
 */
export function getNodeMetadata(type: string): NodeMetadata | undefined {
  return registry.get(type);
}

/**
 * 获取节点的端口定义
 */
export function getNodePorts(type: string): NodePorts | undefined {
  return registry.get(type)?.ports;
}

/**
 * 获取节点默认配置
 */
export function getNodeDefaultConfig(type: string): Record<string, unknown> | undefined {
  const defaultConfig = registry.get(type)?.defaultConfig;
  if (!defaultConfig) return undefined;
  return cloneRegistryValue(defaultConfig);
}

/**
 * 获取节点默认尺寸
 */
export function getNodeDefaultSize(type: string): { width: number; height: number } | undefined {
  const defaultSize = registry.get(type)?.defaultSize;
  if (!defaultSize) return undefined;
  return { ...defaultSize };
}

/**
 * 获取所有已注册的节点类型
 */
export function getRegisteredNodeTypes(): string[] {
  return Array.from(registry.keys());
}

/**
 * 获取所有已注册的节点元数据
 */
export function getAllNodeMetadata(): NodeMetadata[] {
  return Array.from(registry.values());
}

/**
 * 按分类获取节点元数据
 */
export function getNodesByCategory(category: string): NodeMetadata[] {
  return Array.from(registry.values()).filter(meta => meta.category === category);
}

/**
 * 获取特定端口定义
 */
export function getPortDef(
  nodeType: string,
  portKind: 'input' | 'output',
  portId: string
): PortDef | undefined {
  const ports = getNodePorts(nodeType);
  if (!ports) return undefined;

  const portList = portKind === 'input' ? ports.inputs : ports.outputs;
  return portList.find(p => p.id === portId);
}

/**
 * 根据 Handle ID 查找端口定义
 */
export function getPortByHandleId(
  nodeType: string,
  handleType: 'source' | 'target',
  handleId: string
): PortDef | undefined {
  const ports = getNodePorts(nodeType);
  if (!ports) return undefined;
  const portList = handleType === 'source' ? ports.outputs : ports.inputs;

  // 缺省 handle 视为默认控制流端口，兼容旧控制流边
  if (!handleId || handleId === CONTROL_HANDLE_ID) {
    return portList.find(p => p.kind === 'control');
  }

  // 数据流端口
  if (isDataHandle(handleId)) {
    const portId = handleId.slice(5);
    return portList.find(p => p.id === portId);
  }

  // 兼容旧控制流 handle 格式
  if (isControlHandle(handleId)) {
    return portList.find(p => p.kind === 'control');
  }

  return undefined;
}

// ============ 数据类型兼容性检查 ============

/**
 * 数据类型兼容性矩阵
 * 定义哪些类型可以自动转换
 */
const TYPE_COMPATIBILITY: Record<DataType, DataType[]> = {
  string: ['string', 'any'],
  number: ['number', 'any'],
  boolean: ['boolean', 'any'],
  json: ['json', 'any'],
  binary: ['binary', 'any'],
  any: ['string', 'number', 'boolean', 'json', 'binary', 'any'],
};

/**
 * 检查数据类型是否兼容
 * @param sourceType 源端口数据类型
 * @param targetType 目标端口数据类型
 * @returns 是否可以连接
 */
export function isDataTypeCompatible(
  sourceType: DataType | undefined,
  targetType: DataType | undefined
): boolean {
  // 未定义类型视为 any
  const source = sourceType ?? 'any';
  const target = targetType ?? 'any';

  // 相同类型
  if (source === target) return true;

  // 目标为 any，接受任何类型
  if (target === 'any') return true;

  // 源为 any，需要目标也接受 any（已在上面处理）
  // 查找兼容性矩阵
  return TYPE_COMPATIBILITY[source]?.includes(target) ?? false;
}

// ============ 内置节点定义 ============

BUILTIN_NODE_METADATA.forEach(registerNode);
