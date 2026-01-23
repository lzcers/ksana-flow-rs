import { memo, useCallback, useEffect, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';

const SOURCE_HANDLES = [Position.Right];

export const SourceNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();

  const [code, setCode] = useState(data.config?.code || '');
  const [startTime, setStartTime] = useState(data.config?.start_time || '');
  const [endTime, setEndTime] = useState(data.config?.end_time || '');
  const [product, setProduct] = useState(data.config?.product || 'FUND');

  useEffect(() => {
    setCode(data.config?.code || '');
  }, [data.config?.code]);

  useEffect(() => {
    setStartTime(data.config?.start_time || '');
  }, [data.config?.start_time]);

  useEffect(() => {
    setEndTime(data.config?.end_time || '');
  }, [data.config?.end_time]);

  useEffect(() => {
    setProduct(data.config?.product || 'FUND');
  }, [data.config?.product]);

  const handleCodeChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setCode(newValue);
    updateNodeData(id, {
      config: { ...data.config, code: newValue }
    });
  }, [id, data.config, updateNodeData]);

  const handleStartTimeChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setStartTime(newValue);
    updateNodeData(id, {
      config: { ...data.config, start_time: newValue }
    });
  }, [id, data.config, updateNodeData]);

  const handleEndTimeChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setEndTime(newValue);
    updateNodeData(id, {
      config: { ...data.config, end_time: newValue }
    });
  }, [id, data.config, updateNodeData]);

  const handleProductChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const newValue = e.target.value;
    setProduct(newValue);
    updateNodeData(id, {
      config: { ...data.config, product: newValue }
    });
  }, [id, data.config, updateNodeData]);

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
      <div className="px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2">
        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">Code</label>
            <input
              className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
              value={code}
              onChange={handleCodeChange}
              onKeyDown={(e) => e.stopPropagation()}
              placeholder="e.g. 399300.SZ"
            />
          </div>
          <div>
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">Product</label>
            <select
              className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
              value={product}
              onChange={handleProductChange}
              onKeyDown={(e) => e.stopPropagation()}
            >
              <option value="STOCK">Stock</option>
              <option value="FUND">Fund</option>
              <option value="INDEX">Index</option>
            </select>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">Start Time</label>
            <input
              className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
              value={startTime}
              onChange={handleStartTimeChange}
              onKeyDown={(e) => e.stopPropagation()}
              placeholder="YYYYMMDD"
            />
          </div>
          <div>
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">End Time</label>
            <input
              className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
              value={endTime}
              onChange={handleEndTimeChange}
              onKeyDown={(e) => e.stopPropagation()}
              placeholder="Optional"
            />
          </div>
        </div>
      </div>
    </NodeWrapper>
  );
});

SourceNode.displayName = 'SourceNode';
