import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import '@incremark/theme/styles.css';
import { useLLMNodeController } from './hooks';
import { LLMNodeView } from './view';

export const LLMNode = memo((props: NodeProps & { data: NodeData }) => {
  const controller = useLLMNodeController(props.id, props.data);
  return <LLMNodeView {...props} {...controller} />;
});

LLMNode.displayName = 'LLMNode';
