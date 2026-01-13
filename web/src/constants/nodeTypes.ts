import { Play, Settings, MousePointer2, Trash2, Sparkles, Database, Activity, Box } from 'lucide-react';
import type { NodeType } from '../types/workflow';

export const NODE_TYPES: { type: NodeType; label: string; icon: any; color: string }[] = [
  { type: 'start', label: '开始', icon: Play, color: 'text-emerald-500 bg-emerald-50' },
  { type: 'task', label: '任务', icon: Settings, color: 'text-blue-500 bg-blue-50' },
  { type: 'LLMNode', label: 'LLM', icon: Sparkles, color: 'text-purple-500 bg-purple-50' },
  { type: 'ReactiveSourceNode', label: 'Source', icon: Database, color: 'text-blue-500 bg-blue-50' },
  { type: 'VOLMFINode', label: 'Strategy', icon: Activity, color: 'text-orange-500 bg-orange-50' },
  { type: 'Backtester', label: 'Backtest', icon: Box, color: 'text-indigo-500 bg-indigo-50' },
  { type: 'condition', label: '条件', icon: MousePointer2, color: 'text-amber-500 bg-amber-50' },
  { type: 'end', label: '结束', icon: Trash2, color: 'text-rose-500 bg-rose-50' },
];
