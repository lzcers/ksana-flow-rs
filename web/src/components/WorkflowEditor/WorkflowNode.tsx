import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Settings, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';
import { NODE_TYPES } from '../../constants/nodeTypes';
import { cn } from '../../utils/cn';
import type { WorkflowNodeData } from '../../types/workflow';

export const WorkflowNode = memo(({ data, selected }: NodeProps & { data: WorkflowNodeData }) => {
  const typeConfig = NODE_TYPES.find(t => t.type === data.type);
  const status = data.status || 'idle';

  return (
    <div
      className={cn(
        "min-w-[120px] max-w-[280px] bg-white border transition-all duration-300 group relative",
        selected
          ? "border-blue-500 shadow-xl shadow-blue-100 scale-[1.02] ring-1 ring-blue-500"
          : "border-slate-200 hover:border-slate-300 shadow-sm",
        status === 'running' && "border-yellow-400 shadow-lg shadow-yellow-100 ring-1 ring-yellow-400 animate-pulse",
        status === 'completed' && "border-green-500 shadow-md shadow-green-50, border-2",
        status === 'error' && "border-red-500 shadow-lg shadow-red-100 ring-1 ring-red-500 animate-shake"
      )}
      style={{ borderRadius: '8px' }}
    >
      {/* Status Indicator Overlay */}
      <div className="absolute -top-2 -right-2 z-10">
        {status === 'running' && (
          <div className="bg-yellow-400 text-white rounded-full p-0.5 shadow-sm animate-spin-slow">
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

      {/* Handles for all four sides - Centered */}
      {/* Top Handle */}
      {data.type !== 'start' && (
        <Handle
          type="target"
          position={Position.Top}
          id="t-top"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}
      {data.type !== 'end' && (
        <Handle
          type="source"
          position={Position.Top}
          id="s-top"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}

      {/* Bottom Handle */}
      {data.type !== 'start' && (
        <Handle
          type="target"
          position={Position.Bottom}
          id="t-bottom"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}
      {data.type !== 'end' && (
        <Handle
          type="source"
          position={Position.Bottom}
          id="s-bottom"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}

      {/* Left Handle */}
      {data.type !== 'start' && (
        <Handle
          type="target"
          position={Position.Left}
          id="t-left"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}
      {data.type !== 'end' && (
        <Handle
          type="source"
          position={Position.Left}
          id="s-left"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}

      {/* Right Handle */}
      {data.type !== 'start' && (
        <Handle
          type="target"
          position={Position.Right}
          id="t-right"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}
      {data.type !== 'end' && (
        <Handle
          type="source"
          position={Position.Right}
          id="s-right"
          className="opacity-0 group-hover:opacity-100 transition-opacity"
        />
      )}

      <div className="p-3">
        <div className="flex items-center gap-2 mb-2">
          <div className={cn("p-1 rounded-md", typeConfig?.color)}>
            {React.createElement(typeConfig?.icon || Settings, { size: 12 })}
          </div>
          <span className="text-[10px] font-bold text-slate-400 uppercase tracking-wider">
            {data.type}
          </span>
        </div>
        <div className="text-sm font-semibold text-slate-800 break-words leading-tight mb-1">
          {data.label}
        </div>
        {data.errorMessage && (
          <div className="text-[10px] text-red-500 line-clamp-2 mt-1 bg-red-50 p-1 rounded border border-red-100">
            {data.errorMessage}
          </div>
        )}
      </div>

      {data.description && (
        <div className="px-3 pb-3 text-[11px] text-slate-400 break-words border-t border-slate-50 pt-2 mt-1">
          {data.description}
        </div>
      )}
    </div>
  );
});

WorkflowNode.displayName = 'WorkflowNode';
