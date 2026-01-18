import { memo, useCallback, useEffect, useState } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { WorkflowNodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useWorkflowContext } from '../../contexts/WorkflowContext';

export const TimerNode = memo(({ id, data, selected, width, height }: NodeProps & { data: WorkflowNodeData }) => {
  const { updateNodeData } = useWorkflowContext();

  const [cronExpr, setCronExpr] = useState(data.config?.cron_expr || '');

  useEffect(() => {
    setCronExpr(data.config?.cron_expr || '');
  }, [data.config?.cron_expr]);

  const handleCronChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setCronExpr(newValue);
    updateNodeData(id, {
      config: { ...data.config, cron_expr: newValue }
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
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">Cron Expression</label>
          <input
            className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300 font-mono"
            value={cronExpr}
            onChange={handleCronChange}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="* * * * * * *"
          />
          <p className="text-[9px] text-zinc-600 mt-1">sec min hour day month dow year</p>
        </div>
      </div>
    </NodeWrapper>
  );
});

TimerNode.displayName = 'TimerNode';
