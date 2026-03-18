import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import { type NodeData } from '@/model/workflow/types';
import { useStringNodeConfigField } from '../shared/hooks/useNodeConfigValueField';
import { TextMergeNodeView } from './view';

export const TextMergeNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const separatorField = useStringNodeConfigField({
    id,
    config: data.config,
    configKey: 'separator',
    defaultValue: '\n',
    commitMode: 'blur',
  });

  return (
    <TextMergeNodeView
      {...props}
      separator={separatorField.draft}
      onSeparatorChange={separatorField.onChange}
      onSeparatorBlur={separatorField.onBlur}
    />
  );
});

TextMergeNode.displayName = 'TextMergeNode';
