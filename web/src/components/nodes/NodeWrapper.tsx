import React from 'react';
import { Handle, Position, NodeResizer } from '@xyflow/react';
import { Settings, Play } from 'lucide-react';
import { NODE_TYPES } from '../WorkflowEditor/nodeTypes';
import { cn } from '../../utils/cn';
import type { NodeData } from '../../model/types';
import { useStore } from '../../store';
import './index.css';

interface NodeWrapperProps {
  id: string;
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
  data,
  selected,
  sourceHandles = [],
  targetHandles = [],
  children,
  className,
  style,
  resizable = true,
  minWidth,
  minHeight,
  headerActions
}) => {
  const typeConfig = NODE_TYPES.find(t => t.type === data.type);
  const status = data.status || 'idle';
  const { runNode, updateNodeDimensions, isConnecting, connectionSourceId } = useStore();

  const handleRun = (e: React.MouseEvent) => {
    e.stopPropagation();
    runNode(id);
  };

  return (
    <div
      className={cn(
        "bg-zinc-900/95 border transition duration-300 group relative",
        selected
          ? "border-blue-500 shadow-[0_0_15px_rgba(59,130,246,0.5)] scale-[1.02] ring-1 ring-blue-500"
          : "border-zinc-700 hover:border-zinc-500 shadow-lg shadow-black/20",
        status === 'running' && "node-running",
        resizable && "max-w-none w-full h-full",
        className
      )}
      style={{
        borderRadius: '8px',
        minWidth: minWidth ?? 'fit-content',
        minHeight: minHeight ?? 'fit-content',
        ...style
      }}
    >
      {resizable && (
        <NodeResizer
          minWidth={minWidth ?? 0}
          minHeight={minHeight ?? 0}
          isVisible={selected}
          lineClassName="border-blue-500"
          handleClassName="h-3 w-3 bg-white border-2 border-blue-500 rounded"
          onResizeEnd={(_event, params) => {
            updateNodeDimensions(id, params.width, params.height);
          }}
        />
      )}

      {/* Target Handles */}
      {targetHandles.map((position) => (
        <Handle
          key={`target-${position}`}
          type="target"
          position={position}
          id={`t-${position}`}
          className={cn(
            "!w-3 !h-3 !bg-zinc-900 !border-2 !border-blue-500 !rounded-full transition-opacity duration-200 z-50",
            isConnecting && id !== connectionSourceId ? "opacity-100" : "opacity-0 pointer-events-none"
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
            "!w-3 !h-3 !bg-blue-500 !border-2 !border-white !rounded-full transition-opacity duration-200 z-50",
            (!isConnecting || id === connectionSourceId) && (selected ? "opacity-100" : "opacity-0 group-hover:opacity-100")
          )}
          style={HANDLE_STYLES[position]}
        />
      ))}

      <div className="p-2">
        <div className="flex items-center gap-2">
          <div className={cn("p-1 rounded-md bg-zinc-800 border border-zinc-700 text-zinc-400")}>
            {React.createElement(typeConfig?.icon || Settings, { size: 12 })}
          </div>
          <span className="text-xs font-semibold text-zinc-200">
            {data.label}
          </span>
          <div className="ml-auto flex items-center gap-2">
            {headerActions}
            {/* Run Button */}
            <button
              onClick={handleRun}
              className={cn(
                "bg-blue-600 text-white rounded-full p-1 shadow-sm hover:bg-blue-500 transition-all opacity-0 group-hover:opacity-100",
                selected && "opacity-100"
              )}
              title="Run from this node"
            >
              <Play size={10} fill="currentColor" />
            </button>
          </div>
        </div>
        {data.errorMessage && (
          <div className="text-[10px] text-red-400 line-clamp-2 mt-1 bg-red-900/20 p-1 rounded border border-red-900/30">
            {data.errorMessage}
          </div>
        )}
      </div>

      {children}
    </div>
  );
};
