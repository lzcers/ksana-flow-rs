import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../../model/types';
import { NodeWrapper } from '../NodeWrapper';
import { backtesterNodeStyles } from './styles';

const TARGET_HANDLES = [Position.Left];
const SOURCE_HANDLES = [Position.Right];

export function BacktesterNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  initMoney,
  feeRate,
  onInitMoneyChange,
  onFeeRateChange,
}: NodeProps & {
  data: NodeData;
} & {
  initMoney: string;
  feeRate: string;
  onInitMoneyChange: (next: string) => void;
  onFeeRateChange: (next: string) => void;
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
      <div className={backtesterNodeStyles.section}>
        <div>
          <label className={backtesterNodeStyles.label}>Initial Money</label>
          <input
            type="number"
            className={backtesterNodeStyles.initMoneyInput}
            value={initMoney}
            onChange={(e) => onInitMoneyChange(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
            step="1000"
          />
        </div>
        <div>
          <label className={backtesterNodeStyles.label}>Fee Rate</label>
          <input
            type="number"
            className={backtesterNodeStyles.feeRateInput}
            value={feeRate}
            onChange={(e) => onFeeRateChange(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
            step="0.0001"
          />
        </div>
      </div>
    </NodeWrapper>
  );
}
