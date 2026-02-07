import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData, NodeType } from '../../model/workflow/types';
import { NODE_COMPONENTS } from './nodeRegistry';
import { useCurrentWorkflowId } from '../../hooks/useStoreSelectors';

export const WorkflowNode = memo((props: NodeProps & { data: NodeData }) => {
  const { type } = props;
  const currentWorkflowId = useCurrentWorkflowId();
  const Component = NODE_COMPONENTS[type as NodeType];
  return Component ? <Component key={`${currentWorkflowId ?? 'new'}:${props.id}`} {...props} /> : null;
});

WorkflowNode.displayName = 'WorkflowNode';
