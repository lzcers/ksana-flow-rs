import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/types';
import { useShortVideoNodeController } from './hooks';
import { ShortVideoNodeView } from './view';

export const ShortVideoNode = memo((props: NodeProps & { data: NodeData }) => {
  const controller = useShortVideoNodeController(props.id, props.data);
  return (
    <ShortVideoNodeView
      {...props}
      isFullScreen={controller.isFullScreen}
      setIsFullScreen={(next) => controller.setIsFullScreen(next)}
      projectData={controller.projectData}
      isNodeCompleted={controller.isNodeCompleted}
      onProjectDataChange={controller.onProjectDataChange}
    />
  );
});

ShortVideoNode.displayName = 'ShortVideoNode';
