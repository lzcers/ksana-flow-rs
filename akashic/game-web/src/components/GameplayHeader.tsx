import React, { useCallback, useRef, useState } from 'react';
import { Clock3, Hourglass, Sparkles } from 'lucide-react';
import { StatusPill } from './AkashicUI';

interface GameplayHeaderProps {
  currentRound: number;
  currentScene: string;
  isLoading: boolean;
  broadcastMessages: string[];
}

const GameplayHeader: React.FC<GameplayHeaderProps> = ({
  currentRound,
  currentScene,
  isLoading,
  broadcastMessages,
}) => {
  const [broadcastCursor, setBroadcastCursor] = useState({ key: '', index: 0 });
  const [isBroadcastDragging, setIsBroadcastDragging] = useState(false);
  const broadcastSwipeStartRef = useRef<{ pointerId: number; clientX: number } | null>(null);
  const suppressBroadcastClickRef = useRef(false);

  const isFatePlanningScene = isLoading && currentScene.includes('命运编织');
  const broadcastKey = broadcastMessages.join('||');
  const broadcastIndex = broadcastCursor.key === broadcastKey
    ? Math.min(broadcastCursor.index, Math.max(broadcastMessages.length - 1, 0))
    : 0;
  const activeBroadcastMessage = broadcastMessages[broadcastIndex] ?? broadcastMessages[0] ?? '';
  const broadcastCountLabel = broadcastMessages.length > 0
    ? `${Math.min(broadcastIndex + 1, broadcastMessages.length)}/${broadcastMessages.length}`
    : '0/0';

  const moveBroadcastIndex = useCallback((direction: 'prev' | 'next') => {
    if (broadcastMessages.length <= 1) {
      return;
    }

    setBroadcastCursor((prev) => {
      const currentIndex = prev.key === broadcastKey ? prev.index : 0;
      const nextIndex = direction === 'next'
        ? (currentIndex + 1) % broadcastMessages.length
        : (currentIndex - 1 + broadcastMessages.length) % broadcastMessages.length;
      return {
        key: broadcastKey,
        index: nextIndex,
      };
    });
  }, [broadcastKey, broadcastMessages.length]);

  const releaseBroadcastPointer = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    broadcastSwipeStartRef.current = null;
    setIsBroadcastDragging(false);

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }, []);

  const handleBroadcastPointerDown = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (event.pointerType === 'mouse' && event.button !== 0) {
      return;
    }

    broadcastSwipeStartRef.current = {
      pointerId: event.pointerId,
      clientX: event.clientX,
    };
    setIsBroadcastDragging(true);
    event.currentTarget.setPointerCapture(event.pointerId);
  }, []);

  const handleBroadcastPointerEnd = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const start = broadcastSwipeStartRef.current;
    if (!start || start.pointerId !== event.pointerId) {
      return;
    }

    const deltaX = event.clientX - start.clientX;
    releaseBroadcastPointer(event);

    if (Math.abs(deltaX) < 36) {
      return;
    }

    suppressBroadcastClickRef.current = true;
    moveBroadcastIndex(deltaX < 0 ? 'next' : 'prev');
  }, [moveBroadcastIndex, releaseBroadcastPointer]);

  const handleBroadcastPointerCancel = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const start = broadcastSwipeStartRef.current;
    if (!start || start.pointerId !== event.pointerId) {
      return;
    }

    releaseBroadcastPointer(event);
  }, [releaseBroadcastPointer]);

  const handleBroadcastClick = useCallback(() => {
    if (suppressBroadcastClickRef.current) {
      suppressBroadcastClickRef.current = false;
      return;
    }

    moveBroadcastIndex('next');
  }, [moveBroadcastIndex]);

  const handleBroadcastKeyDown = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      moveBroadcastIndex('prev');
      return;
    }

    if (event.key === 'ArrowRight' || event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      moveBroadcastIndex('next');
    }
  }, [moveBroadcastIndex]);

  return (
    <div className="shrink-0 space-y-2">
      <div className="flex flex-wrap gap-1 justify-between">
        <StatusPill icon={Clock3} className="px-2.5 py-1 text-[0.7rem] sm:text-xs">第 {currentRound} 轮</StatusPill>
        <StatusPill
          icon={isFatePlanningScene ? Hourglass : null}
          iconClassName={isFatePlanningScene ? 'h-3 w-3 animate-spin' : undefined}
          className="px-2.5 py-1 text-[0.7rem] sm:text-xs"
        >
          {currentScene}
        </StatusPill>
      </div>
      <div className="relative h-10 sm:h-11">
        {activeBroadcastMessage ? (
          <div
            className={`akashic-pill absolute inset-y-0 left-0 flex w-full max-w-full items-start border-amber-300/50 bg-[#1d1820]/95 px-2.5 py-1 text-[0.72rem] text-amber-100 select-none touch-pan-y sm:text-xs ${isBroadcastDragging ? 'cursor-grabbing' : 'cursor-grab'}`}
            role={broadcastMessages.length > 1 ? 'button' : undefined}
            tabIndex={broadcastMessages.length > 1 ? 0 : undefined}
            aria-label={broadcastMessages.length > 1 ? '切换天命回响' : undefined}
            onPointerDown={handleBroadcastPointerDown}
            onPointerUp={handleBroadcastPointerEnd}
            onPointerCancel={handleBroadcastPointerCancel}
            onClick={handleBroadcastClick}
            onKeyDown={handleBroadcastKeyDown}
          >
            <Sparkles className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-200" />
            <span className="line-clamp-2 min-w-0 flex-1 leading-4">{activeBroadcastMessage}</span>
            <span className="shrink-0 rounded-full border border-amber-300/25 bg-black/15 px-1.5 py-0.5 text-[0.65rem] leading-none text-amber-100/80 sm:text-[0.7rem]">
              {broadcastCountLabel}
            </span>
          </div>
        ) : null}
      </div>
    </div>
  );
};

export default GameplayHeader;
