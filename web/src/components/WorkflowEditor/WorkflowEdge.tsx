import React from 'react';
import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/react';
import type { Edge } from '../../model/workflow/types';
import { useSwipeToDeleteContext } from './SwipeToDeleteContext';

export const WorkflowEdge: React.FC<EdgeProps<Edge['data']>> = ({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style,
  markerEnd,
}) => {
  const { markedEdgeIds, isShiftPressed, markEdgeForDeletion } = useSwipeToDeleteContext();
  const isMarkedForDeletion = markedEdgeIds.has(id);

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
        stroke: '#a1a1aa',
        strokeWidth: 3,
        filter: 'drop-shadow(0 0 8px rgba(161, 161, 170, 0.6))',
      };
    }
    return style;
  }, [isMarkedForDeletion, style]);

  const markerEndFinal = isMarkedForDeletion
    ? { type: 'arrowClosed' as const, color: '#a1a1aa' }
    : markerEnd;

  return (
    <g onMouseEnter={handleMouseEnter}>
      {isMarkedForDeletion && (
        <path
          d={edgePath}
          fill="none"
          stroke="rgba(161, 161, 170, 0.25)"
          strokeWidth={14}
          strokeLinecap="butt"
        />
      )}
      <BaseEdge
        id={id}
        path={edgePath}
        style={edgeStyle}
        markerEnd={markerEndFinal}
        interactionWidth={20}
      />
    </g>
  );
};
