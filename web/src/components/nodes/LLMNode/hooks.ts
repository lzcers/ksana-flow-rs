import { useCallback, useEffect, useRef, useState } from 'react';
import { useIncremark } from '@incremark/react';
import { useStore } from '../../../store';
import { useNodeConfig } from '../shared/hooks/useNodeConfig';
import type { NodeData } from '../../../model/types';

function adjustTextareaHeight(el: HTMLTextAreaElement) {
  el.style.height = 'auto';
  el.style.height = `${el.scrollHeight}px`;
  el.style.overflowY = el.scrollHeight > el.clientHeight ? 'auto' : 'hidden';
}

export function useLLMNodeController(id: string, data: NodeData) {
  const { eventsForNode$ } = useStore();
  const { updateConfig } = useNodeConfig(id, data.config);

  const systemInputRef = useRef<HTMLTextAreaElement>(null);
  const userInputRef = useRef<HTMLTextAreaElement>(null);

  const [systemPrompt, setSystemPrompt] = useState(String(data.config?.system_prompt ?? ''));
  const [userPrompt, setUserPrompt] = useState(String(data.config?.user_prompt_template ?? ''));
  const [model, setModel] = useState(String(data.config?.model ?? 'deepseek-chat'));
  const [stream, setStream] = useState(Boolean(data.config?.stream ?? true));
  const [isConfigOpen, setIsConfigOpen] = useState(false);
  const [outputText, setOutputText] = useState<string>('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [isMarkdown, setIsMarkdown] = useState(true);
  const [isFullScreen, setIsFullScreen] = useState(false);
  const [isSystemFullScreen, setIsSystemFullScreen] = useState(false);

  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
  });
  const incremarkRef = useRef(incremark);
  useEffect(() => {
    incremarkRef.current = incremark;
  }, [incremark]);

  const isStreamingRef = useRef(false);
  const isMarkdownRef = useRef(isMarkdown);
  useEffect(() => {
    isMarkdownRef.current = isMarkdown;
  }, [isMarkdown]);

  const pendingChunkRef = useRef('');
  const flushRafIdRef = useRef<number | null>(null);

  const cancelFlush = useCallback(() => {
    if (flushRafIdRef.current !== null) {
      window.cancelAnimationFrame(flushRafIdRef.current);
      flushRafIdRef.current = null;
    }
    pendingChunkRef.current = '';
  }, []);

  const scheduleFlush = useCallback(() => {
    if (flushRafIdRef.current !== null) return;
    flushRafIdRef.current = window.requestAnimationFrame(() => {
      flushRafIdRef.current = null;
      const chunk = pendingChunkRef.current;
      if (!chunk) return;
      pendingChunkRef.current = '';
      if (isMarkdownRef.current) {
        incremarkRef.current.append(chunk);
      }
      setOutputText((prev) => prev + chunk);
    });
  }, []);

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

  useEffect(() => {
    if (isStreamingRef.current) return;

    let nextText: string | null = null;
    if (typeof data.config?.output === 'string') {
      nextText = data.config.output;
    } else if (typeof data.outputs?.output === 'string') {
      nextText = data.outputs.output;
    } else if (typeof data.lastMessage === 'string') {
      nextText = data.lastMessage;
    }

    if (nextText === null) return;
    setOutputText((prev) => (prev === nextText ? prev : nextText));

    if (isMarkdownRef.current && incremarkRef.current.markdown !== nextText) {
      incremarkRef.current.render(nextText);
    }
  }, [data.config?.output, data.outputs?.output, data.lastMessage]);

  useEffect(() => {
    if (!isMarkdown) return;
    if (isStreamingRef.current) return;
    if (incremarkRef.current.markdown !== outputText) {
      incremarkRef.current.render(outputText);
    }
  }, [isMarkdown, outputText]);

  useEffect(() => {
    if (!isConfigOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setIsConfigOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isConfigOpen]);

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
    if (isMarkdownRef.current) {
      incremarkRef.current.render(outputText);
    }
  }, [outputText, updateConfig]);

  useEffect(() => {
    const stream$ = eventsForNode$?.(id);
    if (!stream$) return;
    const subscription = stream$.subscribe((wrapper: any) => {
      const { event } = wrapper;
      if (event.NodeStarted) {
        if (event.NodeStarted === id) {
          cancelFlush();
          isStreamingRef.current = false;
          setIsStreaming(false);
        }
      } else if (event.NodeStreamStarted) {
        if (event.NodeStreamStarted === id) {
          cancelFlush();
          isStreamingRef.current = true;
          setIsStreaming(true);
          setOutputText('');
          updateConfig({ output: '' });
          isMarkdownRef.current = true;
          setIsMarkdown(true);
          incremarkRef.current.reset();
        }
      } else if (event.NodeStreamNextMessage) {
        const [nodeId, value] = event.NodeStreamNextMessage;
        if (nodeId === id && isStreamingRef.current) {
          if (typeof value === 'string') {
            pendingChunkRef.current += value;
            scheduleFlush();
          }
        }
      } else if (event.NodeError) {
        const [nodeId] = event.NodeError;
        if (nodeId === id) {
          cancelFlush();
          isStreamingRef.current = false;
          setIsStreaming(false);
        }
      } else if (event.NodeCompleted) {
        if (event.NodeCompleted === id) {
          cancelFlush();
          isStreamingRef.current = false;
          setIsStreaming(false);
        }
      } else if (event.NodeOutMessage) {
        const [nodeId, value] = event.NodeOutMessage;
        if (nodeId === id) {
          cancelFlush();
          if (typeof value === 'string') {
            setOutputText(value);
            updateConfig({ output: value });
            if (isMarkdownRef.current) {
              incremarkRef.current.render(value);
            }
          }
          isStreamingRef.current = false;
          setIsStreaming(false);
        }
      }
    });
    return () => {
      cancelFlush();
      subscription.unsubscribe();
    };
  }, [eventsForNode$, id, cancelFlush, scheduleFlush, updateConfig]);

  return {
    systemInputRef,
    userInputRef,
    systemPrompt,
    userPrompt,
    model,
    stream,
    isConfigOpen,
    setIsConfigOpen,
    isStreaming,
    isMarkdown,
    setIsMarkdown,
    isFullScreen,
    setIsFullScreen,
    isSystemFullScreen,
    setIsSystemFullScreen,
    outputText,
    incremark,
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
