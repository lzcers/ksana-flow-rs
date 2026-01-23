import React, { useRef, useEffect, useState } from 'react';
import { NODE_TYPES } from './nodeTypes';
import { FileText, Search } from 'lucide-react';
import { cn } from '../../utils/cn';
import type { NodeMetadata } from '../../api';

interface NodeContextMenuProps {
  visible: boolean;
  position: { x: number; y: number };
  nodeTypes: NodeMetadata[];
  onSelect: (type: string) => void;
  onClose: () => void;
}

const getIcon = (name: string) => {
  const nodeType = NODE_TYPES.find(i => i.type === name);
  return nodeType?.icon || FileText;
};

const getColorForCategory = (category: string) => {
  switch (category.toLowerCase()) {
    case 'source': return 'bg-blue-500/20 text-blue-400 border border-blue-500/20 shadow-[0_0_10px_rgba(59,130,246,0.1)]';
    case 'strategy': return 'bg-purple-500/20 text-purple-400 border border-purple-500/20 shadow-[0_0_10px_rgba(168,85,247,0.1)]';
    case 'sink': return 'bg-orange-500/20 text-orange-400 border border-orange-500/20 shadow-[0_0_10px_rgba(249,115,22,0.1)]';
    default: return 'bg-zinc-500/20 text-zinc-400 border border-zinc-500/20';
  }
};

export const NodeContextMenu: React.FC<NodeContextMenuProps> = ({
  visible,
  position,
  nodeTypes,
  onSelect,
  onClose,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    if (visible) {
      document.addEventListener('mousedown', handleClickOutside);
      setSearchQuery('');
      // Focus input when menu opens
      setTimeout(() => inputRef.current?.focus(), 50);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [visible, onClose]);

  const filteredNodeTypes = nodeTypes.filter(nodeType =>
    nodeType.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    nodeType.category.toLowerCase().includes(searchQuery.toLowerCase())
  );

  if (!visible) return null;

  return (
    <div
      ref={menuRef}
      style={{ top: position.y, left: position.x }}
      className="fixed z-50 w-64 bg-zinc-900/95 backdrop-blur-xl border border-white/10 rounded-xl shadow-2xl p-1.5 flex flex-col gap-0.5 max-h-[400px] animate-in fade-in zoom-in-95 duration-150"
    >
      <div className="px-2 py-1.5 border-b border-white/5 mb-1">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 text-zinc-500" size={14} />
          <input
            ref={inputRef}
            type="text"
            placeholder="Search nodes..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            className="w-full bg-zinc-800/50 border border-white/5 rounded-lg py-1.5 pl-8 pr-2 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-blue-500/30 focus:bg-zinc-800 focus:ring-1 focus:ring-blue-500/20 transition-all"
            onKeyDown={e => e.stopPropagation()}
          />
        </div>
      </div>

      <div className="overflow-y-auto flex-1 flex flex-col gap-0.5 pr-1">
        {filteredNodeTypes.length === 0 ? (
          <div className="text-center py-4 text-zinc-500 text-xs">
            No nodes found
          </div>
        ) : (
          filteredNodeTypes.map(nodeType => {
            const Icon = getIcon(nodeType.name);
            const colorClass = getColorForCategory(nodeType.category);

            return (
              <button
                key={nodeType.name}
                onClick={() => onSelect(nodeType.name)}
                className="w-full flex items-center gap-2.5 p-1.5 rounded-lg hover:bg-white/5 transition-all text-left group hover:pl-2.5"
              >
                <div className={cn("p-1.5 rounded-md transition-colors", colorClass)}>
                  <Icon size={14} />
                </div>
                <div className="flex flex-col">
                  <span className="text-[13px] font-medium text-zinc-300 group-hover:text-white transition-colors">
                    {nodeType.name}
                  </span>
                  <span className="text-[9px] text-zinc-500 group-hover:text-zinc-400 capitalize leading-none mt-0.5">{nodeType.category}</span>
                </div>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
};
