import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useTextSplitNodeController } from './hooks';
import { TextSplitNodeView } from './view';

export const TextSplitNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const controller = useTextSplitNodeController(id, data);

  return <TextSplitNodeView {...props} {...controller} />;
});

TextSplitNode.displayName = 'TextSplitNode';
