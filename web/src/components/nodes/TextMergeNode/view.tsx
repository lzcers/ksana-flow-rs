import { Position, type NodeProps } from '@xyflow/react';
import { NodeWrapper } from '../NodeWrapper';
import { type NodeData } from '../../../model/types';
import { textMergeNodeStyles } from './styles';

const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];
const TARGET_HANDLES = [Position.Left];

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
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={180}
      minHeight={150}
      style={{ width, height }}
    >
      <div className={textMergeNodeStyles.container}>
        <div className={textMergeNodeStyles.title}>Separator</div>
        <input
          className={textMergeNodeStyles.input}
          value={separator}
          onChange={(e) => onSeparatorChange(e.target.value)}
          onBlur={onSeparatorBlur}
          placeholder="Separator"
          onKeyDown={(e) => e.stopPropagation()}
          onMouseDown={(e) => e.stopPropagation()}
        />
        <div className={textMergeNodeStyles.hint}>Inputs are merged in alphabetical order of source node IDs.</div>
      </div>
    </NodeWrapper>
  );
}
