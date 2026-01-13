import React, { memo } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { WorkflowNodeData } from '../../types/workflow';
import { NodeWrapper } from './NodeWrapper';
import { useWorkflowContext } from '../../contexts/WorkflowContext';

export const LLMNode = memo(({ id, data, selected }: NodeProps & { data: WorkflowNodeData }) => {
  const { updateNodeData } = useWorkflowContext();

  const handleConfigChange = (key: string, value: string) => {
    updateNodeData(id, {
      config: {
        ...data.config,
        [key]: value
      }
    });
  };

  return (
    <NodeWrapper data={data} selected={selected}>
      <div className="px-3 pb-3 space-y-2 border-t border-slate-50 pt-2">
        <div>
          <label className="text-[10px] text-slate-400 uppercase font-bold block mb-1">System Prompt</label>
          <textarea
            className="w-full text-[10px] p-1.5 bg-slate-50 border-none rounded focus:ring-1 focus:ring-purple-200 resize-none outline-none nodrag"
            rows={2}
            value={data.config?.system_prompt || ''}
            onChange={(e) => handleConfigChange('system_prompt', e.target.value)}
            placeholder="System prompt..."
          />
        </div>
        <div>
          <label className="text-[10px] text-slate-400 uppercase font-bold block mb-1">User Prompt</label>
          <textarea
            className="w-full text-[10px] p-1.5 bg-slate-50 border-none rounded focus:ring-1 focus:ring-purple-200 resize-none outline-none nodrag"
            rows={2}
            value={data.config?.user_prompt_template || ''}
            onChange={(e) => handleConfigChange('user_prompt_template', e.target.value)}
            placeholder="User prompt template..."
          />
        </div>
      </div>
    </NodeWrapper>
  );
});

LLMNode.displayName = 'LLMNode';
