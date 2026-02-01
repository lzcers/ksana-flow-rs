import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../../model/types';
import { useReduceNodeController } from './hooks';
import { ReduceNodeView } from './view';

export const ReduceNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const controller = useReduceNodeController(id, data);
  return <ReduceNodeView {...props} {...controller} />;
});

ReduceNode.displayName = 'ReduceNode';

