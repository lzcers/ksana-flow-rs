import React from 'react';
import { Handle, Position, NodeResizeControl } from '@xyflow/react';
import { Settings, Play } from 'lucide-react';
import { NODE_TYPES } from '../WorkflowEditor/nodeTypes';
import { cn } from '../../utils/cn';
import type { NodeData } from '../../model/types';
import { useStore } from '../../store';
import './index.css';

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
  type,
  data,
  selected,
  sourceHandles = [],
  targetHandles = [],
  children,
  className,
  minWidth,
  minHeight,
  style,
  resizable = true,
  headerActions
}) => {
  const status = data.status || 'idle';
  const { runNode, updateNodeDimensions, isConnecting, connectionSourceId } = useStore();

  const handleRun = (e: React.MouseEvent) => {
    e.stopPropagation();
    runNode(id);
  };
  return (
    <div
      className={cn("relative group", resizable && "w-full h-full")}
      style={{
        minWidth: minWidth ?? 'fit-content',
        minHeight: minHeight ?? 'fit-content',
        ...style
      }}
    >
      {/* Header Info - Floating above the node */}
      <div
        className={cn(
          "absolute -top-9 left-0 w-full flex items-center justify-between transition-all duration-300 z-10",
          selected ? "opacity-100 pointer-events-auto" : "opacity-0 group-hover:opacity-100 pointer-events-none group-hover:pointer-events-auto"
        )}
      >
        <div className="flex items-center gap-2">
          {/* Dot Indicator */}
          <div className="w-2 h-2 rounded-full bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.8)]"></div>
          {/* Title */}
          <span className="text-sm font-bold text-zinc-200 tracking-wide drop-shadow-md">
            {data.label}
          </span>
        </div>

        <div className="flex items-center gap-2">
          {headerActions}
          {/* Run Button */}
          <button
            onClick={handleRun}
            className="group/run flex items-center justify-center w-7 h-7 backdrop-blur-xl bg-white/5 hover:bg-blue-500/40 text-zinc-200 hover:text-white rounded-full border border-white/10 hover:border-blue-400/50 shadow-[0_4px_12px_rgba(0,0,0,0.3)] hover:shadow-[0_0_15px_rgba(59,130,246,0.5)] hover:scale-110 active:scale-95 transition-all duration-300"
            title="Run Node"
          >
            <Play size={12} fill="currentColor" className="ml-0.5" />
          </button>
        </div>
      </div>

      {/* Main Content Card */}
      <div
        className={cn(
          "w-full h-full flex-1 bg-zinc-900 border transition-all duration-300 relative rounded-xl",
          selected
            ? "border-blue-500/50 shadow-[0_0_20px_rgba(59,130,246,0.15)] ring-1 ring-blue-500/20"
            : "border-zinc-800 hover:border-zinc-700 shadow-lg shadow-black/40",
          status === 'running' && "ring-1 ring-blue-500/50 border-blue-500/50",
          className
        )}
      >
        {/* Resize Control */}
        {resizable && (
          <NodeResizeControl
            minWidth={minWidth ?? 100}
            minHeight={minHeight ?? 50}
            position="bottom-right"
            className={cn(
              "!bg-transparent !border-none z-50",
              selected ? "opacity-100" : "opacity-0 group-hover:opacity-100 transition-opacity duration-300"
            )}
            onResizeEnd={(_event, params) => {
              updateNodeDimensions(id, params.width, params.height);
            }}
          >
            <div className="absolute -bottom-3 -right-3 p-2 cursor-nwse-resize group/resize">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className="text-zinc-600 group-hover/resize:text-blue-500 transition-colors">
                <path d="M 18 6 C 18 14 16 18 6 18" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
              </svg>
            </div>
          </NodeResizeControl>
        )}

        {/* Target Handles */}
        {targetHandles.map((position) => (
          <Handle
            key={`target-${position}`}
            type="target"
            position={position}
            id={`t-${position}`}
            className={cn(
              "!w-3.5 !h-3.5 !bg-zinc-950 !border-[1.5px] !border-zinc-500 hover:!border-blue-500 hover:!bg-zinc-900 !rounded-full transition-all duration-200 z-50",
              isConnecting && id !== connectionSourceId ? "opacity-100" : "opacity-0 group-hover:opacity-100"
            )}
            style={HANDLE_STYLES[position]}
          />
        ))}

        {/* Source Handles */}
        {sourceHandles.map((position) => (
          <Handle
            key={`source-${position}`}
            type="source"
            position={position}
            id={`s-${position}`}
            className={cn(
              "!w-3.5 !h-3.5 !bg-zinc-400 hover:!bg-blue-400 !border-[1.5px] !border-zinc-950 !rounded-full transition-all duration-200 z-50",
              (!isConnecting || id === connectionSourceId) && (selected ? "opacity-100" : "opacity-0 group-hover:opacity-100")
            )}
            style={HANDLE_STYLES[position]}
          />
        ))}

        {/* Content Area */}
        <div className="w-full h-full overflow-hidden" style={{ borderRadius: '12px' }}>
          {children}

          {data.errorMessage && (
            <div className="absolute bottom-3 left-3 right-8 text-[10px] text-red-400 bg-red-950/80 p-1.5 rounded-md border border-red-900/50 truncate backdrop-blur-sm">
              {data.errorMessage}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
