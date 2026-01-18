import React from 'react';
import { Handle, Position, NodeResizer } from '@xyflow/react';
import { Settings, CheckCircle2, AlertCircle, Loader2, Play } from 'lucide-react';
import { NODE_TYPES } from '../WorkflowEditor/nodeTypes';
import { cn } from '../../utils/cn';
import type { WorkflowNodeData } from '../../model/types';
import { useWorkflowContext } from '../../contexts/WorkflowContext';

interface NodeWrapperProps {
  id: string;
  data: WorkflowNodeData;
  selected: boolean;
  showSourceHandle?: boolean;
  showTargetHandle?: boolean;
  children?: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
  resizable?: boolean;
  minWidth?: number;
  minHeight?: number;
}

export const NodeWrapper: React.FC<NodeWrapperProps> = ({
  id,
  data,
  selected,
  showSourceHandle = true,
  showTargetHandle = true,
  children,
  className,
  style,
  resizable = true,
  minWidth,
  minHeight
}) => {
  const typeConfig = NODE_TYPES.find(t => t.type === data.type);
  const status = data.status || 'idle';
  const { runNode, updateNodeDimensions } = useWorkflowContext();
  const handleRun = (e: React.MouseEvent) => {
    e.stopPropagation();
    runNode(id);
  };

  return (
    <div
      className={cn(
        "bg-zinc-900/90 backdrop-blur border transition-all duration-300 group relative",
        selected
          ? "border-blue-500 shadow-[0_0_15px_rgba(59,130,246,0.5)] scale-[1.02] ring-1 ring-blue-500"
          : "border-zinc-700 hover:border-zinc-500 shadow-lg shadow-black/20",
        status === 'running' && "border-yellow-400 shadow-[0_0_15px_rgba(250,204,21,0.5)] ring-1 ring-yellow-400 animate-pulse",
        status === 'completed' && "border-green-500 shadow-[0_0_15px_rgba(34,197,94,0.5)] border-2",
        status === 'error' && "border-red-500 shadow-[0_0_15px_rgba(239,68,68,0.5)] ring-1 ring-red-500 animate-shake",
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
      {/* Status Indicator Overlay */}
      <div className="absolute -top-2 -right-2 z-10 flex gap-1">
        {status === 'running' && (
          <div className="bg-yellow-500 text-white rounded-full p-0.5 shadow-sm animate-spin-slow">
            <Loader2 size={14} />
          </div>
        )}
        {status === 'completed' && (
          <div className="bg-green-500 text-white rounded-full shadow-sm">
            <CheckCircle2 size={16} />
          </div>
        )}
        {status === 'error' && (
          <div className="bg-red-500 text-white rounded-full shadow-sm">
            <AlertCircle size={16} />
          </div>
        )}
      </div>

      {/* Handles */}
      {showTargetHandle && (
        <>
          <Handle
            type="target"
            position={Position.Top}
            id="t-top"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
          <Handle
            type="target"
            position={Position.Bottom}
            id="t-bottom"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
          <Handle
            type="target"
            position={Position.Left}
            id="t-left"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
          <Handle
            type="target"
            position={Position.Right}
            id="t-right"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
        </>
      )}

      {showSourceHandle && (
        <>
          <Handle
            type="source"
            position={Position.Top}
            id="s-top"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
          <Handle
            type="source"
            position={Position.Bottom}
            id="s-bottom"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
          <Handle
            type="source"
            position={Position.Left}
            id="s-left"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
          <Handle
            type="source"
            position={Position.Right}
            id="s-right"
            className="opacity-0 group-hover:opacity-100 transition-opacity"
          />
        </>
      )}

      <div className="p-2">
        <div className="flex items-center gap-2">
          <div className={cn("p-1 rounded-md bg-zinc-800 border border-zinc-700 text-zinc-400")}>
            {React.createElement(typeConfig?.icon || Settings, { size: 12 })}
          </div>
          <span className="text-xs font-semibold text-zinc-200">
            {data.label}
          </span>
          {/* Run Button */}
          <button
            onClick={handleRun}
            className={cn(
              "ml-auto bg-blue-600 text-white rounded-full p-1 shadow-sm hover:bg-blue-500 transition-all opacity-0 group-hover:opacity-100",
              selected && "opacity-100"
            )}
            title="Run from this node"
          >
            <Play size={10} fill="currentColor" />
          </button>
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
