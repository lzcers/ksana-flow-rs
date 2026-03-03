import React, { useMemo } from "react";
import { Handle, NodeResizeControl, Position } from "@xyflow/react";
import { Play } from "lucide-react";
import { cn } from "@/utils/cn";
import type { NodeData } from "@/model/workflow/types";
import type { NodePorts, PortDef } from "@/model/nodeRegistry/types";
import { CONTROL_HANDLE_ID, dataHandleId, getNodePorts } from "@/model/nodeRegistry";
import { useStore } from "@/store";
import { nodeWrapperStyles } from "./styles";
import { useNodeLabel } from "./useNodeLabel";
import "../node.css";

interface NodeWrapperProps {
    id: string;
    type: string;
    data: NodeData;
    selected: boolean;
    /** @deprecated 使用 ports 替代 */
    sourceHandles?: Position[];
    /** @deprecated 使用 ports 替代 */
    targetHandles?: Position[];
    /** 新的端口定义 */
    ports?: NodePorts;
    children?: React.ReactNode;
    className?: string;
    style?: React.CSSProperties;
    resizable?: boolean;
    minWidth?: number;
    minHeight?: number;
    keepAspectRatio?: boolean;
    headerActions?: React.ReactNode;
}

// 端口位置样式映射
const HANDLE_POSITION_STYLES: Record<Position, React.CSSProperties> = {
    [Position.Top]: { top: -6, left: "50%", transform: "translateX(-50%)" },
    [Position.Bottom]: { bottom: -6, left: "50%", transform: "translateX(-50%)" },
    [Position.Left]: { left: -6, top: "50%", transform: "translateY(-50%)" },
    [Position.Right]: { right: -6, top: "50%", transform: "translateY(-50%)" },
};

// 数据端口标签位置样式
const LABEL_POSITION_STYLES: Record<Position, React.CSSProperties> = {
    [Position.Left]: { right: 10, top: "50%", transform: "translateY(-50%)" },
    [Position.Right]: { left: 10, top: "50%", transform: "translateY(-50%)" },
    [Position.Top]: { bottom: 10, left: "50%", transform: "translateX(-50%)" },
    [Position.Bottom]: { top: 10, left: "50%", transform: "translateX(-50%)" },
};

/**
 * 将旧版 Position 数组转换为 NodePorts（向后兼容）
 */
function legacyPositionToPorts(
    sourceHandles: Position[],
    targetHandles: Position[]
): NodePorts {
    return {
        inputs: targetHandles.map((position) => ({
            id: `legacy-${position}`,
            label: "",
            kind: "control" as const,
            position,
        })),
        outputs: sourceHandles.map((position) => ({
            id: `legacy-${position}`,
            label: "",
            kind: "control" as const,
            position,
        })),
    };
}

/**
 * 渲染单个端口
 */
function renderHandle(
    port: PortDef,
    handleType: "source" | "target",
    styles: typeof nodeWrapperStyles,
    isVisible: boolean
): React.ReactNode {
    const isControl = port.kind === "control";
    const handleId = isControl ? CONTROL_HANDLE_ID : dataHandleId(port.id);

    // 选择样式
    const handleClassName = isControl
        ? handleType === "target"
            ? styles.handleControlTarget
            : styles.handleControlSource
        : handleType === "target"
            ? styles.handleDataTarget
            : styles.handleDataSource;

    return (
        <Handle
            key={`${handleType}-${port.id}`}
            type={handleType}
            position={port.position}
            id={handleId}
            className={cn(
                handleClassName,
                isVisible ? "opacity-100" : "opacity-0 group-hover:opacity-100",
                port.required && styles.handleRequired
            )}
            style={HANDLE_POSITION_STYLES[port.position]}
        >
            {/* 数据端口显示标签 */}
            {!isControl && port.label && (
                <span
                    className={styles.handleLabel}
                    style={LABEL_POSITION_STYLES[port.position]}
                >
                    {port.label}
                </span>
            )}
        </Handle>
    );
}

