import React from 'react';
import { Handle, NodeResizeControl, Position } from '@xyflow/react';
import { Play } from 'lucide-react';
import { cn } from '@/utils/cn';
import type { NodeData } from '@/model/types';
import { useStore } from '@/store';
import { nodeWrapperStyles } from './styles';
import { useNodeLabel } from './useNodeLabel';
import '../node.css';

interface NodeWrapperProps {
  id: string;
  type: string;
  data: NodeData;
  selected: boolean;
  sourceHandles?: Position[];
  targetHandles?: Position[];
  children?: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
  resizable?: boolean;
  minWidth?: number;
  minHeight?: number;
  keepAspectRatio?: boolean;
  headerActions?: React.ReactNode;
}

const HANDLE_STYLES: Record<Position, React.CSSProperties> = {
  [Position.Top]: { top: -6, left: '50%', transform: 'translateX(-50%)' },
  [Position.Bottom]: { bottom: -6, left: '50%', transform: 'translateX(-50%)' },
  [Position.Left]: { left: -6, top: '50%', transform: 'translateY(-50%)' },
  [Position.Right]: { right: -6, top: '50%', transform: 'translateY(-50%)' },
};

export const NodeWrapper: React.FC<NodeWrapperProps> = ({
  id,
  data,
  selected,
  sourceHandles = [],
  targetHandles = [],
  children,
  className,
  minWidth,
  minHeight,
  keepAspectRatio = false,
  style,
  resizable = true,
  headerActions,
}) => {
  const status = data.status || 'idle';
  const {
    runNode,
    updateNodeDimensions,
    isConnecting,
    connectionSourceId,
    workflowStatus,
    updateNodeData,
  } = useStore();

  const { editingLabel, setEditingLabel, labelDraft, setLabelDraft, inputRef, commitLabel, cancelLabel } =
    useNodeLabel({
      id,
      label: data.label,
      updateNodeData,
    });

  const handleRun = (e: React.MouseEvent) => {
    e.stopPropagation();
    runNode(id);
  };

  return (
    <div
      className={cn(nodeWrapperStyles.root, 'w-full h-full')}
      style={{
        minWidth: minWidth ?? 'fit-content',
        minHeight: minHeight ?? 'fit-content',
        ...style,
      }}
    >
      <div className={nodeWrapperStyles.header}>
        <div className={nodeWrapperStyles.headerLeft}>
          <div className={nodeWrapperStyles.headerDot} />
          {editingLabel ? (
            <input
              ref={inputRef}
              value={labelDraft}
              onChange={(e) => setLabelDraft(e.target.value)}
              onBlur={(e) => {
                e.stopPropagation();
                commitLabel();
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  commitLabel();
                } else if (e.key === 'Escape') {
                  e.preventDefault();
                  cancelLabel();
                }
              }}
              onPointerDown={(e) => e.stopPropagation()}
              className={nodeWrapperStyles.labelInput}
              placeholder="请输入标签"
            />
          ) : (
            <span
              className={nodeWrapperStyles.headerLabel}
              onDoubleClick={(e) => {
                e.stopPropagation();
                setEditingLabel(true);
              }}
            >
              {data.label}
            </span>
          )}
        </div>

        <div
          className={cn(
            nodeWrapperStyles.headerRight,
            selected
              ? 'opacity-100 pointer-events-auto'
              : 'opacity-0 group-hover:opacity-100 pointer-events-none group-hover:pointer-events-auto',
          )}
        >
          {headerActions}
          {workflowStatus === 'idle' && (
            <button onClick={handleRun} className={nodeWrapperStyles.runButton} title="Run Node">
              <Play size={12} fill="currentColor" className="ml-0.5 opacity-80" />
            </button>
          )}
        </div>
      </div>

      <div
        className={cn(
          nodeWrapperStyles.cardBase,
          status === 'running'
            ? 'node-running'
            : selected
              ? nodeWrapperStyles.cardSelected
              : nodeWrapperStyles.cardIdle,
          className,
        )}
      >
        {resizable && (
          <NodeResizeControl
            minWidth={minWidth ?? 100}
            minHeight={minHeight ?? 50}
            keepAspectRatio={keepAspectRatio}
            position="bottom-right"
            className={cn(
              nodeWrapperStyles.resizeControlBase,
              selected ? 'opacity-100' : nodeWrapperStyles.resizeControlHidden,
            )}
            onResizeEnd={(_event, params) => {
              updateNodeDimensions(id, params.width, params.height);
            }}
          >
            <div className={nodeWrapperStyles.resizeHandle}>
              <svg
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
                className={nodeWrapperStyles.resizeIcon}
              >
                <path d="M 18 6 C 18 14 16 18 6 18" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
              </svg>
            </div>
          </NodeResizeControl>
        )}

        {targetHandles.map((position) => (
          <Handle
            key={`target-${position}`}
            type="target"
            position={position}
            id={`t-${position}`}
            className={cn(
              nodeWrapperStyles.handleTargetBase,
              isConnecting && id !== connectionSourceId ? 'opacity-100' : 'opacity-0 group-hover:opacity-100',
            )}
            style={HANDLE_STYLES[position]}
          />
        ))}

        {sourceHandles.map((position) => (
          <Handle
            key={`source-${position}`}
            type="source"
            position={position}
            id={`s-${position}`}
            className={cn(
              nodeWrapperStyles.handleSourceBase,
              (!isConnecting || id === connectionSourceId) && (selected ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'),
            )}
            style={HANDLE_STYLES[position]}
          />
        ))}

        <div className={nodeWrapperStyles.contentArea} style={{ borderRadius: '12px' }}>
          {children}

          {data.errorMessage && <div className={nodeWrapperStyles.errorToast}>{data.errorMessage}</div>}
        </div>
      </div>
    </div>
  );
};
