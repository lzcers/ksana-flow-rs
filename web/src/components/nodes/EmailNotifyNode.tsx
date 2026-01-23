import { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';

const TARGET_HANDLES = [Position.Left];

export const EmailNotifyNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData } = useStore();

  const [subject, setSubject] = useState(data.config?.subject || '');
  const [body, setBody] = useState(data.config?.body || '');

  const isComposingBody = useRef(false);

  // Sync local state with props when props change
  useEffect(() => {
    setSubject(data.config?.subject || '');
  }, [data.config?.subject]);

  useEffect(() => {
    if (!isComposingBody.current) {
      setBody(data.config?.body || '');
    }
  }, [data.config?.body]);

  const handleSubjectChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setSubject(newValue);
    updateNodeData(id, {
      config: { ...data.config, subject: newValue }
    });
  }, [id, data.config, updateNodeData]);

  const handleBodyChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setBody(newValue);
    if (!isComposingBody.current) {
      updateNodeData(id, {
        config: { ...data.config, body: newValue }
      });
    }
  }, [id, data.config, updateNodeData]);

  const handleCompositionStart = () => {
    isComposingBody.current = true;
  };

  const handleCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposingBody.current = false;
    const newValue = e.currentTarget.value;
    updateNodeData(id, {
      config: { ...data.config, body: newValue }
    });
  }, [id, data.config, updateNodeData]);

  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      minWidth={300}
      minHeight={180}
      style={{ width, height }}
      targetHandles={TARGET_HANDLES}
    >
      <div className="px-3 pb-3 space-y-2 border-t border-zinc-800 pt-2">
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">Subject</label>
          <input
            className="w-full text-[10px] p-1.5 bg-black border border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 outline-none nodrag text-zinc-300"
            value={subject}
            onChange={handleSubjectChange}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="Email subject..."
          />
        </div>
        <div>
          <label className="text-[10px] text-zinc-500 font-bold block mb-1">Body</label>
          <textarea
            className="w-full text-[10px] p-1.5 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded focus:ring-1 focus:ring-purple-500/50 resize-none outline-none nodrag text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600"
            rows={4}
            value={body}
            onChange={handleBodyChange}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="Email body..."
          />
        </div>
      </div>
    </NodeWrapper>
  );
});

EmailNotifyNode.displayName = 'EmailNotifyNode';
