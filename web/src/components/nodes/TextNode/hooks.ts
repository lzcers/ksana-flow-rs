import { useCallback, useEffect, useState } from 'react';
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

  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
  });

  // 直接渲染 lastMessage，因为 instance.ts 已在事件处理时合并了流式消息
  // 对于上游是 isOutputStream 的节点，也是直接渲染上游 lastMessage
  const manualText = typeof data.config?.text === 'string' ? data.config.text : '';
  const hasManualText = manualText.length > 0;

  const upstreamNodeIds = connections.map((conn) => conn.source);
  const upstreamNodes = nodes.filter((n) => upstreamNodeIds.includes(n.id));
  const streamingUpstream = upstreamNodes.find((n) => Boolean((n.data as any)?.isOutputStream));
  // 取上游节点第 0 个，且状态为 running
  const preferredUpstream = streamingUpstream && upstreamNodes[0] && streamingUpstream.data.status === 'running';
  const upstreamText = preferredUpstream ? coerceToText((streamingUpstream.data)?.lastMessage) : '';
  const derivedText = hasManualText
    ? manualText
    : preferredUpstream ? upstreamText : coerceToText(data.lastMessage);

  useEffect(() => {
    setText(derivedText);
    incremark.render(derivedText);
  }, [derivedText]);

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
    incremark.reset();
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
