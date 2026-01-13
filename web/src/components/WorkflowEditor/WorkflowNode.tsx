import React, { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { WorkflowNodeData } from '../../types/workflow';
import { StartNode } from '../nodes/StartNode';
import { EndNode } from '../nodes/EndNode';
import { TaskNode } from '../nodes/TaskNode';
import { ConditionNode } from '../nodes/ConditionNode';
import { LLMNode } from '../nodes/LLMNode';

export const WorkflowNode = memo((props: NodeProps & { data: WorkflowNodeData }) => {
  const { data } = props;

  switch (data.type) {
    case 'start':
      return <StartNode {...props} />;
    case 'end':
      return <EndNode {...props} />;
    case 'task':
      return <TaskNode {...props} />;
    case 'condition':
      return <ConditionNode {...props} />;
    case 'LLMNode':
      return <LLMNode {...props} />;
    default:
      return <TaskNode {...props} />;
  }
});

WorkflowNode.displayName = 'WorkflowNode';
