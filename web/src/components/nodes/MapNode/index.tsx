import { memo, useCallback, useState, useEffect } from "react";
import { type NodeProps } from "@xyflow/react";
import type { NodeData } from "@/model/workflow/types";
import { useNodeConfig } from "../shared/hooks/useNodeConfig";
import { useNodeConfigField } from "../shared/hooks/useNodeConfigField";
import { useStore } from "@/store";
import { useMapNodeStream } from "./hooks";
import { MapNodeView } from "./view";
import { workflowManager } from "@/model/workflowManager";

export const MapNode = memo((props: NodeProps & { data: NodeData }) => {
    const { id, data } = props;
    const { updateConfig } = useNodeConfig(id, data.config);

    const toggleSubgraph = useStore(s => s.toggleSubgraph);
    const onToggle = useCallback(() => {
        if (toggleSubgraph) toggleSubgraph(id);
    }, [id, toggleSubgraph]);

    const maxConcurrencyField = useNodeConfigField<string>({
        value: String(data.config?.max_concurrency ?? 10),
        commitMode: "change",
        updateValue: next => {
            const n = parseInt(next, 10);
            if (Number.isFinite(n) && n >= 0) updateConfig({ max_concurrency: n });
        },
    });

    const streaming = Boolean(data.config?.streaming ?? false);
    const onStreamingToggle = useCallback(() => {
        updateConfig({ streaming: !streaming });
    }, [streaming, updateConfig]);

    const streamState = useMapNodeStream(id, data);

    const maxThreadCount = Math.max(1, parseInt(maxConcurrencyField.draft, 10) || 1);

    const [activeThread, setActiveThread] = useState(streamState.activeThreadIndex ?? 0);
    const activeGraphKey = useStore(s => s.activeGraphKey);

    useEffect(() => {
        if (activeThread >= maxThreadCount) {
            setActiveThread(0);
        }
    }, [maxThreadCount, activeThread]);

    // 当 MapNode 展开时，自动激活当前线程的子图（支持预激活）
    useEffect(() => {
        const isExpanded = data.expanded !== false;

        if (isExpanded && activeGraphKey) {
            const instance = workflowManager.getModelInstance(activeGraphKey);
            if (instance) {
                // 激活当前线程，支持预激活（子图可能尚未创建）
                instance.activateSubgraph(id, activeThread);
            }
        }
    }, [data.expanded, activeGraphKey, id, activeThread]);

    const onThreadChange = useCallback(
        (thread: number) => {
            setActiveThread(thread);

            // 激活对应的子图实例
            if (activeGraphKey) {
                const instance = workflowManager.getModelInstance(activeGraphKey);
                if (instance) {
                    instance.activateSubgraph(id, thread);
                }
            }
        },
        [id, activeGraphKey],
    );

    return (
        <MapNodeView
            {...props}
            expanded={data.expanded !== false}
            onToggle={onToggle}
            maxConcurrency={maxConcurrencyField.draft}
            streaming={streaming}
            onMaxConcurrencyChange={maxConcurrencyField.onChange}
            onStreamingToggle={onStreamingToggle}
            streamState={streamState}
            activeThread={activeThread}
            onThreadChange={onThreadChange}
        />
    );
});

MapNode.displayName = "MapNode";
