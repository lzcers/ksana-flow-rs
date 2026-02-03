import { Position, type NodeProps } from '@xyflow/react';
import { NodeWrapper } from '../shared/NodeWrapper';
import { type NodeData } from '@/model/types';
import { reduceNodeStyles } from './styles';
import type { ReduceReducer } from './hooks';

const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];
const TARGET_HANDLES = [Position.Left];

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
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={300}
      minHeight={220}
      style={{ width, height }}
    >
      <div className={reduceNodeStyles.container}>
        <div className={reduceNodeStyles.title}>Reduce</div>

        <div className="flex flex-col gap-1">
          <div className={reduceNodeStyles.fieldLabel}>Reducer</div>
          <select
            className={reduceNodeStyles.select}
            value={reducer}
            onChange={(e) => onReducerChange(e.target.value as ReduceReducer)}
            onKeyDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
          >
            {Object.entries(LABEL_BY_REDUCER).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
          <div className={reduceNodeStyles.hint}>
            输入为数组；输出为单值。Merge Array 适用于数组拼接；Deep Merge Object 适用于对象深合并。
          </div>
        </div>

        {reducer === 'concat' && (
          <div className="flex flex-col gap-1">
            <div className={reduceNodeStyles.fieldLabel}>Separator</div>
            <input
              className={reduceNodeStyles.input}
              value={separator}
              onChange={(e) => onSeparatorChange(e.target.value)}
              onBlur={onSeparatorBlur}
              placeholder="\\n"
              onKeyDown={(e) => e.stopPropagation()}
              onMouseDown={(e) => e.stopPropagation()}
            />
          </div>
        )}

        {outputPreview && (
          <div className="flex flex-col gap-1">
            <div className={reduceNodeStyles.fieldLabel}>Last Output</div>
            <div className={reduceNodeStyles.preview} title={outputPreview}>
              {outputPreview}
            </div>
          </div>
        )}
      </div>
    </NodeWrapper>
  );
}

