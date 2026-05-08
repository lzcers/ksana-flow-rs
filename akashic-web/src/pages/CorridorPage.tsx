import React, { useEffect, useMemo, useState } from 'react';
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
import { getArchive } from '../lib/api';

const CorridorPage: React.FC = () => {
  const {
    setGameState,
    character,
    world,
    sessionId,
    currentNode,
    turnIndex,
    archives,
    saves,
    fetchCorridorData,
    loadSave,
    isLoading,
    error,
  } = useGameStore();
  const [archiveDetails, setArchiveDetails] = useState<Record<string, string>>({});

  useEffect(() => {
    void fetchCorridorData();
  }, [fetchCorridorData]);

  useEffect(() => {
    const archiveId = archives[0]?.archiveId;
    if (!archiveId || archiveDetails[archiveId]) return;

    void getArchive(archiveId)
      .then((detail) => {
        setArchiveDetails((prev) => ({ ...prev, [archiveId]: detail.ending.biography }));
      })
      .catch(() => undefined);
  }, [archives, archiveDetails]);

  const activeSave = useMemo(() => {
    if (!sessionId) return null;
    return saves.find((item) => item.sessionId === sessionId) ?? null;
  }, [saves, sessionId]);

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
                <StatusPill icon={Clock3}>{archives.length} 份回响</StatusPill>
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
              {archives.map((item) => (
                <article
                  key={item.archiveId}
                  className="overflow-hidden rounded-[1.5rem] border border-[#6f6655]/65 bg-[#0b1323]/82"
                >
                  <div className="aspect-[16/9] overflow-hidden">
                    <img src={item.coverImage} alt={item.title} className="h-full w-full object-cover" />
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
                    {archiveDetails[item.archiveId] ? (
                      <div className="rounded-[1rem] border border-white/10 bg-black/20 px-4 py-3 text-sm leading-7 text-[#d8e0ef]">
                        {archiveDetails[item.archiveId]}
                      </div>
                    ) : null}
                  </div>
                </article>
              ))}
            </div>
            {!archives.length ? (
              <div className="rounded-[1.35rem] border border-dashed border-[#6f6655]/60 bg-[#0b1323]/55 p-6 text-[#c4d0ea]">
                还没有已归档的结局。完成一段命运后，这里会出现真正来自后端的回响馆藏。
              </div>
            ) : null}
          </SectionCard>

          <div className="grid gap-5 lg:grid-cols-[1.1fr_0.9fr]">
            <SectionCard>
              <div className="mb-4 flex items-center gap-3">
                <BookMarked className="h-5 w-5 text-[#d7c4a1]" />
                <h2 className="text-2xl font-semibold text-[#f6eddc]">当前悬挂中的旅程</h2>
              </div>
              {sessionId && currentNode ? (
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
                    <p className="mt-2 text-3xl font-semibold text-[#f3ead8]">{turnIndex}</p>
                    <p className="mt-2 text-sm leading-7 text-[#c4d0ea]">
                      当前片段：{currentNode.text}
                    </p>
                  </div>
                  {activeSave ? (
                    <div className="rounded-[1.35rem] border border-[#6f6655]/60 bg-[#0b1323]/80 p-5">
                      <p className="text-sm text-[#93a1bb]">最近存档</p>
                      <p className="mt-2 text-xl font-semibold text-[#f3ead8]">{activeSave.title}</p>
                      <p className="mt-2 text-sm leading-7 text-[#c4d0ea]">{activeSave.summary}</p>
                    </div>
                  ) : null}
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
                <PrimaryButton onClick={() => setGameState(sessionId ? 'playing' : 'creation')} className="flex-1">
                  <Stars className="h-4 w-4" />
                  {sessionId ? '继续当前旅程' : '开启第一段人生'}
                </PrimaryButton>
              </div>
            </SectionCard>
          </div>

          <SectionCard>
            <div className="mb-5 flex items-center justify-between gap-3">
              <h2 className="text-2xl font-semibold text-[#f6eddc]">进行中存档</h2>
              <span className="text-sm text-[#93a1bb]">来自后端 `/api/saves`</span>
            </div>
            <div className="space-y-3">
              {saves.map((save) => (
                <div
                  key={save.saveId}
                  className="flex flex-col gap-4 rounded-[1.35rem] border border-[#6f6655]/60 bg-[#0b1323]/80 p-5 lg:flex-row lg:items-center lg:justify-between"
                >
                  <div className="space-y-2">
                    <p className="text-sm text-[#93a1bb]">{save.era} · 第 {save.turnIndex} 幕</p>
                    <p className="text-xl font-semibold text-[#f3ead8]">{save.title}</p>
                    <p className="text-sm leading-7 text-[#c4d0ea]">{save.summary}</p>
                  </div>
                  <PrimaryButton onClick={() => void loadSave(save.saveId)} disabled={isLoading}>
                    <Stars className="h-4 w-4" />
                    读档继续
                  </PrimaryButton>
                </div>
              ))}
              {!saves.length ? (
                <div className="rounded-[1.35rem] border border-dashed border-[#6f6655]/60 bg-[#0b1323]/55 p-6 text-[#c4d0ea]">
                  还没有存档。你可以在游玩页先保存一段旅程。
                </div>
              ) : null}
            </div>
          </SectionCard>

          {error ? <p className="text-sm text-[#d9cbb1]">{error}</p> : null}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default CorridorPage;
