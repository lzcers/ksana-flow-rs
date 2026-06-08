import React from "react";
import { useStore, useViewport, type Node } from "@xyflow/react";
import { Group, Play } from "lucide-react";

function cn(...classes: (string | boolean | undefined)[]) {
    return classes.filter(Boolean).join(" ");
}

interface SelectionToolbarProps {
    onGroupNodes: (nodeIds: string[]) => void;
    onRunNodes: (nodeIds: string[]) => void;
}

export const SelectionToolbar: React.FC<SelectionToolbarProps> = ({ onGroupNodes, onRunNodes }) => {
    const nodes = useStore(state => state.nodes);
    const { x, y, zoom } = useViewport();

    // 获取选中的节点
    const selectedNodes = React.useMemo(() => nodes.filter((n: Node) => n.selected), [nodes]);

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
        const nodeX = node.position.x;
        const nodeY = node.position.y;
        const width = (node.measured?.width ?? 150) as number;
        const height = (node.measured?.height ?? 50) as number;

        minX = Math.min(minX, nodeX);
        minY = Math.min(minY, nodeY);
        maxX = Math.max(maxX, nodeX + width);
        maxY = Math.max(maxY, nodeY + height);
    });

    // 计算工具栏位置（在框选区域的上方居中）
    const centerX = (minX + maxX) / 2;
    const toolbarY = minY - 12; // 在框选区域上方 12px
    const left = centerX * zoom + x;
    const top = toolbarY * zoom + y;

    const handleGroupClick = () => {
        const nodeIds = selectedNodes.map((n: Node) => n.id);
        onGroupNodes(nodeIds);
    };

    const handleRunClick = () => {
        const nodeIds = selectedNodes.map((n: Node) => n.id);
        onRunNodes(nodeIds);
    };

    return (
        <div
            className="absolute z-50 pointer-events-auto"
            style={{
                left,
                top,
                transform: "translate(-50%, -100%)",
            }}
        >
            <div className="flex items-center gap-1.5 px-2 py-1.5 bg-zinc-900/95 backdrop-blur-xl border border-white/10 rounded-xl shadow-2xl animate-in fade-in zoom-in-95 duration-200">
                <span className="text-[10px] text-zinc-400 font-medium whitespace-nowrap">{selectedNodes.length} selected</span>
                <div className="w-px h-3 bg-white/10 mx-1" />
                <button
                    onClick={handleRunClick}
                    className={cn(
                        "flex items-center gap-1.5 px-2 py-1 rounded-lg text-[11px] font-medium transition-all",
                        "bg-green-500/10 text-green-400 hover:bg-green-500/20 hover:text-green-300",
                        "border border-green-500/20 hover:border-green-500/30",
                    )}
                >
                    <Play size={12} />
                    Run Selected
                </button>
                <button
                    onClick={handleGroupClick}
                    className={cn(
                        "flex items-center gap-1.5 px-2 py-1 rounded-lg text-[11px] font-medium transition-all",
                        "bg-blue-500/10 text-blue-400 hover:bg-blue-500/20 hover:text-blue-300",
                        "border border-blue-500/20 hover:border-blue-500/30",
                    )}
                >
                    <Group size={12} />
                    Create Subgraph
                </button>
            </div>
        </div>
    );
};
