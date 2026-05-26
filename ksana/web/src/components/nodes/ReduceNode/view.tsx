import { type NodeProps } from '@xyflow/react';
import { type NodeData } from '@/model/workflow/types';
import { FormNodeView } from '../shared/FormNodeView';
import { nodeFormStyles } from '../shared/formStyles';
import type { ReduceReducer } from './hooks';

const LABEL_BY_REDUCER: Record<ReduceReducer, string> = {
  sum: 'Sum',
  count: 'Count',
  max: 'Max',
  min: 'Min',
  concat: 'Concat',
  merge_array: 'Merge Array',
  merge_object_deep: 'Deep Merge Object',
};

export function ReduceNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  reducer,
  separator,
  onReducerChange,
  onSeparatorChange,
  onSeparatorBlur,
  outputPreview,
}: NodeProps & {
  data: NodeData;
} & {
  reducer: ReduceReducer;
  separator: string;
  onReducerChange: (next: ReduceReducer) => void;
  onSeparatorChange: (next: string) => void;
  onSeparatorBlur: () => void;
  outputPreview: string;
}) {
  return (
    <FormNodeView
      id={id}
      type={type}
      data={data}
      selected={selected}
      width={width}
      height={height}
      minWidth={300}
      minHeight={220}
      contentClassName="space-y-3"
      groups={[
        {
          fields: [
            {
              kind: 'select',
              label: 'Reducer',
              value: reducer,
              onChange: next => onReducerChange(next as ReduceReducer),
              options: Object.entries(LABEL_BY_REDUCER).map(([value, label]) => ({ value, label })),
              hint: '输入为数组；输出为单值。Merge Array 适用于数组拼接；Deep Merge Object 适用于对象深合并。',
              controlVariant: 'plain',
              controlClassName: 'text-xs',
            },
          ],
        },
        ...(reducer === 'concat'
          ? [
              {
                fields: [
                  {
                    kind: 'input' as const,
                    label: 'Separator',
                    value: separator,
                    onChange: onSeparatorChange,
                    onBlur: onSeparatorBlur,
                    placeholder: '\\n',
                    controlVariant: 'plain' as const,
                    controlClassName: 'text-xs',
                  },
                ],
              },
            ]
          : []),
      ]}
    >
      {outputPreview ? (
        <div className={nodeFormStyles.field}>
          <div className={nodeFormStyles.label}>Last Output</div>
          <div className="truncate text-[11px] text-zinc-400" title={outputPreview}>
            {outputPreview}
          </div>
        </div>
      ) : null}
    </FormNodeView>
  );
}
