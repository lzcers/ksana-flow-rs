import { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';

const TARGET_HANDLES = [Position.Left];
const SOURCE_HANDLES = [Position.Right];

export const LLMNode = memo(({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();

  const systemInputRef = useRef<HTMLTextAreaElement>(null);
  const userInputRef = useRef<HTMLTextAreaElement>(null);

  const [systemPrompt, setSystemPrompt] = useState(data.config?.system_prompt || '');
  const [userPrompt, setUserPrompt] = useState(data.config?.user_prompt_template || '');

  const isComposingSystem = useRef(false);
  const isComposingUser = useRef(false);

  // Sync local state with props when props change (and not focused)
  useEffect(() => {
    if (document.activeElement !== systemInputRef.current) {
      setSystemPrompt(data.config?.system_prompt || '');
      // Adjust height if needed
      if (systemInputRef.current) {
        systemInputRef.current.style.height = 'auto';
        systemInputRef.current.style.height = `${systemInputRef.current.scrollHeight}px`;
      }
    }
  }, [data.config?.system_prompt]);

  useEffect(() => {
    if (document.activeElement !== userInputRef.current) {
      setUserPrompt(data.config?.user_prompt_template || '');
      // Adjust height if needed
      if (userInputRef.current) {
        userInputRef.current.style.height = 'auto';
        userInputRef.current.style.height = `${userInputRef.current.scrollHeight}px`;
      }
    }
  }, [data.config?.user_prompt_template]);

  // System Prompt Handlers
  const handleSystemPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setSystemPrompt(newValue);

    e.target.style.height = 'auto';
    e.target.style.height = `${e.target.scrollHeight}px`;

    if (!isComposingSystem.current) {
      updateNodeData(id, {
        config: {
          ...data.config,
          system_prompt: newValue
        }
      });
    }
  }, [id, data.config, updateNodeData]);

  const handleSystemCompositionStart = () => {
    isComposingSystem.current = true;
  };

  const handleSystemCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposingSystem.current = false;
    const newValue = e.currentTarget.value;
    // Ensure we trigger an update when composition ends
    updateNodeData(id, {
      config: {
        ...data.config,
        system_prompt: newValue
      }
    });
  }, [id, data.config, updateNodeData]);

  // User Prompt Handlers
  const handleUserPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setUserPrompt(newValue);

    e.target.style.height = 'auto';
    e.target.style.height = `${e.target.scrollHeight}px`;

    if (!isComposingUser.current) {
      updateNodeData(id, {
        config: {
          ...data.config,
          user_prompt_template: newValue
        }
      });
    }
  }, [id, data.config, updateNodeData]);

  const handleUserCompositionStart = () => {
    isComposingUser.current = true;
  };

  const handleUserCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposingUser.current = false;
    const newValue = e.currentTarget.value;
    updateNodeData(id, {
      config: {
        ...data.config,
        user_prompt_template: newValue
      }
    });
  }, [id, data.config, updateNodeData]);

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      style={{ width: width ?? 300, height: height ?? 300 }}
      targetHandles={TARGET_HANDLES}
      sourceHandles={SOURCE_HANDLES}
    >
      <div className="px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2">
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">System Prompt</label>
          <textarea
            ref={systemInputRef}
            className="w-full flex-1 p-2 text-xs bg-zinc-950 border border-zinc-800 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
            rows={1}
            value={systemPrompt}
            onChange={handleSystemPromptChange}
            onCompositionStart={handleSystemCompositionStart}
            onCompositionEnd={handleSystemCompositionEnd}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="System prompt..."
          />
        </div>
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">User Prompt</label>
          <textarea
            ref={userInputRef}
            className="w-full flex-1 p-2 text-xs bg-zinc-950 border border-zinc-800 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
            rows={1}
            value={userPrompt}
            onChange={handleUserPromptChange}
            onCompositionStart={handleUserCompositionStart}
            onCompositionEnd={handleUserCompositionEnd}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="User prompt template..."
          />
        </div>
      </div>
    </NodeWrapper>
  );
});

LLMNode.displayName = 'LLMNode';
