import { useCallback, useEffect, useRef, useState } from 'react';
import { useNodeConnections } from '@xyflow/react';
import { useIncremark } from '@incremark/react';
import { useStore } from '../../../store';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '../../../model/types';

export function useTextNodeController(id: string, data: NodeData) {
  const { eventsForCurrentRun$, events$ } = useStore();
  const { updateConfig } = useNodeConfig(id, data.config);

  const [text, setText] = useState<string>(String(data.config?.text ?? ''));
  const [isMarkdown, setIsMarkdown] = useState(false);
  const [isFullScreen, setIsFullScreen] = useState(false);

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
    const stream$ = eventsForCurrentRun$ || events$;
    if (!stream$) return;

    const subscription = stream$.subscribe((wrapper: any) => {
      const { event } = wrapper;
      const upstreamNodeIds = connectionsRef.current.map((conn) => conn.source);
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
          incremarkRef.current.reset();
          updateConfig({ text: '' });
        }
      } else if (event.NodeStreamNextMessage) {
        const [nodeId, value] = event.NodeStreamNextMessage;
        if (isUpstream(nodeId) && isStreamingRef.current) {
          if (typeof value === 'string') {
            incremarkRef.current.append(value);
            setText((prev) => prev + value);
          }
        }
      } else if (event.NodeOutMessage) {
        const [nodeId, value] = event.NodeOutMessage;
        if (isUpstream(nodeId) && !isStreamingRef.current) {
          if (typeof value === 'string') {
            setText(value);
            updateConfig({ text: value });
            if (isMarkdownRef.current) {
              incremarkRef.current.render(value);
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
