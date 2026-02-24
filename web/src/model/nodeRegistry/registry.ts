/**
 * 节点注册表实现
 *
 * 管理所有节点类型的元数据，提供注册、查询、端口获取等功能
 */

import type { NodeMetadata, NodePorts, PortDef, DataType } from './types';
import { Position } from '@xyflow/react';

// ============ 注册表存储 ============

const registry = new Map<string, NodeMetadata>();

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

  // 控制流端口
  if (handleId === 'ctrl') {
    const portList = handleType === 'source' ? ports.outputs : ports.inputs;
    return portList.find(p => p.kind === 'control');
  }

  // 数据流端口
  if (handleId.startsWith('data:')) {
    const portId = handleId.slice(5);
    const portList = handleType === 'source' ? ports.outputs : ports.inputs;
    return portList.find(p => p.id === portId);
  }

  // 兼容旧格式
  const portList = handleType === 'source' ? ports.outputs : ports.inputs;
  return portList.find(p => p.kind === 'control');
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

  // 源为 any，需要目标也接受 any
  if (source === 'any') return target === 'any';

  // 查找兼容性矩阵
  return TYPE_COMPATIBILITY[source]?.includes(target) ?? false;
}

// ============ 内置节点定义 ============

/**
 * 创建默认的控制流端口
 */
function createControlPorts(): { inputs: PortDef[]; outputs: PortDef[] } {
  return {
    inputs: [
      {
        id: 'ctrl',
        label: '',
        kind: 'control',
        position: Position.Left,
      },
    ],
    outputs: [
      {
        id: 'ctrl',
        label: '',
        kind: 'control',
        position: Position.Right,
      },
    ],
  };
}

// 注册内置节点（后续可移到单独文件）
registerNode({
  type: 'TextNode',
  displayName: 'Text',
  category: 'input',
  icon: 'file-text',
  description: '静态文本输入节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'text', label: 'Text', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {
    text: '',
    isMarkdown: false,
  },
  defaultSize: { width: 280, height: 200 },
});

registerNode({
  type: 'LLMNode',
  displayName: 'LLM',
  category: 'ai',
  icon: 'bot',
  description: '大语言模型节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'system', label: 'System', kind: 'data', dataType: 'string', position: Position.Left },
      { id: 'user', label: 'User', kind: 'data', dataType: 'string', position: Position.Left },
      { id: 'context', label: 'Context', kind: 'data', dataType: 'json', position: Position.Left, multiple: true },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'output', label: 'Output', kind: 'data', dataType: 'string', position: Position.Right },
      { id: 'usage', label: 'Usage', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    model: 'deepseek-chat',
    stream: true,
    systemPrompt: '',
    userPrompt: '',
  },
  defaultSize: { width: 320, height: 400 },
});

registerNode({
  type: 'TextMergeNode',
  displayName: 'Text Merge',
  category: 'transform',
  icon: 'merge',
  description: '合并多个文本输入',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'text', label: 'Text', kind: 'data', dataType: 'string', position: Position.Left, multiple: true },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'merged', label: 'Merged', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {
    separator: '\n',
  },
  defaultSize: { width: 200, height: 120 },
});

registerNode({
  type: 'TextSplitNode',
  displayName: 'Text Split',
  category: 'transform',
  icon: 'split',
  description: '分割文本为多个部分',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'text', label: 'Text', kind: 'data', dataType: 'string', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'parts', label: 'Parts', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    separator: '\n',
    maxChunkSize: 1000,
  },
  defaultSize: { width: 200, height: 120 },
});

registerNode({
  type: 'MapNode',
  displayName: 'Map',
  category: 'flow',
  icon: 'layers',
  description: '并行处理数组中的每个元素',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'items', label: 'Items', kind: 'data', dataType: 'json', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'results', label: 'Results', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    maxParallel: 4,
  },
  defaultSize: { width: 200, height: 150 },
});

registerNode({
  type: 'ReduceNode',
  displayName: 'Reduce',
  category: 'flow',
  icon: 'git-merge',
  description: '将多个输入聚合为单个输出',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'items', label: 'Items', kind: 'data', dataType: 'json', position: Position.Left, multiple: true },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'result', label: 'Result', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {},
  defaultSize: { width: 200, height: 120 },
});

registerNode({
  type: 'SubgraphNode',
  displayName: 'Subgraph',
  category: 'flow',
  icon: 'git-branch',
  description: '子流程节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
    ],
  },
  defaultConfig: {
    subgraph: null,
  },
  defaultSize: { width: 180, height: 80 },
});

registerNode({
  type: 'TimerNode',
  displayName: 'Timer',
  category: 'trigger',
  icon: 'clock',
  description: '定时触发节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'timestamp', label: 'Timestamp', kind: 'data', dataType: 'number', position: Position.Right },
    ],
  },
  defaultConfig: {
    interval: 60,
    unit: 'seconds',
  },
  defaultSize: { width: 180, height: 100 },
});

registerNode({
  type: 'SourceNode',
  displayName: 'Source',
  category: 'trigger',
  icon: 'play',
  description: '工作流起始节点',
  ports: {
    inputs: [],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
    ],
  },
  defaultConfig: {},
  defaultSize: { width: 120, height: 80 },
});

registerNode({
  type: 'EmailNotifyNode',
  displayName: 'Email Notify',
  category: 'output',
  icon: 'mail',
  description: '邮件通知节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'subject', label: 'Subject', kind: 'data', dataType: 'string', position: Position.Left },
      { id: 'body', label: 'Body', kind: 'data', dataType: 'string', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
    ],
  },
  defaultConfig: {
    to: '',
  },
  defaultSize: { width: 200, height: 120 },
});

registerNode({
  type: 'TextFileNode',
  displayName: 'Text File',
  category: 'input',
  icon: 'file',
  description: '从文件读取文本',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'text', label: 'Text', kind: 'data', dataType: 'string', position: Position.Right },
      { id: 'path', label: 'Path', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {
    path: '',
  },
  defaultSize: { width: 200, height: 120 },
});

registerNode({
  type: 'ImgGenNode',
  displayName: 'Image Gen',
  category: 'ai',
  icon: 'image',
  description: 'AI 图像生成节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'prompt', label: 'Prompt', kind: 'data', dataType: 'string', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'imageUrl', label: 'Image URL', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {
    model: 'dall-e-3',
    size: '1024x1024',
  },
  defaultSize: { width: 200, height: 150 },
});