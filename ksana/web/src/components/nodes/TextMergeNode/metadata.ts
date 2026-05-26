import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const textMergeNodeMetadata: NodeMetadata = {
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
  defaultSize: { width: 180, height: 150 },
};
