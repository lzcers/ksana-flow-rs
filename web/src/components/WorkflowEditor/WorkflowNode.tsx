import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Settings } from 'lucide-react';
import { NODE_TYPES } from '../../constants/nodeTypes';
import { cn } from '../../utils/cn';
import type { WorkflowNodeData } from '../../types/workflow';

export const WorkflowNode = memo(({ data, selected }: NodeProps & { data: WorkflowNodeData }) => {
  const typeConfig = NODE_TYPES.find(t => t.type === data.type);

  return (
    <div
      className={cn(
        "w-40 bg-white border transition-all duration-200",
        selected 
          ? "border-slate-900 shadow-xl shadow-slate-200/50 scale-[1.02]" 
          : "border-slate-200 hover:border-slate-300 shadow-sm"
      )}
      style={{ borderRadius: '8px' }}
    >
      {/* Input Handle for all except 'start' */}
      {data.type !== 'start' && (
        <Handle
          type="target"
          position={Position.Top}
          className="w-2 h-2 !bg-slate-300 border-white"
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
        <div className="text-sm font-semibold text-slate-800 truncate leading-tight">
          {data.label}
        </div>
      </div>
      
      {data.description && (
        <div className="px-3 pb-3 text-[11px] text-slate-400 truncate border-t border-slate-50 pt-2 mt-1">
          {data.description}
        </div>
      )}

      {/* Output Handle for all except 'end' */}
      {data.type !== 'end' && (
        <Handle
          type="source"
          position={Position.Bottom}
          className="w-2 h-2 !bg-slate-300 border-white"
        />
      )}
    </div>
  );
});

WorkflowNode.displayName = 'WorkflowNode';
