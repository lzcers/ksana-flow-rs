import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const volMfiNodeMetadata: NodeMetadata = {
  type: 'VOLMFINode',
  displayName: 'VOL MFI',
  category: 'logic',
  icon: 'activity',
  description: '成交量与资金流策略节点',
  ports: {
    inputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Left },
      { id: 'marketData', label: 'Market Data', kind: 'data', dataType: 'json', position: Position.Left },
    ],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'signal', label: 'Signal', kind: 'data', dataType: 'json', position: Position.Right },
    ],
  },
  defaultConfig: {
    ema_period: 20,
    mfi_period: 14,
  },
  defaultSize: { width: 250, height: 140 },
};
