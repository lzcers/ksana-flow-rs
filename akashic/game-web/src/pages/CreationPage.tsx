import React from 'react';
import { ChevronDown, TriangleAlert } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
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
import { appRoutes } from '../lib/appRoutes';

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

interface SearchableSelectProps {
  value: string;
  options: readonly string[];
  placeholder: string;
  emptyText: string;
  createText: string;
  onChange: (value: string) => void;
}

const SearchableSelect: React.FC<SearchableSelectProps> = ({
  value,
  options,
  placeholder,
  emptyText,
  createText,
  onChange,
}) => {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [isOpen, setIsOpen] = React.useState(false);

  React.useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    return () => document.removeEventListener('mousedown', handlePointerDown);
  }, []);

  const trimmedValue = value.trim();
  const filteredOptions = React.useMemo(() => {
    if (!trimmedValue) {
      return options;
    }

    return options.filter((option) => option.toLowerCase().includes(trimmedValue.toLowerCase()));
  }, [options, trimmedValue]);
  const hasExactMatch = options.some((option) => option === trimmedValue);
  const canKeepCustomValue = trimmedValue.length > 0 && !hasExactMatch;

  return (
    <div ref={containerRef} className="relative">
      <div className="relative">
        <input
          type="text"
          value={value}
          onFocus={() => setIsOpen(true)}
          onChange={(e) => {
            onChange(e.target.value);
            setIsOpen(true);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              setIsOpen(false);
            }
            if (e.key === 'Enter') {
              setIsOpen(false);
            }
          }}
          className="akashic-field pr-11"
          placeholder={placeholder}
        />
        <button
          type="button"
          onClick={() => setIsOpen((open) => !open)}
          className="absolute inset-y-0 right-0 flex w-11 items-center justify-center text-[#c8b392] transition-colors hover:text-[#efe4cd]"
          aria-label="展开备选"
        >
          <ChevronDown className={`h-4 w-4 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
        </button>
      </div>

      {isOpen ? (
        <div className="absolute inset-x-0 top-[calc(100%+0.45rem)] z-30 overflow-hidden rounded-2xl border border-[#6f6655]/55 bg-[#0d1627]/96 shadow-[0_16px_36px_rgba(2,8,18,0.48)] backdrop-blur-xl">
          {canKeepCustomValue ? (
            <button
              type="button"
              onClick={() => {
                onChange(value);
                setIsOpen(false);
              }}
              className="flex w-full items-center justify-between gap-3 border-b border-white/8 px-3.5 py-3 text-left transition-colors hover:bg-white/5"
            >
              <span className="text-sm text-[#efe4cd]">{createText}</span>
              <span className="truncate text-xs text-[#9ca7be]">{trimmedValue}</span>
            </button>
          ) : null}

          {filteredOptions.length > 0 ? (
            <div className="max-h-56 overflow-y-auto py-1.5">
              {filteredOptions.map((option) => (
                <button
                  key={option}
                  type="button"
                  onClick={() => {
                    onChange(option);
                    setIsOpen(false);
                  }}
                  className={`block w-full px-3.5 py-2.5 text-left text-sm transition-colors hover:bg-white/5 ${option === trimmedValue ? 'bg-white/6 text-[#f6eddc]' : 'text-[#d7c7ab]'}`}
                >
                  {option}
                </button>
              ))}
            </div>
          ) : (
            <div className="px-3.5 py-3 text-sm text-[#9ca7be]">{emptyText}</div>
          )}
        </div>
      ) : null}
    </div>
  );
};

const CreationPage: React.FC = () => {
  const navigate = useNavigate();
  const {
    character,
    world,
    updateCharacter,
    updateWorld,
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
  const radarSize = 240;
  const radarCenter = radarSize / 2;
  const radarRadius = 72;
  const radarLabelRadius = 96;
  const radarAngles = traitRows.map((_, index) => (-Math.PI / 2) + ((Math.PI * 2 * index) / traitRows.length));
  const radarShapePoints = traitRows.map((trait, index) => {
    const ratio = trait.value / ATTRIBUTE_MAX;
    const x = radarCenter + (Math.cos(radarAngles[index]) * radarRadius * ratio);
    const y = radarCenter + (Math.sin(radarAngles[index]) * radarRadius * ratio);
    return `${x},${y}`;
  }).join(' ');

  const canStart = Boolean(character.name.trim()) && (character.gender === '男' || character.gender === '女');

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
      <StoryFrame className="min-h-fit overflow-visible p-3.5 sm:pb-4 md:p-5 md:pb-5">
        <div className="space-y-5 md:space-y-6">
          {error ? (
            <StatusPill
              icon={TriangleAlert}
              className="border-[#7f3b3b]/50 bg-[#2a1216]/85 text-[#ffd7d7]"
              iconClassName="text-[#ff9b9b]"
            >
              {error}
            </StatusPill>
          ) : null}

          <section className="space-y-3">
            <div className="space-y-1.5">
              <div className="flex flex-wrap items-baseline gap-3">
                <h2 className="text-lg font-semibold text-[#f6eddc] md:text-xl">塑造主角轮廓</h2>
              </div>

            </div>

            <SectionCard className="z-40 space-y-3.5 p-3.5 md:p-4" style={{ overflow: 'visible' }}>
              <div className="grid gap-2.5 md:grid-cols-[minmax(0,1.2fr)_132px_132px] md:gap-3">
                <div>
                  <FieldLabel>姓名</FieldLabel>
                  <input
                    type="text"
                    value={character.name}
                    onChange={(e) => updateCharacter({ name: e.target.value })}
                    className="akashic-field"
                    placeholder="例子：艾莉亚"
                  />
                </div>

                <div className="relative">
                  <FieldLabel>性别</FieldLabel>
                  <select
                    value={character.gender}
                    onChange={(e) => updateCharacter({ gender: e.target.value })}
                    className="akashic-select"
                    required
                  >
                    <option value="" disabled>请选择性别</option>
                    <option value="男">男</option>
                    <option value="女">女</option>
                  </select>
                  <ChevronDown className="pointer-events-none absolute right-3.5 top-[2.95rem] h-4 w-4 -translate-y-1/2 text-[#c8b392] md:top-[3.1rem]" />
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
                <FieldLabel hint="">命运烙印</FieldLabel>
                <SearchableSelect
                  value={character.background}
                  options={backgroundOptions}
                  placeholder="决定主角人生默认模式，但非绝对牢笼"
                  emptyText="你可以直接写下新的烙印。"
                  createText="采用你此刻写下的命运烙印"
                  onChange={(nextValue) => updateCharacter({ background: nextValue })}
                />
              </div>
              <div>
                <FieldLabel hint="">人物描述</FieldLabel>
                <textarea
                  value={character.appearance}
                  onChange={(e) => updateCharacter({ appearance: e.target.value })}
                  className="akashic-field min-h-24 resize-y"
                  placeholder="你可以在这输入详细的人物设定，诸如人物外貌描述，性格等。"
                />
              </div>


            </SectionCard>

            <SectionCard className="space-y-4 p-3.5 md:p-4">
              <div className="flex flex-col gap-2.5 md:flex-row md:items-center md:justify-between">
                <div className="space-y-1">
                  <FieldLabel hint="">人物属性</FieldLabel>

                </div>
                <StatusPill icon={null} className="bg-[#0f1c31]/70 px-2.5 py-1 text-[11px] text-[#d7e5ff] md:text-xs">
                  {remainingPoints === 0 ? '点数已分配完成' : `剩余 ${remainingPoints} 点待分配`}
                </StatusPill>
              </div>

              <div className="grid gap-4 md:grid-cols-[248px_minmax(0,1fr)] md:items-start">
                <div className="mx-auto w-full max-w-[248px]">
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
                            fontSize="11"
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
                      return <circle key={`${trait.key}-point`} cx={x} cy={y} r="3.2" fill="#f6eddc" />;
                    })}
                  </svg>
                </div>

                <div className="space-y-3">
                  {traitRows.map((trait) => (
                    <div key={trait.key} className="rounded-xl border border-white/8 bg-[#0c1422]/58 p-2.5 md:p-3">
                      <div className="mb-1.5 flex items-center justify-between gap-2.5 text-[#efe4cd]">
                        <div>
                          <p className="text-sm font-semibold md:text-base">{trait.label}</p>
                          <p className="text-[11px] leading-3.5 text-[#9ca7be] md:text-xs">{trait.hint}</p>
                        </div>
                        <span className="text-base font-semibold text-[#d8c7aa] md:text-lg">{trait.value}</span>
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

          <section className="space-y-3">
            <div className="space-y-1.5">
              <h2 className="text-lg font-semibold text-[#f6eddc] md:text-xl">勾勒故事舞台</h2>
            </div>

            <SectionCard className="z-30 space-y-3.5 p-3.5 md:p-4" style={{ overflow: 'visible' }}>
              <div>
                <FieldLabel>世界背景</FieldLabel>
                <SearchableSelect
                  value={world.era}
                  options={eraOptions}
                  placeholder="搜索一个世界，或写下你想要的世界"
                  emptyText="没有找到贴近的世界，你可以直接写下新的世界。"
                  createText="采用你此刻写下的世界"
                  onChange={(nextValue) => updateWorld({ era: nextValue })}
                />
              </div>

              <div>
                <FieldLabel hint="">世界描述</FieldLabel>
                <textarea
                  value={world.description}
                  onChange={(e) => updateWorld({ description: e.target.value })}
                  className="akashic-field min-h-24 resize-y"
                  placeholder="你可以在这输入详细的世界描述，诸如世界历史，地理环境等。"
                />
              </div>
            </SectionCard>
          </section>

          {/*
            <section className="space-y-3">
              <div className="space-y-1.5">
                <h2 className="text-lg font-semibold text-[#f6eddc] md:text-xl">写下你的叙事期待</h2>
              </div>

              <SectionCard className="space-y-3.5 p-3.5 md:p-4">
                <div className="grid gap-3 md:grid-cols-2">
                  <div>
                    <FieldLabel hint="决定故事更像悬疑、成长、悲剧、救赎或权谋等">主题</FieldLabel>
                    <input
                      type="text"
                      value={story.theme}
                      onChange={(e) => updateStory({ theme: e.target.value })}
                      className="akashic-field"
                      placeholder="例子：阴谋悬疑中的自我救赎"
                    />
                  </div>

                  <div>
                    <FieldLabel hint="描述你希望故事整体带来的情绪温度">故事氛围</FieldLabel>
                    <input
                      type="text"
                      value={story.atmosphere}
                      onChange={(e) => updateStory({ atmosphere: e.target.value })}
                      className="akashic-field"
                      placeholder="例子：压抑、潮湿、危险中偶有微光"
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
                    placeholder="例子：冷峻克制，偏文学叙事，少解释多暗示"
                  />
                </div>

                <div>
                  <FieldLabel hint="写下不想出现的题材、桥段、关系或叙事手法">禁区</FieldLabel>
                  <textarea
                    value={story.taboos}
                    onChange={(e) => updateStory({ taboos: e.target.value })}
                    className="akashic-field min-h-20 resize-y"
                    placeholder="例子：不要校园恋爱、不要无代价复活、避免轻浮搞笑消解压迫感"
                  />
                </div>
              </SectionCard>
            </section>
          */}

          <div className="sticky bottom-[max(0.75rem,env(safe-area-inset-bottom))] z-20 mt-2 flex touch-pan-y flex-col gap-2 rounded-xl border border-[#6f6655]/50 bg-[#0a1222]/94 p-1.5 shadow-[0_12px_28px_rgba(2,8,18,0.44)] backdrop-blur-xl sm:static sm:inset-auto sm:mt-1 sm:flex-row sm:justify-end sm:rounded-none sm:border-0 sm:bg-transparent sm:p-0 sm:shadow-none">
            <SecondaryButton onClick={() => navigate(appRoutes.lobby)} className="min-h-10 w-full px-3.5 py-2 text-sm sm:w-auto md:min-h-11 md:px-4 md:py-2.5">
              返回大厅
            </SecondaryButton>
            <PrimaryButton onClick={handleStartGame} disabled={!canStart || isLoading} className="min-h-10 w-full px-3.5 py-2 text-sm sm:w-auto md:min-h-11 md:px-4 md:py-2.5">
              {isLoading ? '设定生成中...' : '开启命运'}
            </PrimaryButton>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default CreationPage;
