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
    toggleMarkdown,
    isFullScreen,
    setIsFullScreen,
    incremark,
    onBlur,
  } = useTextNode(id, data);
  return (
    <TextNodeView
      {...props}
      text={text}
      isMarkdown={isMarkdown}
      isFullScreen={isFullScreen}
      onToggleMarkdown={toggleMarkdown}
      onOpenFullScreen={() => setIsFullScreen(true)}
      onCloseFullScreen={() => setIsFullScreen(false)}
      incremark={incremark}
      onTextChange={onChange}
      onTextBlur={onBlur}
    />
  );
});
