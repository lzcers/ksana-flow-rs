import React from 'react';
import { Play, Save } from 'lucide-react';
import { NODE_TYPES } from '../../constants/nodeTypes';
import { cn } from '../../utils/cn';
import type { NodeType } from '../../types/workflow';

interface SidebarProps {
  onAddNode: (type: NodeType) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ onAddNode }) => {
  return (
    <aside className="w-56 border-r border-slate-100 bg-white p-6 z-10 flex flex-col">
      <div className="mb-10">
        <h1 className="text-lg font-bold tracking-tight text-slate-900 flex items-center gap-2">
          <div className="w-6 h-6 bg-slate-900 rounded flex items-center justify-center text-white">
            <Play size={12} fill="currentColor" />
          </div>
          Ksana Flow
        </h1>
      </div>

      <div className="space-y-6">
        <h2 className="text-[11px] font-bold text-slate-400 uppercase tracking-[0.1em]">组件</h2>
        <div className="space-y-1.5">
          {NODE_TYPES.map(nodeType => (
            <button
              key={nodeType.type}
              onClick={() => onAddNode(nodeType.type)}
              className="w-full flex items-center gap-3 p-2 rounded-lg hover:bg-slate-50 transition-colors text-left group"
            >
              <div className={cn("p-1.5 rounded-md transition-colors", nodeType.color)}>
                <nodeType.icon size={16} />
              </div>
              <span className="text-sm font-medium text-slate-600 group-hover:text-slate-900">
                {nodeType.label}
              </span>
            </button>
          ))}
        </div>
      </div>

      <div className="mt-auto">
        <button className="w-full flex items-center justify-center gap-2 bg-slate-900 text-white py-2.5 rounded-lg text-sm font-medium hover:bg-slate-800 transition-all active:scale-[0.98]">
          <Save size={16} />
          保存
        </button>
      </div>
    </aside>
  );
};
