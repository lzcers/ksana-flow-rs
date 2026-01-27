import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData, NodeType } from '../../model/types';
import { NODE_COMPONENTS } from './nodeRegistry';

export const WorkflowNode = memo((props: NodeProps & { data: NodeData }) => {
  const { type } = props;
  const Component = NODE_COMPONENTS[type as NodeType];
  return Component ? <Component {...props} /> : null;
});

WorkflowNode.displayName = 'WorkflowNode';
