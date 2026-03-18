import { Position } from '@xyflow/react';
import type { NodeMetadata } from '@/model/nodeRegistry/types';

export const subgraphNodeMetadata: NodeMetadata = {
  type: 'SubgraphNode',
  displayName: 'Subgraph',
  category: 'flow',
  icon: 'git-branch',
  description: '子流程节点',
  ports: {
    inputs: [{ id: 'ctrl', label: '', kind: 'control', position: Position.Left }],
    outputs: [{ id: 'ctrl', label: '', kind: 'control', position: Position.Right }],
  },
  defaultConfig: {
    subgraph: null,
  },
  defaultSize: { width: 400, height: 300 },
};
