import { useCallback, useEffect, useRef, useState } from 'react';
import { useNodeConnections } from '@xyflow/react';
import { useIncremark } from '@incremark/react';
import { useStore } from '@/store';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '@/model/workflow/types';
import type { FlowEvent } from '@/model/flowEvent/types';

export function useTextNode(id: string, data: NodeData) {
  const { updateConfig } = useNodeConfig(id, data.config);
  const [text, setText] = useState<string>(() => data.config?.text ?? '');
  const [isMarkdown, setIsMarkdown] = useState<boolean>(() => data.config?.isMarkdown ?? true);
  const { events$, eventsForCurrentRun$ } = useStore();
  const connections = useNodeConnections();
  const connectionsRef = useRef(connections);

  useEffect(() => {
    connectionsRef.current = connections;
  }, [connections]);

  const [isFullScreen, setIsFullScreen] = useState(false);

  const isStreamingRef = useRef(false);
  const isMarkdownRef = useRef(isMarkdown);

  useEffect(() => {
    isMarkdownRef.current = isMarkdown;
  }, [isMarkdown]);

  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
  });

  const incremarkRef = useRef(incremark);
  useEffect(() => {
    incremarkRef.current = incremark;
  }, [incremark]);


  useEffect(() => {
    if (data.lastMessage !== undefined && data.lastMessage !== text) {
      const next = typeof data.lastMessage === 'string' ? data.lastMessage : JSON.stringify(data.lastMessage);
      if (next !== text) {
        setText(next);
        updateConfig({ text: next });
        if (isMarkdownRef.current) {
          incremarkRef.current.render(next);
        }
      }
    }
  }, [data.lastMessageRunId, data.lastMessage, text, updateConfig]);

  useEffect(() => {
    const stream$ = eventsForCurrentRun$ || events$;
    if (!stream$) return;

    const subscription = stream$.subscribe((event: FlowEvent) => {
      const upstreamNodeIds = connectionsRef.current.map((conn) => conn.source);
      const isUpstream = (nodeId: string) => upstreamNodeIds.includes(nodeId);
      const isRelevantNode = (nodeId: string) => nodeId === id || isUpstream(nodeId);

      if ('nodeId' in event) {
        const { nodeId } = event;
        if (!isRelevantNode(nodeId)) return;
        switch (event.type) {
          case 'NodeStarted':
            isStreamingRef.current = false;
            break;
          case 'NodeStreamStarted':
            isStreamingRef.current = true;
            setText('');
            setIsMarkdown(true);
            incremarkRef.current.reset();
            updateConfig({ text: '' });
            break;
          case 'NodeStreamNextMessage':
            if (isStreamingRef.current) {
              const next = coerceToText(event.msg);
              if (next !== '') {
                incremarkRef.current.append(next);
                setText(prev => prev + next);
              }
            }
            break;
          case 'NodeOutMessage': {
            isStreamingRef.current = false;
            const next = coerceToText(event.msg);
            if (next !== '') {
              setText(next);
              updateConfig({ text: next });
              if (isMarkdownRef.current) {
                incremarkRef.current.render(next);
              }
            }
            break;
          }
        }
      }
    });

    return () => subscription.unsubscribe();
  }, [eventsForCurrentRun$, events$, updateConfig, id]);

  const onChange = useCallback((next: string) => {
    setText(next);
  }, []);

  const onSave = useCallback((next: string) => {
    updateConfig({ text: next });
  }, [updateConfig]);

  const toggleMarkdown = useCallback(() => {
    setIsMarkdown((prev) => {
      const next = !prev;
      updateConfig({ isMarkdown: next });
      return next;
    });
  }, [updateConfig]);

  const resetText = useCallback(() => {
    setText('');
    updateConfig({ text: '' });
    incremarkRef.current.reset();
  }, [updateConfig]);

  const onBlur = useCallback(() => {
    updateConfig({ text });
  }, [updateConfig, text]);

  return {
    text,
    isMarkdown,
    onChange,
    onSave,
    toggleMarkdown,
    resetText,
    isFullScreen,
    setIsFullScreen,
    incremark,
    onBlur,
  };
}

function coerceToText(value: unknown): string {
  if (typeof value === 'string') return value;
  if (value === null || value === undefined) return '';
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}
