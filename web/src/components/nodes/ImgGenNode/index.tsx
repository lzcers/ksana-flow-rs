import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../../model/types';
import { useImgGenNodeController } from './hooks';
import { ImgGenNodeView } from './view';

export const ImgGenNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data, selected, width, height } = props;
  const controller = useImgGenNodeController({
    id,
    data,
    selected,
    width: typeof width === 'number' ? width : null,
    height: typeof height === 'number' ? height : null,
  });

  return <ImgGenNodeView {...props} {...controller} />;
});

ImgGenNode.displayName = 'ImgGenNode';
