import { type NodeProps } from '@xyflow/react';
import { type NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

export function TextMergeNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  separator,
  onSeparatorChange,
  onSeparatorBlur,
}: NodeProps & {
  data: NodeData;
} & {
  separator: string;
  onSeparatorChange: (next: string) => void;
  onSeparatorBlur: () => void;
}) {
  return (
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={180}
      minHeight={150}
      groups={[
        {
          fields: [
            {
              kind: 'input',
              label: 'Separator',
              value: separator,
              onChange: onSeparatorChange,
              onBlur: onSeparatorBlur,
              placeholder: 'Separator',
              hint: 'Inputs are merged in alphabetical order of source node IDs.',
            },
          ],
        },
      ]}
    />
  );
}
