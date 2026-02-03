import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import '@incremark/theme/styles.css';
import { useTextNodeController } from './hooks';
import { TextNodeView } from './view';

export const TextNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const controller = useTextNodeController(id, data);

  return (
    <TextNodeView
      {...props}
      text={controller.text}
      isMarkdown={controller.isMarkdown}
      isFullScreen={controller.isFullScreen}
      onToggleMarkdown={() => controller.setIsMarkdown((v) => !v)}
      onOpenFullScreen={() => controller.setIsFullScreen(true)}
      onCloseFullScreen={() => controller.setIsFullScreen(false)}
      incremark={controller.incremark}
      onTextChange={controller.onChange}
      onTextBlur={controller.onBlur}
    />
  );
});
