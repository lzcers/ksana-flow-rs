import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useStringNodeConfigField } from '../shared/hooks/useNodeConfigValueField';
import { EmailNotifyNodeView } from './view';

export const EmailNotifyNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const subjectField = useStringNodeConfigField({ id, config: data.config, configKey: 'subject' });
  const bodyField = useStringNodeConfigField({
    id,
    config: data.config,
    configKey: 'body',
    composition: true,
  });

  return (
    <EmailNotifyNodeView
      {...props}
      subject={subjectField.draft}
      body={bodyField.draft}
      onSubjectChange={subjectField.onChange}
      onBodyChange={bodyField.onChange}
      onBodyCompositionStart={bodyField.onCompositionStart}
      onBodyCompositionEnd={bodyField.onCompositionEnd}
    />
  );
});

EmailNotifyNode.displayName = 'EmailNotifyNode';
