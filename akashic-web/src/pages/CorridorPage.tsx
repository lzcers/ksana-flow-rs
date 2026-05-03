import React, { useMemo } from 'react';
import {
  ArrowLeft,
  BookMarked,
  Clock3,
  Image as ImageIcon,
  Library,
  Sparkles,
  Stars,
} from 'lucide-react';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StatusPill,
  StoryFrame,
} from '../components/AkashicUI';
import { useGameStore } from '../store/gameStore';

interface ArchiveCard {
  id: string;
  title: string;
  tag: string;
  era: string;
  summary: string;
  memory: string;
  detail?: string;
}

const archiveSeeds: ArchiveCard[] = [
  {
    id: 'seed-1',
    title: '灰烬港的夜巡人',
    tag: '已归档',
    era: '末日废土',
    summary: '你在塌陷的灯塔下守住最后的航标，让一座濒死聚落撑过风暴夜。',
    memory:
      'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20post-apocalyptic%20harbor%20at%20night%2C%20lonely%20watchman%2C%20blue%20glow%2C%20cinematic%20concept%20art&image_size=landscape_16_9',
  },
  {
    id: 'seed-2',
    title: '纸鹤坠入星潮',
    tag: '已归档',
    era: '星际拓荒',
    summary: '你在外环殖民地拦截失控信号，把一封迟来的家书送回原主人手中。',
    memory:
      'https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20futuristic%20space%20colony%20with%20floating%20paper%20cranes%2C%20melancholic%20blue%20lighting%2C%20concept%20art&image_size=landscape_16_9',
  },
];

