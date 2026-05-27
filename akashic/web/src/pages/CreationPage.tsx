import React from 'react';
import { ChevronDown, TriangleAlert } from 'lucide-react';
import { useGameUIStore } from '../store/gameUIStore';
import {
  FieldLabel,
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

const ATTRIBUTE_TOTAL = 30;
const ATTRIBUTE_MIN = 1;
const ATTRIBUTE_MAX = 10;

const traitConfigs = [
  { key: 'intellect', label: '智力', hint: '理解、推演与拆解复杂问题的能力。' },
  { key: 'physique', label: '体力', hint: '爆发、搏斗与承受高强度行动的能力。' },
  { key: 'endurance', label: '耐力', hint: '在漫长压力下维持状态与行动的能力。' },
  { key: 'courage', label: '勇气', hint: '在危险与未知面前仍愿意迈步的倾向。' },
  { key: 'rationality', label: '理性', hint: '在情绪与局势之间保持判断与克制。' },
  { key: 'altruism', label: '利他', hint: '是否愿意把他人的代价纳入自己的选择。' },
] as const;

type TraitKey = (typeof traitConfigs)[number]['key'];

const CreationPage: React.FC = () => {
  const {
    character,
    world,
    story,
    updateCharacter,
    updateWorld,
    updateStory,
    setGameState,
    startGame,
    isLoading,
    error,
  } = useGameUIStore();

  const traitRows = traitConfigs.map((trait) => ({
    ...trait,
    value: character.traits[trait.key],
  }));
  const totalAllocatedPoints = traitRows.reduce((sum, trait) => sum + trait.value, 0);
  const remainingPoints = ATTRIBUTE_TOTAL - totalAllocatedPoints;
  const radarSize = 260;
  const radarCenter = radarSize / 2;
  const radarRadius = 78;
  const radarLabelRadius = 105;
  const radarAngles = traitRows.map((_, index) => (-Math.PI / 2) + ((Math.PI * 2 * index) / traitRows.length));
  const radarShapePoints = traitRows.map((trait, index) => {
    const ratio = trait.value / ATTRIBUTE_MAX;
    const x = radarCenter + (Math.cos(radarAngles[index]) * radarRadius * ratio);
    const y = radarCenter + (Math.sin(radarAngles[index]) * radarRadius * ratio);
    return `${x},${y}`;
  }).join(' ');

  const canStart = Boolean(
    character.name.trim()
    && character.background
    && world.description.trim()
    && story.theme.trim()
    && story.atmosphere.trim()
    && story.narrativeStyle.trim()
    && remainingPoints === 0,
  );

  const handleStartGame = async () => {
    try {
      await startGame();
    } catch {
      // Store already keeps the error message for UI surfaces elsewhere.
    }
  };

  const handleTraitChange = (key: TraitKey, rawValue: number) => {
    const nextValue = Math.max(ATTRIBUTE_MIN, Math.min(ATTRIBUTE_MAX, rawValue));
    const currentValue = character.traits[key];
    const nextTotal = totalAllocatedPoints - currentValue + nextValue;
    if (nextTotal > ATTRIBUTE_TOTAL) {
      return;
    }

    updateCharacter({
      traits: {
        ...character.traits,
        [key]: nextValue,
      },
    });
  };

  return (
    <ScreenShell className="items-start md:items-center">
      <StoryFrame className="min-h-fit overflow-visible p-4 pb-[calc(env(safe-area-inset-bottom)+9rem)] sm:pb-4 md:p-6 md:pb-6">
        <div className="space-y-6 md:space-y-7">
          {error ? (
            <StatusPill
              icon={TriangleAlert}
              className="border-[#7f3b3b]/50 bg-[#2a1216]/85 text-[#ffd7d7]"
              iconClassName="text-[#ff9b9b]"
            >
              {error}
            </StatusPill>
          ) : null}

          <section className="space-y-4">
            <div className="space-y-1.5">
              <p className="text-xs uppercase tracking-[0.28em] text-[#8f98ab]">人物设定</p>
              <h2 className="text-xl font-semibold text-[#f6eddc] md:text-2xl">塑造主角轮廓</h2>
              <p className="max-w-3xl text-sm leading-6 text-[#9ca7be]">
                先确定主角是谁，再用总计 30 点的人物属性分配，勾勒他在这个故事里会怎样行动、判断与承受代价。
              </p>
            </div>

            <SectionCard className="space-y-4 p-4 md:p-6">
              <div className="grid gap-3 md:grid-cols-[minmax(0,1.2fr)_140px_140px] md:gap-4">
                <div>
                  <FieldLabel>姓名</FieldLabel>
                  <input
                    type="text"
                    value={character.name}
                    onChange={(e) => updateCharacter({ name: e.target.value })}
                    className="akashic-field"
                    placeholder="例如：艾莉亚"
                  />
                </div>

                <div className="relative">
                  <FieldLabel>性别</FieldLabel>
                  <select
                    value={character.gender}
                    onChange={(e) => updateCharacter({ gender: e.target.value })}
                    className="akashic-select"
                  >
                    <option>保密</option>
                    <option>男</option>
                    <option>女</option>
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
                <FieldLabel hint="一句话描出角色气质">人物描述</FieldLabel>
                <input
                  type="text"
                  value={character.appearance}
                  onChange={(e) => updateCharacter({ appearance: e.target.value })}
                  className="akashic-field"
                  placeholder="例如：佩着旧怀表，眉眼带倦意"
                />
              </div>

              <div>
                <FieldLabel hint="决定主角进入命运时带着怎样的前史与宿命感">命运烙印</FieldLabel>
                <div className="relative">
                  <select
                    value={character.background}
                    onChange={(e) => updateCharacter({ background: e.target.value })}
                    className="akashic-select"
                  >
                    <option value="">选择一种命运烙印...</option>
                    {backgroundOptions.map((option) => (
                      <option key={option} value={option}>
                        {option}
                      </option>
                    ))}
                  </select>
                  <ChevronDown className="pointer-events-none absolute right-4 top-1/2 h-4 w-4 -translate-y-1/2 text-[#c8b392]" />
                </div>
              </div>
            </SectionCard>

            <SectionCard className="space-y-5 p-4 md:p-6">
              <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                <div className="space-y-1">
                  <FieldLabel hint="六项属性共 30 点，每项至少 1 点、至多 10 点">人物属性</FieldLabel>
                  <p className="text-sm leading-6 text-[#9ca7be]">
                    身体与精神共同塑造主角。先压低一项，再把点数转移给另一项。
                  </p>
                </div>
                <StatusPill icon={null} className="border-[#365f8a]/35 bg-[#0f1c31]/70 text-[#d7e5ff]">
                  {remainingPoints === 0 ? '点数已分配完成' : `剩余 ${remainingPoints} 点待分配`}
                </StatusPill>
              </div>

              <div className="grid gap-6 md:grid-cols-[280px_minmax(0,1fr)] md:items-start">
                <div className="mx-auto w-full max-w-[280px] rounded-[1.2rem] border border-white/8 bg-[#0d1627]/72 p-3">
                  <svg viewBox={`0 0 ${radarSize} ${radarSize}`} className="h-auto w-full">
                    {[0.25, 0.5, 0.75, 1].map((ratio) => (
                      <polygon
                        key={ratio}
                        points={traitRows.map((_, index) => {
                          const x = radarCenter + (Math.cos(radarAngles[index]) * radarRadius * ratio);
                          const y = radarCenter + (Math.sin(radarAngles[index]) * radarRadius * ratio);
                          return `${x},${y}`;
                        }).join(' ')}
                        fill="none"
                        stroke="rgba(143,152,171,0.28)"
                        strokeWidth="1"
                      />
                    ))}
                    {traitRows.map((trait, index) => {
                      const axisX = radarCenter + (Math.cos(radarAngles[index]) * radarRadius);
                      const axisY = radarCenter + (Math.sin(radarAngles[index]) * radarRadius);
                      const labelX = radarCenter + (Math.cos(radarAngles[index]) * radarLabelRadius);
                      const labelY = radarCenter + (Math.sin(radarAngles[index]) * radarLabelRadius);
                      return (
                        <g key={trait.key}>
                          <line
                            x1={radarCenter}
                            y1={radarCenter}
                            x2={axisX}
                            y2={axisY}
                            stroke="rgba(143,152,171,0.28)"
                            strokeWidth="1"
                          />
                          <text
                            x={labelX}
                            y={labelY}
                            fill="#efe4cd"
                            fontSize="12"
                            textAnchor={labelX < radarCenter - 8 ? 'end' : labelX > radarCenter + 8 ? 'start' : 'middle'}
                            dominantBaseline="middle"
                          >
                            {trait.label}
                          </text>
                        </g>
                      );
                    })}
                    <polygon
                      points={radarShapePoints}
                      fill="rgba(84,160,255,0.24)"
                      stroke="#7dd3fc"
                      strokeWidth="2"
                    />
                    {traitRows.map((trait, index) => {
                      const ratio = trait.value / ATTRIBUTE_MAX;
                      const x = radarCenter + (Math.cos(radarAngles[index]) * radarRadius * ratio);
                      const y = radarCenter + (Math.sin(radarAngles[index]) * radarRadius * ratio);
                      return <circle key={`${trait.key}-point`} cx={x} cy={y} r="3.5" fill="#f6eddc" />;
                    })}
                  </svg>
                </div>

                <div className="space-y-4">
                  {traitRows.map((trait) => (
                    <div key={trait.key} className="rounded-[1rem] border border-white/8 bg-[#0c1422]/58 p-3 md:p-4">
                      <div className="mb-2 flex items-center justify-between gap-3 text-[#efe4cd]">
                        <div>
                          <p className="text-base font-semibold md:text-lg">{trait.label}</p>
                          <p className="text-xs leading-5 text-[#9ca7be] md:text-sm">{trait.hint}</p>
                        </div>
                        <span className="text-xl font-semibold text-[#d8c7aa] md:text-2xl">{trait.value}</span>
                      </div>
                      <input
                        type="range"
                        min={ATTRIBUTE_MIN}
                        max={ATTRIBUTE_MAX}
                        step="1"
                        value={trait.value}
                        onChange={(e) => handleTraitChange(trait.key, parseInt(e.target.value, 10))}
                        className="akashic-range"
                        style={{
                          background: `linear-gradient(90deg, #1273ff 0%, #1273ff ${((trait.value - ATTRIBUTE_MIN) / (ATTRIBUTE_MAX - ATTRIBUTE_MIN)) * 100}%, rgba(255,255,255,0.92) ${((trait.value - ATTRIBUTE_MIN) / (ATTRIBUTE_MAX - ATTRIBUTE_MIN)) * 100}%, rgba(255,255,255,0.92) 100%)`,
                        }}
                      />
                    </div>
                  ))}
                </div>
              </div>
            </SectionCard>
          </section>

          <section className="space-y-4">
            <div className="space-y-1.5">
              <p className="text-xs uppercase tracking-[0.28em] text-[#8f98ab]">世界设定</p>
              <h2 className="text-xl font-semibold text-[#f6eddc] md:text-2xl">勾勒故事舞台</h2>
              <p className="max-w-3xl text-sm leading-6 text-[#9ca7be]">
                这一部分只描述客观世界本身：它处在什么时代、怎样运转、什么裂缝正在悄悄逼近所有人。
              </p>
            </div>

            <SectionCard className="space-y-4 p-4 md:p-6">
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
                <FieldLabel hint="一句话写出世界的现实压力、运转逻辑或正在扩大的裂缝">世界描述</FieldLabel>
                <textarea
                  value={world.description}
                  onChange={(e) => updateWorld({ description: e.target.value })}
                  className="akashic-field min-h-28 resize-y"
                  placeholder="例如：旧帝国用记忆税维持城市运转，越靠近中心城区，失去的自我就越多。"
                />
              </div>
            </SectionCard>
          </section>

          <section className="space-y-4">
            <div className="space-y-1.5">
              <p className="text-xs uppercase tracking-[0.28em] text-[#8f98ab]">故事设定</p>
              <h2 className="text-xl font-semibold text-[#f6eddc] md:text-2xl">写下你的叙事期待</h2>
              <p className="max-w-3xl text-sm leading-6 text-[#9ca7be]">
                这不是补充世界事实，而是在告诉幻世：你想在这个舞台上看到怎样的题材、氛围、文风，以及绝不希望碰见什么。
              </p>
            </div>

            <SectionCard className="space-y-4 p-4 md:p-6">
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <FieldLabel hint="决定故事更像悬疑、成长、悲剧、救赎或权谋等">题材 / 主题</FieldLabel>
                  <input
                    type="text"
                    value={story.theme}
                    onChange={(e) => updateStory({ theme: e.target.value })}
                    className="akashic-field"
                    placeholder="例如：阴谋悬疑中的自我救赎"
                  />
                </div>

                <div>
                  <FieldLabel hint="描述你希望故事整体带来的情绪温度">故事氛围</FieldLabel>
                  <input
                    type="text"
                    value={story.atmosphere}
                    onChange={(e) => updateStory({ atmosphere: e.target.value })}
                    className="akashic-field"
                    placeholder="例如：压抑、潮湿、危险中偶有微光"
                  />
                </div>
              </div>

              <div>
                <FieldLabel hint="告诉系统要用怎样的叙事笔触来展开剧情">叙事风格</FieldLabel>
                <input
                  type="text"
                  value={story.narrativeStyle}
                  onChange={(e) => updateStory({ narrativeStyle: e.target.value })}
                  className="akashic-field"
                  placeholder="例如：冷峻克制，偏文学叙事，少解释多暗示"
                />
              </div>

              <div>
                <FieldLabel hint="写下不想出现的题材、桥段、关系或叙事手法">避雷 / 禁区</FieldLabel>
                <textarea
                  value={story.taboos}
                  onChange={(e) => updateStory({ taboos: e.target.value })}
                  className="akashic-field min-h-24 resize-y"
                  placeholder="例如：不要校园恋爱、不要无代价复活、避免轻浮搞笑消解压迫感"
                />
              </div>
            </SectionCard>
          </section>

          <div className="fixed inset-x-3 bottom-[max(0.75rem,env(safe-area-inset-bottom))] z-20 flex flex-col gap-2 rounded-[1.2rem] border border-[#6f6655]/50 bg-[#0a1222]/94 p-2 shadow-[0_14px_36px_rgba(2,8,18,0.5)] backdrop-blur-xl sm:static sm:inset-auto sm:mt-1 sm:flex-row sm:justify-end sm:rounded-none sm:border-0 sm:bg-transparent sm:p-0 sm:shadow-none">
            <SecondaryButton onClick={() => setGameState('lobby')} className="w-full sm:w-auto">
              返回大厅
            </SecondaryButton>
            <PrimaryButton onClick={handleStartGame} disabled={!canStart || isLoading} className="w-full sm:w-auto">
              {isLoading ? '设定生成中...' : '生成命运开局'}
            </PrimaryButton>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default CreationPage;
