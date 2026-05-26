import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const textNodeMetadata: NodeMetadata = {
  type: 'TextNode',
  displayName: 'Text',
  category: 'input',
  icon: 'file-text',
  description: '静态文本输入节点',
  ports: {
    inputs: [{ id: 'ctrl', label: '', kind: 'control', position: Position.Left }],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'text', label: 'Text', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {
    text: '',
    isMarkdown: false,
  },
  defaultSize: { width: 260, height: 200 },
};
