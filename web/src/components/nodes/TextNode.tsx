import React, { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Position, type NodeProps, useNodeConnections } from '@xyflow/react';
import { AutoScrollContainer, Incremark, ThemeProvider, useIncremark } from '@incremark/react';
import { Eye, Pencil, Maximize2 } from 'lucide-react';
import { NodeWrapper } from './NodeWrapper';
import { FullScreenModal } from '../ui/FullScreenModal';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';
import '@incremark/theme/styles.css';
import './index.css';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];

export const TextNodeComponent = ({ id, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData, currentRunId, events$ } = useStore();
  const [text, setText] = useState<string>(data.config?.text || '');
  const [isMarkdown, setIsMarkdown] = useState(false);
  const [isFullScreen, setIsFullScreen] = useState(false);
  const scrollRef = useRef(null)

  const incremark = useIncremark({
    math: { tex: true }
  });

  const connections = useNodeConnections({
    handleType: 'target',
  });

  const textRef = useRef(text);
  const isStreamingRef = useRef(data.upstreamIsStreaming || false);

  useEffect(() => {
    textRef.current = text;
  }, [text]);

  // Sync isStreamingRef with data prop on mount/update to handle remounts correctly
  useEffect(() => {
    isStreamingRef.current = data.upstreamIsStreaming || false;
  }, [data.upstreamIsStreaming]);

  useEffect(() => {
    if (data.upstreamIsStreaming && text !== data.config?.text) {
      const timeoutId = setTimeout(() => {
        updateNodeData(id, { config: { ...data.config, text } });
      }, 200);
      return () => clearTimeout(timeoutId);
    }
  }, [text, data.upstreamIsStreaming, id, data.config, updateNodeData]);

  useEffect(() => {
    if (!data.upstreamIsStreaming && isMarkdown && text !== incremark.markdown) {
      incremark.render(text);
    }
  }, [text, isMarkdown, data.upstreamIsStreaming, incremark]);

  useEffect(() => {
    if (!events$) return;

    const subscription = events$.subscribe((wrapper: any) => {
      const { event, runId } = wrapper;
      // Filter by runId if available to avoid stale events
      if (currentRunId && runId !== currentRunId) return;

      const upstreamNodeIds = connections.map(conn => conn.source);
      const isUpstream = (nodeId: string) => upstreamNodeIds.includes(nodeId);

      if (event.NodeStarted) {
        const nodeId = event.NodeStarted;
        if (isUpstream(nodeId)) {
          isStreamingRef.current = false;
        }
      } else if (event.NodeStreamStarted) {
        const nodeId = event.NodeStreamStarted;
        if (isUpstream(nodeId)) {
          isStreamingRef.current = true;
          setText('');
          setIsMarkdown(true);
          incremark.reset();
          // Clear data in store immediately
          updateNodeData(id, { config: { ...data.config, text: '' } });
        }
      } else if (event.NodeStreamNextMessage) {
        const [nodeId, value] = event.NodeStreamNextMessage;
        if (isUpstream(nodeId) && isStreamingRef.current) {
          if (typeof value === 'string') {
            incremark.append(value);
            setText(prev => prev + value);
          }
        }
      } else if (event.NodeOutMessage) {
        const [nodeId, value] = event.NodeOutMessage;
        if (isUpstream(nodeId) && !isStreamingRef.current) {
          if (typeof value === 'string') {
            setText(value);
            updateNodeData(id, { config: { ...data.config, text: value } });
            if (isMarkdown) {
              incremark.render(value);
            }
          }
        }
      }
    });

    return () => subscription.unsubscribe();
  }, [events$, connections, incremark, currentRunId, id, updateNodeData, data.config, isMarkdown]);

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
    <div className="flex items-center gap-1">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsFullScreen(true);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title="Full Screen"
      >
        <Maximize2 size={12} />
      </button>
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
    </div>
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

        {isFullScreen && (
          <FullScreenModal
            isOpen={isFullScreen}
            onClose={() => setIsFullScreen(false)}
            title={isMarkdown ? "Markdown Preview" : "Text Content"}
          >
            <div className="w-full h-full text-zinc-200 text-sm overflow-auto custom-scrollbar bg-zinc-950">
              {isMarkdown ? (
                <ThemeProvider theme="dark">
                  <AutoScrollContainer enabled={data.upstreamIsStreaming} className="h-full">
                    <Incremark incremark={incremark} />
                  </AutoScrollContainer>
                </ThemeProvider>
              ) : (
                <textarea
                  className="w-full h-full p-4 bg-zinc-950 resize-none focus:outline-none text-zinc-200 font-mono"
                  value={text}
                  onChange={onChange}
                  placeholder="Enter text here..."
                  spellCheck={false}
                />
              )}
            </div>
          </FullScreenModal>
        )}

        {isMarkdown ? (
          <div
            className="w-full flex-1 text-xs bg-zinc-950 border border-zinc-800 rounded overflow-auto overflow-x-hidden nodrag nowheel text-zinc-200 custom-scrollbar"
            onKeyDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            onWheel={(e) => {
              e.stopPropagation();
            }}
          >
            <ThemeProvider theme="dark">
              <AutoScrollContainer ref={scrollRef} enabled className="h-[300px]">
                <Incremark incremark={incremark} />
              </AutoScrollContainer>
            </ThemeProvider>
          </div>
        ) : (
          <textarea
            className="w-full flex-1 p-2 text-xs bg-zinc-950 border border-zinc-800 rounded resize-none nodrag nowheel focus:outline-none focus:ring-1 focus:ring-blue-500 text-zinc-200"
            value={text}
            onChange={onChange}
            onBlur={onBlur}
            placeholder="Enter text here..."
            onWheel={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
          />
        )}
      </div>
    </NodeWrapper >
  );
};

export const TextNode = memo(TextNodeComponent);
