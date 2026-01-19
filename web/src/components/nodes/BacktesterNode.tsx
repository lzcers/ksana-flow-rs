import { memo, useCallback, useEffect, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';

const TARGET_HANDLES = [Position.Left];
const SOURCE_HANDLES = [Position.Right];

export const BacktesterNode = memo(({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();

  const [initMoney, setInitMoney] = useState(data.config?.init_money ?? 100000.0);
  const [feeRate, setFeeRate] = useState(data.config?.fee_rate ?? 0.0003);

  useEffect(() => {
    setInitMoney(data.config?.init_money ?? 100000.0);
  }, [data.config?.init_money]);

  useEffect(() => {
    setFeeRate(data.config?.fee_rate ?? 0.0003);
  }, [data.config?.fee_rate]);

  const handleInitMoneyChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseFloat(e.target.value);
    setInitMoney(newValue);
    updateNodeData(id, {
      config: { ...data.config, init_money: newValue }
    });
  }, [id, data.config, updateNodeData]);

  const handleFeeRateChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseFloat(e.target.value);
    setFeeRate(newValue);
    updateNodeData(id, {
      config: { ...data.config, fee_rate: newValue }
    });
  }, [id, data.config, updateNodeData]);

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      style={{ width: width ?? 250, height: height ?? 'auto' }}
      targetHandles={TARGET_HANDLES}
      sourceHandles={SOURCE_HANDLES}
    >
      <div className="px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2">
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">Initial Money</label>
          <input
            type="number"
            className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
            value={initMoney}
            onChange={handleInitMoneyChange}
            onKeyDown={(e) => e.stopPropagation()}
            step="1000"
          />
        </div>
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">Fee Rate</label>
          <input
            type="number"
            className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
            value={feeRate}
            onChange={handleFeeRateChange}
            onKeyDown={(e) => e.stopPropagation()}
            step="0.0001"
          />
        </div>
      </div>
    </NodeWrapper>
  );
});

BacktesterNode.displayName = 'BacktesterNode';
