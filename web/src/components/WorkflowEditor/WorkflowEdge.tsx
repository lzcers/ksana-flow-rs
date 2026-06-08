import React from "react";
import { BaseEdge, getBezierPath, type EdgeProps } from "@xyflow/react";
import type { Edge, EdgeData } from "../../model/workflow/types";
import { useSwipeToDeleteContext } from "./SwipeToDeleteContext";
import { getEdgeKindFromHandles } from "../../model/workflow/utils/connection";

/**
 * 边样式配置
 */
const EDGE_STYLES = {
  control: {
    stroke: "#3b82f6", // 蓝色
    strokeWidth: 2,
  },
  data: {
    stroke: "#10b981", // 绿色
    strokeWidth: 2,
    strokeDasharray: "5,5", // 虚线
  },
  default: {
    stroke: "#3b82f6",
    strokeWidth: 2,
  },
};

export const WorkflowEdge: React.FC<EdgeProps<Edge>> = ({
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    style,
    markerEnd,
    data,
    sourceHandleId,
    targetHandleId,
}) => {
    const { markedEdgeIds, isShiftPressed, markEdgeForDeletion } = useSwipeToDeleteContext();
    const isMarkedForDeletion = markedEdgeIds.has(id);

    // 获取边类型
    const edgeKind = (data as EdgeData | undefined)?.kind ?? getEdgeKindFromHandles(sourceHandleId, targetHandleId);
    const baseEdgeStyle = EDGE_STYLES[edgeKind] || EDGE_STYLES.default;

    const [edgePath] = getBezierPath({
        sourceX,
        sourceY,
        targetX,
        targetY,
        sourcePosition,
        targetPosition,
    });

    const handleMouseEnter = React.useCallback(() => {
        if (isShiftPressed) {
            markEdgeForDeletion(id);
        }
    }, [id, isShiftPressed, markEdgeForDeletion]);

    const edgeStyle = React.useMemo(() => {
        if (isMarkedForDeletion) {
            return {
                ...style,
                stroke: "#a1a1aa",
                strokeWidth: 3,
                filter: "drop-shadow(0 0 8px rgba(161, 161, 170, 0.6))",
            };
        }
        // 合并基础样式和传入的样式
        return {
            ...baseEdgeStyle,
            ...style,
        };
    }, [isMarkedForDeletion, style, baseEdgeStyle]);

    const markerEndFinal = isMarkedForDeletion ? undefined : markerEnd;

    return (
        <g onMouseEnter={handleMouseEnter}>
            {isMarkedForDeletion && <path d={edgePath} fill="none" stroke="rgba(161, 161, 170, 0.25)" strokeWidth={14} strokeLinecap="butt" />}
            <BaseEdge id={id} path={edgePath} style={edgeStyle} markerEnd={markerEndFinal} interactionWidth={20} />
        </g>
    );
};
