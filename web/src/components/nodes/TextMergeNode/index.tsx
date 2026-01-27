import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import { type NodeData } from '../../../model/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { useNodeConfigField } from '../shared/hooks/useNodeConfigField';
import { TextMergeNodeView } from './view';

export const TextMergeNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { updateConfig } = useNodeConfig(id, data.config);

  const separatorField = useNodeConfigField<string>({
    value: String(data.config?.separator ?? '\n'),
    commitMode: 'blur',
    updateValue: (next) => updateConfig({ separator: next }),
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
