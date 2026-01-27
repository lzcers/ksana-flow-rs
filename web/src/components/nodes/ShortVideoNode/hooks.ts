import { useCallback, useEffect, useRef, useState } from 'react';
import { parse } from 'jsonriver';
import { useStore } from '../../../store';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '../../../model/types';
import type { ProjectData } from '../../ShortVideoCreation/types';

export function useShortVideoNodeController(id: string, data: NodeData) {
  const { eventsForNode$ } = useStore();
  const { updateConfig } = useNodeConfig(id, data.config);

  const [isFullScreen, setIsFullScreen] = useState(false);
  const [projectData, setProjectData] = useState<ProjectData | null>((data.config?.projectData as ProjectData) || null);

  const streamControllerRef = useRef<ReadableStreamDefaultController<string> | null>(null);
  const completedRef = useRef(new WeakMap<object, boolean>());
  const isStreamingRef = useRef(false);

  const isNodeCompleted = useCallback((value: any) => {
    if (value && typeof value === 'object') {
      return completedRef.current.has(value);
    }
    return false;
  }, []);

  const startNewStream = useCallback(() => {
    if (streamControllerRef.current) {
      try {
        streamControllerRef.current.close();
      } catch {}
      streamControllerRef.current = null;
    }
    completedRef.current = new WeakMap();

    const stream = new ReadableStream<string>({
      start(controller) {
        streamControllerRef.current = controller;
      },
    });

    (async () => {
      try {
        const parser = parse(stream as unknown as AsyncIterable<string>, {
          completeCallback: (value: any) => {
            if (value && typeof value === 'object') {
              completedRef.current.set(value, true);
            }
          },
        });
        for await (const value of parser) {
          if (value && typeof value === 'object') {
            setProjectData(value as unknown as ProjectData);
          }
        }
      } catch {}
    })();
  }, []);

  useEffect(() => {
    const stream$ = eventsForNode$?.(id);
    if (!stream$) return;

    const subscription = stream$.subscribe((wrapper: any) => {
      const { event } = wrapper;

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
          if (isStreamingRef.current) {
            if (streamControllerRef.current) {
              try {
                streamControllerRef.current.close();
              } catch {}
              streamControllerRef.current = null;
            }
          }
          try {
            const parsed = typeof value === 'string' ? JSON.parse(value) : value;
            setProjectData(parsed);
            updateConfig({ projectData: parsed } as any);
          } catch {}
          isStreamingRef.current = false;
        }
      }
    });

    return () => subscription.unsubscribe();
  }, [eventsForNode$, id, startNewStream, updateConfig]);

  useEffect(() => {
    if (data.config?.projectData) {
      setProjectData(data.config.projectData as ProjectData);
      return;
    }
    if (data.lastMessage && !isStreamingRef.current && !projectData) {
      try {
        const parsed = typeof data.lastMessage === 'string' ? JSON.parse(data.lastMessage) : data.lastMessage;
        setProjectData(parsed);
      } catch {}
    }
  }, []);

  const onProjectDataChange = useCallback(
    (next: ProjectData) => {
      setProjectData(next);
      updateConfig({ projectData: next } as any);
    },
    [updateConfig],
  );

  return {
    isFullScreen,
    setIsFullScreen,
    projectData,
    isNodeCompleted,
    onProjectDataChange,
  };
}

