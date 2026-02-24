import type {
  EdgeChange,
  Connection,
  Node as XNode,
  Edge as XEdge,
  NodeChange as XNodeChange,
} from '@xyflow/react';
import type { DataType } from '../nodeRegistry/types';

export type NodeType =
  | 'LLMNode'
  | 'TextNode'
  | 'TextMergeNode'
  | 'TextFileNode'
  | 'EmailNotifyNode'
  | 'TimerNode'
  | 'Backtester'
  | 'ReactiveSourceNode'
  | 'VOLMFINode'
  | 'SubgraphNode'
  | 'MapNode'
  | 'ReduceNode'
  | 'ImgGenNode'
  | 'SourceNode'
  | string;

export type NodeStatus = 'idle' | 'running' | 'completed' | 'error';

export type WorkflowStatus = 'idle' | 'running' | 'paused';

/**
 * 边类型
 * - control: 控制流边，决定节点执行顺序
 * - data: 数据流边，在节点间传递数据
 */
export type EdgeKind = 'control' | 'data';

export interface NodeData extends Record<string, unknown> {
  label?: string;   // 名称
  inputs?: Record<string, any>;   // 端口输入值（从数据流边接收）
  outputs?: Record<string, any>;  // 端口输出值（节点执行后产生）
  config?: Record<string, any>; // 配置
  status?: NodeStatus; // 运行状态
  errorMessage?: string; // 错误消息
  lastMessage?: any;    // 最后的消息
  isOutputStream?: boolean; // 是否输出流
  expanded?: boolean; // 是否展开，Node Group
  expandedSize?: { width: number; height: number }; // 展开大小
  collapsedSize?: { width: number; height: number }; // 收起大小
}

export interface EdgeData extends Record<string, unknown> {
  /** 边类型：控制流或数据流 */
  kind?: EdgeKind;

  /** 源端口 ID（数据流边） */
  sourcePort?: string;

  /** 目标端口 ID（数据流边） */
  targetPort?: string;

  /** 数据类型（数据流边，冗余存储便于验证） */
  dataType?: DataType;

  /** UI 内部使用：子图边标记 */
  __uiSubgraphEdge?: { originalEdgeId: string };
}
export type Node = XNode<NodeData>;
export type Edge = XEdge<EdgeData>;
export type NodeChange = XNodeChange<Node>;

export interface WorkflowState {
  nodes: Node[];
  edges: Edge[];
}

export type { EdgeChange, Connection };
