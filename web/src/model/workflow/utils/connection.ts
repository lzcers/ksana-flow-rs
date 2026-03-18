/**
 * 连接验证工具
 *
 * 验证节点间的连接是否有效，并在创建连接时标准化 handle 与边数据。
 */

import type { Connection } from '@xyflow/react';
import type { Edge, EdgeData, EdgeKind, Node } from '../types';
import type { PortDef } from '../../nodeRegistry/types';
import {
  CONTROL_HANDLE_ID,
  dataHandleId,
  getNodeMetadata,
  getPortByHandleId,
  isDataHandle,
  isDataTypeCompatible,
} from '../../nodeRegistry';

export interface ConnectionValidation {
  valid: boolean;
  error?: string;
  connection?: Connection;
  edgeData?: EdgeData;
}

type EdgeLike = Pick<Edge, 'source' | 'target' | 'sourceHandle' | 'targetHandle'>;

function defaultHandleIdForPort(port: PortDef): string {
  return port.kind === 'control' ? CONTROL_HANDLE_ID : dataHandleId(port.id);
}

function comparableHandleId(handleId?: string | null): string {
  return handleId ?? CONTROL_HANDLE_ID;
}

function resolvePortHandle(
  node: Node,
  handleType: 'source' | 'target',
  handleId?: string | null
): { port?: PortDef; normalizedHandleId?: string } {
  const nodeType = node.type ?? '';
  const port = getPortByHandleId(nodeType, handleType, handleId ?? '');
  if (!port) {
    return {};
  }

  return {
    port,
    normalizedHandleId: defaultHandleIdForPort(port),
  };
}

function hasDuplicateConnection(connection: Connection, edges: EdgeLike[]): boolean {
  return edges.some(
    edge =>
      edge.source === connection.source &&
      edge.target === connection.target &&
      comparableHandleId(edge.sourceHandle) === comparableHandleId(connection.sourceHandle) &&
      comparableHandleId(edge.targetHandle) === comparableHandleId(connection.targetHandle),
  );
}

function buildEdgeData(sourcePort: PortDef, targetPort: PortDef): EdgeData {
  if (sourcePort.kind === 'data') {
    return {
      kind: 'data',
      sourcePort: sourcePort.id,
      targetPort: targetPort.id,
      dataType: sourcePort.dataType,
    };
  }

  return { kind: 'control' };
}

export function getEdgeKindFromHandles(
  sourceHandle?: string | null,
  targetHandle?: string | null
): EdgeKind {
  return isDataHandle(sourceHandle ?? '') && isDataHandle(targetHandle ?? '')
    ? 'data'
    : 'control';
}

export function inferEdgeDataFromHandles(
  sourceHandle?: string | null,
  targetHandle?: string | null,
): EdgeData {
  const kind = getEdgeKindFromHandles(sourceHandle, targetHandle);
  if (kind === 'data') {
    return {
      kind,
      sourcePort: sourceHandle?.slice(5),
      targetPort: targetHandle?.slice(5),
    };
  }

  return { kind };
}

export function validateConnection(
  connection: Connection,
  nodes: Node[],
  edges: Edge[],
): ConnectionValidation {
  if (!connection.source || !connection.target) {
    return { valid: false, error: '连接缺少源节点或目标节点' };
  }

  const sourceNode = nodes.find(node => node.id === connection.source);
  const targetNode = nodes.find(node => node.id === connection.target);

  if (!sourceNode || !targetNode) {
    return { valid: false, error: '找不到源节点或目标节点' };
  }

  if (!sourceNode.type || !targetNode.type) {
    return { valid: false, error: '节点类型缺失' };
  }

  if (!getNodeMetadata(sourceNode.type) || !getNodeMetadata(targetNode.type)) {
    return { valid: false, error: '节点类型未注册，无法解析端口' };
  }

  const source = resolvePortHandle(sourceNode, 'source', connection.sourceHandle);
  const target = resolvePortHandle(targetNode, 'target', connection.targetHandle);

  if (!source.port || !source.normalizedHandleId) {
    return { valid: false, error: '源端口不存在' };
  }
  if (!target.port || !target.normalizedHandleId) {
    return { valid: false, error: '目标端口不存在' };
  }

  if (source.port.kind !== target.port.kind) {
    return {
      valid: false,
      error: `端口类型不匹配: ${source.port.kind} → ${target.port.kind}`,
    };
  }

  if (
    source.port.kind === 'data' &&
    !isDataTypeCompatible(source.port.dataType, target.port.dataType)
  ) {
    return {
      valid: false,
      error: `数据类型不兼容: ${source.port.dataType ?? 'any'} → ${target.port.dataType ?? 'any'}`,
    };
  }

  const normalizedConnection: Connection = {
    source: connection.source,
    target: connection.target,
    sourceHandle: source.normalizedHandleId,
    targetHandle: target.normalizedHandleId,
  };

  if (hasDuplicateConnection(normalizedConnection, edges)) {
    return { valid: false, error: '相同端口连接已存在' };
  }

  if (!canPortAcceptConnection(targetNode.id, target.port, 'target', edges)) {
    return { valid: false, error: '目标端口不允许多个输入连接' };
  }

  return {
    valid: true,
    connection: normalizedConnection,
    edgeData: buildEdgeData(source.port, target.port),
  };
}

export function createEdgeDataFromConnection(
  connection: Connection,
  nodes: Node[],
  edges: Edge[],
): EdgeData | null {
  const result = validateConnection(connection, nodes, edges);
  return result.valid ? result.edgeData ?? {} : null;
}

export function getConnectionPorts(
  connection: Connection,
  nodes: Node[],
): { sourcePort?: PortDef; targetPort?: PortDef } {
  const sourceNode = nodes.find(node => node.id === connection.source);
  const targetNode = nodes.find(node => node.id === connection.target);

  if (!sourceNode || !targetNode) {
    return {};
  }

  return {
    sourcePort: resolvePortHandle(sourceNode, 'source', connection.sourceHandle).port,
    targetPort: resolvePortHandle(targetNode, 'target', connection.targetHandle).port,
  };
}

export function isControlConnection(connection: Connection): boolean {
  return getEdgeKindFromHandles(connection.sourceHandle, connection.targetHandle) === 'control';
}

export function isDataConnection(connection: Connection): boolean {
  return getEdgeKindFromHandles(connection.sourceHandle, connection.targetHandle) === 'data';
}

export function getPortConnectionCount(
  nodeId: string,
  port: PortDef,
  handleType: 'source' | 'target',
  edges: EdgeLike[],
): number {
  const handleId = defaultHandleIdForPort(port);

  if (handleType === 'source') {
    return edges.filter(
      edge =>
        edge.source === nodeId &&
        comparableHandleId(edge.sourceHandle) === comparableHandleId(handleId),
    ).length;
  }

  return edges.filter(
    edge =>
      edge.target === nodeId &&
      comparableHandleId(edge.targetHandle) === comparableHandleId(handleId),
  ).length;
}

export function canPortAcceptConnection(
  nodeId: string,
  port: PortDef,
  handleType: 'source' | 'target',
  edges: EdgeLike[],
): boolean {
  if (port.kind === 'control' || port.multiple) {
    return true;
  }

  return getPortConnectionCount(nodeId, port, handleType, edges) === 0;
}
