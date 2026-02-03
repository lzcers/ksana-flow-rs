import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { useNodeConfigField } from '../shared/hooks/useNodeConfigField';
import { EmailNotifyNodeView } from './view';

export const EmailNotifyNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { updateConfig } = useNodeConfig(id, data.config);

  const subjectField = useNodeConfigField<string>({
    value: String(data.config?.subject ?? ''),
    commitMode: 'change',
    updateValue: (next) => updateConfig({ subject: next }),
  });

  const bodyField = useNodeConfigField<string>({
    value: String(data.config?.body ?? ''),
    commitMode: 'change',
    composition: true,
    updateValue: (next) => updateConfig({ body: next }),
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
