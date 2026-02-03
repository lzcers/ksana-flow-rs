import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { useNodeConfigField } from '../shared/hooks/useNodeConfigField';
import { VolMfiNodeView } from './view';

export const VolMfiNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { updateConfig } = useNodeConfig(id, data.config);

  const emaPeriodField = useNodeConfigField<string>({
    value: String(data.config?.ema_period ?? 20),
    commitMode: 'change',
    updateValue: (next) => {
      const n = parseInt(next, 10);
      if (Number.isFinite(n)) updateConfig({ ema_period: n });
    },
  });

  const mfiPeriodField = useNodeConfigField<string>({
    value: String(data.config?.mfi_period ?? 14),
    commitMode: 'change',
    updateValue: (next) => {
      const n = parseInt(next, 10);
      if (Number.isFinite(n)) updateConfig({ mfi_period: n });
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
