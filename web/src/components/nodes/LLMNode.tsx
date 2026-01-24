import { memo, useCallback, useEffect, useRef, useState } from 'react';
import { Position, type NodeProps } from '@xyflow/react';
import { Settings, X, Eye, Pencil, Maximize2 } from 'lucide-react';
import { AutoScrollContainer, Incremark, ThemeProvider, useIncremark } from '@incremark/react';
import { FullScreenModal } from '../ui/FullScreenModal';
import { theme } from './TextNode/theme';
import '@incremark/theme/styles.css';
import './index.css';
import type { NodeData } from '../../model/types';
import { NodeWrapper } from './NodeWrapper';
import { useStore } from '../../store';

const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];
const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];

export const LLMNode = memo(({ id, type, data, selected, width, height }: NodeProps & { data: NodeData }) => {
  const { updateNodeData, events$, currentRunId } = useStore();

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
  const [stream, setStream] = useState(data.config?.stream || false);
  const [isConfigOpen, setIsConfigOpen] = useState(false);
  const [outputText, setOutputText] = useState<string>('');
  const isStreamingRef = useRef(false);
  const [isStreaming, setIsStreaming] = useState(false);
  const [isMarkdown, setIsMarkdown] = useState(false);
  const [isFullScreen, setIsFullScreen] = useState(false);

  const incremark = useIncremark({
    math: { tex: true },
    gfm: true,
  });

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
    setStream(data.config?.stream || false);
  }, [data.config?.model, data.config?.stream]);

  useEffect(() => {
    if (typeof data.config?.output === 'string') {
      setOutputText(data.config.output);
    } else if (typeof data.lastMessage === 'string' && !isStreamingRef.current) {
      setOutputText(data.lastMessage);
    }
  }, []);

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
    updateNodeData(id, { config: { ...data.config, model: newModel } });
  }, [id, data.config, updateNodeData]);

  const handleStreamChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newStream = e.target.checked;
    setStream(newStream);
    updateNodeData(id, { config: { ...data.config, stream: newStream } });
  }, [id, data.config, updateNodeData]);

  // System Prompt Handlers
  const handleSystemPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setSystemPrompt(newValue);

    adjustHeight(e.target);

    if (!isComposingSystem.current) {
      updateNodeData(id, {
        config: {
          ...data.config,
          system_prompt: newValue
        }
      });
    }
  }, [id, data.config, updateNodeData]);

  const handleSystemCompositionStart = () => {
    isComposingSystem.current = true;
  };

  const handleSystemCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposingSystem.current = false;
    const newValue = e.currentTarget.value;
    // Ensure we trigger an update when composition ends
    updateNodeData(id, {
      config: {
        ...data.config,
        system_prompt: newValue
      }
    });
  }, [id, data.config, updateNodeData]);

  // User Prompt Handlers
  const handleUserPromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setUserPrompt(newValue);

    adjustHeight(e.target);

    if (!isComposingUser.current) {
      updateNodeData(id, {
        config: {
          ...data.config,
          user_prompt_template: newValue
        }
      });
    }
  }, [id, data.config, updateNodeData]);

  const handleUserCompositionStart = () => {
    isComposingUser.current = true;
  };

  const handleUserCompositionEnd = useCallback((e: React.CompositionEvent<HTMLTextAreaElement>) => {
    isComposingUser.current = false;
    const newValue = e.currentTarget.value;
    updateNodeData(id, {
      config: {
        ...data.config,
        user_prompt_template: newValue
      }
    });
  }, [id, data.config, updateNodeData]);

  useEffect(() => {
    if (!events$) return;
    const subscription = events$.subscribe((wrapper: any) => {
      const { event, runId } = wrapper;
      if (currentRunId && runId !== currentRunId) return;
      if (event.NodeStarted) {
        if (event.NodeStarted === id) {
          isStreamingRef.current = false;
          setIsStreaming(false);
        }
      } else if (event.NodeStreamStarted) {
        if (event.NodeStreamStarted === id) {
          isStreamingRef.current = true;
          setIsStreaming(true);
          setOutputText('');
          updateNodeData(id, { config: { ...data.config, output: '' } });
          setIsMarkdown(true);
          incremark.reset();
        }
      } else if (event.NodeStreamNextMessage) {
        const [nodeId, value] = event.NodeStreamNextMessage;
        if (nodeId === id && isStreamingRef.current) {
          if (typeof value === 'string') {
            incremark.append(value);
            setOutputText(prev => prev + value);
          }
        }
      } else if (event.NodeOutMessage) {
        const [nodeId, value] = event.NodeOutMessage;
        if (nodeId === id) {
          if (!isStreamingRef.current) {
            if (typeof value === 'string') {
              setOutputText(value);
              updateNodeData(id, { config: { ...data.config, output: value } });
              if (isMarkdown) {
                incremark.render(value);
              }
            }
          } else {
            if (typeof value === 'string') {
              setOutputText(value);
              updateNodeData(id, { config: { ...data.config, output: value } });
            }
            isStreamingRef.current = false;
            setIsStreaming(false);
          }
        }
      }
    });
    return () => subscription.unsubscribe();
  }, [events$, currentRunId, id, updateNodeData, data.config, incremark, isMarkdown]);

  const onOutputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setOutputText(e.target.value);
  }, []);

  const onOutputBlur = useCallback(() => {
    updateNodeData(id, { config: { ...data.config, output: outputText } });
    if (isMarkdown) {
      incremark.render(outputText);
    }
  }, [id, data.config, outputText, updateNodeData, incremark, isMarkdown]);

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
          <div className="absolute inset-0 z-50 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl shadow-2xl flex flex-col">
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
            <div className="p-3 grid grid-cols-12 gap-2 flex-1 overflow-auto  grid-rows-[auto_1fr] min-auto custom-scrollbar">
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
                <label className="text-[10px] text-zinc-500 font-bold block mb-1">System Prompt</label>
                <textarea
                  style={{ boxSizing: 'content-box' }}
                  ref={systemInputRef}
                  className="flex-1 text-xs bg-zinc-900/60 hover:bg-zinc-900/70 focus:bg-zinc-900 border border-zinc-800 focus:border-zinc-700 rounded resize-none nodrag focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:ring-offset-1 focus:ring-offset-zinc-900 text-zinc-300 focus:text-zinc-200 transition-colors duration-200 placeholder-zinc-500 shadow-inner"
                  value={systemPrompt}
                  onChange={handleSystemPromptChange}
                  onCompositionStart={handleSystemCompositionStart}
                  onCompositionEnd={handleSystemCompositionEnd}
                  onKeyDown={(e) => e.stopPropagation()}
                  placeholder="System prompt..."
                />
              </div>
            </div>
          </div>
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
              placeholder="User prompt template..."
            />
          </div>
        </div>
      </div>
    </NodeWrapper>
  );
});

LLMNode.displayName = 'LLMNode';
