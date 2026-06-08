import React from 'react';

export interface UseSwipeToDeleteReturn {
  markedEdgeIds: Set<string>;
  isShiftPressed: boolean;
  markEdgeForDeletion: (edgeId: string) => void;
}

export const useSwipeToDelete = (
  onDeleteEdges: (edgeIds: string[]) => void
): UseSwipeToDeleteReturn => {
  const [markedEdgeIds, setMarkedEdgeIds] = React.useState<Set<string>>(new Set());
  const [isShiftPressed, setIsShiftPressed] = React.useState(false);

  const shiftPressedRef = React.useRef(false);
  const markedEdgeIdsRef = React.useRef<Set<string>>(new Set());

  React.useEffect(() => {
    markedEdgeIdsRef.current = markedEdgeIds;
  }, [markedEdgeIds]);

  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Shift' && !shiftPressedRef.current) {
        shiftPressedRef.current = true;
        setIsShiftPressed(true);
        setMarkedEdgeIds(new Set());
        markedEdgeIdsRef.current = new Set();
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Shift' && shiftPressedRef.current) {
        shiftPressedRef.current = false;
        setIsShiftPressed(false);

        const edgeIdsToDelete = Array.from(markedEdgeIdsRef.current);
        if (edgeIdsToDelete.length > 0) {
          onDeleteEdges(edgeIdsToDelete);
        }
        setMarkedEdgeIds(new Set());
        markedEdgeIdsRef.current = new Set();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [onDeleteEdges]);

  const markEdgeForDeletion = React.useCallback((edgeId: string) => {
    if (shiftPressedRef.current) {
      setMarkedEdgeIds(prev => {
        const newSet = new Set(prev);
        newSet.add(edgeId);
        return newSet;
      });
    }
  }, []);

  return {
    markedEdgeIds,
    isShiftPressed,
    markEdgeForDeletion,
  };
};
