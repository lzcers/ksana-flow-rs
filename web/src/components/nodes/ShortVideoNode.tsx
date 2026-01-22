import { memo, useState, useEffect, useRef } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import { Maximize2, Clapperboard } from 'lucide-react';
import { parse } from 'jsonriver';
import { NodeWrapper } from './NodeWrapper';
import { FullScreenModal } from '../ui/FullScreenModal';
import { ShortVideoCreation } from '../ShortVideoCreation';
import type { ProjectData } from '../ShortVideoCreation/types';
import { useStore } from '../../store';
import { type NodeData } from '../../model/types';
import './index.css';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left];

export const ShortVideoNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData, events$, currentRunId } = useStore();
  const [isFullScreen, setIsFullScreen] = useState(false);
  const [projectData, setProjectData] = useState<ProjectData | null>(data.config?.projectData || null);

  const streamControllerRef = useRef<ReadableStreamDefaultController<string> | null>(null);
  const completedRef = useRef(new WeakMap<object, boolean>());
  const isStreamingRef = useRef(false);

  const isNodeCompleted = (value: any) => {
    if (value && typeof value === 'object') {
      return completedRef.current.has(value);
    }
    return false;
  };

  const startNewStream = () => {
    // Close existing stream if any
    if (streamControllerRef.current) {
      try { streamControllerRef.current.close(); } catch { }
      streamControllerRef.current = null;
    }
    completedRef.current = new WeakMap();

    const stream = new ReadableStream<string>({
      start(controller) {
        streamControllerRef.current = controller;
      }
    });

    (async () => {
      try {
        const parser = parse(stream as unknown as AsyncIterable<string>, {
          completeCallback: (value: any) => {
            if (value && typeof value === 'object') {
              completedRef.current.set(value, true);
            }
          }
        });
        for await (const value of parser) {
          if (value && typeof value === 'object') {
            const newData = value as unknown as ProjectData;
            setProjectData(newData);
            // Optional: Sync to store occasionally if needed, but for high-freq streaming 
            // we usually wait until end or use a debounce.
            // Here we just update local state for performance.
          }
        }
      } catch (e) {
        console.debug('JSON stream parsing ended', e);
      }
    })();
  };

  useEffect(() => {
    if (!events$) return;

    const subscription = events$.subscribe((wrapper: any) => {
      const { event, runId } = wrapper;
      // Filter by runId if available to avoid stale events
      if (currentRunId && runId !== currentRunId) return;

      if (event.NodeStarted) {
        if (event.NodeStarted === id) {
          isStreamingRef.current = false;
        }
      } else if (event.NodeStreamStarted) {
        if (event.NodeStreamStarted === id) {
          isStreamingRef.current = true;
          setProjectData(null);
          startNewStream();
        }
      } else if (event.NodeStreamNextMessage) {
        const [nodeId, value] = event.NodeStreamNextMessage;
        if (nodeId === id && isStreamingRef.current) {
          if (typeof value === 'string') {
            streamControllerRef.current?.enqueue(value);
          }
        }
      } else if (event.NodeOutMessage) {
        const [nodeId, value] = event.NodeOutMessage;
        if (nodeId === id) {
          // If we receive the final message, ensure we update the state
          // If we were streaming, the stream parser might have already finished or be about to finish.
          // But NodeOutMessage is the source of truth for the final state.

          if (!isStreamingRef.current) {
            // Non-streaming case: parse the full value
            try {
              const parsed = typeof value === 'string' ? JSON.parse(value) : value;
              setProjectData(parsed);
              updateNodeData(id, { config: { ...data.config, projectData: parsed } });
            } catch (e) {
              console.error('Failed to parse NodeOutMessage', e);
            }
          } else {
            // Streaming case finished. 
            // We can close the controller to ensure the parser loop finishes if it hasn't.
            if (streamControllerRef.current) {
              try { streamControllerRef.current.close(); } catch { }
              streamControllerRef.current = null;
            }
            // We might want to use the final value from NodeOutMessage to be safe,
            // or trust the stream parser. 
            // Usually NodeOutMessage has the complete valid JSON.
            try {
              const parsed = typeof value === 'string' ? JSON.parse(value) : value;
              setProjectData(parsed);
              updateNodeData(id, { config: { ...data.config, projectData: parsed } });
            } catch (e) {
              // If stream was successful, we might already have data.
            }
            isStreamingRef.current = false;
          }
        }
      }
    });

    return () => subscription.unsubscribe();
  }, [events$, currentRunId, id, updateNodeData, data.config]);

  // Handle initial data load (if page refreshed or loaded with existing data)
  useEffect(() => {
    if (data.config?.projectData) {
      setProjectData(data.config.projectData);
    } else if (data.lastMessage && !isStreamingRef.current && !projectData) {
      // Fallback to lastMessage if config is empty but lastMessage exists (legacy/migration)
      try {
        const parsed = typeof data.lastMessage === 'string' ? JSON.parse(data.lastMessage) : data.lastMessage;
        setProjectData(parsed);
      } catch (e) {
        // ignore
      }
    }
  }, []); // Run once on mount

  const headerActions = (
    <div className="flex items-center gap-1">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsFullScreen(true);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title="Open Editor"
      >
        <Maximize2 size={12} />
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
      minWidth={240}
      minHeight={200}
      style={{ width: width ?? 300, height: height ?? 300 }}
      headerActions={headerActions}
    >
      <div className="p-4 flex-1 flex flex-col items-center justify-center min-h-0 bg-zinc-950/50">
        <div className="text-center space-y-2">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center mx-auto shadow-lg shadow-indigo-500/20">
            <Clapperboard className="text-white" size={24} />
          </div>
          <div>
            <h3 className="text-sm font-medium text-zinc-200">AI Short Video Creator</h3>
            <p className="text-xs text-zinc-500 mt-1">
              {projectData ? 'Project ready' : 'Waiting for input...'}
            </p>
          </div>

          <button
            onClick={() => setIsFullScreen(true)}
            className="px-4 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs rounded-md transition-colors border border-zinc-700"
          >
            Open Studio
          </button>
        </div>

        {isFullScreen && (
          <FullScreenModal
            isOpen={isFullScreen}
            onClose={() => setIsFullScreen(false)}
            title="Short Video Studio"
            className="bg-white dark:bg-zinc-950"
          >
            <ShortVideoCreation
              data={projectData as ProjectData}
              onBack={() => setIsFullScreen(false)}
              isNodeCompleted={isNodeCompleted}
              onDataChange={(newData) => {
                setProjectData(newData);
                updateNodeData(id, {
                  config: { ...data.config, projectData: newData }
                });
              }}
            />
          </FullScreenModal>
        )}
      </div>
    </NodeWrapper>
  );
});

ShortVideoNode.displayName = 'ShortVideoNode';
