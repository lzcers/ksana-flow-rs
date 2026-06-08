import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const sourceNodeMetadata: NodeMetadata = {
  type: 'ReactiveSourceNode',
  displayName: 'Source',
  category: 'trigger',
  icon: 'play',
  description: '行情数据源节点',
  ports: {
    inputs: [],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'marketData', label: 'Market Data', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    code: '',
    start_time: '',
    end_time: '',
    product: 'FUND',
  },
  defaultSize: { width: 280, height: 140 },
};
