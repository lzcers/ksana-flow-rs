import React from 'react';
import { Trash2 } from 'lucide-react';
import type { WorkflowNode, WorkflowNodeData } from '../../types/workflow';

interface PropertyPanelProps {
  node: WorkflowNode;
  onUpdateData: (id: string, data: Partial<WorkflowNodeData>) => void;
  onDelete: (id: string) => void;
}

export const PropertyPanel: React.FC<PropertyPanelProps> = ({ 
  node, 
  onUpdateData, 
  onDelete 
}) => {
  return (
    <aside className="w-72 border-l border-slate-100 bg-white p-6 z-10 overflow-y-auto">
      <div className="flex items-center justify-between mb-8">
        <h2 className="text-sm font-bold text-slate-900 uppercase tracking-widest">属性</h2>
        <button 
          onClick={() => onDelete(node.id)}
          className="p-1.5 text-slate-300 hover:text-rose-500 transition-colors"
        >
          <Trash2 size={16} />
        </button>
      </div>

      <div className="space-y-6">
        <div className="space-y-2">
          <label className="text-[11px] font-bold text-slate-400 uppercase">名称</label>
          <input
            type="text"
            value={node.data.label}
            onChange={(e) => onUpdateData(node.id, { label: e.target.value })}
            className="w-full text-sm p-2 bg-slate-50 border-none rounded-md focus:ring-1 focus:ring-slate-200 transition-all outline-none"
          />
        </div>

        <div className="space-y-2">
          <label className="text-[11px] font-bold text-slate-400 uppercase">描述</label>
          <textarea
            value={node.data.description || ''}
            onChange={(e) => onUpdateData(node.id, { description: e.target.value })}
            rows={4}
            className="w-full text-sm p-2 bg-slate-50 border-none rounded-md focus:ring-1 focus:ring-slate-200 transition-all outline-none resize-none"
          />
        </div>

        <div className="pt-6 border-t border-slate-100">
          <p className="text-[10px] text-slate-400 italic">
            提示: 通过节点底部的连接点拖拽来建立连接。
          </p>
        </div>
      </div>
    </aside>
  );
};
