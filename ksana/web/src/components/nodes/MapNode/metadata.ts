import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const mapNodeMetadata: NodeMetadata = {
  type: 'MapNode',
  displayName: 'Map',
  category: 'flow',
  icon: 'layers',
  description: '并行处理数组中的每个元素',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'items', label: 'Items', kind: 'data', dataType: 'json', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'results', label: 'Results', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    max_concurrency: 2,
    streaming: false,
  },
  defaultSize: { width: 600, height: 400 },
};
