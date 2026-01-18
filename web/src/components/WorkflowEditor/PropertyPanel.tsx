import React from 'react';
import { Trash2 } from 'lucide-react';
import type { WorkflowNode, WorkflowNodeData } from '../../model/types';

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
    <aside className="w-72 border-l border-zinc-800 bg-zinc-900 p-6 z-10 overflow-y-auto">
      <div className="flex items-center justify-between mb-8">
        <h2 className="text-sm font-bold text-zinc-100 tracking-widest">属性</h2>
        <button
          onClick={() => onDelete(node.id)}
          className="p-1.5 text-zinc-500 hover:text-rose-500 transition-colors"
        >
          <Trash2 size={16} />
        </button>
      </div>

      <div className="space-y-6">
        <div className="space-y-2">
          <label className="text-[11px] font-bold text-zinc-500 ">名称</label>
          <input
            type="text"
            value={node.data.label}
            onChange={(e) => onUpdateData(node.id, { label: e.target.value })}
            className="w-full text-sm p-2 bg-zinc-950 border border-zinc-800 rounded-md focus:ring-1 focus:ring-blue-500/50 transition-all outline-none text-zinc-200"
          />
        </div>

        <div className="space-y-2">
          <label className="text-[11px] font-bold text-zinc-500">描述</label>
          <textarea
            value={node.data.description || ''}
            onChange={(e) => onUpdateData(node.id, { description: e.target.value })}
            rows={4}
            className="w-full text-sm p-2 bg-zinc-950 border border-zinc-800 rounded-md focus:ring-1 focus:ring-blue-500/50 transition-all outline-none resize-none text-zinc-200"
          />
        </div>

        {node.data.type === 'LLMNode' && (
          <>
            <div className="space-y-2">
              <label className="text-[11px] font-bold text-zinc-500">System Prompt</label>
              <textarea
                value={node.data.config?.system_prompt || ''}
                onChange={(e) => onUpdateData(node.id, {
                  config: {
                    ...node.data.config,
                    system_prompt: e.target.value
                  }
                })}
                rows={4}
                className="w-full text-sm p-2 bg-zinc-950 border border-zinc-800 rounded-md focus:ring-1 focus:ring-blue-500/50 transition-all outline-none resize-none text-zinc-200"
                placeholder="Enter system prompt..."
              />
            </div>
            <div className="space-y-2">
              <label className="text-[11px] font-bold text-zinc-500">User Prompt Template</label>
              <textarea
                value={node.data.config?.user_prompt_template || ''}
                onChange={(e) => onUpdateData(node.id, {
                  config: {
                    ...node.data.config,
                    user_prompt_template: e.target.value
                  }
                })}
                rows={4}
                className="w-full text-sm p-2 bg-zinc-950 border border-zinc-800 rounded-md focus:ring-1 focus:ring-blue-500/50 transition-all outline-none resize-none text-zinc-200"
                placeholder="Enter user prompt template..."
              />
            </div>
          </>
        )}

        <div className="pt-6 border-t border-zinc-800">
          <p className="text-[10px] text-zinc-600 italic">
            提示: 通过节点底部的连接点拖拽来建立连接。
          </p>
        </div>
      </div>
    </aside>
  );
};
