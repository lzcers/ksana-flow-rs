import { Sparkles, Database, Activity, Box, Type } from 'lucide-react';
import type { NodeType } from '../../types/workflow';

export const NODE_TYPES: { type: NodeType; label: string; icon: any; color: string }[] = [
  { type: 'TextNode', label: 'Text', icon: Type, color: 'text-slate-500 bg-slate-50' },
  { type: 'LLMNode', label: 'LLM', icon: Sparkles, color: 'text-purple-500 bg-purple-50' },
  { type: 'ReactiveSourceNode', label: 'Source', icon: Database, color: 'text-blue-500 bg-blue-50' },
  { type: 'VOLMFINode', label: 'Strategy', icon: Activity, color: 'text-orange-500 bg-orange-50' },
  { type: 'Backtester', label: 'Backtest', icon: Box, color: 'text-indigo-500 bg-indigo-50' },
];
