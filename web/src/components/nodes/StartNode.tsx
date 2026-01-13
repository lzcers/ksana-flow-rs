import React, { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { WorkflowNodeData } from '../../types/workflow';
import { NodeWrapper } from './NodeWrapper';

export const StartNode = memo(({ data, selected }: NodeProps & { data: WorkflowNodeData }) => {
  return (
    <NodeWrapper 
      data={data} 
      selected={selected} 
      showTargetHandle={false}
    />
  );
});

StartNode.displayName = 'StartNode';
