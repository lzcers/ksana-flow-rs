import { memo, useEffect } from "react";
import { type NodeProps, useReactFlow, type Node, Position, useUpdateNodeInternals } from "@xyflow/react";
import { ChevronDown, ChevronUp, ChevronLeft, ChevronRight, Repeat2 } from "lucide-react";
import { NodeWrapper } from "../shared/NodeWrapper";
import type { NodeData } from "@/model/workflow/types";
import { cn } from "@/utils/cn";
import { mapNodeStyles } from "./styles";
import type { MapNodeStreamState } from "./hooks";

interface MapNodeViewProps extends NodeProps {
    data: NodeData;
    expanded: boolean;
    onToggle: () => void;
    maxConcurrency: string;
    streaming: boolean;
    onMaxConcurrencyChange: (next: string) => void;
    onStreamingToggle: () => void;
    streamState: MapNodeStreamState;
    activeThread: number;
    onThreadChange: (thread: number) => void;
}

export const MapNodeView = memo(
    ({
        id,
        data,
        selected,
        expanded,
        onToggle,
        width,
        height,
        maxConcurrency,
        streaming,
        onMaxConcurrencyChange,
        onStreamingToggle,
        streamState,
        activeThread,
        onThreadChange,
        ...props
    }: MapNodeViewProps) => {
        const { getNodes } = useReactFlow();
        const updateNodeInternals = useUpdateNodeInternals();

        const childNodes = getNodes().filter((n: Node) => n.parentId === id);
        const childCount = childNodes.length;

        useEffect(() => {
            updateNodeInternals(id);
        }, [id, expanded, width, height, updateNodeInternals]);

        const maxThreadCount = Math.max(1, parseInt(maxConcurrency, 10) || 1);
        const showPager = expanded && maxThreadCount >= 2;

        const handlePrevThread = (e: React.MouseEvent) => {
            e.stopPropagation();
            const newThread = activeThread <= 0 ? maxThreadCount - 1 : activeThread - 1;
            onThreadChange(newThread);
        };

        const handleNextThread = (e: React.MouseEvent) => {
            e.stopPropagation();
            const newThread = activeThread >= maxThreadCount - 1 ? 0 : activeThread + 1;
            onThreadChange(newThread);
        };

        const headerActions = (
            <div className={mapNodeStyles.headerActions}>
                {showPager && (
                    <div className={mapNodeStyles.pagerContainer}>
                        <button onClick={handlePrevThread} className={mapNodeStyles.pagerButton} title="Previous thread">
                            <ChevronLeft size={14} />
                        </button>
                        <span className={mapNodeStyles.pagerIndicator}>
                            {activeThread + 1} / {maxThreadCount}
                        </span>
                        <button onClick={handleNextThread} className={mapNodeStyles.pagerButton} title="Next thread">
                            <ChevronRight size={14} />
                        </button>
                    </div>
                )}
                <button
                    onClick={e => {
                        e.stopPropagation();
                        onToggle();
                    }}
                    className={mapNodeStyles.headerButton}
                    title={expanded ? "Collapse" : "Expand"}
                >
                    {expanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                </button>
            </div>
        );

        const statusText = (() => {
            if (streamState.finalCount != null) return `final: ${streamState.finalCount}`;
            if (streamState.doneCount != null) return `done: ${streamState.doneCount}`;
            if (streamState.lastItemIndex != null) return `item: #${streamState.lastItemIndex}`;
            if (streamState.receivedItems > 0) return `items: ${streamState.receivedItems}`;
            return streaming ? "streaming: ready" : "ready";
        })();

        return (
            <NodeWrapper
                id={id}
                data={data}
                selected={selected}
                className={expanded ? mapNodeStyles.expandedContainer : mapNodeStyles.collapsedContainer}
                headerActions={headerActions}
                resizable={expanded}
                minWidth={expanded ? 200 : 260}
                minHeight={expanded ? 200 : 200}
                targetHandles={[Position.Left, Position.Right, Position.Top, Position.Bottom]}
                sourceHandles={[Position.Left, Position.Right, Position.Top, Position.Bottom]}
                style={{ width, height }}
                {...props}
            >
                {!expanded && (
                    <div className="flex flex-col items-center justify-center h-full pt-4 w-full">
                        <div className={mapNodeStyles.collapsedIcon}>
                            <Repeat2 size={18} className="text-zinc-300" />
                        </div>
                        <span className={mapNodeStyles.collapsedLabel}>Map</span>
                        {childCount > 0 && <span className={mapNodeStyles.collapsedCount}>{childCount} nodes</span>}

                        <div className={mapNodeStyles.panel}>
                            <div className={cn(mapNodeStyles.fieldRow, "mt-2")}>
                                <span className={mapNodeStyles.fieldLabel}>Max</span>
                                <input
                                    type="number"
                                    className={mapNodeStyles.numberInput}
                                    value={maxConcurrency}
                                    onChange={e => onMaxConcurrencyChange(e.target.value)}
                                    onKeyDown={e => e.stopPropagation()}
                                    onPointerDown={e => e.stopPropagation()}
                                    min={0}
                                />
                            </div>

                            <div className={cn(mapNodeStyles.fieldRow, "mt-2")}>
                                <span className={mapNodeStyles.fieldLabel}>Streaming</span>
                                <button
                                    className={cn(mapNodeStyles.toggleButton, streaming && mapNodeStyles.toggleOn)}
                                    onClick={e => {
                                        e.stopPropagation();
                                        onStreamingToggle();
                                    }}
                                >
                                    {streaming ? "On" : "Off"}
                                </button>
                            </div>

                            <div className={mapNodeStyles.statusLine} title={statusText}>
                                {statusText}
                            </div>
                        </div>
                    </div>
                )}
            </NodeWrapper>
        );
    },
);
