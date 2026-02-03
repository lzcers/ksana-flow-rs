import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/types';
import { NodeWrapper } from '../shared/NodeWrapper';
import { timerNodeStyles } from './styles';

const SOURCE_HANDLES = [Position.Right];

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
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      minWidth={250}
      minHeight={120}
      sourceHandles={SOURCE_HANDLES}
      style={{ width, height }}
    >
      <div className={timerNodeStyles.section}>
        <div>
          <label className={timerNodeStyles.label}>Cron Expression</label>
          <input
            className={timerNodeStyles.input}
            value={cronExpr}
            onChange={(e) => onCronChange(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="* * * * * * *"
          />
          <p className={timerNodeStyles.hint}>sec min hour day month dow year</p>
        </div>
      </div>
    </NodeWrapper>
  );
}
