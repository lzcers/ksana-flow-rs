import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

export function VolMfiNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  emaPeriod,
  mfiPeriod,
  onEmaPeriodChange,
  onMfiPeriodChange,
}: NodeProps & {
  data: NodeData;
} & {
  emaPeriod: string;
  mfiPeriod: string;
  onEmaPeriodChange: (next: string) => void;
  onMfiPeriodChange: (next: string) => void;
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
              label: 'EMA Period',
              value: emaPeriod,
              onChange: onEmaPeriodChange,
              inputType: 'number',
              min: '1',
            },
            {
              kind: 'input',
              label: 'MFI Period',
              value: mfiPeriod,
              onChange: onMfiPeriodChange,
              inputType: 'number',
              min: '1',
            },
          ],
        },
      ]}
    />
  );
}
