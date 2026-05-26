import React, { useEffect, useMemo, useState } from 'react';
import { ArrowLeft, FolderOpen, Library, Trash2, TriangleAlert } from 'lucide-react';
import {
  PageTitle,
  PrimaryButton,
  ScreenShell,
  SecondaryButton,
  SectionCard,
  StoryFrame,
  StatusPill,
} from '../components/AkashicUI';
import {
  readStoredSaveSlots,
  removeStoredSaveSlot,
  type StoredSaveSlot,
} from '../lib/saveSlots';
import { useGameUIStore } from '../store/gameStore';

function formatTimeLabel(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}

const ArchiveListPage: React.FC = () => {
  const setGameState = useGameUIStore((state) => state.setGameState);
  const loadSave = useGameUIStore((state) => state.loadSave);
  const isLoading = useGameUIStore((state) => state.isLoading);
  const error = useGameUIStore((state) => state.error);
  const [slots, setSlots] = useState<StoredSaveSlot[]>([]);

  useEffect(() => {
    setSlots(readStoredSaveSlots());
  }, []);

  const hasSlots = useMemo(() => slots.length > 0, [slots]);

  const handleLoad = async (slotId: string) => {
    try {
      await loadSave(slotId);
    } catch {
      // Store already exposes the failure reason.
    }
  };

  const handleDelete = (slot: StoredSaveSlot) => {
    const confirmed = window.confirm(`确认删除本地存档索引“${slot.title || slot.slotId}”吗？`);
    if (!confirmed) {
      return;
    }

    removeStoredSaveSlot(slot.slotId);
    setSlots(readStoredSaveSlots());
  };

  return (
    <ScreenShell>
      <StoryFrame className="overflow-hidden p-6 md:p-8">
        <div
          className="absolute inset-0 bg-cover bg-center bg-no-repeat opacity-20"
          style={{
            backgroundImage:
              'url("https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20quiet%20fantasy%20archive%20hall%20filled%20with%20moonlit%20shelves%2C%20mystical%20save%20records%2C%20dark%20blue%20cinematic%20game%20ui%20background&image_size=landscape_16_9")',
          }}
        />
        <div className="relative z-10 space-y-6">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <StatusPill icon={Library}>本地存档索引</StatusPill>
            <SecondaryButton onClick={() => setGameState('lobby')} disabled={isLoading}>
              <ArrowLeft className="h-4 w-4" />
              返回大厅
            </SecondaryButton>
          </div>

          <PageTitle
            title="存档列表"
            subtitle="以下槽位索引来自当前浏览器的 localStorage。"
          />

          {error ? (
            <StatusPill
              icon={TriangleAlert}
              className="border-[#7f3b3b]/50 bg-[#2a1216]/85 text-[#ffd7d7]"
              iconClassName="text-[#ff9b9b]"
            >
              {error}
            </StatusPill>
          ) : null}

          {!hasSlots ? (
            <SectionCard className="space-y-3">
              <p className="text-base text-[#e9edf7]">当前浏览器里还没有记录任何存档槽。</p>
              <p className="text-sm leading-7 text-[#98a3ba]">
                先进入一局游戏并点击“存档”，创建成功后这里会自动列出对应的槽位。
              </p>
            </SectionCard>
          ) : (
            <div className="space-y-4">
              {slots.map((slot) => (
                <SectionCard key={slot.slotId} className="space-y-4">
                  <div className="space-y-2">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="space-y-1">
                        <h3 className="text-lg text-[#f2eadf]">{slot.title || '未命名存档'}</h3>
                        <p className="text-xs uppercase tracking-[0.18em] text-[#8f98ab]">
                          {slot.slotId}
                        </p>
                      </div>
                      <StatusPill icon={null}>最近更新 {formatTimeLabel(slot.updatedAt)}</StatusPill>
                    </div>
                    <p className="text-sm text-[#b6c0d6]">
                      Session: {slot.sessionId}
                    </p>
                    <p className="text-sm text-[#8f98ab]">
                      创建于 {formatTimeLabel(slot.createdAt)}
                    </p>
                  </div>
                  <div className="flex flex-col gap-3 sm:flex-row">
                    <PrimaryButton
                      onClick={() => handleLoad(slot.slotId)}
                      disabled={isLoading}
                      className="flex-1"
                    >
                      <FolderOpen className="h-4 w-4" />
                      {isLoading ? '读取中...' : '读取此存档'}
                    </PrimaryButton>
                    <SecondaryButton
                      type="button"
                      onClick={() => handleDelete(slot)}
                      disabled={isLoading}
                      className="flex-1 text-[#ffb6b6] hover:border-[#7f3b3b]/60 hover:bg-[#2a1216]/85 hover:text-[#ffd7d7]"
                    >
                      <Trash2 className="h-4 w-4" />
                      删除索引
                    </SecondaryButton>
                  </div>
                </SectionCard>
              ))}
            </div>
          )}
        </div>
      </StoryFrame>
    </ScreenShell>
  );
};

export default ArchiveListPage;
