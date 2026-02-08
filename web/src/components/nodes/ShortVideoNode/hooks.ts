import { useCallback, useEffect, useRef, useState } from 'react';
import { parse } from 'jsonriver';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '@/model/workflow/types';
import type { ProjectData } from '../../ShortVideoCreation/types';

export function useShortVideoNodeController(id: string, data: NodeData) {
  const { updateConfig } = useNodeConfig(id, data.config);

  const [isFullScreen, setIsFullScreen] = useState(false);
  const [projectData, setProjectData] = useState<ProjectData | null>((data.config?.projectData as ProjectData) || null);

  const streamControllerRef = useRef<ReadableStreamDefaultController<string> | null>(null);
  const completedRef = useRef(new WeakMap<object, boolean>());
  const isStreamingRef = useRef(false);
  const lastTextRef = useRef<string>('');

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
      } catch { }
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
      } catch { }
    })();
  }, []);

  useEffect(() => {
    const isOutputStream = Boolean(data.isOutputStream);
    if (isOutputStream && !isStreamingRef.current) {
      isStreamingRef.current = true;
      lastTextRef.current = '';
      setProjectData(null);
      startNewStream();
    }

    if (!isOutputStream && isStreamingRef.current) {
      isStreamingRef.current = false;
      lastTextRef.current = '';
      if (streamControllerRef.current) {
        try {
          streamControllerRef.current.close();
        } catch { }
        streamControllerRef.current = null;
      }
    }
  }, [data.isOutputStream, startNewStream]);

  useEffect(() => {
    if (!isStreamingRef.current) return;
    const nextText = typeof data.lastMessage === 'string' ? data.lastMessage : '';
    const prev = lastTextRef.current;
    if (nextText === prev) return;
    if (!nextText.startsWith(prev)) {
      lastTextRef.current = nextText;
      streamControllerRef.current?.enqueue(nextText);
      return;
    }
    const delta = nextText.slice(prev.length);
    lastTextRef.current = nextText;
    if (delta) streamControllerRef.current?.enqueue(delta);
  }, [data.lastMessage]);

  useEffect(() => {
    if (data.config?.projectData) {
      setProjectData(data.config.projectData as ProjectData);
      return;
    }
    if (!isStreamingRef.current && !projectData) {
      const value = (data.outputs && 'output' in data.outputs) ? (data.outputs as any).output : data.lastMessage;
      if (!value) return;
      try {
        const parsed = typeof value === 'string' ? JSON.parse(value) : value;
        setProjectData(parsed);
      } catch { }
    }
  }, [data.config?.projectData, data.lastMessage, data.outputs, projectData]);

  const onProjectDataChange = useCallback(
    (next: ProjectData) => {
      setProjectData(next);
      updateConfig({ projectData: next });
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
