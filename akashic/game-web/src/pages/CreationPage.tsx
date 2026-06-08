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
  '从祭坛逃出的献身者',
  '替王朝守密的史官后裔',
  '在旧神低语中长大的孤儿',
  '被流放的群星领航员',
  '向深海讨回姓名的幸存者',
  '握有禁忌配方的年轻医者',
  '见过未来残片的街头占梦师',
  '替亡者送信的黄昏行者',
  '在帝国边境长大的驯兽人',
  '从时间裂隙归来的见证者',
  '曾为神明铸刃的沉默工匠',
  '带着罪名远行的前圣职者',
  '被镜中倒影不断呼唤的人',
  '掌握古老契约的最后传人',
  '从天空坠落却未曾死去的旅者',
  '替家族偿还旧债的年轻家主',
  '在瘟疫年代守住火种的记录者',
  '被命运误写姓名的替身者',
  '体内封印着灭世魔王的童身',
  '被十二枚时之楔钉住灵魂的永生者',
  '窃取神明眼眸而获罪的凡人',
  '背负整个末日记忆的轮回者',
  '被星海诅咒，触碰之物皆会凋零的园丁',
  '生来与镜像世界相连的钥匙之子',
  '被炼金术改写血脉的混血种',
  '以歌声换取恶魔契约的失声歌伶',
  '无法被死亡接纳的守墓人',
  '从故事中逃出，被作者追杀的书中人',
  '继承了一万把诅咒兵器的兵器冢',
  '被神明降下“存在稀薄化”惩罚的神眷者',
  '饮下时间之沙，目睹所有未来的预言家',
  '与影中群星签订租界协议的暗夜行者',
  '生而被选为活体天灾坐标的封印容器',
  '用灵魂碎片铸造过七把圣剑的剑匠',
  '被遗忘的战争机器，如今苏醒的古代兵器',
  '体内流淌着世界树汁液的半树人',
  '行走于人间的灾厄观测装置',
  '窃取了九层地狱税单的逃税灵魂'
];
const eraOptions = [
  '蒸汽朋克',
  '星际拓荒',
  '东方玄幻',
  '末日废土',
  '深海神秘纪元',
  '黄沙遗城',
  '群星帝国余晖',
  '永夜雪境',
  '浮空群岛文明',
  '机械教廷',
  '旧日神祇复苏前夜',
  '灵潮漫过山海的年代',
  '巨兽与部族并存的荒原',
  '魔法工业化的黄金世代',
  '日蚀笼罩的海上王国',
  '梦境与现实重叠的都会',
  '龙骸埋地的古战场时代',
  '诸国冷战下的秘术世界',
  '王朝崩塌后的乱世九州',
  '被高塔统治的雾都',
  '群山封国与神谕并行的边陲',
  '文明重启后的新曙光纪',
  '齿轮与咒法共鸣的巴洛克魔导世纪',
  '深渊倒映天空的双界重叠大陆',
  '众神陨落为矿脉的遗骸星域',
  '被永不停歇的歌剧风暴笼罩的哀鸣洋',
  '以记忆为货币的通感都市',
  '地下洞窟串联万界的无光迷宫层',
  '植物逆侵吞文明的翠骸纪元',
  '天空浮游着神骸岛屿的破碎世界',
  '每十年翻转一次重力的交错行星',
  '灵魂可以自由买卖的契约联邦',
  '由七种色彩法则统治的色谱王国',
  '昼夜各为不同时空的暮晓城邦',
  '所有死者都会在满月夜醒来的幽灵乐土',
  '被巨大环形山脉圈禁的牢笼世界',
  '语言会实体化为武器的修辞帝国',
  '机器与亡灵共同祷告的蒸汽圣殿时期',
  '潮汐由巨兽呼吸掌控的活体群岛',
  '梦境在现实投下阴影的噩梦边境',
  '万物声音皆被剥夺的寂静纪废墟',
  '太阳与月亮为敌对阵营的永战星空'
];

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
  createText: string;
  onChange: (value: string) => void;
}

const SearchableSelect: React.FC<SearchableSelectProps> = ({
  value,
  options,
  placeholder,
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

          <div className="max-h-56 overflow-y-auto py-1.5">
            {options.map((option) => (
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
  const [ageInput, setAgeInput] = React.useState(() => ({
    sourceAge: character.age,
    value: String(character.age),
  }));
  const displayedAgeInput = ageInput.sourceAge === character.age
    ? ageInput.value
    : String(character.age);

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
                    value={displayedAgeInput}
                    onChange={(e) => {
                      const nextValue = e.target.value;

                      if (nextValue === '') {
                        setAgeInput({
                          sourceAge: character.age,
                          value: nextValue,
                        });
                        return;
                      }

                      const parsedAge = Number.parseInt(nextValue, 10);
                      if (Number.isNaN(parsedAge)) {
                        setAgeInput({
                          sourceAge: character.age,
                          value: nextValue,
                        });
                        return;
                      }

                      const nextAge = Math.max(0, parsedAge);
                      setAgeInput({
                        sourceAge: nextAge,
                        value: nextValue,
                      });
                      updateCharacter({ age: nextAge });
                    }}
                    onBlur={() => {
                      setAgeInput({
                        sourceAge: character.age,
                        value: String(character.age),
                      });
                    }}
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
