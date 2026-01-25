import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../model/types';
import { LLMNode } from '../nodes/LLMNode';
import { TextNode } from '../nodes/TextNode';
import { TextMergeNode } from '../nodes/TextMergeNode';
import { TextFileNode } from '../nodes/TextFileNode';
import { EmailNotifyNode } from '../nodes/EmailNotifyNode';
import { TimerNode } from '../nodes/TimerNode';
import { BacktesterNode } from '../nodes/BacktesterNode';
import { SourceNode } from '../nodes/SourceNode';
import { VolMfiNode } from '../nodes/VolMfiNode';
import { ShortVideoNode } from '../nodes/ShortVideoNode';
import { ImgGenNode } from '../nodes/ImgGenNode';

export const WorkflowNode = memo((props: NodeProps & { data: NodeData }) => {
  const { type } = props;

  switch (type) {
    case 'LLMNode':
      return <LLMNode {...props} />;
    case 'TextNode':
      return <TextNode {...props} />;
    case 'TextMergeNode':
      return <TextMergeNode {...props} />;
    case 'TextFileNode':
      return <TextFileNode {...props} />;
    case 'EmailNotifyNode':
      return <EmailNotifyNode {...props} />;
    case 'TimerNode':
      return <TimerNode {...props} />;
    case 'Backtester':
      return <BacktesterNode {...props} />;
    case 'ReactiveSourceNode':
      return <SourceNode {...props} />;
    case 'VOLMFINode':
      return <VolMfiNode {...props} />;
    case 'ShortVideoScriptNode':
      return <ShortVideoNode {...props} />;
    case 'ImgGenNode':
      return <ImgGenNode {...props} />;
    default:
      return null;
  }
});

WorkflowNode.displayName = 'WorkflowNode';
