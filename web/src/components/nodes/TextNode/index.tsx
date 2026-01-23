import React, { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Position, type NodeProps, useNodeConnections } from '@xyflow/react';
import { AutoScrollContainer, Incremark, ThemeProvider, useIncremark } from '@incremark/react';
import { Eye, Pencil, Maximize2 } from 'lucide-react';
import { NodeWrapper } from '../NodeWrapper';
import { FullScreenModal } from '../../ui/FullScreenModal';
import { useStore } from '../../../store';
import { type NodeData } from '../../../model/types';
import { theme } from './theme';
import '@incremark/theme/styles.css';
import '../index.css';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];

export const TextNodeComponent = ({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData, currentRunId, events$ } = useStore();
  const [text, setText] = useState<string>(data.config?.text || '');
  const [isMarkdown, setIsMarkdown] = useState(false);
  const [isFullScreen, setIsFullScreen] = useState(false);
  const scrollRef = useRef(null)

  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
  });

  const connections = useNodeConnections({
    handleType: 'target',
  });

  const textRef = useRef(text);
  const isStreamingRef = useRef(data.upstreamIsStreaming || false);

  // Use refs for data and connections to avoid dependency cycles in effects
  const dataRef = useRef(data);
  const connectionsRef = useRef(connections);

  useEffect(() => {
    dataRef.current = data;
    connectionsRef.current = connections;
  }, [data, connections]);

  useEffect(() => {
    textRef.current = text;
  }, [text]);

  useEffect(() => {
    isStreamingRef.current = data.upstreamIsStreaming || false;
  }, [data.upstreamIsStreaming]);

  useEffect(() => {
    // We access dataRef.current to avoid re-triggering this effect when data.config changes
    // This prevents the "Maximum update depth exceeded" error caused by circular updates
    const currentData = dataRef.current;

    if (currentData.upstreamIsStreaming && text !== currentData.config?.text) {
      const timeoutId = setTimeout(() => {
        // Double check ref in timeout
        const freshData = dataRef.current;
        updateNodeData(id, { config: { ...freshData.config, text } });
      }, 200);
      return () => clearTimeout(timeoutId);
    }
  }, [text, id, updateNodeData]); // Removed data.config, data.upstreamIsStreaming from deps

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

      const currentConnections = connectionsRef.current;
      const currentData = dataRef.current;

      const upstreamNodeIds = currentConnections.map(conn => conn.source);
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
          updateNodeData(id, { config: { ...currentData.config, text: '' } });
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
            updateNodeData(id, { config: { ...currentData.config, text: value } });
            if (isMarkdown) {
              incremark.render(value);
            }
          }
        }
      }
    });

    return () => subscription.unsubscribe();
  }, [events$, incremark, currentRunId, id, updateNodeData, isMarkdown]);
  // Removed connections and data.config from dependencies

  const onChange = useCallback((evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    setText(evt.target.value);
  }, []);

  const onBlur = useCallback(() => {
    const currentData = dataRef.current;
    if (text !== currentData.config?.text) {
      updateNodeData(id, {
        config: { ...currentData.config, text }
      });
    }
  }, [id, text, updateNodeData]); // Removed data.config

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
      type={type}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className="flex flex-col"
      minWidth={260}
      minHeight={200}
      style={{ width, height }}
      headerActions={headerActions}
    >
      <div className="p-2 h-full flex flex-col">
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
            <div className="w-full h-full flex p-4 text-zinc-200 text-sm overflow-auto custom-scrollbar bg-black justify-center items-center">
              {isMarkdown ? (
                <ThemeProvider theme={theme}>
                  <AutoScrollContainer enabled={data.upstreamIsStreaming} className="h-full w-full">
                    <Incremark incremark={incremark} />
                  </AutoScrollContainer>
                </ThemeProvider>
              ) : (
                <textarea
                  className="w-full h-full p-4 bg-black resize-none focus:outline-none text-zinc-200 font-mono"
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
            className="w-full flex-1 text-xs bg-black border border-zinc-800 rounded overflow-auto overflow-x-hidden nodrag nowheel text-zinc-200 custom-scrollbar"
            onKeyDown={(e) => e.stopPropagation()}
            onWheel={(e) => {
              e.stopPropagation();
            }}
          >
            <ThemeProvider theme={theme}>
              <AutoScrollContainer ref={scrollRef} enabled className="h-[300px]">
                <Incremark incremark={incremark} />
              </AutoScrollContainer>
            </ThemeProvider>
          </div>
        ) : (
          <textarea
            className="w-full flex-1 p-2 text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded resize-none nodrag nowheel focus:outline-none focus:ring-1 focus:ring-blue-500/50 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-600"
            value={text}
            onChange={onChange}
            onBlur={onBlur}
            placeholder="Enter text here..."
            onWheel={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
          />
        )}
      </div>
    </NodeWrapper >
  );
};

export const TextNode = memo(TextNodeComponent);
