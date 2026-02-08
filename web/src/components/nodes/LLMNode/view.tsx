import React from 'react';
import { AutoScrollContainer, Incremark, ThemeProvider } from '@incremark/react';
import { Maximize2, Eye, Pencil, Settings, X } from 'lucide-react';
import { Position, type NodeProps } from '@xyflow/react';
import { FullScreenModal } from '../../ui/FullScreenModal';
import { theme } from '../TextNode/theme';
import type { NodeData } from '@/model/workflow/types';
import { NodeWrapper } from '../shared/NodeWrapper';
import { llmNodeStyles } from './styles';

const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];
const SOURCE_HANDLES = [Position.Right, Position.Top, Position.Bottom];

export function LLMNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  systemInputRef,
  userInputRef,
  systemPrompt,
  userPrompt,
  model,
  stream,
  isConfigOpen,
  setIsConfigOpen,
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
}: NodeProps & {
  data: NodeData;
} & {
  systemInputRef: React.RefObject<HTMLTextAreaElement | null>;
  userInputRef: React.RefObject<HTMLTextAreaElement | null>;
  systemPrompt: string;
  userPrompt: string;
  model: string;
  stream: boolean;
  isConfigOpen: boolean;
  setIsConfigOpen: React.Dispatch<React.SetStateAction<boolean>>;
  isMarkdown: boolean;
  setIsMarkdown: React.Dispatch<React.SetStateAction<boolean>>;
  isFullScreen: boolean;
  setIsFullScreen: React.Dispatch<React.SetStateAction<boolean>>;
  isSystemFullScreen: boolean;
  setIsSystemFullScreen: React.Dispatch<React.SetStateAction<boolean>>;
  outputText: string;
  incremark: any;
  onModelChange: (next: string) => void;
  onStreamChange: (next: boolean) => void;
  onSystemPromptChange: (next: string, el?: HTMLTextAreaElement) => void;
  onSystemCompositionStart: () => void;
  onSystemCompositionEnd: (next: string) => void;
  onUserPromptChange: (next: string, el?: HTMLTextAreaElement) => void;
  onUserCompositionStart: () => void;
  onUserCompositionEnd: (next: string) => void;
  onOutputChange: (next: string) => void;
  onOutputBlur: () => void;
}) {
  const headerActions = (
    <div className={llmNodeStyles.headerActions}>
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsFullScreen(true);
        }}
        className={llmNodeStyles.headerButton}
        title="全屏预览"
      >
        <Maximize2 size={12} />
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsMarkdown((v) => !v);
        }}
        className={llmNodeStyles.headerButton}
        title={isMarkdown ? '切换到编辑模式' : '切换到Markdown预览'}
      >
        {isMarkdown ? <Pencil size={12} /> : <Eye size={12} />}
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsConfigOpen((v) => !v);
        }}
        className={llmNodeStyles.headerButton}
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
          <div className={llmNodeStyles.configOverlay}>
            <div className={llmNodeStyles.configHeader}>
              <div className={llmNodeStyles.configTitleRow}>
                <span>LLM 设置</span>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setIsConfigOpen(false);
                }}
                className={llmNodeStyles.headerButton}
                title="关闭"
              >
                <X size={12} />
              </button>
            </div>
            <div className={llmNodeStyles.configBody}>
              <div className={llmNodeStyles.configRow}>
                <div className="flex items-center gap-2">
                  <span className={llmNodeStyles.configLabel}>Model</span>
                  <select className={llmNodeStyles.select} value={model} onChange={(e) => onModelChange(e.target.value)}>
                    <option value="deepseek-chat">DeepSeek Chat</option>
                    <option value="deepseek-reasoner">DeepSeek Reasoner</option>
                    <option value="google/gemini-3-pro-preview">Gemini 3 Pro preview</option>
                  </select>
                </div>
                <div className="flex items-center gap-2">
                  <span className={llmNodeStyles.configLabel}>Stream</span>
                  <input
                    type="checkbox"
                    className={llmNodeStyles.checkbox}
                    checked={stream}
                    onChange={(e) => onStreamChange(e.target.checked)}
                  />
                </div>
              </div>
              <div className="col-span-12 flex flex-col flex-1">
                <div className={llmNodeStyles.promptLabelRow}>
                  <span>System Prompt</span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setIsSystemFullScreen(true);
                    }}
                    className={llmNodeStyles.headerButton}
                    title="全屏编辑"
                  >
                    <Maximize2 size={12} />
                  </button>
                </div>
                <textarea
                  style={{ boxSizing: 'content-box', height: '100%' }}
                  ref={systemInputRef}
                  className={llmNodeStyles.promptTextarea}
                  value={systemPrompt}
                  onChange={(e) => onSystemPromptChange(e.target.value, e.target)}
                  onCompositionStart={onSystemCompositionStart}
                  onCompositionEnd={(e) => onSystemCompositionEnd(e.currentTarget.value)}
                  onKeyDown={(e) => e.stopPropagation()}
                  onWheel={(e) => e.stopPropagation()}
                  placeholder="System prompt..."
                />
              </div>
            </div>
          </div>
        )}

        {isSystemFullScreen && (
          <FullScreenModal isOpen={isSystemFullScreen} onClose={() => setIsSystemFullScreen(false)} title="System Prompt 编辑">
            <div className="w-full h-full flex flex-1 flex-col p-4 text-zinc-200 text-sm overflow-auto custom-scrollbar bg-black">
              <textarea
                className="flex flex-1 bg-black resize-none focus:outline-none text-zinc-200 font-mono"
                style={{ boxSizing: 'content-box', height: '100%' }}
                value={systemPrompt}
                onChange={(e) => onSystemPromptChange(e.target.value, e.target)}
                onCompositionStart={onSystemCompositionStart}
                onCompositionEnd={(e) => onSystemCompositionEnd(e.currentTarget.value)}
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
            <div className={llmNodeStyles.fullscreenBody}>
              {isMarkdown ? (
                <ThemeProvider theme={theme}>
                  <AutoScrollContainer enabled={data.isOutputStream} className="h-full w-full">
                    <Incremark incremark={incremark} />
                  </AutoScrollContainer>
                </ThemeProvider>
              ) : (
                <textarea
                  className={llmNodeStyles.fullscreenTextarea}
                  value={outputText}
                  onChange={(e) => onOutputChange(e.target.value)}
                  onBlur={onOutputBlur}
                  onWheel={(e) => e.stopPropagation()}
                  placeholder="LLM 输出内容..."
                  spellCheck={false}
                />
              )}
            </div>
          </FullScreenModal>
        )}

        <div className={llmNodeStyles.outputPanel}>
          <div className={llmNodeStyles.outputHeader}>
            <span>LLM 输出</span>
            <span className={llmNodeStyles.outputMode}>{isMarkdown ? 'Markdown' : 'Raw'}</span>
          </div>
          {isMarkdown ? (
            <div className={llmNodeStyles.markdownBox} onKeyDown={(e) => e.stopPropagation()} onWheel={(e) => e.stopPropagation()}>
              <ThemeProvider theme={theme}>
                <AutoScrollContainer enabled={data.isOutputStream} className="h-[300px] p-2">
                  <Incremark incremark={incremark} />
                </AutoScrollContainer>
              </ThemeProvider>
            </div>
          ) : (
            <textarea
              className={llmNodeStyles.rawTextarea}
              value={outputText}
              onChange={(e) => onOutputChange(e.target.value)}
              onBlur={onOutputBlur}
              placeholder="LLM 输出内容..."
              onWheel={(e) => e.stopPropagation()}
              onKeyDown={(e) => e.stopPropagation()}
            />
          )}
        </div>

        <div className={llmNodeStyles.userPromptSection}>
          <div>
            <label className={llmNodeStyles.userPromptLabel}>User Prompt</label>
            <textarea
              style={{ boxSizing: 'content-box' }}
              ref={userInputRef}
              className={llmNodeStyles.userPromptTextarea}
              rows={5}
              value={userPrompt}
              onChange={(e) => onUserPromptChange(e.target.value, e.target)}
              onCompositionStart={onUserCompositionStart}
              onCompositionEnd={(e) => onUserCompositionEnd(e.currentTarget.value)}
              onKeyDown={(e) => e.stopPropagation()}
              onWheel={(e) => e.stopPropagation()}
              placeholder="User prompt template..."
            />
          </div>
        </div>
      </div>
    </NodeWrapper>
  );
}
