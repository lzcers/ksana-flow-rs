import React, { useEffect, useRef } from 'react';
import { Eye } from 'lucide-react';
import type { Choice } from '../lib/api';
import { SecondaryButton } from './AkashicUI';

interface ChoicePanelProps {
  hasChoices: boolean;
  choices: Choice[];
  previews: Record<string, string>;
  activeObsession: boolean;
  obsessionInput: string;
  isChoiceInteractionDisabled: boolean;
  isObsessionSubmitDisabled: boolean;
  onChoiceClick: (choiceId: string) => void | Promise<void>;
  onPreview: (choiceId: string, event: React.MouseEvent<HTMLButtonElement>) => void | Promise<void>;
  onObsessionInputChange: (value: string) => void;
  onObsessionSubmit: () => void | Promise<void>;
}

const ChoicePanel: React.FC<ChoicePanelProps> = ({
  hasChoices,
  choices,
  previews,
  activeObsession,
  obsessionInput,
  isChoiceInteractionDisabled,
  isObsessionSubmitDisabled,
  onChoiceClick,
  onPreview,
  onObsessionInputChange,
  onObsessionSubmit,
}) => {
  const obsessionInputRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (!activeObsession) {
      return;
    }

    obsessionInputRef.current?.focus();
  }, [activeObsession]);

  if (!hasChoices) {
    return null;
  }

  return (
    <div className="flex w-full">
      <div className="game-choices flex-1 rounded-[1.1rem] border border-[rgba(116,103,80,0.35)] bg-[rgba(5,11,22,0.55)] px-1.5 py-2">
        {!activeObsession ? (
          <div className="akashic-scroll max-h-[28dvh] space-y-1 overflow-y-auto pr-0.5 py-0.5">
            {choices.map((choice) => (
              <div key={choice.id} className="space-y-1.5">
                <div className="grid grid-cols-[minmax(0,1fr)_2.5rem] items-center gap-1.5">
                  <button
                    onClick={() => void onChoiceClick(choice.id)}
                    disabled={isChoiceInteractionDisabled || choice.disabled}
                    className="akashic-choice h-10 text-[#f3ead8] disabled:cursor-not-allowed disabled:opacity-50"
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
        ) : null}

        {activeObsession ? (
          <div className="space-y-2 rounded-[0.95rem] border border-red-400/30 bg-red-950/12 px-2 py-2.5">
            <textarea
              ref={obsessionInputRef}
              value={obsessionInput}
              onChange={(event) => onObsessionInputChange(event.target.value)}
              onKeyDown={(event) => {
                if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
                  event.preventDefault();
                  void onObsessionSubmit();
                }
              }}
              disabled={isChoiceInteractionDisabled}
              placeholder="例如：无视眼前的退路，直接闯入钟楼顶层，当面对质那名持灯人。"
              className="min-h-24 w-full resize-none rounded-[0.85rem] border border-red-300/25 bg-[rgba(16,8,14,0.72)] px-3 py-2 text-sm leading-5 text-[#f7efe2] outline-none transition-colors placeholder:text-red-100/35 focus:border-red-300/45 disabled:cursor-not-allowed disabled:opacity-60"
            />

            <div className="flex items-center justify-between gap-2">
              <p className="text-[0.68rem] leading-4 text-red-100/65 sm:text-[0.72rem]">
              </p>
              <SecondaryButton
                type="button"
                onClick={() => void onObsessionSubmit()}
                disabled={isObsessionSubmitDisabled}
                className="min-h-0 px-3 py-1.5 text-[0.72rem] leading-4 text-red-100 disabled:cursor-not-allowed disabled:opacity-60 sm:text-xs"
              >
                以执念行动
              </SecondaryButton>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
};

export default ChoicePanel;
