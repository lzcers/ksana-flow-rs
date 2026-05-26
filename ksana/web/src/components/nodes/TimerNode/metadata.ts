import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const timerNodeMetadata: NodeMetadata = {
  type: 'TimerNode',
  displayName: 'Timer',
  category: 'trigger',
  icon: 'clock',
  description: '定时触发节点',
  ports: {
    inputs: [{ id: 'ctrl', label: '', kind: 'control', position: Position.Left }],
    outputs: [
      { id: 'ctrl', label: '', kind: 'control', position: Position.Right },
      { id: 'timestamp', label: 'Timestamp', kind: 'data', dataType: 'number', position: Position.Right },
    ],
  },
  defaultConfig: {
    cron_expr: '',
  },
  defaultSize: { width: 250, height: 120 },
};
