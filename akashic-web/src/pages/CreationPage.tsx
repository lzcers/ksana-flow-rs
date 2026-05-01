import React, { useState } from 'react';
import { useGameStore } from '../store/gameStore';
import { Dices, ArrowRight, ArrowLeft } from 'lucide-react';

const CreationPage: React.FC = () => {
  const { character, world, updateCharacter, updateWorld, setGameState, addStoryNode } = useGameStore();
  const [step, setStep] = useState(1);

  const handleRandomize = () => {
    updateCharacter({
      name: '亚瑟',
      gender: '男',
      age: 24,
      appearance: '银发红瞳，面带伤疤',
      traits: { courage: 80, rationality: 30, altruism: 40 },
      background: '背负诅咒的继承者'
    });
    updateWorld({
      era: '星际拓荒',
      coreConflict: '资源枯竭与人类存续',
    });
  };

  const handleStartGame = () => {
    // Mock the first story node
    addStoryNode({
      id: 'node-1',
      text: `你叫${character.name}，是一个${character.background}。在这个${world.era}的时代，${world.coreConflict}的阴影笼罩着每一个人。\n\n今天，你站在命运的十字路口。前方是未知的星辰，后方是即将崩塌的故乡。`,
      image: 'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20dramatic%20crossroads%20in%20a%20sci-fi%20or%20fantasy%20world%2C%20epic%20lighting%2C%20concept%20art&image_size=landscape_16_9',
      choices: [
        { id: 'c1', text: '头也不回地登上飞船，追寻新的希望' },
        { id: 'c2', text: '留下来，与故乡共存亡' }
      ]
    });
    setGameState('playing');
  };

  return (
    <div className="w-full h-full flex flex-col items-center justify-center p-4">
      <div className="glass-panel w-full max-w-lg p-6 md:p-8 rounded-2xl relative">
        <button 
          onClick={handleRandomize}
          className="absolute top-4 right-4 text-zinc-400 hover:text-white transition-colors"
          title="随机骰子"
        >
          <Dices className="w-6 h-6" />
        </button>

        {step === 1 && (
          <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
            <h2 className="text-2xl font-bold mb-6 tracking-wider">基础档案</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-zinc-400 mb-1">姓名</label>
                <input 
                  type="text" 
                  value={character.name}
                  onChange={(e) => updateCharacter({ name: e.target.value })}
                  className="w-full bg-black/50 border border-zinc-700 rounded-lg px-4 py-2 focus:outline-none focus:border-zinc-500"
                  placeholder="输入你的名字"
                />
              </div>
              <div className="flex gap-4">
                <div className="flex-1">
                  <label className="block text-sm text-zinc-400 mb-1">性别</label>
                  <select 
                    value={character.gender}
                    onChange={(e) => updateCharacter({ gender: e.target.value })}
                    className="w-full bg-black/50 border border-zinc-700 rounded-lg px-4 py-2 focus:outline-none focus:border-zinc-500"
                  >
                    <option>男</option>
                    <option>女</option>
                    <option>保密</option>
                  </select>
                </div>
                <div className="flex-1">
                  <label className="block text-sm text-zinc-400 mb-1">年龄</label>
                  <input 
                    type="number" 
                    value={character.age}
                    onChange={(e) => updateCharacter({ age: parseInt(e.target.value) || 18 })}
                    className="w-full bg-black/50 border border-zinc-700 rounded-lg px-4 py-2 focus:outline-none focus:border-zinc-500"
                  />
                </div>
              </div>
              <div>
                <label className="block text-sm text-zinc-400 mb-1">外貌特征</label>
                <input 
                  type="text" 
                  value={character.appearance}
                  onChange={(e) => updateCharacter({ appearance: e.target.value })}
                  className="w-full bg-black/50 border border-zinc-700 rounded-lg px-4 py-2 focus:outline-none focus:border-zinc-500"
                  placeholder="例如：银发红瞳，面带伤疤"
                />
              </div>
            </div>
            <div className="mt-8 flex justify-end">
              <button 
                onClick={() => setStep(2)}
                className="bg-zinc-100 text-black px-6 py-2 rounded-lg font-medium flex items-center gap-2 hover:bg-zinc-300 transition-colors"
              >
                下一步 <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
            <h2 className="text-2xl font-bold mb-6 tracking-wider">人格光谱与人生烙印</h2>
            <div className="space-y-6">
              <div>
                <div className="flex justify-between text-sm text-zinc-400 mb-2">
                  <span>谨慎</span>
                  <span className="text-zinc-200">勇气 ({character.traits.courage})</span>
                  <span>鲁莽</span>
                </div>
                <input 
                  type="range" 
                  min="0" max="100" 
                  value={character.traits.courage}
                  onChange={(e) => updateCharacter({ traits: { ...character.traits, courage: parseInt(e.target.value) } })}
                  className="w-full accent-zinc-100"
                />
              </div>
              <div>
                <div className="flex justify-between text-sm text-zinc-400 mb-2">
                  <span>感性</span>
                  <span className="text-zinc-200">理性 ({character.traits.rationality})</span>
                  <span>冷酷</span>
                </div>
                <input 
                  type="range" 
                  min="0" max="100" 
                  value={character.traits.rationality}
                  onChange={(e) => updateCharacter({ traits: { ...character.traits, rationality: parseInt(e.target.value) } })}
                  className="w-full accent-zinc-100"
                />
              </div>
              <div>
                <div className="flex justify-between text-sm text-zinc-400 mb-2">
                  <span>利己</span>
                  <span className="text-zinc-200">利他 ({character.traits.altruism})</span>
                  <span>圣母</span>
                </div>
                <input 
                  type="range" 
                  min="0" max="100" 
                  value={character.traits.altruism}
                  onChange={(e) => updateCharacter({ traits: { ...character.traits, altruism: parseInt(e.target.value) } })}
                  className="w-full accent-zinc-100"
                />
              </div>
              <div>
                <label className="block text-sm text-zinc-400 mb-1">人生烙印</label>
                <select 
                  value={character.background}
                  onChange={(e) => updateCharacter({ background: e.target.value })}
                  className="w-full bg-black/50 border border-zinc-700 rounded-lg px-4 py-2 focus:outline-none focus:border-zinc-500"
                >
                  <option value="">选择一个烙印...</option>
                  <option value="背负诅咒的继承者">背负诅咒的继承者</option>
                  <option value="失忆的流浪剑客">失忆的流浪剑客</option>
                  <option value="被遗弃的生化人">被遗弃的生化人</option>
                  <option value="寻梦的学者">寻梦的学者</option>
                </select>
              </div>
            </div>
            <div className="mt-8 flex justify-between">
              <button 
                onClick={() => setStep(1)}
                className="text-zinc-400 px-4 py-2 rounded-lg font-medium flex items-center gap-2 hover:bg-zinc-800 transition-colors"
              >
                <ArrowLeft className="w-4 h-4" /> 上一步
              </button>
              <button 
                onClick={() => setStep(3)}
                className="bg-zinc-100 text-black px-6 py-2 rounded-lg font-medium flex items-center gap-2 hover:bg-zinc-300 transition-colors"
              >
                下一步 <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}

        {step === 3 && (
          <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
            <h2 className="text-2xl font-bold mb-6 tracking-wider">世界观设定</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-zinc-400 mb-1">时代与舞台</label>
                <select 
                  value={world.era}
                  onChange={(e) => updateWorld({ era: e.target.value })}
                  className="w-full bg-black/50 border border-zinc-700 rounded-lg px-4 py-2 focus:outline-none focus:border-zinc-500"
                >
                  <option value="蒸汽朋克">蒸汽朋克</option>
                  <option value="星际拓荒">星际拓荒</option>
                  <option value="东方玄幻">东方玄幻</option>
                  <option value="末日废土">末日废土</option>
                </select>
              </div>
              <div>
                <label className="block text-sm text-zinc-400 mb-1">核心矛盾</label>
                <input 
                  type="text" 
                  value={world.coreConflict}
                  onChange={(e) => updateWorld({ coreConflict: e.target.value })}
                  className="w-full bg-black/50 border border-zinc-700 rounded-lg px-4 py-2 focus:outline-none focus:border-zinc-500"
                  placeholder="例如：资源枯竭与永生诱惑"
                />
              </div>
            </div>
            <div className="mt-8 flex justify-between">
              <button 
                onClick={() => setStep(2)}
                className="text-zinc-400 px-4 py-2 rounded-lg font-medium flex items-center gap-2 hover:bg-zinc-800 transition-colors"
              >
                <ArrowLeft className="w-4 h-4" /> 上一步
              </button>
              <button 
                onClick={handleStartGame}
                disabled={!character.name || !character.background || !world.coreConflict}
                className="bg-indigo-600 text-white px-6 py-2 rounded-lg font-medium flex items-center gap-2 hover:bg-indigo-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                步入幻世
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default CreationPage;
