import { memo, useCallback, useEffect, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';

const TARGET_HANDLES = [Position.Left];
const SOURCE_HANDLES = [Position.Right];

export const VolMfiNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();

  const [emaPeriod, setEmaPeriod] = useState(data.config?.ema_period ?? 20);
  const [mfiPeriod, setMfiPeriod] = useState(data.config?.mfi_period ?? 14);

  useEffect(() => {
    setEmaPeriod(data.config?.ema_period ?? 20);
  }, [data.config?.ema_period]);

  useEffect(() => {
    setMfiPeriod(data.config?.mfi_period ?? 14);
  }, [data.config?.mfi_period]);

  const handleEmaPeriodChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseInt(e.target.value, 10);
    setEmaPeriod(newValue);
    updateNodeData(id, {
      config: { ...data.config, ema_period: newValue }
    });
  }, [id, data.config, updateNodeData]);

  const handleMfiPeriodChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseInt(e.target.value, 10);
    setMfiPeriod(newValue);
    updateNodeData(id, {
      config: { ...data.config, mfi_period: newValue }
    });
  }, [id, data.config, updateNodeData]);

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
      <div className="px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2">
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">EMA Period</label>
          <input
            type="number"
            className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
            value={emaPeriod}
            onChange={handleEmaPeriodChange}
            onKeyDown={(e) => e.stopPropagation()}
            min="1"
          />
        </div>
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">MFI Period</label>
          <input
            type="number"
            className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
            value={mfiPeriod}
            onChange={handleMfiPeriodChange}
            onKeyDown={(e) => e.stopPropagation()}
            min="1"
          />
        </div>
      </div>
    </NodeWrapper>
  );
});

VolMfiNode.displayName = 'VolMfiNode';
