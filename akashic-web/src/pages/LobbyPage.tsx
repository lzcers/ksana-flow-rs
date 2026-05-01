import React from 'react';
import { useGameStore } from '../store/gameStore';
import { Play, BookOpen } from 'lucide-react';

const LobbyPage: React.FC = () => {
  const setGameState = useGameStore((state) => state.setGameState);
  const resetGame = useGameStore((state) => state.resetGame);

  const handleStart = () => {
    resetGame();
    setGameState('creation');
  };

  return (
    <div className="relative w-full h-full flex flex-col items-center justify-center overflow-hidden">
      {/* Dynamic Background Placeholder */}
      <div 
        className="absolute inset-0 bg-cover bg-center bg-no-repeat opacity-40 transition-transform duration-[20000ms] hover:scale-110"
        style={{ backgroundImage: 'url("https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20mystical%20ancient%20library%20with%20glowing%20floating%20books%20and%20ethereal%20dust%2C%20dark%20fantasy%20style%2C%20cinematic%20lighting&image_size=landscape_16_9")' }}
      />
      
      <div className="relative z-10 flex flex-col items-center max-w-4xl w-full px-6">
        <h1 className="text-5xl md:text-7xl font-bold mb-4 tracking-wider text-transparent bg-clip-text bg-gradient-to-b from-zinc-100 to-zinc-500 text-shadow">
          幻世·人生回响
        </h1>
        <p className="text-zinc-400 text-lg md:text-xl mb-12 tracking-widest">
          每一次选择，都在规则世界中真实发生
        </p>

        <div className="flex flex-col sm:flex-row gap-6 w-full max-w-md">
          <button
            onClick={handleStart}
            className="glass flex-1 py-4 px-6 rounded-xl flex items-center justify-center gap-3 text-lg font-medium hover:bg-white/10 transition-all hover:scale-105 active:scale-95"
          >
            <Play className="w-5 h-5" />
            开启新人生
          </button>
          
          <button
            className="glass flex-1 py-4 px-6 rounded-xl flex items-center justify-center gap-3 text-lg font-medium hover:bg-white/10 transition-all hover:scale-105 active:scale-95 text-zinc-300"
          >
            <BookOpen className="w-5 h-5" />
            我的回廊
          </button>
        </div>
      </div>

      {/* Decorative elements */}
      <div className="absolute bottom-10 left-0 w-full text-center text-xs text-zinc-600 tracking-widest">
        AKASHIC ENGINE V1.1
      </div>
    </div>
  );
};

export default LobbyPage;
