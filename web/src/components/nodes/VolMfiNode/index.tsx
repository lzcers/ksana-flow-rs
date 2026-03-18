import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useNumericStringNodeConfigField } from '../shared/hooks/useNodeConfigValueField';
import { VolMfiNodeView } from './view';

export const VolMfiNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const emaPeriodField = useNumericStringNodeConfigField({
    id,
    config: data.config,
    configKey: 'ema_period',
    defaultValue: '20',
    parse: (next) => {
      const n = parseInt(next, 10);
      return Number.isFinite(n) ? n : undefined;
    },
  });
  const mfiPeriodField = useNumericStringNodeConfigField({
    id,
    config: data.config,
    configKey: 'mfi_period',
    defaultValue: '14',
    parse: (next) => {
      const n = parseInt(next, 10);
      return Number.isFinite(n) ? n : undefined;
    },
  });

  return (
    <VolMfiNodeView
      {...props}
      emaPeriod={emaPeriodField.draft}
      mfiPeriod={mfiPeriodField.draft}
      onEmaPeriodChange={emaPeriodField.onChange}
      onMfiPeriodChange={mfiPeriodField.onChange}
    />
  );
});

VolMfiNode.displayName = 'VolMfiNode';
