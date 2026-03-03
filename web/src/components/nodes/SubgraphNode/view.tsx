import { memo, useEffect } from "react";
import { type NodeProps, useReactFlow, type Node, useUpdateNodeInternals } from "@xyflow/react";
import { ChevronDown, ChevronUp, Network } from "lucide-react";
import { NodeWrapper } from "../shared/NodeWrapper";
import { subgraphNodeStyles } from "./styles";
import type { NodeData } from "@/model/workflow/types";
import { useStore } from "@/store";
import { cn } from "@/utils/cn";

interface SubgraphNodeViewProps extends NodeProps {
    data: NodeData;
    expanded: boolean;
    onToggle: () => void;
}

export const SubgraphNodeView = memo(({ id, data, selected, expanded, onToggle, width, height, ...props }: SubgraphNodeViewProps) => {
    const { getNodes } = useReactFlow();
    const updateNodeInternals = useUpdateNodeInternals();
    const dragOverNodeId = useStore(state => state.dragOverNodeId);

    // Get child nodes count
    const childNodes = getNodes().filter((n: Node) => n.parentId === id);
    const childCount = childNodes.length;

    useEffect(() => {
        updateNodeInternals(id);
    }, [id, expanded, width, height, updateNodeInternals]);

    const isDragOver = dragOverNodeId === id;

    const headerActions = (
        <div className={subgraphNodeStyles.headerActions}>
            <button
                onClick={e => {
                    e.stopPropagation();
                    onToggle();
                }}
                className={subgraphNodeStyles.headerButton}
                title={expanded ? "Collapse" : "Expand"}
            >
                {expanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
            </button>
        </div>
    );

    return (
        <NodeWrapper
            id={id}
            data={data}
            selected={selected}
            className={cn(
                expanded ? subgraphNodeStyles.expandedContainer : subgraphNodeStyles.collapsedContainer,
                isDragOver && subgraphNodeStyles.dragOverHighlight,
            )}
            headerActions={headerActions}
            resizable={expanded}
            minWidth={expanded ? 200 : 260}
            minHeight={expanded ? 200 : 200}
            style={{ width, height }}
            {...props}
        >
            {!expanded && (
                <div className="flex flex-col items-center justify-center h-full pt-4">
                    <div className={subgraphNodeStyles.collapsedIcon}>
                        <Network size={20} className="text-zinc-400" />
                    </div>
                    <span className={subgraphNodeStyles.collapsedLabel}>Subgraph</span>
                    {childCount > 0 && <span className={subgraphNodeStyles.collapsedCount}>{childCount} nodes</span>}
                </div>
            )}
        </NodeWrapper>
    );
});
