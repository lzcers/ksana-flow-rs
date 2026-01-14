import React from 'react';
import { Handle, Position } from '@xyflow/react';
import { Settings, CheckCircle2, AlertCircle, Loader2, Play } from 'lucide-react';
import { NODE_TYPES } from '../../constants/nodeTypes';
import { cn } from '../../utils/cn';
import type { WorkflowNodeData } from '../../types/workflow';
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
}

export const NodeWrapper: React.FC<NodeWrapperProps> = ({
  id,
  data,
  selected,
  showSourceHandle = true,
  showTargetHandle = true,
  children,
  className,
  style
}) => {
  const typeConfig = NODE_TYPES.find(t => t.type === data.type);
  const status = data.status || 'idle';
  const { runNode } = useWorkflowContext();

  const handleRun = (e: React.MouseEvent) => {
    e.stopPropagation();
    runNode(id);
  };

  return (
    <div
      className={cn(
        "min-w-[120px] max-w-[280px] bg-zinc-900/90 backdrop-blur border transition-all duration-300 group relative",
        selected
          ? "border-blue-500 shadow-[0_0_15px_rgba(59,130,246,0.5)] scale-[1.02] ring-1 ring-blue-500"
          : "border-zinc-700 hover:border-zinc-500 shadow-lg shadow-black/20",
        status === 'running' && "border-yellow-400 shadow-[0_0_15px_rgba(250,204,21,0.5)] ring-1 ring-yellow-400 animate-pulse",
        status === 'completed' && "border-green-500 shadow-[0_0_15px_rgba(34,197,94,0.5)] border-2",
        status === 'error' && "border-red-500 shadow-[0_0_15px_rgba(239,68,68,0.5)] ring-1 ring-red-500 animate-shake",
        className
      )}
      style={{ borderRadius: '8px', ...style }}
    >
      {/* Status Indicator Overlay */}
      <div className="absolute -top-2 -right-2 z-10 flex gap-1">
        {/* Run Button - Only visible on hover or selected */}
        <button
          onClick={handleRun}
          className={cn(
            "bg-blue-600 text-white rounded-full p-1 shadow-lg shadow-blue-900/50 hover:bg-blue-500 transition-all opacity-0 group-hover:opacity-100",
            selected && "opacity-100"
          )}
          title="Run from this node"
        >
          <Play size={12} fill="currentColor" />
        </button>

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

      <div className="p-3">
        <div className="flex items-center gap-2 mb-2">
          <div className={cn("p-1 rounded-md bg-zinc-800 border border-zinc-700 text-zinc-400")}>
            {React.createElement(typeConfig?.icon || Settings, { size: 12 })}
          </div>
          <span className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider">
            {data.type}
          </span>
        </div>
        <div className="text-sm font-semibold text-zinc-100 break-words leading-tight mb-1">
          {data.label}
        </div>
        {data.errorMessage && (
          <div className="text-[10px] text-red-400 line-clamp-2 mt-1 bg-red-900/20 p-1 rounded border border-red-900/30">
            {data.errorMessage}
          </div>
        )}
      </div>

      {children}

      {data.description && (
        <div className="px-3 pb-3 text-[11px] text-zinc-500 break-words border-t border-zinc-800 pt-2 mt-1">
          {data.description}
        </div>
      )}
    </div>
  );
};
