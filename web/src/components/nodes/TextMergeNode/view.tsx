import { type NodeProps } from '@xyflow/react';
import { NodeWrapper } from '../shared/NodeWrapper';
import { type NodeData } from '@/model/workflow/types';
import { textMergeNodeStyles } from './styles';

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
