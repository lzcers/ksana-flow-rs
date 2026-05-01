import React, { useState } from 'react';
import { useGameStore } from '../store/gameStore';
import Typewriter from '../components/Typewriter';
import { Settings, Save, History, Users, Flame, Eye, Globe } from 'lucide-react';

const GameplayPage: React.FC = () => {
  const { storyNodes, currentNodeId, makeChoice, obsessionPoints, intuitionPoints, worldNews, useIntuition, useObsession } = useGameStore();
  const [isTyping, setIsTyping] = useState(true);
  const [activeObsession, setActiveObsession] = useState(false);
  const [previews, setPreviews] = useState<Record<string, string>>({});

  const currentNode = storyNodes.find(n => n.id === currentNodeId);

  if (!currentNode) return null;

  const handlePreview = (choiceId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (previews[choiceId]) return;
    if (useIntuition()) {
      // Mocking a blurry preview text snippet based on choice text
      const previewText = `未来的模糊片段：你隐约看到，由于这个选择，将会面临意想不到的转折...`;
      setPreviews(prev => ({ ...prev, [choiceId]: previewText }));
    } else {
      alert("直觉值不足");
    }
  };

  const handleChoiceClick = (choiceId: string) => {
    if (activeObsession) {
      if (useObsession()) {
         makeChoice(choiceId, true);
      } else {
         alert("执念点数不足");
         return;
      }
    } else {
      makeChoice(choiceId, false);
    }
    setIsTyping(true);
    setActiveObsession(false);
    setPreviews({});
  };

  return (
    <div className="w-full h-full flex flex-col relative overflow-hidden bg-black">
      {/* World News Toast */}
      {worldNews && (
        <div className="absolute top-20 left-1/2 -translate-x-1/2 z-50 animate-in fade-in slide-in-from-top-4 duration-500">
          <div className="glass-panel px-6 py-3 rounded-full flex items-center gap-3 border border-amber-500/30 bg-black/60 shadow-[0_0_15px_rgba(245,158,11,0.2)]">
            <Globe className="w-5 h-5 text-amber-400 animate-pulse" />
            <span className="text-amber-100 text-sm font-medium tracking-wide">{worldNews}</span>
          </div>
        </div>
      )}

      {/* Background Image / Dynamic Scene */}
      <div 
        className="absolute inset-0 bg-cover bg-center opacity-60 transition-all duration-1000"
        style={{ backgroundImage: `url("${currentNode.image}")` }}
      />
      <div className="absolute inset-0 bg-gradient-to-t from-black via-black/50 to-transparent" />

      {/* Top Quick Actions Bar */}
      <div className="relative z-10 flex justify-between items-center p-4">
        <div className="flex flex-col gap-2">
          <div className="glass-panel px-4 py-2 rounded-full flex gap-4">
            <button className="text-zinc-300 hover:text-white" title="存档"><Save className="w-5 h-5" /></button>
            <button className="text-zinc-300 hover:text-white" title="历史记录"><History className="w-5 h-5" /></button>
            <button className="text-zinc-300 hover:text-white" title="人物关系"><Users className="w-5 h-5" /></button>
          </div>
          
          {/* Resource Indicators */}
          <div className="flex gap-2 px-2">
            <div className="flex items-center gap-1.5 text-red-400 bg-black/40 px-3 py-1 rounded-full border border-red-900/50" title="执念点数">
              <Flame className="w-4 h-4" />
              <span className="text-sm font-bold">{obsessionPoints}</span>
            </div>
            <div className="flex items-center gap-1.5 text-cyan-400 bg-black/40 px-3 py-1 rounded-full border border-cyan-900/50" title="直觉值">
              <Eye className="w-4 h-4" />
              <span className="text-sm font-bold">{intuitionPoints}</span>
            </div>
          </div>
        </div>
        <button className="glass-panel p-2 rounded-full text-zinc-300 hover:text-white self-start">
          <Settings className="w-5 h-5" />
        </button>
      </div>

      {/* Story Text Area */}
      <div className="relative z-10 flex-1 flex flex-col justify-end p-4 md:p-8 pb-40">
        <div className="glass-panel p-6 md:p-8 rounded-2xl w-full max-w-4xl mx-auto shadow-2xl transition-all duration-500">
          <div className="text-lg md:text-xl leading-relaxed text-zinc-100 min-h-[150px]">
            <Typewriter 
              text={currentNode.text} 
              speed={40} 
              onComplete={() => setIsTyping(false)} 
            />
          </div>
        </div>
      </div>

      {/* Choices Area (Bottom for Thumb Operation) */}
      <div className="absolute bottom-0 left-0 w-full p-4 md:p-8 bg-gradient-to-t from-black via-black/90 to-transparent z-20">
        <div className="max-w-4xl mx-auto flex flex-col gap-3">
          
          {!isTyping && (
            <div className="flex justify-end mb-2">
              <button 
                onClick={() => setActiveObsession(!activeObsession)}
                className={`flex items-center gap-2 px-4 py-2 rounded-full border transition-all ${activeObsession ? 'bg-red-900/40 border-red-500 text-red-400 shadow-[0_0_10px_rgba(239,68,68,0.3)]' : 'bg-black/50 border-zinc-700 text-zinc-400 hover:text-zinc-200'}`}
              >
                <Flame className={`w-4 h-4 ${activeObsession ? 'animate-pulse' : ''}`} />
                <span className="text-sm">倾注执念</span>
              </button>
            </div>
          )}

          {!isTyping && currentNode.choices.map((choice) => (
            <div key={choice.id} className="relative group">
              <button
                onClick={() => handleChoiceClick(choice.id)}
                className={`w-full glass-panel py-4 px-6 rounded-xl text-left transition-all active:scale-[0.98] ${activeObsession ? 'border-red-900/50 hover:border-red-500/80 hover:bg-red-950/20' : 'border-zinc-700/50 hover:bg-white/10 hover:border-white/20'}`}
              >
                <div className="flex justify-between items-center">
                  <span className={`text-lg ${activeObsession ? 'text-red-100' : 'text-zinc-200'}`}>{choice.text}</span>
                  
                  {!previews[choice.id] && (
                    <div 
                      onClick={(e) => handlePreview(choice.id, e)}
                      className="p-2 rounded-full bg-cyan-900/30 text-cyan-400 hover:bg-cyan-800/50 transition-colors border border-cyan-800/50 cursor-pointer"
                      title="消耗 1 直觉值窥探命运"
                    >
                      <Eye className="w-4 h-4" />
                    </div>
                  )}
                </div>
                
                {previews[choice.id] && (
                  <div className="mt-3 p-3 rounded-lg bg-black/40 border border-cyan-900/50 text-cyan-200 text-sm italic relative overflow-hidden">
                    <div className="absolute inset-0 backdrop-blur-[2px] z-0"></div>
                    <span className="relative z-10">{previews[choice.id]}</span>
                  </div>
                )}
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default GameplayPage;
