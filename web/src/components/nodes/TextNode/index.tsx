import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import '@incremark/theme/styles.css';
import { useTextNode } from './hooks';
import { TextNodeView } from './view';

export const TextNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const {
    text,
    isMarkdown,
    onChange,
    onSave,
    toggleMarkdown,
    resetText,
  } = useTextNode(id, data);

  return (
    <TextNodeView
      {...props}
      text={text}
      isMarkdown={isMarkdown}
      isFullScreen={isFullScreen}
      onToggleMarkdown={toggleMarkdown}
      onOpenFullScreen={() => controller.setIsFullScreen(true)}
      onCloseFullScreen={() => controller.setIsFullScreen(false)}
      incremark={controller.incremark}
      onTextChange={controller.onChange}
      onTextBlur={controller.onBlur}
    />
  );
});
