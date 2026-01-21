import { Sparkles, Database, Activity, Box, Type, Mail, Timer, FileText, GitMerge } from 'lucide-react';
import type { NodeType } from '../../model/types';

export const NODE_TYPES: { type: NodeType; label: string; icon: any; color: string }[] = [
  { type: 'TextNode', label: 'Text', icon: Type, color: 'text-slate-500 bg-slate-50' },
  { type: 'TextMergeNode', label: 'Merge', icon: GitMerge, color: 'text-cyan-500 bg-cyan-50' },
  { type: 'TextFileNode', label: 'File', icon: FileText, color: 'text-slate-500 bg-slate-50' },
  { type: 'LLMNode', label: 'LLM', icon: Sparkles, color: 'text-purple-500 bg-purple-50' },
  { type: 'ReactiveSourceNode', label: 'Source', icon: Database, color: 'text-blue-500 bg-blue-50' },
  { type: 'VOLMFINode', label: 'Strategy', icon: Activity, color: 'text-orange-500 bg-orange-50' },
  { type: 'Backtester', label: 'Backtest', icon: Box, color: 'text-indigo-500 bg-indigo-50' },
  { type: 'EmailNotifyNode', label: 'EmailNotifyNode', icon: Mail, color: 'text-indigo-500 bg-indigo-50' },
  { type: 'TimerNode', label: 'TimerNode', icon: Timer, color: 'text-indigo-500 bg-indigo-50' },

];
