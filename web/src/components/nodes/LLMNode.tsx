import { memo, useState, useEffect, useCallback } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { WorkflowNodeData } from '../../types/workflow';
import { NodeWrapper } from './NodeWrapper';
import { useWorkflowContext } from '../../contexts/WorkflowContext';

export const LLMNode = memo(({ id, data, selected }: NodeProps & { data: WorkflowNodeData }) => {
  const { updateNodeData } = useWorkflowContext();
  const [systemPrompt, setSystemPrompt] = useState(data.config?.system_prompt || '');
  const [userPrompt, setUserPrompt] = useState(data.config?.user_prompt_template || '');

  useEffect(() => {
    setSystemPrompt(data.config?.system_prompt || '');
  }, [data.config?.system_prompt]);

  useEffect(() => {
    setUserPrompt(data.config?.user_prompt_template || '');
  }, [data.config?.user_prompt_template]);

  const handleBlur = useCallback(() => {
    updateNodeData(id, {
      config: {
        ...data.config,
        system_prompt: systemPrompt,
        user_prompt_template: userPrompt
      }
    });
  }, [id, data.config, systemPrompt, userPrompt, updateNodeData]);

  return (
    <NodeWrapper id={id} data={data} selected={selected}>
      <div className="px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2">
        <div>
          <label className="text-[10px] text-zinc-500 uppercase font-bold block mb-1">System Prompt</label>
          <textarea
            className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 resize-none outline-none nodrag text-zinc-300"
            rows={2}
            value={systemPrompt}
            onChange={(e) => setSystemPrompt(e.target.value)}
            onBlur={handleBlur}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="System prompt..."
          />
        </div>
        <div>
          <label className="text-[10px] text-zinc-500 uppercase font-bold block mb-1">User Prompt</label>
          <textarea
            className="w-full text-[10px] p-1.5 bg-zinc-950 border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 resize-none outline-none nodrag text-zinc-300"
            rows={2}
            value={userPrompt}
            onChange={(e) => setUserPrompt(e.target.value)}
            onBlur={handleBlur}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="User prompt template..."
          />
        </div>
      </div>
    </NodeWrapper>
  );
});

LLMNode.displayName = 'LLMNode';
