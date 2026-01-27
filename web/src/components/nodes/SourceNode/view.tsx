import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../../model/types';
import { NodeWrapper } from '../NodeWrapper';
import { sourceNodeStyles } from './styles';

const SOURCE_HANDLES = [Position.Right];

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
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      minWidth={280}
      minHeight={140}
      style={{ width, height }}
      sourceHandles={SOURCE_HANDLES}
    >
      <div className={sourceNodeStyles.section}>
        <div className={sourceNodeStyles.grid2}>
          <div>
            <label className={sourceNodeStyles.label}>Code</label>
            <input
              className={sourceNodeStyles.input}
              value={code}
              onChange={(e) => onCodeChange(e.target.value)}
              onKeyDown={(e) => e.stopPropagation()}
              placeholder="e.g. 399300.SZ"
            />
          </div>
          <div>
            <label className={sourceNodeStyles.label}>Product</label>
            <select
              className={sourceNodeStyles.select}
              value={product}
              onChange={(e) => onProductChange(e.target.value)}
              onKeyDown={(e) => e.stopPropagation()}
            >
              <option value="STOCK">Stock</option>
              <option value="FUND">Fund</option>
              <option value="INDEX">Index</option>
            </select>
          </div>
        </div>

        <div className={sourceNodeStyles.grid2}>
          <div>
            <label className={sourceNodeStyles.label}>Start Time</label>
            <input
              className={sourceNodeStyles.inputPlain}
              value={startTime}
              onChange={(e) => onStartTimeChange(e.target.value)}
              onKeyDown={(e) => e.stopPropagation()}
              placeholder="YYYYMMDD"
            />
          </div>
          <div>
            <label className={sourceNodeStyles.label}>End Time</label>
            <input
              className={sourceNodeStyles.input}
              value={endTime}
              onChange={(e) => onEndTimeChange(e.target.value)}
              onKeyDown={(e) => e.stopPropagation()}
              placeholder="Optional"
            />
          </div>
        </div>
      </div>
    </NodeWrapper>
  );
}
