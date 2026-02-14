import React from 'react';

interface SwipeToDeleteContextValue {
  markedEdgeIds: Set<string>;
  isShiftPressed: boolean;
  markEdgeForDeletion: (edgeId: string) => void;
}

export const SwipeToDeleteContext = React.createContext<SwipeToDeleteContextValue>({
  markedEdgeIds: new Set(),
  isShiftPressed: false,
  markEdgeForDeletion: () => {},
});

export const useSwipeToDeleteContext = () => React.useContext(SwipeToDeleteContext);
