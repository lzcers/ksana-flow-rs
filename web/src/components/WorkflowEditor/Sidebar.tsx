import React from 'react';
import { Play, Save, Activity, Box, Database } from 'lucide-react';
import { cn } from '../../utils/cn';
import type { NodeMetadata } from '../../api';

interface SidebarProps {
  nodeTypes: NodeMetadata[];
  onAddNode: (type: string) => void;
  onRun: () => void;
}

const getIconForCategory = (category: string) => {
  switch (category.toLowerCase()) {
    case 'source': return Database;
    case 'strategy': return Activity;
    case 'sink': return Box;
    default: return Box;
  }
};

const getColorForCategory = (category: string) => {
  switch (category.toLowerCase()) {
    case 'source': return 'bg-blue-100 text-blue-600';
    case 'strategy': return 'bg-purple-100 text-purple-600';
    case 'sink': return 'bg-orange-100 text-orange-600';
    default: return 'bg-slate-100 text-slate-600';
  }
};

export const Sidebar: React.FC<SidebarProps> = ({ nodeTypes, onAddNode, onRun }) => {
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
        <h2 className="text-[11px] font-bold text-slate-400 uppercase tracking-[0.1em]">Components</h2>
        <div className="space-y-1.5">
          {nodeTypes.map(nodeType => {
            const Icon = getIconForCategory(nodeType.category);
            const colorClass = getColorForCategory(nodeType.category);
            return (
              <button
                key={nodeType.name}
                onClick={() => onAddNode(nodeType.name)}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData('application/reactflow', nodeType.name);
                  e.dataTransfer.effectAllowed = 'move';
                }}
                className="w-full flex items-center gap-3 p-2 rounded-lg hover:bg-slate-50 transition-colors text-left group cursor-grab active:cursor-grabbing"
              >
                <div className={cn("p-1.5 rounded-md transition-colors", colorClass)}>
                  <Icon size={16} />
                </div>
                <div className="flex flex-col">
                  <span className="text-sm font-medium text-slate-600 group-hover:text-slate-900">
                    {nodeType.name}
                  </span>
                  <span className="text-[10px] text-slate-400">{nodeType.category}</span>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      <div className="mt-auto">
        <button
          onClick={onRun}
          className="w-full flex items-center justify-center gap-2 bg-slate-900 text-white py-2.5 rounded-lg text-sm font-medium hover:bg-slate-800 transition-all active:scale-[0.98]"
        >
          <Play size={16} />
          Run Workflow
        </button>
      </div>
    </aside>
  );
};
