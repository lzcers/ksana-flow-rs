import React, { useEffect, useRef } from 'react';
import Typewriter from './Typewriter';
import { STREAM_PLACEHOLDER_TEXT } from '../store/gameStoreHelpers';
import type { NarrationRoundEntry } from './gameplayTypes';

interface NarrationHistoryItemProps {
  entry: NarrationRoundEntry;
  isCurrentRound: boolean;
  animateCurrentRound: boolean;
  isFinished: boolean;
  onComplete?: () => void;
}

interface NarrationPanelProps {
  narrationHistory: NarrationRoundEntry[];
  currentRound: number;
  isAwaitingNarration: boolean;
  skipRestoredNarrationAnimation: boolean;
  onTypewriterComplete: () => void;
}

const NarrationHistoryItem: React.FC<NarrationHistoryItemProps> = React.memo(({
  entry,
  isCurrentRound,
  animateCurrentRound,
  isFinished,
  onComplete,
}) => {
  return (
    <div className="space-y-2">
      <p className="text-sm font-medium text-[#d8c7aa]">
        第 {entry.round} 轮：{entry.title || ""}
      </p>
      {entry.isAwaitingNarration && !entry.narrationText ? (
        <p className="text-sm font-medium text-[#8f98ab]">
          {STREAM_PLACEHOLDER_TEXT}
        </p>
      ) : (
        <Typewriter
          text={entry.narrationText}
          animate={animateCurrentRound}
          isFinished={isFinished}
          onComplete={isCurrentRound ? onComplete : undefined}
        />
      )}
      {entry.selectedChoiceText ? (
        <p className="text-[0.82rem] font-medium leading-6 text-amber-100/90 sm:text-[0.92rem]">
          你的选择：{entry.selectedChoiceText}
        </p>
      ) : null}
    </div>
  );
});

NarrationHistoryItem.displayName = 'NarrationHistoryItem';

const NarrationPanel: React.FC<NarrationPanelProps> = ({
  narrationHistory,
  currentRound,
  isAwaitingNarration,
  skipRestoredNarrationAnimation,
  onTypewriterComplete,
}) => {
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!skipRestoredNarrationAnimation || !scrollContainerRef.current) {
      return;
    }

    const frameId = window.requestAnimationFrame(() => {
      if (!scrollContainerRef.current) {
        return;
      }
      scrollContainerRef.current.scrollTop = scrollContainerRef.current.scrollHeight;
    });

    return () => window.cancelAnimationFrame(frameId);
  }, [currentRound, narrationHistory, skipRestoredNarrationAnimation]);

  return (
    <section className="akashic-panel flex h-[55dvh] shrink-0 flex-col p-2">
      <div className="flex min-h-0 flex-1 flex-col rounded-2xl bg-[#040912]/90 sm:rounded-[1.2rem] sm:pl-4 md:rounded-[1.3rem] md:pl-5">
        <div
          ref={scrollContainerRef}
          className="akashic-scroll min-h-0 flex-1 touch-pan-y overflow-y-auto"
        >
          <div className="h-full space-y-5 py-1 pr-2 text-[1rem] font-semibold leading-[1.82] text-[#f6eddc] sm:text-[1rem] md:text-[1.2rem]">
            {narrationHistory.map((entry) => {
              return (
                <NarrationHistoryItem
                  key={entry.round}
                  entry={entry}
                  isCurrentRound={entry.round === currentRound}
                  animateCurrentRound={
                    entry.round === currentRound && !skipRestoredNarrationAnimation
                  }
                  isFinished={entry.round !== currentRound || entry.narrationStatus === 'done'}
                  onComplete={onTypewriterComplete}
                />
              );
            })}
            {!narrationHistory.length && isAwaitingNarration ? (
              <p className="text-sm font-medium text-[#8f98ab]">
                {STREAM_PLACEHOLDER_TEXT}
              </p>
            ) : null}
          </div>
        </div>
      </div>
    </section>
  );
};

export default NarrationPanel;
