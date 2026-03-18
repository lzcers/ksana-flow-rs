import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const reduceNodeMetadata: NodeMetadata = {
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
  defaultConfig: {
    reducer: 'sum',
    separator: '\n',
  },
  defaultSize: { width: 300, height: 220 },
};
