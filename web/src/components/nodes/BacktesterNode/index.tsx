import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/types';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import { useNodeConfigField } from '../shared/hooks/useNodeConfigField';
import { BacktesterNodeView } from './view';

export const BacktesterNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const { updateConfig } = useNodeConfig(id, data.config);

  const initMoneyField = useNodeConfigField<string>({
    value: String(data.config?.init_money ?? 100000.0),
    commitMode: 'change',
    updateValue: (next) => {
      const n = parseFloat(next);
      if (Number.isFinite(n)) updateConfig({ init_money: n });
    },
  });

  const feeRateField = useNodeConfigField<string>({
    value: String(data.config?.fee_rate ?? 0.0003),
    commitMode: 'change',
    updateValue: (next) => {
      const n = parseFloat(next);
      if (Number.isFinite(n)) updateConfig({ fee_rate: n });
    },
  });

  return (
    <BacktesterNodeView
      {...props}
      initMoney={initMoneyField.draft}
      feeRate={feeRateField.draft}
      onInitMoneyChange={initMoneyField.onChange}
      onFeeRateChange={feeRateField.onChange}
    />
  );
});

BacktesterNode.displayName = 'BacktesterNode';
