import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { WorkflowNodeData } from '../../types/workflow';
import { NodeWrapper } from './NodeWrapper';

export const ConditionNode = memo(({ id, data, selected }: NodeProps & { data: WorkflowNodeData }) => {
  return (
    <NodeWrapper 
      id={id}
      data={data} 
      selected={selected}
    />
  );
});

ConditionNode.displayName = 'ConditionNode';
