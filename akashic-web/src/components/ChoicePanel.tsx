import React from 'react';
import { Eye } from 'lucide-react';
import type { Choice } from '../lib/api';

interface ChoicePanelProps {
  hasChoices: boolean;
  choices: Choice[];
  previews: Record<string, string>;
  activeObsession: boolean;
  isChoiceInteractionDisabled: boolean;
  onChoiceClick: (choiceId: string) => void | Promise<void>;
  onPreview: (choiceId: string, event: React.MouseEvent<HTMLButtonElement>) => void | Promise<void>;
}

const ChoicePanel: React.FC<ChoicePanelProps> = ({
  hasChoices,
  choices,
  previews,
  activeObsession,
  isChoiceInteractionDisabled,
  onChoiceClick,
  onPreview,
}) => {
  if (!hasChoices) {
    return null;
  }

  return (
    <div className="flex w-full">
      <div className="game-choices flex-1 rounded-[1.1rem] border border-[rgba(116,103,80,0.35)] bg-[rgba(5,11,22,0.55)] px-1.5 py-2">
        <div className="akashic-scroll max-h-[28dvh] space-y-1 overflow-y-auto pr-0.5 py-0.5">
          {choices.map((choice) => (
            <div key={choice.id} className="space-y-1.5">
              <div className="grid grid-cols-[minmax(0,1fr)_2.5rem] items-center gap-1.5">
                <button
                  onClick={() => void onChoiceClick(choice.id)}
                  disabled={isChoiceInteractionDisabled || choice.disabled}
                  className={`akashic-choice h-10 disabled:cursor-not-allowed disabled:opacity-50 ${activeObsession ? 'border-red-400/45 bg-red-950/20 text-red-100' : 'text-[#f3ead8]'}`}
                >
                  <div className="flex min-h-7 items-center text-left">
                    <div className="w-full text-sm font-semibold leading-5 sm:text-[0.95rem]">
                      {choice.text}
                    </div>
                  </div>
                </button>

                <button
                  type="button"
                  onClick={(event) => void onPreview(choice.id, event)}
                  disabled={isChoiceInteractionDisabled}
                  className="akashic-icon-btn h-10 min-h-10 w-10 self-auto disabled:cursor-not-allowed disabled:opacity-50"
                  title="消耗 1 点直觉，窥探命运碎片"
                >
                  <Eye className="h-4 w-4" />
                </button>
              </div>

              {previews[choice.id] ? (
                <div className="rounded-[0.8rem] border border-cyan-400/20 bg-cyan-950/10 px-2 py-2 text-[0.7rem] leading-4.5 text-cyan-100/90 sm:rounded-[0.95rem] sm:px-2 sm:py-2 sm:text-xs">
                  {previews[choice.id]}
                </div>
              ) : null}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default ChoicePanel;
