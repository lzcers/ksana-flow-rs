import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useStringNodeConfigField } from '../shared/hooks/useNodeConfigValueField';
import { SourceNodeView } from './view';

export const SourceNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const codeField = useStringNodeConfigField({ id, config: data.config, configKey: 'code' });
  const startTimeField = useStringNodeConfigField({ id, config: data.config, configKey: 'start_time' });
  const endTimeField = useStringNodeConfigField({ id, config: data.config, configKey: 'end_time' });
  const productField = useStringNodeConfigField({ id, config: data.config, configKey: 'product', defaultValue: 'FUND' });

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
