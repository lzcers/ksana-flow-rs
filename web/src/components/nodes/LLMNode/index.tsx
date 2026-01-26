import { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import { Settings, X, Eye, Pencil, Maximize2 } from 'lucide-react';
import { AutoScrollContainer, Incremark, ThemeProvider, useIncremark } from '@incremark/react';
import { FullScreenModal } from '../../ui/FullScreenModal';
import { theme } from '../TextNode/theme';
import type { NodeData } from '../../../model/types';
import { NodeWrapper } from '../NodeWrapper';
import { useStore } from '../../../store';
import '@incremark/theme/styles.css';
import './index.css';

const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];
const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];

export const LLMNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData, eventsForNode$ } = useStore();

  const adjustHeight = (el: HTMLTextAreaElement) => {
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
    if (el.scrollHeight > el.clientHeight) {
      el.style.overflowY = 'auto';
    } else {
      el.style.overflowY = 'hidden';
    }
  };

  const systemInputRef = useRef<HTMLTextAreaElement>(null);
  const userInputRef = useRef<HTMLTextAreaElement>(null);

  const [systemPrompt, setSystemPrompt] = useState(data.config?.system_prompt || '');
  const [userPrompt, setUserPrompt] = useState(data.config?.user_prompt_template || '');
  const [model, setModel] = useState(data.config?.model || 'deepseek-chat');
  const [stream, setStream] = useState(data.config?.stream ?? true);
  const [isConfigOpen, setIsConfigOpen] = useState(false);
  const [outputText, setOutputText] = useState<string>('');
  const isStreamingRef = useRef(false);
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

  const dataConfigRef = useRef(data.config);
  useEffect(() => {
    dataConfigRef.current = data.config;
  }, [data.config]);

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

  const updateConfig = useCallback((patch: Record<string, unknown>) => {
    updateNodeData(id, { config: { ...(dataConfigRef.current ?? {}), ...patch } });
  }, [id, updateNodeData]);

  const isComposingSystem = useRef(false);
  const isComposingUser = useRef(false);

  // Sync local state with props when props change (and not focused)
  useEffect(() => {
    if (document.activeElement !== systemInputRef.current) {
      setSystemPrompt(data.config?.system_prompt || '');
      // Adjust height if needed
      if (systemInputRef.current) {
        adjustHeight(systemInputRef.current);
      }
    }
  }, [data.config?.system_prompt]);

  useEffect(() => {
    if (document.activeElement !== userInputRef.current) {
      setUserPrompt(data.config?.user_prompt_template || '');
      // Adjust height if needed
      if (userInputRef.current) {
        adjustHeight(userInputRef.current);
      }
    }
  }, [data.config?.user_prompt_template]);

  useEffect(() => {
    setModel(data.config?.model || 'deepseek-chat');
    setStream(data.config?.stream ?? true);
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

  const handleModelChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const newModel = e.target.value;
    setModel(newModel);
    updateConfig({ model: newModel });
  }, [updateConfig]);

  const handleStreamChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newStream = e.target.checked;
    setStream(newStream);
    updateConfig({ stream: newStream });
  }, [updateConfig]);

  // System Prompt Handlers
  const handleSystemPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setSystemPrompt(newValue);

    adjustHeight(e.target);

    if (!isComposingSystem.current) {
      updateConfig({ system_prompt: newValue });
    }
  }, [updateConfig]);

  const handleSystemCompositionStart = () => {
    isComposingSystem.current = true;
  };

  const handleSystemCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposingSystem.current = false;
    const newValue = e.currentTarget.value;
    // Ensure we trigger an update when composition ends
    updateConfig({ system_prompt: newValue });
  }, [updateConfig]);

  // User Prompt Handlers
  const handleUserPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setUserPrompt(newValue);

    adjustHeight(e.target);

    if (!isComposingUser.current) {
      updateConfig({ user_prompt_template: newValue });
    }
  }, [updateConfig]);

  const handleUserCompositionStart = () => {
    isComposingUser.current = true;
  };

  const handleUserCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposingUser.current = false;
    const newValue = e.currentTarget.value;
    updateConfig({ user_prompt_template: newValue });
  }, [updateConfig]);

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
          if (!isStreamingRef.current) {
            if (typeof value === 'string') {
              setOutputText(value);
              updateConfig({ output: value });
              if (isMarkdownRef.current) {
                incremarkRef.current.render(value);
              }
            }
          } else {
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
      }
    });
    return () => {
      cancelFlush();
      subscription.unsubscribe();
    };
  }, [eventsForNode$, id, updateConfig, cancelFlush, scheduleFlush]);

  const onOutputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setOutputText(e.target.value);
  }, []);


  const onOutputBlur = useCallback(() => {
    updateConfig({ output: outputText });
    if (isMarkdownRef.current) {
      incremarkRef.current.render(outputText);
    }
  }, [outputText, updateConfig]);

  const headerActions = (
    <div className="relative flex items-center gap-1">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsFullScreen(true);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title="全屏预览"
      >
        <Maximize2 size={12} />
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsMarkdown(v => !v);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title={isMarkdown ? '切换到编辑模式' : '切换到Markdown预览'}
      >
        {isMarkdown ? <Pencil size={12} /> : <Eye size={12} />}
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsConfigOpen(v => !v);
        }}
        className="text-zinc-400 hover:text-zinc-200 transition-colors p-1 rounded hover:bg-zinc-800"
        title="设置"
      >
        <Settings size={12} />
      </button>
    </div>
  );
  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      minWidth={300}
      minHeight={400}
      style={{ width, height }}
      targetHandles={TARGET_HANDLES}
      sourceHandles={SOURCE_HANDLES}
      headerActions={headerActions}
    >
      <div className="flex flex-col h-full relative">
        {isConfigOpen && (
          <div className="absolute inset-0 z-50 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl shadow-2xl flex flex-col"
          >
            <div className="px-3 py-2 flex items-center justify-between border-b border-zinc-800">
              <div className="flex items-center gap-2 text-[10px] text-zinc-400">
                <span>LLM 设置</span>
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); setIsConfigOpen(false); }}
                className="text-zinc-400 hover:text-zinc-200 p-1 rounded hover:bg-zinc-800"
                title="关闭"
              >
                <X size={12} />
              </button>
            </div>
            <div className="p-3 grid grid-cols-12 gap-2 flex-1 overflow-auto grid-rows-[auto_1fr] min-auto custom-scrollbar">
              <div className="col-span-12 flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-zinc-500 font-bold">Model</span>
                  <select
                    className="min-w-[160px] text-xs bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded px-1 py-1 text-zinc-300 focus:text-zinc-200 focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 transition-colors duration-200"
                    value={model}
                    onChange={handleModelChange}
                  >
                    <option value="deepseek-chat">DeepSeek Chat</option>
                    <option value="deepseek-reasoner">DeepSeek Reasoner</option>
                    <option value="google/gemini-3-pro-preview">Gemini 3 Pro preview</option>
                  </select>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-zinc-500 font-bold">Stream</span>
                  <input
                    type="checkbox"
                    className="rounded border-zinc-800 bg-black text-blue-500 focus:ring-blue-500 cursor-pointer"
                    checked={stream}
                    onChange={handleStreamChange}
                  />
                </div>
              </div>
              <div className="col-span-12 flex flex-col flex-1">
                <div className="flex items-center justify-between text-[10px] text-zinc-500 font-bold mb-1">
                  <span>System Prompt</span>
                  <button
                    onClick={(e) => { e.stopPropagation(); setIsSystemFullScreen(true); }}
                    className="text-zinc-400 hover:text-zinc-200 p-1 rounded hover:bg-zinc-800"
                    title="全屏编辑"
                  >
                    <Maximize2 size={12} />
                  </button>
                </div>
                <textarea
                  style={{ boxSizing: 'content-box', height: "100%" }}
                  ref={systemInputRef}
                  className="flex-1 text-xs nowheel bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner"
                  value={systemPrompt}
                  onChange={handleSystemPromptChange}
                  onCompositionStart={handleSystemCompositionStart}
                  onCompositionEnd={handleSystemCompositionEnd}
                  onKeyDown={(e) => e.stopPropagation()}
                  onWheel={(e) => e.stopPropagation()}
                  placeholder="System prompt..."
                />
              </div>
            </div>
          </div>
        )}
        {isSystemFullScreen && (
          <FullScreenModal
            isOpen={isSystemFullScreen}
            onClose={() => setIsSystemFullScreen(false)}
            title={'System Prompt 编辑'}
          >
            <div className="w-full h-full flex flex-1 flex-col p-4 text-zinc-200 text-sm overflow-auto custom-scrollbar bg-black">
              <textarea
                className="flex flex-1 bg-black resize-none focus:outline-none text-zinc-200 font-mono"
                style={{ boxSizing: 'content-box', height: "100%" }}
                value={systemPrompt}
                onChange={handleSystemPromptChange}
                onCompositionStart={handleSystemCompositionStart}
                onCompositionEnd={handleSystemCompositionEnd}
                onKeyDown={(e) => e.stopPropagation()}
                onWheel={(e) => e.stopPropagation()}
                placeholder="System prompt..."
                spellCheck={false}
              />
            </div>
          </FullScreenModal>
        )}
        {isFullScreen && (
          <FullScreenModal
            isOpen={isFullScreen}
            onClose={() => setIsFullScreen(false)}
            title={isMarkdown ? 'Markdown 预览' : 'LLM 输出'}
          >
            <div className="w-full h-full flex p-4 text-zinc-200 text-sm overflow-auto custom-scrollbar bg-black justify-center items-center">
              {isMarkdown ? (
                <ThemeProvider theme={theme}>
                  <AutoScrollContainer enabled={isStreaming} className="h-full w-full">
                    <Incremark incremark={incremark} />
                  </AutoScrollContainer>
                </ThemeProvider>
              ) : (
                <textarea
                  className="w-full h-full p-4 bg-black resize-none focus:outline-none text-zinc-200 font-mono"
                  value={outputText}
                  onChange={onOutputChange}
                  onBlur={onOutputBlur}
                  onWheel={(e) => e.stopPropagation()}
                  placeholder="LLM 输出内容..."
                  spellCheck={false}
                />
              )}
            </div>
          </FullScreenModal>
        )}
        <div className="flex flex-col flex-1 p-3 bg-zinc-950/50 border-b border-zinc-800 overflow-auto custom-scrollbar">
          <div className="text-[10px] text-zinc-500 font-bold mb-1 flex items-center justify-between">
            <span>LLM 输出</span>
            <span className="text-[10px] opacity-50">{isMarkdown ? 'Markdown' : 'Raw'}</span>
          </div>
          {isMarkdown ? (
            <div
              className="w-full flex-1 text-xs bg-zinc-900/60 border border-zinc-800 rounded overflow-auto overflow-x-hidden nodrag nowheel text-zinc-200 custom-scrollbar"
              onKeyDown={(e) => e.stopPropagation()}
              onWheel={(e) => {
                e.stopPropagation();
              }}
            >
              <ThemeProvider theme={theme}>
                <AutoScrollContainer enabled={isStreaming} className="h-[300px] p-2">
                  <Incremark incremark={incremark} />
                </AutoScrollContainer>
              </ThemeProvider>
            </div>
          ) : (
            <textarea
              className="w-full flex-1 p-2 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag nowheel focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner"
              value={outputText}
              onChange={onOutputChange}
              onBlur={onOutputBlur}
              placeholder="LLM 输出内容..."
              onWheel={(e) => e.stopPropagation()}
              onKeyDown={(e) => e.stopPropagation()}
            />
          )}
        </div>
        <div className="px-3 pb-3 space-y-2 pt-2">
          <div>
            <label className="text-[10px] text-zinc-500 font-bold block mb-1">User Prompt</label>
            <textarea
              style={{ boxSizing: 'content-box' }}
              ref={userInputRef}
              className="w-full flex-1 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border overflow-x-hidden over max-h-[200px] border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner"
              rows={5}
              value={userPrompt}
              onChange={handleUserPromptChange}
              onCompositionStart={handleUserCompositionStart}
              onCompositionEnd={handleUserCompositionEnd}
              onKeyDown={(e) => e.stopPropagation()}
              onWheel={(e) => e.stopPropagation()}
              placeholder="User prompt template..."
            />
          </div>
        </div>
      </div>
    </NodeWrapper>
  );
});

LLMNode.displayName = 'LLMNode';
