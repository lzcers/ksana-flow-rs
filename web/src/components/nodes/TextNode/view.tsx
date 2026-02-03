import { AutoScrollContainer, Incremark, ThemeProvider } from '@incremark/react';
import { Maximize2, Eye, Pencil } from 'lucide-react';
import { Position, type NodeProps } from '@xyflow/react';
import { NodeWrapper } from '../shared/NodeWrapper';
import { FullScreenModal } from '../../ui/FullScreenModal';
import { theme } from './theme';
import type { NodeData } from '@/model/workflow/types';
import { textNodeStyles } from './styles';

const SOURCE_HANDLES = [Position.Right];
const TARGET_HANDLES = [Position.Left, Position.Top, Position.Bottom];

export function TextNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  text,
  isMarkdown,
  isFullScreen,
  onToggleMarkdown,
  onOpenFullScreen,
  onCloseFullScreen,
  incremark,
  onTextChange,
  onTextBlur,
}: NodeProps & {
  data: NodeData;
} & {
  text: string;
  isMarkdown: boolean;
  isFullScreen: boolean;
  onToggleMarkdown: () => void;
  onOpenFullScreen: () => void;
  onCloseFullScreen: () => void;
  incremark: any;
  onTextChange: (next: string) => void;
  onTextBlur: () => void;
}) {
  const headerActions = (
    <div className={textNodeStyles.headerActions}>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onOpenFullScreen();
        }}
        className={textNodeStyles.headerButton}
        title="Full Screen"
      >
        <Maximize2 size={12} />
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onToggleMarkdown();
        }}
        className={textNodeStyles.headerButton}
        title={isMarkdown ? 'Switch to Edit Mode' : 'Switch to Markdown Preview'}
      >
        {isMarkdown ? <Pencil size={12} /> : <Eye size={12} />}
      </button>
    </div>
  );

  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      sourceHandles={SOURCE_HANDLES}
      targetHandles={TARGET_HANDLES}
      className={textNodeStyles.wrapperClass}
      minWidth={260}
      minHeight={200}
      style={{ width, height }}
      headerActions={headerActions}
    >
      <div className={textNodeStyles.container}>
        <div className={textNodeStyles.headerRow}>
          <span>Text Content</span>
          <span className={textNodeStyles.modeHint}>{isMarkdown ? 'Markdown' : 'Raw'}</span>
        </div>

        {isFullScreen && (
          <FullScreenModal
            isOpen={isFullScreen}
            onClose={onCloseFullScreen}
            title={isMarkdown ? 'Markdown Preview' : 'Text Content'}
          >
            <div className={textNodeStyles.fullscreenContainer}>
              {isMarkdown ? (
                <ThemeProvider theme={theme}>
                  <AutoScrollContainer enabled={data.upstreamIsStreaming} className="h-full w-full">
                    <Incremark incremark={incremark} />
                  </AutoScrollContainer>
                </ThemeProvider>
              ) : (
                <textarea
                  className={textNodeStyles.fullscreenTextarea}
                  value={text}
                  onChange={(e) => onTextChange(e.target.value)}
                  placeholder="Enter text here..."
                  spellCheck={false}
                />
              )}
            </div>
          </FullScreenModal>
        )}

        {isMarkdown ? (
          <div
            className={textNodeStyles.previewBox}
            onKeyDown={(e) => e.stopPropagation()}
            onWheel={(e) => e.stopPropagation()}
          >
            <ThemeProvider theme={theme}>
              <AutoScrollContainer enabled className="h-[300px] p-2">
                <Incremark incremark={incremark} />
              </AutoScrollContainer>
            </ThemeProvider>
          </div>
        ) : (
          <textarea
            className={textNodeStyles.editTextarea}
            value={text}
            onChange={(e) => onTextChange(e.target.value)}
            onBlur={onTextBlur}
            placeholder="Enter text here..."
            onWheel={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
          />
        )}
      </div>
    </NodeWrapper>
  );
}
