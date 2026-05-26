import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useSubgraphController } from './hooks';
import { SubgraphNodeView } from './view';

export const SubgraphNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const controller = useSubgraphController(id, data);

  return (
    <SubgraphNodeView
      {...props}
      expanded={controller.expanded}
      onToggle={controller.onToggle}
    />
  );
});
