import React, { useRef, useEffect } from 'react';
import { NODE_TYPES } from './nodeTypes';
import { FileText } from 'lucide-react';
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
    case 'source': return 'bg-blue-900/30 text-blue-400 border border-blue-800/50';
    case 'strategy': return 'bg-purple-900/30 text-purple-400 border border-purple-800/50';
    case 'sink': return 'bg-orange-900/30 text-orange-400 border border-orange-800/50';
    default: return 'bg-zinc-800 text-zinc-400 border border-zinc-700';
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

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    if (visible) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [visible, onClose]);

  if (!visible) return null;

  return (
    <div
      ref={menuRef}
      style={{ top: position.y, left: position.x }}
      className="fixed z-50 w-64 bg-zinc-900 border border-zinc-800 rounded-lg shadow-xl p-2 flex flex-col gap-1 max-h-[400px] overflow-y-auto"
    >
        <div className="text-xs font-semibold text-zinc-500 px-2 py-1 uppercase tracking-wider">
            Add Node
        </div>
      {nodeTypes.map(nodeType => {
        const Icon = getIcon(nodeType.name);
        const colorClass = getColorForCategory(nodeType.category);
        
        return (
          <button
            key={nodeType.name}
            onClick={() => onSelect(nodeType.name)}
            className="w-full flex items-center gap-3 p-2 rounded-md hover:bg-zinc-800 transition-colors text-left group"
          >
            <div className={cn("p-1.5 rounded-md transition-colors", colorClass)}>
              <Icon size={14} />
            </div>
            <div className="flex flex-col">
              <span className="text-sm font-medium text-zinc-300 group-hover:text-zinc-100">
                {nodeType.name}
              </span>
              <span className="text-[10px] text-zinc-600">{nodeType.category}</span>
            </div>
          </button>
        );
      })}
    </div>
  );
};
