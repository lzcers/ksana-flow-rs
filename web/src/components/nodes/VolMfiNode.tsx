import { memo, useCallback, useEffect, useState } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { WorkflowNodeData } from '../../types/workflow';
import { NodeWrapper } from './NodeWrapper';
import { useWorkflowContext } from '../../contexts/WorkflowContext';

export const VolMfiNode = memo(({ id, data, selected, width, height }: NodeProps & { data: WorkflowNodeData }) => {
  const { updateNodeData } = useWorkflowContext();

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
      data={data}
      selected={selected}
      style={{ width: width ?? 250, height: height ?? 'auto' }}
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
