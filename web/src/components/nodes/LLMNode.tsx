import { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';

const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];
const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];

export const LLMNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();

  const adjustHeight = (el: HTMLTextAreaElement) => {
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
    if (el.scrollHeight > el.clientHeight) {
      el.style.overflowY = 'auto';
    } else {
      el.style.overflowY = 'hidden';
    }
  };

  const systemInputRef = useRef<HTMLTextAreaElement>(null);
  const userInputRef = useRef<HTMLTextAreaElement>(null);

  const [systemPrompt, setSystemPrompt] = useState(data.config?.system_prompt || '');
  const [userPrompt, setUserPrompt] = useState(data.config?.user_prompt_template || '');
  const [model, setModel] = useState(data.config?.model || 'deepseek-chat');
  const [stream, setStream] = useState(data.config?.stream || false);

  const isComposingSystem = useRef(false);
  const isComposingUser = useRef(false);

  // Sync local state with props when props change (and not focused)
  useEffect(() => {
    if (document.activeElement !== systemInputRef.current) {
      setSystemPrompt(data.config?.system_prompt || '');
      // Adjust height if needed
      if (systemInputRef.current) {
        adjustHeight(systemInputRef.current);
      }
    }
  }, [data.config?.system_prompt]);

  useEffect(() => {
    if (document.activeElement !== userInputRef.current) {
      setUserPrompt(data.config?.user_prompt_template || '');
      // Adjust height if needed
      if (userInputRef.current) {
        adjustHeight(userInputRef.current);
      }
    }
  }, [data.config?.user_prompt_template]);

  useEffect(() => {
    setModel(data.config?.model || 'deepseek-chat');
    setStream(data.config?.stream || false);
  }, [data.config?.model, data.config?.stream]);

  const handleModelChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const newModel = e.target.value;
    setModel(newModel);
    updateNodeData(id, { config: { ...data.config, model: newModel } });
  }, [id, data.config, updateNodeData]);

  const handleStreamChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newStream = e.target.checked;
    setStream(newStream);
    updateNodeData(id, { config: { ...data.config, stream: newStream } });
  }, [id, data.config, updateNodeData]);

  // System Prompt Handlers
  const handleSystemPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setSystemPrompt(newValue);

    adjustHeight(e.target);

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

    adjustHeight(e.target);

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
      type={type}
      data={data}
      selected={selected}
      minWidth={300}
      minHeight={400}
      style={{ width, height }}
      targetHandles={TARGET_HANDLES}
      sourceHandles={SOURCE_HANDLES}
    >
      <div className="px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2">
        <div className="flex gap-2">
          <div className="flex-1">
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">Model</label>
            <select
              className="w-full text-xs bg-zinc-950 border border-zinc-800 rounded px-1 py-1 text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500"
              value={model}
              onChange={handleModelChange}
            >
              <option value="deepseek-chat">DeepSeek Chat</option>
              <option value="deepseek-reasoner">DeepSeek Reasoner</option>
              <option value="google/gemini-3-pro-preview">Gemini 3 Pro preview</option>
            </select>
          </div>
          <div>
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">Stream</label>
            <div className="flex items-center h-[26px]">
              <input
                type="checkbox"
                className="rounded border-zinc-800 bg-zinc-950 text-blue-500 focus:ring-blue-500 cursor-pointer"
                checked={stream}
                onChange={handleStreamChange}
              />
            </div>
          </div>
        </div>
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">System Prompt</label>
          <textarea
            style={{ boxSizing: 'content-box' }}
            ref={systemInputRef}
            className="w-full flex-1 box-sizing: content-box text-xs bg-zinc-950 border overflow-hidden max-h-[200px] border-zinc-800 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
            rows={5}
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
            style={{ boxSizing: 'content-box' }}
            ref={userInputRef}
            className="w-full flex-1 text-xs bg-zinc-950 border overflow-x-hidden over max-h-[200px] border-zinc-800 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
            rows={5}
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
