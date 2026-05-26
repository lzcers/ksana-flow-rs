import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';

export function TimerNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  cronExpr,
  onCronChange,
}: NodeProps & { data: NodeData } & { cronExpr: string; onCronChange: (next: string) => void }) {
  return (
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={250}
      minHeight={120}
      groups={[
        {
          fields: [
            {
              kind: 'input',
              label: 'Cron Expression',
              value: cronExpr,
              onChange: onCronChange,
              placeholder: '* * * * * * *',
              controlClassName: 'font-mono',
              hint: 'sec min hour day month dow year',
              hintClassName: 'mt-1 text-[9px] text-zinc-600',
            },
          ],
        },
      ]}
    />
  );
}
