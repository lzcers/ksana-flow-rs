import { useCallback, useEffect, useRef, useState } from 'react';
import { useIncremark } from '@incremark/react';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '@/model/workflow/types';

function adjustTextareaHeight(el: HTMLTextAreaElement) {
  el.style.height = 'auto';
  el.style.height = `${el.scrollHeight}px`;
  el.style.overflowY = el.scrollHeight > el.clientHeight ? 'auto' : 'hidden';
}

export function useLLMNodeController(id: string, data: NodeData) {
  const { updateConfig } = useNodeConfig(id, data.config);

  const systemInputRef = useRef<HTMLTextAreaElement>(null);
  const userInputRef = useRef<HTMLTextAreaElement>(null);

  const [systemPrompt, setSystemPrompt] = useState(String(data.config?.system_prompt ?? ''));
  const [userPrompt, setUserPrompt] = useState(String(data.config?.user_prompt_template ?? ''));
  const [model, setModel] = useState(String(data.config?.model ?? 'deepseek-chat'));
  const [stream, setStream] = useState(Boolean(data.config?.stream ?? true));
  const [isConfigOpen, setIsConfigOpen] = useState(false);
  const [outputText, setOutputText] = useState<string>('');
  const [isMarkdown, setIsMarkdown] = useState(true);
  const [isFullScreen, setIsFullScreen] = useState(false);
  const [isSystemFullScreen, setIsSystemFullScreen] = useState(false);

  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
  });

  const isComposingSystem = useRef(false);
  const isComposingUser = useRef(false);

  useEffect(() => {
    if (document.activeElement !== systemInputRef.current) {
      setSystemPrompt(String(data.config?.system_prompt ?? ''));
      if (systemInputRef.current) adjustTextareaHeight(systemInputRef.current);
    }
  }, [data.config?.system_prompt]);

  useEffect(() => {
    if (document.activeElement !== userInputRef.current) {
      setUserPrompt(String(data.config?.user_prompt_template ?? ''));
      if (userInputRef.current) adjustTextareaHeight(userInputRef.current);
    }
  }, [data.config?.user_prompt_template]);

  useEffect(() => {
    setModel(String(data.config?.model ?? 'deepseek-chat'));
    setStream(Boolean(data.config?.stream ?? true));
  }, [data.config?.model, data.config?.stream]);

  // 直接渲染 lastMessage，因为 instance.ts 已在事件处理时合并了流式消息
  useEffect(() => {
    const text = typeof data.lastMessage === 'string' ? data.lastMessage : '';
    setOutputText(text);
    if (isMarkdown) {
      incremark.render(text);
    }
  }, [data.lastMessage, isMarkdown]);
  useEffect(() => {
    if (!isConfigOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setIsConfigOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isConfigOpen]);

  useEffect(() => {
    if (Boolean(data.isOutputStream)) return;
    const value = data?.outputs?.output;
    if (typeof value !== 'string') return;
    setOutputText(value);
    if (isMarkdown) {
      incremark.render(value);
    }
  }, [data?.outputs, data.isOutputStream, isMarkdown]);

  const onModelChange = useCallback(
    (next: string) => {
      setModel(next);
      updateConfig({ model: next });
    },
    [updateConfig],
  );

  const onStreamChange = useCallback(
    (next: boolean) => {
      setStream(next);
      updateConfig({ stream: next });
    },
    [updateConfig],
  );

  const onSystemPromptChange = useCallback(
    (next: string, el?: HTMLTextAreaElement) => {
      setSystemPrompt(next);
      if (el) adjustTextareaHeight(el);
      if (!isComposingSystem.current) {
        updateConfig({ system_prompt: next });
      }
    },
    [updateConfig],
  );

  const onSystemCompositionStart = useCallback(() => {
    isComposingSystem.current = true;
  }, []);

  const onSystemCompositionEnd = useCallback(
    (next: string) => {
      isComposingSystem.current = false;
      updateConfig({ system_prompt: next });
    },
    [updateConfig],
  );

  const onUserPromptChange = useCallback(
    (next: string, el?: HTMLTextAreaElement) => {
      setUserPrompt(next);
      if (el) adjustTextareaHeight(el);
      if (!isComposingUser.current) {
        updateConfig({ user_prompt_template: next });
      }
    },
    [updateConfig],
  );

  const onUserCompositionStart = useCallback(() => {
    isComposingUser.current = true;
  }, []);

  const onUserCompositionEnd = useCallback(
    (next: string) => {
      isComposingUser.current = false;
      updateConfig({ user_prompt_template: next });
    },
    [updateConfig],
  );

  const onOutputChange = useCallback((next: string) => {
    setOutputText(next);
  }, []);

  const onOutputBlur = useCallback(() => {
    updateConfig({ output: outputText });
    if (isMarkdown) {
      incremark.render(outputText);
    }
  }, [outputText, updateConfig, isMarkdown, incremark]);

  return {
    systemInputRef,
    userInputRef,
    systemPrompt,
    userPrompt,
    model,
    stream,
    isConfigOpen,
    isMarkdown,
    isFullScreen,
    isSystemFullScreen,
    outputText,
    incremark,
    setIsConfigOpen,
    setIsMarkdown,
    setIsFullScreen,
    setIsSystemFullScreen,
    onModelChange,
    onStreamChange,
    onSystemPromptChange,
    onSystemCompositionStart,
    onSystemCompositionEnd,
    onUserPromptChange,
    onUserCompositionStart,
    onUserCompositionEnd,
    onOutputChange,
    onOutputBlur,
  };
}
