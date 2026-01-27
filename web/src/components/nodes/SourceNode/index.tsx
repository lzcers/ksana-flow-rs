import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../../model/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { useNodeConfigField } from '../shared/hooks/useNodeConfigField';
import { SourceNodeView } from './view';

export const SourceNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { updateConfig } = useNodeConfig(id, data.config);

  const codeField = useNodeConfigField<string>({
    value: String(data.config?.code ?? ''),
    commitMode: 'change',
    updateValue: (next) => updateConfig({ code: next }),
  });
  const startTimeField = useNodeConfigField<string>({
    value: String(data.config?.start_time ?? ''),
    commitMode: 'change',
    updateValue: (next) => updateConfig({ start_time: next }),
  });
  const endTimeField = useNodeConfigField<string>({
    value: String(data.config?.end_time ?? ''),
    commitMode: 'change',
    updateValue: (next) => updateConfig({ end_time: next }),
  });
  const productField = useNodeConfigField<string>({
    value: String(data.config?.product ?? 'FUND'),
    commitMode: 'change',
    updateValue: (next) => updateConfig({ product: next }),
  });

  return (
    <SourceNodeView
      {...props}
      code={codeField.draft}
      startTime={startTimeField.draft}
      endTime={endTimeField.draft}
      product={productField.draft}
      onCodeChange={codeField.onChange}
      onStartTimeChange={startTimeField.onChange}
      onEndTimeChange={endTimeField.onChange}
      onProductChange={productField.onChange}
    />
  );
});

SourceNode.displayName = 'SourceNode';
