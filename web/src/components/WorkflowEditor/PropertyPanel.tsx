import React from 'react';
import { Trash2 } from 'lucide-react';
import type { Node, NodeData } from '../../model/workflow/types';

interface PropertyPanelProps {
  node: Node;
  onUpdateData: (id: string, data: Partial<NodeData>) => void;
  onDelete: (id: string) => void;
}

export const PropertyPanel: React.FC<PropertyPanelProps> = ({
  node,
  onUpdateData,
  onDelete
}) => {
  return (
    <aside className="w-72 border-l border-white/10 bg-zinc-900/95 backdrop-blur-xl p-6 z-10 overflow-y-auto shadow-2xl">
      <div className="flex items-center justify-between mb-8">
        <h2 className="text-sm font-bold text-zinc-100 tracking-widest uppercase">Properties</h2>
        <button
          onClick={() => onDelete(node.id)}
          className="p-2 rounded-lg text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-all"
          title="Delete Node"
        >
          <Trash2 size={16} />
        </button>
      </div>

      <div className="space-y-6">
        <div className="space-y-2">
          <label className="text-[11px] font-bold text-zinc-500 uppercase tracking-wide">Label</label>
          <input
            type="text"
            value={node.data.label}
            onChange={(e) => onUpdateData(node.id, { label: e.target.value })}
            className="w-full text-sm p-3 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded-xl focus:ring-1 focus:ring-zinc-700/50 transition-all outline-none text-zinc-200 placeholder-zinc-600"
            placeholder="Node Label"
          />
        </div>

        <div className="space-y-2">
          <label className="text-[11px] font-bold text-zinc-500 uppercase tracking-wide">Description</label>
          <textarea
            value={node.data.description || ''}
            onChange={(e) => onUpdateData(node.id, { description: e.target.value })}
            rows={4}
            className="w-full text-sm p-3 bg-black/40 border border-white/10 rounded-xl focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500/50 transition-all outline-none resize-none text-zinc-200 placeholder-zinc-700"
            placeholder="Add a description..."
          />
        </div>

        {node.type === 'LLMNode' && (
          <>
            <div className="space-y-2">
              <label className="text-[11px] font-bold text-zinc-500 uppercase tracking-wide">System Prompt</label>
              <textarea
                value={node.data.config?.system_prompt || ''}
                onChange={(e) => onUpdateData(node.id, {
                  config: {
                    ...node.data.config,
                    system_prompt: e.target.value
                  }
                })}
                rows={4}
                className="w-full text-sm p-3 bg-black/40 border border-white/10 rounded-xl focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500/50 transition-all outline-none resize-none text-zinc-200 placeholder-zinc-700 font-mono text-xs"
                placeholder="Enter system prompt..."
              />
            </div>
            <div className="space-y-2">
              <label className="text-[11px] font-bold text-zinc-500 uppercase tracking-wide">User Prompt Template</label>
              <textarea
                value={node.data.config?.user_prompt_template || ''}
                onChange={(e) => onUpdateData(node.id, {
                  config: {
                    ...node.data.config,
                    user_prompt_template: e.target.value
                  }
                })}
                rows={4}
                className="w-full text-sm p-3 bg-zinc-800/50 hover:bg-zinc-800/80 focus:bg-black border border-transparent focus:border-zinc-800 rounded-xl focus:ring-1 focus:ring-zinc-700/50 transition-all outline-none resize-none text-zinc-200 placeholder-zinc-600 font-mono text-xs"
                placeholder="Enter user prompt template..."
              />
            </div>
          </>
        )}

        <div className="pt-6 border-t border-white/10">
          <p className="text-[10px] text-zinc-500 italic flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-blue-500/50"></span>
            Connect nodes by dragging from handles.
          </p>
        </div>
      </div>
    </aside>
  );
};
