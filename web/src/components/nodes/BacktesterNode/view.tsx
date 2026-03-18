import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

export function BacktesterNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  initMoney,
  feeRate,
  onInitMoneyChange,
  onFeeRateChange,
}: NodeProps & {
  data: NodeData;
} & {
  initMoney: string;
  feeRate: string;
  onInitMoneyChange: (next: string) => void;
  onFeeRateChange: (next: string) => void;
}) {
  return (
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={250}
      minHeight={140}
      groups={[
        {
          fields: [
            {
              kind: 'input',
              label: 'Initial Money',
              value: initMoney,
              onChange: onInitMoneyChange,
              inputType: 'number',
              step: '1000',
              controlVariant: 'plain',
            },
            {
              kind: 'input',
              label: 'Fee Rate',
              value: feeRate,
              onChange: onFeeRateChange,
              inputType: 'number',
              step: '0.0001',
            },
          ],
        },
      ]}
    />
  );
}
