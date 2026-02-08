import { useCallback, useEffect, useRef, useState } from 'react';
import { useNodeConnections } from '@xyflow/react';
import { useIncremark } from '@incremark/react';
import { useStore } from '@/store';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '@/model/workflow/types';

export function useTextNode(id: string, data: NodeData) {
  const { updateConfig } = useNodeConfig(id, data.config);
  const [text, setText] = useState<string>("");
  const [isMarkdown, setIsMarkdown] = useState<boolean>(() => data.config?.isMarkdown ?? true);
  const nodes = useStore((s) => s.nodes);
  const connections = useNodeConnections();

  const [isFullScreen, setIsFullScreen] = useState(false);

  const isStreamingRef = useRef(false);
  const lastTextRef = useRef<string>('');
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


  const manualText = typeof data.config?.text === 'string' ? data.config.text : '';
  const hasManualText = manualText.length > 0;

  const upstreamNodeIds = connections.map((conn) => conn.source);
  const upstreamNodes = nodes.filter((n) => upstreamNodeIds.includes(n.id));
  const streamingUpstream = upstreamNodes.find((n) => Boolean((n.data as any)?.isOutputStream));
  const preferredUpstream = streamingUpstream ?? upstreamNodes[0];
  const upstreamText = preferredUpstream ? coerceToText((preferredUpstream.data as any)?.lastMessage) : '';
  const upstreamIsStreaming = Boolean((preferredUpstream?.data as any)?.isOutputStream);

  const derivedText = hasManualText
    ? manualText
    : upstreamText || coerceToText(data.lastMessage);

  useEffect(() => {
    const nextText = derivedText;
    const shouldStreamAppend = !hasManualText && upstreamIsStreaming;

    if (shouldStreamAppend && !isStreamingRef.current) {
      isStreamingRef.current = true;
      lastTextRef.current = '';
      setText('');
      incremarkRef.current.reset();
    }
    if (!shouldStreamAppend) {
      isStreamingRef.current = false;
    }

    const prev = lastTextRef.current;
    if (nextText === prev) return;
    lastTextRef.current = nextText;
    setText(nextText);

    if (!isMarkdownRef.current) return;
    if (shouldStreamAppend && nextText.startsWith(prev)) {
      const delta = nextText.slice(prev.length);
      if (delta) incremarkRef.current.append(delta);
      return;
    }
    incremarkRef.current.render(nextText);
  }, [derivedText, hasManualText, upstreamIsStreaming]);

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
    setText,
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
