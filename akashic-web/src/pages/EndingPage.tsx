import React, { useState } from 'react';
import { useGameStore } from '../store/gameStore';
import { Share2, RotateCcw, Image as ImageIcon } from 'lucide-react';

const EndingPage: React.FC = () => {
  const { endingData, resetGame, setGameState } = useGameStore();
  const [showFlashback, setShowFlashback] = useState<number | null>(null);

  if (!endingData) return null;

  const handleRestart = () => {
    resetGame();
    setGameState('lobby');
  };

  return (
    <div className="w-full h-full overflow-y-auto bg-zinc-950 text-zinc-300 pb-24">
      {/* Header Image */}
      <div 
        className="w-full h-[50vh] bg-cover bg-center relative"
        style={{ backgroundImage: `url("${endingData.cgs[2]}")` }}
      >
        <div className="absolute inset-0 bg-gradient-to-b from-transparent via-zinc-950/80 to-zinc-950" />
        <div className="absolute bottom-10 left-0 w-full text-center px-4">
          <h1 className="text-4xl md:text-6xl font-bold text-zinc-100 tracking-widest text-shadow mb-2">
            此生回响录
          </h1>
          <p className="text-zinc-400 tracking-wider">你的故事，已刻入阿卡夏记录</p>
        </div>
      </div>

      <div className="max-w-3xl mx-auto px-6 py-12 space-y-16">
        {/* Biography */}
        <section className="animate-in fade-in slide-in-from-bottom-8 duration-1000 delay-300 fill-mode-both">
          <h2 className="text-2xl font-bold text-zinc-100 mb-6 flex items-center gap-3">
            <span className="w-8 h-[1px] bg-zinc-600"></span>
            生平纪事
            <span className="w-8 h-[1px] bg-zinc-600"></span>
          </h2>
          <p className="text-lg leading-loose text-zinc-300 whitespace-pre-wrap">
            {endingData.biography}
          </p>
        </section>

        {/* Turning Points (Butterfly Effect) */}
        <section className="animate-in fade-in slide-in-from-bottom-8 duration-1000 delay-700 fill-mode-both">
          <h2 className="text-2xl font-bold text-zinc-100 mb-8 flex items-center gap-3">
            <span className="w-8 h-[1px] bg-zinc-600"></span>
            蝴蝶涟漪
            <span className="w-8 h-[1px] bg-zinc-600"></span>
          </h2>
          <div className="space-y-6 relative before:absolute before:inset-0 before:ml-5 before:-translate-x-px md:before:mx-auto md:before:translate-x-0 before:h-full before:w-0.5 before:bg-gradient-to-b before:from-transparent before:via-zinc-700 before:to-transparent">
            {endingData.turningPoints.map((point, index) => (
              <div key={index} className="relative flex items-center justify-between md:justify-normal md:odd:flex-row-reverse group is-active">
                {/* Icon */}
                <div className="flex items-center justify-center w-10 h-10 rounded-full border-4 border-zinc-950 bg-zinc-700 text-zinc-100 shadow shrink-0 md:order-1 md:group-odd:-translate-x-1/2 md:group-even:translate-x-1/2 z-10 cursor-pointer hover:bg-zinc-500 transition-colors"
                     onClick={() => setShowFlashback(showFlashback === index ? null : index)}>
                  <div className="w-2 h-2 bg-zinc-300 rounded-full" />
                </div>
                {/* Card */}
                <div className="w-[calc(100%-4rem)] md:w-[calc(50%-2.5rem)] p-4 rounded-xl border border-zinc-800 bg-zinc-900/50 backdrop-blur-sm">
                  <div className="mb-1 text-sm font-medium text-zinc-500">{point.cause}</div>
                  <div className="text-zinc-200">{point.effect}</div>
                  
                  {/* Flashback Easter Egg */}
                  {showFlashback === index && (
                    <div className="mt-4 pt-4 border-t border-zinc-800 text-sm text-indigo-300 animate-in fade-in slide-in-from-top-2">
                      <span className="font-bold">如果当时...</span><br/>
                      也许一切都会截然不同。平行宇宙中的你，或许正经历着另一种悲欢离合。
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* Legacy */}
        <section className="animate-in fade-in slide-in-from-bottom-8 duration-1000 delay-1000 fill-mode-both text-center">
          <h2 className="text-2xl font-bold text-zinc-100 mb-6">精神遗产</h2>
          <p className="text-xl italic text-zinc-400 bg-zinc-900/30 p-8 rounded-2xl border border-zinc-800/50">
            "{endingData.legacy}"
          </p>
        </section>

        {/* CG Gallery */}
        <section className="animate-in fade-in slide-in-from-bottom-8 duration-1000 delay-1000 fill-mode-both">
          <h2 className="text-2xl font-bold text-zinc-100 mb-6 flex items-center gap-3">
            <span className="w-8 h-[1px] bg-zinc-600"></span>
            记忆切片
            <span className="w-8 h-[1px] bg-zinc-600"></span>
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {endingData.cgs.map((cg, i) => (
              <div key={i} className="aspect-video rounded-lg overflow-hidden relative group">
                <img src={cg} alt={`Memory ${i+1}`} className="w-full h-full object-cover transition-transform duration-700 group-hover:scale-110" />
                <div className="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                  <ImageIcon className="text-white w-6 h-6" />
                </div>
              </div>
            ))}
          </div>
        </section>
      </div>

      {/* Floating Action Bar */}
      <div className="fixed bottom-0 left-0 w-full p-4 bg-gradient-to-t from-zinc-950 via-zinc-950 to-transparent z-50">
        <div className="max-w-md mx-auto flex gap-4">
          <button 
            className="flex-1 glass-panel py-3 px-6 rounded-xl flex items-center justify-center gap-2 text-zinc-200 hover:text-white hover:bg-white/10 transition-colors"
          >
            <Share2 className="w-5 h-5" />
            生成分享卡
          </button>
          <button 
            onClick={handleRestart}
            className="flex-1 glass-panel py-3 px-6 rounded-xl flex items-center justify-center gap-2 text-zinc-200 hover:text-white hover:bg-white/10 transition-colors"
          >
            <RotateCcw className="w-5 h-5" />
            重归大厅
          </button>
        </div>
      </div>
    </div>
  );
};

export default EndingPage;
