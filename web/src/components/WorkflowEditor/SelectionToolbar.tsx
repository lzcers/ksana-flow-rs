import React from 'react';
import { useReactFlow, type Node } from '@xyflow/react';
import { Group } from 'lucide-react';

// Utility for className merging
function cn(...classes: (string | boolean | undefined)[]) {
  return classes.filter(Boolean).join(' ');
}

interface SelectionToolbarProps {
  onGroupNodes: (nodeIds: string[]) => void;
}

export const SelectionToolbar: React.FC<SelectionToolbarProps> = ({
  onGroupNodes,
}) => {
  const { getNodes, screenToFlowPosition } = useReactFlow();

  // 获取选中的节点
  const selectedNodes = getNodes().filter((n: Node) => n.selected);

  // 如果没有选中节点或多个节点，则不显示
  if (selectedNodes.length < 2) {
    return null;
  }

  // 计算选中节点的边界框
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  selectedNodes.forEach((node: Node) => {
    const x = node.position.x;
    const y = node.position.y;
    const width = (node.measured?.width ?? 150) as number;
    const height = (node.measured?.height ?? 50) as number;

    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x + width);
    maxY = Math.max(maxY, y + height);
  });

  // 计算工具栏位置（在框选区域的上方居中）
  const centerX = (minX + maxX) / 2;
  const toolbarY = minY - 50; // 在框选区域上方 50px

  const handleGroupClick = () => {
    const nodeIds = selectedNodes.map((n: Node) => n.id);
    onGroupNodes(nodeIds);
  };

  return (
    <div
      className="absolute z-50 pointer-events-auto"
      style={{
        left: centerX,
        top: toolbarY,
        transform: 'translate(-50%, 0)',
      }}
    >
      <div className="flex items-center gap-1.5 px-2 py-1.5 bg-zinc-900/95 backdrop-blur-xl border border-white/10 rounded-xl shadow-2xl animate-in fade-in zoom-in-95 duration-200">
        <span className="text-[10px] text-zinc-400 font-medium whitespace-nowrap">
          {selectedNodes.length} selected
        </span>
        <div className="w-px h-3 bg-white/10 mx-1" />
        <button
          onClick={handleGroupClick}
          className={cn(
            "flex items-center gap-1.5 px-2 py-1 rounded-lg text-[11px] font-medium transition-all",
            "bg-blue-500/10 text-blue-400 hover:bg-blue-500/20 hover:text-blue-300",
            "border border-blue-500/20 hover:border-blue-500/30"
          )}
        >
          <Group size={12} />
          Create Subgraph
        </button>
      </div>
    </div>
  );
};
