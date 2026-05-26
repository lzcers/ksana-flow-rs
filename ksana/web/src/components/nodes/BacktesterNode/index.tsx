import { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { useNumericStringNodeConfigField } from '../shared/hooks/useNodeConfigValueField';
import { BacktesterNodeView } from './view';

export const BacktesterNode = memo((props: NodeProps & { data: NodeData }) => {
  const { id, data } = props;
  const initMoneyField = useNumericStringNodeConfigField({
    id,
    config: data.config,
    configKey: 'init_money',
    defaultValue: '100000',
    parse: (next) => {
      const n = parseFloat(next);
      return Number.isFinite(n) ? n : undefined;
    },
  });
  const feeRateField = useNumericStringNodeConfigField({
    id,
    config: data.config,
    configKey: 'fee_rate',
    defaultValue: '0.0003',
    parse: (next) => {
      const n = parseFloat(next);
      return Number.isFinite(n) ? n : undefined;
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