export const NodeWrapper: React.FC<NodeWrapperProps> = ({
    id,
    type,
    data,
    selected,
    sourceHandles = [],
    targetHandles = [],
    ports,
    children,
    className,
    minWidth,
    minHeight,
    keepAspectRatio = false,
    style,
    resizable = true,
    headerActions,
}) => {
    const status = data.status || "idle";
    const { runNode, updateNodeDimensions, isConnecting, connectionSourceId, currentWorkflowStatus, updateNodeData } = useStore();

    const { editingLabel, setEditingLabel, labelDraft, setLabelDraft, inputRef, commitLabel, cancelLabel } = useNodeLabel({
        id,
        label: data.label,
        updateNodeData,
    });

    // 解析端口定义：优先使用传入的 ports，其次从注册表获取，最后使用旧版兼容
    const resolvedPorts: NodePorts = useMemo(() => {
        if (ports) return ports;
        const registeredPorts = getNodePorts(type);
        if (registeredPorts) return registeredPorts;
        return legacyPositionToPorts(sourceHandles, targetHandles);
    }, [ports, type, sourceHandles, targetHandles]);

    // 是否使用新端口系统
    const useNewPorts = ports !== undefined || getNodePorts(type) !== undefined;

    const handleRun = (e: React.MouseEvent) => {
        e.stopPropagation();
        runNode([id]);
    };

    return (
        <div
            className={cn(nodeWrapperStyles.root, "w-full h-full")}
            style={{
                minWidth: minWidth ?? "fit-content",
                minHeight: minHeight ?? "fit-content",
                ...style,
            }}
        >
            <div className={nodeWrapperStyles.header}>
                <div className={nodeWrapperStyles.headerLeft}>
                    <div className={nodeWrapperStyles.headerDot} />
                    {editingLabel ? (
                        <input
                            ref={inputRef}
                            value={labelDraft}
                            onChange={e => setLabelDraft(e.target.value)}
                            onBlur={e => {
                                e.stopPropagation();
                                commitLabel();
                            }}
                            onKeyDown={e => {
                                if (e.key === "Enter") {
                                    e.preventDefault();
                                    commitLabel();
                                } else if (e.key === "Escape") {
                                    e.preventDefault();
                                    cancelLabel();
                                }
                            }}
                            onPointerDown={e => e.stopPropagation()}
                            className={nodeWrapperStyles.labelInput}
                            placeholder="请输入标签"
                        />
                    ) : (
                        <span
                            className={nodeWrapperStyles.headerLabel}
                            onDoubleClick={e => {
                                e.stopPropagation();
                                setEditingLabel(true);
                            }}
                        >
                            {data.label}
                        </span>
                    )}
                </div>

                <div
                    className={cn(
                        nodeWrapperStyles.headerRight,
                        selected
                            ? "opacity-100 pointer-events-auto"
                            : "opacity-0 group-hover:opacity-100 pointer-events-none group-hover:pointer-events-auto",
                    )}
                >
                    {headerActions}
                    {currentWorkflowStatus === "idle" && (
                        <button onClick={handleRun} className={nodeWrapperStyles.runButton} title="Run Node">
                            <Play size={12} fill="currentColor" className="ml-0.5 opacity-80" />
                        </button>
                    )}
                </div>
            </div>

            <div
                className={cn(
                    nodeWrapperStyles.cardBase,
                    status === "running" ? "node-running" : selected ? nodeWrapperStyles.cardSelected : nodeWrapperStyles.cardIdle,
                    className,
                )}
            >
                {resizable && (
                    <NodeResizeControl
                        minWidth={minWidth ?? 100}
                        minHeight={minHeight ?? 50}
                        keepAspectRatio={keepAspectRatio}
                        position="bottom-right"
                        className={cn(nodeWrapperStyles.resizeControlBase, selected ? "opacity-100" : nodeWrapperStyles.resizeControlHidden)}
                        onResizeEnd={(_event, params) => {
                            updateNodeDimensions(id, params.width, params.height);
                        }}
                    >
                        <div className={nodeWrapperStyles.resizeHandle}>
                            <svg
                                width="24"
                                height="24"
                                viewBox="0 0 24 24"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                                className={nodeWrapperStyles.resizeIcon}
                            >
                                <path d="M 18 6 C 18 14 16 18 6 18" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
                            </svg>
                        </div>
                    </NodeResizeControl>
                )}

                {/* 新端口系统渲染 */}
                {useNewPorts ? (
                    <>
                        {resolvedPorts.inputs.map(port =>
                            renderHandle(port, "target", nodeWrapperStyles, isConnecting && id !== connectionSourceId)
                        )}
                        {resolvedPorts.outputs.map(port =>
                            renderHandle(port, "source", nodeWrapperStyles, !isConnecting || id === connectionSourceId)
                        )}
                    </>
                ) : (
                    /* 旧版兼容渲染 */
                    <>
                        {targetHandles.map(position => (
                            <Handle
                                key={`target-${position}`}
                                type="target"
                                position={position}
                                id={`t-${position}`}
                                className={cn(
                                    nodeWrapperStyles.handleTargetBase,
                                    isConnecting && id !== connectionSourceId ? "opacity-100" : "opacity-0 group-hover:opacity-100",
                                )}
                                style={HANDLE_POSITION_STYLES[position]}
                            />
                        ))}

                        {sourceHandles.map(position => (
                            <Handle
                                key={`source-${position}`}
                                type="source"
                                position={position}
                                id={`s-${position}`}
                                className={cn(
                                    nodeWrapperStyles.handleSourceBase,
                                    (!isConnecting || id === connectionSourceId) && (selected ? "opacity-100" : "opacity-0 group-hover:opacity-100"),
                                )}
                                style={HANDLE_POSITION_STYLES[position]}
                            />
                        ))}
                    </>
                )}

                <div className={nodeWrapperStyles.contentArea} style={{ borderRadius: "12px" }}>
                    {children}

                    {data.errorMessage && <div className={nodeWrapperStyles.errorToast}>{data.errorMessage}</div>}
                </div>
            </div>
        </div>
    );
};
