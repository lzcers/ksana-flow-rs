import React, { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Handle, Position, type NodeProps, useNodeConnections } from '@xyflow/react';
import { AutoScrollContainer, IncremarkContent, ThemeProvider } from '@incremark/react';
import { Eye, Pencil } from 'lucide-react';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';
import '@incremark/theme/styles.css';
import './index.css';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];

export const TextNodeComponent = ({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData, currentRunId } = useStore();
  const [text, setText] = useState<string>(data.config?.text || '');
  const [isMarkdown, setIsMarkdown] = useState(false);
  const scrollRef = useRef(null)

  const connections = useNodeConnections({
    handleType: 'target',
  });

  const prevStatus = useRef(data.status);
  const textRef = useRef(text);
  const wasUpstreamStreaming = useRef(data.upstreamIsStreaming || false);
  const prevLastMessage = useRef(data.lastMessage);
  const justStreamed = useRef(false);

  useEffect(() => {
    textRef.current = text;
  }, [text]);

  useEffect(() => {
    if (data.status === 'running' && prevStatus.current !== 'running') {
      if (justStreamed.current) {
        justStreamed.current = false;
      } else {
        // If we have an upstream connection and the message is from the current run, don't clear
        const isNewMessage = connections.length > 0 &&
          data.lastMessageRunId === currentRunId &&
          data.lastMessage !== undefined;

        if (!isNewMessage) {
          setText('');
          updateNodeData(id, { config: { ...data.config, text: '' } });
        }
      }
    } else if (data.status === 'idle' || !data.status) {
      justStreamed.current = false;
    }
    prevStatus.current = data.status;
  }, [data.status, id, updateNodeData, data.config, connections.length, data.lastMessageRunId, currentRunId, data.lastMessage]);

  // Sync text to store during streaming (debounced)
  useEffect(() => {
    if (data.upstreamIsStreaming && text !== data.config?.text) {
      const timeoutId = setTimeout(() => {
        updateNodeData(id, { config: { ...data.config, text } });
      }, 200);
      return () => clearTimeout(timeoutId);
    }
  }, [text, data.upstreamIsStreaming, id, data.config, updateNodeData]);

  // Handle upstream streaming and messages
  useEffect(() => {
    const isStreaming = data.upstreamIsStreaming || false;
    const lastMessageChanged = data.lastMessage !== prevLastMessage.current;

    if (isStreaming && !wasUpstreamStreaming.current) {
      // Start streaming: switch to markdown and clear text
      setText('');
      updateNodeData(id, { config: { ...data.config, text: '' } });
    } else if (isStreaming && lastMessageChanged) {
      // Streaming: append new chunk
      if (typeof data.lastMessage === 'string') {
        setText(prev => prev + data.lastMessage);
      }
    } else if (!isStreaming && wasUpstreamStreaming.current) {
      // End streaming: ignore the final message (NodeOutMessage)
      justStreamed.current = true;
    } else if (!isStreaming && !wasUpstreamStreaming.current) {
      // Normal mode: update text if changed
      if (lastMessageChanged && typeof data.lastMessage === 'string' && connections.length > 0) {
        if (data.config?.text !== data.lastMessage) {
          setText(data.lastMessage);
          updateNodeData(id, { config: { ...data.config, text: data.lastMessage } });
        }
      }
    }

    wasUpstreamStreaming.current = isStreaming;
    prevLastMessage.current = data.lastMessage;
  }, [data.lastMessage, data.upstreamIsStreaming, id, updateNodeData, connections.length, data.config]);

  const onChange = useCallback((evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    setText(evt.target.value);
  }, []);

  const onBlur = useCallback(() => {
    if (text !== data.config?.text) {
      updateNodeData(id, {
        config: { ...data.config, text }
      });
    }
  }, [id, data.config, text, updateNodeData]);

  const headerActions = (
    <button
      onClick={(e) => {
        e.stopPropagation();
        setIsMarkdown(!isMarkdown);
      }}
      className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
      title={isMarkdown ? "Switch to Edit Mode" : "Switch to Markdown Preview"}
    >
      {isMarkdown ? <Pencil size={12} /> : <Eye size={12} />}
    </button>
  );

  return (
    <NodeWrapper
      id={id}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={200}
      minHeight={150}
      style={{ width: width ?? 240, height: height ?? 160 }}
      headerActions={headerActions}
    >
      <div className="p-2 flex-1 flex flex-col min-h-0">
        <div className="text-xs text-zinc-500 mb-1 flex items-center justify-between">
          <span>Text Content</span>
          <span className="text-[10px] opacity-50">{isMarkdown ? 'Markdown' : 'Raw'}</span>
        </div>

        {isMarkdown ? (
          <div
            className="w-full flex-1 text-xs bg-zinc-950 border border-zinc-800 rounded overflow-auto overflow-x-hidden nodrag text-zinc-200 custom-scrollbar"
            onKeyDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            onWheel={(e) => {
              e.stopPropagation();
            }}
          >
            <ThemeProvider theme="dark">
              <AutoScrollContainer ref={scrollRef} enabled className="h-[300px]">
                <IncremarkContent content={text} isFinished={true} incremarkOptions={{
                  math: { tex: true }
                }} />
              </AutoScrollContainer>
            </ThemeProvider>
          </div>
        ) : (
          <textarea
            className="w-full flex-1 p-2 text-xs bg-zinc-950 border border-zinc-800 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
            value={text}
            onChange={onChange}
            onBlur={onBlur}
            placeholder="Enter text here..."
            onKeyDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
          />
        )}
      </div>
    </NodeWrapper >
  );
};

export const TextNode = memo(TextNodeComponent);