const CorridorPage: React.FC = () => {
  const { setGameState, endingData, storyNodes, character, world } = useGameStore();

  const liveArchive = useMemo(() => {
    if (!endingData) return null;

    return {
      id: 'latest-ending',
      title: '最新回响',
      tag: '本次命运',
      era: world.era,
      summary: endingData.legacy,
      memory: endingData.cgs[0],
      detail: endingData.biography,
    };
  }, [endingData, world.era]);

  const archiveCards = liveArchive ? [liveArchive, ...archiveSeeds] : archiveSeeds;

  return (
    <ScreenShell className="items-stretch">
      <StoryFrame className="p-5 md:p-6">
        <div className="space-y-5">
          <div
            className="relative overflow-hidden rounded-[1.8rem] border border-[#6f6655] bg-cover bg-center px-5 py-6 md:px-8 md:py-8"
            style={{
              backgroundImage:
                'linear-gradient(180deg, rgba(4, 10, 24, 0.45), rgba(4, 10, 24, 0.86)), url("https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20vast%20mystical%20hall%20of%20memory%2C%20dark%20blue%20archive%20walls%2C%20golden%20lines%2C%20cinematic%20concept%20art&image_size=landscape_16_9")',
            }}
          >
            <div className="space-y-4">
              <div className="flex flex-wrap gap-2 md:gap-3">
                <StatusPill icon={Library}>我的回廊</StatusPill>
                <StatusPill icon={Sparkles}>记忆归档</StatusPill>
                <StatusPill icon={Clock3}>{archiveCards.length} 份回响</StatusPill>
              </div>
              <PageTitle
                title="我的回廊"
                subtitle="这里陈列着每一段被你触发过的命运残片。已完成的结局会沉入馆藏，未完成的旅程则以当前态悬挂在回廊尽头。"
                action={
                  <SecondaryButton onClick={() => setGameState('lobby')}>
                    <ArrowLeft className="h-4 w-4" />
                    返回大厅
                  </SecondaryButton>
                }
              />
            </div>
          </div>

          <SectionCard>
            <div className="mb-5 flex items-center justify-between gap-3">
              <h2 className="text-2xl font-semibold text-[#f6eddc]">回响藏品</h2>
              <span className="text-sm text-[#93a1bb]">按最近可见的记忆排序</span>
            </div>
            <div className="grid gap-4 lg:grid-cols-2">
              {archiveCards.map((item) => (
                <article
                  key={item.id}
                  className="overflow-hidden rounded-[1.5rem] border border-[#6f6655]/65 bg-[#0b1323]/82"
                >
                  <div className="aspect-[16/9] overflow-hidden">
                    <img src={item.memory} alt={item.title} className="h-full w-full object-cover" />
                  </div>
                  <div className="space-y-3 p-5">
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <p className="text-xs tracking-[0.25em] text-[#9aa8c1]">{item.era}</p>
                        <h3 className="mt-1 text-xl font-semibold text-[#f3ead8]">{item.title}</h3>
                      </div>
                      <span className="rounded-full border border-[#6f6655]/70 bg-[#111b2f] px-3 py-1 text-xs text-[#d9c8a8]">
                        {item.tag}
                      </span>
                    </div>
                    <p className="text-sm leading-7 text-[#c4d0ea]">{item.summary}</p>
    {item.detail ? (
                      <div className="rounded-[1rem] border border-white/10 bg-black/20 px-4 py-3 text-sm leading-7 text-[#d8e0ef]">
                        {item.detail}
                      </div>
                    ) : null}
                  </div>
                </article>
              ))}
            </div>
          </SectionCard>

          <div className="grid gap-5 lg:grid-cols-[1.1fr_0.9fr]">
            <SectionCard>
              <div className="mb-4 flex items-center gap-3">
                <BookMarked className="h-5 w-5 text-[#d7c4a1]" />
                <h2 className="text-2xl font-semibold text-[#f6eddc]">当前悬挂中的旅程</h2>
              </div>
              {storyNodes.length > 0 ? (
                <div className="space-y-4">
                  <div className="rounded-[1.35rem] border border-[#6f6655]/60 bg-[#0b1323]/80 p-5">
                    <p className="text-sm text-[#93a1bb]">当前主角</p>
                    <p className="mt-2 text-2xl font-semibold text-[#f3ead8]">{character.name || '未命名旅人'}</p>
                    <p className="mt-2 text-sm leading-7 text-[#c4d0ea]">
                      {character.background || '尚未写入人生烙印'} · {world.era}
                    </p>
                  </div>
                  <div className="rounded-[1.35rem] border border-[#6f6655]/60 bg-[#0b1323]/80 p-5">
                    <p className="text-sm text-[#93a1bb]">已推进章节</p>
                    <p className="mt-2 text-3xl font-semibold text-[#f3ead8]">{storyNodes.length}</p>
                    <p className="mt-2 text-sm leading-7 text-[#c4d0ea]">
                      最近一段命运仍停留在回廊边缘，等待你重新返回故事继续书写。
                    </p>
                  </div>
                </div>
              ) : (
                <div className="rounded-[1.35rem] border border-dashed border-[#6f6655]/60 bg-[#0b1323]/55 p-6 text-[#c4d0ea]">
                  你还没有留下正在进行中的旅程。开启一段新人生后，这里会显示当前故事的摘要与推进进度。
                </div>
              )}
            </SectionCard>

            <SectionCard>
              <div className="mb-4 flex items-center gap-3">
                <ImageIcon className="h-5 w-5 text-[#d7c4a1]" />
                <h2 className="text-2xl font-semibold text-[#f6eddc]">馆藏规则</h2>
              </div>
              <div className="space-y-3 text-sm leading-7 text-[#c4d0ea]">
                <div className="rounded-[1.2rem] border border-white/10 bg-black/20 px-4 py-3">
                  已完成的结局会优先展示，并附带记忆切片封面。
                </div>
                <div className="rounded-[1.2rem] border border-white/10 bg-black/20 px-4 py-3">
                  进行中的故事会显示主角、时代与章节数，方便继续接续。
                </div>
                <div className="rounded-[1.2rem] border border-white/10 bg-black/20 px-4 py-3">
                  后续若接入持久化，这里可以直接扩展成真正的存档回廊。
                </div>
              </div>
              <div className="mt-5 flex flex-col gap-3 sm:flex-row">
                <PrimaryButton onClick={() => setGameState(storyNodes.length > 0 ? 'playing' : 'creation')} className="flex-1">
                  <Stars className="h-4 w-4" />
                  {storyNodes.length > 0 ? '继续当前旅程' : '开启第一段人生'}
                </PrimaryButton>
              </div>
            </SectionCard>
          </div>
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default CorridorPage;
