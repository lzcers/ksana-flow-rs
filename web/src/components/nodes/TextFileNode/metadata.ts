import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const textFileNodeMetadata: NodeMetadata = {
  type: 'TextFileNode',
  displayName: 'Text File',
  category: 'input',
  icon: 'file',
  description: '从文件读取文本',
  ports: {
    inputs: [{ id: 'ctrl', label: '', kind: 'control', position: Position.Left }],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'text', label: 'Text', kind: 'data', dataType: 'string', position: Position.Right },
      { id: 'path', label: 'Path', kind: 'data', dataType: 'string', position: Position.Right },
    ],
  },
  defaultConfig: {},
  defaultSize: { width: 280, height: 120 },
};
