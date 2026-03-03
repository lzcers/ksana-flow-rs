/**
 * 连接验证工具
 *
 * 验证节点间的连接是否有效，并在创建连接时自动填充边数据
 */

import type { Connection, Node } from '@xyflow/react';
import type { EdgeData, NodeData } from '../types';
import type { PortDef } from '../../nodeRegistry/types';
import { isDataTypeCompatible, getPortByHandleId } from '../../nodeRegistry';

/**
 * 连接验证结果
 */
export interface ConnectionValidation {
  valid: boolean;
  error?: string;
  edgeData?: EdgeData;
}

/**
 * 验证连接是否有效
 *
 * @param connection 连接信息
 * @param nodes 节点列表
 * @returns 验证结果
 */
export function validateConnection(
  connection: Connection,
  nodes: Node<NodeData>[]
): ConnectionValidation {
  const sourceNode = nodes.find(n => n.id === connection.source);
  const targetNode = nodes.find(n => n.id === connection.target);

  if (!sourceNode || !targetNode) {
    return { valid: false, error: '找不到源节点或目标节点' };
  }

  // 获取端口定义
  const sourcePort = getPortByHandleId(
    sourceNode.type || '',
    'source',
    connection.sourceHandle || ''
  );
  const targetPort = getPortByHandleId(
    targetNode.type || '',
    'target',
    connection.targetHandle || ''
  );

  // 检查端口是否存在
  if (!sourcePort) {
    return { valid: false, error: '源端口不存在' };
  }
  if (!targetPort) {
    return { valid: false, error: '目标端口不存在' };
  }

  // 检查端口类型是否匹配（控制流只能连控制流，数据流只能连数据流）
  if (sourcePort.kind !== targetPort.kind) {
    return {
      valid: false,
      error: `端口类型不匹配: ${sourcePort.kind} → ${targetPort.kind}`,
    };
  }

  // 数据流端口检查数据类型兼容性
  if (sourcePort.kind === 'data') {
    if (!isDataTypeCompatible(sourcePort.dataType, targetPort.dataType)) {
      return {
        valid: false,
        error: `数据类型不兼容: ${sourcePort.dataType || 'any'} → ${targetPort.dataType || 'any'}`,
      };
    }
  }

  // 检查目标端口是否允许多个连接
  if (!targetPort.multiple) {
    // 这里需要检查是否已有其他连接连入该端口
    // 暂时跳过，在更高层级处理
  }

  // 构建边数据
  const edgeData: EdgeData = {
    kind: sourcePort.kind,
  };

  // 数据流边添加端口信息
  if (sourcePort.kind === 'data') {
    edgeData.sourcePort = sourcePort.id;
    edgeData.targetPort = targetPort.id;
    edgeData.dataType = sourcePort.dataType;
  }

  return { valid: true, edgeData };
}

/**
 * 从连接创建边数据
 *
 * @param connection 连接信息
 * @param nodes 节点列表
 * @returns 边数据，如果验证失败则返回 null
 */
export function createEdgeDataFromConnection(
  connection: Connection,
  nodes: Node<NodeData>[]
): EdgeData | null {
  const result = validateConnection(connection, nodes);
  return result.valid ? result.edgeData || {} : null;
}

/**
 * 获取连接的端口信息
 */
export function getConnectionPorts(
  connection: Connection,
  nodes: Node<NodeData>[]
): { sourcePort?: PortDef; targetPort?: PortDef } {
  const sourceNode = nodes.find(n => n.id === connection.source);
  const targetNode = nodes.find(n => n.id === connection.target);

  if (!sourceNode || !targetNode) {
    return {};
  }

  return {
    sourcePort: getPortByHandleId(
      sourceNode.type || '',
      'source',
      connection.sourceHandle || ''
    ),
    targetPort: getPortByHandleId(
      targetNode.type || '',
      'target',
      connection.targetHandle || ''
    ),
  };
}

/**
 * 检查是否为控制流连接
 */
export function isControlConnection(connection: Connection): boolean {
  return connection.sourceHandle === 'ctrl' && connection.targetHandle === 'ctrl';
}

/**
 * 检查是否为数据流连接
 */
export function isDataConnection(connection: Connection): boolean {
  return (
    (connection.sourceHandle?.startsWith('data:') ?? false) &&
    (connection.targetHandle?.startsWith('data:') ?? false)
  );
}

/**
 * 获取端口的连接计数
 *
 * @param nodeId 节点 ID
 * @param portId 端口 ID
 * @param handleType 端口类型
 * @param edges 边列表
 * @returns 连接到该端口的边数量
 */
export function getPortConnectionCount(
  nodeId: string,
  portId: string,
  handleType: 'source' | 'target',
  edges: Array<{ source: string; target: string; sourceHandle?: string | null; targetHandle?: string | null }>
): number {
  const handleId = `data:${portId}`;

  if (handleType === 'source') {
    return edges.filter(
      e => e.source === nodeId && e.sourceHandle === handleId
    ).length;
  } else {
    return edges.filter(
      e => e.target === nodeId && e.targetHandle === handleId
    ).length;
  }
}

/**
 * 检查端口是否可以接受新连接
 *
 * @param nodeId 节点 ID
 * @param port 端口定义
 * @param edges 边列表
 * @returns 是否可以接受新连接
 */
export function canPortAcceptConnection(
  nodeId: string,
  port: PortDef,
  edges: Array<{ source: string; target: string; sourceHandle?: string | null; targetHandle?: string | null }>
): boolean {
  // 控制流端口或允许多连接的端口
  if (port.kind === 'control' || port.multiple) {
    return true;
  }

  // 数据流端口检查是否已有连接
  const count = getPortConnectionCount(nodeId, port.id, 'target', edges);
  return count === 0;
}