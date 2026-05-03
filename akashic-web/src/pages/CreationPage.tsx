import React from 'react';
import { ChevronDown, Dices, Sparkles, WandSparkles } from 'lucide-react';
import { useGameStore } from '../store/gameStore';
import {
  FieldLabel,
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StoryFrame,
  StatusPill,
} from '../components/AkashicUI';

const backgroundOptions = [
  '背负诅咒的继承者',
  '失忆的流浪剑客',
  '被遗弃的生化人',
  '寻梦的学者',
];

const eraOptions = ['蒸汽朋克', '星际拓荒', '东方玄幻', '末日废土'];

const CreationPage: React.FC = () => {
  const { character, world, updateCharacter, updateWorld, setGameState, addStoryNode } = useGameStore();

  const traitRows = [
    { key: 'courage', label: '勇气', value: character.traits.courage },
    { key: 'rationality', label: '理性', value: character.traits.rationality },
    { key: 'altruism', label: '利他', value: character.traits.altruism },
  ] as const;

  const handleRandomize = () => {
    updateCharacter({
      name: '艾莉亚',
      gender: '女',
      age: 24,
      appearance: '灰蓝眼瞳，佩着一枚旧怀表',
      traits: { courage: 60, rationality: 70, altruism: 45 },
      background: '失忆的流浪剑客',
    });
    updateWorld({
      era: '蒸汽朋克',
      coreConflict: '齿轮城的旧秩序正在崩塌，真相与忠诚互相撕扯',
    });
  };

  const handleStartGame = () => {
    addStoryNode({
      id: 'node-1',
      text: `雨水顺着屋檐滑落。你攥紧了怀中的黄铜怀表，那是唯一仍然忠于你的线索。今夜的${world.era}比往常更沉默，而“${world.coreConflict}”像薄雾一样笼住街道。\n\n你叫${character.name}，是${character.background}。门缝里透出的昏黄灯光，像某种召唤，也像一场试探。`,
      image:
        'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20dim%20steampunk%20district%20in%20the%20rain%2C%20warm%20light%20leaking%20from%20a%20tavern%20door%2C%20moody%20cinematic%20concept%20art&image_size=landscape_16_9',
      choices: [
        { id: 'c1', text: '推门进入酒馆，打听怀表的来历' },
        { id: 'c2', text: '绕到后巷，从侧门潜入观察' },
      ],
    });
    setGameState('playing');
  };

  const canStart = Boolean(character.name && character.background && world.coreConflict);

  return (
    <ScreenShell className="items-start md:items-center">
      <StoryFrame className="min-h-fit overflow-visible p-4 pb-[calc(env(safe-area-inset-bottom)+9rem)] sm:pb-4 md:p-6 md:pb-6">
        <div className="space-y-4 md:space-y-5">
          <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
            <div className="space-y-2">
              <h1 className="text-[2rem] font-semibold tracking-wide text-[#f6eddc] sm:text-[2.35rem] md:text-5xl">雕琢原点</h1>
              <p className="max-w-2xl text-sm leading-6 text-[#9ca7be] md:text-base">
                在命运开场前，先为主角定下姓名、性格与时代纹理。每一项设定都会回响到后续叙事。
              </p>
            </div>
            <div className="shrink-0 self-start">
              <SecondaryButton onClick={handleRandomize}>
                <Dices className="h-4 w-4" />
                随机骰子
              </SecondaryButton>
            </div>
          </div>

          <div className="flex flex-wrap gap-2 md:gap-3">
            <StatusPill icon={Sparkles}>角色塑形</StatusPill>
            <StatusPill icon={WandSparkles}>叙事开场</StatusPill>
          </div>

          <SectionCard className="p-4 md:p-6">
            <FieldLabel>姓名</FieldLabel>
            <div className="space-y-3 md:space-y-4">
              <input
                type="text"
                value={character.name}
                onChange={(e) => updateCharacter({ name: e.target.value })}
                className="akashic-field"
                placeholder="例如：艾莉亚"
              />
              <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_120px] md:gap-4">
                <div className="relative">
                  <FieldLabel>性别</FieldLabel>
                  <select
                    value={character.gender}
                    onChange={(e) => updateCharacter({ gender: e.target.value })}
                    className="akashic-select"
                  >
                    <option>男</option>
                    <option>女</option>
                    <option>保密</option>
                  </select>
                  <ChevronDown className="pointer-events-none absolute right-4 top-[3.15rem] h-4 w-4 -translate-y-1/2 text-[#c8b392] md:top-[3.4rem]" />
                </div>

                <div>
                  <FieldLabel>年龄</FieldLabel>
                  <input
                    type="number"
                    min="0"
                    value={character.age}
                    onChange={(e) => updateCharacter({ age: parseInt(e.target.value, 10) || 18 })}
                    className="akashic-field"
                  />
                </div>
              </div>
              <div>
                <FieldLabel hint="一句话描出角色气质">外貌 / 标记</FieldLabel>
                <input
                  type="text"
                  value={character.appearance}
                  onChange={(e) => updateCharacter({ appearance: e.target.value })}
                  className="akashic-field"
                  placeholder="例如：佩着旧怀表，眉眼带倦意"
                />
              </div>
            </div>
          </SectionCard>

          <SectionCard className="space-y-3 p-4 md:space-y-6 md:p-6">
            {traitRows.map((trait) => (
              <div key={trait.key}>
                <div className="mb-2 flex items-center justify-between text-[#efe4cd] sm:mb-2.5 md:mb-3">
                  <span className="text-[0.98rem] font-semibold leading-none sm:text-[1.22rem] md:text-[2.15rem]">{trait.label}</span>
                  <span className="text-[1.02rem] font-semibold text-[#d8c7aa] sm:text-[1.3rem] md:text-3xl">{trait.value}</span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={trait.value}
                  onChange={(e) =>
                    updateCharacter({
                      traits: {
                        ...character.traits,
                        [trait.key]: parseInt(e.target.value, 10),
                      },
                    })
                  }
                  className="akashic-range"
                  style={{
                    background: `linear-gradient(90deg, #1273ff 0%, #1273ff ${trait.value}%, rgba(255,255,255,0.92) ${trait.value}%, rgba(255,255,255,0.92) 100%)`,
                  }}
                />
              </div>
            ))}
          </SectionCard>

          <SectionCard className="p-4 md:p-6">
            <FieldLabel>人生烙印</FieldLabel>
            <div className="relative">
              <select
                value={character.background}
                onChange={(e) => updateCharacter({ background: e.target.value })}
                className="akashic-select"
              >
                <option value="">选择一个烙印...</option>
                {backgroundOptions.map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </select>
              <ChevronDown className="pointer-events-none absolute right-4 top-1/2 h-4 w-4 -translate-y-1/2 text-[#c8b392]" />
            </div>
          </SectionCard>

          <SectionCard className="space-y-3 p-4 md:space-y-4 md:p-6">
            <div>
              <FieldLabel>世界观 · 时代</FieldLabel>
              <div className="relative">
                <select
                  value={world.era}
                  onChange={(e) => updateWorld({ era: e.target.value })}
                  className="akashic-select"
                >
                  {eraOptions.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
                <ChevronDown className="pointer-events-none absolute right-4 top-1/2 h-4 w-4 -translate-y-1/2 text-[#c8b392]" />
              </div>
            </div>

            <div>
              <FieldLabel hint="用于决定开场问题与叙事张力">核心矛盾</FieldLabel>
              <input
                type="text"
                value={world.coreConflict}
                onChange={(e) => updateWorld({ coreConflict: e.target.value })}
                className="akashic-field"
                placeholder="例如：齿轮城的旧秩序正在崩塌"
              />
            </div>
          </SectionCard>

          <div className="fixed inset-x-3 bottom-[max(0.75rem,env(safe-area-inset-bottom))] z-20 flex flex-col gap-2 rounded-[1.2rem] border border-[#6f6655]/50 bg-[#0a1222]/94 p-2 shadow-[0_14px_36px_rgba(2,8,18,0.5)] backdrop-blur-xl sm:static sm:inset-auto sm:mt-1 sm:flex-row sm:justify-end sm:rounded-none sm:border-0 sm:bg-transparent sm:p-0 sm:shadow-none">
            <SecondaryButton onClick={() => setGameState('lobby')} className="w-full sm:w-auto">
              返回大厅
            </SecondaryButton>
            <PrimaryButton onClick={handleStartGame} disabled={!canStart} className="w-full sm:w-auto">
              步入幻世
            </PrimaryButton>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default CreationPage;
