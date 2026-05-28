import React, { useState } from 'react';
import { Eye, Flame, House, MoreHorizontal, Save, Share2 } from 'lucide-react';
import { SecondaryButton } from './AkashicUI';

interface GameplayToolbarProps {
  activeObsession: boolean;
  isObsessionToggleDisabled: boolean;
  obsessionPoints: number;
  intuitionPoints: number;
  onToggleObsession: () => void;
  onBackToLobby: () => void;
  onSave: () => void | Promise<void>;
  onShare: () => void;
}

const GameplayToolbar: React.FC<GameplayToolbarProps> = ({
  activeObsession,
  isObsessionToggleDisabled,
  obsessionPoints,
  intuitionPoints,
  onToggleObsession,
  onBackToLobby,
  onSave,
  onShare,
}) => {
  const [isUtilityMenuOpen, setIsUtilityMenuOpen] = useState(false);

  return (
    <div className="game-opts inset-x-0 rounded-full border border-[rgba(116,103,80,0.4)] bg-[rgba(8,14,26,0.82)] px-2 py-2 backdrop-blur-md">
      <div className="relative flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <SecondaryButton
            onClick={onToggleObsession}
            className={`min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs ${activeObsession ? 'border-red-300/50 bg-red-950/25 text-red-100' : ''}`}
            disabled={isObsessionToggleDisabled}
          >
            <Flame className={`h-3.5 w-3.5 ${activeObsession ? 'animate-pulse' : ''}`} />
            执念
          </SecondaryButton>
        </div>
        <div className="flex items-center gap-2">
          <span className="inline-flex items-center gap-1 text-[0.72rem] leading-4 text-[#d9cbb1] sm:text-xs">
            <Flame className="h-3.5 w-3.5" />
            <span>{obsessionPoints}</span>
          </span>
          <span className="text-[0.72rem] leading-4 text-[#8f98ab] sm:text-xs">|</span>
          <span className="inline-flex items-center gap-1 text-[0.72rem] leading-4 text-[#d9cbb1] sm:text-xs">
            <Eye className="h-3.5 w-3.5" />
            <span>{`${intuitionPoints}/2`}</span>
          </span>
        </div>
        <div className="relative">
          <SecondaryButton
            type="button"
            onClick={() => setIsUtilityMenuOpen((prev) => !prev)}
            className="min-h-0 gap-1.5 px-2.5 py-1.5 text-[0.72rem] leading-4 sm:text-xs"
          >
            <MoreHorizontal className="h-3.5 w-3.5" />
            菜单
          </SecondaryButton>
          {isUtilityMenuOpen ? (
            <div className="absolute bottom-[calc(100%+0.45rem)] right-0 z-20 min-w-[8.8rem] rounded-[0.95rem] border border-[rgba(116,103,80,0.5)] bg-[rgba(7,13,24,0.96)] p-1.5 shadow-[0_10px_24px_rgba(0,0,0,0.45)]">
              <button
                type="button"
                onClick={() => {
                  onBackToLobby();
                  setIsUtilityMenuOpen(false);
                }}
                className="flex w-full items-center gap-1.5 rounded-[0.7rem] px-2 py-1.5 text-left text-[0.72rem] leading-4 text-[#f3ead8] transition-colors hover:bg-[rgba(188,169,124,0.14)] sm:text-xs"
              >
                <House className="h-3.5 w-3.5" />
                返回大厅
              </button>
              <button
                type="button"
                onClick={() => {
                  void onSave();
                  setIsUtilityMenuOpen(false);
                }}
                className="flex w-full items-center gap-1.5 rounded-[0.7rem] px-2 py-1.5 text-left text-[0.72rem] leading-4 text-[#f3ead8] transition-colors hover:bg-[rgba(188,169,124,0.14)] sm:text-xs"
              >
                <Save className="h-3.5 w-3.5" />
                存档
              </button>
              <button
                type="button"
                onClick={() => {
                  onShare();
                  setIsUtilityMenuOpen(false);
                }}
                className="flex w-full items-center gap-1.5 rounded-[0.7rem] px-2 py-1.5 text-left text-[0.72rem] leading-4 text-[#f3ead8] transition-colors hover:bg-[rgba(188,169,124,0.14)] sm:text-xs"
              >
                <Share2 className="h-3.5 w-3.5" />
                分享
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
};

export default GameplayToolbar;
