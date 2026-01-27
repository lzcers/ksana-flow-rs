import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../../model/types';
import { NodeWrapper } from '../NodeWrapper';
import { volMfiNodeStyles } from './styles';

const TARGET_HANDLES = [Position.Left];
const SOURCE_HANDLES = [Position.Right];

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
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      minWidth={250}
      minHeight={140}
      style={{ width, height }}
      targetHandles={TARGET_HANDLES}
      sourceHandles={SOURCE_HANDLES}
    >
      <div className={volMfiNodeStyles.section}>
        <div>
          <label className={volMfiNodeStyles.label}>EMA Period</label>
          <input
            type="number"
            className={volMfiNodeStyles.input}
            value={emaPeriod}
            onChange={(e) => onEmaPeriodChange(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
            min="1"
          />
        </div>
        <div>
          <label className={volMfiNodeStyles.label}>MFI Period</label>
          <input
            type="number"
            className={volMfiNodeStyles.input}
            value={mfiPeriod}
            onChange={(e) => onMfiPeriodChange(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
            min="1"
          />
        </div>
      </div>
    </NodeWrapper>
  );
}
