import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

export function SourceNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  code,
  startTime,
  endTime,
  product,
  onCodeChange,
  onStartTimeChange,
  onEndTimeChange,
  onProductChange,
}: NodeProps & {
  data: NodeData;
} & {
  code: string;
  startTime: string;
  endTime: string;
  product: string;
  onCodeChange: (next: string) => void;
  onStartTimeChange: (next: string) => void;
  onEndTimeChange: (next: string) => void;
  onProductChange: (next: string) => void;
}) {
  return (
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={280}
      minHeight={140}
      groups={[
        {
          layout: 'grid2',
          fields: [
            {
              kind: 'input',
              label: 'Code',
              value: code,
              onChange: onCodeChange,
              placeholder: 'e.g. 399300.SZ',
            },
            {
              kind: 'select',
              label: 'Product',
              value: product,
              onChange: onProductChange,
              options: [
                { value: 'STOCK', label: 'Stock' },
                { value: 'FUND', label: 'Fund' },
                { value: 'INDEX', label: 'Index' },
              ],
            },
          ],
        },
        {
          layout: 'grid2',
          fields: [
            {
              kind: 'input',
              label: 'Start Time',
              value: startTime,
              onChange: onStartTimeChange,
              placeholder: 'YYYYMMDD',
              controlVariant: 'plain',
            },
            {
              kind: 'input',
              label: 'End Time',
              value: endTime,
              onChange: onEndTimeChange,
              placeholder: 'Optional',
            },
          ],
        },
      ]}
    />
  );
}
