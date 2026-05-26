/**
 * 节点端口类型定义
 *
 * 端口设计原则：
 * 1. 每个节点有两种端口类型：控制流端口（control）和数据流端口（data）
 * 2. 控制流端口决定节点的执行顺序
 * 3. 数据流端口用于在节点间传递数据
 */

import { Position } from '@xyflow/react';

/**
 * 端口类型
 * - control: 控制流端口，决定执行顺序
 * - data: 数据流端口，传递数据值
 */
export type PortKind = 'control' | 'data';

/**
 * 数据类型定义
 * - string: 字符串
 * - number: 数字
 * - boolean: 布尔值
 * - json: JSON 对象
 * - binary: 二进制数据
 * - any: 任意类型
 */
export type DataType = 'string' | 'number' | 'boolean' | 'json' | 'binary' | 'any';

/**
 * 端口定义
 */
export interface PortDef {
  /** 端口唯一标识，如 "prompt", "output", "context" */
  id: string;

  /** 显示名称，如 "提示词", "输出", "上下文" */
  label: string;

  /** 端口类型：控制流或数据流 */
  kind: PortKind;

  /** 数据类型（仅 data 端口有效） */
  dataType?: DataType;

  /** 显示位置 */
  position: Position;

  /** 是否必须连接（默认 false） */
  required?: boolean;

  /** 是否允许多个连接（输入端口有效，默认 false） */
  multiple?: boolean;

  /** 默认值（未连接时使用） */
  defaultValue?: unknown;

  /** 端口描述（用于 tooltip） */
  description?: string;
}

/**
 * 节点的端口配置
 */
export interface NodePorts {
  /** 输入端口列表 */
  inputs: PortDef[];

  /** 输出端口列表 */
  outputs: PortDef[];
}

/**
 * 节点元数据
 * 定义节点类型的静态信息
 */
export interface NodeMetadata {
  /** 节点类型标识，如 "LLMNode", "TextNode" */
  type: string;

  /** 显示名称 */
  displayName: string;

  /** 分类：ai, input, output, transform, logic */
  category: string;

  /** 描述 */
  description?: string;

  /** 图标名称（lucide icon） */
  icon?: string;

  /** 端口定义 */
  ports: NodePorts;

  /** 配置表单 JSON Schema */
  configSchema?: Record<string, unknown>;

  /** 默认配置 */
  defaultConfig?: Record<string, unknown>;

  /** 默认尺寸 */
  defaultSize?: {
    width: number;
    height: number;
  };
}

// ============ Handle ID 编码规则 ============

/**
 * Handle ID 编码工具函数
 *
 * 编码规则：
 * - 控制流端口: "ctrl"
 * - 数据流端口: "data:{portId}"
 *
 * 示例：
 * - 控制流输入: "ctrl"
 * - 数据流输入 (user 端口): "data:user"
 * - 数据流输出 (output 端口): "data:output"
 */

/** 控制流 Handle ID */
export const CONTROL_HANDLE_ID = 'ctrl';

/** 生成数据流 Handle ID */
export function dataHandleId(portId: string): string {
  return `data:${portId}`;
}

/** 解析 Handle ID，返回端口类型和端口 ID */
export function parseHandleId(handleId: string): { kind: PortKind; portId: string } {
  if (handleId === CONTROL_HANDLE_ID) {
    return { kind: 'control', portId: 'ctrl' };
  }

  if (handleId.startsWith('data:')) {
    return { kind: 'data', portId: handleId.slice(5) };
  }

  // 兼容旧格式：t-Left, s-Right 等
  // 将其视为控制流端口
  return { kind: 'control', portId: handleId };
}

/** 判断 Handle ID 是否为控制流端口 */
export function isControlHandle(handleId: string): boolean {
  return handleId === CONTROL_HANDLE_ID || handleId.startsWith('t-') || handleId.startsWith('s-');
}

/** 判断 Handle ID 是否为数据流端口 */
export function isDataHandle(handleId: string): boolean {
  return handleId.startsWith('data:');
}