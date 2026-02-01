import type { ComponentType } from 'react';
import type { NodeProps } from '@xyflow/react';
import { Activity, Box, Clapperboard, FileText, GitMerge, Image, Mail, Sparkles, Timer, Type, Database, Group, Repeat2, Sigma } from 'lucide-react';
import type { NodeData, NodeType } from '../../model/types';
import {
  BacktesterNode,
  EmailNotifyNode,
  ImgGenNode,
  LLMNode,
  MapNode,
  ReduceNode,
  ShortVideoNode,
  SourceNode,
  TextFileNode,
  TextMergeNode,
  TextNode,
  TimerNode,
  VolMfiNode,
  SubgraphNode,
} from '../nodes';
import { TextSplitNode } from '../nodes/TextSplitNode';

export type NodeRegistryItem = {
  type: NodeType;
  label: string;
  icon: any;
  color: string;
  Component: ComponentType<NodeProps & { data: NodeData }>;
};

export const NODE_REGISTRY: NodeRegistryItem[] = [
  { type: 'TextNode', label: 'Text', icon: Type, color: 'text-slate-500 bg-slate-50', Component: TextNode },
  { type: 'TextMergeNode', label: 'Merge', icon: GitMerge, color: 'text-cyan-500 bg-cyan-50', Component: TextMergeNode },
  { type: 'TextFileNode', label: 'File', icon: FileText, color: 'text-slate-500 bg-slate-50', Component: TextFileNode },
  { type: 'TextSplitNode', label: 'Split', icon: FileText, color: 'text-slate-500 bg-slate-50', Component: TextSplitNode },

  { type: 'MapNode', label: 'Map', icon: Repeat2, color: 'text-zinc-500 bg-zinc-50', Component: MapNode },
  { type: 'ReduceNode', label: 'Reduce', icon: Sigma, color: 'text-zinc-500 bg-zinc-50', Component: ReduceNode },
  { type: 'LLMNode', label: 'LLM', icon: Sparkles, color: 'text-purple-500 bg-purple-50', Component: LLMNode },
  { type: 'ImgGenNode', label: 'Image', icon: Image, color: 'text-emerald-500 bg-emerald-50', Component: ImgGenNode },
  { type: 'ReactiveSourceNode', label: 'Source', icon: Database, color: 'text-indigo-500 bg-indigo-50', Component: SourceNode },
  { type: 'VOLMFINode', label: 'Strategy', icon: Activity, color: 'text-orange-500 bg-orange-50', Component: VolMfiNode },
  { type: 'Backtester', label: 'Backtest', icon: Box, color: 'text-indigo-500 bg-indigo-50', Component: BacktesterNode },
  { type: 'EmailNotifyNode', label: 'EmailNotifyNode', icon: Mail, color: 'text-indigo-500 bg-indigo-50', Component: EmailNotifyNode },
  { type: 'TimerNode', label: 'TimerNode', icon: Timer, color: 'text-indigo-500 bg-indigo-50', Component: TimerNode },
  { type: 'ShortVideoScriptNode', label: 'AI Video', icon: Clapperboard, color: 'text-rose-500 bg-rose-50', Component: ShortVideoNode },
  { type: 'SubgraphNode', label: 'Group', icon: Group, color: 'text-zinc-500 bg-zinc-50', Component: SubgraphNode },
];

export const NODE_TYPES = NODE_REGISTRY.map(({ type, label, icon, color }) => ({ type, label, icon, color }));

export const NODE_COMPONENTS: Partial<Record<NodeType, ComponentType<NodeProps & { data: NodeData }>>> = Object.fromEntries(
  NODE_REGISTRY.map(({ type, Component }) => [type, Component]),
);
