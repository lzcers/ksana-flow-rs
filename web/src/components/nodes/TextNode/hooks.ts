import { useCallback, useEffect, useRef, useState } from 'react';
import { useNodeConnections } from '@xyflow/react';
import { useIncremark } from '@incremark/react';
import { useStore } from '@/store';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '@/model/types';

export function useTextNodeController(id: string, data: NodeData) {
  const { eventsForCurrentRun$, events$ } = useStore();
  const { updateConfig } = useNodeConfig(id, data.config);

  const [text, setText] = useState<string>(String(data.config?.text ?? ''));
  const [isMarkdown, setIsMarkdown] = useState(false);
  const [isFullScreen, setIsFullScreen] = useState(false);

  const coerceToText = useCallback((value: any): string => {
    if (value == null) return '';
    if (typeof value === 'string') return value;
    if (typeof value === 'number' || typeof value === 'boolean') return String(value);
    if (typeof value === 'object') {
      try {
        return JSON.stringify(value);
      } catch {
        return String(value);
      }
    }
    return String(value);
  }, []);

  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
  });
  const incremarkRef = useRef(incremark);
  useEffect(() => {
    incremarkRef.current = incremark;
  }, [incremark]);

  const connections = useNodeConnections({ handleType: 'target' });
  const connectionsRef = useRef(connections);
  const dataRef = useRef(data);
  const isMarkdownRef = useRef(isMarkdown);
  const isStreamingRef = useRef(data.upstreamIsStreaming || false);

  useEffect(() => {
    connectionsRef.current = connections;
  }, [connections]);

  useEffect(() => {
    dataRef.current = data;
    isStreamingRef.current = data.upstreamIsStreaming || false;
  }, [data]);

  useEffect(() => {
    isMarkdownRef.current = isMarkdown;
  }, [isMarkdown]);

  useEffect(() => {
    if (data.upstreamIsStreaming && text !== data.config?.text) {
      const timeoutId = setTimeout(() => {
        updateConfig({ text });
      }, 200);
      return () => clearTimeout(timeoutId);
    }
  }, [data.upstreamIsStreaming, text, updateConfig, data.config?.text]);

  useEffect(() => {
    if (!data.upstreamIsStreaming && isMarkdown && text !== incremarkRef.current.markdown) {
      incremarkRef.current.render(text);
    }
  }, [text, isMarkdown, data.upstreamIsStreaming]);

  useEffect(() => {
    if (data.lastMessageRunId == null) return;
    const next = coerceToText(data.lastMessage);
    if (next === '') return;
    if (next === text) return;
    setText(next);
    if (isMarkdownRef.current) {
      incremarkRef.current.render(next);
    }
  }, [data.lastMessageRunId, data.lastMessage, coerceToText, text]);

  useEffect(() => {
    const stream$ = eventsForCurrentRun$ || events$;
    if (!stream$) return;

    const subscription = stream$.subscribe((wrapper: any) => {
      const { event } = wrapper;
      const upstreamNodeIds = connectionsRef.current.map((conn) => conn.source);
      const isUpstream = (nodeId: string) => upstreamNodeIds.includes(nodeId);
      const isRelevantNode = (nodeId: string) => nodeId === id || isUpstream(nodeId);

      if (event.NodeStarted) {
        const nodeId = event.NodeStarted;
        if (isRelevantNode(nodeId)) {
          isStreamingRef.current = false;
        }
      } else if (event.NodeStreamStarted) {
        const nodeId = event.NodeStreamStarted;
        if (isRelevantNode(nodeId)) {
          isStreamingRef.current = true;
          setText('');
          setIsMarkdown(true);
          incremarkRef.current.reset();
          updateConfig({ text: '' });
        }
      } else if (event.NodeStreamNextMessage) {
        const [nodeId, value] = event.NodeStreamNextMessage;
        if (isRelevantNode(nodeId) && isStreamingRef.current) {
          const next = coerceToText(value);
          if (next !== '') {
            incremarkRef.current.append(next);
            setText((prev) => prev + next);
          }
        }
      } else if (event.NodeOutMessage) {
        const [nodeId, value] = event.NodeOutMessage;
        if (isRelevantNode(nodeId)) {
          isStreamingRef.current = false;
          const next = coerceToText(value);
          if (next !== '') {
            setText(next);
            updateConfig({ text: next });
            if (isMarkdownRef.current) {
              incremarkRef.current.render(next);
            }
          }
        }
      }
    });

    return () => subscription.unsubscribe();
  }, [eventsForCurrentRun$, events$, updateConfig]);

  const onChange = useCallback((next: string) => {
    setText(next);
  }, []);

  const onBlur = useCallback(() => {
    if (text !== dataRef.current.config?.text) {
      updateConfig({ text });
    }
  }, [text, updateConfig]);

  return {
    text,
    isMarkdown,
    setIsMarkdown,
    isFullScreen,
    setIsFullScreen,
    incremark,
    onChange,
    onBlur,
  };
}
