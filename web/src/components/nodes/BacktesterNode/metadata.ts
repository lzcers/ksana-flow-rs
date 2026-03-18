import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const backtesterNodeMetadata: NodeMetadata = {
  type: 'Backtester',
  displayName: 'Backtester',
  category: 'output',
  icon: 'line-chart',
  description: '回测执行节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'signal', label: 'Signal', kind: 'data', dataType: 'json', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'report', label: 'Report', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    init_money: 100000,
    fee_rate: 0.0003,
  },
  defaultSize: { width: 250, height: 140 },
};
