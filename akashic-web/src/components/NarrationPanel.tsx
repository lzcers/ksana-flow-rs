import React from 'react';
import Typewriter from './Typewriter';
import { STREAM_PLACEHOLDER_TEXT } from '../store/gameStoreHelpers';
import type { NarrationRoundEntry } from './gameplayTypes';

interface NarrationHistoryItemProps {
  entry: NarrationRoundEntry;
  isCurrentRound: boolean;
  isFinished: boolean;
  onComplete?: () => void;
}

interface NarrationPanelProps {
  narrationHistory: NarrationRoundEntry[];
  currentRound: number;
  isAwaitingNarration: boolean;
  onTypewriterComplete: () => void;
}

const NarrationHistoryItem: React.FC<NarrationHistoryItemProps> = React.memo(({
  entry,
  isCurrentRound,
  isFinished,
  onComplete,
}) => {
  return (
    <div className="space-y-2">
      <span className="text-xs font-medium tracking-[0.18em] text-[#8f98ab] uppercase">
        第 {entry.round} 轮
      </span>
      {entry.isAwaitingNarration && !entry.narrationText ? (
        <p className="text-sm font-medium text-[#8f98ab]">
          {STREAM_PLACEHOLDER_TEXT}
        </p>
      ) : (
        <Typewriter
          text={entry.narrationText}
          animate={isCurrentRound}
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
  onTypewriterComplete,
}) => {
  return (
    <section className="akashic-panel flex h-[55dvh] shrink-0 flex-col p-2">
      <div className="flex min-h-0 flex-1 flex-col rounded-2xl bg-[#040912]/90 sm:rounded-[1.2rem] sm:pl-4 md:rounded-[1.3rem] md:pl-5">
        <div className="akashic-scroll min-h-0 flex-1 overflow-y-auto">
          <div className="h-full space-y-5 py-1 pr-2 text-[1rem] font-semibold leading-[1.82] text-[#f6eddc] sm:text-[1rem] md:text-[1.2rem]">
            {narrationHistory.map((entry) => {
              return (
                <NarrationHistoryItem
                  key={entry.round}
                  entry={entry}
                  isCurrentRound={entry.round === currentRound}
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
